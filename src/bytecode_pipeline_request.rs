use crate::{
    bytecode_artifact_pin::BytecodeArtifactPin, bytecode_config::BytecodeConfig,
    bytecode_module_input::BytecodeModuleInput, bytecode_policy::BytecodePolicy,
    bytecode_rule::BytecodeRule,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use flate2::{Compression, GzBuilder};
use guard_protocol::Digest;
use guard_runner::CommandSpec;
use serde::Serialize;
use std::{collections::BTreeMap, io::Write, path::Path};
/// 与Java PipelineRequest字段对齐的冻结请求；projectRoot仅允许受控占位符。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BytecodePipelineRequest {
    pub schema_version: u32,
    pub project_root: String,
    pub java_executable: String,
    pub maven_home: String,
    pub repository: String,
    pub home: String,
    pub temporary_directory: String,
    pub tool_artifacts: Vec<BytecodeArtifactPin>,
    pub modules: Vec<BytecodeModuleInput>,
    pub rules: Vec<BytecodeRule>,
}
impl BytecodePipelineRequest {
    /// 从执行前配置与完整规则生成请求；不读取运行结果。
    pub fn new(config: &BytecodeConfig, policy: &BytecodePolicy) -> Result<Self, String> {
        config.validate()?;
        Ok(Self {
            schema_version: 1,
            project_root: "{{subject_root}}".into(),
            java_executable: config.java_executable.clone(),
            maven_home: config.maven_home.clone(),
            repository: config.repository.clone(),
            home: config.home.clone(),
            temporary_directory: config.temporary_directory.clone(),
            tool_artifacts: config.tool_artifacts.clone(),
            modules: config.modules.clone(),
            rules: policy.rules().to_vec(),
        })
    }
    /// 生成确定性gzip/base64，避免Win JSON转义与命令行长度错误；超限明确拒绝。
    pub fn packed(&self) -> Result<String, String> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("request exceeds decompression budget".into());
        }
        let mut gzip = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::default());
        gzip.write_all(&bytes).map_err(|e| e.to_string())?;
        let packed = STANDARD.encode(gzip.finish().map_err(|e| e.to_string())?);
        if packed.len() > 24000 {
            return Err("packed request exceeds native argv budget".into());
        }
        Ok(packed)
    }
    /// 生成v2逻辑命令；物理nonce由共享Runtime替换，配置pin保持稳定。
    pub fn command(&self, config: &BytecodeConfig, project: &Path) -> Result<CommandSpec, String> {
        let executable = Path::new(&config.java_executable)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let pin = config
            .tool_artifacts
            .iter()
            .find(|p| Path::new(&p.path).canonicalize().ok().as_ref() == Some(&executable))
            .ok_or("Java executable pin missing")?;
        let separator = if cfg!(windows) { ";" } else { ":" };
        let classpath = config
            .classpath
            .iter()
            .map(|p| crate::bytecode_paths::native(&crate::bytecode_paths::safe(Path::new(p))?))
            .collect::<Result<Vec<_>, String>>()?
            .join(separator);
        let args = vec![
            "-Dfile.encoding=UTF-8".into(),
            format!("-Djava.io.tmpdir={}", config.temporary_directory),
            format!("-Duser.home={}", config.home),
            "-cp".into(),
            classpath,
            "ai.partme.archguard.PipelineMain".into(),
            "pipeline-packed".into(),
            "{{subject_root}}".into(),
            self.packed()?,
        ];
        if args
            .iter()
            .map(|v| v.encode_utf16().count() + 3)
            .sum::<usize>()
            > 30000
        {
            return Err("native invocation exceeds Windows safe argv budget".into());
        }
        Ok(CommandSpec {
            executable,
            executable_digest: pin.sha256.clone(),
            args,
            directory: project.to_path_buf(),
            environment: config.environment.clone(),
        })
    }
}
/// 与Java TreeMap清单公式一致：UTF16键序，key/value的UTF8字节各加u64大端长度。
pub fn manifest_digest(values: &BTreeMap<String, String>) -> Digest {
    use sha2::{Digest as _, Sha256};
    let mut entries: Vec<_> = values.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
    let mut hash = Sha256::new();
    for (key, value) in entries {
        for text in [key, value] {
            hash.update((text.len() as u64).to_be_bytes());
            hash.update(text.as_bytes());
        }
    }
    Digest::new(format!("sha256:{:x}", hash.finalize())).expect("SHA256 format")
}
/// 返回原始字节SHA256；该摘要不代表受信发布者身份。
pub fn content_digest(bytes: &[u8]) -> Digest {
    use sha2::{Digest as _, Sha256};
    Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).expect("SHA256 format")
}
