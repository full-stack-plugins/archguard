use guardengine::GuardFact;
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyDeclaration {
    pub member: String,
    pub package: String,
    #[serde(deserialize_with = "required_nullable")]
    pub rename: Option<String>,
    pub kind: String,
    pub optional: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub target: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub dependency_source: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub path: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
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

fn required_nullable<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    serde::Deserialize::deserialize(d)
}
