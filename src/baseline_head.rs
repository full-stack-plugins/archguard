use serde::{Deserialize, Serialize};

/// 当前模型基线的唯一指针，通过原子替换发布新版本。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaselineHead {
    pub schema_version: u32,
    pub digest: String,
}
