mod common;
#[allow(dead_code)]
#[path = "common/trusted.rs"]
mod trusted;
use archguard::{
    CargoWorkspaceAnalyzer,
    integration::{projection::bytes, rollout::*, trust::consume},
};
use guardengine::{GuardAnalyzer, integration::attempt_store::*};
use trusted::*;
#[test]
fn actual_cargo_four_stage_rollout_preserves_required_check_and_history() {
    let (root, b, p, mut a) = fixture(false, false);
    let native = CargoWorkspaceAnalyzer
        .analyze(&root.0, "native-shadow")
        .unwrap();
    let original = bytes(&b).unwrap();
    let mut store = InMemoryAttemptStore::default();
    let target = Target {
        repo_id: p.eligibility.binding.repo_id.clone(),
        task_id: p.eligibility.binding.task_id.clone(),
        requirement_ids: p.eligibility.binding.requirement_ids.clone(),
    };
    let generation = store
        .advance(
            target.clone(),
            0,
            p.content_digest(),
            b.envelope.run_id.clone(),
        )
        .unwrap();
    let mut observations = vec![];
    for stage in [
        Stage::Advisory,
        Stage::Shadow,
        Stage::OptInProtected,
        Stage::Enforce,
    ] {
        let r = check_stage(
            &b,
            &p,
            &a,
            NOW,
            Selection {
                stage,
                adapter_enabled: true,
                native_facts: Some(&native),
            },
        );
        assert_eq!(
            r.required_check_satisfied,
            matches!(stage, Stage::OptInProtected | Stage::Enforce)
        );
        observations.push(serde_json::to_value(&r).unwrap());
    }
    let r = consume(&b, &p, &a, NOW, None);
    store
        .append(AttemptRecord {
            run_id: b.envelope.run_id.clone(),
            target: target.clone(),
            generation,
            content_digest: p.content_digest(),
            envelope_digest: r.audit.envelope_digest.clone(),
            eligible: r.eligible,
        })
        .unwrap();
    store.publish(&b.envelope.run_id).unwrap();
    let disabled = check_stage(
        &b,
        &p,
        &a,
        NOW,
        Selection {
            stage: Stage::Enforce,
            adapter_enabled: false,
            native_facts: Some(&native),
        },
    );
    assert!(!disabled.required_check_satisfied);
    let rollback = check_stage(
        &b,
        &p,
        &a,
        NOW,
        Selection {
            stage: Stage::Advisory,
            adapter_enabled: true,
            native_facts: Some(&native),
        },
    );
    assert!(!rollback.required_check_satisfied);
    a.producer_validity.revoked = true;
    let revoked = check_stage(
        &b,
        &p,
        &a,
        NOW,
        Selection {
            stage: Stage::Enforce,
            adapter_enabled: true,
            native_facts: Some(&native),
        },
    );
    assert!(!revoked.required_check_satisfied);
    assert!(
        store.current(&target).unwrap().eligible,
        "historical eligibility cannot replace fresh consumption"
    );
    let mut changed = p.clone();
    changed.eligibility.binding.candidate_oid = "c".repeat(40);
    store
        .advance(
            target.clone(),
            generation,
            changed.content_digest(),
            "new-attempt".into(),
        )
        .unwrap();
    assert!(store.publish(&b.envelope.run_id).is_err());
    assert!(store.current(&target).is_none());
    assert_eq!(store.history(&target).len(), 1);
    assert_eq!(bytes(&b).unwrap(), original);
    let output = serde_json::json!({"profile":"cargo-declarations-v1","authority":"local-fixture-only","native":"actual CargoWorkspaceAnalyzer facts/completeness parity; not CLI report parity","bundle":b,"nativeFacts":native,"protectedPolicy":p,"binding":b.envelope.binding,"capabilities":archguard::integration::trust::CAPABILITIES,"artifacts":b.envelope.artifacts,"stages":observations,"disabled":disabled,"rollback":rollback,"revoked":revoked,"retained_history":store.history(&target).len(),"original_bytes_unchanged":true,"late_publish_rejected":true});
    if let Ok(path) = std::env::var("ARCHGUARD_ROLLOUT_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    }
}
#[test]
fn native_mismatch_missing_coverage_and_expired_approval_never_satisfy_protected_rollout() {
    let (root, b, mut p, mut a) = fixture(false, false);
    approve(&mut p, &mut a, "architecture-baseline");
    let mut native = CargoWorkspaceAnalyzer
        .analyze(&root.0, "native-shadow")
        .unwrap();
    fn selection(facts: Option<&guardengine::GuardFacts>) -> Selection<'_> {
        Selection {
            stage: Stage::Enforce,
            adapter_enabled: true,
            native_facts: facts,
        }
    }
    assert!(check_stage(&b, &p, &a, NOW, selection(Some(&native))).required_check_satisfied);
    assert!(!check_stage(&b, &p, &a, NOW, selection(None)).required_check_satisfied);
    native.facts.clear();
    assert!(!check_stage(&b, &p, &a, NOW, selection(Some(&native))).required_check_satisfied);
    let native = CargoWorkspaceAnalyzer
        .analyze(&root.0, "native-shadow")
        .unwrap();
    a.approvals.values_mut().next().unwrap().validity.expires_at = NOW;
    assert!(!check_stage(&b, &p, &a, NOW, selection(Some(&native))).required_check_satisfied);
    let (root, b, p, a) = fixture(true, false);
    let native = CargoWorkspaceAnalyzer
        .analyze(&root.0, "native-shadow")
        .unwrap();
    assert!(!check_stage(&b, &p, &a, NOW, selection(Some(&native))).required_check_satisfied);
}
