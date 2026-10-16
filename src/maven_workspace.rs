use crate::{input_snapshot::InputSnapshot, pom_model::PomModel, resolved_pom::ResolvedPom};
use guardengine::GuardFact;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::{Path, PathBuf},
};

/// 有界 reactor 遍历与本地父模型解析状态；无 Java 对应类。
pub(crate) struct MavenWorkspace {
    pub(crate) inputs: InputSnapshot,
    pub(crate) entities: Vec<String>,
    pub(crate) module_roots: BTreeMap<String, PathBuf>,
    models: BTreeMap<PathBuf, PomModel>,
    resolved: BTreeMap<PathBuf, ResolvedPom>,
    scenario: crate::maven_scenario::MavenScenario,
    seen_profiles: BTreeSet<String>,
}
impl MavenWorkspace {
    pub(crate) fn new() -> Self {
        Self {
            inputs: InputSnapshot::default(),
            entities: vec![],
            module_roots: BTreeMap::new(),
            models: BTreeMap::new(),
            resolved: BTreeMap::new(),
            scenario: crate::maven_scenario::MavenScenario::default(),
            seen_profiles: BTreeSet::new(),
        }
    }
    fn model(&mut self, root: &Path, path: &Path) -> Result<(PathBuf, PomModel), String> {
        let (path, text) = self.inputs.read(root, path)?;
        if !self.models.contains_key(&path) {
            let source = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            self.models.insert(
                path.clone(),
                PomModel::parse(
                    &text,
                    &source,
                    &self.scenario.profiles,
                    &mut self.seen_profiles,
                )?,
            );
        }
        Ok((path.clone(), self.models[&path].clone()))
    }
    fn resolve(
        &mut self,
        root: &Path,
        path: &Path,
        visiting: &mut HashSet<PathBuf>,
    ) -> Result<(PathBuf, ResolvedPom), String> {
        let (path, model) = self.model(root, path)?;
        if let Some(resolved) = self.resolved.get(&path) {
            return Ok((path, resolved.clone()));
        }
        if visiting.len() >= 64 || !visiting.insert(path.clone()) {
            return Err("cyclic or excessive parent hierarchy".into());
        }
        let mut result = if let Some(parent) = &model.parent {
            if parent.relative.is_empty() {
                return Err("external parent requires effective-model analysis".into());
            }
            let parent_path = path.parent().unwrap().join(&parent.relative);
            let parent_path = if parent_path.is_dir() {
                parent_path.join("pom.xml")
            } else {
                parent_path
            };
            self.resolve(root, &parent_path, visiting)
                .map_err(|e| format!("unresolved local parent: {e}"))?
                .1
        } else {
            ResolvedPom::default()
        };
        let inherited_group = result.group.clone();
        let inherited_version = result.version.clone();
        let inherited_artifact = result.artifact.clone();
        result.properties.extend(model.properties.clone());
        let group = model.group.as_deref().unwrap_or(&inherited_group);
        let version = model.version.as_deref().unwrap_or(&inherited_version);
        for (key, value) in [
            ("project.groupId", group),
            ("pom.groupId", group),
            ("project.artifactId", model.artifact.as_str()),
            ("pom.artifactId", model.artifact.as_str()),
            ("project.version", version),
            ("pom.version", version),
            ("project.parent.groupId", inherited_group.as_str()),
            ("project.parent.version", inherited_version.as_str()),
        ] {
            result.properties.insert(key.into(), value.into());
        }
        result.group = expand(group, &result.properties)?;
        result.artifact = expand(&model.artifact, &result.properties)?;
        result.version = expand(version, &result.properties)?;
        if result.group.is_empty() || result.artifact.is_empty() || result.version.is_empty() {
            return Err("incomplete Maven coordinates".into());
        }
        if let Some(parent) = &model.parent {
            if expand(&parent.group, &result.properties)? != inherited_group
                || expand(&parent.artifact, &result.properties)? != inherited_artifact
                || expand(&parent.version, &result.properties)? != inherited_version
            {
                return Err("parent coordinates do not match local parent model".into());
            }
        }
        result.dependencies.extend(model.dependencies);
        result.managed.extend(model.managed);
        visiting.remove(&path);
        self.resolved.insert(path.clone(), result.clone());
        Ok((path, result))
    }
    pub(crate) fn collect(&mut self, root: &Path) -> Result<Vec<GuardFact>, String> {
        let scenario_path = root.join(".archguard/maven.json");
        if !std::fs::symlink_metadata(&scenario_path)
            .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
        {
            let (_, bytes) = self.inputs.read(root, &scenario_path)?;
            self.scenario =
                serde_json::from_str(&bytes).map_err(|e| format!("invalid Maven scenario: {e}"))?;
        }
        for file in [
            ".mvn/maven.config",
            ".mvn/jvm.config",
            ".mvn/extensions.xml",
        ] {
            if root.join(file).exists() {
                let (_, text) = self.inputs.read(root, &root.join(file))?;
                if file == ".mvn/jvm.config" && text.split_whitespace().all(memory_option) {
                    continue;
                }
                return Err(format!("{file} requires effective-model analysis"));
            }
        }
        let mut pending = vec![(root.join("pom.xml"), 0)];
        let mut seen = BTreeSet::new();
        let mut reactor = BTreeMap::new();
        while let Some((path, depth)) = pending.pop() {
            if depth > 64 || seen.len() >= 1024 {
                return Err("reactor traversal budget exceeded".into());
            }
            let (path, model) = self.resolve(root, &path, &mut HashSet::new())?;
            if !seen.insert(path.clone()) {
                return Err("duplicate or cyclic reactor module declaration".into());
            }
            let id = format!("{}:{}", model.group, model.artifact);
            if reactor.insert(id.clone(), model.clone()).is_some() {
                return Err(format!("ambiguous reactor coordinate: {id}"));
            }
            self.module_roots
                .insert(id.clone(), path.parent().unwrap().to_path_buf());
            self.entities.push(id);
            for module in self.models[&path].modules.clone() {
                let module = expand(&module, &model.properties)?;
                let child = path.parent().unwrap().join(module);
                pending.push((
                    if child.is_file() {
                        child
                    } else {
                        child.join("pom.xml")
                    },
                    depth + 1,
                ));
            }
        }
        self.entities.sort();
        if self
            .scenario
            .profiles
            .keys()
            .any(|id| !self.seen_profiles.contains(id))
        {
            return Err("Maven scenario references unknown profile ID".into());
        }
        let mut facts = vec![];
        for (subject, model) in &reactor {
            let mut managed = BTreeMap::new();
            for declaration in &model.managed {
                let coordinate = format!(
                    "{}:{}",
                    expand(&declaration.group, &model.properties)?,
                    expand(&declaration.artifact, &model.properties)?
                );
                if let Some(version) = &declaration.version {
                    managed.insert(coordinate, version);
                }
            }
            let mut dependencies = BTreeMap::new();
            for dep in &model.dependencies {
                let object = format!(
                    "{}:{}",
                    expand(&dep.group, &model.properties)?,
                    expand(&dep.artifact, &model.properties)?
                );
                dependencies.insert(object, dep);
            }
            for (object, dep) in dependencies {
                if let Some(target) = reactor.get(&object) {
                    let version = dep
                        .version
                        .as_ref()
                        .or_else(|| managed.get(&object).copied())
                        .ok_or_else(|| {
                            format!("{object}: unresolved managed dependency version")
                        })?;
                    if expand(version, &model.properties)? != target.version {
                        return Err(format!(
                            "{object}: dependency version differs from reactor member; effective resolution required"
                        ));
                    }
                    facts.push(GuardFact {
                        subject: subject.clone(),
                        predicate: "depends_on".into(),
                        object,
                        source: dep.source.clone(),
                    });
                }
            }
        }
        facts.sort();
        facts.dedup();
        self.inputs.unchanged(root)?;
        Ok(facts)
    }
}
fn memory_option(value: &str) -> bool {
    let Some(size) = ["-Xmx", "-Xms", "-Xss"]
        .iter()
        .find_map(|prefix| value.strip_prefix(prefix))
    else {
        return false;
    };
    let digits = size
        .strip_suffix(['k', 'K', 'm', 'M', 'g', 'G'])
        .unwrap_or(size);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}
pub(crate) fn expand(value: &str, properties: &BTreeMap<String, String>) -> Result<String, String> {
    let mut result = value.to_owned();
    let mut seen = HashSet::new();
    for _ in 0..32 {
        if result.len() > 65536 || !seen.insert(result.clone()) {
            return Err("cyclic or excessive Maven property expansion".into());
        }
        let Some(start) = result.find("${") else {
            return Ok(result);
        };
        let end = result[start..]
            .find('}')
            .map(|offset| start + offset)
            .ok_or("unterminated Maven property")?;
        let key = &result[start + 2..end];
        let replacement = properties
            .get(key)
            .ok_or_else(|| format!("unresolved Maven property: {key}"))?;
        result.replace_range(start..=end, replacement);
    }
    Err("Maven property expansion depth exceeded".into())
}
