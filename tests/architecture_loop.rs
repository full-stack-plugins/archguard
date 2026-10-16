use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn write(root: &Path, name: &str, body: &str) {
    let file = root.join(name);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, body).unwrap();
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "pom.xml",
        r#"<project><modelVersion>4.0.0</modelVersion><groupId>io.demo</groupId><artifactId>root</artifactId><version>${revision}</version><packaging>pom</packaging><properties><revision>1.0</revision></properties><modules><module>app</module><module>domain</module><module>infra</module></modules><dependencyManagement><dependencies><dependency><groupId>io.demo</groupId><artifactId>infra</artifactId><version>1.0</version></dependency></dependencies></dependencyManagement></project>"#,
    );
    for module in ["app", "domain", "infra"] {
        write(
            temp.path(),
            &format!("{module}/pom.xml"),
            &pom(
                module,
                if module == "app" {
                    Some("domain")
                } else {
                    None
                },
            ),
        );
    }
    write(temp.path(), ".archguard/contract.yaml", &json!({"apiVersion":"guard.partme.ai/v1alpha2","kind":"GuardContract","metadata":{"id":"layers","revision":"1"},"spec":{"rules":[
        {"id":"DOMAIN-BOUNDARY","enforcement":"enforce","assertion":{"type":"forbid_path","subject":"io.demo:domain","predicate":"depends_on","object":"io.demo:infra"}},
        {"id":"NO-CYCLES","enforcement":"enforce","assertion":{"type":"acyclic","predicate":"depends_on"}}
    ]}}).to_string());
    temp
}
fn pom(name: &str, dependency: Option<&str>) -> String {
    let dependencies = dependency.map(|d| format!("<dependencies><dependency><groupId>${{project.groupId}}</groupId><artifactId>{d}</artifactId><version>${{revision}}</version></dependency></dependencies>")).unwrap_or_default();
    format!(
        "<project><modelVersion>4.0.0</modelVersion><parent><groupId>io.demo</groupId><artifactId>root</artifactId><version>1.0</version></parent><artifactId>{name}</artifactId>{dependencies}</project>"
    )
}
fn call(root: &Path, args: &[&str], code: i32) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args(args)
        .arg("--project")
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(code),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn selected_profile_violation_repair_and_configuration_drift() {
    let root = fixture();
    let profile = "<profiles><profile><id>integration</id><dependencies><dependency><groupId>io.demo</groupId><artifactId>infra</artifactId><version>1.0</version></dependency></dependencies></profile></profiles>";
    write(
        root.path(),
        "domain/pom.xml",
        &pom("domain", None).replace("</project>", &format!("{profile}</project>")),
    );
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"integration":true}}"#,
    );
    let blocked = call(root.path(), &["check"], 2);
    assert_eq!(blocked["evaluations"][0]["status"], "FAIL");
    let explained = call(root.path(), &["explain", "--rule", "DOMAIN-BOUNDARY"], 2);
    assert!(
        explained["evaluation"]["matchedFacts"][0]["source"]
            .as_str()
            .unwrap()
            .starts_with("domain/pom.xml:")
    );
    let fixed_profile = profile.replace(
        "<artifactId>infra</artifactId>",
        "<artifactId>external-api</artifactId>",
    );
    write(
        root.path(),
        "domain/pom.xml",
        &pom("domain", None).replace("</project>", &format!("{fixed_profile}</project>")),
    );
    let fixed = call(root.path(), &["check"], 0);
    write(root.path(), "fixed.json", &fixed.to_string());
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("fixed.json").to_str().unwrap(),
        ],
        0,
    );
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"integration":false}}"#,
    );
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("fixed.json").to_str().unwrap(),
        ],
        4,
    );
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"typo":true}}"#,
    );
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
}

#[test]
fn native_spec_binding_is_validated_and_invalidates_old_reports() {
    let root = fixture();
    for file in [
        "proposal.md",
        "design.md",
        "tasks.md",
        "specs/layers/spec.md",
    ] {
        write(
            root.path(),
            &format!("openspec/changes/layers/{file}"),
            "REQ-1 domain boundary\nREQ-2 acyclic",
        );
    }
    let artifact = "openspec/changes/layers/specs/layers/spec.md";
    write(
        root.path(),
        ".archguard/spec-binding.json",
        &json!({"system":"openspec","target":"openspec/changes/layers","rules":{
        "DOMAIN-BOUNDARY":{"artifact":artifact,"requirement":"REQ-1"},
        "NO-CYCLES":{"artifact":artifact,"requirement":"REQ-2"}}})
        .to_string(),
    );
    let report = call(root.path(), &["check"], 0);
    write(root.path(), "bound.json", &report.to_string());
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("bound.json").to_str().unwrap(),
        ],
        0,
    );
    write(
        root.path(),
        artifact,
        "REQ-1 revised domain boundary\nREQ-2 acyclic",
    );
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("bound.json").to_str().unwrap(),
        ],
        4,
    );
    write(root.path(), artifact, "no requirements");
    call(root.path(), &["check"], 4);
}

#[test]
fn profile_properties_modules_and_managed_dependencies_are_applied() {
    let root = fixture();
    let path = root.path().join("pom.xml");
    let original = fs::read_to_string(&path)
        .unwrap()
        .replace("<module>infra</module>", "");
    let profile = "<profiles><profile><id>full</id><properties><infra.version>1.0</infra.version></properties><modules><module>infra</module></modules><dependencyManagement><dependencies><dependency><groupId>io.demo</groupId><artifactId>infra</artifactId><version>${infra.version}</version></dependency></dependencies></dependencyManagement></profile></profiles>";
    write(
        root.path(),
        "pom.xml",
        &original.replace("</project>", &format!("{profile}</project>")),
    );
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"full":true}}"#,
    );
    write(
        root.path(),
        "domain/pom.xml",
        &pom("domain", Some("infra")).replace("<version>${revision}</version>", ""),
    );
    let blocked = call(root.path(), &["check"], 2);
    assert_eq!(blocked["evaluations"][0]["status"], "FAIL");
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"full":false}}"#,
    );
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"full":"true"}}"#,
    );
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
}

#[test]
fn later_profile_property_overrides_earlier_and_duplicates_block() {
    let root = fixture();
    let source = fs::read_to_string(root.path().join("domain/pom.xml")).unwrap();
    let profiles = "<profiles><profile><id>early</id><properties><target>external</target></properties></profile><profile><id>late</id><properties><target>infra</target></properties><dependencies><dependency><groupId>io.demo</groupId><artifactId>${target}</artifactId><version>1.0</version></dependency></dependencies></profile></profiles>";
    write(
        root.path(),
        "domain/pom.xml",
        &source.replace("</project>", &format!("{profiles}</project>")),
    );
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"late":true,"early":true}}"#,
    );
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "FAIL"
    );
    write(
        root.path(),
        "domain/pom.xml",
        &source.replace(
            "</project>",
            &format!(
                "{}</project>",
                profiles.replace("<id>late</id>", "<id>early</id>")
            ),
        ),
    );
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
}

#[test]
fn cargo_native_spec_binding_and_current_source_verify_remain_supported() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Cargo.toml",
        "[package]\nname = \"orders\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(
        root.path(),
        "src/lib.rs",
        "pub fn order_count() -> usize { 0 }\n",
    );
    write(root.path(), ".archguard/contract.yaml", &json!({"apiVersion":"guard.partme.ai/v1alpha2","kind":"GuardContract","metadata":{"id":"orders","revision":"1"},"spec":{"rules":[{"id":"NO-CYCLES","enforcement":"enforce","assertion":{"type":"acyclic","predicate":"depends_on"}}]}}).to_string());
    write(root.path(), ".specify/memory/constitution.md", "principles");
    for file in ["spec.md", "plan.md", "tasks.md"] {
        write(
            root.path(),
            &format!("specs/001-orders/{file}"),
            "REQ-1 acyclic",
        );
    }
    write(root.path(), ".archguard/spec-binding.json", &json!({"system":"speckit","target":"specs/001-orders","rules":{"NO-CYCLES":{"artifact":"specs/001-orders/spec.md","requirement":"REQ-1"}}}).to_string());
    let report = call(root.path(), &["check"], 0);
    assert_eq!(report["analyzer"]["id"], "archguard.cargo.workspace");
    write(root.path(), "report.json", &report.to_string());
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("report.json").to_str().unwrap(),
        ],
        0,
    );
    write(
        root.path(),
        "specs/001-orders/plan.md",
        "REQ-1 revised plan",
    );
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("report.json").to_str().unwrap(),
        ],
        4,
    );
}

#[test]
fn native_binding_requires_exact_rule_set_and_requirement_markers() {
    let root = fixture();
    write(
        root.path(),
        "docs/superpowers/specs/layers.md",
        "REQ-10 domain\nREQ-2 cycles",
    );
    let artifact = "docs/superpowers/specs/layers.md";
    let mut binding = json!({"system":"superpowers","target":artifact,"rules":{
        "DOMAIN-BOUNDARY":{"artifact":artifact,"requirement":"REQ-1"},
        "NO-CYCLES":{"artifact":artifact,"requirement":"REQ-2"}}});
    write(
        root.path(),
        ".archguard/spec-binding.json",
        &binding.to_string(),
    );
    call(root.path(), &["check"], 4);
    binding["rules"]["DOMAIN-BOUNDARY"]["requirement"] = json!("REQ-10");
    write(
        root.path(),
        ".archguard/spec-binding.json",
        &binding.to_string(),
    );
    call(root.path(), &["check"], 0);
    binding["rules"]
        .as_object_mut()
        .unwrap()
        .remove("NO-CYCLES");
    write(
        root.path(),
        ".archguard/spec-binding.json",
        &binding.to_string(),
    );
    call(root.path(), &["check"], 4);
}
#[test]
fn maven_repair_and_current_source_verification_close_the_loop() {
    let root = fixture();
    let allowed = call(root.path(), &["check"], 0);
    assert_eq!(allowed["decision"], "ALLOW");
    write(root.path(), "domain/pom.xml", &pom("domain", Some("infra")));
    let blocked = call(root.path(), &["check"], 2);
    assert_eq!(blocked["evaluations"][0]["status"], "FAIL");
    let explanation = call(root.path(), &["explain", "--rule", "DOMAIN-BOUNDARY"], 2);
    assert!(explanation["remediation"].is_string());
    write(root.path(), "blocked.json", &blocked.to_string());
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("blocked.json").to_str().unwrap(),
        ],
        2,
    );
    write(root.path(), "domain/pom.xml", &pom("domain", None));
    let stale = call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("blocked.json").to_str().unwrap(),
        ],
        4,
    );
    assert_eq!(stale["status"], "error");
    let fixed = call(root.path(), &["check"], 0);
    write(root.path(), "fixed.json", &fixed.to_string());
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("fixed.json").to_str().unwrap(),
        ],
        0,
    );
}
#[test]
fn model_profiles_and_missing_modules_cannot_silently_pass() {
    let root = fixture();
    write(root.path(), "domain/pom.xml", &pom("domain", None).replace("</project>", "<profiles><profile><id>conditional</id><dependencies><dependency><groupId>io.demo</groupId><artifactId>infra</artifactId></dependency></dependencies></profile></profiles></project>"));
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
    write(root.path(), "domain/pom.xml", &pom("domain", None));
    fs::remove_file(root.path().join("infra/pom.xml")).unwrap();
    assert_eq!(call(root.path(), &["check"], 2)["decision"], "BLOCK");
}
#[test]
fn output_collision_and_unknown_options_preserve_inputs() {
    let root = fixture();
    let path = root.path().join("pom.xml");
    let before = fs::read(&path).unwrap();
    call(
        root.path(),
        &["check", "--report", path.to_str().unwrap()],
        4,
    );
    assert_eq!(fs::read(path).unwrap(), before);
    call(root.path(), &["check", "--typo", "yes"], 4);
}

#[test]
fn maven_four_namespace_subprojects_and_managed_versions() {
    let root = fixture();
    let path = root.path().join("pom.xml");
    let source = fs::read_to_string(&path)
        .unwrap()
        .replace(
            "<project>",
            "<project xmlns=\"http://maven.apache.org/POM/4.1.0\">",
        )
        .replace("4.0.0", "4.1.0")
        .replace("modules>", "subprojects>")
        .replace("module>", "subproject>");
    fs::write(path, source).unwrap();
    write(
        root.path(),
        "domain/pom.xml",
        &pom("domain", Some("infra")).replace("<version>${revision}</version>", ""),
    );
    let report = call(root.path(), &["check"], 2);
    assert_eq!(report["evaluations"][0]["status"], "FAIL");
    assert!(
        report["evaluations"][0]["matchedFacts"][0]["source"]
            .as_str()
            .unwrap()
            .starts_with("domain/pom.xml:")
    );
}

#[test]
fn reactor_cycle_and_multi_hop_path_have_witnesses() {
    let root = fixture();
    write(root.path(), "domain/pom.xml", &pom("domain", Some("app")));
    write(root.path(), "app/pom.xml", &pom("app", Some("infra")));
    write(root.path(), "infra/pom.xml", &pom("infra", Some("domain")));
    let report = call(root.path(), &["check"], 2);
    assert_eq!(
        report["evaluations"][0]["matchedFacts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        report["evaluations"][1]["matchedFacts"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn unsafe_or_ambiguous_models_are_indeterminate() {
    let mutations = [
        (
            "<artifactId>domain</artifactId>",
            "<artifactId>infra</artifactId>",
        ),
        (
            "<artifactId>domain</artifactId>",
            "<artifactId>${missing}</artifactId>",
        ),
        ("</parent>", "<relativePath/></parent>"),
        ("<version>1.0</version>", "<version>2.0</version>"),
        ("<project>", "<project xmlns=\"urn:unexpected\">"),
        (
            "<project>",
            "<!DOCTYPE project [<!ENTITY fake \"domain\">]><project>",
        ),
    ];
    for (from, to) in mutations {
        let root = fixture();
        write(
            root.path(),
            "domain/pom.xml",
            &pom("domain", None).replace(from, to),
        );
        assert_eq!(
            call(root.path(), &["check"], 2)["evaluations"][0]["status"],
            "INDETERMINATE",
            "{from} => {to}"
        );
    }
}

#[test]
fn escaped_module_and_version_conflict_do_not_pass() {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("inside");
    fs::create_dir_all(&root).unwrap();
    write(outer.path(), "outside/pom.xml", &pom("outside", None));
    write(
        &root,
        "pom.xml",
        "<project><modelVersion>4.0.0</modelVersion><groupId>io.demo</groupId><artifactId>root</artifactId><version>1.0</version><modules><module>../outside</module></modules></project>",
    );
    let contract_root = fixture();
    let contract = contract_root.path().join(".archguard/contract.yaml");
    assert_eq!(
        call(
            &root,
            &["check", "--contract", contract.to_str().unwrap()],
            2
        )["evaluations"][0]["status"],
        "INDETERMINATE"
    );
    let other = fixture();
    write(
        other.path(),
        "domain/pom.xml",
        &pom("domain", Some("infra"))
            .replace("<version>${revision}</version>", "<version>9.0</version>"),
    );
    assert_eq!(
        call(other.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
}

#[test]
fn export_preflight_contract_changes_and_tampered_reports() {
    let root = fixture();
    let fresh = root.path().join("new.json");
    call(
        root.path(),
        &[
            "check",
            "--facts",
            fresh.to_str().unwrap(),
            "--report",
            root.path().join("pom.xml").to_str().unwrap(),
        ],
        4,
    );
    assert!(!fresh.exists());
    let report = call(
        root.path(),
        &["check", "--report", fresh.to_str().unwrap()],
        0,
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&fresh).unwrap()).unwrap(),
        report
    );
    let mut tampered = report;
    tampered["signed"] = json!(true);
    write(root.path(), "tampered.json", &tampered.to_string());
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("tampered.json").to_str().unwrap(),
        ],
        4,
    );
    let contract_path = root.path().join(".archguard/contract.yaml");
    let mut contract: Value = serde_json::from_slice(&fs::read(&contract_path).unwrap()).unwrap();
    contract["metadata"]["revision"] = json!("2");
    fs::write(contract_path, contract.to_string()).unwrap();
    call(
        root.path(),
        &["verify", "--report", fresh.to_str().unwrap()],
        4,
    );
    call(root.path(), &["contract", "validate"], 0);
    call(root.path(), &["explain", "--rule", "typo"], 4);
}

#[test]
fn actual_mcp_checks_and_verifies_using_same_read_only_engine() {
    use std::io::Write;
    use std::process::Stdio;
    let root = fixture();
    let source = fs::read_to_string(root.path().join("pom.xml")).unwrap();
    write(root.path(), "pom.xml", &source.replace("</project>", "<profiles><profile><id>ci</id><properties><scenario>ci</scenario></properties></profile></profiles></project>"));
    write(
        root.path(),
        ".archguard/maven.json",
        r#"{"profiles":{"ci":true}}"#,
    );
    for file in [
        "proposal.md",
        "design.md",
        "tasks.md",
        "specs/layers/spec.md",
    ] {
        write(
            root.path(),
            &format!("openspec/changes/layers/{file}"),
            "REQ-1 domain\nREQ-2 cycles",
        );
    }
    write(
        root.path(),
        ".specify/memory/constitution.md",
        "other system, do not choose automatically",
    );
    let artifact = "openspec/changes/layers/specs/layers/spec.md";
    write(
        root.path(),
        ".archguard/spec-binding.json",
        &json!({"system":"openspec","target":"openspec/changes/layers","rules":{
        "DOMAIN-BOUNDARY":{"artifact":artifact,"requirement":"REQ-1"},
        "NO-CYCLES":{"artifact":artifact,"requirement":"REQ-2"}}})
        .to_string(),
    );
    let report = call(root.path(), &["check"], 0);
    write(root.path(), "report.json", &report.to_string());
    let config: Value = serde_json::from_str(include_str!("../plugin/mcp.json")).unwrap();
    let server = &config["mcpServers"]["archguard"];
    assert_eq!(server["command"], "archguard");
    let args: Vec<&str> = server["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"archguard_check","arguments":{"project":root.path()}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"archguard_explain","arguments":{"project":root.path(),"rule":"DOMAIN-BOUNDARY"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"archguard_verify","arguments":{"project":root.path(),"report":root.path().join("report.json")}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"archguard_check","arguments":{"project":root.path(),"facts":root.path().join("must-not-write.json")}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"archguard_check","arguments":{"project":"."}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"archguard_context","arguments":{"project":root.path(),"spec_system":"openspec","spec_target":"openspec/changes/layers"}}}),
    ];
    let before = fs::read(root.path().join("pom.xml")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let input = requests
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let responses: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        responses[1]["result"]["tools"].as_array().unwrap().len(),
        10
    );
    let from_mcp: Value = serde_json::from_str(
        responses[2]["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(from_mcp, report);
    for index in [2, 3, 4] {
        assert_eq!(responses[index]["result"]["isError"], false);
    }
    for index in [5, 6] {
        assert_eq!(responses[index]["result"]["isError"], true);
    }
    assert_eq!(responses[7]["result"]["isError"], false);
    let context: Value = serde_json::from_str(
        responses[7]["result"]["content"][0]["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(context["specification"]["status"], "ready");
    assert_eq!(context["domain"]["spec_binding"]["status"], "bound");
    assert!(!root.path().join("must-not-write.json").exists());
    assert_eq!(before, fs::read(root.path().join("pom.xml")).unwrap());
}

#[test]
fn jvm_memory_limits_are_supported_but_model_overrides_are_not() {
    let root = fixture();
    write(root.path(), ".mvn/jvm.config", "-Xmx1024m\n-Xms256m\n");
    call(root.path(), &["check"], 0);
    write(root.path(), ".mvn/jvm.config", "-Drevision=other\n");
    assert_eq!(
        call(root.path(), &["check"], 2)["evaluations"][0]["status"],
        "INDETERMINATE"
    );
}

#[test]
fn local_parent_outside_reactor_inherits_dependencies_and_binds_snapshot() {
    let root = fixture();
    write(root.path(), "base/pom.xml", &pom("base", Some("domain")));
    write(
        root.path(),
        "app/pom.xml",
        "<project><modelVersion>4.0.0</modelVersion><parent><groupId>io.demo</groupId><artifactId>base</artifactId><version>1.0</version><relativePath>../base/pom.xml</relativePath></parent><artifactId>app</artifactId></project>",
    );
    let facts_path = root.path().join("facts.json");
    let report = call(
        root.path(),
        &["check", "--facts", facts_path.to_str().unwrap()],
        0,
    );
    let facts: Value = serde_json::from_slice(&fs::read(facts_path).unwrap()).unwrap();
    assert!(
        facts["facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["subject"] == "io.demo:app"
                && f["object"] == "io.demo:domain"
                && f["source"].as_str().unwrap().starts_with("base/pom.xml:"))
    );
    write(root.path(), "report.json", &report.to_string());
    write(root.path(), "base/pom.xml", &pom("base", Some("infra")));
    call(
        root.path(),
        &[
            "verify",
            "--report",
            root.path().join("report.json").to_str().unwrap(),
        ],
        4,
    );
}

#[test]
fn plugin_manifest_and_injected_rules_describe_real_capabilities() {
    let manifest: Value = serde_json::from_str(include_str!("../plugin/plugin.json")).unwrap();
    assert_eq!(
        manifest["$schema"],
        "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"
    );
    assert_eq!(manifest["name"], "archguard");
    assert_eq!(manifest["version"], env!("CARGO_PKG_VERSION"));
    assert!(
        include_str!("../plugin/skills/architecture-check/SKILL.md")
            .starts_with("---\nname: architecture-check\n")
    );
    let root = fixture();
    let context = call(root.path(), &["context"], 0);
    assert_eq!(context["domain"]["status"], "configured");
    assert_eq!(context["quality_decision"], "not_evaluated");
    let prompt = call(root.path(), &["instructions", "render"], 0);
    let text = prompt["instructions"].as_str().unwrap();
    assert!(text.contains("DOMAIN-BOUNDARY"));
    assert!(text.contains("archguard_check"));
    assert!(text.contains("<!-- ARCHGUARD_START -->"));
    write(root.path(), ".archguard/contract.yaml", "broken: yaml");
    assert_eq!(
        call(root.path(), &["context"], 0)["domain"]["status"],
        "invalid"
    );
}
