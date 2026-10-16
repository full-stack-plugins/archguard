/// POM 的本地父模型引用，空 relativePath 明确表示不能本地解析。无 Java 对应类。
#[derive(Clone)]
pub(crate) struct ParentReference {
    pub(crate) group: String,
    pub(crate) artifact: String,
    pub(crate) version: String,
    pub(crate) relative: String,
}
