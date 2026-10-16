use crate::dependency_declaration::DependencyDeclaration;
use std::collections::BTreeMap;

/// 在限定本地继承范围内解析的模块模型，非 Maven effective POM。无 Java 对应类。
#[derive(Clone, Default)]
pub(crate) struct ResolvedPom {
    pub(crate) group: String,
    pub(crate) artifact: String,
    pub(crate) version: String,
    pub(crate) properties: BTreeMap<String, String>,
    pub(crate) dependencies: Vec<DependencyDeclaration>,
    pub(crate) managed: Vec<DependencyDeclaration>,
}
