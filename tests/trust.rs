mod common;
#[path = "common/trusted.rs"]
mod trusted;
use archguard::integration::{projection::*, trust::*};
use guardengine::{Decision, integration::eligibility::*};
use trusted::*;
#[test]
fn protected_cargo_capability_requires_real_artifacts_and_fresh_authority() {
    let (_root, b, p, a) = fixture(false, false);
    assert!(consume(&b, &p, &a, NOW, None).eligible);
    for mutation in 0..8 {
        let mut changed = EvidenceBundle::load(&bytes(&b).unwrap()).unwrap();
        match mutation {
            0 => changed.domain = None,
            1 => changed.facts = None,
            2 => {
                changed.envelope.artifacts.report.as_mut().unwrap().uri =
                    "https://attacker.invalid/report".into()
            }
            3 => changed.envelope.producer.analyzer_version = "forged".into(),
            4 => changed.envelope.coverage.observed_scopes.clear(),
            5 => changed.envelope.binding.requirement_ids = vec!["other".into()],
            6 => changed.envelope.binding.candidate_oid = "a".repeat(40),
            _ => changed.envelope.artifacts.domain.clear(),
        }
        assert!(
            !consume(&changed, &p, &a, NOW, None).eligible,
            "mutation {mutation}"
        );
    }
    let mut unsupported = p.clone();
    unsupported
        .required_capabilities
        .insert("domain.method.purity".into());
    assert_eq!(
        consume(&b, &unsupported, &a, NOW, None).code,
        EligibilityCode::InvalidPolicy
    );
    let mut weak = p.clone();
    weak.eligibility.contract_digest = digest(b"weakened policy");
    assert_eq!(
        consume(&b, &weak, &a, NOW, None).code,
        EligibilityCode::BindingChanged
    );
    let mut signed = serde_json::to_value(&b).unwrap();
    signed["envelope"]["signed"] = true.into();
    assert!(EvidenceBundle::load(&bytes(&signed).unwrap()).is_err());
}
#[test]
fn producer_identity_freshness_and_unavailability_are_rechecked() {
    let (_root, b, p, a) = fixture(false, false);
    for mutation in 0..5 {
        let mut a = a.clone();
        match mutation {
            0 => a.producer_principal = "forged".into(),
            1 => a.producer_validity.expires_at = NOW,
            2 => a.producer_validity.revoked = true,
            3 => a.unavailable = true,
            _ => a.producer_digest = Some(digest(b"other envelope")),
        }
        assert!(
            !consume(&b, &p, &a, NOW, None).eligible,
            "mutation {mutation}"
        );
    }
}
#[test]
fn baseline_and_exception_require_exact_external_scope_even_for_allow() {
    let (_root, b, mut p, mut a) = fixture(false, false);
    approve(&mut p, &mut a, "architecture-baseline");
    approve(&mut p, &mut a, "architecture-exception");
    assert!(consume(&b, &p, &a, NOW, None).eligible);
    for purpose in ["architecture-baseline", "architecture-exception"] {
        for mutation in 0..9 {
            let mut a = a.clone();
            let r = a
                .approvals
                .get_mut(&format!("fixture-authority:{purpose}"))
                .unwrap();
            match mutation {
                0 => r.validity.revoked = true,
                1 => r.validity.expires_at = NOW,
                2 => r.binding.requirement_ids = vec!["other".into()],
                3 => r.binding.candidate_oid = "b".repeat(40),
                4 => r.contract_digest = digest(b"wrong"),
                5 => r.action = "push".into(),
                6 => r.purpose = "other-purpose".into(),
                7 => r.principal = "forged".into(),
                _ => r.binding.baseline_digest = Some(digest(b"other baseline")),
            }
            assert!(
                !consume(&b, &p, &a, NOW, None).eligible,
                "{purpose} mutation {mutation}"
            );
        }
    }
    a.approvals.clear();
    assert!(!consume(&b, &p, &a, NOW, None).eligible);
}
#[test]
fn approved_review_and_partial_keep_original_technical_decisions_and_redacted_audit() {
    let (_root, mut b, mut p, mut a) = fixture(false, true);
    approve(&mut p, &mut a, "architecture-exception");
    b.envelope.approval_refs = p.required_approvals.values().cloned().collect();
    let before = bytes(&b).unwrap();
    let r = consume(&b, &p, &a, NOW, Some("secret controller predecessor"));
    assert!(r.eligible, "{r:?}");
    assert_eq!(r.technical_decision, Some(Decision::RequireApproval));
    assert_eq!(bytes(&b).unwrap(), before);
    let audit = serde_json::to_string(&r.audit).unwrap();
    for raw in [
        "fixture-producer",
        "fixture-approver",
        "secret controller predecessor",
    ] {
        assert!(!audit.contains(raw));
    }
    assert_eq!(r.audit.content_digest, p.content_digest());
    let (_root, b, mut p, mut a) = fixture(true, false);
    approve(&mut p, &mut a, "architecture-exception");
    let before = bytes(&b).unwrap();
    let r = consume(&b, &p, &a, NOW, None);
    assert!(!r.eligible);
    assert_eq!(r.technical_decision, Some(Decision::Block));
    assert_eq!(bytes(&b).unwrap(), before);
}
