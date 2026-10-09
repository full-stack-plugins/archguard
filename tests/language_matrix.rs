use archguard::domain::model::{
    Language, LanguageObservation, Relation, SourceSpan, Symbol, SymbolId, SymbolKind,
};

fn method(language: Language, signature: &str) -> SymbolId {
    SymbolId::new(
        language,
        "package",
        "service",
        SymbolKind::Method,
        "run",
        signature,
    )
    .unwrap()
}

#[test]
fn symbol_identity_separates_overloads_languages_and_structural_boundaries() {
    let first = method(Language::Java, "(String):void");
    assert_eq!(first, method(Language::Java, "(String):void"));
    assert_ne!(first, method(Language::Java, "(int):void"));
    assert_ne!(first, method(Language::TypeScript, "(String):void"));
    assert_ne!(first, method(Language::Rust, "(String):void"));
    let id = |module, owner| {
        SymbolId::new(Language::Java, module, owner, SymbolKind::Type, "T", "").unwrap()
    };
    assert_ne!(id("a:b", "c"), id("a", "b:c"));
    assert!(SymbolId::new(Language::Java, "p", "T", SymbolKind::Method, "run", "").is_err());
}

#[test]
fn unknown_dispatch_preserves_source_and_prevents_complete_relation_coverage() {
    let caller = method(Language::TypeScript, "():void");
    let callee = method(Language::TypeScript, "(string):void");
    let source = SourceSpan::new("src/service.ts", 10, 2, 10, 14).unwrap();
    let mut observation =
        LanguageObservation::new(Language::TypeScript, "ts-evaluation/5.9.3").unwrap();
    observation
        .add_symbol(Symbol {
            id: caller.clone(),
            source: source.clone(),
        })
        .unwrap();
    observation
        .add_symbol(Symbol {
            id: callee.clone(),
            source: source.clone(),
        })
        .unwrap();
    observation
        .confirm_relation(Relation::Calls, &caller, &callee, source.clone())
        .unwrap();
    observation.mark_scope_complete(Relation::Calls);
    assert!(observation.is_complete(Relation::Calls));
    observation
        .unknown_relation(Relation::Calls, &caller, source.clone(), "dynamic dispatch")
        .unwrap();
    observation.mark_scope_complete(Relation::Calls);
    assert!(!observation.is_complete(Relation::Calls));
    assert!(!observation.is_complete(Relation::DependsOn));
    assert_eq!(observation.edges().len(), 1);
    assert_eq!(observation.edges()[0].source, source);
    assert_eq!(observation.unknowns()[0].source, source);
    assert_eq!(observation.unknowns()[0].reason, "dynamic dispatch");
    assert_eq!(
        observation.model_version(),
        "archguard.language-model/v1alpha1"
    );
}

#[test]
fn malformed_provenance_and_unregistered_or_foreign_symbols_are_rejected() {
    assert!(SourceSpan::new("../outside", 1, 1, 1, 2).is_err());
    assert!(SourceSpan::new("/absolute", 1, 1, 1, 2).is_err());
    assert!(SourceSpan::new("src/a", 0, 1, 1, 2).is_err());
    assert!(SourceSpan::new("src/a", 2, 5, 2, 4).is_err());
    let source = SourceSpan::new("src/a", 1, 1, 1, 2).unwrap();
    let id = method(Language::Rust, "():()");
    let mut observation = LanguageObservation::new(Language::Rust, "fixture-index/v1").unwrap();
    assert!(
        observation
            .confirm_relation(Relation::Calls, &id, &id, source.clone())
            .is_err()
    );
    assert!(
        observation
            .unknown_relation(Relation::Calls, &id, source.clone(), "unknown")
            .is_err()
    );
    assert!(
        observation
            .add_symbol(Symbol {
                id: method(Language::Java, "():void"),
                source: source.clone()
            })
            .is_err()
    );
    observation
        .add_symbol(Symbol {
            id: id.clone(),
            source: source.clone(),
        })
        .unwrap();
    assert!(observation.add_symbol(Symbol { id, source }).is_err());
}

#[test]
fn unresolved_relation_requires_actionable_reason_without_mutating_observation() {
    let id = method(Language::Java, "():void");
    let source = SourceSpan::new("src/Service.java", 1, 1, 1, 3).unwrap();
    let mut observation = LanguageObservation::new(Language::Java, "fixture-bytecode/v1").unwrap();
    observation
        .add_symbol(Symbol {
            id: id.clone(),
            source: source.clone(),
        })
        .unwrap();
    assert!(
        observation
            .unknown_relation(Relation::Calls, &id, source, "  ")
            .is_err()
    );
    assert!(observation.unknowns().is_empty());
    assert!(!observation.is_complete(Relation::Calls));
}
