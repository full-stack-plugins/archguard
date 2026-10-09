//! Explicit fixture-only approval; never a production provider.
use super::{Temp, evidence};
use archguard::{
    domain::baseline::{ProtectedTraceMapping, RuleTraceLink},
    integration::projection::ProtectedCargoPolicy,
};
use gitguard::{
    Repository,
    candidate::{CandidateRequest, CandidateSnapshot},
    scope::TaskScope,
    subject::SubjectRequest,
};
use specguard::{architecture::*, baseline::*, integration::approval::*, model::*, source::*};
use std::collections::BTreeSet;
pub struct FixturePort {
    pub revoked: bool,
    pub unavailable: bool,
}
impl ApprovalValidationPort for FixturePort {
    fn profile(&self) -> Profile {
        Profile::Fixture
    }
    fn validate(&self, b: &ApprovedBaseline) -> Result<Authentication, ApprovalError> {
        if self.unavailable {
            return Err(ApprovalError::Unavailable("fixture unavailable".into()));
        }
        Ok(Authentication {
            issuer: "fixture-controller".into(),
            purpose: "specification-baseline".into(),
            repository: b.repository.clone(),
            scope: b.scope.clone(),
            baseline_digest: digest(b),
            policy_digest: b.policy_digest.clone(),
            issued_at: 10,
            expires_at: 100,
            revoked: self.revoked,
        })
    }
}
pub fn port() -> FixturePort {
    FixturePort {
        revoked: false,
        unavailable: false,
    }
}
pub fn key(id: &str) -> Identity {
    Identity {
        namespace: "demo".into(),
        id: id.into(),
    }
}
pub fn policy() -> ProtectedCargoPolicy {
    ProtectedCargoPolicy::freeze(evidence::contract(), vec![]).unwrap()
}
pub fn links() -> Vec<RuleTraceLink> {
    vec![RuleTraceLink {
        requirement: key("R1"),
        adr: key("ADR1"),
        guard_requirement_id: "R".into(),
        rule_ids: BTreeSet::from(["AGJ-ARCH-001".into()]),
    }]
}
pub struct Fixture {
    pub root: Temp,
    pub repo: Repository,
    pub candidate: CandidateSnapshot,
    pub bytes: Vec<u8>,
    pub expected: ExpectedHandoff,
    pub approved: ValidatedBaseline,
    pub source_policy: SourcePolicy,
}
impl Fixture {
    pub fn new() -> Self {
        let root = Temp::new();
        root.copy_fixture("forbidden");
        std::fs::create_dir(root.0.join("specs")).unwrap();
        std::fs::write(root.0.join("specs/a.md"),"---\nformat: markdown-explicit/v1\nnamespace: demo\n---\n## Requirement: R1\nA user can sign in.\n### Acceptance: A1\nA correct password opens a session.\n\n- traces_to_adr: demo:ADR1\n").unwrap();
        std::fs::write(root.0.join("adr.md"),"---\nformat: markdown-adr/v1\nnamespace: demo\n---\n## ADR: ADR1\nKeep the protected package boundary.\n").unwrap();
        let context = evidence::context(&root.0);
        let baseline: ApprovedBaseline = serde_json::from_slice(include_bytes!(
            "../../fixtures/integration/specguard/baseline.json"
        ))
        .unwrap();
        let approved = authenticate(&baseline, &port(), Profile::Fixture, 50).unwrap();
        let source_policy = SourcePolicy {
            api_version: Version::V1,
            roots: vec![
                SourceRoot {
                    path: "specs".into(),
                    format: "markdown-explicit/v1".into(),
                    namespace: "demo".into(),
                    authority: "primary".into(),
                },
                SourceRoot {
                    path: "adr.md".into(),
                    format: "markdown-adr/v1".into(),
                    namespace: "demo".into(),
                    authority: "primary".into(),
                },
            ],
            limits: Limits::default(),
        };
        let source = freeze(
            &root.0,
            &discover(&root.0, &source_policy).unwrap(),
            CandidateBinding {
                candidate_oid: context.candidate_oid.clone(),
                base_oid: context.base_oid.clone(),
                object_format: "sha1".into(),
            },
        )
        .unwrap();
        let handoff = export(&source, &approved, &baseline.scope).unwrap();
        let bytes = encode(&handoff).unwrap();
        let expected = ExpectedHandoff {
            artifact_digest: artifact_digest(&bytes),
            repository: baseline.repository,
            candidate_binding: source.binding,
            baseline_digest: handoff.baseline_digest,
            source_digest: source.digest,
            scope: baseline.scope,
            authentication_profile: AuthenticationProfile::FixtureOnly,
        };
        let repo = Repository::discover(&root.0, "fixture-repo").unwrap();
        let p = policy();
        let scope = TaskScope::advisory(
            "task",
            vec!["R".into()],
            vec![b"candidate-note.txt".to_vec()],
            p.profile_digest().trim_start_matches("sha256:"),
            None,
        )
        .unwrap();
        let candidate = repo
            .prepare_candidate(
                &repo
                    .resolve_subject(SubjectRequest::Commit(context.candidate_oid))
                    .unwrap(),
                &scope,
                &CandidateRequest {
                    worktree_id: "wt".into(),
                    base_oid: context.base_oid,
                    merge_group_id: None,
                    members: vec![],
                },
            )
            .unwrap();
        Self {
            root,
            repo,
            candidate,
            bytes,
            expected,
            approved,
            source_policy,
        }
    }
    pub fn mapping(&self) -> ProtectedTraceMapping {
        ProtectedTraceMapping::freeze(&policy(), self.expected.clone(), links()).unwrap()
    }
}
