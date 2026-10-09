//! Export an actual Cargo producer run over real temporary Git objects.
use archguard::integration::projection::{CandidateContext, CargoEvidence, ProtectedCargoPolicy};
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
        .expect("usage: export_evidence NEW_DIRECTORY");
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
    let context = CandidateContext {
        repo_id: "archguard-golden-repo".into(),
        task_id: "golden-task".into(),
        worktree_id: "golden-worktree".into(),
        requirement_ids: vec!["R-GOLDEN".into()],
        candidate_oid: git(root.path(), &["rev-parse", "HEAD"]),
        base_oid: base,
        merge_group_id: None,
    };
    let contract = guardengine::load_contract_yaml(
        &std::fs::read(repository.join("examples/agent-job-contract.yaml")).unwrap(),
    )
    .unwrap();
    let policy = ProtectedCargoPolicy::freeze(contract, vec![]).unwrap();
    let bundle = CargoEvidence::prepare(root.path(), policy, context)
        .unwrap()
        .run(&AtomicBool::new(false))
        .unwrap();
    bundle.verify().unwrap();
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
