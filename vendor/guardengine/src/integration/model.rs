use crate::Decision;
use serde::{Deserialize, Deserializer, Serialize};
fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::deserialize(d)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Producer {
    pub guard: String,
    pub version: String,
    pub analyzer_id: String,
    pub analyzer_version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunBinding {
    pub repo_id: String,
    pub task_id: String,
    pub worktree_id: String,
    pub requirement_ids: Vec<String>,
    pub candidate_oid: String,
    pub base_oid: String,
    #[serde(deserialize_with = "required_nullable")]
    pub merge_group_id: Option<String>,
    pub source_snapshot_digest: String,
    #[serde(deserialize_with = "required_nullable")]
    pub baseline_digest: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CoverageStatus {
    Complete,
    Partial,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Coverage {
    pub status: CoverageStatus,
    pub required_scopes: Vec<String>,
    pub observed_scopes: Vec<String>,
    pub missing_scopes: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRef {
    pub uri: String,
    pub digest: String,
    pub media_type: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Artifacts {
    #[serde(deserialize_with = "required_nullable")]
    pub contract: Option<ArtifactRef>,
    #[serde(deserialize_with = "required_nullable")]
    pub facts: Option<ArtifactRef>,
    #[serde(deserialize_with = "required_nullable")]
    pub report: Option<ArtifactRef>,
    pub domain: Vec<ArtifactRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub source: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Completed,
    Error,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuardRunEnvelope {
    pub api_version: String,
    pub kind: String,
    pub run_id: String,
    pub producer: Producer,
    pub binding: RunBinding,
    pub run_status: RunStatus,
    #[serde(deserialize_with = "required_nullable")]
    pub decision: Option<Decision>,
    pub coverage: Coverage,
    pub artifacts: Artifacts,
    pub approval_refs: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub started_at: String,
    pub finished_at: String,
    #[serde(deserialize_with = "required_nullable")]
    pub expires_at: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransportDiagnostic {
    pub code: String,
    pub message: String,
}
