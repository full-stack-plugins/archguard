use archguard::integration::projection::CandidateContext;
use std::{path::Path, process::Command};
pub fn context(root: &Path) -> CandidateContext {
    let git = |args: &[&str]| {
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
            .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z")
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "base"]);
    let base = git(&["rev-parse", "HEAD"]);
    std::fs::write(root.join("candidate-note.txt"), "candidate fixture\n").unwrap();
    git(&["add", "candidate-note.txt"]);
    git(&["commit", "-qm", "candidate"]);
    CandidateContext {
        repo_id: "controller-repo".into(),
        task_id: "task".into(),
        worktree_id: "worktree".into(),
        requirement_ids: vec!["R".into()],
        candidate_oid: git(&["rev-parse", "HEAD"]),
        base_oid: base,
        merge_group_id: None,
    }
}
pub fn contract() -> guardengine::GuardContract {
    guardengine::load_contract_yaml(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/agent-job-contract.yaml"),
        )
        .unwrap(),
    )
    .unwrap()
}
