mod common;
use std::{
    path::Path,
    process::{Command, Output},
};
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap()
}
#[test]
fn failed_run_invalidates_old_report_and_facts() {
    let tmp = common::Temp::new();
    let report = tmp.0.join("report.json");
    let facts = tmp.0.join("facts.json");
    std::fs::write(&report, "old ALLOW").unwrap();
    std::fs::write(&facts, "old complete").unwrap();
    let output = run(&[
        "check",
        "--project",
        "/missing-archguard-fixture-root",
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        report.to_str().unwrap(),
        "--facts",
        facts.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert!(
        !report.exists(),
        "old report must not survive failed attempt"
    );
    assert!(!facts.exists(), "old facts must not survive failed attempt");
    assert!(output.stdout.is_empty());
}
#[test]
fn review_advise_missing_manifest_and_file_output_preserve_legacy_semantics() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    let original = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/agent-job-contract.yaml"),
    )
    .unwrap();
    for (enforcement, code, decision) in [("review", 3, "REQUIRE_APPROVAL"), ("advise", 0, "ALLOW")]
    {
        let contract = tmp.0.join(format!("{enforcement}.yaml"));
        std::fs::write(
            &contract,
            original.replace(
                "enforcement: enforce",
                &format!("enforcement: {enforcement}"),
            ),
        )
        .unwrap();
        let report = tmp.0.join("report.json");
        let output = run(&[
            "check",
            "--project",
            "fixtures/forbidden",
            "--contract",
            contract.to_str().unwrap(),
            "--report",
            report.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(code));
        assert!(output.stdout.is_empty());
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
        assert_eq!(value["decision"], decision);
    }
    assert_eq!(
        run(&[
            "check",
            "--project",
            "examples",
            "--contract",
            "examples/agent-job-contract.yaml"
        ])
        .status
        .code(),
        Some(2)
    );
    let output = run(&[
        "check",
        "--project",
        "fixtures/allowed",
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        tmp.0.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
}

#[test]
fn profile_selection_cannot_silently_fall_back_on_missing_or_duplicate_values() {
    let base = [
        "check",
        "--project",
        "fixtures/allowed",
        "--contract",
        "examples/agent-job-contract.yaml",
    ];
    for tail in [
        vec!["--profile"],
        vec!["--profile", "legacy", "--profile", "cargo-declarations-v1"],
        vec!["--unknown", "ignored"],
    ] {
        let args: Vec<_> = base.iter().copied().chain(tail).collect();
        assert_eq!(run(&args).status.code(), Some(4));
    }
}
