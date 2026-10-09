use archguard::domain::{model::*, rules::*};
use guardengine::{Completeness, Decision, Enforcement, RuleStatus};
use std::collections::{BTreeMap, BTreeSet};
fn module(name: &str) -> ModuleKey {
    ModuleKey::new(Language::Rust, name).unwrap()
}
fn id(name: &str) -> SymbolId {
    SymbolId::new(Language::Rust, name, "", SymbolKind::Module, name, "").unwrap()
}
fn span(name: &str) -> SourceSpan {
    SourceSpan::new(&format!("src/{name}.rs"), 1, 1, 1, 2).unwrap()
}
fn observation(edges: &[(&str, &str)], complete: bool, names: &[&str]) -> LanguageObservation {
    let mut o = LanguageObservation::new(Language::Rust, "fixture/v1").unwrap();
    for n in names {
        o.add_symbol(Symbol {
            id: id(n),
            source: span(n),
        })
        .unwrap();
    }
    for (a, b) in edges {
        o.confirm_relation(Relation::DependsOn, &id(a), &id(b), span(a))
            .unwrap();
    }
    if complete {
        o.mark_scope_complete(Relation::DependsOn);
    }
    o
}
fn kinds() -> Vec<SystemRuleKind> {
    vec![
        SystemRuleKind::Layers {
            levels: BTreeMap::from([(module("a"), 1), (module("b"), 0), (module("c"), 0)]),
        },
        SystemRuleKind::ForbiddenDirection {
            from: module("b"),
            to: module("a"),
        },
        SystemRuleKind::Acyclic,
    ]
}
fn policy(kind: SystemRuleKind, enforcement: Enforcement) -> ProtectedSystemPolicy {
    ProtectedSystemPolicy::freeze(
        BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
        BTreeSet::from([module("a"), module("b"), module("c")]),
        Relation::DependsOn,
        vec![SystemRule {
            id: "R".into(),
            enforcement,
            kind,
        }],
    )
    .unwrap()
}
#[test]
fn every_enforce_rule_has_legal_violation_and_missing_coverage() {
    for kind in kinds() {
        let p = policy(kind, Enforcement::Enforce);
        let legal = p
            .evaluate(&[observation(&[("a", "b")], true, &["a", "b", "c"])])
            .unwrap();
        assert_eq!(legal.report().decision, Decision::Allow);
        let bad = p
            .evaluate(&[observation(
                &[("a", "b"), ("b", "a")],
                true,
                &["a", "b", "c"],
            )])
            .unwrap();
        assert_eq!(bad.report().decision, Decision::Block);
        assert!(!bad.findings().is_empty());
        for o in [
            observation(&[], false, &["a", "b", "c"]),
            observation(&[], true, &["a", "b"]),
        ] {
            let partial = p.evaluate(&[o]).unwrap();
            assert_eq!(partial.facts().completeness, Completeness::Partial);
            assert_eq!(partial.report().decision, Decision::Block);
            assert_eq!(
                partial.report().evaluations[0].status,
                RuleStatus::Indeterminate
            );
        }
    }
}
#[test]
fn cycle_witness_is_contiguous_actual_source_and_deterministic() {
    let p = policy(SystemRuleKind::Acyclic, Enforcement::Enforce);
    let first = observation(
        &[("b", "c"), ("c", "a"), ("a", "b")],
        true,
        &["b", "a", "c"],
    );
    let second = observation(
        &[("a", "b"), ("c", "a"), ("b", "c")],
        true,
        &["c", "a", "b"],
    );
    let a = p.evaluate(std::slice::from_ref(&first)).unwrap();
    let b = p.evaluate(&[second]).unwrap();
    assert_eq!(a.findings(), b.findings());
    assert_eq!(a.report(), b.report());
    let path = &a.findings()[0].path;
    assert_eq!(path.len(), 3);
    for i in 0..path.len() {
        assert_eq!(path[i].to, path[(i + 1) % path.len()].from);
        assert!(first.edges().contains(&path[i]));
    }
    assert!(guardengine::verify_report(a.report(), a.contract(), a.facts()).unwrap());
}
#[test]
fn unresolved_and_absent_provider_cannot_be_empty_success() {
    for kind in kinds() {
        let p = policy(kind, Enforcement::Enforce);
        let mut o = observation(&[], true, &["a", "b", "c"]);
        o.unknown_relation(
            Relation::DependsOn,
            &id("a"),
            span("a"),
            "unresolved import",
        )
        .unwrap();
        assert_eq!(p.evaluate(&[o]).unwrap().report().decision, Decision::Block);
        assert_eq!(p.evaluate(&[]).unwrap().report().decision, Decision::Block);
    }
}
#[test]
fn enforcement_is_preserved_by_exact_neutral_projection() {
    for kind in kinds() {
        for (enforcement, expected) in [
            (Enforcement::Review, Decision::RequireApproval),
            (Enforcement::Advise, Decision::Allow),
        ] {
            let p = policy(kind.clone(), enforcement);
            let r = p
                .evaluate(&[observation(
                    &[("a", "b"), ("b", "a")],
                    true,
                    &["a", "b", "c"],
                )])
                .unwrap();
            assert_eq!(r.report().decision, expected);
            assert!(!r.findings().is_empty());
            assert!(!r.report().signed);
        }
    }
}
#[test]
fn unsupported_relation_profile_and_scope_are_not_weakened() {
    let p = policy(SystemRuleKind::Acyclic, Enforcement::Enforce);
    let wrong = LanguageObservation::new(Language::Rust, "another/v1").unwrap();
    assert!(p.evaluate(&[wrong]).is_err());
    let extra = observation(&[], true, &["a", "b", "c", "d"]);
    assert_eq!(
        p.evaluate(&[extra]).unwrap().report().decision,
        Decision::Block
    );
    let o = observation(&[], true, &["a", "b", "c"]);
    assert!(p.evaluate(&[o.clone(), o]).is_err());
    assert!(
        ProtectedSystemPolicy::freeze(
            BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
            BTreeSet::from([module("a")]),
            Relation::Calls,
            vec![SystemRule {
                id: "R".into(),
                enforcement: Enforcement::Enforce,
                kind: SystemRuleKind::Acyclic
            }]
        )
        .is_err()
    );
}
#[test]
fn policy_and_graph_budgets_refuse_before_expansion() {
    let rules = (0..65)
        .map(|n| SystemRule {
            id: format!("R{n}"),
            enforcement: Enforcement::Enforce,
            kind: SystemRuleKind::Acyclic,
        })
        .collect();
    assert!(
        ProtectedSystemPolicy::freeze(
            BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
            BTreeSet::from([module("a")]),
            Relation::DependsOn,
            rules
        )
        .is_err()
    );
    let p = policy(SystemRuleKind::Acyclic, Enforcement::Enforce);
    let mut o = observation(&[], true, &["a", "b", "c"]);
    for _ in 0..8193 {
        o.confirm_relation(Relation::DependsOn, &id("a"), &id("b"), span("a"))
            .unwrap();
    }
    assert!(p.evaluate(&[o]).is_err());
}

#[test]
fn repeated_layer_policy_is_bounded_before_serialization() {
    let modules = (0..1024)
        .map(|n| module(&format!("m{n:04}{}", "x".repeat(245))))
        .collect::<BTreeSet<_>>();
    let levels = modules
        .iter()
        .cloned()
        .map(|m| (m, 0))
        .collect::<BTreeMap<_, _>>();
    let rules = (0..64)
        .map(|n| SystemRule {
            id: format!("R{n}"),
            enforcement: Enforcement::Enforce,
            kind: SystemRuleKind::Layers {
                levels: levels.clone(),
            },
        })
        .collect();
    assert!(
        ProtectedSystemPolicy::freeze(
            BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
            modules,
            Relation::DependsOn,
            rules
        )
        .is_err()
    );
}
#[test]
fn self_cycles_disconnected_graph_and_internal_symbol_dependencies_have_explicit_semantics() {
    let p = policy(SystemRuleKind::Acyclic, Enforcement::Enforce);
    for edges in [vec![("c", "c")], vec![("b", "c"), ("c", "b")]] {
        let result = p
            .evaluate(&[observation(&edges, true, &["a", "b", "c"])])
            .unwrap();
        assert_eq!(result.report().decision, Decision::Block);
        assert_eq!(result.findings()[0].path.len(), edges.len());
    }
    let mut o = observation(&[], true, &["a", "b", "c"]);
    let a = SymbolId::new(Language::Rust, "a", "", SymbolKind::Type, "A", "").unwrap();
    let b = SymbolId::new(Language::Rust, "a", "", SymbolKind::Type, "B", "").unwrap();
    for s in [&a, &b] {
        o.add_symbol(Symbol {
            id: s.clone(),
            source: span("a"),
        })
        .unwrap();
    }
    o.confirm_relation(Relation::DependsOn, &a, &b, span("a"))
        .unwrap();
    o.confirm_relation(Relation::DependsOn, &b, &a, span("a"))
        .unwrap();
    assert_eq!(p.evaluate(&[o]).unwrap().report().decision, Decision::Allow);
}
#[test]
fn protected_profile_digest_and_actual_unknown_provenance_invalidate_reuse() {
    let mut rules = vec![SystemRule {
        id: "R".into(),
        enforcement: Enforcement::Enforce,
        kind: SystemRuleKind::Acyclic,
    }];
    let p = ProtectedSystemPolicy::freeze(
        BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
        BTreeSet::from([module("a"), module("b"), module("c")]),
        Relation::DependsOn,
        rules.clone(),
    )
    .unwrap();
    rules[0].enforcement = Enforcement::Advise;
    let changed = ProtectedSystemPolicy::freeze(
        BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
        BTreeSet::from([module("a"), module("b"), module("c")]),
        Relation::DependsOn,
        rules,
    )
    .unwrap();
    assert_ne!(p.profile_digest(), changed.profile_digest());
    let mut o = observation(&[], true, &["a", "b", "c"]);
    let before = p.evaluate(std::slice::from_ref(&o)).unwrap();
    o.unknown_relation(Relation::DependsOn, &id("a"), span("a"), "cannot resolve")
        .unwrap();
    let after = p.evaluate(&[o]).unwrap();
    assert_ne!(
        before.facts().subject.snapshot_digest,
        after.facts().subject.snapshot_digest
    );
    assert_eq!(after.report().decision, Decision::Block);
}

#[test]
fn multilingual_module_identity_and_frozen_provider_coverage_remain_distinct() {
    let rust = module("a");
    let java = ModuleKey::new(Language::Java, "a").unwrap();
    let p = ProtectedSystemPolicy::freeze(
        BTreeMap::from([
            (Language::Rust, "fixture/v1".into()),
            (Language::Java, "java-fixture/v1".into()),
        ]),
        BTreeSet::from([rust, java]),
        Relation::DependsOn,
        vec![SystemRule {
            id: "cycle".into(),
            enforcement: Enforcement::Enforce,
            kind: SystemRuleKind::Acyclic,
        }],
    )
    .unwrap();
    let r = observation(&[], true, &["a"]);
    assert_eq!(
        p.evaluate(std::slice::from_ref(&r))
            .unwrap()
            .report()
            .decision,
        Decision::Block
    );
    let mut j = LanguageObservation::new(Language::Java, "java-fixture/v1").unwrap();
    j.add_symbol(Symbol {
        id: SymbolId::new(Language::Java, "a", "", SymbolKind::Module, "a", "").unwrap(),
        source: SourceSpan::new("java/A.java", 1, 1, 1, 2).unwrap(),
    })
    .unwrap();
    j.mark_scope_complete(Relation::DependsOn);
    let one = p.evaluate(&[r.clone(), j.clone()]).unwrap();
    let two = p.evaluate(&[j, r]).unwrap();
    assert_eq!(one.report().decision, Decision::Allow);
    assert_eq!(one.report(), two.report());
}

#[test]
fn protected_layers_must_cover_all_modules_and_rule_ids_are_unique() {
    let freeze = |rules| {
        ProtectedSystemPolicy::freeze(
            BTreeMap::from([(Language::Rust, "fixture/v1".into())]),
            BTreeSet::from([module("a"), module("b")]),
            Relation::DependsOn,
            rules,
        )
    };
    assert!(
        freeze(vec![SystemRule {
            id: "R".into(),
            enforcement: Enforcement::Enforce,
            kind: SystemRuleKind::Layers {
                levels: BTreeMap::from([(module("a"), 0)])
            }
        }])
        .is_err()
    );
    assert!(
        freeze(vec![SystemRule {
            id: "R".into(),
            enforcement: Enforcement::Enforce,
            kind: SystemRuleKind::ForbiddenDirection {
                from: module("a"),
                to: module("c")
            }
        }])
        .is_err()
    );
    let rule = SystemRule {
        id: "R".into(),
        enforcement: Enforcement::Enforce,
        kind: SystemRuleKind::Acyclic,
    };
    assert!(freeze(vec![rule.clone(), rule]).is_err());
}
