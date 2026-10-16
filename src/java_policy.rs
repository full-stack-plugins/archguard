use crate::{
    architecture_model::{ArchitectureModel, identity},
    java_dependency_rule::JavaDependencyRule,
    java_layer::JavaLayer,
    java_package_rule::JavaPackageRule,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 纳入项目基线的 Java 源码规则；无 Java 对应类。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JavaPolicy {
    pub packages: Vec<JavaPackageRule>,
    pub layers: Vec<JavaLayer>,
    pub forbidden_dependencies: Vec<JavaDependencyRule>,
    #[serde(default)]
    pub include_tests: bool,
}
pub(crate) fn qualified(value: &str) -> bool {
    value.split('.').all(|part| {
        let mut chars = part.chars();
        chars
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
            && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    })
}
pub(crate) fn within(name: &str, prefix: &str) -> bool {
    name == prefix
        || name
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('.'))
}
impl JavaPolicy {
    pub(crate) fn package(&self, name: &str) -> Option<&JavaPackageRule> {
        self.packages
            .iter()
            .filter(|p| within(name, &p.prefix))
            .max_by_key(|p| p.prefix.len())
    }
    pub(crate) fn validate(&self, model: &ArchitectureModel) -> Result<(), String> {
        if self.packages.is_empty()
            || self.layers.is_empty()
            || self.packages.len() > 1024
            || self.layers.len() > 128
            || self.forbidden_dependencies.len() > 1024
        {
            return Err("Java policy requires bounded nonempty packages and layers".into());
        }
        let mut layers = BTreeSet::new();
        for layer in &self.layers {
            if !identity(&layer.id) || !layers.insert(&layer.id) {
                return Err("invalid or duplicate Java layer".into());
            }
        }
        for layer in &self.layers {
            let allowed: BTreeSet<_> = layer.allowed_dependencies.iter().collect();
            if allowed.len() != layer.allowed_dependencies.len()
                || allowed.iter().any(|id| !layers.contains(id))
            {
                return Err("unknown or duplicate allowed Java layer".into());
            }
        }
        let mut prefixes = BTreeSet::new();
        for rule in &self.packages {
            if !qualified(&rule.prefix)
                || !prefixes.insert(&rule.prefix)
                || !layers.contains(&rule.layer)
                || !model
                    .elements
                    .iter()
                    .any(|e| e.id == rule.module && e.kind == "module")
            {
                return Err("invalid Java package mapping".into());
            }
        }
        let mut ids = BTreeSet::new();
        for rule in &self.forbidden_dependencies {
            if !identity(&rule.id)
                || !ids.insert(&rule.id)
                || !qualified(&rule.from)
                || !qualified(&rule.to)
            {
                return Err("invalid Java forbidden dependency rule".into());
            }
        }
        let mut types = BTreeSet::new();
        for element in &model.elements {
            let domain = ["aggregate", "entity", "value_object", "interface", "event"]
                .contains(&element.kind.as_str());
            if domain {
                let name = element
                    .java_type
                    .as_deref()
                    .filter(|s| qualified(s) && s.contains('.'))
                    .ok_or("Java domain object requires qualified java_type")?;
                if !types.insert(name) {
                    return Err("duplicate Java domain type mapping".into());
                }
                let package = name.rsplit_once('.').unwrap().0;
                let mapping = self
                    .package(package)
                    .ok_or("domain type has no Java package mapping")?;
                if Some(&mapping.module) != element.module.as_ref() {
                    return Err("domain type package and model module ownership differ".into());
                }
            } else if element.java_type.is_some() {
                return Err("java_type only maps domain objects".into());
            }
        }
        Ok(())
    }
}
