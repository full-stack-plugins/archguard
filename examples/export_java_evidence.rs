//! Actual fixed-JDK fixture compiler, provider and native system-rule captures.
use archguard::{
    analysis::{
        java::{JavaProfile, JavaToolchain},
        runner::{self, Budget},
    },
    domain::{
        model::{Language, Relation},
        rules::{ModuleKey, ProtectedSystemPolicy, SystemRule, SystemRuleKind},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::Path,
};
fn write(root: &Path, name: &str, value: &impl serde::Serialize) {
    std::fs::write(root.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn main() {
    let mut args = std::env::args_os().skip(1);
    let home = args.next().expect("JDK_HOME NEW_DIRECTORY");
    let dest = args.next().expect("NEW_DIRECTORY");
    let home = Path::new(&home);
    let dest = Path::new(&dest);
    std::fs::create_dir(dest).expect("new directory only");
    let dest = dest.canonicalize().unwrap();
    let classes = dest.join("classes");
    std::fs::create_dir(&classes).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/languages/java/src");
    let files = [
        "app/Legal.java",
        "app/Forbidden.java",
        "app/Reflective.java",
        "app/NeverRun.java",
        "core/Port.java",
        "infra/Store.java",
    ];
    let mut compiler: Vec<OsString> = vec![
        "-J-Xmx128m".into(),
        "-J-XX:ActiveProcessorCount=2".into(),
        "-proc:none".into(),
        "--release".into(),
        "21".into(),
        "-g".into(),
        "-d".into(),
        classes.clone().into(),
    ];
    compiler.extend(files.map(|p| source.join(p).into_os_string()));
    let output = runner::run(
        &home.join("bin/javac"),
        &compiler,
        &dest,
        &BTreeMap::new(),
        &Budget::default(),
    )
    .unwrap();
    write(
        &dest,
        "fixture-build.json",
        &serde_json::json!({"args":compiler.iter().map(|a|a.to_string_lossy()).collect::<Vec<_>>(),"stdout":String::from_utf8_lossy(&output),"exit":0}),
    );
    let tools = JavaToolchain::freeze(home).unwrap();
    std::fs::write(dest.join("helper-build.json"), tools.build_log()).unwrap();
    for name in ["Legal", "Forbidden", "Reflective", "NeverRun"] {
        let folder = dest.join(name);
        std::fs::create_dir(&folder).unwrap();
        let profile = JavaProfile::freeze(
            BTreeMap::from([
                (format!("app/{name}"), "app".into()),
                ("core/Port".into(), "core".into()),
                ("infra/Store".into(), "infra".into()),
            ]),
            BTreeSet::new(),
        )
        .unwrap();
        let result = tools.analyze(&classes, &profile).unwrap();
        std::fs::create_dir(folder.join("captures")).unwrap();
        std::fs::write(folder.join("captures/java-bytecode.tsv"), result.capture()).unwrap();
        write(&folder, "profile.json", &profile);
        write(
            &folder,
            "observation.json",
            &serde_json::json!({"model_version":result.observation().model_version(),"provider_profile":result.observation().provider_profile(),"source_digest":result.source_digest(),"classfile_digests":result.classfile_digests(),"static_depends_on_complete":result.observation().is_complete(Relation::DependsOn),"calls_complete":result.observation().is_complete(Relation::Calls),"symbols":result.observation().symbols().collect::<Vec<_>>(),"edges":result.observation().edges(),"unknowns":result.observation().unknowns(),"gaps":result.gaps(),"external_references":result.external_references()}),
        );
        let module = |m| ModuleKey::new(Language::Java, m).unwrap();
        let policy = ProtectedSystemPolicy::freeze(
            BTreeMap::from([(Language::Java, tools.provider_id(&profile))]),
            BTreeSet::from([module("app"), module("core"), module("infra")]),
            Relation::DependsOn,
            vec![SystemRule {
                id: "JAVA-DIRECTION".into(),
                enforcement: guardengine::Enforcement::Enforce,
                kind: SystemRuleKind::ForbiddenDirection {
                    from: module("app"),
                    to: module("infra"),
                },
            }],
        )
        .unwrap();
        let evaluation = policy
            .evaluate(std::slice::from_ref(result.observation()))
            .unwrap();
        write(&folder, "contract.json", evaluation.contract());
        write(&folder, "facts.json", evaluation.facts());
        write(&folder, "report.json", evaluation.report());
        write(&folder, "findings.json", &evaluation.findings());
        let native = runner::run(
            &home.join("bin/javap"),
            &[
                "-p".into(),
                "-s".into(),
                "-c".into(),
                "-v".into(),
                classes.join(format!("app/{name}.class")).into(),
            ],
            &dest,
            &BTreeMap::new(),
            &Budget::default(),
        )
        .unwrap();
        std::fs::write(folder.join("javap.txt"), native).unwrap();
    }
    println!("{}", dest.display());
}
