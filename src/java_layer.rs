use serde::{Deserialize, Serialize};
/// Java 包分层的显式允许表；原生 Rust 契约，无 Java 对应类。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaLayer {
    pub id: String,
    pub allowed_dependencies: Vec<String>,
}
