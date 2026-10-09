//! Versioned integration contracts, separate from the engine protocol.
mod adapter;
mod model;
mod validation;
mod yaml_validation;
pub use adapter::{evaluate_bounded, verify_engine_artifacts};
pub use model::*;
pub use validation::load_envelope_json;

pub const INTEGRATION_VERSION: &str = "guard.integration/v1alpha1";
pub const MAX_ENVELOPE_BYTES: usize = 1_048_576;
pub const MAX_ARTIFACT_BYTES: usize = 16_777_216;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceProfile {
    EngineBacked,
    NativeOnly,
}
#[derive(Debug, thiserror::Error)]
#[error("integration validation failed: {0}")]
pub struct IntegrationError(pub String);
fn fail<T>(message: &str) -> Result<T, IntegrationError> {
    Err(IntegrationError(message.into()))
}
pub mod attempt_store;
pub mod eligibility;

mod attempt;
pub use attempt::{AttemptOutput, BoundAttempt, InvocationDraft, prepare_attempt};

mod publication;
pub use publication::{PublishedAttempt, StagedAttempt, stage_attempt};

mod fact_budget;
pub use fact_budget::FactBudget;
