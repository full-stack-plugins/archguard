mod common;
use archguard::{
    analysis::typescript::{TypeScriptProfile, TypeScriptToolchain},
    domain::model::{Relation, SymbolKind},
};
use std::{collections::BTreeMap, path::Path, sync::OnceLock};
const NODE: &str = "/opt/codex/runtimes/codex-primary-runtime/dependencies/node/bin/node";
fn tool() -> &'static TypeScriptToolchain {
    static TOOL: OnceLock<TypeScriptToolchain> = OnceLock::new();
    TOOL.get_or_init(|| {
        TypeScriptToolchain::freeze(
            Path::new(NODE),
            Path::new("/workspace/guard-implementation/archguard/fixtures/languages/typescript/node_modules/typescript"),
        )
        .unwrap()
    })
}
fn profile() -> TypeScriptProfile {
    TypeScriptProfile::freeze(
        BTreeMap::from([
            ("app/main.ts".into(), "app".into()),
            ("core/job.ts".into(), "core".into()),
        ]),
        BTreeMap::from([("@core/*".into(), vec!["core/*".into()])]),
    )
    .unwrap()
}
fn fixture() -> common::Temp {
    let t = common::Temp::new();
    std::fs::create_dir_all(t.0.join("app")).unwrap();
    std::fs::create_dir_all(t.0.join("core")).unwrap();
    std::fs::write(t.0.join("app/main.ts"),"import { Job as Alias } from '@core/job'; export class App { run(x: Alias): Alias { return x; } }").unwrap();
    std::fs::write(t.0.join("core/job.ts"),"export class Job { run(x: string): string; run(x: number): number; run(x: string | number): string | number { return x; } }").unwrap();
    t
}
#[test]
fn actual_compiler_resolves_alias_cross_package_overloads_and_sources() {
    let t = fixture();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(a.observation().is_complete(Relation::DependsOn));
    assert!(!a.observation().is_complete(Relation::Calls));
    assert!(
        a.observation()
            .edges()
            .iter()
            .any(|e| e.from.module() == "app"
                && e.to.module() == "core"
                && e.source.path() == "app/main.ts")
    );
    assert_eq!(
        a.observation()
            .symbols()
            .filter(|s| s.id.kind() == SymbolKind::Method)
            .count(),
        4
    );
    assert!(!a.capture().is_empty());
    assert!(a.source_digest().starts_with("sha256:"));
}
fn evaluate(
    a: &archguard::analysis::typescript::TypeScriptAnalysis,
    deny: &str,
) -> archguard::domain::rules::SystemEvaluation {
    use archguard::domain::{
        model::Language,
        rules::{ModuleKey, ProtectedSystemPolicy, SystemRule, SystemRuleKind},
    };
    let module = |name: &str| ModuleKey::new(Language::TypeScript, name).unwrap();
    ProtectedSystemPolicy::freeze(
        BTreeMap::from([(
            Language::TypeScript,
            a.observation().provider_profile().into(),
        )]),
        std::collections::BTreeSet::from([module("app"), module("core")]),
        Relation::DependsOn,
        vec![SystemRule {
            id: "TS-DIRECTION".into(),
            enforcement: guardengine::Enforcement::Enforce,
            kind: SystemRuleKind::ForbiddenDirection {
                from: module(if deny == "reverse" { "core" } else { "app" }),
                to: module(if deny == "reverse" { "app" } else { deny }),
            },
        }],
    )
    .unwrap()
    .evaluate(std::slice::from_ref(a.observation()))
    .unwrap()
}
#[test]
fn actual_type_edges_drive_forbidden_rule_and_missing_is_partial() {
    let t = fixture();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    let no_violation = evaluate(&a, "reverse");
    assert_eq!(no_violation.report().decision, guardengine::Decision::Allow);
    let blocked = evaluate(&a, "core");
    assert_eq!(blocked.report().decision, guardengine::Decision::Block);
    assert!(
        blocked
            .findings()
            .iter()
            .any(|f| !f.path.is_empty() && f.path[0].source.path() == "app/main.ts")
    );
    std::fs::remove_file(t.0.join("core/job.ts")).unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(!a.observation().is_complete(Relation::DependsOn));
    assert_eq!(
        evaluate(&a, "core").report().decision,
        guardengine::Decision::Block
    );
    assert!(a.observation().edges().is_empty());
}
#[test]
fn literal_dynamic_import_is_resolved_but_nonliteral_stays_unknown() {
    let t = fixture();
    std::fs::write(
        t.0.join("app/main.ts"),
        "export const load = () => import('@core/job');",
    )
    .unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(a.observation().is_complete(Relation::DependsOn));
    assert!(
        a.observation()
            .edges()
            .iter()
            .any(|e| e.to.module() == "core")
    );
    std::fs::write(
        t.0.join("app/main.ts"),
        "export const load = (p:string) => import(p);",
    )
    .unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(!a.observation().is_complete(Relation::DependsOn));
    assert!(
        a.observation()
            .unknowns()
            .iter()
            .any(|u| u.reason.contains("nonliteral"))
    );
}
#[test]
fn references_unknown_options_and_candidate_configs_are_rejected() {
    assert!(
        TypeScriptProfile::from_json(
            br#"{"modules":{"app.ts":"app"},"paths":{},"references":[{"path":"../foreign"}]}"#
        )
        .is_err()
    );
    assert!(TypeScriptProfile::from_json(br#"{"modules":{"app.ts":"app"},"paths":{},"compilerOptions":{"plugins":[{"name":"evil"}]}}"#).is_err());
    assert!(
        TypeScriptProfile::freeze(
            BTreeMap::from([("../escape.ts".into(), "app".into())]),
            BTreeMap::new()
        )
        .is_err()
    );
    assert!(
        TypeScriptProfile::freeze(
            BTreeMap::from([("app.ts".into(), "app".into())]),
            BTreeMap::from([("@outside/*".into(), vec!["../outside/*".into()])])
        )
        .is_err()
    );
    let t = fixture();
    std::fs::write(
        t.0.join("tsconfig.json"),
        "{\"references\":[{\"path\":\"../other\"}]}",
    )
    .unwrap();
    assert!(tool().analyze(&t.0, &profile()).is_err());
}
#[test]
fn compiler_diagnostics_and_recursive_aliases_cannot_claim_complete() {
    let t = fixture();
    std::fs::write(t.0.join("core/job.ts"), "export type Job = Job;").unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(!a.observation().is_complete(Relation::DependsOn));
    std::fs::write(t.0.join("core/job.ts"), "export class Job {").unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(!a.observation().is_complete(Relation::DependsOn));
}
#[test]
fn frozen_identity_changes_with_bytes_scope_and_alias_configuration() {
    let t = fixture();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    let b = tool().analyze(&t.0, &profile()).unwrap();
    assert_eq!(a.source_digest(), b.source_digest());
    assert_eq!(a.capture(), b.capture());
    std::fs::write(t.0.join("unused.ts"), "throw Error('not executed')").unwrap();
    assert_eq!(
        a.source_digest(),
        tool().analyze(&t.0, &profile()).unwrap().source_digest()
    );
    std::fs::write(
        t.0.join("app/main.ts"),
        "throw Error('candidate never executed'); export class Different {}",
    )
    .unwrap();
    assert_ne!(
        a.source_digest(),
        tool().analyze(&t.0, &profile()).unwrap().source_digest()
    );
    let wrong = TypeScriptProfile::freeze(
        BTreeMap::from([
            ("app/main.ts".into(), "app".into()),
            ("core/job.ts".into(), "core".into()),
        ]),
        BTreeMap::new(),
    )
    .unwrap();
    assert_ne!(tool().provider_id(&profile()), tool().provider_id(&wrong));
}
#[test]
fn input_and_preprojection_node_budgets_fail_as_errors() {
    let t = fixture();
    std::fs::write(t.0.join("app/main.ts"), " ".repeat(512 * 1024 + 1)).unwrap();
    assert!(tool().analyze(&t.0, &profile()).is_err());
    std::fs::write(
        t.0.join("app/main.ts"),
        (0..5000)
            .map(|i| format!("export class C{i} {{}}\n"))
            .collect::<String>(),
    )
    .unwrap();
    assert!(tool().analyze(&t.0, &profile()).is_err());
}
#[test]
fn nested_same_named_types_are_not_silently_conflated() {
    let t = fixture();
    std::fs::write(t.0.join("app/main.ts"),"export function a() { class Same { run(): void {} } return Same; } export function b() { class Same { run(): void {} } return Same; }").unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(!a.observation().is_complete(Relation::DependsOn));
}
#[test]
fn unresolved_alias_does_not_fall_back_to_unprotected_disk_source() {
    let t = fixture();
    let p = TypeScriptProfile::freeze(
        BTreeMap::from([("app/main.ts".into(), "app".into())]),
        BTreeMap::from([("@core/*".into(), vec!["core/*".into()])]),
    )
    .unwrap();
    let a = tool().analyze(&t.0, &p).unwrap();
    assert!(!a.observation().is_complete(Relation::DependsOn));
    assert!(a.observation().edges().is_empty());
    let no_alias = TypeScriptProfile::freeze(
        BTreeMap::from([
            ("app/main.ts".into(), "app".into()),
            ("core/job.ts".into(), "core".into()),
        ]),
        BTreeMap::new(),
    )
    .unwrap();
    assert!(
        !tool()
            .analyze(&t.0, &no_alias)
            .unwrap()
            .observation()
            .is_complete(Relation::DependsOn)
    );
}
#[test]
fn unpinned_tools_and_source_symlinks_are_rejected() {
    let t = fixture();
    std::fs::write(t.0.join("fake-node"), "#!/bin/sh\necho v24.19.0\n").unwrap();
    assert!(TypeScriptToolchain::freeze(&t.0.join("fake-node"), &t.0).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", t.0.join("escape.ts")).unwrap();
        assert!(tool().analyze(&t.0, &profile()).is_err());
    }
}
#[test]
fn global_constructor_and_type_query_keep_actual_cross_file_type_origins() {
    let t = fixture();
    std::fs::write(
        t.0.join("core/job.ts"),
        "declare class Job { run(): void; }",
    )
    .unwrap();
    std::fs::write(
        t.0.join("app/main.ts"),
        "const value = new Job(); type Constructor = typeof Job;",
    )
    .unwrap();
    let a = tool().analyze(&t.0, &profile()).unwrap();
    assert!(
        a.observation()
            .edges()
            .iter()
            .any(|e| e.from.module() == "app" && e.to.module() == "core")
    );
}
#[test]
fn prepared_toolchain_does_not_execute_a_replaced_node_path() {
    let t = fixture();
    let copied = t.0.join("node");
    std::fs::copy(NODE, &copied).unwrap();
    let compiler = Path::new(
        "/workspace/guard-implementation/archguard/fixtures/languages/typescript/node_modules/typescript",
    );
    let tools = TypeScriptToolchain::freeze(&copied, compiler).unwrap();
    std::fs::write(&copied, "#!/bin/sh\nexit 73\n").unwrap();
    std::fs::remove_file(&copied).unwrap();
    assert!(tools.analyze(&t.0, &profile()).is_ok());
}

#[test]
fn independent_ambient_value_dependency_is_not_zero_edge_complete() {
    let t = common::Temp::new();
    std::fs::write(
        t.0.join("ambient.d.ts"),
        "declare const ExternalValue: {id: string};",
    )
    .unwrap();
    std::fs::write(
        t.0.join("main.ts"),
        "export const value = ExternalValue.id;",
    )
    .unwrap();
    let p = TypeScriptProfile::freeze(
        BTreeMap::from([
            ("ambient.d.ts".into(), "core".into()),
            ("main.ts".into(), "app".into()),
        ]),
        BTreeMap::new(),
    )
    .unwrap();
    let a = tool().analyze(&t.0, &p).unwrap();
    assert!(
        !a.observation().is_complete(Relation::DependsOn)
            || a.observation()
                .edges()
                .iter()
                .any(|e| e.from.module() == "app" && e.to.module() == "core"),
        "ambient value access lacks a dependency yet claims complete: {}",
        String::from_utf8_lossy(a.capture())
    );
}
#[test]
fn independent_ambient_value_access_must_not_pass_real_forbidden_direction() {
    let t = common::Temp::new();
    std::fs::write(
        t.0.join("ambient.d.ts"),
        "declare const ExternalValue: {id: string};",
    )
    .unwrap();
    std::fs::write(
        t.0.join("main.ts"),
        "export const value = ExternalValue.id;",
    )
    .unwrap();
    let p = TypeScriptProfile::freeze(
        BTreeMap::from([
            ("ambient.d.ts".into(), "core".into()),
            ("main.ts".into(), "app".into()),
        ]),
        BTreeMap::new(),
    )
    .unwrap();
    let a = tool().analyze(&t.0, &p).unwrap();
    let result = evaluate(&a, "core");
    assert_ne!(
        result.report().decision,
        guardengine::Decision::Allow,
        "real protected app->core rule incorrectly allows ambient source dependency"
    );
}
#[test]
fn ambient_values_functions_enum_and_shorthand_preserve_semantic_origins() {
    for source in [
        "export const value = ExternalValue.id;",
        "export const value = externalFunction();",
        "export const value = {ExternalValue};",
        "export const value = ExternalEnum.Member;",
        "const localAlias = ExternalValue; export const value = localAlias.id;",
    ] {
        let t = common::Temp::new();
        std::fs::write(t.0.join("ambient.d.ts"),"declare const ExternalValue: {id:string}; declare function externalFunction(): string; declare enum ExternalEnum { Member }").unwrap();
        std::fs::write(t.0.join("main.ts"), source).unwrap();
        let p = TypeScriptProfile::freeze(
            BTreeMap::from([
                ("ambient.d.ts".into(), "core".into()),
                ("main.ts".into(), "app".into()),
            ]),
            BTreeMap::new(),
        )
        .unwrap();
        let a = tool().analyze(&t.0, &p).unwrap();
        assert!(
            a.observation()
                .edges()
                .iter()
                .any(|e| e.from.module() == "app"
                    && e.to.module() == "core"
                    && e.source.path() == "main.ts"),
            "{source}: {}",
            String::from_utf8_lossy(a.capture())
        );
        assert_eq!(
            evaluate(&a, "core").report().decision,
            guardengine::Decision::Block
        );
        assert!(!a.observation().is_complete(Relation::Calls));
    }
}
#[test]
fn ambient_dependency_does_not_clear_dynamic_import_unknown() {
    let t = common::Temp::new();
    std::fs::write(
        t.0.join("ambient.d.ts"),
        "declare const ExternalValue: {id:string};",
    )
    .unwrap();
    std::fs::write(
        t.0.join("main.ts"),
        "export const value = ExternalValue.id; export const load = (path:string) => import(path);",
    )
    .unwrap();
    let p = TypeScriptProfile::freeze(
        BTreeMap::from([
            ("ambient.d.ts".into(), "core".into()),
            ("main.ts".into(), "app".into()),
        ]),
        BTreeMap::new(),
    )
    .unwrap();
    let a = tool().analyze(&t.0, &p).unwrap();
    assert!(
        a.observation()
            .edges()
            .iter()
            .any(|e| e.from.module() == "app" && e.to.module() == "core")
    );
    assert!(
        a.observation()
            .unknowns()
            .iter()
            .any(|u| u.reason.contains("nonliteral"))
    );
    assert!(!a.observation().is_complete(Relation::DependsOn));
    assert!(!a.observation().is_complete(Relation::Calls));
}
