//! Actual fixed compiler API observations and existing system-rule GE artifacts.
use archguard::{
    analysis::typescript::{TypeScriptProfile, TypeScriptToolchain},
    domain::{
        model::{Language, Relation},
        rules::{ModuleKey, ProtectedSystemPolicy, SystemRule, SystemRuleKind},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn write(root: &Path, name: &str, v: &impl serde::Serialize) {
    std::fs::write(root.join(name), serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
fn main() {
    let mut args = std::env::args_os().skip(1);
    let node = args.next().expect("NODE COMPILER_ROOT NEW_DIRECTORY");
    let compiler = args.next().expect("COMPILER_ROOT");
    let dest = args.next().expect("NEW_DIRECTORY");
    let dest = Path::new(&dest);
    std::fs::create_dir(dest).expect("new destination");
    let tools = TypeScriptToolchain::freeze(Path::new(&node), Path::new(&compiler)).unwrap();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/languages/typescript/provider");
    for (name, entry) in [
        ("Legal", "legal.ts"),
        ("Forbidden", "forbidden.ts"),
        ("Dynamic", "dynamic.ts"),
        ("Missing", "forbidden.ts"),
    ] {
        let folder = dest.join(name);
        std::fs::create_dir(&folder).unwrap();
        let inputs = folder.join("sources");
        std::fs::create_dir_all(inputs.join("core")).unwrap();
        std::fs::create_dir_all(inputs.join("infra")).unwrap();
        for file in [entry, "core/job.ts", "infra/store.ts"] {
            if name != "Missing" || file != "infra/store.ts" {
                std::fs::copy(source.join(file), inputs.join(file)).unwrap();
            }
        }
        let profile = TypeScriptProfile::freeze(
            BTreeMap::from([
                (entry.into(), "app".into()),
                ("core/job.ts".into(), "core".into()),
                ("infra/store.ts".into(), "infra".into()),
            ]),
            BTreeMap::from([
                ("@core/*".into(), vec!["core/*".into()]),
                ("@infra/*".into(), vec!["infra/*".into()]),
            ]),
        )
        .unwrap();
        let result = tools.analyze(&inputs, &profile).unwrap();
        std::fs::write(folder.join("native-capture.json"), result.capture()).unwrap();
        write(&folder, "profile.json", &profile);
        write(
            &folder,
            "observation.json",
            &serde_json::json!({"provider_profile":result.observation().provider_profile(),"source_digest":result.source_digest(),"static_depends_on_complete":result.observation().is_complete(Relation::DependsOn),"calls_complete":result.observation().is_complete(Relation::Calls),"symbols":result.observation().symbols().collect::<Vec<_>>(),"edges":result.observation().edges(),"unknowns":result.observation().unknowns()}),
        );
        let module = |m| ModuleKey::new(Language::TypeScript, m).unwrap();
        let policy = ProtectedSystemPolicy::freeze(
            BTreeMap::from([(Language::TypeScript, tools.provider_id(&profile))]),
            BTreeSet::from([module("app"), module("core"), module("infra")]),
            Relation::DependsOn,
            vec![SystemRule {
                id: "TS-DIRECTION".into(),
                enforcement: guardengine::Enforcement::Enforce,
                kind: SystemRuleKind::ForbiddenDirection {
                    from: module("app"),
                    to: module("infra"),
                },
            }],
        )
        .unwrap();
        let e = policy
            .evaluate(std::slice::from_ref(result.observation()))
            .unwrap();
        write(&folder, "contract.json", e.contract());
        write(&folder, "facts.json", e.facts());
        write(&folder, "report.json", e.report());
        write(&folder, "findings.json", &e.findings());
    }
    println!("{}", dest.display());
}
