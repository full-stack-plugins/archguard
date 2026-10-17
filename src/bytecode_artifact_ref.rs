use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 私有store原件引用；必须回读内容核验，不作为签名或可信标签。无Java对应类型。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeArtifactRef {
    pub operation: String,
    pub digest: Digest,
    pub size: u64,
}
