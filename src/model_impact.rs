use crate::architecture_model::ArchitectureModel;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// 从增量端点出发遍历旧/新关系与归属联合图，返回保守影响范围及见证路径。
pub(crate) fn analyze(
    before: &ArchitectureModel,
    after: &ArchitectureModel,
    seeds: &BTreeSet<String>,
) -> Value {
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut edge = |a: &str, b: &str| {
        graph.entry(a.into()).or_default().insert(b.into());
        graph.entry(b.into()).or_default().insert(a.into());
    };
    for model in [before, after] {
        for relation in &model.relations {
            edge(&relation.from, &relation.to);
        }
        for element in &model.elements {
            for parent in [&element.context, &element.module].into_iter().flatten() {
                edge(&element.id, parent);
            }
        }
    }
    let mut predecessors: BTreeMap<String, Option<String>> =
        seeds.iter().map(|id| (id.clone(), None)).collect();
    let mut queue: VecDeque<_> = seeds.iter().cloned().collect();
    while let Some(id) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&id) {
            for neighbor in neighbors {
                if !predecessors.contains_key(neighbor) {
                    predecessors.insert(neighbor.clone(), Some(id.clone()));
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }
    let mut modules = BTreeSet::new();
    let mut contexts = BTreeSet::new();
    for element in before.elements.iter().chain(&after.elements) {
        if predecessors.contains_key(&element.id) {
            if let Some(implementation) = &element.implementation {
                modules.insert(implementation);
            }
            if element.kind == "context" {
                contexts.insert(&element.id);
            }
        }
    }
    // 可达集合保持完整，路径仅物化有界样本；避免长链图导致二次方内存开销。
    let mut paths = BTreeMap::new();
    let mut truncated = predecessors.len() > 128;
    for id in predecessors.keys().take(128) {
        let mut path = vec![id.clone()];
        let mut current = id;
        while let Some(parent) = &predecessors[current] {
            if path.len() == 64 {
                truncated = true;
                break;
            }
            path.push(parent.clone());
            current = parent;
        }
        path.reverse();
        paths.insert(id.clone(), path);
    }
    json!({"directly_changed":seeds,"affected_elements":predecessors.keys().collect::<Vec<_>>(),"affected_modules":modules,"affected_contexts":contexts,"paths":paths,"predecessors":predecessors,"witnesses_truncated":truncated,"path_sample_limit":128,"path_node_limit":64,"scope":"conservative old/new declared relationships and ownership; not a source call graph","check_scope":"full project"})
}
