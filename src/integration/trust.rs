//! Protected local Cargo consumption. The caller owns policy, clock and authority.
//! Integrity checks never authenticate records; no provider or credential is installed.
use super::projection::{EvidenceBundle, bytes, digest};
use guardengine::integration::eligibility::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// The complete advertised capabilities of this consumer, not every native analyzer.
pub const CAPABILITIES: &[&str] = &["cargo.declarations", "cargo.depends_on"];

#[derive(Clone, Serialize)]
pub struct ConsumptionPolicy {
    pub eligibility: EligibilityPolicy,
    pub required_capabilities: BTreeSet<String>,
    /// Protected purpose -> external authority reference. Checked even for ALLOW.
    /// Baseline and architecture exceptions remain distinct purpose/scope records.
    pub required_approvals: BTreeMap<String, String>,
}
impl ConsumptionPolicy {
    pub fn content_digest(&self) -> String {
        digest(
            &serde_json::to_vec(&("archguard.consumption/v1alpha1", self))
                .expect("closed policy serializes"),
        )
    }
}
fn deny(result: &mut EligibilityResult, code: EligibilityCode) {
    result.eligible = false;
    result.code = code;
    result.audit.code = code;
}
/// Rechecks current authority on each call without changing the envelope/report.
/// All URI references must match the canonical artifact://archguard digest paths.
pub fn consume(
    bundle: &EvidenceBundle,
    policy: &ConsumptionPolicy,
    authority: &dyn AuthorityProvider,
    now: i64,
    cause: Option<&str>,
) -> EligibilityResult {
    let artifact =
        |v: &Option<serde_json::Value>| v.as_ref().and_then(|v| bytes(v).ok()).unwrap_or_default();
    let contract = artifact(&bundle.contract);
    let facts = artifact(&bundle.facts);
    let report = artifact(&bundle.report);
    let mut result = evaluate_eligibility(
        &bundle.envelope,
        ArtifactBytes {
            contract: &contract,
            facts: &facts,
            report: &report,
        },
        &policy.eligibility,
        authority,
        now,
        cause,
    );
    result.audit.content_digest = policy.content_digest();
    if bundle.verify().is_err() {
        deny(&mut result, EligibilityCode::InvalidEvidence);
    } else if policy.required_capabilities.is_empty()
        || policy
            .required_capabilities
            .iter()
            .any(|c| !CAPABILITIES.contains(&c.as_str()))
        || policy
            .required_approvals
            .keys()
            .any(|p| !policy.eligibility.approval_principals.contains_key(p))
    {
        deny(&mut result, EligibilityCode::InvalidPolicy);
    } else if result.eligible {
        for (purpose, reference) in &policy.required_approvals {
            let record = match authority.verify_approval(reference) {
                Ok(record) => record,
                Err(error) => {
                    deny(
                        &mut result,
                        match error {
                            AuthorityError::Unavailable => EligibilityCode::ProviderUnavailable,
                            AuthorityError::Untrusted => EligibilityCode::InvalidApproval,
                        },
                    );
                    break;
                }
            };
            if let Err(code) = validate_approval_record(&record, &policy.eligibility, purpose, now)
            {
                deny(&mut result, code);
                break;
            }
            result
                .audit
                .approver_digests
                .push(digest(record.principal.as_bytes()));
        }
        result.audit.approver_digests.sort();
        result.audit.approver_digests.dedup();
    }
    result
}
