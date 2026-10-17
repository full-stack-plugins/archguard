use crate::{bytecode_config::BytecodeConfig, bytecode_pipeline_request::manifest_digest};
use guard_protocol::Digest;
use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::{
    io::Read,
    path::{Path, PathBuf},
};
/// 拒绝链接和Windows reparse的原件路径，返回当前真实位置；不提供OS隔离。
pub(crate) fn safe(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("absolute path required".into());
    }
    for parent in path.ancestors() {
        let meta = std::fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err("linked tool path rejected".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                return Err("reparse tool path rejected".into());
            }
        }
    }
    path.canonicalize().map_err(|e| e.to_string())
}
/// 将控制端真实绝对路径转换为Java/Git支持的native表示，保持文件身份不变。
pub(crate) fn native(path: &Path) -> Result<String, String> {
    let value = path.to_str().ok_or("UTF8 native path required")?;
    #[cfg(windows)]
    {
        if let Some(tail) = value.strip_prefix(r"\\?\UNC\") {
            return Ok(format!(r"\\{}", tail));
        }
        if let Some(tail) = value.strip_prefix(r"\\?\") {
            if tail.as_bytes().get(1) == Some(&b':') {
                return Ok(tail.into());
            }
            return Err("unsupported native path namespace".into());
        }
    }
    Ok(value.into())
}
/// 流式复验全部显式工具pin和启动classpath；Java另核实Maven/仓库闭包。
pub(crate) fn tools(config: &BytecodeConfig) -> Result<Digest, String> {
    let mut map = BTreeMap::new();
    for pin in &config.tool_artifacts {
        let path = safe(Path::new(&pin.path))?;
        let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > 512 * 1024 * 1024 {
            return Err("bounded regular tool artifact required".into());
        }
        let mut file = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut count = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let length = file.read(&mut buffer).map_err(|e| e.to_string())?;
            if length == 0 {
                break;
            }
            count += length as u64;
            if count > 512 * 1024 * 1024 {
                return Err("tool grew beyond budget".into());
            }
            hash.update(&buffer[..length]);
        }
        if count != meta.len() || format!("sha256:{:x}", hash.finalize()) != pin.sha256.as_str() {
            return Err("tool artifact pin mismatch".into());
        }
        if map
            .insert(native(&path)?, pin.sha256.as_str().to_string())
            .is_some()
        {
            return Err("duplicate canonical tool artifact".into());
        }
    }
    for file in config
        .classpath
        .iter()
        .chain(std::iter::once(&config.java_executable))
    {
        if !map.contains_key(&native(&safe(Path::new(file))?)?) {
            return Err("classpath or executable artifact pin missing".into());
        }
    }
    // 明确pin JVM启动实现、JDK镜像与发行声明，但不宣称整个JDK/OS依赖闭包。
    let java = safe(Path::new(&config.java_executable))?;
    let jdk = java
        .parent()
        .and_then(Path::parent)
        .ok_or("JDK layout required")?;
    for relative in [
        "release",
        "lib/modules",
        if cfg!(windows) {
            "bin/server/jvm.dll"
        } else {
            "lib/server/libjvm.so"
        },
    ] {
        if !map.contains_key(&native(&safe(&jdk.join(relative))?)?) {
            return Err("JDK release/modules/VM artifact pin missing".into());
        }
    }
    Ok(manifest_digest(&map))
}
