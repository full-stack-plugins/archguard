/// 一条 Java import 的语法事实及源行号；无 Java 对应类。
pub(crate) struct JavaImport {
    pub(crate) name: String,
    pub(crate) wildcard: bool,
    pub(crate) static_import: bool,
    pub(crate) line: usize,
}
