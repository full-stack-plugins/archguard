use serde::{Deserialize, Serialize};

/// Java适配器的精确包前缀禁止边；规则身份由调用端保留。无Java对应类型。
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BytecodeRule {
    rule_id: String,
    from: String,
    forbidden_to: Vec<String>,
}

fn package_prefix(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && value.split('.').all(|part| {
            let mut bytes = part.bytes();
            bytes
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_' || b == b'$')
                && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
        })
}

impl BytecodeRule {
    pub(crate) fn new(
        rule_id: String,
        from: String,
        forbidden_to: Vec<String>,
    ) -> Result<Self, String> {
        let rule = Self {
            rule_id,
            from,
            forbidden_to,
        };
        rule.validate()?;
        Ok(rule)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.rule_id.is_empty()
            || self.rule_id.len() > 128
            || !self
                .rule_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            || !package_prefix(&self.from)
            || self.forbidden_to.is_empty()
            || self.forbidden_to.len() > 128
        {
            return Err("unsupported bytecode rule identity, prefix or target count".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for prefix in &self.forbidden_to {
            if !package_prefix(prefix) || !seen.insert(prefix) {
                return Err("invalid or duplicate forbidden package prefix".into());
            }
        }
        Ok(())
    }

    /// 返回保留的规则ID；不将位置相同的不同规则折叠。
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }
    /// 返回按点分段匹配的源包前缀，不支持通配符。
    pub fn from(&self) -> &str {
        &self.from
    }
    /// 返回显式禁止的目标包前缀全集。
    pub fn forbidden_to(&self) -> &[String] {
        &self.forbidden_to
    }
}
