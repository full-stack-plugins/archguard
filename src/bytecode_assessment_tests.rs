use super::{assess, content_digest, source_digest};
use crate::{bytecode_config::BytecodeConfig, bytecode_policy::BytecodePolicy};
use guard_protocol::{GitOid, GitPath, Identifier, ProtocolDocument};
use guard_subject::{CaptureBinding, CapturedSubject, InputEntry, Selection};
use serde_json::json;
fn fixture() -> (
    BytecodeConfig,
    BytecodePolicy,
    CapturedSubject,
    serde_json::Value,
) {
    let root = std::env::current_dir().unwrap();
    let file = root.join("java.exe");
    let config=BytecodeConfig::from_json(&serde_json::to_vec(&json!({"schemaVersion":1,"repositoryId":"repo-example","taskId":"REQ-1","worktreeId":"worktree-1","verifierId":"java-bytecode","javaExecutable":file,"classpath":[file],"mavenHome":root,"repository":root,"home":root,"temporaryDirectory":root,"toolArtifacts":[{"path":file,"sha256":content_digest(b"tools")}],"modules":[{"path":"","expectedTypes":[]},{"path":"domain","expectedTypes":["demo.domain.Order"]},{"path":"infra","expectedTypes":["demo.infra.SqlStore"]}],"environment":{"GIT_CEILING_DIRECTORIES":"{{subject_parent}}"},"budget":{"timeoutMs":30000,"maxOutputBytes":1048576,"maxConcurrency":1}})).unwrap()).unwrap();
    let policy=BytecodePolicy::from_local_json(br#"{"schemaVersion":1,"rules":[{"ruleId":"DOMAIN-NO-INFRA","from":"demo.domain","forbiddenTo":["demo.infra"]}]}"#).unwrap();
    let ProtocolDocument::SubjectSnapshot(snapshot) = guard_protocol::decode(include_bytes!(
        "../../guardengine/crates/guard-protocol/fixtures/valid/subject-worktree.json"
    ))
    .unwrap()
    .document()
    .clone() else {
        panic!("fixture")
    };
    let captured = CapturedSubject {
        snapshot,
        binding: CaptureBinding {
            repository_id: Identifier::new("repo-example").unwrap(),
            task_id: Identifier::new("REQ-1").unwrap(),
            worktree_id: Identifier::new("worktree-1").unwrap(),
            contract_digest: content_digest(b"contract"),
            policy_digest: content_digest(b"policy"),
            verifier_digest: content_digest(b"verifier"),
            environment_digest: content_digest(b"env"),
        },
        selection: Selection::Worktree {
            base: GitOid::new("1".repeat(40)).unwrap(),
        },
        manifest: vec![InputEntry {
            path: GitPath::from_bytes(b"domain/src/main/java/demo/domain/Order.java").unwrap(),
            mode: "100644".into(),
            object_id: None,
            content_digest: content_digest(b"package demo.domain; class Order {}"),
        }],
        identity_digest: content_digest(b"identity"),
    };
    let result = json!({"schemaVersion":1,"completeness":"incomplete","classCount":1,"violations":[{"ruleId":"DOMAIN-NO-INFRA","from":"demo.domain.Order","to":"demo.infra.SqlStore","file":"domain/src/main/java/demo/domain/Order.java","line":1,"description":"actual edge"}],"diagnostics":["later verifier rule unsupported"],"sourceDigest":source_digest(&captured).unwrap(),"bytecodeDigest":content_digest(b"actual compiled artifact"),"toolDigest":content_digest(b"tools"),"ruleDigest":content_digest(&serde_json::to_vec(policy.rules()).unwrap()),"adapterVersion":"archguard-bytecode/1","archunitVersion":"1.5.1","modules":["domain"]});
    (config, policy, captured, result)
}
#[test]
fn incomplete_unknown_rule_outside_source_and_wrong_digest_are_not_confirmed_findings() {
    let (config, policy, captured, result) = fixture();
    for key in ["ruleId", "file", "from"] {
        let mut forged = result.clone();
        forged["violations"][0][key] = json!("OUTSIDE");
        assert!(
            assess(
                &config,
                &policy,
                &captured,
                &serde_json::to_vec(&forged).unwrap(),
                &content_digest(b"tools")
            )
            .is_err(),
            "partial {key} must not bypass attribution"
        );
    }
    for key in ["sourceDigest", "toolDigest", "ruleDigest"] {
        let mut forged = result.clone();
        forged[key] = json!(content_digest(b"forged"));
        assert!(
            assess(
                &config,
                &policy,
                &captured,
                &serde_json::to_vec(&forged).unwrap(),
                &content_digest(b"tools")
            )
            .is_err(),
            "partial {key} must not bypass identity"
        );
    }
    let mut no_origin = result;
    no_origin["bytecodeDigest"] = serde_json::Value::Null;
    assert!(
        assess(
            &config,
            &policy,
            &captured,
            &serde_json::to_vec(&no_origin).unwrap(),
            &content_digest(b"tools")
        )
        .is_err()
    );
}
#[test]
fn incomplete_later_rule_failure_retains_previously_bound_positive_without_full_coverage() {
    let (config, policy, captured, result) = fixture();
    let accepted = assess(
        &config,
        &policy,
        &captured,
        &serde_json::to_vec(&result).unwrap(),
        &content_digest(b"tools"),
    )
    .unwrap();
    assert_eq!(accepted.completeness, "incomplete");
    assert_eq!(accepted.violations.len(), 1);
    assert_eq!(accepted.modules, ["domain"]);
}

#[test]
fn repair_scope_fixes_baseline_but_allows_candidate_commit_to_change() {
    let a = GitOid::new("1".repeat(40)).unwrap();
    let b = GitOid::new("2".repeat(40)).unwrap();
    assert_ne!(
        super::selection_scope_digest(&Selection::Worktree { base: a.clone() }).unwrap(),
        super::selection_scope_digest(&Selection::Worktree { base: b.clone() }).unwrap()
    );
    assert_ne!(
        super::selection_scope_digest(&Selection::Index { base: a.clone() }).unwrap(),
        super::selection_scope_digest(&Selection::Index { base: b.clone() }).unwrap()
    );
    assert_eq!(
        super::selection_scope_digest(&Selection::Commit {
            base: a.clone(),
            commit: a.clone()
        })
        .unwrap(),
        super::selection_scope_digest(&Selection::Commit {
            base: a.clone(),
            commit: b.clone()
        })
        .unwrap()
    );
    assert_ne!(
        super::selection_scope_digest(&Selection::Commit {
            base: a.clone(),
            commit: b.clone()
        })
        .unwrap(),
        super::selection_scope_digest(&Selection::Commit {
            base: b.clone(),
            commit: b
        })
        .unwrap()
    );
}

#[test]
fn non_main_java_is_an_explicit_capability_gap_not_complete_coverage() {
    let (config, _, mut captured, _) = fixture();
    assert!(super::ensure_supported_sources(&config, &captured).is_ok());
    captured.manifest[0].path =
        GitPath::from_bytes(b"domain/src/test/java/demo/domain/Order.java").unwrap();
    assert!(super::ensure_supported_sources(&config, &captured).is_err());
}
