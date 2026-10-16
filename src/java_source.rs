use crate::{
    architecture_model::ArchitectureModel,
    input_snapshot::safe_file,
    java_policy::{JavaPolicy, within},
    java_unit::JavaUnit,
    maven_workspace::MavenWorkspace,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// 检查模型启用的 Java 源码范围；失败返回显式不完整结果，不降级为通过。
pub(crate) fn check(root: &Path, model: &ArchitectureModel, analyzer: &str) -> Option<Value> {
    let policy = model.java.as_ref()?;
    Some(match analyze(root, model, policy, analyzer) {
        Ok(report) => report,
        Err(error) => {
            json!({"complete":false,"findings":[{"code":"java_analysis_incomplete","detail":error}],"edges":[],"scope":"Java syntax; analysis incomplete"})
        }
    })
}
fn inventory(
    root: &Path,
    modules: &BTreeMap<String, PathBuf>,
    tests: bool,
) -> Result<BTreeMap<PathBuf, String>, String> {
    let mut files = BTreeMap::new();
    let mut directories = 0;
    for (module, directory) in modules {
        for relative in ["src/main/java", "src/test/java"] {
            if relative == "src/test/java" && !tests {
                continue;
            }
            let source = directory.join(relative);
            // 检查已存在祖先，断链或重解析点不会被误当作无源码根。
            for ancestor in source.ancestors().take_while(|p| p.starts_with(root)) {
                if fs::symlink_metadata(ancestor).is_ok() {
                    safe_file(root, ancestor)?;
                }
            }
            if !source.exists() {
                continue;
            }
            let mut pending = vec![(source, 0)];
            while let Some((path, depth)) = pending.pop() {
                directories += 1;
                if depth > 64 || directories > 8192 {
                    return Err("Java directory budget exceeded".into());
                }
                safe_file(root, &path)?;
                for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
                    let path = entry.map_err(|e| e.to_string())?.path();
                    safe_file(root, &path)?;
                    if path.is_dir() {
                        pending.push((path, depth + 1));
                    } else if path.extension().is_some_and(|e| e == "java")
                        && (files.len() >= 1024 || files.insert(path, module.clone()).is_some())
                    {
                        return Err("Java file budget or overlapping roots".into());
                    }
                }
            }
        }
        for generated in ["target/generated-sources", "target/generated-test-sources"] {
            let path = directory.join(generated);
            if path.exists()
                && fs::read_dir(&path)
                    .map_err(|e| e.to_string())?
                    .next()
                    .is_some()
            {
                return Err("generated Java sources require compiler analysis".into());
            }
        }
    }
    Ok(files)
}
fn local_type<'a>(
    name: &str,
    types: &'a BTreeMap<String, (usize, usize)>,
) -> Option<(&'a String, &'a (usize, usize))> {
    // 最长类型前缀覆盖静态成员和嵌套类型，不把同名类的简单名字当作全限定名。
    let mut prefix = name;
    loop {
        if let Some(entry) = types.get_key_value(prefix) {
            return Some(entry);
        }
        prefix = prefix.rsplit_once('.')?.0;
    }
}
fn analyze(
    root: &Path,
    model: &ArchitectureModel,
    policy: &JavaPolicy,
    analyzer: &str,
) -> Result<Value, String> {
    if analyzer == "cargo" || !root.join("pom.xml").is_file() {
        return Err("Java policy requires Maven analysis".into());
    }
    let mut workspace = MavenWorkspace::new();
    workspace.collect(root)?;
    // 不根据不完整构建配置猜测源码目录或执行生成器。
    for (path, bytes) in &workspace.inputs.files {
        if path.file_name().is_none_or(|n| n != "pom.xml") {
            continue;
        }
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        let document = roxmltree::Document::parse(text).map_err(|e| e.to_string())?;
        if document.descendants().any(|n| {
            matches!(
                n.tag_name().name(),
                "sourceDirectory" | "testSourceDirectory" | "sources"
            ) || (n.tag_name().name() == "artifactId"
                && matches!(
                    n.text(),
                    Some("build-helper-maven-plugin" | "maven-processor-plugin")
                ))
        }) {
            return Err("custom or generated Java source roots require compiler analysis".into());
        }
    }
    let files = inventory(root, &workspace.module_roots, policy.include_tests)?;
    if files.is_empty() {
        return Err("Java policy enabled but no Java source files found".into());
    }
    let modules: BTreeMap<_, _> = model
        .elements
        .iter()
        .filter(|e| e.kind == "module")
        .map(|e| (e.implementation.as_deref().unwrap(), e.id.as_str()))
        .collect();
    let mut units = vec![];
    for (path, coordinate) in &files {
        let (_, text) = workspace.inputs.read(root, path)?;
        let module = modules
            .get(coordinate.as_str())
            .ok_or("Java reactor module missing in project model")?;
        let file = path
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        units.push(JavaUnit::parse((*module).into(), file, &text)?);
    }
    let mut types = BTreeMap::new();
    let mut findings = vec![];
    for (index, unit) in units.iter().enumerate() {
        match policy.package(&unit.package) {
            None=>findings.push(json!({"code":"java_unmapped_package","package":unit.package,"file":unit.file,"line":1})),
            Some(mapping) if mapping.module!=unit.module=>findings.push(json!({"code":"java_package_owner_mismatch","package":unit.package,"expected_module":mapping.module,"actual_module":unit.module,"file":unit.file,"line":1})),
            _=>{}
        }
        for (name, line) in &unit.types {
            if types.insert(name.clone(), (index, *line)).is_some() {
                return Err(format!("duplicate Java type: {name}"));
            }
        }
    }
    let mut edges = BTreeSet::new();
    for (index, unit) in units.iter().enumerate() {
        for import in &unit.imports {
            let target = if let Some((_, (target, _))) = local_type(&import.name, &types) {
                units[*target].package.clone()
            } else if import.wildcard && !import.static_import {
                if policy.package(&import.name).is_some()
                    && !units.iter().any(|u| u.package == import.name)
                {
                    return Err(format!(
                        "{}:{} unresolved internal wildcard import {}",
                        unit.file, import.line, import.name
                    ));
                }
                import.name.clone()
            } else {
                if policy.package(&import.name).is_some() {
                    return Err(format!(
                        "{}:{} unresolved internal import {}",
                        unit.file, import.line, import.name
                    ));
                }
                let name = if import.static_import && !import.wildcard {
                    import
                        .name
                        .rsplit_once('.')
                        .ok_or("invalid static import")?
                        .0
                } else {
                    &import.name
                };
                name.rsplit_once('.')
                    .ok_or("unqualified Java import")?
                    .0
                    .to_owned()
            };
            if target != unit.package {
                edges.insert((index, target, import.line));
            }
        }
        for (reference, line) in &unit.references {
            // 外部全限定引用也服从显式禁止前缀；同名变量遮蔽保守视为引用。
            for rule in &policy.forbidden_dependencies {
                if reference != &rule.to && within(reference, &rule.to) {
                    edges.insert((index, rule.to.clone(), *line));
                }
            }
            if let Some((_, (target, _))) = local_type(reference, &types) {
                if units[*target].package != unit.package {
                    edges.insert((index, units[*target].package.clone(), *line));
                }
            } else if policy.package(reference).is_some()
                && !units.iter().any(|u| u.package == *reference)
            {
                // 内部全限定引用不允许因缺少声明而静默消失。
                return Err(format!(
                    "{}:{line} unresolved internal reference {reference}",
                    unit.file
                ));
            }
        }
    }
    let mut serialized_edges = vec![];
    for (index, to, line) in edges {
        let unit = &units[index];
        serialized_edges.push(json!({"from":unit.package,"to":to,"file":unit.file,"line":line}));
        if let (Some(from), Some(target)) = (policy.package(&unit.package), policy.package(&to)) {
            let layer = policy.layers.iter().find(|l| l.id == from.layer).unwrap();
            if from.layer != target.layer && !layer.allowed_dependencies.contains(&target.layer) {
                findings.push(json!({"code":"java_layer_violation","from":unit.package,"to":to,"from_layer":from.layer,"to_layer":target.layer,"file":unit.file,"line":line}));
            }
        }
        for rule in &policy.forbidden_dependencies {
            if within(&unit.package, &rule.from) && within(&to, &rule.to) {
                findings.push(json!({"code":"java_forbidden_dependency","rule":rule.id,"from":unit.package,"to":to,"file":unit.file,"line":line}));
            }
        }
    }
    for element in &model.elements {
        if let Some(name) = &element.java_type {
            match types.get(name) {
                None=>findings.push(json!({"code":"java_missing_domain_type","element":element.id,"java_type":name})),
                Some((index,line))=>{
                    let unit=&units[*index];
                    if element.module.as_ref()!=Some(&unit.module) {
                        findings.push(json!({"code":"java_domain_owner_mismatch","element":element.id,"java_type":name,"expected_module":element.module,"actual_module":unit.module,"file":unit.file,"line":line}));
                    }
                }
            }
        }
    }
    workspace.inputs.unchanged(root)?;
    if files != inventory(root, &workspace.module_roots, policy.include_tests)? {
        return Err("Java source inventory changed during analysis".into());
    }
    let roots:Vec<_>=workspace.module_roots.iter().map(|(module,path)|json!({"module":module,"path":path.strip_prefix(root).unwrap().to_string_lossy().replace('\\',"/")})).collect();
    let files: Vec<_> = units.iter().map(|u| &u.file).collect();
    Ok(
        json!({"complete":true,"analyzer":"tree-sitter-java-0.23.5","version":env!("CARGO_PKG_VERSION"),"source_digest":workspace.inputs.digest(root),"files":files,"modules":roots,"include_tests":policy.include_tests,"scope":"Maven src/main/java and optionally src/test/java; import declarations and explicit known-project qualified references; conservative wildcard and forbidden external prefix edges","excluded":["compiler type inference","runtime calls and reflection","generated/custom roots","domain business invariants","unconfigured external qualified references without imports"],"edges":serialized_edges,"findings":findings}),
    )
}
