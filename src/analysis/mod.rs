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
    Ok(PreparedAnalysis::prepare(root, profile)?
        .observe(subject)?
        .facts)
}
pub(crate) struct PreparedAnalysis {
    isolated: runner::IsolatedProject,
    tools: runner::Toolchain,
    environment: std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    profile: profile::FrozenAnalysisProfile,
    inventory: snapshot::SnapshotInventory,
}
pub(crate) struct DeclarationAnalysis {
    pub facts: GuardFacts,
    pub observation: cargo::CargoObservation,
}
impl PreparedAnalysis {
    pub fn snapshot_digest(&self) -> String {
        self.inventory.digest()
    }
    pub fn inventory_keys(&self) -> Vec<String> {
        self.inventory.keys().map(str::to_owned).collect()
    }
    pub fn prepare(
        root: &Path,
        profile: &profile::FrozenAnalysisProfile,
    ) -> Result<Self, GuardError> {
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
            if key.starts_with("CARGO_")
                || key.starts_with("RUST")
                || matches!(key, "PATH" | "HOME")
            {
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
        Ok(Self {
            isolated,
            tools,
            environment,
            profile: profile.clone(),
            inventory,
        })
    }
    pub fn observe(self, subject: &str) -> Result<DeclarationAnalysis, GuardError> {
        let metadata = runner::run(
            &self.tools.cargo,
            &[
                "metadata".into(),
                "--no-deps".into(),
                "--offline".into(),
                "--format-version".into(),
                "1".into(),
                "--manifest-path".into(),
                self.isolated.root().join("Cargo.toml").into_os_string(),
            ],
            self.isolated.root(),
            &self.environment,
            &runner::Budget::default(),
        )
        .map_err(|e| GuardError::Input(format!("cargo metadata: {e}")))?;
        let observation = crate::observation_from_metadata(self.isolated.root(), &metadata)
            .map_err(GuardError::Input)?;
        let gaps = self.profile.coverage_gaps(&observation.members);
        let facts = GuardFacts {
            api_version: API_VERSION.into(),
            kind: "GuardFacts".into(),
            analyzer: AnalyzerIdentity {
                id: "archguard.cargo.declarations".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            subject: GuardSubject {
                id: subject.into(),
                snapshot_digest: self.inventory.digest(),
            },
            completeness: if gaps.is_empty() {
                Completeness::Complete
            } else {
                Completeness::Partial
            },
            facts: observation.facts.clone(),
            diagnostics: gaps,
        };
        Ok(DeclarationAnalysis { facts, observation })
    }
}
