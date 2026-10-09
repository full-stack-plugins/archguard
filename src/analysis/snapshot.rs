//! Conservative source-tree inventory, separate from the legacy manifest digest.
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotInventory {
    inputs: BTreeMap<String, Vec<u8>>,
}
impl SnapshotInventory {
    /// Names are protocol keys, not host paths. Duplicate keys are invalid.
    pub fn from_inputs(
        inputs: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<Self, String> {
        let mut result = BTreeMap::new();
        for (name, bytes) in inputs {
            if name.is_empty() || result.insert(name, bytes).is_some() {
                return Err("empty or duplicate inventory key".into());
            }
        }
        Ok(Self { inputs: result })
    }
    pub fn digest(&self) -> String {
        self.digest_selected(false)
    }
    pub(crate) fn files_digest(&self) -> String {
        self.digest_selected(true)
    }
    fn digest_selected(&self, files_only: bool) -> String {
        let mut h = Sha256::new();
        h.update(b"archguard.cargo.inventory/v1\0");
        for (name, bytes) in &self.inputs {
            if files_only && !name.starts_with("file:") {
                continue;
            }
            h.update((name.len() as u64).to_le_bytes());
            h.update(name.as_bytes());
            h.update((bytes.len() as u64).to_le_bytes());
            h.update(bytes);
        }
        format!("sha256:{:x}", h.finalize())
    }
    pub(crate) fn materialize_files(&self, root: &Path) -> Result<(), String> {
        for (key, bytes) in &self.inputs {
            let path = key.strip_prefix("file:").ok_or("not a file inventory")?;
            let destination = root.join(path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            use std::io::Write;
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .and_then(|mut file| file.write_all(bytes))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.inputs.keys().map(String::as_str)
    }
    pub fn capture(
        root: &Path,
        identity: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<Self, String> {
        fn walk(
            root: &Path,
            dir: &Path,
            inputs: &mut Vec<(String, Vec<u8>)>,
            depth: usize,
            total: &mut u64,
        ) -> Result<(), String> {
            if depth > 64 {
                return Err("inventory depth budget exceeded".into());
            }
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_name() == ".git" || entry.file_name() == "target" {
                    continue;
                }
                let ty = entry.file_type().map_err(|e| e.to_string())?;
                if ty.is_symlink() {
                    return Err("inventory symlink is outside supported profile".into());
                }
                if ty.is_dir() {
                    walk(root, &entry.path(), inputs, depth + 1, total)?;
                } else if ty.is_file() {
                    let size = entry.metadata().map_err(|e| e.to_string())?.len();
                    *total = total.checked_add(size).ok_or("inventory size overflow")?;
                    if size > 16 * 1024 * 1024
                        || *total > 128 * 1024 * 1024
                        || inputs.len() >= 10_000
                    {
                        return Err("inventory size budget exceeded".into());
                    }
                    let relative = entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_str()
                        .ok_or("non UTF-8 inventory path")?
                        .replace('\\', "/");
                    inputs.push((
                        format!("file:{relative}"),
                        fs::read(entry.path()).map_err(|e| e.to_string())?,
                    ));
                } else {
                    return Err("unsupported inventory file type".into());
                }
            }
            Ok(())
        }
        let mut inputs = Vec::new();
        walk(root, root, &mut inputs, 0, &mut 0)?;
        inputs.extend(
            identity
                .into_iter()
                .map(|(key, bytes)| (format!("identity:{key}"), bytes)),
        );
        Self::from_inputs(inputs)
    }
}
