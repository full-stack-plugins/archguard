use serde::{Deserialize, Serialize};
/// 禁止指定包前缀间的源码引用；无 Java 对应类。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaDependencyRule {
    pub id: String,
    pub from: String,
    pub to: String,
}
