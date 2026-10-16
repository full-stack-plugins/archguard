use crate::{
    architecture_model::{ArchitectureModel, identity},
    model_element::ModelElement,
    model_relation::ModelRelation,
    requirement_reference::RequirementReference,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// 一个需求相对于指定项目基线的增量，不形成第二套完整架构。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelChange {
    pub schema_version: u32,
    pub id: String,
    pub base_digest: String,
    pub requirement: RequirementReference,
    pub upsert_elements: Vec<ModelElement>,
    pub remove_elements: Vec<String>,
    pub upsert_relations: Vec<ModelRelation>,
    pub remove_relations: Vec<String>,
    pub touched_modules: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java: Option<crate::java_policy::JavaPolicy>,
}
impl ModelChange {
    /// 在给定基线上形成候选模型并返回直接受影响的稳定 ID；不修改文件。
    pub fn apply(
        &self,
        base: &ArchitectureModel,
    ) -> Result<(ArchitectureModel, BTreeSet<String>), String> {
        if self.schema_version != 1 || !identity(&self.id) {
            return Err("invalid change version or ID".into());
        }
        let mut candidate = base.clone();
        let mut seeds = BTreeSet::new();
        let mut seen = BTreeSet::new();
        for id in &self.remove_relations {
            if !seen.insert(id) {
                return Err("duplicate relation removal".into());
            }
            let old = base
                .relations
                .iter()
                .find(|r| &r.id == id)
                .ok_or("cannot remove unknown relation")?;
            seeds.extend([old.from.clone(), old.to.clone()]);
            candidate.relations.retain(|r| &r.id != id);
        }
        for relation in &self.upsert_relations {
            if !seen.insert(&relation.id) {
                return Err("conflicting relation operations".into());
            }
            if let Some(old) = base.relations.iter().find(|r| r.id == relation.id) {
                if old != relation {
                    seeds.extend([old.from.clone(), old.to.clone()]);
                } else {
                    continue;
                }
            }
            seeds.extend([relation.from.clone(), relation.to.clone()]);
            candidate.relations.retain(|r| r.id != relation.id);
            candidate.relations.push(relation.clone());
        }
        seen.clear();
        for id in &self.remove_elements {
            if !seen.insert(id) || !base.elements.iter().any(|e| &e.id == id) {
                return Err("duplicate or unknown element removal".into());
            }
            seeds.insert(id.clone());
            candidate.elements.retain(|e| &e.id != id);
        }
        for element in &self.upsert_elements {
            if !seen.insert(&element.id) {
                return Err("conflicting element operations".into());
            }
            if base.elements.iter().any(|e| e == element) {
                continue;
            }
            seeds.insert(element.id.clone());
            candidate.elements.retain(|e| e.id != element.id);
            candidate.elements.push(element.clone());
        }
        let mut touched = BTreeSet::new();
        for id in &self.touched_modules {
            if !touched.insert(id)
                || !base
                    .elements
                    .iter()
                    .chain(&candidate.elements)
                    .any(|e| &e.id == id && e.kind == "module")
            {
                return Err("duplicate or unknown touched module".into());
            }
            seeds.insert(id.clone());
        }
        if let Some(policy) = &self.java {
            if base.java.as_ref() != Some(policy) {
                candidate.java = Some(policy.clone());
                seeds.extend(
                    base.elements
                        .iter()
                        .chain(&candidate.elements)
                        .filter(|e| e.kind == "module")
                        .map(|e| e.id.clone()),
                );
            }
        }
        if seeds.is_empty() {
            return Err("change has no model delta or touched modules".into());
        }
        candidate.elements.sort_by(|a, b| a.id.cmp(&b.id));
        candidate.relations.sort_by(|a, b| a.id.cmp(&b.id));
        candidate.validate()?;
        Ok((candidate, seeds))
    }
}
