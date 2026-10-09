use archguard::integration::{binding::GitCargoEvidence, projection::ProtectedCargoPolicy};
use gitguard::{
    Repository,
    candidate::{CandidateRequest, CandidateSnapshot},
    scope::TaskScope,
    subject::SubjectRequest,
};
use std::{path::Path, process::Command, sync::atomic::AtomicBool};
mod common;
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("/usr/bin/git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn fixture(format: &str) -> (common::Temp, String, String) {
    let dir = common::Temp::new();
    dir.copy_fixture("forbidden");
    git(
        dir.0.as_path(),
        &["init", "-q", &format!("--object-format={format}")],
    );
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "base"]);
    let base = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        dir.0.as_path().join("candidate-note.txt"),
        "exact candidate",
    )
    .unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "candidate"]);
    let candidate = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    (dir, base, candidate)
}
fn policy() -> ProtectedCargoPolicy {
    ProtectedCargoPolicy::freeze(common::evidence::contract(), vec![]).unwrap()
}
fn snapshot(
    repo: &Repository,
    base: &str,
    candidate: &str,
    p: &ProtectedCargoPolicy,
) -> CandidateSnapshot {
    let scope = TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"candidate-note.txt".to_vec()],
        p.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let subject = repo
        .resolve_subject(SubjectRequest::Commit(candidate.into()))
        .unwrap();
    repo.prepare_candidate(
        &subject,
        &scope,
        &CandidateRequest {
            worktree_id: "wt".into(),
            base_oid: base.into(),
            merge_group_id: Some("queue".into()),
            members: vec![base.into(), candidate.into()],
        },
    )
    .unwrap()
}
#[test]
fn actual_git_candidate_is_analyzed_instead_of_mutable_head_for_both_formats() {
    for format in ["sha1", "sha256"] {
        let (dir, base, candidate) = fixture(format);
        git(dir.0.as_path(), &["checkout", "-q", &base]);
        let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
        let p = policy();
        let s = snapshot(&repo, &base, &candidate, &p);
        let prepared = GitCargoEvidence::prepare(&repo, &s, p).unwrap();
        std::fs::write(dir.0.as_path().join("Cargo.toml"), "mutable poison").unwrap();
        let result = prepared.run(&AtomicBool::new(false)).unwrap();
        assert_eq!(result.cargo().envelope.binding.candidate_oid, candidate);
        assert_eq!(result.object_format(), format);
        assert_eq!(
            result.cargo().envelope.run_status,
            guardengine::integration::RunStatus::Completed
        );
        assert_eq!(
            result.cargo().envelope.decision,
            Some(guardengine::Decision::Block)
        );
    }
}
#[test]
fn nonexistent_candidate_wrong_repository_and_policy_never_bind() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let mut value = serde_json::to_value(&s).unwrap();
    value["candidate_oid"] = serde_json::json!("1".repeat(40));
    let bad = serde_json::from_value(value).unwrap();
    assert!(GitCargoEvidence::prepare(&repo, &bad, policy()).is_err());
    let other = Repository::discover(dir.0.as_path(), "other").unwrap();
    assert!(GitCargoEvidence::prepare(&other, &s, policy()).is_err());
    let wrong =
        ProtectedCargoPolicy::freeze(common::evidence::contract(), vec!["unrequired".into()])
            .unwrap();
    assert!(GitCargoEvidence::prepare(&repo, &s, wrong).is_err());
}

#[test]
fn exact_wrapper_rejects_old_queue_base_members_requirements_and_tampered_source() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    let serialized = serde_json::to_vec(&result).unwrap();
    archguard::integration::binding::GitEvidenceBundle::load(&serialized, &repo, &s).unwrap();
    for (key, value) in [
        ("base_oid", serde_json::json!(candidate)),
        ("merge_group_id", serde_json::json!("new-queue")),
        ("members", serde_json::json!([candidate])),
        ("requirement_ids", serde_json::json!(["OTHER"])),
    ] {
        let mut changed = serde_json::to_value(&s).unwrap();
        changed[key] = value;
        let changed: CandidateSnapshot = serde_json::from_value(changed).unwrap();
        changed.validate(&repo).unwrap();
        assert!(result.verify(&repo, &changed).is_err());
    }
    let mut damaged = serde_json::to_value(&result).unwrap();
    damaged["files_digest"] = serde_json::json!(format!("sha256:{}", "0".repeat(64)));
    assert!(
        archguard::integration::binding::GitEvidenceBundle::load(
            &serde_json::to_vec(&damaged).unwrap(),
            &repo,
            &s
        )
        .is_err()
    );
    damaged = serde_json::to_value(&result).unwrap();
    damaged["object_format"] = serde_json::json!("sha256");
    assert!(
        archguard::integration::binding::GitEvidenceBundle::load(
            &serde_json::to_vec(&damaged).unwrap(),
            &repo,
            &s
        )
        .is_err()
    );
}
#[test]
fn cancellation_retains_real_binding_and_null_decision() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(true))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    assert_eq!(
        result.cargo().envelope.run_status,
        guardengine::integration::RunStatus::Cancelled
    );
    assert!(result.cargo().envelope.decision.is_none());
    assert!(result.cargo().domain.is_none());
}
#[test]
fn ignored_candidate_files_and_nonregular_objects_do_not_disappear_from_snapshot() {
    for kind in ["target", "symlink"] {
        let (dir, base, _) = fixture("sha1");
        if kind == "target" {
            std::fs::create_dir(dir.0.join("target")).unwrap();
            std::fs::write(dir.0.join("target/input"), "tracked source").unwrap();
        } else {
            std::os::unix::fs::symlink("Cargo.toml", dir.0.join("manifest-link")).unwrap();
        }
        git(dir.0.as_path(), &["add", "-f", "."]);
        git(dir.0.as_path(), &["commit", "-qm", "unsupported candidate"]);
        let candidate = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
        let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
        let p = policy();
        let s = snapshot(&repo, &base, &candidate, &p);
        assert!(GitCargoEvidence::prepare(&repo, &s, p).is_err());
    }
}

#[test]
fn real_candidate_metadata_failure_is_bound_error_and_missing_scope_is_completed_partial() {
    let (dir, base, candidate) = fixture("sha1");
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = ProtectedCargoPolicy::freeze(
        common::evidence::contract(),
        vec!["missing-protected-member".into()],
    )
    .unwrap();
    let s = snapshot(&repo, &base, &candidate, &p);
    let partial = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    partial.verify(&repo, &s).unwrap();
    assert_eq!(
        partial.cargo().envelope.run_status,
        guardengine::integration::RunStatus::Completed
    );
    assert_eq!(
        partial.cargo().envelope.coverage.status,
        guardengine::integration::CoverageStatus::Partial
    );
    assert_eq!(
        partial.cargo().envelope.decision,
        Some(guardengine::Decision::Block)
    );
    std::fs::write(
        dir.0.join("Cargo.toml"),
        "[workspace]\nmembers=[\"missing-member\"]\n",
    )
    .unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "invalid metadata"]);
    let candidate = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let s = snapshot(&repo, &base, &candidate, &p);
    let failed = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    failed.verify(&repo, &s).unwrap();
    assert_eq!(
        failed.cargo().envelope.run_status,
        guardengine::integration::RunStatus::Error
    );
    assert!(failed.cargo().envelope.decision.is_none());
    assert!(failed.cargo().contract.is_none());
}

#[test]
fn actual_two_parent_queue_merge_differs_from_member_head() {
    let (dir, base, _) = fixture("sha1");
    git(dir.0.as_path(), &["checkout", "-q", &base]);
    std::fs::write(dir.0.join("member-a.txt"), "member a").unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "member a"]);
    let a = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    git(dir.0.as_path(), &["checkout", "-q", &base]);
    std::fs::write(dir.0.join("member-b.txt"), "member b").unwrap();
    git(dir.0.as_path(), &["add", "."]);
    git(dir.0.as_path(), &["commit", "-qm", "member b"]);
    let b = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    git(
        dir.0.as_path(),
        &["merge", "--no-ff", "-m", "synthetic queue candidate", &a],
    );
    let queue = git(dir.0.as_path(), &["rev-parse", "HEAD"]);
    git(dir.0.as_path(), &["checkout", "-q", &a]);
    let repo = Repository::discover(dir.0.as_path(), "repo").unwrap();
    let p = policy();
    let scope = TaskScope::advisory(
        "task",
        vec!["R".into()],
        vec![b"member-a.txt".to_vec(), b"member-b.txt".to_vec()],
        p.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let s = repo
        .prepare_candidate(
            &repo
                .resolve_subject(SubjectRequest::Commit(queue.clone()))
                .unwrap(),
            &scope,
            &CandidateRequest {
                worktree_id: "wt".into(),
                base_oid: base.clone(),
                merge_group_id: Some("queue".into()),
                members: vec![a.clone(), b],
            },
        )
        .unwrap();
    let result = GitCargoEvidence::prepare(&repo, &s, p)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    result.verify(&repo, &s).unwrap();
    let member = snapshot(&repo, &base, &a, &policy());
    assert!(result.verify(&repo, &member).is_err());
    assert_eq!(result.cargo().envelope.binding.candidate_oid, queue);
}
