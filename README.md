# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard is the architecture-focused specialist Guard in [Partme Guard](https://github.com/full-stack-plugins).
Its **v0.1 proof of concept** extracts real Rust Cargo workspace relationships and evaluates an independent GuardEngine contract.

## Prerequisites

Rust 1.85+, Cargo, sibling `../guardengine` checkout (temporary local path dependency). Both repositories are independent; a versioned GuardEngine artifact will replace the path for public releases.

```text
workspace-full-stack-plugins/
├── guardengine/
└── archguard/
```

## Run the architecture gate

```sh
# In archguard/
cargo run -- check --project fixtures/allowed --contract examples/agent-job-contract.yaml
# ALLOW, exit 0

cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml
# BLOCK, exit 2: agent-job depends_on agent-saas

cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml --facts facts.json --report evidence.json
# Emits facts and evidence even when the gate blocks.
```

Rule DSL:

```yaml
apiVersion: guard.partme.ai/v1alpha1
kind: GuardContract
metadata: { id: agent-job-boundary, revision: "1" }
spec:
  rules:
    - id: AGJ-ARCH-001
      enforcement: enforce
      assertion:
        type: forbid_relation
        subject: agent-job
        predicate: depends_on
        object: agent-saas
```

## Exact scope

- Uses `cargo metadata --no-deps --offline --format-version 1` to inspect **direct Cargo workspace member package dependencies**.
- Detects local path dependencies (including declared optional/dev/build dependencies) between workspace members.
- Does **not** analyze Rust `use` or functions, external crate call graphs, runtime edges, Java modules, domain aggregate design, or interface ownership.
- On Cargo metadata failure or incomplete inspection, it reports `partial`, `INDETERMINATE` and `BLOCK` rather than assuming success.
- Evidence includes an SHA-256 snapshot over the inspected `Cargo.toml` files, **not a full repository commit**.
- Local reports are unsigned. Trusted CI must independently re-run on a pinned checkout with protected contract input.

## Interfaces

`CargoWorkspaceAnalyzer` implements `guardengine::GuardAnalyzer`, returning `GuardFacts`. The engine has no Cargo-specific knowledge. This is the first adapter, not a general code graph.

Read [Architecture](docs/architecture.md), [OpenSpec implementation](openspec/changes/add-cargo-workspace-guard/), and [GuardEngine protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md).

## Architectural governance roadmap

The complete ArchGuard design includes **System Guard** (layering, module dependencies and cycles), **Domain Guard** (bounded contexts, aggregates and state invariants), **Object Guard** (types, ownership and public APIs), and **Method Guard** (signatures, call relationships and side effects). These are **future capabilities**, not features of the current Cargo-only analyzer.

The distinction between deterministic **ENFORCE** checks and human **REVIEW** of architectural trade-offs is part of the architecture contract. Design Diff and the code graph must retain source locations, analyzer coverage, unknown relations and the approved baseline rather than producing a universal quality score.

Read the expanded [architecture and ADRs](docs/architecture.md) and the [technical implementation design](docs/technical-design.md) for detailed module boundaries, Java/Rust/TypeScript adapters, rule semantics, CI trust conditions, milestones and negative-test criteria.

## Test

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

License: Apache-2.0.
