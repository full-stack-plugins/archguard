//! Preparation is separate from outcomes: unresolved inputs never acquire a fabricated binding.
use super::*;
/// Domain/controller resolution outputs; no fields here prove authority.
pub struct InvocationDraft {
    pub run_id: String,
    pub producer: Option<Producer>,
    pub binding: Option<RunBinding>,
    pub coverage: Option<Coverage>,
    pub profile: Option<EvidenceProfile>,
    pub started_at: String,
}
pub struct AttemptOutput {
    pub coverage: Coverage,
    pub run_status: RunStatus,
    pub decision: Option<crate::Decision>,
    pub artifacts: Artifacts,
    pub approval_refs: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub finished_at: String,
    pub expires_at: Option<String>,
}
/// Validated immutable invocation context. No public mutation or serialization before completion.
pub struct BoundAttempt {
    envelope: GuardRunEnvelope,
    profile: EvidenceProfile,
}
fn transport(code: &str, message: &str) -> TransportDiagnostic {
    TransportDiagnostic {
        code: code.into(),
        message: message.into(),
    }
}
pub fn prepare_attempt(draft: InvocationDraft) -> Result<BoundAttempt, TransportDiagnostic> {
    let binding = draft
        .binding
        .ok_or_else(|| transport("binding.unresolved", "invocation binding is unresolved"))?;
    let producer = draft
        .producer
        .ok_or_else(|| transport("producer.unresolved", "producer is unresolved"))?;
    let coverage = draft
        .coverage
        .ok_or_else(|| transport("coverage.unresolved", "required coverage is unresolved"))?;
    let profile = draft
        .profile
        .ok_or_else(|| transport("profile.unresolved", "evidence profile is unresolved"))?;
    // This internal validation template is neither serialized nor exposed as a run result.
    let envelope = GuardRunEnvelope {
        api_version: INTEGRATION_VERSION.into(),
        kind: "GuardRunEnvelope".into(),
        run_id: draft.run_id,
        producer,
        binding,
        run_status: RunStatus::Error,
        decision: None,
        coverage,
        artifacts: Artifacts {
            contract: None,
            facts: None,
            report: None,
            domain: vec![],
        },
        approval_refs: vec![],
        diagnostics: vec![Diagnostic {
            code: "attempt.prepared".into(),
            message: "internal binding validation".into(),
            retryable: false,
            source: None,
        }],
        finished_at: draft.started_at.clone(),
        started_at: draft.started_at,
        expires_at: None,
    };
    envelope
        .validate(profile)
        .map_err(|_| transport("binding.invalid", "resolved invocation is invalid"))?;
    Ok(BoundAttempt { envelope, profile })
}
impl BoundAttempt {
    pub fn finish(
        mut self,
        output: AttemptOutput,
    ) -> Result<GuardRunEnvelope, TransportDiagnostic> {
        if output.coverage.required_scopes != self.envelope.coverage.required_scopes {
            return Err(transport(
                "coverage.changed",
                "attempt outcome changed frozen required scopes",
            ));
        }
        self.envelope.coverage = output.coverage;
        self.envelope.run_status = output.run_status;
        self.envelope.decision = output.decision;
        self.envelope.artifacts = output.artifacts;
        self.envelope.approval_refs = output.approval_refs;
        self.envelope.diagnostics = output.diagnostics;
        self.envelope.finished_at = output.finished_at;
        self.envelope.expires_at = output.expires_at;
        self.envelope.validate(self.profile).map_err(|_| {
            transport(
                "attempt.invalid",
                "attempt outcome does not satisfy its bound profile",
            )
        })?;
        Ok(self.envelope)
    }
}
