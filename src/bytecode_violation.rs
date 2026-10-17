use serde::{Deserialize, Serialize};
/// ArchUnit实际依赖边与可定位源码。对应Java BytecodeViolation。
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeViolation {
    pub rule_id: String,
    pub from: String,
    pub to: String,
    pub file: String,
    pub line: u32,
    pub description: String,
}
