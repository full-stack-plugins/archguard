use archguard_git_spec::integration::{
    binding::GitCargoEvidence, projection::ProtectedCargoPolicy,
};
use gitguard::{
    Repository,
    candidate::{CandidateRequest, CandidateSnapshot},
    scope::TaskScope,
    subject::SubjectRequest,
};
use std::{path::Path, process::Command, sync::atomic::AtomicBool};
mod common;
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn fixture(format: &str) -> (common::Temp, String, String) {
    let dir = common::Temp::new();
    dir.copy_fixture("forbidden");
    git(
        dir.0.as_path(),
        &["init", "-q", &format!("--object-format={format}")],
    );
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "base"]);
    let base = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        dir.0.as_path().join("candidate-note.txt"),
        "exact candidate",
    )
    .unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "candidate"]);
    let candidate = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    (dir, base, candidate)
}
fn policy() -> ProtectedCargoPolicy {
    ProtectedCargoPolicy::freeze(common::evidence::contract(), vec![]).unwrap()
}
fn snapshot(
    repo: &Repository,
    base: &str,
    candidate: &str,
    p: &ProtectedCargoPolicy,
) -> CandidateSnapshot {
    let scope = TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"candidate-note.txt".to_vec()],
        p.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let subject = repo
        .resolve_subject(SubjectRequest::Commit(candidate.into()))
        .unwrap();
    repo.prepare_candidate(
        &subject,
        &scope,
        &CandidateRequest {
            worktree_id: "wt".into(),
            base_oid: base.into(),
            merge_group_id: Some("queue".into()),
            members: vec![base.into(), candidate.into()],
        },
    )
    .unwrap()
}
#[test]
fn actual_git_candidate_is_analyzed_instead_of_mutable_head_for_both_formats() {
    for format in ["sha1", "sha256"] {
        let (dir, base, candidate) = fixture(format);
        git(dir.0.as_path(), &["checkout", "-q", &base]);
        let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
        let p = policy();
        let s = snapshot(&repo, &base, &candidate, &p);
        let prepared = GitCargoEvidence::prepare(&repo, &s, p).unwrap();
        let frozen_binding = prepared.binding().clone();
        assert_ne!(
            frozen_binding.source_snapshot_digest,
            s.source_snapshot_digest()
        );
        std::fs::write(dir.0.as_path().join("Cargo.toml"), "mutable poison").unwrap();
        let result = prepared.run(&AtomicBool::new(false)).unwrap();
        assert_eq!(result.cargo().envelope.binding, frozen_binding);
        assert_eq!(result.cargo().envelope.binding.candidate_oid, candidate);
        assert_eq!(result.object_format(), format);
        assert_eq!(
            result.cargo().envelope.run_status,
            guardengine::integration::RunStatus::Completed
        );
        assert_eq!(
            result.cargo().envelope.decision,
            Some(guardengine::Decision::Block)
        );
    }
}
#[test]
fn nonexistent_candidate_wrong_repository_and_policy_never_bind() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let mut value = serde_json::to_value(&s).unwrap();
    value["candidate_oid"] = serde_json::json!("1".repeat(40));
    let bad = serde_json::from_value(value).unwrap();
    assert!(GitCargoEvidence::prepare(&repo, &bad, policy()).is_err());
    let other = Repository::discover(dir.0.as_path(), "other").unwrap();
    assert!(GitCargoEvidence::prepare(&other, &s, policy()).is_err());
    let wrong =
        ProtectedCargoPolicy::freeze(common::evidence::contract(), vec!["unrequired".into()])
            .unwrap();
    assert!(GitCargoEvidence::prepare(&repo, &s, wrong).is_err());
}

#[test]
fn exact_wrapper_rejects_old_queue_base_members_requirements_and_tampered_source() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    let serialized = serde_json::to_vec(&result).unwrap();
    archguard_git_spec::integration::binding::GitEvidenceBundle::load(&serialized, &repo, &s)
        .unwrap();
    for (key, value) in [
        ("base_oid", serde_json::json!(candidate)),
        ("merge_group_id", serde_json::json!("new-queue")),
        ("members", serde_json::json!([candidate])),
        ("requirement_ids", serde_json::json!(["OTHER"])),
    ] {
        let mut changed = serde_json::to_value(&s).unwrap();
        changed[key] = value;
        let changed: CandidateSnapshot = serde_json::from_value(changed).unwrap();
        changed.validate(&repo).unwrap();
        assert!(result.verify(&repo, &changed).is_err());
    }
    let mut damaged = serde_json::to_value(&result).unwrap();
    damaged["files_digest"] = serde_json::json!(format!("sha256:{}", "0".repeat(64)));
    assert!(
        archguard_git_spec::integration::binding::GitEvidenceBundle::load(
            &serde_json::to_vec(&damaged).unwrap(),
            &repo,
            &s
        )
        .is_err()
    );
    damaged = serde_json::to_value(&result).unwrap();
    damaged["object_format"] = serde_json::json!("sha256");
    assert!(
        archguard_git_spec::integration::binding::GitEvidenceBundle::load(
            &serde_json::to_vec(&damaged).unwrap(),
            &repo,
            &s
        )
        .is_err()
    );
}
#[test]
fn cancellation_retains_real_binding_and_null_decision() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(true))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    assert_eq!(
        result.cargo().envelope.run_status,
        guardengine::integration::RunStatus::Cancelled
    );
    assert!(result.cargo().envelope.decision.is_none());
    assert!(result.cargo().domain.is_none());
}
#[test]
fn ignored_candidate_files_and_nonregular_objects_do_not_disappear_from_snapshot() {
    for kind in ["target", "symlink"] {
        let (dir, base, _) = fixture("sha1");
        if kind == "target" {
            std::fs::create_dir(dir.0.join("target")).unwrap();
            std::fs::write(dir.0.join("target/input"), "tracked source").unwrap();
        } else {
            std::os::unix::fs::symlink("Cargo.toml", dir.0.join("manifest-link")).unwrap();
        }
        git(dir.0.as_path(), &["add", "-f", "."]);
        git(dir.0.as_path(), &["commit", "-qm", "unsupported candidate"]);
        let candidate = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
        let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
        let p = policy();
        let s = snapshot(&repo, &base, &candidate, &p);
        assert!(GitCargoEvidence::prepare(&repo, &s, p).is_err());
    }
}

#[test]
fn real_candidate_metadata_failure_is_bound_error_and_missing_scope_is_completed_partial() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = ProtectedCargoPolicy::freeze(
        common::evidence::contract(),
        vec!["missing-protected-member".into()],
    )
    .unwrap();
    let s = snapshot(&repo, &base, &candidate, &p);
    let partial = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    partial.verify(&repo, &s).unwrap();
    assert_eq!(
        partial.cargo().envelope.run_status,
        guardengine::integration::RunStatus::Completed
    );
    assert_eq!(
        partial.cargo().envelope.coverage.status,
        guardengine::integration::CoverageStatus::Partial
    );
    assert_eq!(
        partial.cargo().envelope.decision,
        Some(guardengine::Decision::Block)
    );
    std::fs::write(
        dir.0.join("Cargo.toml"),
        "[workspace]\nmembers=[\"missing-member\"]\n",
    )
    .unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "invalid metadata"]);
    let candidate = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let failed = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    failed.verify(&repo, &s).unwrap();
    assert_eq!(
        failed.cargo().envelope.run_status,
        guardengine::integration::RunStatus::Error
    );
    assert!(failed.cargo().envelope.decision.is_none());
    assert!(failed.cargo().contract.is_none());
}

#[test]
fn actual_two_parent_queue_merge_differs_from_member_head() {
    let (dir, base, _) = fixture("sha1");
    git(dir.0.as_path(), &["checkout", "-q", &base]);
    std::fs::write(dir.0.join("member-a.txt"), "member a").unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "member a"]);
    let a = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    git(dir.0.as_path(), &["checkout", "-q", &base]);
    std::fs::write(dir.0.join("member-b.txt"), "member b").unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "member b"]);
    let b = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    git(
        dir.0.as_path(),
        &["merge", "--no-ff", "-m", "synthetic queue candidate", &a],
    );
    let queue = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    git(dir.0.as_path(), &["checkout", "-q", &a]);
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let scope = TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"member-a.txt".to_vec(), b"member-b.txt".to_vec()],
        p.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let s = repo
        .prepare_candidate(
            &repo
                .resolve_subject(SubjectRequest::Commit(queue.clone()))
                .unwrap(),
            &scope,
            &CandidateRequest {
                worktree_id: "wt".into(),
                base_oid: base.clone(),
                merge_group_id: Some("queue".into()),
                members: vec![a.clone(), b],
            },
        )
        .unwrap();
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    let member = snapshot(&repo, &base, &a, &policy());
    assert!(result.verify(&repo, &member).is_err());
    assert_eq!(result.cargo().envelope.binding.candidate_oid, queue);
}

#[test]
fn review_domain_file_inventory_must_match_actual_candidate_files() {
    use archguard_git_spec::integration::binding::GitEvidenceBundle;
    use sha2::{Digest, Sha256};
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let bundle = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let original = serde_json::to_value(&bundle).unwrap();
    GitEvidenceBundle::load(&serde_json::to_vec(&original).unwrap(), &repo, &s).unwrap();
    let mut contradictions = vec![];
    for alteration in [
        "missing-file",
        "foreign-file",
        "foreign-git-binding",
        "foreign-git-files",
        "foreign-git-namespace",
    ] {
        let mut v = original.clone();
        let keys = v["cargo"]["domain"]["inventory_keys"]
            .as_array_mut()
            .unwrap();
        match alteration {
            "missing-file" => keys.retain(|k| k.as_str() != Some("file:Cargo.toml")),
            "foreign-file" => keys.push(serde_json::json!("file:never-in-candidate")),
            "foreign-git-binding" => keys.push(serde_json::json!(format!(
                "identity:gitguard.binding:{}",
                "f".repeat(64)
            ))),
            "foreign-git-files" => keys.push(serde_json::json!(format!(
                "identity:gitguard.files:sha256:{}",
                "f".repeat(64)
            ))),
            _ => keys.push(serde_json::json!("identity:gitguard.unexpected")),
        }
        keys.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&v["cargo"]["domain"]).unwrap())
        );
        v["cargo"]["envelope"]["artifacts"]["domain"][0]["digest"] =
            serde_json::json!(format!("sha256:{digest}"));
        v["cargo"]["envelope"]["artifacts"]["domain"][0]["uri"] =
            serde_json::json!(format!("artifact://archguard/domain/{digest}"));
        let accepted = GitEvidenceBundle::load(&serde_json::to_vec(&v).unwrap(), &repo, &s).is_ok();
        eprintln!("{alteration}: accepted={accepted}");
        if accepted {
            contradictions.push(alteration);
        }
    }
    assert!(
        contradictions.is_empty(),
        "accepted contradictory inventories: {contradictions:?}"
    );
}

#[test]
fn ordinary_source_directory_cannot_become_a_verified_candidate_repository() {
    let dir = common::Temp::new();
    dir.copy_fixture("allowed");
    assert!(!dir.0.join(".git").exists());
    assert!(Repository::discover(dir.0.as_path(), "claimed-repository").is_err());
}

#[test]
fn exact_optional_candidate_flows_through_core_trust_without_importing_git_into_core() {
    use archguard::integration::trust::{ConsumptionPolicy, consume};
    use guardengine::integration::eligibility::*;
    use std::collections::{BTreeMap, BTreeSet};
    struct FixtureAuthority;
    impl AuthorityProvider for FixtureAuthority {
        fn verify_producer(
            &self,
            e: &guardengine::integration::GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            Ok(ProducerRecord {
                principal: "local-fixture".into(),
                producer: e.producer.clone(),
                envelope_digest: d.into(),
                validity: Validity {
                    issued_at: 0,
                    expires_at: 2_100_000_000,
                    revoked: false,
                },
            })
        }
        fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
            Err(AuthorityError::Untrusted)
        }
    }
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let mut c = common::evidence::contract();
    let guardengine::GuardAssertion::ForbidRelation { object, .. } = &mut c.spec.rules[0].assertion;
    *object = "agent-job".into();
    let p = ProtectedCargoPolicy::freeze(c, vec![]).unwrap();
    let s = snapshot(&repo, &base, &candidate, &p);
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    let bundle = result.cargo();
    let mut policy = ConsumptionPolicy {
        eligibility: EligibilityPolicy {
            binding: bundle.envelope.binding.clone(),
            producer: bundle.envelope.producer.clone(),
            required_scopes: bundle.envelope.coverage.required_scopes.clone(),
            contract_digest: bundle
                .envelope
                .artifacts
                .contract
                .as_ref()
                .unwrap()
                .digest
                .clone(),
            action: "merge".into(),
            producer_principals: BTreeSet::from(["local-fixture".into()]),
            approval_principals: BTreeMap::new(),
        },
        required_capabilities: BTreeSet::from(["cargo.declarations".into()]),
        required_approvals: BTreeMap::new(),
    };
    assert!(consume(bundle, &policy, &FixtureAuthority, 2_000_000_000, None).eligible);
    policy.eligibility.binding.candidate_oid = base;
    assert!(!consume(bundle, &policy, &FixtureAuthority, 2_000_000_000, None).eligible);
    policy.eligibility.binding.candidate_oid = candidate;
    policy.eligibility.binding.requirement_ids = vec!["other-requirement".into()];
    assert!(!consume(bundle, &policy, &FixtureAuthority, 2_000_000_000, None).eligible);
}

#[test]
fn gitguard_specialist_consumer_requires_architecture_profile_artifacts_and_coverage() {
    use archguard::integration::{
        projection::{EvidenceBundle, bytes},
        trust::ConsumptionPolicy,
    };
    use archguard_git_spec::integration::specialists::{ArchitectureInput, consume};
    use gitguard::evidence::specialists::{
        ImmutableArtifactBytes, PreparedSpecialistConsumption, RequiredSpecialist,
    };
    use guardengine::integration::{EvidenceProfile, GuardRunEnvelope, eligibility::*};
    use std::collections::{BTreeMap, BTreeSet};
    struct Authority;
    impl AuthorityProvider for Authority {
        fn verify_producer(
            &self,
            e: &GuardRunEnvelope,
            d: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            Ok(ProducerRecord {
                principal: "fixture-authority".into(),
                producer: e.producer.clone(),
                envelope_digest: d.into(),
                validity: Validity {
                    issued_at: 0,
                    expires_at: 2_100_000_000,
                    revoked: false,
                },
            })
        }
        fn verify_approval(&self, _: &str) -> Result<ApprovalRecord, AuthorityError> {
            Err(AuthorityError::Untrusted)
        }
    }
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let mut contract = common::evidence::contract();
    let guardengine::GuardAssertion::ForbidRelation { object, .. } =
        &mut contract.spec.rules[0].assertion;
    *object = "agent-job".into();
    let protected = ProtectedCargoPolicy::freeze(contract, vec![]).unwrap();
    let candidate = snapshot(&repo, &base, &candidate, &protected);
    let actual = GitCargoEvidence::prepare(&repo, &candidate, protected)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    actual.verify(&repo, &candidate).unwrap();
    let b = actual.cargo();
    let policy = ConsumptionPolicy {
        eligibility: EligibilityPolicy {
            binding: b.envelope.binding.clone(),
            producer: b.envelope.producer.clone(),
            required_scopes: b.envelope.coverage.required_scopes.clone(),
            contract_digest: b
                .envelope
                .artifacts
                .contract
                .as_ref()
                .unwrap()
                .digest
                .clone(),
            action: "merge".into(),
            producer_principals: BTreeSet::from(["fixture-authority".into()]),
            approval_principals: BTreeMap::new(),
        },
        required_capabilities: BTreeSet::from(["cargo.declarations".into()]),
        required_approvals: BTreeMap::new(),
    };
    let required = PreparedSpecialistConsumption::freeze(
        &repo,
        &candidate,
        vec![RequiredSpecialist {
            id: "archguard".into(),
            policy: policy.eligibility.clone(),
            profile: EvidenceProfile::EngineBacked,
            baseline: None,
        }],
    )
    .unwrap();
    let artifact_bytes = ImmutableArtifactBytes::new(vec![
        (
            b.envelope.artifacts.contract.as_ref().unwrap().clone(),
            bytes(b.contract.as_ref().unwrap()).unwrap(),
        ),
        (
            b.envelope.artifacts.facts.as_ref().unwrap().clone(),
            bytes(b.facts.as_ref().unwrap()).unwrap(),
        ),
        (
            b.envelope.artifacts.report.as_ref().unwrap().clone(),
            bytes(b.report.as_ref().unwrap()).unwrap(),
        ),
    ])
    .unwrap();
    let original = bytes(b).unwrap();
    let original_policy = policy.content_digest();
    let mut capture = vec![];
    for mutation in 0..6 {
        let mut bundle = EvidenceBundle::load(&original).unwrap();
        let mut requested = policy.clone();
        match mutation {
            0 => {}
            1 => {
                requested
                    .required_capabilities
                    .insert("domain.method.purity".into());
            }
            2 => bundle.domain = None,
            3 => {
                bundle.envelope.coverage.status = guardengine::integration::CoverageStatus::Partial;
                bundle.envelope.coverage.missing_scopes =
                    bundle.envelope.coverage.required_scopes.clone();
                bundle.envelope.coverage.observed_scopes.clear();
            }
            5 => requested.eligibility.action = "push".into(),
            _ => {}
        }
        let mut envelopes = BTreeMap::from([("archguard".into(), bundle.envelope.clone())]);
        if mutation == 4 {
            envelopes.get_mut("archguard").unwrap().run_id = "different-envelope".into();
        }
        let generic = required
            .consume(
                &repo,
                &candidate,
                &envelopes,
                &artifact_bytes,
                &Authority,
                2_000_000_000,
                None,
            )
            .unwrap();
        if mutation < 3 {
            assert!(
                generic.eligible(),
                "generic GE evidence does not qualify AG capabilities/domain"
            );
        }
        let result = consume(
            &required,
            &repo,
            &candidate,
            &envelopes,
            &artifact_bytes,
            ArchitectureInput {
                id: "archguard",
                bundle: &bundle,
                policy: &requested,
            },
            &Authority,
            2_000_000_000,
            None,
        );
        if let Ok(ref accepted) = result {
            assert_eq!(
                accepted.architecture_result().audit.content_digest,
                requested.content_digest()
            );
            assert_eq!(
                accepted.specialist_results()["archguard"]
                    .audit
                    .content_digest,
                policy.eligibility.content_digest()
            );
        }
        let mut grant_calls = 0;
        let mut request_fixture_grant = || {
            grant_calls += 1;
        };
        let satisfied = match result {
            Ok(ref result) if result.eligible() => {
                request_fixture_grant();
                true
            }
            _ => false,
        };
        assert_eq!(satisfied, mutation == 0, "mutation {mutation}");
        assert_eq!(grant_calls, usize::from(mutation == 0));
        assert_eq!(
            envelopes.keys().cloned().collect::<Vec<_>>(),
            vec!["archguard"]
        );
        capture.push(serde_json::json!({"mutation":mutation,"case":(["supported","unsupported-required-capability","missing-domain-artifact","missing-coverage","selected-envelope-substitution","changed-action-policy"][mutation]),"generic_eligible":generic.eligible(),"combined_consumer_satisfied":satisfied,"downstream_fixture_grant_calls":grant_calls,"required_ids":["archguard"]}));
    }
    assert_eq!(bytes(b).unwrap(), original);
    assert_eq!(policy.content_digest(), original_policy);
    let envelopes = BTreeMap::from([("archguard".into(), b.envelope.clone())]);
    assert!(
        consume(
            &required,
            &repo,
            &candidate,
            &envelopes,
            &artifact_bytes,
            ArchitectureInput {
                id: "archguard",
                bundle: b,
                policy: &policy
            },
            &Authority,
            2_000_000_000,
            None
        )
        .unwrap()
        .eligible()
    );
    if let Ok(path) = std::env::var("ARCHGUARD_GG_CONSUMER_CAPTURE") {
        std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"profile":"cargo-declarations-v1","authority":"local-fixture-only","actual_bundle":b,"observations":capture,"required_set_unchanged":true,"recovery_eligible":true})).unwrap()).unwrap();
    }
}
