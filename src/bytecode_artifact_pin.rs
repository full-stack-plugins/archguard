use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 原生工具原件的显式内容pin；对应Java ArtifactPin，不证明发布者身份。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeArtifactPin {
    pub path: String,
    pub sha256: Digest,
}
