use crate::{
    bytecode_config::BytecodeConfig, bytecode_policy::BytecodePolicy,
    bytecode_receipt::BytecodeReceipt, bytecode_service,
};
use guard_protocol::GitOid;
use guard_runner::Cancellation;
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::Selection;
use guardengine::Decision;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};
fn read_bounded(path: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("bytecode input document exceeds budget".into());
    }
    Ok(bytes)
}
/// 显式本地技术CLI；不会从JSON生成批准对象，不在原项目运行编译。无Java对应类型。
pub fn run(args: &[String]) -> Result<(serde_json::Value, Decision), String> {
    let action = args
        .first()
        .map(String::as_str)
        .ok_or("expected bytecode check|verify")?;
    if !["check", "verify"].contains(&action) {
        return Err("expected bytecode check|verify".into());
    }
    let mut options = BTreeMap::new();
    let allowed = [
        "--project",
        "--scratch",
        "--store",
        "--config",
        "--policy",
        "--subject",
        "--base",
        "--commit",
        "--report",
        "--previous",
        "--operation",
    ];
    for pair in args[1..].chunks(2) {
        if pair.len() != 2
            || !allowed.contains(&pair[0].as_str())
            || pair[1].starts_with("--")
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown duplicate or incomplete bytecode option".into());
        }
    }
    let required = |key| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("explicit {key} required"))
    };
    let project = Path::new(required("--project")?);
    let scratch = Path::new(required("--scratch")?);
    let root = Path::new(required("--store")?);
    for path in [project, scratch, root] {
        if !path.is_absolute() {
            return Err("absolute project/scratch/store required".into());
        }
    }
    let config = BytecodeConfig::from_json(&read_bounded(required("--config")?, 1024 * 1024)?)?;
    let policy =
        BytecodePolicy::from_local_json(&read_bounded(required("--policy")?, 1024 * 1024)?)?;
    let oid = |key| {
        let value = required(key)?;
        GitOid::new(value)
    };
    let base = oid("--base")?;
    let selection = match required("--subject")? {
        "worktree" => Selection::Worktree { base },
        "commit" => Selection::Commit {
            base,
            commit: oid("--commit")?,
        },
        "index" => Selection::Index { base },
        _ => return Err("bytecode local subject unsupported; use worktree|commit|index".into()),
    };
    let store = if root.exists() {
        EvidenceStore::open(root, StoreLimits::default())?
    } else {
        EvidenceStore::create(root, StoreLimits::default())?
    };
    let operation = required("--operation")?;
    let retain = bytecode_service::now()?
        .checked_add(86400000)
        .ok_or("retention overflow")?;
    let receipt = if action == "verify" {
        let bytes = read_bounded(required("--previous")?, 2 * 1024 * 1024)?;
        let old = BytecodeReceipt::from_json(&bytes, &store, bytecode_service::now()?)?;
        bytecode_service::verify_local(
            &old,
            project,
            scratch,
            &config,
            &policy,
            &selection,
            &store,
            operation,
            retain,
            Cancellation::new(),
        )?
    } else {
        if options.contains_key("--previous") {
            return Err("previous only valid with verify".into());
        }
        bytecode_service::check_local(
            project,
            scratch,
            &config,
            &policy,
            &selection,
            &store,
            operation,
            retain,
            Cancellation::new(),
        )?
    };
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?;
    if let Some(path) = options.get("--report") {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut file| file.write_all(&bytes))
            .map_err(|e| e.to_string())?;
    }
    let decision = if receipt.report.status == "passed" {
        Decision::Allow
    } else {
        Decision::Block
    };
    Ok((
        serde_json::to_value(receipt).map_err(|e| e.to_string())?,
        decision,
    ))
}
