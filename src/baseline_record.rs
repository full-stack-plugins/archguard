use crate::architecture_model::ArchitectureModel;
use serde::{Deserialize, Serialize};

/// 内容寻址的不可变项目基线版本；摘要不构成签名或人工批准。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineRecord {
    pub schema_version: u32,
    pub revision: u64,
    pub parent: Option<String>,
    pub model: ArchitectureModel,
    pub contract_digest: String,
    pub change_id: Option<String>,
    pub change_digest: Option<String>,
    pub report_digest: String,
}
