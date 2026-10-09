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
fn session_escape_is_refused_before_starting_detached_work() {
    let tmp = common::Temp::new();
    let budget = Budget {
        timeout: Duration::from_millis(30),
        ..Default::default()
    };
    let start = Instant::now();
    // The trusted test double tries to leave the group; inherited filter refuses it.
    assert_eq!(
        script("/usr/bin/setsid /bin/sleep 0.6", &budget, &tmp.0),
        Err(Failure::Exit(Some(1)))
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

#[cfg(target_os = "linux")]
#[test]
fn detached_tool_cannot_write_after_timeout() {
    let tmp = common::Temp::new();
    let budget = Budget {
        timeout: Duration::from_millis(50),
        ..Default::default()
    };
    let result = script(
        "/usr/bin/setsid /bin/sh -c '/bin/sleep 0.15; printf escaped > late-marker' & /bin/sleep 1",
        &budget,
        &tmp.0,
    );
    assert_eq!(result, Err(Failure::Timeout));
    std::thread::sleep(Duration::from_millis(250));
    assert!(
        !tmp.0.join("late-marker").exists(),
        "escaped descendant survived timeout"
    );
}

#[test]
fn invalid_budgets_fail_before_running_or_copying() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    for budget in [
        Budget {
            timeout: Duration::ZERO,
            ..Default::default()
        },
        Budget {
            file_bytes: u64::MAX,
            ..Default::default()
        },
        Budget {
            output_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            script("printf unsafe > executed", &budget, &tmp.0),
            Err(Failure::Input(_))
        ));
        assert!(!tmp.0.join("executed").exists());
        assert!(matches!(
            IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &budget),
            Err(Failure::Input(_))
        ));
    }
}

#[test]
fn copy_deadline_is_checked_before_traversal() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    let budget = Budget {
        timeout: Duration::from_nanos(1),
        ..Default::default()
    };
    assert!(matches!(
        IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &budget),
        Err(Failure::Timeout)
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn source_fifo_and_each_input_budget_fail_without_blocking() {
    use std::os::unix::ffi::OsStrExt;
    let tmp = common::Temp::new();
    fs::write(tmp.0.join("one"), b"12345").unwrap();
    fs::write(tmp.0.join("two"), b"12345").unwrap();
    fs::create_dir(tmp.0.join("deep")).unwrap();
    fs::write(tmp.0.join("deep/three"), b"1").unwrap();
    for budget in [
        Budget {
            total_bytes: 9,
            ..Default::default()
        },
        Budget {
            files: 1,
            ..Default::default()
        },
        Budget {
            depth: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &budget),
            Err(Failure::Input(_))
        ));
    }
    let fifo = std::ffi::CString::new(tmp.0.join("fifo").as_os_str().as_bytes()).unwrap();
    // SAFETY: valid NUL-terminated temporary path; only creates a controlled test FIFO.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let start = Instant::now();
    assert!(matches!(
        IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &Budget::default()),
        Err(Failure::Input(_))
    ));
    assert!(start.elapsed() < Duration::from_secs(1));
}

#[cfg(target_os = "linux")]
#[test]
fn successful_leader_cannot_leave_a_background_writer() {
    let tmp = common::Temp::new();
    assert!(
        script(
            "(/bin/sleep 0.1; printf leaked > late-marker) >/dev/null 2>&1 & exit 0",
            &Budget::default(),
            &tmp.0
        )
        .is_ok()
    );
    std::thread::sleep(Duration::from_millis(200));
    assert!(!tmp.0.join("late-marker").exists());
}

#[cfg(target_os = "linux")]
#[test]
fn inherited_filter_rejects_session_and_group_changes_in_children() {
    let tmp = common::Temp::new();
    let output = runner::run(Path::new("/usr/bin/python3"), &["-c".into(), "import os,errno\nfor f in [os.setsid,lambda:os.setpgid(0,0)]:\n try:f()\n except OSError as e:assert e.errno==errno.EPERM\n else:raise AssertionError('group escaped')\nprint('confined')".into()], &tmp.0, &BTreeMap::new(), &Budget::default()).unwrap();
    assert_eq!(output, b"confined\n");
}

#[test]
fn actual_cargo_lock_is_written_only_to_private_copy() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    let budget = Budget::default();
    let copy = IsolatedProject::copy(&tmp.0, std::slice::from_ref(&tmp.0), &budget).unwrap();
    let tools = runner::Toolchain::discover(&budget).unwrap();
    let output = runner::run(
        &tools.cargo,
        &[
            "metadata".into(),
            "--offline".into(),
            "--no-deps".into(),
            "--format-version=1".into(),
        ],
        copy.root(),
        &tools.environment(&copy.tool_home()),
        &budget,
    )
    .unwrap();
    assert!(
        serde_json::from_slice::<serde_json::Value>(&output).unwrap()["packages"]
            .as_array()
            .unwrap()
            .len()
            >= 2
    );
    assert!(!tmp.0.join("Cargo.lock").exists());
    // Cargo is allowed to emit lock state only inside the isolated tree.
    assert!(copy.root().join("Cargo.toml").exists());
}

#[test]
fn controlled_metadata_failure_is_error_but_legacy_stays_partial() {
    use std::os::unix::fs::PermissionsExt;
    let tools = common::Temp::new();
    for name in ["cargo", "rustc"] {
        let path = tools.0.join(name);
        fs::write(&path, "#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'controlled-tool 1\\n'; exit 0; fi\nexit 7\n").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let invoke = |enhanced| {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_archguard"));
        command
            .args([
                "check",
                "--project",
                "fixtures/allowed",
                "--contract",
                "examples/agent-job-contract.yaml",
            ])
            .env("PATH", &tools.0)
            .current_dir(env!("CARGO_MANIFEST_DIR"));
        if enhanced {
            command.args(["--profile", "cargo-declarations-v1"]);
        }
        command.output().unwrap()
    };
    let enhanced = invoke(true);
    assert_eq!(enhanced.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&enhanced.stderr).contains("cargo metadata: Exit(Some(7))"));
    assert!(enhanced.stdout.is_empty());
    let legacy = invoke(false);
    assert_eq!(legacy.status.code(), Some(2));
    let report: serde_json::Value = serde_json::from_slice(&legacy.stdout).unwrap();
    assert_eq!(report["decision"], "BLOCK");
}
