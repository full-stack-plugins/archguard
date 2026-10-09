#![cfg(unix)]
mod common;
use archguard::analysis::runner::{self, Budget, Failure, IsolatedProject};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Duration, Instant},
};
fn script(script: &str, budget: &Budget, root: &Path) -> Result<Vec<u8>, Failure> {
    // Explicit controlled shell test double; production declaration scans never use it.
    runner::run(
        Path::new("/bin/sh"),
        &["-c".into(), script.into()],
        root,
        &BTreeMap::new(),
        budget,
    )
}
#[test]
fn timeout_kills_tool_and_pipe_holding_descendants() {
    let tmp = common::Temp::new();
    let budget = Budget {
        timeout: Duration::from_millis(70),
        ..Default::default()
    };
    let start = Instant::now();
    assert_eq!(
        script("/bin/sleep 5 & wait", &budget, &tmp.0),
        Err(Failure::Timeout)
    );
    assert!(start.elapsed() < Duration::from_secs(2));
}
#[test]
fn combined_output_budget_and_nonzero_exit_are_typed_failures() {
    let tmp = common::Temp::new();
    let budget = Budget {
        output_bytes: 16,
        ..Default::default()
    };
    assert_eq!(
        script(
            "printf '1234567890'; printf '1234567890' >&2",
            &budget,
            &tmp.0
        ),
        Err(Failure::OutputLimit)
    );
    assert_eq!(
        script("exit 7", &budget, &tmp.0),
        Err(Failure::Exit(Some(7)))
    );
    assert_eq!(script("printf ok", &budget, &tmp.0).unwrap(), b"ok");
}
#[test]
fn isolated_tool_writes_cannot_modify_original_lock() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    fs::write(tmp.0.join("Cargo.lock"), b"original").unwrap();
    let copy =
        IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &Budget::default()).unwrap();
    script(
        "printf changed > Cargo.lock",
        &Budget::default(),
        copy.root(),
    )
    .unwrap();
    assert_eq!(fs::read(tmp.0.join("Cargo.lock")).unwrap(), b"original");
    assert_eq!(
        fs::read(copy.root().join("Cargo.lock")).unwrap(),
        b"changed"
    );
    let path = copy.root().to_owned();
    drop(copy);
    assert!(!path.exists());
}
#[test]
fn allowlist_symlink_and_file_budget_fail_before_execution() {
    use std::os::unix::fs::symlink;
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    let budget = Budget::default();
    assert!(IsolatedProject::copy(&tmp.0, &[], &budget).is_err());
    assert!(
        IsolatedProject::copy(
            &tmp.0,
            std::slice::from_ref(&tmp.0),
            &Budget {
                file_bytes: 1,
                ..budget.clone()
            }
        )
        .is_err()
    );
    symlink("/etc/passwd", tmp.0.join("escape")).unwrap();
    assert!(IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &budget).is_err());
}
#[test]
fn candidate_cargo_config_cannot_select_a_wrapper_tool() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    fs::create_dir(tmp.0.join(".cargo")).unwrap();
    fs::write(
        tmp.0.join(".cargo/config.toml"),
        "[build]\nrustc-wrapper='/candidate/arbitrary-script'\n",
    )
    .unwrap();
    let result = IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &Budget::default());
    assert!(
        matches!(result, Err(Failure::Input(message)) if message.contains("configuration is unsupported"))
    );
}

#[test]
fn deadline_does_not_wait_for_a_detached_process_to_close_pipes() {
    let tmp = common::Temp::new();
    let budget = Budget {
        timeout: Duration::from_millis(30),
        ..Default::default()
    };
    let start = Instant::now();
    // Finite-lived controlled test double, deliberately outside the process group.
    assert_eq!(
        script("/usr/bin/setsid /bin/sleep 0.6 & wait", &budget, &tmp.0),
        Err(Failure::Timeout)
    );
    assert!(
        start.elapsed() < Duration::from_millis(400),
        "reader cleanup exceeded the deadline: {:?}",
        start.elapsed()
    );
}

#[test]
fn ambient_temporary_ancestor_cargo_config_is_rejected() {
    let tmp = common::Temp::new();
    fs::create_dir(tmp.0.join(".cargo")).unwrap();
    fs::write(tmp.0.join(".cargo/config.toml"), "[build]\njobs=1\n").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args([
            "check",
            "--project",
            "fixtures/allowed",
            "--contract",
            "examples/agent-job-contract.yaml",
            "--profile",
            "cargo-declarations-v1",
        ])
        .env("TMPDIR", &tmp.0)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(4),
        "ambient config must not be silently inherited"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("ancestor Cargo configuration"));
}
