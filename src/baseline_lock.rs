use std::{
    fs,
    path::{Path, PathBuf},
};

/// 协调本工具的基线写入；不把残留锁自动当作失效锁删除。
pub(crate) struct BaselineLock(PathBuf);
impl BaselineLock {
    /// 对已验证目录独占创建锁，返回 RAII 句柄或并发冲突。
    pub(crate) fn acquire(root: &Path) -> Result<Self, String> {
        let path = root.join(".archguard/.baseline.lock");
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| format!("baseline lock unavailable: {e}"))?;
        Ok(Self(path))
    }
}
impl Drop for BaselineLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
