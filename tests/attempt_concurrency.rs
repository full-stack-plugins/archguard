use archguard::integration::{
    binding::attempts::{AttemptHistory, PreparedGitAttempt},
    projection::ProtectedCargoPolicy,
};
use gitguard::{
    Repository,
    candidate::{CandidateRequest, CandidateSnapshot},
    scope::TaskScope,
    subject::SubjectRequest,
};
use std::{path::Path, process::Command, sync::atomic::AtomicBool};
mod common;
fn independent_binding(
    repo: &Repository,
    candidate: &CandidateSnapshot,
) -> guardengine::integration::RunBinding {
    PreparedGitAttempt::prepare(repo, candidate, policy())
        .unwrap()
        .binding()
        .clone()
}
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
fn policy() -> ProtectedCargoPolicy {
    ProtectedCargoPolicy::freeze(common::evidence::contract(), vec![]).unwrap()
}
fn fixture(format: &str) -> (common::Temp, String, String) {
    let dir = common::Temp::new();
    dir.copy_fixture("forbidden");
    std::fs::write(dir.0.join("agent-job/Cargo.toml"), "[package]\nname=\"agent-job\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[dependencies]\nagent-contracts={path=\"../agent-contracts\"}\n").unwrap();
    git(
        &dir.0,
        &["init", "-q", &format!("--object-format={format}")],
    );
    git(&dir.0, &["add", "."]);
    git(&dir.0, &["commit", "-qm", "allowed"]);
    let old = git(&dir.0, &["rev-parse", "HEAD"]);
    std::fs::write(dir.0.join("agent-job/Cargo.toml"), "[package]\nname=\"agent-job\"\nversion=\"0.1.0\"\nedition=\"2024\"\n[dependencies]\nagent-contracts={path=\"../agent-contracts\"}\nagent-saas={path=\"../agent-saas\"}\n").unwrap();
    git(&dir.0, &["add", "."]);
    git(&dir.0, &["commit", "-qm", "forbidden"]);
    let new = git(&dir.0, &["rev-parse", "HEAD"]);
    (dir, old, new)
}
fn snapshot(repo: &Repository, base: &str, oid: &str, req: &str, wt: &str) -> CandidateSnapshot {
    snapshot_policy(repo, base, oid, req, wt, &policy())
}
fn snapshot_policy(
    repo: &Repository,
    base: &str,
    oid: &str,
    req: &str,
    wt: &str,
    p: &ProtectedCargoPolicy,
) -> CandidateSnapshot {
    let scope = TaskScope::advisory(
        "task",
        vec![req.into()],
        vec![b"agent-job/Cargo.toml".to_vec()],
        p.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let subject = repo
        .resolve_subject(SubjectRequest::Commit(oid.into()))
        .unwrap();
    repo.prepare_candidate(
        &subject,
        &scope,
        &CandidateRequest {
            worktree_id: wt.into(),
            base_oid: base.into(),
            merge_group_id: Some("queue".into()),
            members: vec![oid.into()],
        },
    )
    .unwrap()
}
#[test]
fn late_old_allow_cannot_replace_new_block_for_actual_git_candidates() {
    for format in ["sha1", "sha256"] {
        let (dir, old, new) = fixture(format);
        let repo = Repository::discover(&dir.0, "repo").unwrap();
        let a = snapshot(&repo, &old, &old, "R", "wt");
        let b = snapshot(&repo, &old, &new, "R", "wt");
        let mut history = AttemptHistory::new();
        let old_run = history
            .register(PreparedGitAttempt::prepare(&repo, &a, policy()).unwrap(), 0)
            .unwrap();
        let new_run = history
            .register(PreparedGitAttempt::prepare(&repo, &b, policy()).unwrap(), 1)
            .unwrap();
        assert_ne!(old_run.run_id(), new_run.run_id());
        let new_result = new_run.run(&AtomicBool::new(false)).unwrap();
        assert_eq!(
            new_result.bundle().cargo().envelope.decision,
            Some(guardengine::Decision::Block)
        );
        history.import(&new_result, &repo, &b).unwrap();
        history.publish(&new_result, &repo, &b).unwrap();
        let old_result = old_run.run(&AtomicBool::new(false)).unwrap();
        assert_eq!(
            old_result.bundle().cargo().envelope.decision,
            Some(guardengine::Decision::Allow)
        );
        history.import(&old_result, &repo, &a).unwrap();
        assert!(history.publish(&old_result, &repo, &a).is_err());
        assert!(
            history
                .current_matches(
                    &new_result,
                    &repo,
                    &b,
                    &policy(),
                    &independent_binding(&repo, &b)
                )
                .unwrap()
        );
        assert!(
            !history
                .current_matches(
                    &old_result,
                    &repo,
                    &a,
                    &policy(),
                    &independent_binding(&repo, &a)
                )
                .unwrap()
        );
        assert!(
            history
                .current_matches(
                    &new_result,
                    &repo,
                    &a,
                    &policy(),
                    &independent_binding(&repo, &b)
                )
                .is_err()
        );
    }
}

#[test]
fn four_actual_runs_finish_concurrently_without_cross_requirement_or_worktree_reuse() {
    use std::sync::{Barrier, Mutex};
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let history = Mutex::new(AttemptHistory::new());
    let barrier = Barrier::new(4);
    let mut inputs = Vec::new();
    for req in ["R1", "R2"] {
        for wt in ["wt1", "wt2"] {
            let s = snapshot(&repo, &old, &new, req, wt);
            let prepared = PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap();
            let key = prepared.dedup_key().to_owned();
            inputs.push((
                s,
                history.lock().unwrap().register(prepared, 0).unwrap(),
                key,
            ));
        }
    }
    assert_eq!(
        inputs
            .iter()
            .map(|(_, _, k)| k)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    let results = std::thread::scope(|scope| {
        let handles = inputs
            .into_iter()
            .map(|(s, run, key)| {
                let h = &history;
                let b = &barrier;
                let r = &repo;
                scope.spawn(move || {
                    b.wait();
                    let result = run.run(&AtomicBool::new(false)).unwrap();
                    let mut history = h.lock().unwrap();
                    history.import(&result, r, &s).unwrap();
                    history.publish(&result, r, &s).unwrap();
                    (s, result, key)
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    let h = history.lock().unwrap();
    for (s, result, _) in &results {
        assert_eq!(result.generation(), 1);
        assert_eq!(h.history(result).unwrap().len(), 1);
        assert!(!h.history(result).unwrap()[0].eligible);
        assert!(
            h.current_matches(result, &repo, s, &policy(), &independent_binding(&repo, s))
                .unwrap()
        );
        for (other, _, _) in &results {
            if s.binding_digest() != other.binding_digest() {
                assert!(
                    h.current_matches(
                        result,
                        &repo,
                        other,
                        &policy(),
                        &independent_binding(&repo, s)
                    )
                    .is_err()
                );
            }
        }
    }
}
#[test]
fn mutex_cas_has_exactly_one_winner_for_the_same_generation() {
    use std::sync::{Barrier, Mutex};
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let first = PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap();
    let second = PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap();
    assert_ne!(first.run_id(), second.run_id());
    assert_eq!(first.dedup_key(), second.dedup_key());
    let h = Mutex::new(AttemptHistory::new());
    let barrier = Barrier::new(2);
    let wins = std::thread::scope(|scope| {
        let handles = [first, second]
            .into_iter()
            .map(|p| {
                let h = &h;
                let b = &barrier;
                scope.spawn(move || {
                    b.wait();
                    h.lock().unwrap().register(p, 0).is_ok()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>()
    });
    assert_eq!(wins, 1);
}
#[test]
fn replay_is_idempotent_registration_clears_current_and_foreign_history_cannot_import() {
    use archguard::integration::binding::attempts::AppendOutcome;
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let mut h = AttemptHistory::new();
    let result = h
        .register(PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(), 0)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    let original = serde_json::to_vec(result.bundle()).unwrap();
    assert!(h.publish(&result, &repo, &s).is_err());
    assert_eq!(
        h.import(&result, &repo, &s).unwrap(),
        AppendOutcome::Inserted
    );
    assert_eq!(
        h.import(&result, &repo, &s).unwrap(),
        AppendOutcome::IdenticalReplay
    );
    h.publish(&result, &repo, &s).unwrap();
    assert!(
        h.register(PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(), 0)
            .is_err()
    );
    assert!(
        h.current_matches(
            &result,
            &repo,
            &s,
            &policy(),
            &independent_binding(&repo, &s)
        )
        .unwrap()
    );
    let next = h
        .register(PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(), 1)
        .unwrap();
    assert!(
        !h.current_matches(
            &result,
            &repo,
            &s,
            &policy(),
            &independent_binding(&repo, &s)
        )
        .unwrap()
    );
    assert!(h.publish(&result, &repo, &s).is_err());
    let cancelled = next.run(&AtomicBool::new(true)).unwrap();
    assert_eq!(
        cancelled.bundle().cargo().envelope.run_status,
        guardengine::integration::RunStatus::Cancelled
    );
    h.import(&cancelled, &repo, &s).unwrap();
    h.publish(&cancelled, &repo, &s).unwrap();
    assert!(
        h.current_matches(
            &cancelled,
            &repo,
            &s,
            &policy(),
            &independent_binding(&repo, &s)
        )
        .unwrap()
    );
    assert_eq!(cancelled.bundle().cargo().envelope.decision, None);
    assert!(h.history(&cancelled).unwrap().iter().all(|r| !r.eligible));
    let mut other = AttemptHistory::new();
    assert!(other.import(&result, &repo, &s).is_err());
    assert!(other.publish(&result, &repo, &s).is_err());
    assert!(other.history(&result).is_err());
    assert_eq!(serde_json::to_vec(result.bundle()).unwrap(), original);
}
#[test]
fn independent_candidate_fields_and_policy_cannot_reuse_current_result() {
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let mut h = AttemptHistory::new();
    let result = h
        .register(PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(), 0)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    h.import(&result, &repo, &s).unwrap();
    h.publish(&result, &repo, &s).unwrap();
    for (key, v) in [
        ("repo_id", serde_json::json!("other")),
        ("task_id", serde_json::json!("other")),
        ("worktree_id", serde_json::json!("other")),
        ("requirement_ids", serde_json::json!(["OTHER"])),
        ("candidate_oid", serde_json::json!(old)),
        ("base_oid", serde_json::json!(new)),
        ("merge_group_id", serde_json::json!("another-queue")),
        ("members", serde_json::json!([old])),
        ("policy_digest", serde_json::json!("a".repeat(64))),
        ("source_snapshot_digest", serde_json::json!("b".repeat(64))),
    ] {
        let mut changed = serde_json::to_value(&s).unwrap();
        changed[key] = v;
        let candidate: CandidateSnapshot = serde_json::from_value(changed).unwrap();
        assert!(
            h.current_matches(
                &result,
                &repo,
                &candidate,
                &policy(),
                &independent_binding(&repo, &s)
            )
            .is_err(),
            "{key}"
        );
    }
    let other = ProtectedCargoPolicy::freeze(
        common::evidence::contract(),
        vec!["missing-required-member".into()],
    )
    .unwrap();
    assert!(
        h.current_matches(&result, &repo, &s, &other, &independent_binding(&repo, &s))
            .is_err()
    );
    assert!(PreparedGitAttempt::prepare(&repo, &s, other).is_err());
}
#[test]
fn config_source_change_invalidates_full_work_even_when_dependencies_are_identical() {
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let first = PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap();
    let manifest = std::fs::read_to_string(dir.0.join("Cargo.toml")).unwrap();
    std::fs::write(
        dir.0.join("Cargo.toml"),
        format!("{manifest}\n[workspace.metadata.controller]\nconfig = \"changed\"\n"),
    )
    .unwrap();
    git(&dir.0, &["add", "."]);
    git(&dir.0, &["commit", "-qm", "config"]);
    let changed = git(&dir.0, &["rev-parse", "HEAD"]);
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let after = snapshot(&repo, &old, &changed, "R", "wt");
    let second = PreparedGitAttempt::prepare(&repo, &after, policy()).unwrap();
    assert_ne!(
        first.binding().source_snapshot_digest,
        second.binding().source_snapshot_digest
    );
    assert_ne!(first.dedup_key(), second.dedup_key());
}

struct Observer;
thread_local! { static TRACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; static MAX_ALLOC: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
fn observe(size: usize) {
    let _ = TRACK.try_with(|on| {
        if on.get() {
            let _ = MAX_ALLOC.try_with(|n| n.set(n.get().max(size)));
        }
    });
}
unsafe impl std::alloc::GlobalAlloc for Observer {
    unsafe fn alloc(&self, l: std::alloc::Layout) -> *mut u8 {
        observe(l.size());
        unsafe { std::alloc::GlobalAlloc::alloc(&std::alloc::System, l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: std::alloc::Layout) {
        unsafe { std::alloc::GlobalAlloc::dealloc(&std::alloc::System, p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: std::alloc::Layout, n: usize) -> *mut u8 {
        observe(n);
        unsafe { std::alloc::GlobalAlloc::realloc(&std::alloc::System, p, l, n) }
    }
}
#[global_allocator]
static ALLOCATOR: Observer = Observer;
#[test]
fn oversized_candidate_rejects_before_any_large_allocation_or_source_preparation() {
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let mut wire = serde_json::to_value(&s).unwrap();
    wire["task_id"] = serde_json::json!("x".repeat(17 * 1024 * 1024));
    let enormous: CandidateSnapshot = serde_json::from_value(wire).unwrap();
    let p = policy();
    MAX_ALLOC.with(|n| n.set(0));
    TRACK.with(|v| v.set(true));
    let result = PreparedGitAttempt::prepare(&repo, &enormous, p);
    TRACK.with(|v| v.set(false));
    assert!(result.is_err());
    assert!(MAX_ALLOC.with(|n| n.get()) < 4096);
    let mut wire = serde_json::to_value(&s).unwrap();
    wire["requirement_ids"] =
        serde_json::json!((0..65).map(|i| format!("R{i:02}")).collect::<Vec<_>>());
    assert!(
        PreparedGitAttempt::prepare(&repo, &serde_json::from_value(wire).unwrap(), policy())
            .is_err()
    );
}
#[test]
fn observation_error_can_be_current_metadata_but_never_an_eligible_record() {
    let (dir, old, _) = fixture("sha1");
    std::fs::write(
        dir.0.join("agent-contracts/Cargo.toml"),
        "[package]\nname=\"agent-saas\"\nversion=\"0.1.0\"\nedition=\"2024\"\n",
    )
    .unwrap();
    git(&dir.0, &["add", "."]);
    git(&dir.0, &["commit", "-qm", "duplicate package names"]);
    let oid = git(&dir.0, &["rev-parse", "HEAD"]);
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &oid, "R", "wt");
    let mut h = AttemptHistory::new();
    let result = h
        .register(PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(), 0)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        result.bundle().cargo().envelope.run_status,
        guardengine::integration::RunStatus::Error
    );
    assert_eq!(result.bundle().cargo().envelope.decision, None);
    h.import(&result, &repo, &s).unwrap();
    h.publish(&result, &repo, &s).unwrap();
    assert!(
        h.current_matches(
            &result,
            &repo,
            &s,
            &policy(),
            &independent_binding(&repo, &s)
        )
        .unwrap()
    );
    assert!(!h.history(&result).unwrap()[0].eligible);
}
#[test]
fn bounded_history_rejects_registration_257_without_replacing_current() {
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let mut h = AttemptHistory::new();
    for generation in 0..255 {
        h.register(
            PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(),
            generation,
        )
        .unwrap();
    }
    let last = h
        .register(
            PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(),
            255,
        )
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    h.import(&last, &repo, &s).unwrap();
    h.publish(&last, &repo, &s).unwrap();
    assert!(
        h.register(
            PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(),
            256
        )
        .is_err()
    );
    assert!(
        h.current_matches(&last, &repo, &s, &policy(), &independent_binding(&repo, &s))
            .unwrap()
    );
    assert_eq!(last.generation(), 256);
}
#[test]
fn changed_required_coverage_cannot_reuse_extraction_for_the_same_tree() {
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let original = snapshot(&repo, &old, &new, "R", "wt");
    let original = PreparedGitAttempt::prepare(&repo, &original, policy()).unwrap();
    let extended = ProtectedCargoPolicy::freeze(
        common::evidence::contract(),
        vec!["missing-obligation".into()],
    )
    .unwrap();
    let s = snapshot_policy(&repo, &old, &new, "R", "wt", &extended);
    let prepared = PreparedGitAttempt::prepare(&repo, &s, extended).unwrap();
    assert_ne!(original.dedup_key(), prepared.dedup_key());
    let expected_binding = prepared.binding().clone();
    let mut h = AttemptHistory::new();
    let result = h
        .register(prepared, 0)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        result.bundle().cargo().envelope.coverage.status,
        guardengine::integration::CoverageStatus::Partial
    );
    assert_eq!(
        result.bundle().cargo().envelope.decision,
        Some(guardengine::Decision::Block)
    );
    h.import(&result, &repo, &s).unwrap();
    h.publish(&result, &repo, &s).unwrap();
    assert!(
        h.current_matches(&result, &repo, &s, &policy(), &expected_binding)
            .is_err()
    );
    assert!(!h.history(&result).unwrap()[0].eligible);
}
#[test]
fn independently_expected_full_binding_rejects_source_tool_and_baseline_drift() {
    let (dir, old, new) = fixture("sha1");
    let repo = Repository::discover(&dir.0, "repo").unwrap();
    let s = snapshot(&repo, &old, &new, "R", "wt");
    let expected = independent_binding(&repo, &s);
    let mut h = AttemptHistory::new();
    let result = h
        .register(PreparedGitAttempt::prepare(&repo, &s, policy()).unwrap(), 0)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    h.import(&result, &repo, &s).unwrap();
    h.publish(&result, &repo, &s).unwrap();
    for (key, v) in [
        ("repoId", serde_json::json!("other")),
        ("taskId", serde_json::json!("other")),
        ("worktreeId", serde_json::json!("other")),
        ("requirementIds", serde_json::json!(["OTHER"])),
        ("candidateOid", serde_json::json!(old)),
        ("baseOid", serde_json::json!(new)),
        ("mergeGroupId", serde_json::json!("other")),
        (
            "sourceSnapshotDigest",
            serde_json::json!(format!("sha256:{}", "a".repeat(64))),
        ),
        (
            "baselineDigest",
            serde_json::json!(format!("sha256:{}", "b".repeat(64))),
        ),
    ] {
        let mut wire = serde_json::to_value(&expected).unwrap();
        assert!(wire.get(key).is_some(), "{key}");
        wire[key] = v;
        let changed = serde_json::from_value(wire).unwrap();
        assert!(
            h.current_matches(&result, &repo, &s, &policy(), &changed)
                .is_err(),
            "{key}"
        );
    }
    let mut huge = expected.clone();
    huge.source_snapshot_digest = "x".repeat(17 * 1024 * 1024);
    let protected = policy();
    MAX_ALLOC.with(|n| n.set(0));
    TRACK.with(|v| v.set(true));
    let rejected = h.current_matches(&result, &repo, &s, &protected, &huge);
    TRACK.with(|v| v.set(false));
    assert!(rejected.is_err());
    assert!(MAX_ALLOC.with(|n| n.get()) < 4096);
    assert!(
        h.current_matches(&result, &repo, &s, &policy(), &expected)
            .unwrap()
    );
}
