use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// 仅创建新的报告文件，预检所有目标；后续写入失败时撤回本次创建的文件。
pub(crate) fn write_new(outputs: &[(PathBuf, String)]) -> Result<(), String> {
    let mut targets = Vec::new();
    for (path, _) in outputs {
        let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
        let parent = absolute.parent().ok_or("output has no parent")?;
        let canonical_parent = parent.canonicalize().map_err(|e| e.to_string())?;
        crate::input_snapshot::safe_file(&canonical_parent, parent)?;
        let name = absolute.file_name().ok_or("output has no filename")?;
        if [
            "pom.xml",
            "cargo.toml",
            "cargo.lock",
            "contract.yaml",
            "baseline.json",
            "model.json",
        ]
        .iter()
        .any(|reserved| name.to_string_lossy().eq_ignore_ascii_case(reserved))
        {
            return Err("output must not replace a project input".into());
        }
        let target = canonical_parent.join(name);
        if canonical_parent.ancestors().any(|ancestor| {
            ancestor.file_name().is_some_and(|n| {
                ["baselines", "changes"]
                    .iter()
                    .any(|s| n.to_string_lossy().eq_ignore_ascii_case(s))
            }) && ancestor
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(".archguard"))
        }) {
            return Err("reports cannot be written into model state directories".into());
        }
        if fs::symlink_metadata(&target).is_ok()
            || targets
                .iter()
                .any(|previous: &PathBuf| paths_equal(previous, &target))
        {
            return Err("output already exists or targets collide".into());
        }
        targets.push(target);
    }
    let mut created = Vec::new();
    for ((_, body), target) in outputs.iter().zip(targets) {
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)
                .map_err(|e| e.to_string())?;
            created.push(target);
            file.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())
        })();
        if let Err(error) = result {
            for path in created {
                let _ = fs::remove_file(path);
            }
            return Err(error);
        }
    }
    Ok(())
}
fn paths_equal(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}
