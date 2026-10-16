use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

/// 对已有原生需求的引用，不复制需求正文或签发批准。
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementReference {
    pub system: String,
    pub target: String,
    pub artifact: String,
    pub requirement: String,
}
impl RequirementReference {
    /// 校验目标产物和完整需求标识；返回当前原生规格摘要，供复验绑定。
    pub fn inspect(&self, root: &Path) -> Result<Value, String> {
        if self.requirement.trim().is_empty() {
            return Err("requirement ID is empty".into());
        }
        let selected =
            guard_install::specification_context(root, Some(&self.system), Some(&self.target))?;
        if selected["status"] != "ready" {
            return Err("native requirement source is incomplete or ambiguous".into());
        }
        let source = selected["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["path"] == self.artifact)
            .ok_or("requirement artifact is outside selected native specification")?;
        let bytes = guard_install::read_project_file(root, &self.artifact)?;
        if source["sha256"] != format!("{:x}", Sha256::digest(&bytes)) {
            return Err("native requirement changed while reading".into());
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| "requirement must be UTF-8")?;
        let token = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '/');
        if !text.match_indices(&self.requirement).any(|(i, _)| {
            !text[..i].chars().next_back().is_some_and(token)
                && !text[i + self.requirement.len()..]
                    .chars()
                    .next()
                    .is_some_and(token)
        }) {
            return Err("native requirement identifier not found".into());
        }
        Ok(selected)
    }
}
