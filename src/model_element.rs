use serde::{Deserialize, Serialize};

/// 项目模型中的稳定身份与职责声明；原生 Rust 模型，无 Java 对应类。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelElement {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub responsibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_type: Option<String>,
}
