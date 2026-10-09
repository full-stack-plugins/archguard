pub mod cargo;
pub mod profile;
pub mod runner;
pub mod snapshot;

use guardengine::{
    API_VERSION, AnalyzerIdentity, Completeness, GuardError, GuardFacts, GuardSubject,
};
use std::path::Path;

/// Analyze only declarations; this never asserts active-build or source-level coverage.
pub fn analyze(
    root: &Path,
    subject: &str,
    profile: &profile::FrozenAnalysisProfile,
) -> Result<GuardFacts, GuardError> {
    let root = root
        .canonicalize()
        .map_err(|e| GuardError::Input(e.to_string()))?;
    let budget = runner::Budget::default();
    let isolated = runner::IsolatedProject::copy(&root, std::slice::from_ref(&root), &budget)
        .map_err(|e| GuardError::Input(e.to_string()))?;
    let tools =
        runner::Toolchain::discover(&budget).map_err(|e| GuardError::Input(e.to_string()))?;
    let environment = tools.environment(&isolated.tool_home());
    let mut identity = vec![("profile".into(), profile.identity_bytes())];
    // Declared ambient configuration is bound even when sanitized away. This is
    // conservative invalidation, not a promise to honor active feature builds.
    for (key, value) in std::env::vars_os() {
        let key = key
            .to_str()
            .ok_or_else(|| GuardError::Input("non UTF-8 environment key".into()))?;
        if key.starts_with("CARGO_") || key.starts_with("RUST") || matches!(key, "PATH" | "HOME") {
            let value = value
                .to_str()
                .ok_or_else(|| GuardError::Input("non UTF-8 tool environment".into()))?;
            identity.push((format!("env:{key}"), value.as_bytes().to_vec()));
        }
    }
    for (name, tool) in [("cargo", &tools.cargo), ("rustc", &tools.rustc)] {
        let version = runner::run(
            tool,
            &["--version".into()],
            isolated.root(),
            &environment,
            &budget,
        )
        .map_err(|e| GuardError::Input(format!("tool identity: {e}")))?;
        identity.push((format!("tool:{name}"), version));
        identity.push((
            format!("tool-path:{name}"),
            tool.to_string_lossy().as_bytes().to_vec(),
        ));
    }
    let inventory = snapshot::SnapshotInventory::capture(isolated.root(), identity)
        .map_err(GuardError::Input)?;
    let metadata = runner::run(
        &tools.cargo,
        &[
            "metadata".into(),
            "--no-deps".into(),
            "--offline".into(),
            "--format-version".into(),
            "1".into(),
            "--manifest-path".into(),
            isolated.root().join("Cargo.toml").into_os_string(),
        ],
        isolated.root(),
        &environment,
        &budget,
    )
    .map_err(|e| GuardError::Input(format!("cargo metadata: {e}")))?;
    let observation =
        crate::observation_from_metadata(isolated.root(), &metadata).map_err(GuardError::Input)?;
    let gaps = profile.coverage_gaps(&observation.members);
    Ok(GuardFacts {
        api_version: API_VERSION.into(),
        kind: "GuardFacts".into(),
        analyzer: AnalyzerIdentity {
            id: "archguard.cargo.declarations".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        subject: GuardSubject {
            id: subject.into(),
            snapshot_digest: inventory.digest(),
        },
        completeness: if gaps.is_empty() {
            Completeness::Complete
        } else {
            Completeness::Partial
        },
        facts: observation.facts,
        diagnostics: gaps,
    })
}
