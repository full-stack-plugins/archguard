mod common;
use archguard::{
    analysis::java::{JavaProfile, JavaToolchain},
    domain::model::{Relation, SymbolKind},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
    sync::OnceLock,
};
const JDK: &str = "/workspace/guard-toolchain/jdk21/usr/lib/jvm/java-21-openjdk-amd64";
fn toolchain() -> &'static JavaToolchain {
    static TOOLS: OnceLock<JavaToolchain> = OnceLock::new();
    TOOLS.get_or_init(|| JavaToolchain::freeze(Path::new(JDK)).unwrap())
}
fn fixture() -> common::Temp {
    let temp = common::Temp::new();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/languages/java/src");
    let files = [
        "app/Legal.java",
        "app/Forbidden.java",
        "app/Reflective.java",
        "core/Port.java",
        "infra/Store.java",
        "app/NeverRun.java",
    ];
    let output = Command::new(Path::new(JDK).join("bin/javac"))
        .args([
            "-J-Xmx128m",
            "-J-XX:ActiveProcessorCount=2",
            "-proc:none",
            "--release",
            "21",
            "-g",
            "-d",
        ])
        .arg(&temp.0)
        .args(files.map(|p| source.join(p)))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    temp
}
fn profile(name: &str) -> JavaProfile {
    JavaProfile::freeze(
        BTreeMap::from([
            (format!("app/{name}"), "app".into()),
            ("core/Port".into(), "core".into()),
            ("infra/Store".into(), "infra".into()),
        ]),
        BTreeSet::new(),
    )
    .unwrap()
}
#[test]
fn real_jdk_preserves_overloads_and_static_type_provenance() {
    let input = fixture();
    let result = toolchain().analyze(&input.0, &profile("Legal")).unwrap();
    assert!(result.observation().is_complete(Relation::DependsOn));
    assert!(!result.observation().is_complete(Relation::Calls));
    assert!(
        result
            .observation()
            .edges()
            .iter()
            .any(|e| e.from.module() == "app" && e.to.module() == "core")
    );
    assert!(
        result
            .observation()
            .symbols()
            .filter(|s| s.id.kind() == SymbolKind::Method)
            .count()
            >= 6
    );
    assert!(
        result
            .observation()
            .edges()
            .iter()
            .all(|e| e.source.path() == "captures/java-bytecode.tsv")
    );
    assert!(!result.capture().is_empty());
    let methods = result
        .observation()
        .symbols()
        .map(|s| serde_json::to_value(&s.id).unwrap())
        .filter(|v| v["owner"] == "app/Legal" && v["name"] == "run")
        .map(|v| v["signature"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        methods,
        BTreeSet::from(["(I)V".into(), "(Ljava/lang/String;)V".into()])
    );
    for edge in result.observation().edges() {
        assert!(
            std::str::from_utf8(result.capture())
                .unwrap()
                .lines()
                .nth(edge.source.start().0 as usize - 1)
                .unwrap()
                .starts_with("D\t")
        );
    }
    use sha2::{Digest, Sha256};
    assert_eq!(
        result.classfile_digests()["app/Legal"],
        format!(
            "sha256:{:x}",
            Sha256::digest(std::fs::read(input.0.join("app/Legal.class")).unwrap())
        )
    );
}

fn evaluate(
    result: &archguard::analysis::java::JavaAnalysis,
) -> archguard::domain::rules::SystemEvaluation {
    use archguard::domain::{model::Language, rules::*};
    let module = |m| ModuleKey::new(Language::Java, m).unwrap();
    let policy = ProtectedSystemPolicy::freeze(
        BTreeMap::from([(
            Language::Java,
            result.observation().provider_profile().to_owned(),
        )]),
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
    policy
        .evaluate(std::slice::from_ref(result.observation()))
        .unwrap()
}
#[test]
fn actual_java_observations_drive_legal_forbidden_and_missing_rules() {
    let input = fixture();
    let legal = toolchain().analyze(&input.0, &profile("Legal")).unwrap();
    assert_eq!(
        evaluate(&legal).report().decision,
        guardengine::Decision::Allow
    );
    let forbidden = toolchain()
        .analyze(&input.0, &profile("Forbidden"))
        .unwrap();
    let evaluation = evaluate(&forbidden);
    assert_eq!(evaluation.report().decision, guardengine::Decision::Block);
    assert!(
        evaluation
            .findings()
            .iter()
            .any(|f| !f.path.is_empty() && f.path[0].source.path() == "captures/java-bytecode.tsv")
    );
    std::fs::remove_file(input.0.join("infra/Store.class")).unwrap();
    let missing = toolchain()
        .analyze(&input.0, &profile("Forbidden"))
        .unwrap();
    assert!(!missing.observation().is_complete(Relation::DependsOn));
    assert!(missing.gaps().iter().any(|g| g.class_name == "infra/Store"));
    assert_eq!(
        evaluate(&missing).facts().completeness,
        guardengine::Completeness::Partial
    );
    assert_eq!(
        evaluate(&missing).report().decision,
        guardengine::Decision::Block
    );
}
#[test]
fn reflection_dynamic_dispatch_and_version_gaps_never_claim_runtime_coverage() {
    let input = fixture();
    let reflected = toolchain()
        .analyze(&input.0, &profile("Reflective"))
        .unwrap();
    assert!(!reflected.observation().is_complete(Relation::DependsOn));
    assert!(
        reflected
            .observation()
            .unknowns()
            .iter()
            .any(|u| u.reason.contains("reflective"))
    );
    assert!(
        reflected
            .observation()
            .unknowns()
            .iter()
            .any(|u| u.reason.contains("invokedynamic"))
    );
    assert_eq!(
        evaluate(&reflected).report().decision,
        guardengine::Decision::Block
    );
    let file = input.0.join("core/Port.class");
    let original = std::fs::read(&file).unwrap();
    for version in [[0, 0, 0, 66], [255, 255, 0, 65]] {
        let mut raw = original.clone();
        raw[4..8].copy_from_slice(&version);
        std::fs::write(&file, raw).unwrap();
        let unknown = toolchain().analyze(&input.0, &profile("Legal")).unwrap();
        assert!(unknown.gaps().iter().any(|g| g.reason.contains("version")));
        assert!(!unknown.observation().is_complete(Relation::DependsOn));
    }
}
#[test]
fn malformed_bytecode_and_input_budget_are_execution_failures_not_empty_graphs() {
    let input = fixture();
    let path = input.0.join("app/Legal.class");
    std::fs::write(&path, [0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 65]).unwrap();
    assert!(matches!(
        toolchain().analyze(&input.0, &profile("Legal")),
        Err(archguard::analysis::java::JavaFailure::Tool(_))
    ));
    std::fs::write(&path, vec![0; 512 * 1024 + 1]).unwrap();
    assert!(toolchain().analyze(&input.0, &profile("Legal")).is_err());
}
#[test]
fn protected_scope_external_references_and_input_identity_are_preserved() {
    assert!(JavaProfile::freeze(BTreeMap::new(), BTreeSet::new()).is_err());
    assert!(
        JavaProfile::freeze(
            BTreeMap::from([("../escape".into(), "m".into())]),
            BTreeSet::new()
        )
        .is_err()
    );
    assert!(
        JavaProfile::freeze(
            (0..65).map(|i| (format!("A{i}"), "m".into())).collect(),
            BTreeSet::new()
        )
        .is_err()
    );
    let input = fixture();
    let narrow = JavaProfile::freeze(
        BTreeMap::from([("app/Legal".into(), "app".into())]),
        BTreeSet::new(),
    )
    .unwrap();
    let missing = toolchain().analyze(&input.0, &narrow).unwrap();
    assert!(!missing.observation().is_complete(Relation::DependsOn));
    assert!(
        missing
            .observation()
            .unknowns()
            .iter()
            .any(|u| u.reason.contains("core/Port"))
    );
    let external = JavaProfile::freeze(
        BTreeMap::from([("app/Legal".into(), "app".into())]),
        BTreeSet::from(["core/Port".into()]),
    )
    .unwrap();
    let scoped = toolchain().analyze(&input.0, &external).unwrap();
    assert!(scoped.observation().is_complete(Relation::DependsOn));
    assert!(scoped.external_references().contains("core/Port"));
    assert_ne!(missing.source_digest(), scoped.source_digest());
    assert_ne!(
        toolchain().provider_id(&narrow),
        toolchain().provider_id(&external)
    );
    let path = input.0.join("app/Legal.class");
    let mut raw = std::fs::read(&path).unwrap();
    let at = raw.windows(10).position(|w| w == b"Legal.java").unwrap();
    raw[at + 4] = b'X';
    std::fs::write(path, raw).unwrap();
    assert_ne!(
        scoped.source_digest(),
        toolchain()
            .analyze(&input.0, &external)
            .unwrap()
            .source_digest()
    );
}
#[test]
fn candidate_static_initializer_is_never_executed_and_tool_build_is_pinned() {
    let input = fixture();
    let result = toolchain().analyze(&input.0, &profile("NeverRun")).unwrap();
    assert!(result.observation().is_complete(Relation::DependsOn));
    let log: serde_json::Value = serde_json::from_slice(toolchain().build_log()).unwrap();
    assert_eq!(log["exit"], 0);
    assert_eq!(log["javac_version_stdout"], "javac 21.0.12.1\n");
    assert!(
        log["args"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("-proc:none"))
    );
    let fake = common::Temp::new();
    std::fs::write(fake.0.join("release"), "different JDK").unwrap();
    assert!(JavaToolchain::freeze(&fake.0).is_err());
}

#[test]
fn trailing_bytes_are_not_accepted_as_a_complete_classfile() {
    let input = fixture();
    let path = input.0.join("app/Legal.class");
    let mut raw = std::fs::read(&path).unwrap();
    raw.extend_from_slice(b"opaque trailing payload");
    std::fs::write(path, raw).unwrap();
    assert!(toolchain().analyze(&input.0, &profile("Legal")).is_err());
}

#[test]
fn classfile_binary_name_must_match_its_frozen_input_path() {
    let input = fixture();
    let a = input.0.join("core/Port.class");
    let b = input.0.join("infra/Store.class");
    let first = std::fs::read(&a).unwrap();
    let second = std::fs::read(&b).unwrap();
    std::fs::write(&a, second).unwrap();
    std::fs::write(&b, first).unwrap();
    assert!(toolchain().analyze(&input.0, &profile("Legal")).is_err());
}

#[test]
fn java_package_named_target_is_not_silently_omitted_by_cargo_copy_rules() {
    let temp = common::Temp::new();
    let src = temp.0.join("Plain.java");
    std::fs::write(&src, "package target; public class Plain { }").unwrap();
    let status = Command::new(Path::new(JDK).join("bin/javac"))
        .args(["-J-Xmx128m", "-proc:none", "--release", "21", "-d"])
        .arg(&temp.0)
        .arg(&src)
        .status()
        .unwrap();
    assert!(status.success());
    let profile = JavaProfile::freeze(
        BTreeMap::from([("target/Plain".into(), "core".into())]),
        BTreeSet::new(),
    )
    .unwrap();
    let result = toolchain().analyze(&temp.0, &profile).unwrap();
    assert!(result.gaps().is_empty());
    assert!(result.observation().is_complete(Relation::DependsOn));
}

#[test]
fn native_structure_budget_is_checked_before_emitting_a_large_observation() {
    let temp = common::Temp::new();
    let source = temp.0.join("Many.java");
    let mut body = "package app; public class Many {".to_owned();
    for n in 0..257 {
        body.push_str(&format!("public int m{n}() {{ return {n}; }}"));
    }
    body.push('}');
    std::fs::write(&source, body).unwrap();
    let output = Command::new(Path::new(JDK).join("bin/javac"))
        .args(["-J-Xmx128m", "-proc:none", "--release", "21", "-d"])
        .arg(&temp.0)
        .arg(&source)
        .output()
        .unwrap();
    assert!(output.status.success());
    let profile = JavaProfile::freeze(
        BTreeMap::from([("app/Many".into(), "app".into())]),
        BTreeSet::new(),
    )
    .unwrap();
    assert!(matches!(
        toolchain().analyze(&temp.0, &profile),
        Err(archguard::analysis::java::JavaFailure::Tool(_))
    ));
}

#[test]
fn malformed_utf16_method_name_is_not_lossily_relabelled() {
    let input = fixture();
    let path = input.0.join("app/Legal.class");
    let mut raw = std::fs::read(&path).unwrap();
    let at = raw
        .windows(6)
        .position(|w| w == [1, 0, 3, b'r', b'u', b'n'])
        .unwrap();
    raw[at + 3..at + 6].copy_from_slice(&[0xed, 0xa0, 0x80]);
    std::fs::write(path, raw).unwrap();
    assert!(toolchain().analyze(&input.0, &profile("Legal")).is_err());
}
