use crate::{dependency_declaration::DependencyDeclaration, parent_reference::ParentReference};
use roxmltree::Node;
use std::collections::{BTreeMap, BTreeSet};

/// POM 的受支持静态声明；XML 为输入格式，不执行 Maven 插件。无 Java 对应类。
#[derive(Clone)]
pub(crate) struct PomModel {
    pub(crate) group: Option<String>,
    pub(crate) artifact: String,
    pub(crate) version: Option<String>,
    pub(crate) parent: Option<ParentReference>,
    pub(crate) properties: BTreeMap<String, String>,
    pub(crate) dependencies: Vec<DependencyDeclaration>,
    pub(crate) managed: Vec<DependencyDeclaration>,
    pub(crate) modules: Vec<String>,
}
impl PomModel {
    pub(crate) fn parse(
        text: &str,
        source: &str,
        choices: &BTreeMap<String, bool>,
        seen: &mut BTreeSet<String>,
    ) -> Result<Self, String> {
        let document = roxmltree::Document::parse(text)
            .map_err(|e| format!("{source}: invalid POM XML: {e}"))?;
        let root = document.root_element();
        if root.tag_name().name() != "project"
            || !matches!(
                root.tag_name().namespace(),
                None | Some(
                    "http://maven.apache.org/POM/4.0.0" | "http://maven.apache.org/POM/4.1.0"
                )
            )
        {
            return Err(format!("{source}: invalid Maven project namespace"));
        }
        // 结构化模型不接受异命名空间字段，插件 configuration 内部数据不参与架构提取。
        for node in root.descendants().filter(Node::is_element) {
            if !node
                .ancestors()
                .any(|a| a.tag_name().name() == "configuration")
                && node.tag_name().namespace() != root.tag_name().namespace()
            {
                return Err(format!("{source}: mixed model namespaces are unsupported"));
            }
        }
        if !matches!(
            value(root, "modelVersion")?.as_deref(),
            Some("4.0.0" | "4.1.0")
        ) {
            return Err(format!("{source}: unsupported Maven modelVersion"));
        }
        let mut layers = vec![root];
        let mut local_ids = BTreeSet::new();
        if let Some(profiles) = single(root, "profiles")? {
            for profile in profiles.children().filter(Node::is_element) {
                if profile.tag_name().name() != "profile" {
                    return Err("unsupported profile declaration".into());
                }
                let id = required(profile, "id")?;
                if !local_ids.insert(id.clone()) {
                    return Err("duplicate Maven profile ID".into());
                }
                seen.insert(id.clone());
                let affecting = profile.children().any(|child| {
                    matches!(
                        child.tag_name().name(),
                        "dependencies"
                            | "dependencyManagement"
                            | "modules"
                            | "subprojects"
                            | "properties"
                    )
                });
                if affecting && !choices.contains_key(&id) {
                    return Err(format!(
                        "{source}: model-affecting profile {id} requires explicit selection in .archguard/maven.json; automatic activation requires effective-model analysis"
                    ));
                }
                if choices.get(&id) == Some(&true) {
                    layers.push(profile);
                }
            }
        }
        let parent = single(root, "parent")?
            .map(|p| -> Result<ParentReference, String> {
                Ok(ParentReference {
                    group: required(p, "groupId")?,
                    artifact: required(p, "artifactId")?,
                    version: required(p, "version")?,
                    relative: value(p, "relativePath")?.unwrap_or_else(|| "../pom.xml".into()),
                })
            })
            .transpose()?;
        let mut properties = BTreeMap::new();
        for layer in &layers {
            let mut local_properties = BTreeSet::new();
            if let Some(node) = single(*layer, "properties")? {
                for child in node.children().filter(Node::is_element) {
                    if !local_properties.insert(child.tag_name().name()) {
                        return Err(format!("{source}: duplicate property"));
                    }
                    properties.insert(
                        child.tag_name().name().into(),
                        child.text().unwrap_or("").trim().into(),
                    );
                }
            }
        }
        let mut modules = vec![];
        for layer in &layers {
            for (container, tag) in [("modules", "module"), ("subprojects", "subproject")] {
                if let Some(node) = single(*layer, container)? {
                    for child in node.children().filter(Node::is_element) {
                        if child.tag_name().name() != tag {
                            return Err(format!("{source}: unsupported module declaration"));
                        }
                        let name = child.text().unwrap_or("").trim();
                        if name.is_empty() {
                            return Err(format!("{source}: empty module"));
                        }
                        modules.push(name.into());
                    }
                }
            }
        }
        let mut managed = vec![];
        let mut declared = vec![];
        for layer in layers {
            if let Some(node) = single(layer, "dependencyManagement")? {
                managed.extend(dependencies(node, source)?);
            }
            declared.extend(dependencies(layer, source)?);
        }
        Ok(Self {
            group: value(root, "groupId")?,
            artifact: required(root, "artifactId")?,
            version: value(root, "version")?,
            parent,
            properties,
            dependencies: declared,
            managed,
            modules,
        })
    }
}
fn single<'a, 'input>(
    node: Node<'a, 'input>,
    name: &str,
) -> Result<Option<Node<'a, 'input>>, String> {
    let mut matches = node
        .children()
        .filter(|child| child.is_element() && child.tag_name().name() == name);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(format!("duplicate POM element {name}"));
    }
    Ok(first)
}
fn value(node: Node<'_, '_>, name: &str) -> Result<Option<String>, String> {
    Ok(single(node, name)?.map(|n| n.text().unwrap_or("").trim().into()))
}
fn required(node: Node<'_, '_>, name: &str) -> Result<String, String> {
    value(node, name)?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("POM missing {name}"))
}
fn dependencies(node: Node<'_, '_>, source: &str) -> Result<Vec<DependencyDeclaration>, String> {
    let mut result = vec![];
    if let Some(parent) = single(node, "dependencies")? {
        for dep in parent.children().filter(Node::is_element) {
            if dep.tag_name().name() != "dependency" {
                return Err("unsupported dependency declaration".into());
            }
            result.push(DependencyDeclaration {
                group: required(dep, "groupId")?,
                artifact: required(dep, "artifactId")?,
                version: value(dep, "version")?,
                source: format!(
                    "{source}:{}",
                    dep.document().text_pos_at(dep.range().start).row
                ),
            });
        }
    }
    Ok(result)
}
