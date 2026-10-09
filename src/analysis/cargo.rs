use guardengine::GuardFact;
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DependencyDeclaration {
    pub member: String,
    pub package: String,
    pub rename: Option<String>,
    pub kind: String,
    pub optional: bool,
    pub target: Option<String>,
    pub dependency_source: Option<String>,
    pub path: Option<String>,
    pub member_target: Option<String>,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct CargoObservation {
    pub members: BTreeSet<String>,
    pub declarations: Vec<DependencyDeclaration>,
    pub facts: Vec<GuardFact>,
    pub manifest_digest: String,
}

pub fn observe(root: &Path) -> Result<CargoObservation, String> {
    crate::collect_workspace_observation(&root.canonicalize().map_err(|e| e.to_string())?)
}
