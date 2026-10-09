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

#[test]
fn output_failure_cleans_every_independently_safe_old_artifact() {
    for existing_flag in ["--report", "--facts"] {
        for invalid_kind in ["missing-parent", "directory"] {
            let tmp = common::Temp::new();
            let existing = tmp.0.join("old.json");
            std::fs::write(&existing, "OLD SUCCESS").unwrap();
            let invalid = if invalid_kind == "directory" {
                tmp.0.clone()
            } else {
                tmp.0.join("missing/output.json")
            };
            let other_flag = if existing_flag == "--report" {
                "--facts"
            } else {
                "--report"
            };
            let output = run(&[
                "check",
                "--project",
                "fixtures/allowed",
                "--contract",
                "examples/agent-job-contract.yaml",
                existing_flag,
                existing.to_str().unwrap(),
                other_flag,
                invalid.to_str().unwrap(),
            ]);
            assert_eq!(output.status.code(), Some(4));
            assert!(
                !existing.exists(),
                "{existing_flag} survived {invalid_kind} failure"
            );
            assert!(output.stdout.is_empty());
        }
    }
}

#[test]
fn output_invalidation_preserves_member_manifest_lock_and_source() {
    for relative in ["agent-job/Cargo.toml", "Cargo.lock", "agent-job/src/lib.rs"] {
        let tmp = common::Temp::new();
        tmp.copy_fixture("allowed");
        std::fs::write(tmp.0.join("Cargo.lock"), "# protected lock\nversion = 4\n").unwrap();
        let input = tmp.0.join(relative);
        let before = std::fs::read(&input).unwrap();
        let output = run(&[
            "check",
            "--project",
            tmp.0.to_str().unwrap(),
            "--contract",
            "examples/agent-job-contract.yaml",
            "--profile",
            "cargo-declarations-v1",
            "--report",
            input.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(
            std::fs::read(&input).ok(),
            Some(before),
            "input modified: {relative}"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("not modifying"));
    }
}

#[cfg(unix)]
#[test]
fn external_source_aliases_are_preserved_while_safe_report_is_invalidated() {
    for alias_kind in ["symlink", "hardlink"] {
        let project = common::Temp::new();
        project.copy_fixture("allowed");
        let outputs = common::Temp::new();
        let source = project.0.join("agent-job/src/lib.rs");
        let alias = outputs.0.join("alias.json");
        if alias_kind == "symlink" {
            std::os::unix::fs::symlink(&source, &alias).unwrap();
        } else {
            std::fs::hard_link(&source, &alias).unwrap();
        }
        let before = std::fs::read(&source).unwrap();
        let report = outputs.0.join("report.json");
        std::fs::write(&report, "OLD SUCCESS").unwrap();
        let output = run(&[
            "check",
            "--project",
            project.0.to_str().unwrap(),
            "--contract",
            "examples/agent-job-contract.yaml",
            "--report",
            report.to_str().unwrap(),
            "--facts",
            alias.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert!(alias.exists(), "protected alias must not be removed");
        assert!(
            !report.exists(),
            "safe old report must still be invalidated"
        );
    }
}

#[test]
fn new_project_report_is_allowed_but_existing_inventory_file_is_not_replaced() {
    let project = common::Temp::new();
    project.copy_fixture("allowed");
    let report = project.0.join("new-report.json");
    let args = [
        "check",
        "--project",
        project.0.to_str().unwrap(),
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        report.to_str().unwrap(),
    ];
    assert_eq!(run(&args).status.code(), Some(0));
    let before = std::fs::read(&report).unwrap();
    let output = run(&args);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(std::fs::read(&report).unwrap(), before);
    assert!(String::from_utf8_lossy(&output.stderr).contains("not modifying"));
}

#[cfg(unix)]
#[test]
fn dangling_project_symlink_is_preserved_as_an_existing_input_entry() {
    let project = common::Temp::new();
    project.copy_fixture("allowed");
    let link = project.0.join("unresolved.json");
    std::os::unix::fs::symlink("missing-source", &link).unwrap();
    let output = run(&[
        "check",
        "--project",
        project.0.to_str().unwrap(),
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        link.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        Path::new("missing-source")
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("not modifying"));
}

#[test]
fn member_project_selection_still_protects_other_workspace_member_inputs() {
    let workspace = common::Temp::new();
    workspace.copy_fixture("allowed");
    let project = workspace.0.join("agent-job");
    let other_manifest = workspace.0.join("agent-contracts/Cargo.toml");
    let before = std::fs::read(&other_manifest).unwrap();
    let output = run(&[
        "check",
        "--project",
        project.to_str().unwrap(),
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        other_manifest.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(std::fs::read(&other_manifest).ok(), Some(before));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not modifying"));
}

fn external_member_workspace(
    parent: &Path,
    members: &str,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let workspace = parent.join("workspace");
    let member = parent.join("member");
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(member.join("src")).unwrap();
    std::fs::write(
        workspace.join("Cargo.toml"),
        format!("[workspace]\nmembers = [{members}]\nresolver = '2'\n"),
    )
    .unwrap();
    std::fs::write(
        member.join("Cargo.toml"),
        "[package]\nname='member'\nversion='0.1.0'\nworkspace='../workspace'\n",
    )
    .unwrap();
    std::fs::write(member.join("src/lib.rs"), "pub struct Member;\n").unwrap();
    (workspace, member)
}

#[test]
fn external_declared_and_glob_members_are_protected_without_touching_safe_outputs() {
    for members in ["'../member'", "'../mem*'", "'../memb[e]r'"] {
        for from_member in [false, true] {
            for input_flag in ["--report", "--facts"] {
                let tmp = common::Temp::new();
                let (workspace, member) = external_member_workspace(&tmp.0, members);
                let project = if from_member { &member } else { &workspace };
                assert_eq!(
                    run(&[
                        "check",
                        "--project",
                        project.to_str().unwrap(),
                        "--contract",
                        "examples/agent-job-contract.yaml"
                    ])
                    .status
                    .code(),
                    Some(0),
                    "fixture must be a supported legacy Cargo workspace"
                );
                let manifest = member.join("Cargo.toml");
                let before = std::fs::read(&manifest).unwrap();
                let safe_output = tmp.0.join("safe-old-output.json");
                std::fs::write(&safe_output, "OLD SUCCESS").unwrap();
                let safe_flag = if input_flag == "--report" {
                    "--facts"
                } else {
                    "--report"
                };
                let output = run(&[
                    "check",
                    "--project",
                    project.to_str().unwrap(),
                    "--contract",
                    "examples/agent-job-contract.yaml",
                    input_flag,
                    manifest.to_str().unwrap(),
                    safe_flag,
                    safe_output.to_str().unwrap(),
                ]);
                assert_eq!(
                    output.status.code(),
                    Some(4),
                    "external member must be rejected before analysis"
                );
                assert_eq!(std::fs::read(&manifest).unwrap(), before);
                assert!(
                    !safe_output.exists(),
                    "unrelated safe output must still be invalidated"
                );
                assert!(String::from_utf8_lossy(&output.stderr).contains("not modifying"));
            }
        }
    }
}

#[test]
fn external_nonmember_path_dependency_inputs_are_protected_transitively() {
    let tmp = common::Temp::new();
    let project = tmp.0.join("project");
    let external = tmp.0.join("external");
    let transitive = tmp.0.join("transitive");
    for dir in [&project, &external, &transitive] {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub struct Item;\n").unwrap();
    }
    std::fs::write(project.join("Cargo.toml"), "[package]\nname='project'\nversion='0.1.0'\n[workspace]\n[dependencies]\nexternal={path='../external'}\n").unwrap();
    std::fs::write(external.join("Cargo.toml"), "[package]\nname='external'\nversion='0.1.0'\n[workspace]\n[dependencies]\ntransitive={path='../transitive'}\n").unwrap();
    std::fs::write(
        transitive.join("Cargo.toml"),
        "[package]\nname='transitive'\nversion='0.1.0'\n[workspace]\n",
    )
    .unwrap();
    assert_eq!(
        run(&[
            "check",
            "--project",
            project.to_str().unwrap(),
            "--contract",
            "examples/agent-job-contract.yaml"
        ])
        .status
        .code(),
        Some(0)
    );
    for input in [external.join("Cargo.toml"), transitive.join("src/lib.rs")] {
        let before = std::fs::read(&input).unwrap();
        let output = run(&[
            "check",
            "--project",
            project.to_str().unwrap(),
            "--contract",
            "examples/agent-job-contract.yaml",
            "--report",
            input.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(4));
        assert_eq!(std::fs::read(&input).unwrap(), before);
        assert!(String::from_utf8_lossy(&output.stderr).contains("not modifying"));
    }
}

#[cfg(unix)]
#[test]
fn symlinked_manifest_preserves_the_dependency_root_actually_read_by_cargo() {
    let tmp = common::Temp::new();
    let project = tmp.0.join("primary/project");
    let stored = tmp.0.join("stored/project");
    for root in [&project, &stored] {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "").unwrap();
        let dependency = root.parent().unwrap().join("dependency");
        std::fs::create_dir_all(dependency.join("src")).unwrap();
        std::fs::write(dependency.join("src/lib.rs"), "").unwrap();
        std::fs::write(
            dependency.join("Cargo.toml"),
            "[package]\nname='dependency'\nversion='0.1.0'\n[workspace]\n",
        )
        .unwrap();
    }
    std::fs::write(stored.join("Cargo.toml"), "[package]\nname='project'\nversion='0.1.0'\n[workspace]\n[dependencies]\ndependency={path='../dependency'}\n").unwrap();
    std::os::unix::fs::symlink(stored.join("Cargo.toml"), project.join("Cargo.toml")).unwrap();
    let metadata = Command::new("cargo")
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        metadata.status.success(),
        "{}",
        String::from_utf8_lossy(&metadata.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let observed = metadata["packages"][0]["dependencies"][0]["path"]
        .as_str()
        .unwrap();
    let input = Path::new(observed).join("Cargo.toml");
    let before = std::fs::read(&input).unwrap();
    let output = run(&[
        "check",
        "--project",
        project.to_str().unwrap(),
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        input.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(std::fs::read(&input).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn literal_glob_metacharacter_ancestors_do_not_change_member_scope() {
    for name in ["project[1]", "project?", "project*"] {
        for member_kind in ["literal", "absolute-glob"] {
            let tmp = common::Temp::new();
            let parent = tmp.0.join(name);
            let member_pattern = match member_kind {
                "literal" => "../member".to_owned(),
                _ => format!("{}/mem*", glob::Pattern::escape(parent.to_str().unwrap())),
            };
            let (workspace, member) =
                external_member_workspace(&parent, &format!("'{member_pattern}'"));
            let dependency = parent.join("dependency[2]?*");
            std::fs::create_dir_all(dependency.join("src")).unwrap();
            std::fs::write(dependency.join("src/lib.rs"), "").unwrap();
            std::fs::write(
                dependency.join("Cargo.toml"),
                "[package]\nname='dependency'\nversion='0.1.0'\n[workspace]\n",
            )
            .unwrap();
            let mut manifest = std::fs::read_to_string(member.join("Cargo.toml")).unwrap();
            manifest.push_str("[dependencies]\ndependency={path='../dependency[2]?*'}\n");
            std::fs::write(member.join("Cargo.toml"), manifest).unwrap();
            assert_eq!(
                run(&[
                    "check",
                    "--project",
                    workspace.to_str().unwrap(),
                    "--contract",
                    "examples/agent-job-contract.yaml"
                ])
                .status
                .code(),
                Some(0),
                "real Cargo fixture {name}/{member_kind} must be valid"
            );
            for input in [member.join("Cargo.toml"), dependency.join("Cargo.toml")] {
                let before = std::fs::read(&input).unwrap();
                let safe = tmp.0.join("old-report.json");
                std::fs::write(&safe, "OLD SUCCESS").unwrap();
                let output = run(&[
                    "check",
                    "--project",
                    workspace.to_str().unwrap(),
                    "--contract",
                    "examples/agent-job-contract.yaml",
                    "--report",
                    input.to_str().unwrap(),
                    "--facts",
                    safe.to_str().unwrap(),
                ]);
                assert_eq!(output.status.code(), Some(4), "{name}/{member_kind}");
                assert_eq!(std::fs::read(&input).unwrap(), before);
                assert!(
                    !safe.exists(),
                    "unrelated safe artifact must still be invalidated"
                );
            }
        }
    }
}

#[test]
fn unresolved_member_scope_fails_closed_for_existing_and_new_destinations() {
    for members in [
        "'../missing'",
        "'../missing*'",
        "'../empty'",
        "'../[broken'",
    ] {
        for existing in [false, true] {
            let tmp = common::Temp::new();
            let workspace = tmp.0.join("workspace");
            std::fs::create_dir(&workspace).unwrap();
            std::fs::create_dir(tmp.0.join("empty")).unwrap();
            std::fs::write(
                workspace.join("Cargo.toml"),
                format!("[workspace]\nmembers=[{members}]\nresolver='2'\n"),
            )
            .unwrap();
            let report = tmp.0.join("report.json");
            if existing {
                std::fs::write(&report, "OLD SUCCESS").unwrap();
            }
            let output = run(&[
                "check",
                "--project",
                workspace.to_str().unwrap(),
                "--contract",
                "examples/agent-job-contract.yaml",
                "--report",
                report.to_str().unwrap(),
            ]);
            assert_eq!(
                output.status.code(),
                Some(4),
                "scope {members}, existing={existing}"
            );
            if existing {
                assert_eq!(std::fs::read(&report).unwrap(), b"OLD SUCCESS");
            } else {
                assert!(!report.exists());
            }
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("cannot establish input aliases")
            );
        }
    }
}

#[test]
fn manifest_parse_error_cannot_publish_to_a_new_output_path() {
    let tmp = common::Temp::new();
    std::fs::write(tmp.0.join("Cargo.toml"), "[workspace\n").unwrap();
    let report = tmp.0.join("report.json");
    let output = run(&[
        "check",
        "--project",
        tmp.0.to_str().unwrap(),
        "--contract",
        "examples/agent-job-contract.yaml",
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(4));
    assert!(!report.exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot establish input aliases"));
}
