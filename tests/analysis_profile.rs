use std::{path::Path, process::Command};

fn cli(project: &str, profile: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_archguard"))
        .args([
            "check",
            "--project",
            project,
            "--contract",
            "examples/agent-job-contract.yaml",
            "--profile",
            profile,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap()
}

#[test]
fn enhanced_profile_does_not_pass_unknown_required_member() {
    let output = cli("fixtures/allowed", "cargo-declarations-v1");
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let facts: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(facts["decision"], "BLOCK");
}

#[test]
fn unknown_profile_is_rejected_instead_of_running_legacy() {
    assert_eq!(cli("fixtures/allowed", "unknown").status.code(), Some(4));
}

#[test]
fn explicit_legacy_keeps_allowed_behavior() {
    assert_eq!(cli("fixtures/allowed", "legacy").status.code(), Some(0));
    assert!(Path::new(env!("CARGO_MANIFEST_DIR")).is_dir());
}

fn contract() -> guardengine::GuardContract {
    guardengine::load_contract_yaml(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/agent-job-contract.yaml"),
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn protected_scope_survives_candidate_contract_deletion_and_unknown_relations() {
    use archguard::analysis::profile::FrozenAnalysisProfile;
    let mut protected = contract();
    let profile = FrozenAnalysisProfile::freeze(&protected, ["extra-member".to_owned()]);
    protected.spec.rules.clear();
    let gaps = profile.coverage_gaps(&["agent-job".into(), "agent-contracts".into()].into());
    assert!(gaps.iter().any(|g| g.contains("agent-saas")));
    assert!(gaps.iter().any(|g| g.contains("extra-member")));
    let mut protected = contract();
    let guardengine::GuardAssertion::ForbidRelation { predicate, .. } =
        &mut protected.spec.rules[0].assertion;
    *predicate = "calls".into();
    let gaps = FrozenAnalysisProfile::freeze(&protected, [])
        .coverage_gaps(&["agent-job".into(), "agent-saas".into()].into());
    assert_eq!(gaps, ["unsupported relation: calls"]);
}

#[test]
fn declarations_preserve_rename_kind_optional_target_and_nonmember_provenance() {
    let observation = archguard::analysis::cargo::observe(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/profile"),
    )
    .unwrap();
    assert_eq!(
        observation.members,
        ["profile-app".into(), "profile-core".into()].into()
    );
    assert_eq!(observation.facts.len(), 1);
    assert_eq!(observation.facts[0].object, "profile-core");
    let core: Vec<_> = observation
        .declarations
        .iter()
        .filter(|d| d.package == "profile-core")
        .collect();
    assert_eq!(core.len(), 4);
    assert!(
        core.iter()
            .all(|d| d.rename.as_deref() == Some("renamed") && d.source == "app/Cargo.toml")
    );
    assert!(core.iter().any(|d| d.kind == "normal" && d.optional));
    assert!(core.iter().any(|d| d.kind == "dev"));
    assert!(core.iter().any(|d| d.kind == "build"));
    assert!(
        core.iter()
            .any(|d| d.target.as_deref() == Some("cfg(unix)"))
    );
    let external = observation
        .declarations
        .iter()
        .find(|d| d.package == "profile-external")
        .unwrap();
    assert_eq!(external.path.as_deref(), Some("external"));
    assert_eq!(external.member_target, None);
}

#[cfg(unix)]
#[test]
fn enhanced_analysis_rejects_external_dependency_before_reading_it() {
    use std::{fs, os::unix::fs::symlink};
    let base = std::env::temp_dir().join(format!("archguard-escape-{}", std::process::id()));
    fs::create_dir_all(base.join("inside/src")).unwrap();
    fs::create_dir_all(base.join("outside/src")).unwrap();
    fs::write(
        base.join("inside/Cargo.toml"),
        "[package]\nname='inside'\nversion='0.1.0'\n[dependencies]\noutside={path='../outside'}\n",
    )
    .unwrap();
    fs::write(base.join("inside/src/lib.rs"), "").unwrap();
    fs::write(
        base.join("outside/Cargo.toml"),
        "[package]\nname='outside'\nversion='0.1.0'\n",
    )
    .unwrap();
    fs::write(base.join("outside/src/lib.rs"), "").unwrap();
    let profile = archguard::analysis::profile::FrozenAnalysisProfile::freeze(&contract(), []);
    let result = archguard::analysis::analyze(&base.join("inside"), "escape", &profile);
    assert!(
        result.is_err(),
        "external path must be rejected before Cargo is run"
    );
    symlink(base.join("outside"), base.join("inside/link")).unwrap();
    assert!(archguard::analysis::analyze(&base.join("inside"), "escape", &profile).is_err());
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn enhanced_profile_allows_complete_scope_with_no_forbidden_edge() {
    let mut protected = contract();
    let guardengine::GuardAssertion::ForbidRelation {
        subject, object, ..
    } = &mut protected.spec.rules[0].assertion;
    *subject = "agent-saas".into();
    *object = "agent-job".into();
    let profile = archguard::analysis::profile::FrozenAnalysisProfile::freeze(&protected, []);
    let facts = archguard::analysis::analyze(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/forbidden"),
        "complete",
        &profile,
    )
    .unwrap();
    assert_eq!(facts.completeness, guardengine::Completeness::Complete);
    assert_eq!(
        guardengine::evaluate(&protected, &facts).unwrap().decision,
        guardengine::Decision::Allow
    );
}
