// Each integration-test crate imports only the helpers its scenarios need.
#![allow(dead_code)]

use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Temp(pub PathBuf);
impl Temp {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "archguard-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    pub fn copy_fixture(&self, name: &str) {
        fn copy(from: &Path, to: &Path) {
            std::fs::create_dir_all(to).unwrap();
            for entry in std::fs::read_dir(from).unwrap() {
                let entry = entry.unwrap();
                if entry.file_name() == "target" || entry.file_name() == "Cargo.lock" {
                    continue;
                }
                let dest = to.join(entry.file_name());
                if entry.file_type().unwrap().is_dir() {
                    copy(&entry.path(), &dest);
                } else {
                    std::fs::copy(entry.path(), dest).unwrap();
                }
            }
        }
        copy(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join(name),
            &self.0,
        );
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[allow(dead_code)]
pub mod evidence;

/// Locate a caller-selected JDK; provider freeze still verifies its exact bytes.
pub fn jdk_home() -> PathBuf {
    std::env::var_os("ARCHGUARD_TEST_JDK")
        .or_else(|| std::env::var_os("JAVA_HOME"))
        .map(PathBuf::from)
        .expect("set ARCHGUARD_TEST_JDK or JAVA_HOME to the qualified JDK")
}

/// Resolve the native executable once; provider freeze verifies its exact bytes.
pub fn node() -> PathBuf {
    if let Some(path) = std::env::var_os("ARCHGUARD_TEST_NODE") {
        return PathBuf::from(path);
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join("node"))
        .find(|path| path.is_file())
        .expect("set ARCHGUARD_TEST_NODE or put the qualified Node on PATH")
}
