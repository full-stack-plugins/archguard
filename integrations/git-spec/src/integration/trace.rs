//! Explicit fixture-only SG references attached to real candidate Cargo evidence.
use super::{
    binding::{GitCargoEvidence, GitEvidenceBundle},
    projection::{CargoDomain, ProtectedCargoPolicy, bytes, diagnostic},
};
use crate::domain::baseline::{ArchitectureTrace, ProtectedTraceMapping, TraceReceipt};
use gitguard::{Repository, candidate::CandidateSnapshot};
use guardengine::integration::{RunBinding, RunStatus, TransportDiagnostic};
use serde::{Deserialize, Serialize};
use specguard::integration::approval::ApprovalValidationPort;
use std::sync::atomic::AtomicBool;
const VERSION: &str = "archguard.spec-trace-evidence/v1alpha1";
pub struct TracedCargoEvidence {
    git: GitCargoEvidence,
    trace: ArchitectureTrace,
}
impl TracedCargoEvidence {
    pub fn prepare(
        repo: &Repository,
        candidate: &CandidateSnapshot,
        policy: ProtectedCargoPolicy,
        trace: ArchitectureTrace,
    ) -> Result<Self, TransportDiagnostic> {
        if trace.receipt.candidate_digest != candidate.binding_digest()
            || trace.receipt.policy_digest != policy.profile_digest()
        {
            return Err(diagnostic(
                "trace.binding",
                "trace is for another candidate or policy",
            ));
        }
        let key = trace
            .identity_key()
            .map_err(|_| diagnostic("trace.budget", "trace identity budget"))?;
        let body = bytes(&trace.receipt)
            .map_err(|_| diagnostic("trace.budget", "trace identity budget"))?;
        let git =
            GitCargoEvidence::prepare_with_identity(repo, candidate, policy, vec![(key, body)])?;
        Ok(Self { git, trace })
    }
    pub fn binding(&self) -> &RunBinding {
        self.git.binding()
    }
    pub fn run(self, cancelled: &AtomicBool) -> Result<TracedEvidenceBundle, TransportDiagnostic> {
        let result = TracedEvidenceBundle {
            api_version: VERSION.into(),
            trace: self.trace.receipt,
            git: self.git.run(cancelled)?,
        };
        bytes(&result)
            .map_err(|_| diagnostic("trace.output", "traced evidence budget exceeded"))?;
        Ok(result)
    }
}
#[derive(Serialize)]
pub struct TracedEvidenceBundle {
    api_version: String,
    trace: TraceReceipt,
    git: GitEvidenceBundle,
}
impl TracedEvidenceBundle {
    pub fn git(&self) -> &GitEvidenceBundle {
        &self.git
    }
    pub fn baseline(&self) -> &specguard::baseline::ApprovedBaseline {
        &self.trace.baseline
    }
    pub fn references(&self) -> &[specguard::architecture::ArchitectureReference] {
        &self.trace.references
    }
    pub fn baseline_obligations(&self) -> &specguard::obligations::ObligationSet {
        &self.trace.baseline_obligations
    }
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        input: &[u8],
        sg_input: &[u8],
        mapping: &ProtectedTraceMapping,
        repo: &Repository,
        candidate: &CandidateSnapshot,
        port: &dyn ApprovalValidationPort,
        now: i64,
    ) -> Result<Self, String> {
        if input.len() > guardengine::integration::MAX_ARTIFACT_BYTES {
            return Err("traced bundle budget exceeded".into());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            api_version: String,
            trace: TraceReceipt,
            #[serde(deserialize_with = "GitEvidenceBundle::deserialize_unverified")]
            git: GitEvidenceBundle,
        }
        let raw: Wire =
            serde_json::from_slice(input).map_err(|_| "invalid strict traced evidence")?;
        let result = Self {
            api_version: raw.api_version,
            trace: raw.trace,
            git: raw.git,
        };
        result.verify(sg_input, mapping, repo, candidate, port, now)?;
        Ok(result)
    }
    pub fn verify(
        &self,
        sg_input: &[u8],
        mapping: &ProtectedTraceMapping,
        repo: &Repository,
        candidate: &CandidateSnapshot,
        port: &dyn ApprovalValidationPort,
        now: i64,
    ) -> Result<(), String> {
        if self.api_version != VERSION {
            return Err("unsupported traced evidence version".into());
        }
        let actual = ArchitectureTrace::read(sg_input, mapping, repo, candidate, port, now)?;
        if self.trace != actual.receipt {
            return Err("trace receipt differs from protected source context".into());
        }
        self.git.verify(repo, candidate)?;
        if self.git.cargo().envelope.run_status == RunStatus::Completed {
            let domain: CargoDomain = serde_json::from_value(
                self.git
                    .cargo()
                    .domain
                    .clone()
                    .ok_or("missing Cargo domain")?,
            )
            .map_err(|_| "invalid Cargo domain")?;
            let expected = format!("identity:{}", actual.identity_key()?);
            let keys = domain
                .inventory_keys
                .iter()
                .filter(|k| k.starts_with("identity:specguard"));
            if !keys.eq(std::iter::once(&expected)) {
                return Err("SG trace inventory identity mismatch".into());
            }
        }
        Ok(())
    }
}
