use crate::{
    bytecode_artifact_ref::BytecodeArtifactRef, bytecode_pipeline_result::BytecodePipelineResult,
};
use guard_protocol::{Digest, ExecutionStatus, UtcTimestamp};
use serde::{Deserialize, Serialize};
/// 专业技术观察与独立原始流引用；本地结果不包含交付批准。无Java对应类型。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeReport {
    pub schema_version: u32,
    pub mode: String,
    pub trust: String,
    pub approval: Option<String>,
    pub status: String,
    pub subject_digest: Digest,
    pub config_digest: Digest,
    pub policy_digest: Digest,
    pub context_digest: Digest,
    pub candidate_kind: String,
    pub selection_scope_digest: Digest,
    pub retain_until_ms: u64,
    pub previous_report: Option<BytecodeArtifactRef>,
    pub stop_reason: String,
    pub execution_status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub started_at: Option<UtcTimestamp>,
    pub finished_at: Option<UtcTimestamp>,
    pub input_drifted: bool,
    pub output_truncated: bool,
    pub diagnostics: Vec<String>,
    pub result: Option<BytecodePipelineResult>,
    pub stdout: BytecodeArtifactRef,
    pub stderr: BytecodeArtifactRef,
    pub execution_observation: Option<BytecodeArtifactRef>,
}
