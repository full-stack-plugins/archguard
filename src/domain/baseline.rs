//! Fixture-profile SG trace consumption. References never become Cargo policy or
//! production approval; actual source equality is checked through GitGuard.
use crate::integration::projection::{ProtectedCargoPolicy, bytes, digest};
use gitguard::{Repository, candidate::CandidateSnapshot};
use serde::{Deserialize, Serialize};
use specguard::{
    architecture::{self, ArchitectureReference, AuthenticationProfile, ExpectedHandoff},
    baseline::ApprovedBaseline,
    integration::approval::{self, ApprovalValidationPort, Profile},
    model::Identity,
    obligations::{self, ObligationSet},
};
use std::collections::{BTreeMap, BTreeSet};
const VERSION: &str = "archguard.spec-trace/v1alpha1";
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct RuleTraceLink {
    pub requirement: Identity,
    pub adr: Identity,
    pub guard_requirement_id: String,
    pub rule_ids: BTreeSet<String>,
}
/// Controller-owned exact pins and explicit links to preexisting rule IDs.
pub struct ProtectedTraceMapping {
    pub(crate) expected: ExpectedHandoff,
    links: Vec<RuleTraceLink>,
    policy_digest: String,
    digest: String,
}
impl ProtectedTraceMapping {
    pub fn freeze(
        policy: &ProtectedCargoPolicy,
        expected: ExpectedHandoff,
        mut links: Vec<RuleTraceLink>,
    ) -> Result<Self, String> {
        if links.is_empty()
            || links.len() > 256
            || expected.scope.is_empty()
            || expected.scope.len() > 256
        {
            return Err("trace mapping scope budget".into());
        }
        for (text, limit) in [
            (&expected.repository, 4096),
            (&expected.artifact_digest, 71),
            (&expected.baseline_digest, 71),
            (&expected.source_digest, 71),
            (&expected.candidate_binding.candidate_oid, 64),
            (&expected.candidate_binding.base_oid, 64),
            (&expected.candidate_binding.object_format, 6),
        ] {
            if text.trim().is_empty() || text.len() > limit {
                return Err("protected trace metadata budget".into());
            }
        }
        if expected.scope.iter().any(|id| {
            [&id.namespace, &id.id]
                .iter()
                .any(|text| text.trim().is_empty() || text.len() > 256)
        }) {
            return Err("protected trace scope identity budget".into());
        }
        let mut size = 0usize;
        for link in &links {
            if link.rule_ids.is_empty() || link.rule_ids.len() > 256 {
                return Err("trace rule mapping budget".into());
            }
            for text in [
                &link.requirement.namespace,
                &link.requirement.id,
                &link.adr.namespace,
                &link.adr.id,
                &link.guard_requirement_id,
            ]
            .into_iter()
            .chain(link.rule_ids.iter())
            {
                if text.trim().is_empty() || text.len() > 256 {
                    return Err("invalid trace mapping identity".into());
                }
                size += text.len();
                if size > 262144 {
                    return Err("trace mapping byte budget".into());
                }
            }
        }
        links.sort();
        let mut pairs = BTreeSet::new();
        let mut alias_by_requirement = BTreeMap::new();
        let mut requirement_by_alias = BTreeMap::new();
        let mut mapped_rules = BTreeSet::new();
        for link in &links {
            if !expected.scope.contains(&link.requirement)
                || !pairs.insert((&link.requirement, &link.adr))
            {
                return Err("duplicate or foreign requirement/ADR mapping".into());
            }
            if alias_by_requirement
                .insert(&link.requirement, &link.guard_requirement_id)
                .is_some_and(|id| id != &link.guard_requirement_id)
                || requirement_by_alias
                    .insert(&link.guard_requirement_id, &link.requirement)
                    .is_some_and(|id| id != &link.requirement)
            {
                return Err("ambiguous requirement alias".into());
            }
            mapped_rules.extend(link.rule_ids.iter().cloned());
        }
        if alias_by_requirement
            .keys()
            .copied()
            .collect::<BTreeSet<_>>()
            != expected.scope.iter().collect()
            || mapped_rules
                != policy
                    .contract
                    .spec
                    .rules
                    .iter()
                    .map(|r| r.id.clone())
                    .collect()
        {
            return Err("mapping must exactly cover frozen requirements and Cargo rules".into());
        }
        let encoded = bytes(&(
            VERSION,
            policy.profile_digest(),
            &expected.artifact_digest,
            &expected.repository,
            &expected.candidate_binding,
            &expected.baseline_digest,
            &expected.source_digest,
            &expected.scope,
            &expected.authentication_profile,
            &links,
        ))?;
        if encoded.len() > 1048576 {
            return Err("protected trace context budget".into());
        }
        let digest = digest(&encoded);
        Ok(Self {
            expected,
            links,
            policy_digest: policy.profile_digest().into(),
            digest,
        })
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct TraceReceipt {
    pub api_version: String,
    pub authentication_profile: AuthenticationProfile,
    pub handoff_digest: String,
    pub baseline: ApprovedBaseline,
    pub baseline_digest: String,
    pub source_digest: String,
    pub candidate_digest: String,
    pub policy_digest: String,
    pub mapping_digest: String,
    pub links: Vec<RuleTraceLink>,
    pub references: Vec<ArchitectureReference>,
    pub baseline_obligations: ObligationSet,
}
/// An owned validated local trace receipt. No public Deserialize bypass exists.
pub struct ArchitectureTrace {
    pub(crate) receipt: TraceReceipt,
}
impl ArchitectureTrace {
    pub fn read(
        raw: &[u8],
        mapping: &ProtectedTraceMapping,
        repo: &Repository,
        candidate: &CandidateSnapshot,
        port: &dyn ApprovalValidationPort,
        now: i64,
    ) -> Result<Self, String> {
        let h = architecture::decode(raw, &mapping.expected)?;
        if bytes(candidate)?.len() > 65536 {
            return Err("candidate context budget".into());
        }
        candidate
            .validate(repo)
            .map_err(|_| "invalid actual candidate")?;
        if h.repository != candidate.repo_id()
            || h.candidate_binding.candidate_oid != candidate.candidate_oid()
            || h.candidate_binding.base_oid != candidate.base_oid()
            || h.candidate_binding.object_format != repo.object_format()
            || mapping.policy_digest.strip_prefix("sha256:") != Some(candidate.policy_digest())
        {
            return Err("trace and actual candidate/policy disagree".into());
        }
        let aliases = mapping
            .links
            .iter()
            .map(|l| l.guard_requirement_id.as_str())
            .collect::<BTreeSet<_>>();
        if aliases
            != candidate
                .requirement_ids()
                .iter()
                .map(String::as_str)
                .collect()
        {
            return Err("cross-requirement trace mapping".into());
        }
        let pairs = h
            .architecture_references
            .iter()
            .map(|r| (&r.requirement, &r.adr))
            .collect::<BTreeSet<_>>();
        if pairs
            != mapping
                .links
                .iter()
                .map(|l| (&l.requirement, &l.adr))
                .collect()
        {
            return Err("missing or foreign ADR mapping".into());
        }
        let approved = approval::authenticate(&h.baseline, port, Profile::Fixture, now)
            .map_err(|_| "fixture baseline inactive or unavailable")?;
        // SG freezes declared source bytes but does not prove they are commit bytes.
        // Read backward through GG's verified private object store, never a worktree.
        let files = repo
            .read_commit_files(candidate.candidate_oid())
            .map_err(|_| "candidate source unavailable")?;
        let by_path = files
            .iter()
            .map(|f| (f.path(), f.contents()))
            .collect::<BTreeMap<_, _>>();
        for (path, contents) in &h.snapshot.contents {
            if by_path.get(path.as_bytes()).copied() != Some(contents.as_slice()) {
                return Err("SG snapshot bytes differ from actual candidate".into());
            }
        }
        let obligations = obligations::export_obligations(
            &h.baseline.graph,
            &approved,
            &h.scope,
            &h.baseline.source_digest,
        )?;
        if !obligations.complete {
            return Err("baseline acceptance obligations incomplete".into());
        }
        let receipt = TraceReceipt {
            api_version: VERSION.into(),
            authentication_profile: AuthenticationProfile::FixtureOnly,
            handoff_digest: mapping.expected.artifact_digest.clone(),
            baseline: h.baseline,
            baseline_digest: h.baseline_digest,
            source_digest: h.source_digest,
            candidate_digest: candidate.binding_digest(),
            policy_digest: mapping.policy_digest.clone(),
            mapping_digest: mapping.digest.clone(),
            links: mapping.links.clone(),
            references: h.architecture_references,
            baseline_obligations: obligations,
        };
        if bytes(&receipt)?.len() > 2 * architecture::MAX_HANDOFF_BYTES {
            return Err("trace receipt budget".into());
        }
        Ok(Self { receipt })
    }
    pub fn baseline(&self) -> &ApprovedBaseline {
        &self.receipt.baseline
    }
    pub fn references(&self) -> &[ArchitectureReference] {
        &self.receipt.references
    }
    /// Frozen baseline references only, not candidate test coverage or execution.
    pub fn baseline_obligations(&self) -> &ObligationSet {
        &self.receipt.baseline_obligations
    }
    pub fn mappings(&self) -> &[RuleTraceLink] {
        &self.receipt.links
    }
    pub fn digest(&self) -> Result<String, String> {
        Ok(digest(&bytes(&self.receipt)?))
    }
    pub(crate) fn identity_key(&self) -> Result<String, String> {
        Ok(format!("specguard.trace:{}", self.digest()?))
    }
}
