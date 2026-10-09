mod common;
use archguard::{
    domain::baseline::{ArchitectureTrace, ProtectedTraceMapping},
    integration::trace::{TracedCargoEvidence, TracedEvidenceBundle},
};
use common::spec_trace::*;
use specguard::{architecture, source};
use std::sync::atomic::AtomicBool;
#[test]
fn actual_sg_handoff_drives_frozen_trace_without_changing_cargo_policy() {
    let f = Fixture::new();
    let mapping = f.mapping();
    let trace =
        ArchitectureTrace::read(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50).unwrap();
    assert_eq!(trace.references()[0].requirement, key("R1"));
    assert_eq!(trace.references()[0].adr, key("ADR1"));
    assert!(trace.baseline_obligations().complete);
    assert_eq!(
        trace.baseline_obligations().obligations[0].id,
        "obligation:sha256:e297dc9327c15def0f2a0358aae2910d862cab63c0b3527566f7cc2cb080026a"
    );
    assert_eq!(
        trace.baseline_obligations().obligations[0].requirement,
        key("R1")
    );
    let evidence = TracedCargoEvidence::prepare(&f.repo, &f.candidate, policy(), trace).unwrap();
    let pinned = evidence.binding().clone();
    let result = evidence.run(&AtomicBool::new(false)).unwrap();
    assert_eq!(result.git().cargo().envelope.binding, pinned);
    assert_eq!(
        result.git().cargo().contract.as_ref().unwrap(),
        &serde_json::to_value(common::evidence::contract()).unwrap()
    );
    assert_eq!(
        result.git().cargo().envelope.decision,
        Some(guardengine::Decision::Block)
    );
    assert!(result.git().cargo().envelope.approval_refs.is_empty());
    let bytes = serde_json::to_vec(&result).unwrap();
    TracedEvidenceBundle::load(
        &bytes,
        &f.bytes,
        &mapping,
        &f.repo,
        &f.candidate,
        &port(),
        50,
    )
    .unwrap();
    assert!(
        TracedEvidenceBundle::load(
            &bytes,
            &f.bytes,
            &mapping,
            &f.repo,
            &f.candidate,
            &port(),
            100
        )
        .is_err()
    );
}
#[test]
fn external_context_version_digest_and_reference_tampering_are_rejected() {
    let f = Fixture::new();
    for field in [
        "repository",
        "source_digest",
        "baseline_digest",
        "artifact_digest",
    ] {
        let mut e = f.expected.clone();
        match field {
            "repository" => e.repository = "other".into(),
            "source_digest" => e.source_digest = format!("sha256:{}", "f".repeat(64)),
            "baseline_digest" => e.baseline_digest = format!("sha256:{}", "f".repeat(64)),
            _ => e.artifact_digest = format!("sha256:{}", "f".repeat(64)),
        };
        let mapping = ProtectedTraceMapping::freeze(&policy(), e, links()).unwrap();
        assert!(
            ArchitectureTrace::read(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50)
                .is_err()
        );
    }
    for field in ["apiVersion", "unexpected", "architectureReferences"] {
        let mut value: serde_json::Value = serde_json::from_slice(&f.bytes).unwrap();
        value[field] = serde_json::json!("unsupported");
        let bytes = serde_json::to_vec(&value).unwrap();
        let mut e = f.expected.clone();
        e.artifact_digest = architecture::artifact_digest(&bytes);
        let mapping = ProtectedTraceMapping::freeze(&policy(), e, links()).unwrap();
        assert!(
            ArchitectureTrace::read(&bytes, &mapping, &f.repo, &f.candidate, &port(), 50).is_err()
        );
    }
}
#[test]
fn real_dirty_sg_snapshot_cannot_claim_candidate_source_equivalence() {
    let f = Fixture::new();
    std::fs::write(f.root.0.join("adr.md"),"---\nformat: markdown-adr/v1\nnamespace: demo\n---\n## ADR: ADR1\nDirty source changed after commit.\n").unwrap();
    let snapshot = source::freeze(
        &f.root.0,
        &source::discover(&f.root.0, &f.source_policy).unwrap(),
        f.expected.candidate_binding.clone(),
    )
    .unwrap();
    let handoff = architecture::export(&snapshot, &f.approved, &f.expected.scope).unwrap();
    let bytes = architecture::encode(&handoff).unwrap();
    let mut e = f.expected.clone();
    e.artifact_digest = architecture::artifact_digest(&bytes);
    e.source_digest = snapshot.digest;
    architecture::decode(&bytes, &e).unwrap();
    // Fresh GG snapshot honestly records dirty=false-clean state; failure must be
    // the mismatching actual committed source, not the old clean flag alone.
    let mut value = serde_json::to_value(&f.candidate).unwrap();
    value["clean"] = serde_json::json!(false);
    let dirty = serde_json::from_value(value).unwrap();
    dirty_validate(&dirty, &f.repo);
    let mapping = ProtectedTraceMapping::freeze(&policy(), e, links()).unwrap();
    assert!(ArchitectureTrace::read(&bytes, &mapping, &f.repo, &dirty, &port(), 50).is_err());
}
fn dirty_validate(candidate: &gitguard::candidate::CandidateSnapshot, repo: &gitguard::Repository) {
    candidate.validate(repo).unwrap();
}
#[test]
fn mapping_scope_and_baseline_lifecycle_fail_closed_without_runtime_sg_for_basic_cargo() {
    let f = Fixture::new();
    assert!(ProtectedTraceMapping::freeze(&policy(), f.expected.clone(), vec![]).is_err());
    let mut bad = links();
    bad[0].rule_ids = std::collections::BTreeSet::from(["UNDECLARED".into()]);
    assert!(ProtectedTraceMapping::freeze(&policy(), f.expected.clone(), bad).is_err());
    let mut bad = links();
    bad[0].guard_requirement_id = "WRONG".into();
    let mapping = ProtectedTraceMapping::freeze(&policy(), f.expected.clone(), bad).unwrap();
    assert!(
        ArchitectureTrace::read(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50).is_err()
    );
    for p in [
        FixturePort {
            revoked: true,
            unavailable: false,
        },
        FixturePort {
            revoked: false,
            unavailable: true,
        },
    ] {
        assert!(
            ArchitectureTrace::read(&f.bytes, &f.mapping(), &f.repo, &f.candidate, &p, 50).is_err()
        );
    }
    assert!(
        ArchitectureTrace::read(&f.bytes, &f.mapping(), &f.repo, &f.candidate, &port(), 9).is_err()
    );
    assert!(
        ArchitectureTrace::read(&f.bytes, &f.mapping(), &f.repo, &f.candidate, &port(), 100)
            .is_err()
    );
    archguard::integration::binding::GitCargoEvidence::prepare(&f.repo, &f.candidate, policy())
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
}

#[test]
fn protected_trace_metadata_is_bounded_before_serialization() {
    let f = Fixture::new();
    let mut expected = f.expected.clone();
    expected.repository = "r".repeat(4097);
    assert!(ProtectedTraceMapping::freeze(&policy(), expected, links()).is_err());
}

#[test]
fn strict_traced_bundle_rejects_receipt_and_rehashed_inventory_tampering() {
    use sha2::{Digest, Sha256};
    let f = Fixture::new();
    let mapping = f.mapping();
    let trace =
        ArchitectureTrace::read(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50).unwrap();
    let bundle = TracedCargoEvidence::prepare(&f.repo, &f.candidate, policy(), trace)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let original = serde_json::to_value(&bundle).unwrap();
    for alteration in [
        "version",
        "unknown",
        "baseline",
        "mapping",
        "reference",
        "missing-identity",
        "foreign-identity",
    ] {
        let mut value = original.clone();
        match alteration {
            "version" => value["api_version"] = "future".into(),
            "unknown" => value["unexpected"] = true.into(),
            "baseline" => value["trace"]["baseline"]["sourceRevision"] = "f".repeat(40).into(),
            "mapping" => value["trace"]["links"][0]["guard_requirement_id"] = "foreign".into(),
            "reference" => value["trace"]["references"][0]["adr"]["id"] = "foreign".into(),
            other => {
                let keys = value["git"]["cargo"]["domain"]["inventory_keys"]
                    .as_array_mut()
                    .unwrap();
                if other == "missing-identity" {
                    keys.retain(|k| !k.as_str().unwrap().starts_with("identity:specguard"));
                } else {
                    keys.push("identity:specguard.foreign".into());
                    keys.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
                }
                let hash = format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&value["git"]["cargo"]["domain"]).unwrap())
                );
                value["git"]["cargo"]["envelope"]["artifacts"]["domain"][0]["digest"] =
                    format!("sha256:{hash}").into();
                value["git"]["cargo"]["envelope"]["artifacts"]["domain"][0]["uri"] =
                    format!("artifact://archguard/domain/{hash}").into();
            }
        }
        assert!(
            TracedEvidenceBundle::load(
                &serde_json::to_vec(&value).unwrap(),
                &f.bytes,
                &mapping,
                &f.repo,
                &f.candidate,
                &port(),
                50
            )
            .is_err(),
            "{alteration}"
        );
    }
    let raw = serde_json::to_string(&bundle).unwrap();
    let duplicate = raw.replacen(
        "\"api_version\":",
        "\"api_version\":\"archguard.spec-trace-evidence/v1alpha1\",\"api_version\":",
        1,
    );
    let nested_duplicate = raw.replacen(
        "\"kind\":\"GuardFacts\"",
        "\"kind\":\"GuardFacts\",\"kind\":\"GuardFacts\"",
        1,
    );
    assert_ne!(raw, nested_duplicate);
    assert!(
        TracedEvidenceBundle::load(
            nested_duplicate.as_bytes(),
            &f.bytes,
            &mapping,
            &f.repo,
            &f.candidate,
            &port(),
            50
        )
        .is_err()
    );
    assert_ne!(raw, duplicate);
    assert!(
        TracedEvidenceBundle::load(
            duplicate.as_bytes(),
            &f.bytes,
            &mapping,
            &f.repo,
            &f.candidate,
            &port(),
            50
        )
        .is_err()
    );
}

#[test]
fn cancellation_keeps_frozen_trace_and_scoped_binding_without_approval() {
    let f = Fixture::new();
    let mapping = f.mapping();
    let trace =
        ArchitectureTrace::read(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50).unwrap();
    let evidence = TracedCargoEvidence::prepare(&f.repo, &f.candidate, policy(), trace).unwrap();
    let binding = evidence.binding().clone();
    assert_ne!(binding.source_snapshot_digest, f.expected.source_digest);
    assert_ne!(
        binding.source_snapshot_digest,
        f.candidate.source_snapshot_digest()
    );
    assert!(binding.baseline_digest.is_none());
    let bundle = evidence.run(&AtomicBool::new(true)).unwrap();
    assert_eq!(bundle.git().cargo().envelope.binding, binding);
    assert_eq!(
        bundle.git().cargo().envelope.run_status,
        guardengine::integration::RunStatus::Cancelled
    );
    assert!(bundle.git().cargo().envelope.decision.is_none());
    assert!(bundle.git().cargo().envelope.approval_refs.is_empty());
    assert_eq!(
        bundle.baseline_obligations().baseline_digest,
        f.expected.baseline_digest
    );
    bundle
        .verify(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50)
        .unwrap();
}

#[test]
fn architecture_revision_cannot_self_approve_or_reuse_deleted_obligation_approval() {
    use archguard::domain::baseline::{ArchitectureBaseline, ArchitectureBaselineState};
    use guardengine::integration::{RunBinding, eligibility::*};
    use std::collections::{BTreeMap, BTreeSet};
    struct Issuer {
        record: ApprovalRecord,
    }
    impl AuthorityProvider for Issuer {
        fn verify_producer(
            &self,
            _: &guardengine::integration::GuardRunEnvelope,
            _: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            Err(AuthorityError::Unavailable)
        }
        fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
            if reference == "external:approval" {
                Ok(self.record.clone())
            } else {
                Err(AuthorityError::Untrusted)
            }
        }
    }
    let binding = RunBinding {
        repo_id: "repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: vec!["R".into()],
        candidate_oid: "a".repeat(40),
        base_oid: "b".repeat(40),
        merge_group_id: None,
        source_snapshot_digest: format!("sha256:{}", "c".repeat(64)),
        baseline_digest: None,
    };
    let draft = ArchitectureBaseline::draft(
        binding.clone(),
        "git:refs/heads/main",
        BTreeMap::from([("ADR1".into(), b"status: accepted\napproved: true".to_vec())]),
        &common::evidence::contract(),
        BTreeSet::from(["rule-A".into(), "rule-B".into()]),
    )
    .unwrap();
    assert_eq!(draft.state(), ArchitectureBaselineState::Draft);
    let frozen = serde_json::to_vec(&draft).unwrap();
    let review = draft.submit().unwrap();
    let mut approval_binding = binding;
    approval_binding.baseline_digest = Some(draft.content_digest().into());
    let protected = EligibilityPolicy {
        binding: approval_binding.clone(),
        producer: guardengine::integration::Producer {
            guard: "archguard".into(),
            analyzer_version: "1".into(),
            version: "1".into(),
            analyzer_id: "archguard".into(),
        },
        required_scopes: vec!["R".into()],
        contract_digest: draft.contract_digest().into(),
        action: "architecture-baseline:approve".into(),
        producer_principals: BTreeSet::from(["issuer".into()]),
        approval_principals: BTreeMap::from([(
            "architecture-baseline".into(),
            BTreeSet::from(["reviewer".into()]),
        )]),
    };
    let issuer = Issuer {
        record: ApprovalRecord {
            principal: "reviewer".into(),
            purpose: "architecture-baseline".into(),
            action: protected.action.clone(),
            binding: approval_binding,
            contract_digest: draft.contract_digest().into(),
            validity: Validity {
                issued_at: 10,
                expires_at: 100,
                revoked: false,
            },
        },
    };
    assert!(
        draft
            .approve("external:approval", &protected, &issuer, 50)
            .is_err()
    );
    let approved = review
        .approve("external:approval", &protected, &issuer, 50)
        .unwrap();
    assert_eq!(approved.state(), ArchitectureBaselineState::Approved);
    approved.qualify(&protected, &issuer, 50).unwrap();
    assert!(approved.qualify(&protected, &issuer, 100).is_err());
    let revised = approved
        .revise(
            BTreeMap::from([("ADR1".into(), b"approved: true\ndelete B".to_vec())]),
            &common::evidence::contract(),
            BTreeSet::from(["rule-A".into()]),
        )
        .unwrap();
    assert_ne!(approved.content_digest(), revised.content_digest());
    assert_eq!(revised.state(), ArchitectureBaselineState::Draft);
    assert!(
        revised
            .submit()
            .unwrap()
            .approve("external:approval", &protected, &issuer, 50)
            .is_err()
    );
    for mutation in 0..8 {
        let mut record = issuer.record.clone();
        match mutation {
            0 => record.principal = "candidate-self-approved".into(),
            1 => record.purpose = "other-purpose".into(),
            2 => record.action = "other-action".into(),
            3 => record.binding.task_id = "other-task".into(),
            4 => record.contract_digest = format!("sha256:{}", "f".repeat(64)),
            5 => record.validity.revoked = true,
            6 => record.validity.issued_at = 51,
            _ => record.validity.expires_at = 50,
        }
        assert!(
            approved
                .qualify(&protected, &Issuer { record }, 50)
                .is_err(),
            "mutation {mutation}"
        );
    }
    assert!(
        review
            .approve("missing:external", &protected, &issuer, 50)
            .is_err()
    );
    assert!(approved.submit().is_err());
    // A removed obligation can be approved only by a newly authenticated exact revision.
    let mut revised_policy = protected.clone();
    revised_policy.binding.baseline_digest = Some(revised.content_digest().into());
    let mut revised_record = issuer.record.clone();
    revised_record.binding = revised_policy.binding.clone();
    let replacement = revised
        .submit()
        .unwrap()
        .approve(
            "external:approval",
            &revised_policy,
            &Issuer {
                record: revised_record.clone(),
            },
            50,
        )
        .unwrap();
    assert_eq!(replacement.obligations().len(), 1);
    assert_eq!(approved.obligations().len(), 2);
    let mut revoke_policy = protected.clone();
    revoke_policy.action = "architecture-baseline:revoke".into();
    assert!(
        approved
            .revoke("external:approval", &revoke_policy, &issuer, 50)
            .is_err()
    );
    let mut revoke_record = issuer.record.clone();
    revoke_record.action = revoke_policy.action.clone();
    let revoked = approved
        .revoke(
            "external:approval",
            &revoke_policy,
            &Issuer {
                record: revoke_record,
            },
            50,
        )
        .unwrap();
    assert_eq!(revoked.state(), ArchitectureBaselineState::Revoked);
    assert!(revoked.qualify(&protected, &issuer, 50).is_err());
    assert!(revoked.submit().is_err());
    assert!(
        revoked
            .revise(
                BTreeMap::from([("ADR1".into(), b"rewrite".to_vec())]),
                &common::evidence::contract(),
                BTreeSet::from(["B".into()])
            )
            .is_err()
    );
    struct Records(BTreeMap<String, ApprovalRecord>);
    impl AuthorityProvider for Records {
        fn verify_producer(
            &self,
            _: &guardengine::integration::GuardRunEnvelope,
            _: &str,
        ) -> Result<ProducerRecord, AuthorityError> {
            Err(AuthorityError::Unavailable)
        }
        fn verify_approval(&self, reference: &str) -> Result<ApprovalRecord, AuthorityError> {
            self.0
                .get(reference)
                .cloned()
                .ok_or(AuthorityError::Unavailable)
        }
    }
    let mut retire_policy = protected.clone();
    retire_policy.action = "architecture-baseline:supersede".into();
    let mut retire_record = issuer.record.clone();
    retire_record.action = retire_policy.action.clone();
    let records = Records(BTreeMap::from([
        ("external:approval".into(), revised_record),
        ("external:retire".into(), retire_record),
    ]));
    let superseded = approved
        .supersede(
            &replacement,
            &revised_policy,
            "external:retire",
            &retire_policy,
            &records,
            50,
        )
        .unwrap();
    assert_eq!(superseded.state(), ArchitectureBaselineState::Superseded);
    assert!(superseded.qualify(&protected, &issuer, 50).is_err());
    assert!(
        approved
            .supersede(
                &replacement,
                &revised_policy,
                "external:retire",
                &retire_policy,
                &records,
                100
            )
            .is_err()
    );
    // Candidate serialization cannot construct this private capability or mutate old records.
    assert_eq!(draft.state(), ArchitectureBaselineState::Draft);
    assert_eq!(approved.state(), ArchitectureBaselineState::Approved);
    assert_eq!(frozen, serde_json::to_vec(&draft).unwrap());
}

#[test]
fn architecture_baseline_admits_borrowed_bytes_before_hash_and_rejects_empty_obligations() {
    use archguard::domain::baseline::ArchitectureBaseline;
    use guardengine::integration::RunBinding;
    use std::collections::{BTreeMap, BTreeSet};
    let binding: RunBinding = serde_json::from_value(serde_json::json!({"repoId":"repo", "taskId":"task", "worktreeId":"worktree", "requirementIds":["R"], "mergeGroupId":null, "baselineDigest":null, "candidateOid":"a".repeat(40), "baseOid":"b".repeat(40), "sourceSnapshotDigest":format!("sha256:{}", "c".repeat(64))})).unwrap();
    let contract = common::evidence::contract();
    let adrs = BTreeMap::from([
        ("ADR".into(), b"accepted".to_vec()),
        ("ADR2".into(), b"decision".to_vec()),
    ]);
    let obligations = BTreeSet::from(["A".into(), "B".into()]);
    let original = ArchitectureBaseline::draft(
        binding.clone(),
        "source",
        adrs.clone(),
        &contract,
        obligations.clone(),
    )
    .unwrap();
    for change in 0..6 {
        let mut b = binding.clone();
        let mut a = adrs.clone();
        let mut c = contract.clone();
        let mut o = obligations.clone();
        let source_ref = if change == 0 {
            "other-source"
        } else {
            "source"
        };
        match change {
            1 => b.source_snapshot_digest = format!("sha256:{}", "d".repeat(64)),
            2 => {
                a.insert("ADR".into(), b"changed".to_vec());
            }
            3 => {
                a.remove("ADR2");
            }
            4 => c.metadata.revision = "different-contract".into(),
            5 => {
                o.remove("B");
            }
            _ => {}
        }
        let changed = ArchitectureBaseline::draft(b, source_ref, a, &c, o).unwrap();
        assert_ne!(
            original.content_digest(),
            changed.content_digest(),
            "independent dimension {change}"
        );
    }
    assert!(
        ArchitectureBaseline::draft(
            binding.clone(),
            "source",
            BTreeMap::from([("ADR".into(), vec![0; 1_048_576])]),
            &contract,
            BTreeSet::from(["R".into()])
        )
        .is_err()
    );
    assert!(
        ArchitectureBaseline::draft(
            binding.clone(),
            "source",
            BTreeMap::from([("ADR".into(), b"accepted".to_vec())]),
            &contract,
            BTreeSet::new()
        )
        .is_err()
    );
    assert!(
        ArchitectureBaseline::draft(
            binding,
            "source",
            BTreeMap::new(),
            &contract,
            BTreeSet::from(["R".into()])
        )
        .is_err()
    );
}
