//! Reproducible local fixture producer; its approval port is deliberately fixture-only.
#[path = "../tests/common/mod.rs"]
mod common;
use archguard::{domain::baseline::ArchitectureTrace, integration::trace::TracedCargoEvidence};
use common::spec_trace::{Fixture, links, policy, port};
use std::{path::Path, sync::atomic::AtomicBool};
fn write(output: &Path, name: &str, value: &impl serde::Serialize) {
    std::fs::write(output.join(name), serde_json::to_vec(value).unwrap()).unwrap();
}
fn main() {
    let destination = std::env::args_os()
        .nth(1)
        .expect("usage: export_spec_trace NEW_DIRECTORY");
    let output = Path::new(&destination);
    std::fs::create_dir(output).expect("output must be new");
    let output = output.canonicalize().unwrap();
    let f = Fixture::new();
    let mapping = f.mapping();
    let trace =
        ArchitectureTrace::read(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50).unwrap();
    let evidence = TracedCargoEvidence::prepare(&f.repo, &f.candidate, policy(), trace).unwrap();
    let frozen_binding = evidence.binding().clone();
    let result = evidence.run(&AtomicBool::new(false)).unwrap();
    result
        .verify(&f.bytes, &mapping, &f.repo, &f.candidate, &port(), 50)
        .unwrap();
    assert_eq!(frozen_binding, result.git().cargo().envelope.binding);
    std::fs::write(output.join("handoff.json"), &f.bytes).unwrap();
    write(&output, "traced-bundle.json", &result);
    write(&output, "git-bundle.json", result.git());
    write(&output, "candidate.json", &f.candidate);
    write(
        &output,
        "baseline-obligations.json",
        result.baseline_obligations(),
    );
    write(&output, "baseline.json", result.baseline());
    write(&output, "links.json", &links());
    write(
        &output,
        "expected.json",
        &serde_json::json!({
            "artifact_digest":f.expected.artifact_digest,
            "repository":f.expected.repository,
            "candidate_binding":f.expected.candidate_binding,
            "baseline_digest":f.expected.baseline_digest,
            "source_digest":f.expected.source_digest,
            "scope":f.expected.scope,
            "authentication_profile":f.expected.authentication_profile,
            "mapping_digest":mapping.digest(),
            "fixture_clock":50,
            "historical_baseline_git_proof":false
        }),
    );
    let bundle = result.git().cargo();
    write(&output, "bundle.json", bundle);
    write(&output, "envelope.json", &bundle.envelope);
    for (name, value) in [
        ("contract", &bundle.contract),
        ("facts", &bundle.facts),
        ("report", &bundle.report),
        ("domain", &bundle.domain),
    ] {
        write(&output, &format!("{name}.json"), value.as_ref().unwrap());
    }
    let status = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.root.0)
        .args(["bundle", "create"])
        .arg(output.join("source.bundle"))
        .arg("--all")
        .status()
        .unwrap();
    assert!(status.success());
    println!("{}", output.display());
}
