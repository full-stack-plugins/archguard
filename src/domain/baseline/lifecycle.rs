//! Architecture-owned immutable revisions. Serialization is archival, never authority.
use crate::integration::projection::{bytes, digest};
use guardengine::{
    GuardContract,
    integration::{
        RunBinding,
        eligibility::{AuthorityProvider, EligibilityPolicy, validate_approval_record},
    },
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchitectureBaselineState {
    Draft,
    InReview,
    Approved,
    Superseded,
    Revoked,
}
#[derive(Clone, Serialize)]
struct Content {
    version: &'static str,
    binding: RunBinding,
    source_ref: String,
    adr_digests: BTreeMap<String, String>,
    contract_digest: String,
    obligations: BTreeSet<String>,
    predecessor: Option<String>,
}
/// No Deserialize or public field mutation; an approved record is still not current authority.
#[derive(Clone, Serialize)]
pub struct ArchitectureBaseline {
    content: Content,
    content_digest: String,
    state: ArchitectureBaselineState,
    predecessor_record: Option<String>,
    approval_ref: Option<String>,
    replacement_digest: Option<String>,
    digest: String,
}
fn admit<T: Serialize>(value: &T) -> Result<(), String> {
    struct Count(usize);
    impl std::io::Write for Count {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(b.len())
                .filter(|n| *n <= 1048576)
                .ok_or_else(|| std::io::Error::other("budget"))?;
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Count(0), value).map_err(|_| "baseline byte budget".into())
}
fn identifier(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 2048 && !s.contains('\0')
}
impl ArchitectureBaseline {
    pub fn draft(
        binding: RunBinding,
        source_ref: &str,
        adrs: BTreeMap<String, Vec<u8>>,
        contract: &GuardContract,
        obligations: BTreeSet<String>,
    ) -> Result<Self, String> {
        Self::freeze(binding, source_ref, adrs, contract, obligations, None)
    }
    fn freeze(
        binding: RunBinding,
        source_ref: &str,
        adrs: BTreeMap<String, Vec<u8>>,
        contract: &GuardContract,
        obligations: BTreeSet<String>,
        predecessor: Option<String>,
    ) -> Result<Self, String> {
        admit(&(&binding, source_ref, &adrs, contract, &obligations))?;
        if !identifier(source_ref)
            || adrs.is_empty()
            || adrs.len() > 256
            || adrs.iter().any(|(k, v)| !identifier(k) || v.is_empty())
            || obligations.is_empty()
            || obligations.len() > 4096
            || obligations.iter().any(|s| !identifier(s))
        {
            return Err("invalid baseline source/ADR/obligations".into());
        }
        contract
            .validate()
            .map_err(|_| "invalid baseline contract")?;
        let content = Content {
            version: "archguard.architecture-baseline/v1alpha1",
            binding,
            source_ref: source_ref.into(),
            adr_digests: adrs
                .into_iter()
                .map(|(id, raw)| (id, digest(&raw)))
                .collect(),
            contract_digest: digest(&bytes(contract)?),
            obligations,
            predecessor,
        };
        let content_digest = digest(&bytes(&content)?);
        let mut result = Self {
            content,
            content_digest,
            state: ArchitectureBaselineState::Draft,
            predecessor_record: None,
            approval_ref: None,
            replacement_digest: None,
            digest: String::new(),
        };
        result.rehash()?;
        Ok(result)
    }
    fn rehash(&mut self) -> Result<(), String> {
        self.digest = digest(&bytes(&(
            "archguard.baseline-state/v1alpha1",
            &self.content_digest,
            self.state,
            &self.predecessor_record,
            &self.approval_ref,
            &self.replacement_digest,
        ))?);
        Ok(())
    }
    fn transition(
        &self,
        state: ArchitectureBaselineState,
        reference: Option<&str>,
        replacement: Option<String>,
    ) -> Result<Self, String> {
        if reference.is_some_and(|r| !identifier(r)) {
            return Err("invalid external approval reference".into());
        }
        let mut next = self.clone();
        next.state = state;
        next.predecessor_record = Some(self.digest.clone());
        next.approval_ref = reference.map(str::to_owned);
        next.replacement_digest = replacement;
        next.rehash()?;
        Ok(next)
    }
    pub fn source_ref(&self) -> &str {
        &self.content.source_ref
    }
    pub fn binding(&self) -> &RunBinding {
        &self.content.binding
    }
    pub fn state(&self) -> ArchitectureBaselineState {
        self.state
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }
    pub fn contract_digest(&self) -> &str {
        &self.content.contract_digest
    }
    pub fn obligations(&self) -> &BTreeSet<String> {
        &self.content.obligations
    }
    pub fn submit(&self) -> Result<Self, String> {
        if self.state != ArchitectureBaselineState::Draft {
            return Err("only draft can enter review".into());
        }
        self.transition(ArchitectureBaselineState::InReview, None, None)
    }
    pub fn revise(
        &self,
        adrs: BTreeMap<String, Vec<u8>>,
        contract: &GuardContract,
        obligations: BTreeSet<String>,
    ) -> Result<Self, String> {
        self.revise_source(
            self.content.binding.clone(),
            &self.content.source_ref,
            adrs,
            contract,
            obligations,
        )
    }
    pub fn revise_source(
        &self,
        binding: RunBinding,
        source_ref: &str,
        adrs: BTreeMap<String, Vec<u8>>,
        contract: &GuardContract,
        obligations: BTreeSet<String>,
    ) -> Result<Self, String> {
        if matches!(
            self.state,
            ArchitectureBaselineState::Revoked | ArchitectureBaselineState::Superseded
        ) {
            return Err("retired revision cannot be revised".into());
        }
        if binding.repo_id != self.content.binding.repo_id
            || binding.requirement_ids != self.content.binding.requirement_ids
        {
            return Err("cross-scope revision".into());
        }
        Self::freeze(
            binding,
            source_ref,
            adrs,
            contract,
            obligations,
            Some(self.content_digest.clone()),
        )
    }
    fn authenticate(
        &self,
        reference: &str,
        action: &str,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<(), String> {
        admit(policy)?;
        if now < 0
            || !identifier(reference)
            || policy.action != action
            || policy.contract_digest != self.content.contract_digest
        {
            return Err("baseline approval context mismatch".into());
        }
        let mut expected = self.content.binding.clone();
        expected.baseline_digest = Some(self.content_digest.clone());
        if policy.binding != expected {
            return Err("baseline approval binding mismatch".into());
        }
        let record = provider
            .verify_approval(reference)
            .map_err(|_| "baseline authority unavailable or untrusted")?;
        validate_approval_record(&record, policy, "architecture-baseline", now)
            .map_err(|_| "baseline approval invalid or inactive".into())
    }
    pub fn approve(
        &self,
        reference: &str,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<Self, String> {
        if self.state != ArchitectureBaselineState::InReview {
            return Err("only reviewed revision can be approved".into());
        }
        self.authenticate(
            reference,
            "architecture-baseline:approve",
            policy,
            provider,
            now,
        )?;
        self.transition(ArchitectureBaselineState::Approved, Some(reference), None)
    }
    /// Always requery the external issuer. Historical approved state is not cached authority.
    pub fn qualify(
        &self,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<(), String> {
        if self.state != ArchitectureBaselineState::Approved {
            return Err("baseline is not approved".into());
        }
        self.authenticate(
            self.approval_ref
                .as_deref()
                .ok_or("missing approval reference")?,
            "architecture-baseline:approve",
            policy,
            provider,
            now,
        )
    }
    pub fn revoke(
        &self,
        reference: &str,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<Self, String> {
        if self.state != ArchitectureBaselineState::Approved {
            return Err("only approved revision can be revoked".into());
        }
        self.authenticate(
            reference,
            "architecture-baseline:revoke",
            policy,
            provider,
            now,
        )?;
        self.transition(ArchitectureBaselineState::Revoked, Some(reference), None)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn supersede(
        &self,
        replacement: &Self,
        replacement_policy: &EligibilityPolicy,
        reference: &str,
        policy: &EligibilityPolicy,
        provider: &dyn AuthorityProvider,
        now: i64,
    ) -> Result<Self, String> {
        if self.state != ArchitectureBaselineState::Approved
            || replacement.content.predecessor.as_deref() != Some(&self.content_digest)
        {
            return Err("replacement must be a direct approved revision".into());
        }
        replacement.qualify(replacement_policy, provider, now)?;
        self.authenticate(
            reference,
            "architecture-baseline:supersede",
            policy,
            provider,
            now,
        )?;
        self.transition(
            ArchitectureBaselineState::Superseded,
            Some(reference),
            Some(replacement.content_digest.clone()),
        )
    }
}
