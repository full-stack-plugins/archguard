use crate::{bytecode_artifact_ref::BytecodeArtifactRef, bytecode_report::BytecodeReport};
use guard_store::EvidenceStore;
use serde::{Deserialize, Serialize};
/// 独立报告原件与可读投影；外部JSON不能恢复已批准计划。无Java对应类型。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeReceipt {
    pub report: BytecodeReport,
    pub artifact: BytecodeArtifactRef,
}
impl BytecodeReceipt {
    /// 严格读取收据并与真实store原件逐字节摘要、内容核对；失败不可用于复验通过。
    pub fn from_json(bytes: &[u8], store: &EvidenceStore, now_ms: u64) -> Result<Self, String> {
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("receipt exceeds budget".into());
        }
        let canonical = guard_protocol::canonicalize(bytes)?;
        let result: Self = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
        result.verify(store, now_ms)?;
        Ok(result)
    }
    /// 核对不可变报告与双原始流；私有本地store完整性不等同受信证据。
    pub fn verify(&self, store: &EvidenceStore, now_ms: u64) -> Result<(), String> {
        use crate::bytecode_pipeline_request::content_digest;
        let raw = store.read_private(&self.artifact.operation, now_ms)?;
        if raw.len() as u64 != self.artifact.size
            || content_digest(&raw) != self.artifact.digest
            || raw != serde_json::to_vec(&self.report).map_err(|e| e.to_string())?
        {
            return Err("report receipt drift".into());
        }
        if self.report.schema_version != 1
            || self.report.trust != "local_unverified"
            || self.report.approval.is_some()
        {
            return Err("unsupported report trust or approval claim".into());
        }
        for reference in [&self.report.stdout, &self.report.stderr]
            .into_iter()
            .chain(self.report.execution_observation.iter())
        {
            let bytes = store.read_private(&reference.operation, now_ms)?;
            if bytes.len() as u64 != reference.size || content_digest(&bytes) != reference.digest {
                return Err("execution artifact drift".into());
            }
        }
        Ok(())
    }
}
