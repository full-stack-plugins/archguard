mod common;
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
