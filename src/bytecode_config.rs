use crate::{
    bytecode_artifact_pin::BytecodeArtifactPin, bytecode_module_input::BytecodeModuleInput,
};
use guard_protocol::{ExecutionBudget, Identifier};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
/// 显式本地身份声明、工具pin与候选覆盖；配置文件不是批准。无Java对应类型。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeConfig {
    pub schema_version: u32,
    pub repository_id: Identifier,
    pub task_id: Identifier,
    pub worktree_id: Identifier,
    pub verifier_id: Identifier,
    pub java_executable: String,
    pub classpath: Vec<String>,
    pub maven_home: String,
    pub repository: String,
    pub home: String,
    pub temporary_directory: String,
    pub tool_artifacts: Vec<BytecodeArtifactPin>,
    pub modules: Vec<BytecodeModuleInput>,
    pub environment: BTreeMap<String, String>,
    pub budget: ExecutionBudget,
}
impl BytecodeConfig {
    /// 从闭合有界JSON载入声明；拒绝重复字段、隐式环境和不受支持目标。
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("bytecode configuration exceeds 1 MiB".into());
        }
        let bytes = guard_protocol::canonicalize(bytes)?;
        let result: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        result.validate()?;
        Ok(result)
    }
    /// 复验显式模板与完整模块/类型清单；不从候选扫描结果倒推覆盖范围。
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || self.classpath.is_empty()
            || self.classpath.len() > 64
            || self.tool_artifacts.is_empty()
            || self.tool_artifacts.len() > 8192
            || self.modules.is_empty()
            || self.modules.len() > 1024
        {
            return Err("unsupported bounded bytecode configuration".into());
        }
        for value in [
            &self.java_executable,
            &self.maven_home,
            &self.repository,
            &self.home,
            &self.temporary_directory,
        ]
        .into_iter()
        .chain(self.classpath.iter())
        .chain(self.tool_artifacts.iter().map(|p| &p.path))
        {
            if !Path::new(value).is_absolute() || value.contains(['\0', '{', '}']) {
                return Err("absolute literal tool paths required".into());
            }
        }
        if self
            .environment
            .get("GIT_CEILING_DIRECTORIES")
            .map(String::as_str)
            != Some("{{subject_parent}}")
        {
            return Err("explicit subject_parent Git ceiling template required".into());
        }
        let mut keys = BTreeSet::new();
        for (key, value) in &self.environment {
            if [
                "JAVA_TOOL_OPTIONS",
                "_JAVA_OPTIONS",
                "JDK_JAVA_OPTIONS",
                "CLASSPATH",
                "MAVEN_OPTS",
                "MAVEN_ARGS",
            ]
            .contains(&key.to_ascii_uppercase().as_str())
            {
                return Err(
                    "implicit JVM/Maven option routing unsupported; use pinned native profile"
                        .into(),
                );
            }
            if key.is_empty()
                || !key.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
                || !keys.insert(key.to_ascii_uppercase())
                || (key.to_ascii_uppercase().starts_with("GIT_")
                    && key != "GIT_CEILING_DIRECTORIES")
                || value.contains('\0')
                || ((value.contains("{{") || value.contains("}}"))
                    && key != "GIT_CEILING_DIRECTORIES")
            {
                return Err("ambiguous or unsupported explicit environment".into());
            }
        }
        let mut modules = BTreeSet::new();
        let mut types = BTreeSet::new();
        for module in &self.modules {
            if (!module.path.is_empty()
                && guard_protocol::GitPath::from_bytes(module.path.as_bytes()).is_err())
                || module.path.contains('\\')
                || !modules.insert(&module.path)
                || module.expected_types.len() > 100000
            {
                return Err("invalid unique Maven module path".into());
            }
            for name in &module.expected_types {
                if name.is_empty()
                    || !name.split('.').all(|s| {
                        !s.is_empty()
                            && s.bytes().enumerate().all(|(i, b)| {
                                b == b'_'
                                    || b == b'$'
                                    || b.is_ascii_alphabetic()
                                    || (i > 0 && b.is_ascii_digit())
                            })
                    })
                    || !types.insert(name)
                {
                    return Err("invalid or duplicate expected compiled type".into());
                }
            }
        }
        if !modules.contains(&String::new()) || types.is_empty() {
            return Err("root module and nonempty explicit compiled type coverage required".into());
        }
        guard_runner::ProcessRunner::new(self.budget.clone())?;
        Ok(())
    }
}
