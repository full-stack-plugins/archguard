//! Export an actual Cargo producer run over real temporary Git objects.
use archguard::integration::{binding::GitCargoEvidence, projection::ProtectedCargoPolicy};
use gitguard::{
    Repository, candidate::CandidateRequest, scope::TaskScope, subject::SubjectRequest,
};
use std::{path::Path, process::Command, sync::atomic::AtomicBool};
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
        .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z")
        .output()
        .unwrap();
    assert!(out.status.success(), "Git fixture setup failed");
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_name() == "target" || entry.file_name() == "Cargo.lock" {
            continue;
        }
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}
fn main() {
    let output = std::env::args_os()
        .nth(1)
        .expect("usage: export_git_evidence NEW_DIRECTORY");
    let output = Path::new(&output);
    std::fs::create_dir(output).expect("output must be new");
    let output = output.canonicalize().unwrap();
    let root = tempfile::tempdir().unwrap();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy(&repository.join("fixtures/forbidden"), root.path());
    git(root.path(), &["init", "-q"]);
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "base"]);
    let base = git(root.path(), &["rev-parse", "HEAD"]);
    std::fs::write(
        root.path().join("candidate-note.txt"),
        "actual Cargo producer fixture\n",
    )
    .unwrap();
    git(root.path(), &["add", "."]);
    git(root.path(), &["commit", "-qm", "candidate"]);
    let candidate = git(root.path(), &["rev-parse", "HEAD"]);
    // Keep HEAD on the base to demonstrate this is the exact queue candidate,
    // not a scan of whichever files the working directory currently contains.
    git(root.path(), &["checkout", "-q", &base]);
    let contract = guardengine::load_contract_yaml(
        &std::fs::read(repository.join("examples/agent-job-contract.yaml")).unwrap(),
    )
    .unwrap();
    let policy = ProtectedCargoPolicy::freeze(contract, vec![]).unwrap();
    let repo = Repository::discover(root.path(), "archguard-golden-repo").unwrap();
    let scope = TaskScope::advisory(
        "golden-task",
        vec!["R-GOLDEN".into()],
        vec![b"candidate-note.txt".to_vec()],
        policy.profile_digest().trim_start_matches("sha256:"),
        None,
    )
    .unwrap();
    let snapshot = repo
        .prepare_candidate(
            &repo
                .resolve_subject(SubjectRequest::Commit(candidate.clone()))
                .unwrap(),
            &scope,
            &CandidateRequest {
                worktree_id: "golden-worktree".into(),
                base_oid: base.clone(),
                merge_group_id: Some("local-synthetic-queue".into()),
                members: vec![base, candidate],
            },
        )
        .unwrap();
    let verified = GitCargoEvidence::prepare(&repo, &snapshot, policy)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    verified.verify(&repo, &snapshot).unwrap();
    std::fs::write(
        output.join("git-bundle.json"),
        serde_json::to_vec(&verified).unwrap(),
    )
    .unwrap();
    std::fs::write(
        output.join("candidate.json"),
        serde_json::to_vec(&snapshot).unwrap(),
    )
    .unwrap();
    let bundle = verified.cargo();
    for (name, value) in [
        ("contract", bundle.contract.as_ref()),
        ("facts", bundle.facts.as_ref()),
        ("report", bundle.report.as_ref()),
        ("domain", bundle.domain.as_ref()),
    ] {
        std::fs::write(
            output.join(format!("{name}.json")),
            serde_json::to_vec(value.unwrap()).unwrap(),
        )
        .unwrap();
    }
    std::fs::write(
        output.join("envelope.json"),
        serde_json::to_vec(&bundle.envelope).unwrap(),
    )
    .unwrap();
    std::fs::write(
        output.join("bundle.json"),
        serde_json::to_vec(&bundle).unwrap(),
    )
    .unwrap();
    git(
        root.path(),
        &[
            "bundle",
            "create",
            output.join("source.bundle").to_str().unwrap(),
            "--all",
        ],
    );
    println!("{}", output.display());
}
