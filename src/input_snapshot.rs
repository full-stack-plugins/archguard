use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// 受限本地输入快照，绑定真实被读取的构建文件；无 Java 对应类。
#[derive(Default)]
pub(crate) struct InputSnapshot {
    pub(crate) files: BTreeMap<PathBuf, Vec<u8>>,
}
impl InputSnapshot {
    pub(crate) fn read(&mut self, root: &Path, path: &Path) -> Result<(PathBuf, String), String> {
        let canonical = safe_file(root, path)?;
        if !self.files.contains_key(&canonical) {
            if self.files.len() >= 1024 {
                return Err("input file count exceeds 1024".into());
            }
            let bytes = read_bounded(&canonical)?;
            if self.files.values().map(Vec::len).sum::<usize>() + bytes.len() > 64 * 1024 * 1024 {
                return Err("input snapshot exceeds 64 MiB".into());
            }
            self.files.insert(canonical.clone(), bytes);
        }
        let text =
            String::from_utf8(self.files[&canonical].clone()).map_err(|_| "model must be UTF-8")?;
        Ok((canonical, text))
    }
    pub(crate) fn digest(&self, root: &Path) -> String {
        let mut hash = Sha256::new();
        for (path, bytes) in &self.files {
            let name = path
                .strip_prefix(root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            hash.update((name.len() as u64).to_le_bytes());
            hash.update(name.as_bytes());
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(bytes);
        }
        format!("sha256:{:x}", hash.finalize())
    }
    pub(crate) fn unchanged(&self, root: &Path) -> Result<(), String> {
        for (path, expected) in &self.files {
            safe_file(root, path)?;
            if &read_bounded(path)? != expected {
                return Err(format!("input changed while analyzing: {}", path.display()));
            }
        }
        Ok(())
    }
}
pub(crate) fn safe_file(root: &Path, path: &Path) -> Result<PathBuf, String> {
    for ancestor in path.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(ancestor) {
            if meta.file_type().is_symlink() {
                return Err("linked model paths are unsupported".into());
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err("reparse model paths are unsupported".into());
                }
            }
        }
    }
    let canonical = path
        .canonicalize()
        .map_err(|e| format!("cannot resolve {}: {e}", path.display()))?;
    if !canonical.starts_with(root) {
        return Err(format!("model escapes project root: {}", path.display()));
    }
    Ok(canonical)
}
pub(crate) fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() > 4 * 1024 * 1024 {
        return Err("input must be a regular file at most 4 MiB".into());
    }
    let mut bytes = vec![];
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("input grew beyond 4 MiB".into());
    }
    Ok(bytes)
}
