#![cfg(feature = "bytecode-java-tests")]
#[path = "bytecode_support/mod.rs"]
mod support;
use archguard::{
    bytecode_config::BytecodeConfig, bytecode_policy::BytecodePolicy, bytecode_service,
};
use guard_protocol::GitOid;
use guard_runner::Cancellation;
use guard_store::{EvidenceStore, StoreLimits};
use guard_subject::Selection;
use std::{
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn copy(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let path = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &path)
        } else {
            std::fs::copy(entry.path(), path).unwrap();
        }
    }
}
#[test]
fn real_commit_ignores_dirty_worktree_then_worktree_violation_fix_and_verify() {
    let fixture = tempfile::Builder::new()
        .prefix("a01-java-")
        .tempdir_in(std::env::temp_dir())
        .unwrap();
    let project = fixture.path().join("project");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("java-adapter/src/test/resources/reactor"),
        &project,
    );
    git(&project, &["init"]);
    git(&project, &["config", "user.name", "Fixture"]);
    git(
        &project,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&project, &["add", "."]);
    git(&project, &["commit", "-m", "fixture"]);
    let oid = GitOid::new(git(&project, &["rev-parse", "HEAD"])).unwrap();
    let config = BytecodeConfig::from_json(
        &std::fs::read(
            std::env::var("ARCHGUARD_BYTECODE_TEST_CONFIG")
                .expect("explicit Java tool configuration required"),
        )
        .unwrap(),
    )
    .unwrap();
    let store =
        EvidenceStore::create(&fixture.path().join("store"), StoreLimits::default()).unwrap();
    let config_file = fixture.path().join("config.json");
    std::fs::write(&config_file, serde_json::to_vec(&config).unwrap()).unwrap();
    let policy_file = fixture.path().join("policy.json");
    std::fs::write(&policy_file,br#"{"schemaVersion":1,"rules":[{"ruleId":"DOMAIN-NO-INFRA","from":"demo.domain","forbiddenTo":["demo.infra"]}]}"#).unwrap();
    let invoke = |action: &str, subject: &str, operation: &str, extra: &[String]| {
        let mut args = vec![
            "bytecode".into(),
            action.into(),
            "--project".into(),
            project.to_str().unwrap().into(),
            "--scratch".into(),
            fixture.path().to_str().unwrap().into(),
            "--store".into(),
            fixture.path().join("store").to_str().unwrap().into(),
            "--config".into(),
            config_file.to_str().unwrap().into(),
            "--policy".into(),
            policy_file.to_str().unwrap().into(),
            "--subject".into(),
            subject.into(),
            "--base".into(),
            oid.as_str().into(),
            "--operation".into(),
            operation.into(),
        ];
        args.extend_from_slice(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_archguard"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            matches!(output.status.code(), Some(0 | 2)),
            "CLI {:?}: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        let receipt = archguard::bytecode_receipt::BytecodeReceipt::from_json(
            &output.stdout,
            &store,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
        .unwrap();
        (output.status.code(), receipt)
    };
    let source = project.join("domain/src/main/java/demo/domain/Order.java");
    let allowed = std::fs::read(&source).unwrap();
    std::fs::write(&source,"package demo.domain; public class Order { public String id() { return new demo.infra.SqlStore().load(); } }").unwrap();
    let (code, committed) = invoke(
        "check",
        "commit",
        "commit",
        &["--commit".into(), oid.as_str().into()],
    );
    assert_eq!(code, Some(0));
    assert_eq!(committed.report.status, "passed", "{:?}", committed.report);
    assert!(!project.join("domain/target").exists());
    let (code, forbidden) = invoke("check", "worktree", "violation", &[]);
    assert_eq!(code, Some(2));
    assert_eq!(
        forbidden.report.status, "violations",
        "{:?}",
        forbidden.report
    );
    assert!(
        !forbidden
            .report
            .result
            .as_ref()
            .unwrap()
            .violations
            .is_empty()
    );
    assert!(forbidden.report.approval.is_none());
    assert_eq!(forbidden.report.trust, "local_unverified");
    let mut tampered = forbidden.clone();
    tampered.report.status = "passed".into();
    assert!(
        tampered
            .verify(
                &store,
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
            )
            .is_err()
    );
    git(&project, &["commit", "--allow-empty", "-m", "new baseline"]);
    let different_base = GitOid::new(git(&project, &["rev-parse", "HEAD"])).unwrap();
    let cancelled = Cancellation::new();
    cancelled.cancel();
    let policy = BytecodePolicy::from_local_json(&std::fs::read(&policy_file).unwrap()).unwrap();
    let retain = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 86400000;
    let wrong_base = bytecode_service::verify_local(
        &forbidden,
        &project,
        fixture.path(),
        &config,
        &policy,
        &Selection::Worktree {
            base: different_base,
        },
        &store,
        "wrong-base",
        retain,
        cancelled,
    );
    assert!(
        wrong_base.is_err(),
        "changing baseline must reject before starting a new repair observation"
    );
    std::fs::write(&source, allowed).unwrap();
    let previous = fixture.path().join("previous.json");
    std::fs::write(&previous, serde_json::to_vec(&forbidden).unwrap()).unwrap();
    let (code, fixed) = invoke(
        "verify",
        "worktree",
        "fixed",
        &["--previous".into(), previous.to_str().unwrap().into()],
    );
    assert_eq!(code, Some(0));
    assert_eq!(fixed.report.status, "passed", "{:?}", fixed.report);
    assert!(fixed.report.previous_report.is_some());
    assert_ne!(fixed.report.subject_digest, forbidden.report.subject_digest);
    assert!(!project.join("domain/target").exists());
}

#[test]
fn registered_contract_plan_runtime_real_java_and_idempotency() {
    let fixture = tempfile::Builder::new()
        .prefix("a01-registered-")
        .tempdir_in(std::env::temp_dir())
        .unwrap();
    let project = fixture.path().join("project");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("java-adapter/src/test/resources/reactor"),
        &project,
    );
    git(&project, &["init"]);
    git(&project, &["config", "user.name", "Fixture"]);
    git(
        &project,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&project, &["add", "."]);
    git(&project, &["commit", "-m", "fixture"]);
    let mut config = BytecodeConfig::from_json(
        &std::fs::read(std::env::var("ARCHGUARD_BYTECODE_TEST_CONFIG").unwrap()).unwrap(),
    )
    .unwrap();
    config.budget.timeout_ms = 120000;
    let policy=BytecodePolicy::from_local_json(br#"{"schemaVersion":1,"rules":[{"ruleId":"DOMAIN-NO-INFRA","from":"demo.domain","forbiddenTo":["demo.infra"]}]}"#).unwrap();
    let store = std::sync::Arc::new(
        EvidenceStore::create(&fixture.path().join("store"), StoreLimits::default()).unwrap(),
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let retain = now + 86400000;
    let selection = Selection::Commit {
        base: GitOid::new(git(&project, &["rev-parse", "HEAD"])).unwrap(),
        commit: GitOid::new(git(&project, &["rev-parse", "HEAD"])).unwrap(),
    };
    let (contract, plan) =
        support::prepare(&project, &config, &policy, &selection, &store, retain, now);
    // 原仓dirty内容不可污染Commit候选实际执行；完整Java pipeline必须只读私有副本。
    std::fs::write(
        project.join("domain/src/main/java/demo/domain/Order.java"),
        "THIS IS NOT JAVA",
    )
    .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let obligation = guard_protocol::Identifier::new("OB-1").unwrap();
    let pin = contract
        .verifiers()
        .iter()
        .find(|p| p.verifier_id == config.verifier_id)
        .unwrap();
    let context = archguard::bytecode_projection_context::BytecodeProjectionContext {
        producer: guard_protocol::ProducerIdentity {
            schema_version: guard_protocol::SchemaVersion::new("1.0.0").unwrap(),
            producer_id: guard_protocol::Identifier::new("archguard").unwrap(),
            version: pin.version.clone(),
            artifact_digest: pin.artifact_digest.clone(),
            config_digest: pin.config_digest.clone(),
            adapter_version: pin.adapter_version.clone(),
            execution_environment_digest: plan.capture_binding().environment_digest.clone(),
            extensions: guard_protocol::Extensions::default(),
        },
        result_id: guard_protocol::Identifier::new("RESULT-A01").unwrap(),
        run_id: guard_protocol::Identifier::new("RUN-A01").unwrap(),
        operation_id: guard_protocol::Identifier::new("registered").unwrap(),
        lineage_id: guard_protocol::Identifier::new("LINE-A01").unwrap(),
        retention_policy_id: guard_protocol::Identifier::new("private-1d").unwrap(),
    };
    let result = executor
        .block_on(archguard::bytecode_projection::check_registered_result(
            &project,
            fixture.path(),
            &config,
            &contract,
            &plan,
            &obligation,
            &context,
            store.clone(),
            "registered",
            retain,
            Cancellation::new(),
        ))
        .unwrap();
    let archguard::bytecode_public_outcome::BytecodePublicOutcome::Finished(public) = result else {
        panic!("expected real first execution")
    };
    assert_eq!(
        public.result.assessment.verdict,
        guard_protocol::Verdict::Satisfied
    );
    assert_eq!(
        public.result.assessment.coverage,
        guard_protocol::Coverage::Complete
    );
    assert_eq!(public.result.rule_id.as_str(), "DOMAIN-NO-INFRA");
    assert_eq!(public.result.mode, contract.rules()[0].mode);
    assert!(
        public
            .result
            .limitations
            .iter()
            .any(|v| v.as_str() == "UNVERIFIED")
    );
    assert!(
        public
            .result
            .evidence_refs
            .iter()
            .all(|v| v.classification == guard_protocol::Classification::Private)
    );
    assert_eq!(
        public.result.execution.started_at,
        public.receipt.report.started_at
    );
    assert_eq!(public.result.execution.sequence.as_str(), "1");
    let encoded = store
        .read_private(
            &public.artifact.operation,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
        .unwrap();
    assert!(matches!(
        guard_protocol::decode(&encoded).unwrap().document(),
        guard_protocol::ProtocolDocument::GuardResult(_)
    ));
    let mut false_producer = context.clone();
    false_producer.producer.artifact_digest =
        archguard::bytecode_pipeline_request::content_digest(b"wrong producer");
    assert!(
        archguard::bytecode_projection::project_registered_result(
            &project,
            &public.receipt,
            &store,
            &contract,
            &plan,
            &obligation,
            &false_producer
        )
        .is_err()
    );
    let receipt = public.receipt;
    assert_eq!(receipt.report.status, "passed", "{:?}", receipt.report);
    assert!(receipt.report.execution_observation.is_some());
    assert!(receipt.report.approval.is_none());
    receipt
        .verify(
            &store,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
        .unwrap();
    let again = executor
        .block_on(bytecode_service::check_registered(
            &project,
            fixture.path(),
            &config,
            &contract,
            &plan,
            &obligation,
            store.clone(),
            "registered",
            retain,
            Cancellation::new(),
        ))
        .unwrap_err();
    assert!(again.contains("already terminal"));
    assert!(!project.join("domain/target").exists());
    let mut drift = config.clone();
    drift
        .environment
        .insert("UNAPPROVED".into(), "value".into());
    assert!(
        executor
            .block_on(bytecode_service::check_registered(
                &project,
                fixture.path(),
                &drift,
                &contract,
                &plan,
                &obligation,
                store,
                "wrong-config",
                retain,
                Cancellation::new()
            ))
            .is_err()
    );
}
