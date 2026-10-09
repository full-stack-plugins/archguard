//! Opt-in library producer over actual frozen Cargo declaration observation.
use super::projection::*;
use crate::analysis::PreparedAnalysis;
use guardengine::integration::{
    self, Artifacts, AttemptOutput, BoundAttempt, Diagnostic, EvidenceProfile, InvocationDraft,
    Producer, RunBinding, RunStatus, TransportDiagnostic,
};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
fn now() -> Result<String, TransportDiagnostic> {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| diagnostic("clock.unavailable", "observation clock unavailable"))
}
pub struct CargoEvidence {
    analysis: PreparedAnalysis,
    policy: ProtectedCargoPolicy,
    binding: RunBinding,
    attempt: BoundAttempt,
}
impl CargoEvidence {
    /// Frozen before execution; this observation conveys no admission authority.
    pub fn binding(&self) -> &RunBinding {
        &self.binding
    }

    pub fn prepare(
        root: &Path,
        policy: ProtectedCargoPolicy,
        context: CandidateContext,
    ) -> Result<Self, TransportDiagnostic> {
        if bytes(&context)
            .map_err(|_| diagnostic("binding.invalid", "oversized candidate context"))?
            .len()
            > 65536
        {
            return Err(diagnostic("binding.invalid", "oversized candidate context"));
        }
        let analysis = PreparedAnalysis::prepare(root, &policy.profile)
            .map_err(|_| diagnostic("source.unresolved", "Cargo source preparation failed"))?;
        Self::from_analysis(analysis, policy, context)
    }
    pub(crate) fn from_analysis(
        analysis: PreparedAnalysis,
        policy: ProtectedCargoPolicy,
        context: CandidateContext,
    ) -> Result<Self, TransportDiagnostic> {
        let binding = RunBinding {
            repo_id: context.repo_id,
            task_id: context.task_id,
            worktree_id: context.worktree_id,
            requirement_ids: context.requirement_ids,
            candidate_oid: context.candidate_oid,
            base_oid: context.base_oid,
            merge_group_id: context.merge_group_id,
            source_snapshot_digest: analysis.snapshot_digest(),
            baseline_digest: None,
        };
        let nonce = tempfile::tempdir()
            .map_err(|_| diagnostic("identity.unavailable", "run identity unavailable"))?;
        let run_id = digest(nonce.path().as_os_str().as_encoded_bytes());
        let attempt = integration::prepare_attempt(InvocationDraft {
            run_id,
            binding: Some(binding.clone()),
            producer: Some(Producer {
                guard: "ArchGuard".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                analyzer_id: "archguard.cargo.declarations".into(),
                analyzer_version: env!("CARGO_PKG_VERSION").into(),
            }),
            coverage: Some(policy.coverage(None)),
            profile: Some(EvidenceProfile::EngineBacked),
            started_at: now()?,
        })?;
        Ok(Self {
            analysis,
            policy,
            binding,
            attempt,
        })
    }
    pub fn run(self, cancelled: &AtomicBool) -> Result<EvidenceBundle, TransportDiagnostic> {
        let inventory_keys = self.analysis.inventory_keys();
        let result = if cancelled.load(Ordering::SeqCst) {
            Err("cancelled".to_owned())
        } else {
            (|| -> Result<_, String> {
                let outcome = self
                    .analysis
                    .observe(&format!("cargo:{}", digest(&bytes(&self.binding)?)))
                    .map_err(|_| "Cargo observation failed")?;
                let domain = CargoDomain {
                    api_version: DOMAIN_VERSION.into(),
                    profile: PROFILE.into(),
                    profile_digest: self.policy.profile_digest.clone(),
                    candidate_binding: digest(&bytes(&self.binding)?),
                    advisory: true,
                    authority: "controller-resolved-unverified".into(),
                    source_snapshot_digest: outcome.facts.subject.snapshot_digest.clone(),
                    manifest_digest: outcome.observation.manifest_digest,
                    inventory_keys,
                    required_members: self.policy.profile.members().clone(),
                    required_relations: self.policy.profile.relations().clone(),
                    observed_members: outcome.observation.members,
                    declarations: outcome.observation.declarations,
                };
                let coverage = self.policy.coverage(Some(&domain.observed_members));
                let domain = value(&domain)?;
                let contract = value(&self.policy.contract)?;
                let facts = value(&outcome.facts)?;
                let report = integration::evaluate_bounded(&self.policy.contract, &outcome.facts)
                    .map_err(|_| "evaluation budget or validation failed")?;
                let decision = report.decision.clone();
                let report = value(&report)?;
                Ok((domain, contract, facts, report, decision, coverage))
            })()
        };
        let mut artifacts = Artifacts {
            contract: None,
            facts: None,
            report: None,
            domain: vec![],
        };
        let mut bundle = (None, None, None, None);
        let mut coverage = self.policy.coverage(None);
        let (run_status, decision, diagnostics) = if cancelled.load(Ordering::SeqCst) {
            (
                RunStatus::Cancelled,
                None,
                vec![Diagnostic {
                    code: "run.cancelled".into(),
                    message: "Cargo evidence cancelled".into(),
                    retryable: true,
                    source: None,
                }],
            )
        } else {
            match result {
                Ok((domain, contract, facts, report, decision, observed)) => {
                    let artifact_error =
                        |_| diagnostic("artifact.invalid", "artifact publication failed");
                    artifacts.contract =
                        Some(reference("contract", &contract).map_err(artifact_error)?);
                    artifacts.facts = Some(reference("facts", &facts).map_err(artifact_error)?);
                    artifacts.report = Some(reference("report", &report).map_err(artifact_error)?);
                    artifacts
                        .domain
                        .push(reference("domain", &domain).map_err(artifact_error)?);
                    coverage = observed;
                    bundle = (Some(contract), Some(facts), Some(report), Some(domain));
                    (RunStatus::Completed, Some(decision), vec![])
                }
                Err(_) => (
                    RunStatus::Error,
                    None,
                    vec![Diagnostic {
                        code: "observation.failed".into(),
                        message: "Cargo observation or bounded evaluation failed".into(),
                        retryable: true,
                        source: None,
                    }],
                ),
            }
        };
        let envelope = self.attempt.finish(AttemptOutput {
            coverage,
            run_status,
            decision,
            artifacts,
            approval_refs: vec![],
            diagnostics,
            finished_at: now()?,
            expires_at: None,
        })?;
        let result = EvidenceBundle {
            api_version: BUNDLE_VERSION.into(),
            envelope,
            contract: bundle.0,
            facts: bundle.1,
            report: bundle.2,
            domain: bundle.3,
        };
        result
            .verify()
            .map_err(|_| diagnostic("evidence.invalid", "produced evidence failed verification"))?;
        Ok(result)
    }
}
