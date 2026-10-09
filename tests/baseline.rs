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
