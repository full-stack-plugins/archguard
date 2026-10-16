use serde::{Deserialize, Serialize};
/// 点边界包前缀与模型模块、层的映射；无 Java 对应类。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaPackageRule {
    pub prefix: String,
    pub module: String,
    pub layer: String,
}
