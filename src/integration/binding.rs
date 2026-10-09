//! Actual local Git candidate source bridge. Object verification is not producer
//! authentication, queue admission, baseline approval or authorization to write.
use super::{evidence::CargoEvidence, projection::*};
use crate::analysis::{PreparedAnalysis, snapshot::SnapshotInventory};
use gitguard::{Repository, candidate::CandidateSnapshot};
use guardengine::integration::{RunStatus, TransportDiagnostic};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
const VERSION: &str = "archguard.git-cargo-evidence/v1alpha1";
const MAX_CONTEXT: usize = 65536;

fn check(repo: &Repository, candidate: &CandidateSnapshot) -> Result<(), TransportDiagnostic> {
    if bytes(candidate)
        .map_err(|_| diagnostic("candidate.invalid", "invalid candidate"))?
        .len()
        > MAX_CONTEXT
    {
        return Err(diagnostic("candidate.invalid", "candidate budget exceeded"));
    }
    candidate
        .validate(repo)
        .map_err(|_| diagnostic("candidate.invalid", "GitGuard candidate validation failed"))
}
fn identity_key(candidate: &CandidateSnapshot) -> String {
    format!("gitguard.binding:{}", candidate.binding_digest())
}
fn files(
    repo: &Repository,
    candidate: &CandidateSnapshot,
) -> Result<SnapshotInventory, TransportDiagnostic> {
    let entries = repo
        .read_commit_files(candidate.candidate_oid())
        .map_err(|_| diagnostic("candidate.source", "Git candidate tree cannot be read"))?;
    let mut inputs = Vec::new();
    for entry in entries {
        let path = std::str::from_utf8(entry.path())
            .map_err(|_| diagnostic("candidate.source", "non UTF-8 candidate path"))?;
        if path.split('/').count() > 64
            || path
                .split('/')
                .any(|s| matches!(s, "target" | ".git") || s.is_empty() || s == "." || s == "..")
            || path.contains(['\\', ':'])
        {
            return Err(diagnostic("candidate.source", "unsupported candidate path"));
        }
        inputs.push((format!("file:{path}"), entry.contents().to_vec()));
    }
    SnapshotInventory::from_inputs(inputs)
        .map_err(|_| diagnostic("candidate.source", "invalid source inventory"))
}
/// Owns a prepared isolated analysis. No mutable worktree content is analyzed.
pub struct GitCargoEvidence {
    cargo: CargoEvidence,
    candidate: CandidateSnapshot,
    object_format: String,
    files_digest: String,
}
impl GitCargoEvidence {
    /// Exact prepared Cargo binding, including its independently scoped source digest.
    pub fn binding(&self) -> &guardengine::integration::RunBinding {
        self.cargo.binding()
    }

    pub fn prepare(
        repo: &Repository,
        candidate: &CandidateSnapshot,
        policy: ProtectedCargoPolicy,
    ) -> Result<Self, TransportDiagnostic> {
        check(repo, candidate)?;
        if policy.profile_digest().strip_prefix("sha256:") != Some(candidate.policy_digest()) {
            return Err(diagnostic(
                "candidate.policy",
                "Git task scope and Cargo policy differ",
            ));
        }
        let inventory = files(repo, candidate)?;
        let files_digest = inventory.digest();
        let root = tempfile::tempdir()
            .map_err(|_| diagnostic("source.unavailable", "private source unavailable"))?;
        inventory
            .materialize_files(root.path())
            .map_err(|_| diagnostic("source.invalid", "candidate source materialization failed"))?;
        let object_format = repo.object_format().to_owned();
        let analysis = PreparedAnalysis::prepare_bound(
            root.path(),
            &policy.profile,
            vec![
                (
                    identity_key(candidate),
                    bytes(&(candidate, &object_format))
                        .map_err(|_| diagnostic("candidate.invalid", "invalid candidate"))?,
                ),
                (
                    format!("gitguard.files:{files_digest}"),
                    files_digest.as_bytes().to_vec(),
                ),
            ],
        )
        .map_err(|_| {
            diagnostic(
                "source.unresolved",
                "candidate Cargo source preparation failed",
            )
        })?;
        if analysis.files_digest() != files_digest {
            return Err(diagnostic(
                "source.mismatch",
                "analyzed files differ from candidate tree",
            ));
        }
        let context = CandidateContext {
            repo_id: candidate.repo_id().into(),
            task_id: candidate.task_id().into(),
            worktree_id: candidate.worktree_id().into(),
            requirement_ids: candidate.requirement_ids().to_vec(),
            candidate_oid: candidate.candidate_oid().into(),
            base_oid: candidate.base_oid().into(),
            merge_group_id: candidate.merge_group_id().map(str::to_owned),
        };
        let cargo = CargoEvidence::from_analysis(analysis, policy, context)?;
        Ok(Self {
            cargo,
            candidate: candidate.clone(),
            object_format,
            files_digest,
        })
    }
    pub fn run(self, cancelled: &AtomicBool) -> Result<GitEvidenceBundle, TransportDiagnostic> {
        Ok(GitEvidenceBundle {
            api_version: VERSION.into(),
            candidate: self.candidate,
            object_format: self.object_format,
            files_digest: self.files_digest,
            cargo: self.cargo.run(cancelled)?,
        })
    }
}
#[derive(Serialize)]
pub struct GitEvidenceBundle {
    api_version: String,
    candidate: CandidateSnapshot,
    object_format: String,
    files_digest: String,
    cargo: EvidenceBundle,
}
impl GitEvidenceBundle {
    pub fn cargo(&self) -> &EvidenceBundle {
        &self.cargo
    }
    pub fn candidate(&self) -> &CandidateSnapshot {
        &self.candidate
    }
    pub fn object_format(&self) -> &str {
        &self.object_format
    }
    pub fn files_digest(&self) -> &str {
        &self.files_digest
    }
    pub fn load(
        input: &[u8],
        repo: &Repository,
        expected: &CandidateSnapshot,
    ) -> Result<Self, String> {
        if input.len()
            > 4 * guardengine::integration::MAX_ARTIFACT_BYTES + 1024 * 1024 + MAX_CONTEXT
        {
            return Err("Git bundle budget exceeded".into());
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            api_version: String,
            candidate: CandidateSnapshot,
            object_format: String,
            files_digest: String,
            cargo: EvidenceBundle,
        }
        let wire: Wire =
            serde_json::from_slice(input).map_err(|_| "invalid Git evidence bundle")?;
        let result = Self {
            api_version: wire.api_version,
            candidate: wire.candidate,
            object_format: wire.object_format,
            files_digest: wire.files_digest,
            cargo: wire.cargo,
        };
        result.verify(repo, expected)?;
        Ok(result)
    }
    pub fn verify(&self, repo: &Repository, expected: &CandidateSnapshot) -> Result<(), String> {
        check(repo, expected).map_err(|_| "expected Git candidate invalid")?;
        if self.api_version != VERSION
            || self.object_format != repo.object_format()
            || bytes(&self.candidate)?.len() > MAX_CONTEXT
            || self.candidate.binding_digest() != expected.binding_digest()
        {
            return Err("Git candidate binding mismatch".into());
        }
        self.cargo.verify()?;
        let b = &self.cargo.envelope.binding;
        if b.repo_id != expected.repo_id()
            || b.task_id != expected.task_id()
            || b.worktree_id != expected.worktree_id()
            || b.requirement_ids != expected.requirement_ids()
            || b.candidate_oid != expected.candidate_oid()
            || b.base_oid != expected.base_oid()
            || b.merge_group_id.as_deref() != expected.merge_group_id()
            || b.baseline_digest.is_some()
            || !self
                .cargo
                .envelope
                .coverage
                .required_scopes
                .contains(&format!(
                    "cargo.profile:sha256:{}",
                    expected.policy_digest()
                ))
        {
            return Err("Cargo binding differs from Git candidate".into());
        }
        let file_inventory = files(repo, expected).map_err(|_| "candidate files unavailable")?;
        if file_inventory.digest() != self.files_digest {
            return Err("Git file digest mismatch".into());
        }
        if self.cargo.envelope.run_status == RunStatus::Completed {
            let domain: CargoDomain =
                serde_json::from_value(self.cargo.domain.clone().ok_or("missing domain")?)
                    .map_err(|_| "invalid domain")?;
            let declared_files = domain
                .inventory_keys
                .iter()
                .filter(|key| key.starts_with("file:"))
                .map(String::as_str);
            if !declared_files.eq(file_inventory.keys()) {
                return Err("Cargo file inventory differs from actual candidate tree".into());
            }
            // Reserve the entire GitGuard namespace. Presence checks would
            // allow a second contradictory candidate or file identity.
            let expected_git_keys = [
                format!("identity:{}", identity_key(expected)),
                format!("identity:gitguard.files:{}", self.files_digest),
            ];
            let declared_git_keys = domain
                .inventory_keys
                .iter()
                .filter(|key| key.starts_with("identity:gitguard"));
            if !declared_git_keys.eq(expected_git_keys.iter()) {
                return Err("Cargo Git identity inventory differs from expected binding".into());
            }
        }
        Ok(())
    }
}
