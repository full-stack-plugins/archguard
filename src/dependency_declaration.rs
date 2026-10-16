/// Maven 声明依赖及原始行号；不表示经过 Maven 解析后的运行时依赖。无 Java 对应类。
#[derive(Clone)]
pub(crate) struct DependencyDeclaration {
    pub(crate) group: String,
    pub(crate) artifact: String,
    pub(crate) version: Option<String>,
    pub(crate) source: String,
}
