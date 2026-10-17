use crate::bytecode_violation::BytecodeViolation;
use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 严格解析的技术结果；未核验摘要和执行元数据前不能构成通过。对应Java PipelineResult。
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodePipelineResult {
    pub schema_version: u32,
    pub completeness: String,
    pub class_count: u32,
    pub violations: Vec<BytecodeViolation>,
    pub diagnostics: Vec<String>,
    pub source_digest: Option<Digest>,
    pub bytecode_digest: Option<Digest>,
    pub tool_digest: Option<Digest>,
    pub rule_digest: Option<Digest>,
    pub adapter_version: String,
    pub archunit_version: String,
    pub modules: Vec<String>,
}
impl BytecodePipelineResult {
    /// 解析有界、无重复键的唯一JSON；拒绝未覆盖却自报complete。
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("bytecode result exceeds 1 MiB".into());
        }
        let canonical = guard_protocol::canonicalize(bytes)?;
        let result: Self = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
        if result.schema_version != 1
            || result.adapter_version != "archguard-bytecode/1"
            || result.archunit_version != "1.5.1"
            || !["complete", "incomplete"].contains(&result.completeness.as_str())
        {
            return Err("unsupported bytecode result protocol".into());
        }
        if result.violations.len() > 10000
            || result.diagnostics.len() > 1024
            || result.modules.len() > 1024
        {
            return Err("bytecode result collection exceeds budget".into());
        }
        if result.completeness == "complete"
            && (result.class_count == 0
                || result.source_digest.is_none()
                || result.bytecode_digest.is_none()
                || result.tool_digest.is_none()
                || result.rule_digest.is_none()
                || result.modules.is_empty())
        {
            return Err("complete result lacks actual source/bytecode/tool/rule coverage".into());
        }
        Ok(result)
    }
}
