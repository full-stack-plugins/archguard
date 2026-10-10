//! Optional caller composition: AG domain qualification plus GG's generic required
//! specialist consumer. No authority provider, required-set owner or grant API.
use archguard::integration::{projection::EvidenceBundle, trust::ConsumptionPolicy};
use gitguard::{
    Repository,
    candidate::CandidateSnapshot,
    evidence::specialists::{
        ImmutableArtifactBytes, PreparedSpecialistConsumption, SpecialistConsumption,
    },
};
use guardengine::integration::{
    GuardRunEnvelope,
    eligibility::{AuthorityProvider, EligibilityResult},
};
use std::collections::BTreeMap;

/// Inputs frozen by the protected caller before dispatch. `id` selects the exact
/// AG entry in GG's unchanged required set. For Git source identity, callers first
/// verify the existing GitEvidenceBundle against this repository/candidate.
pub struct ArchitectureInput<'a> {
    pub id: &'a str,
    pub bundle: &'a EvidenceBundle,
    pub policy: &'a ConsumptionPolicy,
}

/// Both independently scoped audit records are retained; no new authority state.
pub struct ArchitectureSpecialistConsumption {
    architecture: EligibilityResult,
    specialists: SpecialistConsumption,
}
impl ArchitectureSpecialistConsumption {
    pub fn eligible(&self) -> bool {
        self.architecture.eligible && self.specialists.eligible()
    }
    pub fn architecture_result(&self) -> &EligibilityResult {
        &self.architecture
    }
    pub fn specialist_results(&self) -> &BTreeMap<String, EligibilityResult> {
        self.specialists.results()
    }
}

/// Freshly consume both ports before returning any usable GG success. GG remains
/// domain-neutral; an engine-valid envelope alone cannot qualify AG capabilities
/// or domain artifacts. Errors never remove an entry from the frozen required set.
#[allow(clippy::too_many_arguments)]
pub fn consume(
    required: &PreparedSpecialistConsumption,
    repo: &Repository,
    candidate: &CandidateSnapshot,
    envelopes: &BTreeMap<String, GuardRunEnvelope>,
    artifacts: &ImmutableArtifactBytes,
    architecture: ArchitectureInput<'_>,
    authority: &dyn AuthorityProvider,
    now: i64,
    cause: Option<&str>,
) -> Result<ArchitectureSpecialistConsumption, String> {
    let result = required.consume(repo, candidate, envelopes, artifacts, authority, now, cause)?;
    if envelopes.get(architecture.id) != Some(&architecture.bundle.envelope) {
        return Err("selected architecture envelope differs from required specialist".into());
    }
    if result
        .results()
        .get(architecture.id)
        .is_none_or(|selected| {
            selected.audit.content_digest != architecture.policy.eligibility.content_digest()
        })
    {
        return Err("architecture eligibility policy differs from frozen specialist policy".into());
    }
    let qualified = archguard::integration::trust::consume(
        architecture.bundle,
        architecture.policy,
        authority,
        now,
        cause,
    );
    if !qualified.eligible {
        return Err(format!(
            "architecture profile is ineligible: {:?}",
            qualified.code
        ));
    }
    Ok(ArchitectureSpecialistConsumption {
        architecture: qualified,
        specialists: result,
    })
}
