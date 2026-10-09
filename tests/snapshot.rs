mod common;
use archguard::{
    CargoWorkspaceAnalyzer,
    analysis::{analyze, profile::FrozenAnalysisProfile},
};
use guardengine::GuardAnalyzer;
use std::path::Path;

fn profile() -> FrozenAnalysisProfile {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/agent-job-contract.yaml"),
    )
    .unwrap();
    FrozenAnalysisProfile::freeze(&guardengine::load_contract_yaml(&bytes).unwrap(), [])
}

#[test]
fn lock_change_invalidates_enhanced_snapshot_but_not_legacy_manifest_digest() {
    let tmp = common::Temp::new();
    tmp.copy_fixture("allowed");
    std::fs::write(
        tmp.0.join("Cargo.lock"),
        "# frozen fixture lock\nversion = 4\n",
    )
    .unwrap();
    let legacy = CargoWorkspaceAnalyzer.analyze(&tmp.0, "sample").unwrap();
    let before = analyze(&tmp.0, "sample", &profile()).unwrap();
    let lock = tmp.0.join("Cargo.lock");
    let mut bytes = std::fs::read(&lock).unwrap();
    bytes.extend_from_slice(b"\n# snapshot identity changed\n");
    std::fs::write(lock, bytes).unwrap();
    let after = analyze(&tmp.0, "sample", &profile()).unwrap();
    assert_ne!(
        before.subject.snapshot_digest,
        after.subject.snapshot_digest
    );
    assert_eq!(
        legacy.subject.snapshot_digest,
        CargoWorkspaceAnalyzer
            .analyze(&tmp.0, "sample")
            .unwrap()
            .subject
            .snapshot_digest
    );
}

#[test]
fn declared_environment_change_invalidates_cli_identity() {
    let run = |flags: &str| {
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
            .env("RUSTFLAGS", flags)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .unwrap();
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        report["subject"]["snapshotDigest"].clone()
    };
    assert_ne!(run("--cfg snapshot_a"), run("--cfg snapshot_b"));
}

#[test]
fn canonical_inventory_is_order_independent_and_binds_every_input() {
    use archguard::analysis::snapshot::SnapshotInventory;
    let inputs: Vec<(String, Vec<u8>)> = [
        "Cargo.toml",
        "Cargo.lock",
        ".cargo/config.toml",
        "rust-toolchain.toml",
        "src/lib.rs",
        "identity:tool-version",
    ]
    .into_iter()
    .map(|n| (n.into(), b"original".to_vec()))
    .collect();
    let before = SnapshotInventory::from_inputs(inputs.clone())
        .unwrap()
        .digest();
    assert_eq!(
        before,
        SnapshotInventory::from_inputs(inputs.iter().rev().cloned())
            .unwrap()
            .digest()
    );
    for i in 0..inputs.len() {
        let mut changed = inputs.clone();
        changed[i].1.push(b'x');
        assert_ne!(
            before,
            SnapshotInventory::from_inputs(changed).unwrap().digest()
        );
    }
    assert!(
        SnapshotInventory::from_inputs([
            ("duplicate".into(), vec![]),
            ("duplicate".into(), vec![])
        ])
        .is_err()
    );
}

#[test]
fn legacy_manifest_digest_golden_is_preserved() {
    let facts = CargoWorkspaceAnalyzer
        .analyze(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/allowed"),
            "golden",
        )
        .unwrap();
    assert_eq!(
        facts.subject.snapshot_digest,
        "sha256:3e2be9ddf0b5165a3be9ce258c3d8ca4b1d65a7a456394108c7369c410417187"
    );
}
