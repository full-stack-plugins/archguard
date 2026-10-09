//! Process-local attempt consistency, never producer or approval authentication.
//! No extraction cache, durable current pointer or restart recovery is provided.
use super::{GitCargoEvidence, GitEvidenceBundle};
use crate::integration::projection::{ProtectedCargoPolicy, bytes, digest, value};
use gitguard::{Repository, candidate::CandidateSnapshot};
pub use guardengine::integration::attempt_store::AppendOutcome;
use guardengine::integration::{
    RunBinding, RunStatus,
    attempt_store::{AttemptRecord, AttemptStore, InMemoryAttemptStore, Target},
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, atomic::AtomicBool},
};

const MAX_WORK: usize = 256 * 1024;
const MAX_RUNS: usize = 256;
const MAX_BUNDLE: usize = 4 * guardengine::integration::MAX_ARTIFACT_BYTES + 2 * 1024 * 1024;

/// Count encoded bytes without allocating proportional to borrowed untrusted input.
fn admit<T: Serialize>(input: &T, limit: usize) -> Result<(), String> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_sub(data.len())
                .ok_or_else(|| std::io::Error::other("attempt budget"))?;
            Ok(data.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    serde_json::to_writer(Counter(limit), input).map_err(|_| "attempt input budget exceeded".into())
}
fn candidate_budget(candidate: &CandidateSnapshot) -> Result<(), String> {
    if candidate.requirement_ids().len() > 64 {
        return Err("requirement budget exceeded".into());
    }
    admit(candidate, 64 * 1024)
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct FullTarget {
    repo: String,
    task: String,
    worktree: String,
    requirements: Vec<String>,
}
impl FullTarget {
    fn from_binding(b: &RunBinding) -> Self {
        Self {
            repo: b.repo_id.clone(),
            task: b.task_id.clone(),
            worktree: b.worktree_id.clone(),
            requirements: b.requirement_ids.clone(),
        }
    }
    fn engine(&self) -> Target {
        Target {
            repo_id: self.repo.clone(),
            task_id: self.task.clone(),
            requirement_ids: self.requirements.clone(),
        }
    }
}
#[derive(Serialize)]
struct Work {
    binding: RunBinding,
    candidate_digest: String,
    profile_digest: String,
    contract_digest: String,
    required_scopes: Vec<String>,
    analyzer: &'static str,
    version: &'static str,
}
/// A real prepared immutable Git analysis, frozen before registration/execution.
/// Values can only originate from the existing candidate verifier and runner.
pub struct PreparedGitAttempt {
    analysis: GitCargoEvidence,
    work: Work,
    key: String,
    target: FullTarget,
}
impl PreparedGitAttempt {
    pub fn prepare(
        repo: &Repository,
        candidate: &CandidateSnapshot,
        policy: ProtectedCargoPolicy,
    ) -> Result<Self, String> {
        candidate_budget(candidate)?;
        admit(
            &(
                &policy.contract,
                &policy.profile_digest,
                &policy.profile.members(),
                &policy.profile.relations(),
            ),
            MAX_WORK,
        )?;
        let contract_digest = digest(&bytes(&value(&policy.contract)?)?);
        let profile_digest = policy.profile_digest().to_owned();
        let required_scopes = policy.coverage(None).required_scopes;
        let analysis = GitCargoEvidence::prepare(repo, candidate, policy)
            .map_err(|_| "candidate preparation failed")?;
        admit(analysis.binding(), 64 * 1024)?;
        let work = Work {
            binding: analysis.binding().clone(),
            candidate_digest: candidate.binding_digest(),
            profile_digest,
            contract_digest,
            required_scopes,
            analyzer: "archguard.cargo.declarations",
            version: env!("CARGO_PKG_VERSION"),
        };
        admit(&work, MAX_WORK)?;
        let key = digest(&bytes(&work)?);
        let target = FullTarget::from_binding(&work.binding);
        Ok(Self {
            analysis,
            work,
            key,
            target,
        })
    }
    pub fn run_id(&self) -> &str {
        self.analysis.run_id()
    }
    /// Full-work deduplication identity; never a source-only cache hit or authority.
    pub fn dedup_key(&self) -> &str {
        &self.key
    }
    pub fn binding(&self) -> &RunBinding {
        &self.work.binding
    }
}
struct Ticket {
    owner: Arc<()>,
    generation: u64,
    run_id: String,
    key: String,
    target: FullTarget,
}
/// Opaque local registration; execution takes ownership and cannot change its run.
pub struct RegisteredGitAttempt {
    ticket: Ticket,
    prepared: PreparedGitAttempt,
}
impl RegisteredGitAttempt {
    pub fn run_id(&self) -> &str {
        &self.ticket.run_id
    }
    pub fn generation(&self) -> u64 {
        self.ticket.generation
    }
    pub fn run(self, cancelled: &AtomicBool) -> Result<CompletedGitAttempt, String> {
        let bundle = self
            .prepared
            .analysis
            .run(cancelled)
            .map_err(|_| "attempt execution failed")?;
        Ok(CompletedGitAttempt {
            ticket: self.ticket,
            work: self.prepared.work,
            bundle,
        })
    }
}
/// Sealed in-process completion, not a signed producer attestation.
/// There is deliberately no Deserialize or mutable bundle access.
/// ```compile_fail
/// use archguard::integration::binding::attempts::CompletedGitAttempt;
/// let _: CompletedGitAttempt = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use archguard::integration::binding::attempts::CompletedGitAttempt;
/// fn relabel(result: &mut CompletedGitAttempt) {
///     result.bundle().cargo().envelope.run_id.clear();
/// }
/// ```
pub struct CompletedGitAttempt {
    ticket: Ticket,
    work: Work,
    bundle: GitEvidenceBundle,
}
impl CompletedGitAttempt {
    pub fn bundle(&self) -> &GitEvidenceBundle {
        &self.bundle
    }
    pub fn run_id(&self) -> &str {
        &self.ticket.run_id
    }
    pub fn generation(&self) -> u64 {
        self.ticket.generation
    }
}
/// Controller-owned local history. Mutations are exclusive; a caller may put
/// this value in a Mutex to serialize real threads. New instances have no current.
pub struct AttemptHistory {
    owner: Arc<()>,
    stores: BTreeMap<FullTarget, InMemoryAttemptStore>,
    runs: BTreeSet<String>,
}
impl Default for AttemptHistory {
    fn default() -> Self {
        Self::new()
    }
}
impl AttemptHistory {
    pub fn new() -> Self {
        Self {
            owner: Arc::new(()),
            stores: BTreeMap::new(),
            runs: BTreeSet::new(),
        }
    }
    pub fn register(
        &mut self,
        prepared: PreparedGitAttempt,
        expected_generation: u64,
    ) -> Result<RegisteredGitAttempt, String> {
        if self.runs.len() >= MAX_RUNS {
            return Err("attempt capacity exceeded".into());
        }
        let run_id = prepared.run_id();
        if run_id.len() > 128 || self.runs.contains(run_id) {
            return Err("run identity conflict".into());
        }
        let run_id = run_id.to_owned();
        let target = prepared.target.clone();
        // Do not retain empty stores when an initial compare-and-swap fails.
        if !self.stores.contains_key(&target) && expected_generation != 0 {
            return Err("stale attempt generation".into());
        }
        let generation = self
            .stores
            .entry(target.clone())
            .or_default()
            .advance(
                target.engine(),
                expected_generation,
                prepared.key.clone(),
                run_id.clone(),
            )
            .map_err(|_| "attempt registration conflict")?;
        self.runs.insert(run_id.clone());
        let ticket = Ticket {
            owner: Arc::clone(&self.owner),
            generation,
            run_id,
            key: prepared.key.clone(),
            target,
        };
        Ok(RegisteredGitAttempt { ticket, prepared })
    }
    fn verify(
        &self,
        completed: &CompletedGitAttempt,
        repo: &Repository,
        expected: &CandidateSnapshot,
    ) -> Result<AttemptRecord, String> {
        let t = &completed.ticket;
        if !Arc::ptr_eq(&self.owner, &t.owner) {
            return Err("foreign history completion".into());
        }
        candidate_budget(expected)?;
        admit(&completed.bundle, MAX_BUNDLE)?;
        admit(&completed.work, MAX_WORK)?;
        completed.bundle.verify(repo, expected)?;
        let e = &completed.bundle.cargo().envelope;
        if e.run_id != t.run_id
            || e.binding != completed.work.binding
            || e.coverage.required_scopes != completed.work.required_scopes
            || expected.binding_digest() != completed.work.candidate_digest
            || digest(&bytes(&completed.work)?) != t.key
        {
            return Err("attempt frozen work mismatch".into());
        }
        if e.run_status == RunStatus::Completed
            && e.artifacts.contract.as_ref().map(|r| r.digest.as_str())
                != Some(completed.work.contract_digest.as_str())
        {
            return Err("attempt contract mismatch".into());
        }
        Ok(AttemptRecord {
            run_id: t.run_id.clone(),
            target: t.target.engine(),
            generation: t.generation,
            content_digest: t.key.clone(),
            envelope_digest: digest(&bytes(e)?),
            eligible: false,
        })
    }
    pub fn import(
        &mut self,
        completed: &CompletedGitAttempt,
        repo: &Repository,
        expected: &CandidateSnapshot,
    ) -> Result<AppendOutcome, String> {
        let record = self.verify(completed, repo, expected)?;
        self.stores
            .get_mut(&completed.ticket.target)
            .ok_or("unknown attempt target")?
            .append(record)
            .map_err(|_| "attempt append conflict".into())
    }
    pub fn publish(
        &mut self,
        completed: &CompletedGitAttempt,
        repo: &Repository,
        expected: &CandidateSnapshot,
    ) -> Result<(), String> {
        let record = self.verify(completed, repo, expected)?;
        let store = self
            .stores
            .get_mut(&completed.ticket.target)
            .ok_or("unknown attempt target")?;
        if !store
            .history(&record.target)
            .iter()
            .any(|old| **old == record)
        {
            return Err("attempt not imported".into());
        }
        store
            .publish(&record.run_id)
            .map_err(|_| "stale attempt publication".into())
    }
    /// Fresh local integrity/current check only, never approval or merge eligibility.
    /// The policy and GG candidate must be independently fixed by the controller.
    pub fn current_matches(
        &self,
        completed: &CompletedGitAttempt,
        repo: &Repository,
        expected: &CandidateSnapshot,
        policy: &ProtectedCargoPolicy,
        expected_binding: &RunBinding,
    ) -> Result<bool, String> {
        admit(expected_binding, 64 * 1024)?;
        if expected_binding != &completed.work.binding {
            return Err("independent expected binding mismatch".into());
        }
        admit(&(&policy.contract, &policy.profile_digest), MAX_WORK)?;
        if policy.profile_digest() != completed.work.profile_digest
            || digest(&bytes(&value(&policy.contract)?)?) != completed.work.contract_digest
        {
            return Err("expected policy mismatch".into());
        }
        let record = self.verify(completed, repo, expected)?;
        Ok(self
            .stores
            .get(&completed.ticket.target)
            .and_then(|s| s.current(&record.target))
            == Some(&record))
    }
    /// Bounded metadata only. History never vouches for ongoing artifact availability.
    pub fn history(&self, completed: &CompletedGitAttempt) -> Result<Vec<&AttemptRecord>, String> {
        if !Arc::ptr_eq(&self.owner, &completed.ticket.owner) {
            return Err("foreign history completion".into());
        }
        Ok(self
            .stores
            .get(&completed.ticket.target)
            .ok_or("unknown target")?
            .history(&completed.ticket.target.engine()))
    }
}
