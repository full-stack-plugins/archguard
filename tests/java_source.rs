use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn call(root: &Path, args: &[&str], expected: i32) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args(args)
        .arg("--project")
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "pom.xml",
        "<project><modelVersion>4.0.0</modelVersion><groupId>io.demo</groupId><artifactId>app</artifactId><version>1</version></project>",
    );
    write(root,".archguard/contract.yaml", &json!({"apiVersion":"guard.partme.ai/v1alpha2","kind":"GuardContract","metadata":{"id":"layers","revision":"1"},"spec":{"rules":[{"id":"ACYCLIC","enforcement":"enforce","assertion":{"type":"acyclic","predicate":"depends_on"}}]}}).to_string());
    write(root,"model.json",&json!({"schema_version":1,"project":"sales","elements":[{"id":"sales","kind":"context","name":"Sales","responsibility":"Sales"},{"id":"app","kind":"module","name":"App","responsibility":"Sales module","context":"sales","implementation":"io.demo:app"},{"id":"Order","kind":"aggregate","name":"Order","responsibility":"Order lifecycle","context":"sales","module":"app","java_type":"io.demo.domain.Order"}],"relations":[],"java":{"packages":[{"prefix":"io.demo.domain","module":"app","layer":"domain"},{"prefix":"io.demo.infra","module":"app","layer":"infra"}],"layers":[{"id":"domain","allowed_dependencies":[]},{"id":"infra","allowed_dependencies":["domain"]}],"forbidden_dependencies":[{"id":"NO_SQL","from":"io.demo.domain","to":"java.sql"}],"include_tests":false}}).to_string());
    order(root, "package io.demo.domain; public class Order {}");
    write(
        root,
        "src/main/java/io/demo/infra/Repository.java",
        "package io.demo.infra; public class Repository { public static final int VALUE = 1; }",
    );
    for file in [
        "proposal.md",
        "design.md",
        "tasks.md",
        "specs/orders/spec.md",
    ] {
        write(
            root,
            &format!("openspec/changes/order/{file}"),
            "REQ-ORDER Change order implementation.",
        );
    }
    temp
}
fn order(root: &Path, text: &str) {
    write(root, "src/main/java/io/demo/domain/Order.java", text);
}
fn init(root: &Path) -> String {
    call(
        root,
        &["baseline", "init", "--model", "model.json", "--apply"],
        0,
    )["digest"]
        .as_str()
        .unwrap()
        .into()
}
fn delta(root: &Path, base: &str) -> Value {
    let value = json!({"schema_version":1,"id":"REQ-ORDER","base_digest":base,"requirement":{"system":"openspec","target":"openspec/changes/order","artifact":"openspec/changes/order/specs/orders/spec.md","requirement":"REQ-ORDER"},"upsert_elements":[],"remove_elements":[],"upsert_relations":[],"remove_relations":[],"touched_modules":["app"]});
    write(
        root,
        ".archguard/changes/REQ-ORDER.json",
        &value.to_string(),
    );
    value
}
fn checked(root: &Path, code: i32) -> Value {
    call(root, &["change", "check", "--change", "REQ-ORDER"], code)
}

#[test]
fn same_module_java_layer_violation_repair_verify_and_publish() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    order(
        root,
        "package io.demo.domain;\nimport io.demo.infra.Repository;\npublic class Order { Repository repository; }",
    );
    let blocked = checked(root, 2);
    assert!(
        blocked["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_layer_violation" && f["line"] == 2)
    );
    call(root, &["check"], 2);
    order(root, "package io.demo.domain; public class Order {}");
    call(
        root,
        &[
            "change",
            "check",
            "--change",
            "REQ-ORDER",
            "--report",
            "fresh.json",
        ],
        0,
    );
    call(
        root,
        &[
            "change",
            "verify",
            "--change",
            "REQ-ORDER",
            "--report",
            "fresh.json",
        ],
        0,
    );
    let updated = call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-ORDER",
            "--report",
            "fresh.json",
            "--apply",
        ],
        0,
    );
    assert_eq!(updated["revision"], 2);
    call(root, &["check"], 0);
}
#[test]
fn static_wildcard_and_fully_qualified_references_cannot_bypass_layers() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    for source in [
        "package io.demo.domain; import static io.demo.infra.Repository.VALUE; public class Order {}",
        "package io.demo.domain; import io.demo.infra.*; public class Order {}",
        "package io.demo.domain; public class Order { io.demo.infra.Repository repository; }",
        "package io.demo.domain; public class Order { int value = io.demo.infra.Repository.VALUE; }",
    ] {
        order(root, source);
        let report = checked(root, 2);
        assert!(
            report["java_analysis"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f["code"] == "java_layer_violation")
        );
    }
}
#[test]
fn external_forbidden_package_and_missing_domain_type_are_blocked() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    order(
        root,
        "package io.demo.domain; import java.sql.Connection; public class Order {}",
    );
    assert!(
        checked(root, 2)["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_forbidden_dependency" && f["rule"] == "NO_SQL")
    );
    order(root, "package io.demo.domain; class RenamedOrder {}");
    assert!(
        checked(root, 2)["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_missing_domain_type")
    );
}
#[test]
fn comments_strings_and_valid_records_are_parsed_without_false_dependencies() {
    let temp = fixture();
    let root = temp.path();
    order(
        root,
        "package io.demo.domain; /* import io.demo.infra.Repository; */ public record Order(String id) { static String text = \"io.demo.infra.Repository\"; }",
    );
    let base = init(root);
    delta(root, &base);
    let report = checked(root, 0);
    assert_eq!(report["java_analysis"]["edges"], json!([]));
}
#[test]
fn syntax_errors_and_inventory_or_content_drift_reject_old_evidence() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    call(
        root,
        &[
            "change",
            "check",
            "--change",
            "REQ-ORDER",
            "--report",
            "fresh.json",
        ],
        0,
    );
    order(root, "package io.demo.domain; public class Order { }");
    call(
        root,
        &[
            "change",
            "verify",
            "--change",
            "REQ-ORDER",
            "--report",
            "fresh.json",
        ],
        4,
    );
    order(root, "package io.demo.domain; public class Order {}");
    write(
        root,
        "src/main/java/io/demo/domain/Added.java",
        "package io.demo.domain; class Added {}",
    );
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-ORDER",
            "--report",
            "fresh.json",
            "--apply",
        ],
        4,
    );
    order(
        root,
        "package io.demo.domain; class Order { broken syntax !!! }",
    );
    assert_eq!(checked(root, 2)["java_analysis"]["complete"], false);
    assert_eq!(call(root, &["model", "show"], 0)["digest"], base);
}

#[test]
fn policy_and_domain_mapping_deltas_use_existing_baseline_lifecycle() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let mut change = delta(root, &base);
    let model: Value = serde_json::from_slice(&fs::read(root.join("model.json")).unwrap()).unwrap();
    change["java"] = model["java"].clone();
    change["java"]["layers"][0]["allowed_dependencies"] = json!(["infra"]);
    let mut domain = model["elements"][2].clone();
    domain["java_type"] = json!("io.demo.domain.OrderRecord");
    change["upsert_elements"] = json!([domain]);
    write(
        root,
        ".archguard/changes/REQ-ORDER.json",
        &change.to_string(),
    );
    order(
        root,
        "package io.demo.domain; import io.demo.infra.Repository; record OrderRecord(Repository repository) {}",
    );
    call(
        root,
        &[
            "change",
            "check",
            "--change",
            "REQ-ORDER",
            "--report",
            "candidate.json",
        ],
        0,
    );
    call(root, &["check"], 2);
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-ORDER",
            "--report",
            "candidate.json",
            "--apply",
        ],
        0,
    );
    assert_eq!(
        call(root, &["model", "show"], 0)["model"]["java"]["layers"][0]["allowed_dependencies"],
        json!(["infra"])
    );
    call(root, &["check"], 0);
}

#[test]
fn actual_domain_module_and_duplicate_declarations_are_checked() {
    let temp = fixture();
    let root = temp.path();
    let mut model: Value =
        serde_json::from_slice(&fs::read(root.join("model.json")).unwrap()).unwrap();
    model["elements"].as_array_mut().unwrap().push(json!({"id":"other","kind":"module","name":"Other","responsibility":"Other module","context":"sales","implementation":"io.demo:other"}));
    model["elements"][2]["module"] = json!("other");
    model["java"]["packages"][0]["module"] = json!("other");
    write(root, "model.json", &model.to_string());
    write(
        root,
        "pom.xml",
        "<project><modelVersion>4.0.0</modelVersion><groupId>io.demo</groupId><artifactId>app</artifactId><version>1</version><packaging>pom</packaging><modules><module>other</module></modules></project>",
    );
    write(
        root,
        "other/pom.xml",
        "<project><modelVersion>4.0.0</modelVersion><groupId>io.demo</groupId><artifactId>other</artifactId><version>1</version></project>",
    );
    let blocked = call(
        root,
        &["baseline", "init", "--model", "model.json", "--apply"],
        2,
    );
    assert!(
        blocked["check"]["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_domain_owner_mismatch")
    );
    assert!(!root.join(".archguard/baseline.json").exists());
    write(
        root,
        "other/src/main/java/io/demo/domain/Order.java",
        "package io.demo.domain; public class Order {}",
    );
    assert_eq!(
        call(root, &["baseline", "init", "--model", "model.json"], 2)["check"]["java_analysis"]["complete"],
        false
    );
}

#[test]
fn java_policy_rejects_unknown_layers_and_unmapped_source_packages() {
    let temp = fixture();
    let root = temp.path();
    let mut model: Value =
        serde_json::from_slice(&fs::read(root.join("model.json")).unwrap()).unwrap();
    model["java"]["layers"][0]["allowed_dependencies"] = json!(["typo"]);
    write(root, "bad-model.json", &model.to_string());
    call(root, &["model", "validate", "--model", "bad-model.json"], 4);
    let base = init(root);
    delta(root, &base);
    write(
        root,
        "src/main/java/io/demo/domainx/Leak.java",
        "package io.demo.domainx; class Leak {}",
    );
    assert!(
        checked(root, 2)["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_unmapped_package" && f["package"] == "io.demo.domainx")
    );
}

#[test]
fn fully_qualified_external_forbidden_reference_is_not_an_import_bypass() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    order(
        root,
        "package io.demo.domain; public class Order { java.sql.Connection connection; }",
    );
    assert!(
        checked(root, 2)["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_forbidden_dependency" && f["rule"] == "NO_SQL")
    );
}

#[test]
fn package_annotations_are_real_dependencies() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    write(
        root,
        "src/main/java/io/demo/infra/Boundary.java",
        "package io.demo.infra; public @interface Boundary {}",
    );
    write(
        root,
        "src/main/java/io/demo/domain/package-info.java",
        "@io.demo.infra.Boundary package io.demo.domain;",
    );
    assert!(
        checked(root, 2)["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["code"] == "java_layer_violation"
                && f["file"] == "src/main/java/io/demo/domain/package-info.java")
    );
}

#[test]
fn escaped_windows_path_is_a_literal_but_real_unicode_escape_is_incomplete() {
    let temp = fixture();
    let root = temp.path();
    order(
        root,
        r#"package io.demo.domain; public class Order { String path = "C:\\users\\data"; }"#,
    );
    let base = init(root);
    delta(root, &base);
    checked(root, 0);
    order(
        root,
        r#"package io.demo.domain; public class Order { String name = "\u0061"; }"#,
    );
    assert_eq!(checked(root, 2)["java_analysis"]["complete"], false);
}

#[test]
fn configured_test_scope_and_custom_source_roots_fail_truthfully() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let mut change = delta(root, &base);
    write(
        root,
        "src/test/java/io/demo/domain/TestOrder.java",
        "package io.demo.domain; import io.demo.infra.Repository; class TestOrder {}",
    );
    assert_eq!(checked(root, 0)["java_analysis"]["include_tests"], false);
    let model: Value = serde_json::from_slice(&fs::read(root.join("model.json")).unwrap()).unwrap();
    change["java"] = model["java"].clone();
    change["java"]["include_tests"] = json!(true);
    write(
        root,
        ".archguard/changes/REQ-ORDER.json",
        &change.to_string(),
    );
    assert!(
        checked(root, 2)["java_analysis"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["file"] == "src/test/java/io/demo/domain/TestOrder.java")
    );
    let pom = fs::read_to_string(root.join("pom.xml")).unwrap().replace(
        "</project>",
        "<build><sourceDirectory>custom</sourceDirectory></build></project>",
    );
    write(root, "pom.xml", &pom);
    assert_eq!(checked(root, 2)["java_analysis"]["complete"], false);
}

#[test]
fn java_source_evidence_also_invalidates_ordinary_reports() {
    let temp = fixture();
    let root = temp.path();
    init(root);
    let report = root.join("ordinary.json");
    call(root, &["check", "--report", report.to_str().unwrap()], 0);
    order(
        root,
        "package io.demo.domain; public class Order {} // source-only drift",
    );
    call(root, &["verify", "--report", report.to_str().unwrap()], 4);
}

#[test]
fn actual_readonly_mcp_returns_same_java_block_as_cli() {
    use std::{io::Write, process::Stdio};
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    order(
        root,
        "package io.demo.domain; import io.demo.infra.Repository; public class Order {}",
    );
    let expected = checked(root, 2);
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"java-test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"archguard_change_check","arguments":{"project":root,"change":"REQ-ORDER"}}}),
    ];
    let mut process = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args(["serve", "--mcp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    process
        .stdin
        .take()
        .unwrap()
        .write_all(
            (messages
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n")
                .as_bytes(),
        )
        .unwrap();
    let output = process.wait_with_output().unwrap();
    assert!(output.status.success());
    let replies: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(replies[1]["result"]["isError"], false);
    let report: Value =
        serde_json::from_str(replies[1]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(report, expected);
    assert_eq!(call(root, &["model", "show"], 0)["digest"], base);
}

#[test]
fn deeply_nested_reference_is_bounded_before_extracting_names() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    order(
        root,
        &format!(
            "package io.demo.domain; class Order {{ Object value = io.demo.infra.Repository{}; }}",
            ".next".repeat(500)
        ),
    );
    assert_eq!(checked(root, 2)["java_analysis"]["complete"], false);
}
