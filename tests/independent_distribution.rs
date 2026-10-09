//! Exercise the source distribution outside every development sibling repository.
use std::{path::Path, process::Command};
#[test]
fn core_distribution_builds_tests_and_has_no_git_spec_dependencies() {
    if std::env::var_os("ARCHGUARD_DISTRIBUTION_CHILD").is_some() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("core.tar");
    let root = temp.path().join("core");
    std::fs::create_dir(&root).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        Command::new("python3")
            .arg(source.join("scripts/package-core.py"))
            .arg(&archive)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("tar")
            .args(["-xf"])
            .arg(&archive)
            .arg("-C")
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    assert!(!root.join(".git").exists());
    for args in [
        vec!["build", "--locked"],
        vec!["test", "--locked"],
        vec!["tree", "--locked", "--prefix", "none"],
    ] {
        let output = Command::new("cargo")
            .args(&args)
            .current_dir(&root)
            .env("ARCHGUARD_DISTRIBUTION_CHILD", "1")
            .env("CARGO_TARGET_DIR", temp.path().join("target"))
            .output()
            .unwrap();
        eprintln!(
            "isolated cargo {args:?}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.status.success(),
            "isolated cargo {args:?}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if args[0] == "tree" {
            let tree = String::from_utf8(output.stdout).unwrap();
            assert!(
                !tree
                    .lines()
                    .any(|l| l.starts_with("gitguard ") || l.starts_with("specguard "))
            );
        }
    }
    let metadata = Command::new("cargo")
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(metadata.status.success());
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    for package in metadata["packages"].as_array().unwrap() {
        if package["source"].is_null() {
            assert!(
                Path::new(package["manifest_path"].as_str().unwrap()).starts_with(&root),
                "path dependency escaped distribution: {package}"
            );
        }
    }
}
