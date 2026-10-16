use crate::java_import::JavaImport;
use std::collections::BTreeMap;
use tree_sitter::{Node, Parser};

/// 单个 Java 编译单元的包、声明和显式引用；无 Java 对应类。
pub(crate) struct JavaUnit {
    pub(crate) package: String,
    pub(crate) module: String,
    pub(crate) file: String,
    pub(crate) types: BTreeMap<String, usize>,
    pub(crate) imports: Vec<JavaImport>,
    pub(crate) references: Vec<(String, usize)>,
}
fn name(node: Node<'_>, text: &str) -> String {
    let mut parts = vec![];
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if matches!(node.kind(), "identifier" | "type_identifier") {
            parts.push(node.utf8_text(text.as_bytes()).unwrap_or(""));
        } else {
            let mut cursor = node.walk();
            let children: Vec<_> = node.named_children(&mut cursor).collect();
            pending.extend(children.into_iter().rev());
        }
    }
    parts.join(".")
}
fn is_type(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "annotation_type_declaration"
    )
}
impl JavaUnit {
    pub(crate) fn parse(module: String, file: String, text: &str) -> Result<Self, String> {
        // 首个可生效 Unicode 转义之前，连续反斜杠为奇数时才具备转义资格。
        // 偶数反斜杠后的 u 是普通内容，如 Java 字符串中的 Windows users 路径。
        let mut slashes = 0;
        let unicode_escape = text.bytes().any(|byte| {
            if byte == b'\\' {
                slashes += 1;
                false
            } else {
                let eligible = byte == b'u' && slashes % 2 == 1;
                slashes = 0;
                eligible
            }
        });
        if unicode_escape {
            return Err(format!(
                "{file}: Java Unicode escapes require compiler analysis"
            ));
        }
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .map_err(|e| e.to_string())?;
        let tree = parser.parse(text, None).ok_or("Java parser failed")?;
        let root = tree.root_node();
        if root.has_error() {
            return Err(format!("{file}: Java syntax error"));
        }
        // 在展开限定名之前限制语法树深度，避免深层字段链产生平方级字符串复制。
        let mut pending = vec![(root, 0)];
        let mut nodes = 0;
        while let Some((node, depth)) = pending.pop() {
            nodes += 1;
            if nodes > 200000 || depth > 256 {
                return Err("Java syntax tree budget exceeded".into());
            }
            let mut cursor = node.walk();
            pending.extend(
                node.named_children(&mut cursor)
                    .map(|child| (child, depth + 1)),
            );
        }
        let mut unit = Self {
            package: String::new(),
            module,
            file,
            types: BTreeMap::new(),
            imports: vec![],
            references: vec![],
        };
        let mut cursor = root.walk();
        for node in root.named_children(&mut cursor) {
            match node.kind() {
                "module_declaration" => return Err("JPMS source requires compiler analysis".into()),
                "package_declaration" => {
                    if !unit.package.is_empty() {
                        return Err("multiple Java package declarations".into());
                    }
                    let mut c = node.walk();
                    let part = node
                        .named_children(&mut c)
                        .find(|n| matches!(n.kind(), "identifier" | "scoped_identifier"))
                        .ok_or("Java package name missing")?;
                    unit.package = name(part, text);
                }
                "import_declaration" => {
                    let mut c = node.walk();
                    let children: Vec<_> = node.children(&mut c).collect();
                    let part = children
                        .iter()
                        .find(|n| matches!(n.kind(), "identifier" | "scoped_identifier"))
                        .ok_or("Java import name missing")?;
                    unit.imports.push(JavaImport {
                        name: name(*part, text),
                        wildcard: children
                            .iter()
                            .any(|n| n.kind() == "asterisk" || n.kind() == "*"),
                        static_import: children.iter().any(|n| n.kind() == "static"),
                        line: node.start_position().row + 1,
                    });
                }
                _ => {}
            }
        }
        if !crate::java_policy::qualified(&unit.package) {
            return Err(format!("{}: explicit Java package required", unit.file));
        }
        // 只收集顶层和成员类型，局部/匿名类不会冒充领域对象的全限定声明。
        let mut cursor = root.walk();
        let mut declarations: Vec<_> = root
            .named_children(&mut cursor)
            .filter(|n| is_type(n.kind()))
            .map(|n| (n, unit.package.clone()))
            .collect();
        while let Some((node, parent)) = declarations.pop() {
            let local = node
                .child_by_field_name("name")
                .ok_or("Java type name missing")?
                .utf8_text(text.as_bytes())
                .map_err(|e| e.to_string())?;
            let full = format!("{parent}.{local}");
            if unit
                .types
                .insert(full.clone(), node.start_position().row + 1)
                .is_some()
            {
                return Err("duplicate Java type declaration".into());
            }
            if let Some(body) = node.child_by_field_name("body") {
                let mut c = body.walk();
                for member in body.named_children(&mut c) {
                    if is_type(member.kind()) {
                        declarations.push((member, full.clone()));
                    }
                    if member.kind() == "enum_body_declarations" {
                        let mut c = member.walk();
                        declarations.extend(
                            member
                                .named_children(&mut c)
                                .filter(|n| is_type(n.kind()))
                                .map(|n| (n, full.clone())),
                        );
                    }
                }
            }
        }
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if node.kind() == "package_declaration" {
                let mut cursor = node.walk();
                pending.extend(
                    node.named_children(&mut cursor)
                        .filter(|n| matches!(n.kind(), "annotation" | "marker_annotation")),
                );
                continue;
            }
            if matches!(
                node.kind(),
                "package_declaration"
                    | "import_declaration"
                    | "line_comment"
                    | "block_comment"
                    | "string_literal"
                    | "character_literal"
            ) {
                continue;
            }
            if matches!(
                node.kind(),
                "scoped_type_identifier" | "scoped_identifier" | "field_access"
            ) {
                unit.references
                    .push((name(node, text), node.start_position().row + 1));
            }
            let mut c = node.walk();
            pending.extend(node.named_children(&mut c));
        }
        unit.references.sort();
        unit.references.dedup();
        Ok(unit)
    }
}
