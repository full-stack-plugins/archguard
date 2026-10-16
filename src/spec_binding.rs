//! 可选的原生规格引用；摘要不构成审批或签名。
use guardengine::{GuardContract, GuardFacts, digest_json};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

fn load(root: &Path) -> Result<Option<(Value, Value, Vec<u8>)>, String> {
    let path = ".archguard/spec-binding.json";
    if std::fs::symlink_metadata(root.join(path))
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        return Ok(None);
    }
    let bytes = guard_install::read_project_file(root, path)?;
    let binding: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let object = binding.as_object().ok_or("binding must be an object")?;
    if object.len() != 3 || !object.contains_key("rules") {
        return Err("binding requires only system, target and rules".into());
    }
    let system = binding["system"]
        .as_str()
        .ok_or("binding system required")?;
    let target = binding["target"]
        .as_str()
        .ok_or("binding target required")?;
    let selected = guard_install::specification_context(root, Some(system), Some(target))?;
    if selected["status"] != "ready" {
        return Err(format!("bound specification is not ready: {selected}"));
    }
    Ok(Some((binding, selected, bytes)))
}
/// 获取项目绑定及原生规格引用，返回未绑定或已选上下文；不批准或写入任何产物。
pub fn context(root: &Path) -> Result<Value, String> {
    match load(root)? {
        None => Ok(json!({"status":"unbound","approved":false})),
        Some((binding, selected, _)) => Ok(
            json!({"status":"bound","binding":binding,"specification":selected,"approved":false}),
        ),
    }
}
/// 校验所有规则的原生需求引用，并将规格快照加入事实摘要；返回错误时不得产生通过报告。
pub fn bind(root: &Path, contract: &GuardContract, facts: &mut GuardFacts) -> Result<(), String> {
    let Some((binding, selected, bytes)) = load(root)? else {
        return Ok(());
    };
    let rules = binding["rules"]
        .as_object()
        .ok_or("binding rules must be an object")?;
    let expected: BTreeSet<&str> = contract.spec.rules.iter().map(|r| r.id.as_str()).collect();
    if rules.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err("spec binding must map every contract rule exactly once".into());
    }
    for reference in rules.values() {
        let object = reference
            .as_object()
            .ok_or("rule reference must be an object")?;
        if object.len() != 2 {
            return Err("rule reference requires artifact and requirement only".into());
        }
        let path = reference["artifact"]
            .as_str()
            .ok_or("reference artifact required")?;
        let requirement = reference["requirement"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .ok_or("requirement marker required")?;
        let artifact = selected["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["path"] == path)
            .ok_or("reference is outside selected native specification")?;
        let content = guard_install::read_project_file(root, path)?;
        if artifact["sha256"] != format!("{:x}", Sha256::digest(&content)) {
            return Err("specification changed during analysis".into());
        }
        let text = std::str::from_utf8(&content).map_err(|_| "specification must be UTF-8")?;
        // 仅验证完整文本标识存在；不把子串 REQ-1 误当作 REQ-10，也不宣称语义满足。
        let token = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '/');
        let found = text.match_indices(requirement).any(|(start, _)| {
            let end = start + requirement.len();
            !text[..start].chars().next_back().is_some_and(token)
                && !text[end..].chars().next().is_some_and(token)
        });
        if !found {
            return Err(format!("requirement marker {requirement} absent in {path}"));
        }
    }
    facts.subject.snapshot_digest = digest_json(&json!({"source":facts.subject.snapshot_digest,"binding_sha256":format!("{:x}", Sha256::digest(bytes)),"specification_digest":selected["digest"]})).map_err(|e| e.to_string())?;
    Ok(())
}
