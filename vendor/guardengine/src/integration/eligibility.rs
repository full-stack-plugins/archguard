//! Protected-controller eligibility ports. No authority provider is selected here.
use super::{GuardRunEnvelope, Producer, RunBinding};
use crate::Decision;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize)]
pub struct EligibilityPolicy {
    pub binding: RunBinding,
    pub producer: Producer,
    pub required_scopes: Vec<String>,
    pub contract_digest: String,
    pub action: String,
    pub producer_principals: BTreeSet<String>,
    pub approval_principals: BTreeMap<String, BTreeSet<String>>,
}
#[derive(Clone, Debug)]
pub struct Validity {
    pub issued_at: i64,
    pub expires_at: i64,
    pub revoked: bool,
}
#[derive(Clone, Debug)]
pub struct ProducerRecord {
    pub principal: String,
    pub producer: Producer,
    pub envelope_digest: String,
    pub validity: Validity,
}
#[derive(Clone, Debug)]
pub struct ApprovalRecord {
    pub principal: String,
    pub purpose: String,
    pub action: String,
    pub binding: RunBinding,
    pub contract_digest: String,
    pub validity: Validity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityError {
    Untrusted,
    Unavailable,
}
pub trait AuthorityProvider {
    fn verify_producer(
        &self,
        envelope: &GuardRunEnvelope,
        envelope_digest: &str,
    ) -> Result<ProducerRecord, AuthorityError>;
    fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityCode {
    Eligible,
    InvalidPolicy,
    InvalidEvidence,
    BindingChanged,
    ExecutionFailed,
    Incomplete,
    Stale,
    ProviderUnavailable,
    UntrustedProducer,
    UnauthorizedProducer,
    InvalidApproval,
    MissingApproval,
    TechnicalBlock,
}
#[derive(Clone, Debug, Serialize)]
pub struct AuditRecord {
    pub code: EligibilityCode,
    pub actor_digest: Option<String>,
    pub approver_digests: Vec<String>,
    pub action_digest: String,
    pub purpose_digests: Vec<String>,
    pub binding_digest: String,
    pub content_digest: String,
    pub envelope_digest: String,
    pub cause_digest: Option<String>,
    pub timestamp: i64,
}
#[derive(Clone, Debug)]
pub struct EligibilityResult {
    pub eligible: bool,
    pub code: EligibilityCode,
    pub technical_decision: Option<Decision>,
    pub audit: AuditRecord,
}
pub struct ArtifactBytes<'a> {
    pub contract: &'a [u8],
    pub facts: &'a [u8],
    pub report: &'a [u8],
}
/// Evaluate using actual artifact bytes and freshly queried authority. `now` is a
/// trusted controller UTC Unix timestamp (seconds); validity is [issued, expires).
/// `cause` is an opaque predecessor identifier, hashed before audit serialization.
pub fn evaluate_eligibility(
    envelope: &GuardRunEnvelope,
    artifacts: ArtifactBytes<'_>,
    policy: &EligibilityPolicy,
    provider: &dyn AuthorityProvider,
    now: i64,
    cause: Option<&str>,
) -> EligibilityResult {
    use super::{CoverageStatus, EvidenceProfile, RunStatus, verify_engine_artifacts};
    use EligibilityCode::*;
    let envelope_digest = object_digest(envelope);
    let mut result = EligibilityResult {
        eligible: false,
        code: InvalidEvidence,
        technical_decision: None,
        audit: AuditRecord {
            code: InvalidEvidence,
            actor_digest: None,
            approver_digests: Vec::new(),
            action_digest: byte_digest(policy.action.as_bytes()),
            purpose_digests: policy
                .approval_principals
                .keys()
                .map(|s| byte_digest(s.as_bytes()))
                .collect(),
            binding_digest: object_digest(&envelope.binding),
            content_digest: policy.content_digest(),
            envelope_digest: envelope_digest.clone(),
            cause_digest: cause.map(|s| byte_digest(s.as_bytes())),
            timestamp: now,
        },
    };
    let code = (|| {
        if !policy.valid() {
            return InvalidPolicy;
        }
        if envelope.validate(EvidenceProfile::EngineBacked).is_err() {
            return InvalidEvidence;
        }
        if envelope.binding != policy.binding
            || envelope.producer != policy.producer
            || envelope.coverage.required_scopes != policy.required_scopes
            || envelope
                .artifacts
                .contract
                .as_ref()
                .is_some_and(|r| r.digest != policy.contract_digest)
        {
            return BindingChanged;
        }
        if envelope.run_status != RunStatus::Completed {
            return ExecutionFailed;
        }
        let report = match verify_engine_artifacts(
            envelope,
            artifacts.contract,
            artifacts.facts,
            artifacts.report,
        ) {
            Ok(r) => r,
            Err(_) => return InvalidEvidence,
        };
        result.technical_decision = Some(report.decision.clone());
        if envelope.coverage.status != CoverageStatus::Complete {
            return Incomplete;
        }
        // Compare nanoseconds so fractional timestamps cannot become prematurely valid.
        let now_nanos = i128::from(now) * 1_000_000_000;
        if timestamp_nanos(&envelope.finished_at).is_none_or(|t| t > now_nanos)
            || envelope
                .expires_at
                .as_ref()
                .is_some_and(|s| timestamp_nanos(s).is_none_or(|t| now_nanos >= t))
        {
            return Stale;
        }
        let producer = match provider.verify_producer(envelope, &envelope_digest) {
            Ok(p) => p,
            Err(AuthorityError::Unavailable) => return ProviderUnavailable,
            Err(AuthorityError::Untrusted) => return UntrustedProducer,
        };
        if producer.producer != policy.producer
            || producer.envelope_digest != envelope_digest
            || !producer.validity.current(now)
        {
            return UntrustedProducer;
        }
        result.audit.actor_digest = Some(byte_digest(producer.principal.as_bytes()));
        if !policy.producer_principals.contains(&producer.principal) {
            return UnauthorizedProducer;
        }
        match report.decision {
            Decision::Block => TechnicalBlock,
            Decision::Allow => Eligible,
            Decision::RequireApproval => {
                if policy.approval_principals.is_empty() {
                    return MissingApproval;
                }
                let mut satisfied = BTreeSet::new();
                let mut actors = BTreeSet::new();
                for reference in &envelope.approval_refs {
                    let approval = match provider.verify_approval(reference) {
                        Ok(a) => a,
                        Err(AuthorityError::Unavailable) => return ProviderUnavailable,
                        Err(AuthorityError::Untrusted) => return InvalidApproval,
                    };
                    if validate_approval_record(&approval, policy, &approval.purpose, now).is_err()
                    {
                        return InvalidApproval;
                    }
                    satisfied.insert(approval.purpose);
                    actors.insert(byte_digest(approval.principal.as_bytes()));
                }
                result.audit.approver_digests = actors.into_iter().collect();
                if policy
                    .approval_principals
                    .keys()
                    .all(|p| satisfied.contains(p))
                {
                    Eligible
                } else {
                    MissingApproval
                }
            }
        }
    })();
    result.eligible = code == Eligible;
    result.code = code;
    result.audit.code = code;
    result
}
/// Validate an already authenticated approval against a protected controller policy.
/// The caller obtains the record from its authority provider and supplies trusted UTC
/// Unix seconds. This does not authenticate the record, grant execution, or change a
/// technical decision. Validity is inclusive at issuance and exclusive at expiry.
pub fn validate_approval_record(
    record: &ApprovalRecord,
    policy: &EligibilityPolicy,
    purpose: &str,
    now: i64,
) -> Result<(), EligibilityCode> {
    if !policy.valid()
        || super::validation::validate_binding(&policy.binding).is_err()
        || !super::validation::digest(&policy.contract_digest)
    {
        return Err(EligibilityCode::InvalidPolicy);
    }
    if record.purpose != purpose
        || record.action != policy.action
        || record.binding != policy.binding
        || record.contract_digest != policy.contract_digest
        || !record.validity.current(now)
        || !policy
            .approval_principals
            .get(purpose)
            .is_some_and(|principals| principals.contains(&record.principal))
    {
        return Err(EligibilityCode::InvalidApproval);
    }
    Ok(())
}
impl EligibilityPolicy {
    fn valid(&self) -> bool {
        !(self.action.trim().is_empty()
            || self.producer_principals.is_empty()
            || self.producer_principals.iter().any(|p| p.trim().is_empty())
            || self
                .approval_principals
                .iter()
                .any(|(purpose, principals)| {
                    purpose.trim().is_empty()
                        || principals.is_empty()
                        || principals.iter().any(|p| p.trim().is_empty())
                }))
    }
    /// Versioned local Rust canonical key: compact typed Serde JSON in declaration
    /// order, with BTree collections sorted. Includes current authorization policy.
    pub fn content_digest(&self) -> String {
        object_digest(&("guard.eligibility/v1alpha1", self))
    }
}
impl Validity {
    fn current(&self, now: i64) -> bool {
        !self.revoked && self.issued_at <= now && now < self.expires_at
    }
}
fn timestamp_nanos(s: &str) -> Option<i128> {
    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .ok()
        .map(|t| t.unix_timestamp_nanos())
}
fn byte_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}
fn object_digest(value: &impl Serialize) -> String {
    // All callers serialize closed structures containing strings, integers and enums.
    byte_digest(&serde_json::to_vec(value).expect("closed trust types serialize"))
}
