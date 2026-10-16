use crate::{
    baseline_head::BaselineHead,
    baseline_record::BaselineRecord,
    input_snapshot::{read_bounded, safe_file},
};
use guardengine::digest_json;
use serde::de::DeserializeOwned;
use std::{
    fs,
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn digest<T: serde::Serialize>(value: &T) -> Result<String, String> {
    digest_json(value).map_err(|e| e.to_string())
}
pub(crate) fn read<T: DeserializeOwned>(root: &Path, path: &Path) -> Result<T, String> {
    let path = safe_file(root, path)?;
    serde_json::from_slice(&read_bounded(&path)?).map_err(|e| format!("{}: {e}", path.display()))
}
pub(crate) fn valid_digest(value: &str) -> Result<&str, String> {
    value
        .strip_prefix("sha256:")
        .filter(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        })
        .ok_or_else(|| "invalid baseline digest".into())
}
/// 读取并校验指针及内容寻址快照；无基线返回 None，损坏或链接不会降级为未配置。
pub(crate) fn load(root: &Path) -> Result<Option<(String, BaselineRecord)>, String> {
    let dir = root.join(".archguard");
    if fs::symlink_metadata(&dir).is_ok() {
        safe_file(root, &dir)?;
    }
    let path = dir.join("baseline.json");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
        _ => {}
    }
    let head: BaselineHead = read(root, &path)?;
    let hash = valid_digest(&head.digest)?;
    if head.schema_version != 1 {
        return Err("unknown baseline head version".into());
    }
    let record: BaselineRecord = read(root, &dir.join("baselines").join(format!("{hash}.json")))?;
    if record.schema_version != 1 || record.revision == 0 || digest(&record)? != head.digest {
        return Err("corrupt baseline snapshot".into());
    }
    if (record.revision == 1) != record.parent.is_none() {
        return Err("invalid baseline ancestry".into());
    }
    if let Some(parent) = &record.parent {
        valid_digest(parent)?;
    }
    valid_digest(&record.contract_digest)?;
    record.model.validate()?;
    Ok(Some((head.digest, record)))
}

/// 保存不可变快照并原子替换指针；调用者持有锁，expected 防止覆盖另一基线。
pub(crate) fn publish(
    root: &Path,
    expected: Option<&str>,
    record: &BaselineRecord,
) -> Result<String, String> {
    if load(root)?.as_ref().map(|v| v.0.as_str()) != expected {
        return Err("baseline changed before publication".into());
    }
    let hash = digest(record)?;
    let bytes = serde_json::to_vec_pretty(record).map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("baseline snapshot exceeds 4 MiB".into());
    }
    let short = valid_digest(&hash)?;
    let directory = root.join(".archguard/baselines");
    match fs::create_dir(&directory) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.to_string()),
    }
    safe_file(root, &directory)?;
    let snapshot = directory.join(format!("{short}.json"));
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&snapshot)
    {
        Ok(mut file) => {
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing: BaselineRecord = read(root, &snapshot)?;
            if digest(&existing)? != hash {
                return Err("snapshot collision".into());
            }
        }
        Err(e) => return Err(e.to_string()),
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp = root.join(format!(
        ".archguard/.baseline-{}-{nonce}.tmp",
        std::process::id()
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        let head = BaselineHead {
            schema_version: 1,
            digest: hash.clone(),
        };
        file.write_all(&serde_json::to_vec_pretty(&head).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        if load(root)?.as_ref().map(|v| v.0.as_str()) != expected {
            return Err("baseline changed during publication".into());
        }
        fs::rename(&temp, root.join(".archguard/baseline.json")).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result?;
    Ok(hash)
}
