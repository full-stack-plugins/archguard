mod common;
use archguard::integration::projection::{CargoEvidence, ProtectedCargoPolicy};
use common::{
    Temp,
    evidence::{context, contract},
};
use guardengine::{
    Decision, GuardAnalyzer,
    integration::{CoverageStatus, RunStatus},
};
use std::sync::atomic::AtomicBool;
#[test]
fn missing_oid_and_unresolved_source_produce_transport_diagnostics_only() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let valid_ctx = context(&root.0);
    let mut ctx = valid_ctx.clone();
    ctx.candidate_oid.clear();
    let result = CargoEvidence::prepare(
        &root.0,
        ProtectedCargoPolicy::freeze(contract(), vec![]).unwrap(),
        ctx,
    );
    assert_eq!(result.err().unwrap().code, "binding.invalid");
    let ctx = valid_ctx;
    assert!(
        CargoEvidence::prepare(
            &root.0.join("absent"),
            ProtectedCargoPolicy::freeze(contract(), vec![]).unwrap(),
            ctx
        )
        .is_err()
    );
}
#[test]
fn valid_missing_scope_is_partial_block_and_cancelled_attempt_has_no_report() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let ctx = context(&root.0);
    let policy = || ProtectedCargoPolicy::freeze(contract(), vec![]).unwrap();
    let partial = CargoEvidence::prepare(&root.0, policy(), ctx.clone())
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(partial.envelope.run_status, RunStatus::Completed);
    assert_eq!(partial.envelope.decision, Some(Decision::Block));
    assert_eq!(partial.envelope.coverage.status, CoverageStatus::Partial);
    assert!(!partial.envelope.coverage.missing_scopes.is_empty());
    let cancelled = CargoEvidence::prepare(&root.0, policy(), ctx)
        .unwrap()
        .run(&AtomicBool::new(true))
        .unwrap();
    assert_eq!(cancelled.envelope.run_status, RunStatus::Cancelled);
    assert!(cancelled.envelope.decision.is_none());
    assert!(cancelled.report.is_none());
    assert!(cancelled.envelope.coverage.observed_scopes.is_empty());
    assert_ne!(partial.envelope.run_id, cancelled.envelope.run_id);
}
#[test]
fn actual_metadata_failure_is_bound_error_while_legacy_remains_partial() {
    let root = Temp::new();
    root.copy_fixture("allowed");
    let ctx = context(&root.0);
    let manifest = root.0.join("agent-job/Cargo.toml");
    let text = std::fs::read_to_string(&manifest)
        .unwrap()
        .replace("0.1.0", "not-a-version");
    std::fs::write(&manifest, text).unwrap();
    let result = CargoEvidence::prepare(
        &root.0,
        ProtectedCargoPolicy::freeze(contract(), vec![]).unwrap(),
        ctx,
    )
    .unwrap()
    .run(&AtomicBool::new(false))
    .unwrap();
    assert_eq!(result.envelope.run_status, RunStatus::Error);
    assert!(result.envelope.decision.is_none());
    assert!(result.report.is_none());
    assert!(result.domain.is_none());
    let legacy = archguard::CargoWorkspaceAnalyzer
        .analyze(&root.0, "legacy")
        .unwrap();
    assert_eq!(legacy.completeness, guardengine::Completeness::Partial);
    assert_eq!(
        guardengine::evaluate(&contract(), &legacy)
            .unwrap()
            .decision,
        Decision::Block
    );
}
#[test]
fn protected_policy_and_isolated_source_stay_frozen_after_preparation() {
    let root = Temp::new();
    root.copy_fixture("forbidden");
    let ctx = context(&root.0);
    let prepared = CargoEvidence::prepare(
        &root.0,
        ProtectedCargoPolicy::freeze(contract(), vec![]).unwrap(),
        ctx,
    )
    .unwrap();
    std::fs::write(
        root.0.join("architecture.yaml"),
        "approved: true\nrules: []\n",
    )
    .unwrap();
    std::fs::write(root.0.join("agent-job/Cargo.toml"), "invalid replacement").unwrap();
    let bundle = prepared.run(&AtomicBool::new(false)).unwrap();
    assert_eq!(bundle.envelope.run_status, RunStatus::Completed);
    assert_eq!(bundle.envelope.decision, Some(Decision::Block));
}
#[test]
fn real_large_declaration_graph_refuses_engine_amplification_as_bound_error() {
    let root = Temp::new();
    let mut members = vec!["core".to_owned()];
    members.extend((0..600).map(|i| format!("app{i}")));
    std::fs::write(
        root.0.join("Cargo.toml"),
        format!("[workspace]\nresolver=\"2\"\nmembers={:?}\n", members),
    )
    .unwrap();
    for name in &members {
        let dir = root.0.join(name);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let dependency = if name == "core" {
            ""
        } else {
            "[dependencies]\ncore={path=\"../core\"}\n"
        };
        std::fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[package]\nname=\"{name}\"\nversion=\"0.1.0\"\nedition=\"2021\"\n{dependency}"
            ),
        )
        .unwrap();
        std::fs::write(dir.join("src/lib.rs"), "").unwrap();
    }
    let ctx = context(&root.0);
    let mut contract = contract();
    let guardengine::GuardAssertion::ForbidRelation {
        subject, object, ..
    } = &mut contract.spec.rules[0].assertion;
    *subject = "app0".into();
    *object = "core".into();
    let rule = contract.spec.rules[0].clone();
    contract.spec.rules = (0..256)
        .map(|i| {
            let mut rule = rule.clone();
            rule.id = format!("rule-{i}");
            rule
        })
        .collect();
    let native_profile = archguard::analysis::profile::FrozenAnalysisProfile::freeze(&contract, []);
    let native = archguard::analysis::analyze(&root.0, "budget-proof", &native_profile).unwrap();
    assert_eq!(native.facts.len(), 600);
    assert_eq!(native.completeness, guardengine::Completeness::Complete);
    assert!(guardengine::integration::evaluate_bounded(&contract, &native).is_err());
    let policy = ProtectedCargoPolicy::freeze(contract, vec![]).unwrap();
    let outcome = CargoEvidence::prepare(&root.0, policy, ctx)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(outcome.envelope.run_status, RunStatus::Error);
    assert!(outcome.envelope.decision.is_none());
    assert!(outcome.report.is_none());
}
#[test]
fn real_engine_review_and_advise_decisions_are_not_rewritten_by_adapter() {
    let root = Temp::new();
    root.copy_fixture("forbidden");
    let ctx = context(&root.0);
    for (enforcement, expected) in [
        (guardengine::Enforcement::Review, Decision::RequireApproval),
        (guardengine::Enforcement::Advise, Decision::Allow),
    ] {
        let mut contract = contract();
        contract.spec.rules[0].enforcement = enforcement;
        let outcome = CargoEvidence::prepare(
            &root.0,
            ProtectedCargoPolicy::freeze(contract, vec![]).unwrap(),
            ctx.clone(),
        )
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
        assert_eq!(outcome.envelope.decision, Some(expected));
        assert!(outcome.envelope.approval_refs.is_empty());
        assert_eq!(outcome.envelope.coverage.status, CoverageStatus::Complete);
        outcome.verify().unwrap();
    }
}
