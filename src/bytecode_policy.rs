pub use crate::bytecode_rule::BytecodeRule;
use guard_contract::CompiledContract;
use guard_protocol::{Digest, DomainOwner, Identifier};
use serde::Deserialize;

/// 本地技术规则或编译契约规则的只读投影；不授予执行和交付权限。无Java对应类型。
#[derive(Clone, Debug)]
pub struct BytecodePolicy {
    rules: Vec<BytecodeRule>,
    contract_digest: Option<Digest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocalPolicy {
    schema_version: u32,
    rules: Vec<BytecodeRule>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Parameters {
    schema_version: u32,
    kind: String,
    from: String,
    forbidden_to: Vec<String>,
}

impl BytecodePolicy {
    fn checked(rules: Vec<BytecodeRule>, contract_digest: Option<Digest>) -> Result<Self, String> {
        if rules.is_empty() || rules.len() > 1024 {
            return Err("nonempty bounded bytecode rules required".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for rule in &rules {
            rule.validate()?;
            if !ids.insert(rule.rule_id()) {
                return Err("duplicate bytecode rule identity".into());
            }
        }
        Ok(Self {
            rules,
            contract_digest,
        })
    }

    /// 从严格闭合JSON读取本地技术配置；拒绝自报approved等额外字段，不构成批准。
    pub fn from_local_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("bytecode local policy exceeds 1 MiB".into());
        }
        let canonical = guard_protocol::canonicalize(bytes)?;
        let policy: LocalPolicy = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
        if policy.schema_version != 1 {
            return Err("unsupported bytecode local policy version".into());
        }
        Self::checked(policy.rules, None)
    }

    /// 从不可变编译契约投影指定验证器的全部规则；未知类型或跨专业owner明确拒绝。
    /// 该引用仅保留契约身份，真实批准服务与执行许可仍由外部控制端核验。
    pub fn from_contract(
        contract: &CompiledContract,
        verifier: &Identifier,
    ) -> Result<Self, String> {
        if !contract
            .verifiers()
            .iter()
            .any(|pin| &pin.verifier_id == verifier)
        {
            return Err("bytecode verifier missing from compiled pins".into());
        }
        let mut rules = Vec::new();
        for rule in contract
            .rules()
            .iter()
            .filter(|rule| &rule.verifier_id == verifier)
        {
            if rule.owner != DomainOwner::Archguard {
                return Err("bytecode verifier contains another professional owner's rule".into());
            }
            let params: Parameters =
                serde_json::from_value(rule.parameters.clone()).map_err(|e| e.to_string())?;
            if params.schema_version != 1 || params.kind != "forbid_package_dependency" {
                return Err("unsupported bytecode rule kind/version; coverage not proven".into());
            }
            rules.push(BytecodeRule::new(
                rule.rule_id.as_str().into(),
                params.from,
                params.forbidden_to,
            )?);
        }
        Self::checked(rules, Some(contract.digest().clone()))
    }

    /// 返回已校验的完整规则集合，不由扫描结果删减。
    pub fn rules(&self) -> &[BytecodeRule] {
        &self.rules
    }
    /// 返回是否来自编译契约；该标记不代表受信身份或交付授权。
    pub fn is_contract_bound(&self) -> bool {
        self.contract_digest.is_some()
    }
    /// 返回对应不可变契约摘要；本地技术配置保持None。
    pub fn contract_digest(&self) -> Option<&Digest> {
        self.contract_digest.as_ref()
    }
}
