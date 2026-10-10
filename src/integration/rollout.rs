//! Stateless stage selection supplied by the protected caller. No rollout owner,
//! cached permission, external action, or weakening of a required check is provided.
use super::{
    projection::EvidenceBundle,
    trust::{ConsumptionPolicy, consume},
};
use guardengine::{
    GuardFacts,
    integration::eligibility::{AuditRecord, AuthorityProvider},
};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Advisory,
    Shadow,
    OptInProtected,
    Enforce,
}
pub struct Selection<'a> {
    pub stage: Stage,
    pub adapter_enabled: bool,
    /// Actual same-input legacy observation supplied by the protected caller.
    /// Legacy manifest identity differs from the enhanced inventory identity;
    /// compare facts and completeness, without claiming equivalent snapshots.
    pub native_facts: Option<&'a GuardFacts>,
}
#[derive(Debug, Serialize)]
pub struct StageResult {
    pub stage: Stage,
    pub adapter_enabled: bool,
    pub native_parity: bool,
    pub required_check_satisfied: bool,
    pub eligibility: bool,
    pub audit: AuditRecord,
}
pub fn check_stage(
    bundle: &EvidenceBundle,
    policy: &ConsumptionPolicy,
    authority: &dyn AuthorityProvider,
    now: i64,
    selection: Selection<'_>,
) -> StageResult {
    let result = consume(bundle, policy, authority, now, None);
    let observed = bundle
        .facts
        .as_ref()
        .and_then(|value| serde_json::from_value::<GuardFacts>(value.clone()).ok());
    let native_parity =
        selection
            .native_facts
            .zip(observed.as_ref())
            .is_some_and(|(native, enhanced)| {
                native.validate().is_ok()
                    && native.facts == enhanced.facts
                    && native.completeness == enhanced.completeness
            });
    StageResult {
        stage: selection.stage,
        adapter_enabled: selection.adapter_enabled,
        native_parity,
        required_check_satisfied: selection.adapter_enabled
            && native_parity
            && result.eligible
            && matches!(selection.stage, Stage::OptInProtected | Stage::Enforce),
        eligibility: result.eligible,
        audit: result.audit,
    }
}
