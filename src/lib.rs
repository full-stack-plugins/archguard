//! ArchGuard's first adapter reads REAL Cargo workspace package dependencies.
//! It produces neutral GuardFacts and delegates evaluation to GuardEngine.

pub mod analysis;
pub mod integration;

use analysis::cargo::{CargoObservation, DependencyDeclaration};
use guardengine::{
    API_VERSION, AnalyzerIdentity, Completeness, GuardAnalyzer, GuardError, GuardFact, GuardFacts,
    GuardSubject,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Default)]
pub struct CargoWorkspaceAnalyzer;

impl GuardAnalyzer for CargoWorkspaceAnalyzer {
    fn analyze(&self, project_root: &Path, subject_id: &str) -> Result<GuardFacts, GuardError> {
        let root = project_root.canonicalize().map_err(|error| {
            GuardError::Input(format!("project root cannot be resolved: {error}"))
        })?;

        let result = collect_workspace_observation(&root);
        let (completeness, facts, digest, diagnostics) = match result {
            Ok(observation) => (
                Completeness::Complete,
                observation.facts,
                observation.manifest_digest,
                vec![],
            ),
            Err(problem) => (
                Completeness::Partial,
                vec![],
                fallback_digest(&root),
                vec![problem],
            ),
        };

        Ok(GuardFacts {
            api_version: API_VERSION.into(),
            kind: "GuardFacts".into(),
            analyzer: AnalyzerIdentity {
                id: "archguard.cargo.workspace".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            subject: GuardSubject {
                id: subject_id.into(),
                snapshot_digest: digest,
            },
            completeness,
            facts,
            diagnostics,
        })
    }
}

fn collect_workspace_observation(root: &Path) -> Result<CargoObservation, String> {
    let manifest_path = root.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Err(format!(
            "Cargo.toml not found in project root: {}",
            root.display()
        ));
    }
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(&manifest_path)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cargo metadata could not start: {error}"))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed: {error}"));
    }

    observation_from_metadata(root, &output.stdout)
}

fn observation_from_metadata(root: &Path, bytes: &[u8]) -> Result<CargoObservation, String> {
    let manifest_path = root.join("Cargo.toml");
    let metadata: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("cargo metadata returned invalid JSON: {error}"))?;
    let package_items = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata omitted packages")?;
    let workspace_members: HashSet<&str> = metadata["workspace_members"]
        .as_array()
        .ok_or("cargo metadata omitted workspace_members")?
        .iter()
        .filter_map(Value::as_str)
        .collect();

    // This adapter tracks only local packages that are members of this Cargo workspace.
    // The resolved absolute dependency path, not just its name, must match a member.
    let packages: Vec<&Value> = package_items
        .iter()
        .filter(|item| {
            item["id"]
                .as_str()
                .is_some_and(|id| workspace_members.contains(id))
        })
        .collect();
    let mut names_by_dir: HashMap<PathBuf, String> = HashMap::new();
    let mut content_hash_input: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    insert_manifest_bytes(root, &manifest_path, &mut content_hash_input)?;
    for pkg in &packages {
        let name = pkg["name"].as_str().ok_or("package without a name")?;
        let manifest = pkg["manifest_path"]
            .as_str()
            .ok_or("package without manifest_path")?;
        let path = Path::new(manifest);
        let parent = path.parent().ok_or("manifest has no parent")?;
        let canonical_parent = parent.canonicalize().map_err(|error| error.to_string())?;
        names_by_dir.insert(canonical_parent, name.into());
        insert_manifest_bytes(root, path, &mut content_hash_input)?;
    }

    let mut facts = Vec::new();
    let mut declarations = Vec::new();
    for pkg in &packages {
        let pkg_name = pkg["name"].as_str().ok_or("package without a name")?;
        let manifest = pkg["manifest_path"]
            .as_str()
            .ok_or("package without manifest_path")?;
        let source = display_relative(root, Path::new(manifest));
        let dependencies = pkg["dependencies"]
            .as_array()
            .ok_or("package without dependencies")?;
        for dependency in dependencies {
            let member_target = if let Some(target_dir) = dependency["path"].as_str() {
                let canonical_target = Path::new(target_dir).canonicalize().map_err(|error| {
                    format!("dependency path {target_dir} inaccessible: {error}")
                })?;
                names_by_dir.get(&canonical_target).cloned()
            } else {
                None
            };
            declarations.push(DependencyDeclaration {
                member: pkg_name.into(),
                package: dependency["name"]
                    .as_str()
                    .ok_or("dependency without name")?
                    .into(),
                rename: dependency["rename"].as_str().map(str::to_owned),
                kind: dependency["kind"].as_str().unwrap_or("normal").into(),
                optional: dependency["optional"]
                    .as_bool()
                    .ok_or("dependency without optional flag")?,
                target: dependency["target"].as_str().map(str::to_owned),
                dependency_source: dependency["source"].as_str().map(str::to_owned),
                path: dependency["path"]
                    .as_str()
                    .map(|p| display_relative(root, Path::new(p))),
                member_target: member_target.clone(),
                source: source.clone(),
            });
            if let Some(target_name) = member_target {
                facts.push(GuardFact {
                    subject: pkg_name.into(),
                    predicate: "depends_on".into(),
                    object: target_name,
                    source: source.clone(),
                });
            }
        }
    }
    declarations.sort();
    facts.sort();
    facts.dedup();

    let mut hasher = Sha256::new();
    for (file, bytes) in content_hash_input {
        hasher.update((file.len() as u64).to_le_bytes());
        hasher.update(file.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    let digest = format!("sha256:{:x}", hasher.finalize());
    Ok(CargoObservation {
        members: names_by_dir.into_values().collect(),
        declarations,
        facts,
        manifest_digest: digest,
    })
}

fn insert_manifest_bytes(
    root: &Path,
    manifest: &Path,
    target: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let relative = display_relative(root, manifest);
    let bytes = fs::read(manifest).map_err(|error| format!("cannot read {relative}: {error}"))?;
    target.insert(relative, bytes);
    Ok(())
}

fn display_relative(root: &Path, source: &Path) -> String {
    source
        .strip_prefix(root)
        .unwrap_or(source)
        .to_string_lossy()
        .replace('\\', "/")
}

fn fallback_digest(root: &Path) -> String {
    // Only diagnostic material is available after a failed analysis.
    // INDETERMINATE always blocks regardless of this digest.
    let mut hasher = Sha256::new();
    hasher.update(b"incomplete:");
    if let Ok(bytes) = fs::read(root.join("Cargo.toml")) {
        hasher.update(&bytes);
    }
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    #[test]
    fn declaration_order_is_canonical_across_metadata_order() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/profile")
            .canonicalize()
            .unwrap();
        let output = Command::new("cargo")
            .args([
                "metadata",
                "--offline",
                "--no-deps",
                "--format-version",
                "1",
            ])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(output.status.success());
        let mut metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
        let before = observation_from_metadata(&root, &output.stdout).unwrap();
        for package in metadata["packages"].as_array_mut().unwrap() {
            package["dependencies"].as_array_mut().unwrap().reverse();
        }
        metadata["packages"].as_array_mut().unwrap().reverse();
        let after =
            observation_from_metadata(&root, &serde_json::to_vec(&metadata).unwrap()).unwrap();
        assert_eq!(before.declarations, after.declarations);
        assert_eq!(before.facts, after.facts);
        assert_eq!(before.manifest_digest, after.manifest_digest);
    }
}
