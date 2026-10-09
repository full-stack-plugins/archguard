use archguard::CargoWorkspaceAnalyzer;
use guardengine::{
    Completeness, Decision, GuardAnalyzer, RuleStatus, evaluate, load_contract_yaml,
};
use std::{path::Path, process::Command};

fn example_root(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}
fn contract() -> guardengine::GuardContract {
    load_contract_yaml(&std::fs::read(example_root("examples/agent-job-contract.yaml")).unwrap())
        .unwrap()
}

#[test]
fn allowed_workspace_passes() {
    let facts = CargoWorkspaceAnalyzer
        .analyze(&example_root("fixtures/allowed"), "allowed")
        .unwrap();
    assert_eq!(
        facts.completeness,
        Completeness::Complete,
        "{:?}",
        facts.diagnostics
    );
    assert!(
        facts
            .facts
            .iter()
            .any(|f| f.subject == "agent-job" && f.object == "agent-contracts")
    );
    let report = evaluate(&contract(), &facts).unwrap();
    assert_eq!(report.decision, Decision::Allow);
    assert_eq!(report.evaluations[0].status, RuleStatus::Pass);
}

#[test]
fn forbidden_workspace_is_blocked_with_source_evidence() {
    let facts = CargoWorkspaceAnalyzer
        .analyze(&example_root("fixtures/forbidden"), "forbidden")
        .unwrap();
    assert_eq!(
        facts.completeness,
        Completeness::Complete,
        "{:?}",
        facts.diagnostics
    );
    assert!(
        facts
            .facts
            .iter()
            .any(|f| f.subject == "agent-job" && f.object == "agent-saas")
    );
    let report = evaluate(&contract(), &facts).unwrap();
    assert_eq!(report.decision, Decision::Block);
    assert_eq!(
        report.evaluations[0].matched_facts[0].source,
        "agent-job/Cargo.toml"
    );
    assert!(report.subject.snapshot_digest.starts_with("sha256:"));
}

#[test]
fn missing_manifest_is_indeterminate_and_blocks() {
    let facts = CargoWorkspaceAnalyzer
        .analyze(&example_root("examples"), "broken")
        .unwrap();
    assert_eq!(facts.completeness, Completeness::Partial);
    let report = evaluate(&contract(), &facts).unwrap();
    assert_eq!(report.decision, Decision::Block);
    assert_eq!(report.evaluations[0].status, RuleStatus::Indeterminate);
}

#[test]
fn actual_cli_exit_code_blocks_forbidden_workspace() {
    let output = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args([
            "check",
            "--project",
            "fixtures/forbidden",
            "--contract",
            "examples/agent-job-contract.yaml",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: guardengine::GuardReport = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.decision, Decision::Block);
}

#[test]
fn actual_cli_exit_code_allows_compliant_workspace() {
    let output = Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args([
            "check",
            "--project",
            "fixtures/allowed",
            "--contract",
            "examples/agent-job-contract.yaml",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let parsed: guardengine::GuardReport = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(parsed.decision, Decision::Allow);
}
