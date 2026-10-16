use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn write(root: &Path, path: &str, value: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, value).unwrap();
}
fn call(root: &Path, args: &[&str], code: i32) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args(args)
        .arg("--project")
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(code),
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn pom(name: &str, dependencies: &str) -> String {
    format!(
        "<project><modelVersion>4.0.0</modelVersion><groupId>io.demo</groupId><artifactId>{name}</artifactId><version>1</version>{dependencies}</project>"
    )
}
fn dependency(name: &str) -> String {
    format!(
        "<dependencies><dependency><groupId>io.demo</groupId><artifactId>{name}</artifactId><version>1</version></dependency></dependencies>"
    )
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "pom.xml",
        &pom(
            "root",
            "<packaging>pom</packaging><modules><module>app</module><module>domain</module></modules>",
        ),
    );
    write(root, "app/pom.xml", &pom("app", &dependency("domain")));
    write(root, "domain/pom.xml", &pom("domain", ""));
    write(root,".archguard/contract.yaml",&json!({"apiVersion":"guard.partme.ai/v1alpha2","kind":"GuardContract","metadata":{"id":"layers","revision":"1"},"spec":{"rules":[{"id":"ACYCLIC","enforcement":"enforce","assertion":{"type":"acyclic","predicate":"depends_on"}}]}}).to_string());
    let mut elements =
        vec![json!({"id":"sales","kind":"context","name":"Sales","responsibility":"Sales domain"})];
    for name in ["root", "app", "domain"] {
        elements.push(json!({"id":name,"kind":"module","name":name,"responsibility":"Explicit module responsibility","context":"sales","implementation":format!("io.demo:{name}")}));
    }
    elements.push(json!({"id":"Order","kind":"aggregate","name":"Order","responsibility":"Own order lifecycle","context":"sales","module":"domain"}));
    write(root,"model.json",&json!({"schema_version":1,"project":"sales-project","elements":elements,"relations":[{"id":"app-domain","from":"app","to":"domain","kind":"depends_on"}]}).to_string());
    for file in [
        "proposal.md",
        "design.md",
        "tasks.md",
        "specs/orders/spec.md",
    ] {
        write(
            root,
            &format!("openspec/changes/points/{file}"),
            "REQ-POINTS Add loyalty points to orders.",
        );
    }
    temp
}
fn init(root: &Path) -> String {
    let model = root.join("model.json");
    let preview = call(
        root,
        &["baseline", "init", "--model", model.to_str().unwrap()],
        0,
    );
    assert_eq!(preview["applied"], false);
    assert!(!root.join(".archguard/baseline.json").exists());
    let result = call(
        root,
        &[
            "baseline",
            "init",
            "--model",
            model.to_str().unwrap(),
            "--apply",
        ],
        0,
    );
    result["digest"].as_str().unwrap().to_owned()
}
fn delta(root: &Path, base: &str) -> Value {
    let change = json!({"schema_version":1,"id":"REQ-POINTS","base_digest":base,"requirement":{"system":"openspec","target":"openspec/changes/points","artifact":"openspec/changes/points/specs/orders/spec.md","requirement":"REQ-POINTS"},"upsert_elements":[{"id":"Points","kind":"entity","name":"Points","responsibility":"Track awarded points","context":"sales","module":"domain"}],"remove_elements":[],"upsert_relations":[{"id":"order-points","from":"Order","to":"Points","kind":"uses"}],"remove_relations":[],"touched_modules":["domain"]});
    write(
        root,
        ".archguard/changes/REQ-POINTS.json",
        &change.to_string(),
    );
    change
}
#[test]
fn maven_requirement_impact_repair_verify_and_update() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    let impact = call(root, &["change", "impact", "--change", "REQ-POINTS"], 0);
    assert!(
        impact["affected_modules"]
            .as_array()
            .unwrap()
            .contains(&json!("io.demo:app"))
    );
    assert!(impact["paths"]["app"].as_array().unwrap().len() >= 2);
    write(root, "domain/pom.xml", &pom("domain", &dependency("app")));
    assert_eq!(
        call(root, &["change", "check", "--change", "REQ-POINTS"], 2)["decision"],
        "BLOCK"
    );
    write(root, "domain/pom.xml", &pom("domain", ""));
    let report = root.join("change-report.json");
    call(
        root,
        &[
            "change",
            "check",
            "--change",
            "REQ-POINTS",
            "--report",
            report.to_str().unwrap(),
        ],
        0,
    );
    call(
        root,
        &[
            "change",
            "verify",
            "--change",
            "REQ-POINTS",
            "--report",
            report.to_str().unwrap(),
        ],
        0,
    );
    let before = fs::read(root.join(".archguard/baseline.json")).unwrap();
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            report.to_str().unwrap(),
        ],
        0,
    );
    assert_eq!(
        before,
        fs::read(root.join(".archguard/baseline.json")).unwrap()
    );
    let updated = call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            report.to_str().unwrap(),
            "--apply",
        ],
        0,
    );
    assert_ne!(updated["digest"], base);
    assert_eq!(updated["revision"], 2);
    assert_eq!(
        fs::read_dir(root.join(".archguard/baselines"))
            .unwrap()
            .count(),
        2
    );
    let model = call(root, &["model", "show"], 0);
    assert!(
        model["model"]["elements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == "Points")
    );
    call(root, &["check"], 0);
    call(root, &["change", "check", "--change", "REQ-POINTS"], 4);
}
#[test]
fn drift_and_forged_report_never_move_baseline() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let original = delta(root, &base);
    let report = call(root, &["change", "check", "--change", "REQ-POINTS"], 0);
    write(root, "report.json", &report.to_string());
    let path = root.join("report.json");
    let before = fs::read(root.join(".archguard/baseline.json")).unwrap();
    let mut changed = original.clone();
    changed["upsert_elements"][0]["responsibility"] = json!("different intent");
    write(
        root,
        ".archguard/changes/REQ-POINTS.json",
        &changed.to_string(),
    );
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            path.to_str().unwrap(),
            "--apply",
        ],
        4,
    );
    write(
        root,
        ".archguard/changes/REQ-POINTS.json",
        &original.to_string(),
    );
    write(
        root,
        "openspec/changes/points/tasks.md",
        "REQ-POINTS changed native scope",
    );
    call(
        root,
        &[
            "change",
            "verify",
            "--change",
            "REQ-POINTS",
            "--report",
            path.to_str().unwrap(),
        ],
        4,
    );
    write(root, "report.json", "{\"decision\":\"ALLOW\"}");
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            path.to_str().unwrap(),
            "--apply",
        ],
        4,
    );
    assert_eq!(
        before,
        fs::read(root.join(".archguard/baseline.json")).unwrap()
    );
}
#[test]
fn invalid_model_and_dangling_delta_are_rejected() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let mut change = delta(root, &base);
    change["remove_elements"] = json!(["Order"]);
    write(
        root,
        ".archguard/changes/REQ-POINTS.json",
        &change.to_string(),
    );
    call(root, &["change", "impact", "--change", "REQ-POINTS"], 4);
    let model = root.join("model.json");
    call(
        root,
        &[
            "baseline",
            "init",
            "--model",
            model.to_str().unwrap(),
            "--apply",
        ],
        4,
    );
    call(root, &["change", "check", "--change", "../outside"], 4);
}
#[test]
fn ordinary_check_rejects_unmodeled_dependency_and_binds_baseline() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let previous = call(root, &["check"], 0);
    write(root, "old.json", &previous.to_string());
    write(root, "app/pom.xml", &pom("app", ""));
    call(root, &["check"], 2);
    write(root, "app/pom.xml", &pom("app", &dependency("domain")));
    delta(root, &base);
    let report = call(root, &["change", "check", "--change", "REQ-POINTS"], 0);
    write(root, "r.json", &report.to_string());
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            root.join("r.json").to_str().unwrap(),
            "--apply",
        ],
        0,
    );
    call(
        root,
        &[
            "verify",
            "--report",
            root.join("old.json").to_str().unwrap(),
        ],
        4,
    );
}

#[test]
fn cargo_project_model_uses_same_requirement_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\", \"domain\"]\nresolver = \"2\"\n",
    );
    for name in ["app", "domain"] {
        let dependency = if name == "app" {
            "\n[dependencies]\ndomain = { path = \"../domain\" }\n"
        } else {
            ""
        };
        write(
            root,
            &format!("{name}/Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{dependency}"
            ),
        );
        write(
            root,
            &format!("{name}/src/lib.rs"),
            "pub fn value() -> u32 { 42 }\n",
        );
    }
    let output = Command::new("cargo")
        .args(["generate-lockfile", "--offline"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success());
    let maven = fixture();
    let mut model: Value =
        serde_json::from_slice(&fs::read(maven.path().join("model.json")).unwrap()).unwrap();
    model["elements"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["id"] != "root");
    for e in model["elements"].as_array_mut().unwrap() {
        if e["kind"] == "module" {
            e["implementation"] = e["id"].clone();
        }
    }
    write(root, "model.json", &model.to_string());
    write(
        root,
        ".archguard/contract.yaml",
        &fs::read_to_string(maven.path().join(".archguard/contract.yaml")).unwrap(),
    );
    for file in [
        "proposal.md",
        "design.md",
        "tasks.md",
        "specs/orders/spec.md",
    ] {
        write(
            root,
            &format!("openspec/changes/points/{file}"),
            "REQ-POINTS points",
        );
    }
    let base = init(root);
    delta(root, &base);
    let report = call(root, &["change", "check", "--change", "REQ-POINTS"], 0);
    assert_eq!(
        report["architecture_report"]["analyzer"]["id"],
        "archguard.cargo.workspace"
    );
    write(root, "report.json", &report.to_string());
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            root.join("report.json").to_str().unwrap(),
            "--apply",
        ],
        0,
    );
    call(root, &["check"], 0);
}

#[test]
fn removal_impacts_old_neighbors_and_must_be_implemented() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let mut change = delta(root, &base);
    change["remove_relations"] = json!(["app-domain"]);
    write(
        root,
        ".archguard/changes/REQ-POINTS.json",
        &change.to_string(),
    );
    let impact = call(root, &["change", "impact", "--change", "REQ-POINTS"], 0);
    assert!(
        impact["directly_changed"]
            .as_array()
            .unwrap()
            .contains(&json!("app"))
    );
    let report = call(root, &["change", "check", "--change", "REQ-POINTS"], 2);
    assert_eq!(report["model_findings"][0]["code"], "unmodeled_dependency");
    write(root, "app/pom.xml", &pom("app", ""));
    call(root, &["change", "check", "--change", "REQ-POINTS"], 0);
}

#[test]
fn blocked_report_and_lock_and_contract_drift_preserve_pointer() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    let before = fs::read(root.join(".archguard/baseline.json")).unwrap();
    write(root, "domain/pom.xml", &pom("domain", &dependency("app")));
    let blocked = call(root, &["change", "check", "--change", "REQ-POINTS"], 2);
    write(root, "report.json", &blocked.to_string());
    let path = root.join("report.json");
    assert_eq!(
        call(
            root,
            &[
                "baseline",
                "update",
                "--change",
                "REQ-POINTS",
                "--report",
                path.to_str().unwrap(),
                "--apply"
            ],
            2
        )["applied"],
        false
    );
    write(root, "domain/pom.xml", &pom("domain", ""));
    let report = call(root, &["change", "check", "--change", "REQ-POINTS"], 0);
    write(root, "report.json", &report.to_string());
    write(root, ".archguard/.baseline.lock", "another writer");
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            path.to_str().unwrap(),
            "--apply",
        ],
        4,
    );
    assert_eq!(
        fs::read_to_string(root.join(".archguard/.baseline.lock")).unwrap(),
        "another writer"
    );
    fs::remove_file(root.join(".archguard/.baseline.lock")).unwrap();
    let contract = fs::read_to_string(root.join(".archguard/contract.yaml"))
        .unwrap()
        .replace("\"revision\":\"1\"", "\"revision\":\"2\"");
    write(root, ".archguard/contract.yaml", &contract);
    call(root, &["change", "check", "--change", "REQ-POINTS"], 4);
    assert_eq!(
        before,
        fs::read(root.join(".archguard/baseline.json")).unwrap()
    );
}

#[test]
fn readonly_model_mcp_matches_cli_and_has_no_publish_tool() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    let before = fs::read(root.join(".archguard/baseline.json")).unwrap();
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"archguard_change_check","arguments":{"project":root,"change":"REQ-POINTS"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"archguard_model_show","arguments":{"project":root,"entity":"Order"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"archguard_change_check","arguments":{"project":root,"change":"REQ-POINTS","apply":true}}}),
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
    assert!(
        !replies[1]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"].as_str().unwrap().contains("update"))
    );
    assert_eq!(replies[2]["result"]["isError"], false, "{}", replies[2]);
    let report: Value =
        serde_json::from_str(replies[2]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        report,
        call(root, &["change", "check", "--change", "REQ-POINTS"], 0)
    );
    let view: Value =
        serde_json::from_str(replies[3]["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(view, call(root, &["model", "show", "--entity", "Order"], 0));
    assert_eq!(replies[4]["result"]["isError"], true);
    assert_eq!(
        before,
        fs::read(root.join(".archguard/baseline.json")).unwrap()
    );
}

#[test]
fn reports_cannot_be_exported_into_model_state() {
    let temp = fixture();
    let root = temp.path();
    call(
        root,
        &[
            "check",
            "--report",
            root.join(".archguard/baseline.json").to_str().unwrap(),
        ],
        4,
    );
    assert!(!root.join(".archguard/baseline.json").exists());
    let base = init(root);
    delta(root, &base);
    call(
        root,
        &[
            "change",
            "check",
            "--change",
            "REQ-POINTS",
            "--report",
            root.join(".archguard/changes/forged.json")
                .to_str()
                .unwrap(),
        ],
        4,
    );
    assert!(!root.join(".archguard/changes/forged.json").exists());
}

#[test]
fn impact_keeps_full_reachability_but_bounds_materialized_paths() {
    let temp = fixture();
    let root = temp.path();
    let mut model: Value =
        serde_json::from_slice(&fs::read(root.join("model.json")).unwrap()).unwrap();
    for index in 0..150 {
        model["elements"].as_array_mut().unwrap().push(json!({"id":format!("Object{index}"),"kind":"entity","name":"Declared object","responsibility":"Related domain state","context":"sales","module":"domain"}));
    }
    write(root, "model.json", &model.to_string());
    let base = init(root);
    delta(root, &base);
    let impact = call(root, &["change", "impact", "--change", "REQ-POINTS"], 0);
    assert!(impact["affected_elements"].as_array().unwrap().len() > 150);
    assert!(impact["paths"].as_object().unwrap().len() <= 128);
    assert_eq!(impact["witnesses_truncated"], true);
}

#[test]
fn relative_report_is_project_relative_even_from_another_directory() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    let invocation = root.join("invocation");
    fs::create_dir(&invocation).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .current_dir(&invocation)
        .args([
            "change",
            "check",
            "--project",
            root.to_str().unwrap(),
            "--change",
            "REQ-POINTS",
            "--report",
            "relative.json",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(root.join("relative.json").is_file());
    assert!(!invocation.join("relative.json").exists());
    call(
        root,
        &[
            "change",
            "verify",
            "--change",
            "REQ-POINTS",
            "--report",
            "relative.json",
        ],
        0,
    );
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            "relative.json",
            "--apply",
        ],
        0,
    );
}

#[test]
fn external_report_is_rejected_before_export() {
    let temp = fixture();
    let external = fixture();
    let root = temp.path();
    let base = init(root);
    delta(root, &base);
    let report = external.path().join("external.json");
    call(
        root,
        &[
            "change",
            "check",
            "--change",
            "REQ-POINTS",
            "--report",
            report.to_str().unwrap(),
        ],
        4,
    );
    assert!(!report.exists());
}

#[test]
fn source_drift_and_corrupt_snapshot_cannot_reuse_allow_report() {
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
            "REQ-POINTS",
            "--report",
            "report.json",
        ],
        0,
    );
    let before = fs::read(root.join(".archguard/baseline.json")).unwrap();
    let source = fs::read_to_string(root.join("domain/pom.xml")).unwrap();
    write(
        root,
        "domain/pom.xml",
        &(source.clone() + "\n<!-- same graph, changed source -->\n"),
    );
    call(
        root,
        &[
            "change",
            "verify",
            "--change",
            "REQ-POINTS",
            "--report",
            "report.json",
        ],
        4,
    );
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            "report.json",
            "--apply",
        ],
        4,
    );
    assert_eq!(
        before,
        fs::read(root.join(".archguard/baseline.json")).unwrap()
    );
    write(root, "domain/pom.xml", &source);
    let snapshot = format!(
        ".archguard/baselines/{}.json",
        base.strip_prefix("sha256:").unwrap()
    );
    let mut record: Value =
        serde_json::from_slice(&fs::read(root.join(&snapshot)).unwrap()).unwrap();
    record["model"]["project"] = json!("tampered");
    write(root, &snapshot, &record.to_string());
    call(root, &["model", "show"], 4);
    call(
        root,
        &[
            "baseline",
            "update",
            "--change",
            "REQ-POINTS",
            "--report",
            "report.json",
            "--apply",
        ],
        4,
    );
    assert_eq!(
        before,
        fs::read(root.join(".archguard/baseline.json")).unwrap()
    );
}

#[test]
fn native_reference_requires_selected_artifact_and_exact_requirement() {
    let temp = fixture();
    let root = temp.path();
    let base = init(root);
    let original = delta(root, &base);
    for (field, value) in [
        ("artifact", "model.json"),
        ("requirement", "REQ-POINT"),
        ("target", "openspec/changes/missing"),
    ] {
        let mut invalid = original.clone();
        invalid["requirement"][field] = json!(value);
        write(
            root,
            ".archguard/changes/REQ-POINTS.json",
            &invalid.to_string(),
        );
        call(root, &["change", "check", "--change", "REQ-POINTS"], 4);
    }
    assert_eq!(call(root, &["model", "show"], 0)["digest"], base);
}

#[test]
fn review_and_partial_analysis_cannot_initialize_baseline() {
    let temp = fixture();
    let root = temp.path();
    let contract = fs::read_to_string(root.join(".archguard/contract.yaml")).unwrap();
    let mut review: Value = serde_json::from_str(&contract).unwrap();
    review["spec"]["rules"][0] = json!({"id":"REVIEW_DEPENDENCY","enforcement":"review","assertion":{"type":"forbid_relation","subject":"io.demo:app","predicate":"depends_on","object":"io.demo:domain"}});
    write(root, ".archguard/contract.yaml", &review.to_string());
    let result = call(
        root,
        &["baseline", "init", "--model", "model.json", "--apply"],
        3,
    );
    assert_eq!(result["applied"], false);
    assert!(!root.join(".archguard/baseline.json").exists());
    write(root, ".archguard/contract.yaml", &contract);
    fs::remove_file(root.join("domain/pom.xml")).unwrap();
    let result = call(
        root,
        &["baseline", "init", "--model", "model.json", "--apply"],
        2,
    );
    assert_eq!(result["applied"], false);
    assert!(!root.join(".archguard/baseline.json").exists());
}
