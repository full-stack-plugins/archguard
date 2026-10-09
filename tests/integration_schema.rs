mod common;
use archguard::integration::projection::{CargoEvidence, EvidenceBundle, ProtectedCargoPolicy};
use common::Temp;
use guardengine::{Decision, load_contract_yaml};
use std::{path::Path, sync::atomic::AtomicBool};
fn contract() -> guardengine::GuardContract {
    let mut c = load_contract_yaml(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/agent-job-contract.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let guardengine::GuardAssertion::ForbidRelation { object, .. } = &mut c.spec.rules[0].assertion;
    *object = "agent-job".into();
    c
}
fn policy() -> ProtectedCargoPolicy {
    ProtectedCargoPolicy::freeze(contract(), vec![]).unwrap()
}
use common::evidence::context;
#[test]
fn actual_cargo_producer_exports_verifiable_independent_envelope_and_native_artifacts() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let bundle = CargoEvidence::prepare(&root.0, policy(), context(&root.0))
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(bundle.envelope.decision, Some(Decision::Allow));
    bundle.verify().unwrap();
    assert_eq!(
        bundle.facts.as_ref().unwrap()["apiVersion"],
        guardengine::API_VERSION
    );
    assert_eq!(
        bundle.domain.as_ref().unwrap()["profile"],
        "cargo-declarations-v1"
    );
    let bytes = serde_json::to_vec(&bundle).unwrap();
    EvidenceBundle::load(&bytes).unwrap().verify().unwrap();
    for name in ["binding", "coverage"] {
        let mut native = bundle.facts.clone().unwrap();
        native[name] = serde_json::json!({});
        assert!(guardengine::load_facts_json(&serde_json::to_vec(&native).unwrap()).is_err());
    }
}
#[test]
fn frozen_input_budgets_reject_before_profile_expansion() {
    let mut contract = contract();
    let rule = contract.spec.rules[0].clone();
    contract.spec.rules = (0..1000)
        .map(|i| {
            let mut rule = rule.clone();
            rule.id = format!("rule-{i}");
            rule
        })
        .collect();
    assert!(ProtectedCargoPolicy::freeze(contract, vec![]).is_err());
}

#[test]
fn strict_bundle_rejects_unknown_versions_fields_and_report_disagreement() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let bundle = CargoEvidence::prepare(&root.0, policy(), context(&root.0))
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let base = serde_json::to_value(&bundle).unwrap();
    for mutation in 0..7 {
        let mut value = base.clone();
        match mutation {
            0 => value["api_version"] = "archguard.evidence/future".into(),
            1 => value["envelope"]["apiVersion"] = "guard.integration/future".into(),
            2 => value["envelope"]["approved"] = true.into(),
            3 => value["envelope"]["decision"] = "BLOCK".into(),
            4 => value["report"]["decision"] = "BLOCK".into(),
            5 => value["domain"]["approved"] = true.into(),
            6 => {
                value.as_object_mut().unwrap().remove("facts");
            }
            _ => unreachable!(),
        }
        assert!(
            EvidenceBundle::load(&serde_json::to_vec(&value).unwrap()).is_err(),
            "mutation {mutation}"
        );
    }
    let mut report = bundle.report.clone().unwrap();
    report["coverage"] = serde_json::json!({});
    assert!(serde_json::from_value::<guardengine::GuardReport>(report).is_err());
}
#[test]
fn domain_retains_all_declaration_kinds_and_matches_existing_native_facts() {
    let root = Temp::new();
    root.copy_fixture("profile");
    let context = context(&root.0);
    let mut contract = contract();
    let guardengine::GuardAssertion::ForbidRelation {
        subject, object, ..
    } = &mut contract.spec.rules[0].assertion;
    *subject = "app".into();
    *object = "app".into();
    let profile = archguard::analysis::profile::FrozenAnalysisProfile::freeze(&contract, []);
    let native = archguard::analysis::analyze(&root.0, "native-parity", &profile).unwrap();
    let policy = ProtectedCargoPolicy::freeze(contract, vec![]).unwrap();
    let bundle = CargoEvidence::prepare(&root.0, policy, context)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let projected =
        guardengine::load_facts_json(&serde_json::to_vec(bundle.facts.as_ref().unwrap()).unwrap())
            .unwrap();
    assert_eq!(projected.facts, native.facts);
    assert_eq!(
        projected.subject.snapshot_digest,
        native.subject.snapshot_digest
    );
    let declarations = bundle.domain.as_ref().unwrap()["declarations"]
        .as_array()
        .unwrap();
    assert!(declarations.iter().any(|d| d["kind"] == "dev"));
    assert!(declarations.iter().any(|d| d["kind"] == "build"));
    assert!(declarations.iter().any(|d| d["optional"] == true));
    assert!(declarations.iter().any(|d| !d["target"].is_null()));
    assert!(declarations.iter().any(|d| !d["rename"].is_null()));
    assert!(declarations.iter().any(|d| d["member_target"].is_null()));
}
#[test]
fn rehashed_unknown_or_invalid_domain_fields_remain_rejected() {
    use sha2::{Digest, Sha256};
    let root = Temp::new();
    root.copy_fixture("allowed");
    let bundle = CargoEvidence::prepare(&root.0, policy(), context(&root.0))
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    for (field, replacement) in [
        ("approved", serde_json::json!(true)),
        ("manifest_digest", serde_json::json!("not-a-digest")),
        ("profile", serde_json::json!("active-feature-graph")),
    ] {
        let mut object = serde_json::to_value(&bundle).unwrap();
        object["domain"][field] = replacement;
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&object["domain"]).unwrap())
        );
        object["envelope"]["artifacts"]["domain"][0]["digest"] = format!("sha256:{digest}").into();
        object["envelope"]["artifacts"]["domain"][0]["uri"] =
            format!("artifact://archguard/domain/{digest}").into();
        assert!(
            EvidenceBundle::load(&serde_json::to_vec(&object).unwrap()).is_err(),
            "{field}"
        );
    }
}
#[test]
fn duplicate_native_artifact_fields_are_not_lost_during_bundle_loading() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let bundle = CargoEvidence::prepare(&root.0, policy(), context(&root.0))
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let raw = serde_json::to_string(&bundle).unwrap();
    let duplicate = raw.replacen(
        "\"kind\":\"GuardFacts\"",
        "\"kind\":\"GuardFacts\",\"kind\":\"GuardFacts\"",
        1,
    );
    assert_ne!(raw, duplicate);
    assert!(EvidenceBundle::load(duplicate.as_bytes()).is_err());
}
#[test]
fn artifact_namespaces_cannot_be_relabelled_with_unchanged_bytes() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let bundle = CargoEvidence::prepare(&root.0, policy(), context(&root.0))
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let mut value = serde_json::to_value(&bundle).unwrap();
    let hash = value["envelope"]["artifacts"]["facts"]["digest"]
        .as_str()
        .unwrap()
        .trim_start_matches("sha256:")
        .to_owned();
    value["envelope"]["artifacts"]["facts"]["uri"] =
        format!("artifact://another-producer/facts/{hash}").into();
    assert!(EvidenceBundle::load(&serde_json::to_vec(&value).unwrap()).is_err());
}
