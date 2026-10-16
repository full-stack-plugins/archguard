use crate::{model_element::ModelElement, model_relation::ModelRelation};
use guardengine::{Completeness, GuardFacts};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// 项目唯一目标模型，领域声明与实际构建事实分离；无 Java 对应类。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureModel {
    pub schema_version: u32,
    pub project: String,
    pub elements: Vec<ModelElement>,
    pub relations: Vec<ModelRelation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<crate::java_policy::JavaPolicy>,
}

pub(crate) fn identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && value != "."
        && value != ".."
}
impl ArchitectureModel {
    /// 校验模型身份、模块映射和关系归属；输入不合法返回诊断，不修正规则。
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.project.trim().is_empty()
            || self.elements.is_empty()
            || self.elements.len() > 10000
            || self.relations.len() > 20000
        {
            return Err("invalid model version, project or model size".into());
        }
        let mut elements = BTreeMap::new();
        let mut implementations = BTreeSet::new();
        for element in &self.elements {
            if !identity(&element.id)
                || element.name.trim().is_empty()
                || element.responsibility.trim().is_empty()
                || elements.insert(&element.id, element).is_some()
            {
                return Err(format!(
                    "invalid or duplicate model element: {}",
                    element.id
                ));
            }
            if ![
                "context",
                "module",
                "capability",
                "aggregate",
                "entity",
                "value_object",
                "interface",
                "event",
            ]
            .contains(&element.kind.as_str())
            {
                return Err(format!("unsupported element kind: {}", element.kind));
            }
            if element.kind == "module" {
                let implementation = element
                    .implementation
                    .as_ref()
                    .filter(|s| !s.trim().is_empty())
                    .ok_or("module implementation required")?;
                if !implementations.insert(implementation) || element.module.is_some() {
                    return Err("duplicate implementation or module nested as domain object".into());
                }
            } else if element.implementation.is_some() {
                return Err("only modules may declare implementation mappings".into());
            }
        }
        if implementations.is_empty() {
            return Err("model must include implementation modules".into());
        }
        for element in &self.elements {
            if element.kind == "context" {
                if element.context.is_some() || element.module.is_some() {
                    return Err("context cannot have parent ownership".into());
                }
                continue;
            }
            let context = element
                .context
                .as_ref()
                .and_then(|id| elements.get(id))
                .filter(|e| e.kind == "context")
                .ok_or("element requires an existing context")?;
            if let Some(module) = &element.module {
                let owner = elements
                    .get(module)
                    .filter(|e| e.kind == "module")
                    .ok_or("unknown module ownership")?;
                if owner.context.as_ref() != Some(&context.id) {
                    return Err("domain object and module contexts differ".into());
                }
            } else if !["module", "capability"].contains(&element.kind.as_str()) {
                return Err("domain object requires module ownership".into());
            }
        }
        let mut ids = BTreeSet::new();
        let mut edges = BTreeSet::new();
        for relation in &self.relations {
            if !identity(&relation.id)
                || !ids.insert(&relation.id)
                || elements.contains_key(&relation.id)
                || !edges.insert((&relation.from, &relation.kind, &relation.to))
            {
                return Err("duplicate or invalid relation identity/edge".into());
            }
            let from = elements
                .get(&relation.from)
                .ok_or("dangling relation source")?;
            let to = elements
                .get(&relation.to)
                .ok_or("dangling relation target")?;
            match relation.kind.as_str() {
                "depends_on" if from.kind == "module" && to.kind == "module" => {}
                "uses" => {}
                "publishes" | "subscribes" if to.kind == "event" && from.kind != "context" => {}
                _ => return Err("unsupported relation kind or endpoint types".into()),
            }
        }
        if let Some(policy) = &self.java {
            policy.validate(self)?;
        } else if self.elements.iter().any(|e| e.java_type.is_some()) {
            return Err("java_type requires a project java policy".into());
        }
        Ok(())
    }

    /// 比对当前完整构建声明与目标模块/依赖集合，返回有源位置的差异；不检查业务语义。
    pub fn compare(&self, facts: &GuardFacts) -> Vec<Value> {
        if facts.completeness != Completeness::Complete || facts.coverage.is_none() {
            return vec![json!({"code":"incomplete_build_analysis","detail":facts.diagnostics})];
        }
        let modules: BTreeMap<_, _> = self
            .elements
            .iter()
            .filter(|e| e.kind == "module")
            .map(|e| (e.id.as_str(), e.implementation.as_deref().unwrap()))
            .collect();
        let expected: BTreeSet<_> = modules.values().copied().collect();
        let actual: BTreeSet<_> = facts
            .coverage
            .as_ref()
            .unwrap()
            .entities
            .iter()
            .map(String::as_str)
            .collect();
        let mut findings = vec![];
        for id in actual.difference(&expected) {
            findings.push(json!({"code":"unmodeled_module","module":id}));
        }
        for id in expected.difference(&actual) {
            findings.push(json!({"code":"missing_module","module":id}));
        }
        let expected_edges: BTreeSet<_> = self
            .relations
            .iter()
            .filter(|r| r.kind == "depends_on")
            .map(|r| (modules[r.from.as_str()], modules[r.to.as_str()]))
            .collect();
        let actual_edges: BTreeSet<_> = facts
            .facts
            .iter()
            .filter(|f| f.predicate == "depends_on")
            .map(|f| (f.subject.as_str(), f.object.as_str()))
            .collect();
        for edge in actual_edges.difference(&expected_edges) {
            let sources: Vec<_> = facts
                .facts
                .iter()
                .filter(|f| {
                    f.subject == edge.0 && f.object == edge.1 && f.predicate == "depends_on"
                })
                .map(|f| &f.source)
                .collect();
            findings.push(
                json!({"code":"unmodeled_dependency","from":edge.0,"to":edge.1,"sources":sources}),
            );
        }
        for edge in expected_edges.difference(&actual_edges) {
            findings.push(json!({"code":"missing_dependency","from":edge.0,"to":edge.1}));
        }
        findings
    }
}
