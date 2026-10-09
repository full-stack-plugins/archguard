# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard is the architecture-focused specialist Guard in the [Guard ecosystem](https://github.com/full-stack-plugins).
Its **v0.1 proof of concept** extracts real Rust Cargo workspace relationships and evaluates an independent GuardEngine contract.

Current local implementation (crate `0.1.0`): bounded Cargo declarations and strict GE evidence, deterministic system rules over supplied LanguageObservation, and an opt-in GitGuard candidate source bridge. The Git bridge reads actual immutable candidate blobs into private analysis; it does not authenticate a producer, controller or queue. Native CLI/schema remain compatible. See [implementation status](docs/implementation-progress.md), [system rules](docs/decisions/0003-local-system-rules.md) and [Git source boundary](docs/decisions/0004-git-candidate-source.md). The fixed JDK21 static bytecode provider is independently accepted; the fixed TypeScript5.9.3 compiler provider is implemented pending review. Both retain explicit static scope and runtime gaps. Authenticated baselines, production trust services, admission and published packages remain unimplemented. See the [capability matrix](docs/analyzer-capabilities.md).

## Prerequisites

Rust 1.90+, Cargo, and sibling `../guardengine`, `../gitguard` and `../specguard` checkouts (temporary local path dependencies). The actual GitGuard dependency requires Rust 1.90. Repositories remain independent; fixed published artifacts are a separate release task.

```text
workspace-full-stack-plugins/
├── guardengine/
├── gitguard/
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
- Cargo metadata/manifest inspection failures produce `partial`, `INDETERMINATE` and `BLOCK`. Failure to canonicalize the project root is instead an input error (exit 4).
- Evidence includes an SHA-256 snapshot over the inspected `Cargo.toml` files, **not a full repository commit**.
- Edges use actual package names and canonical dependency paths; renamed dependencies do not create alias nodes. Ordinary/dev/build/optional/target-conditioned declarations are not distinguished in facts. This is not a resolved feature/target build graph. External registry/git and non-member path edges are omitted; transitive edges and cycles are not computed.
- `complete` means extraction succeeded within that limited scope. A contract naming a nonexistent package or unsupported relation can still pass because the engine matches exact triples; it does not prove rule applicability.
- The snapshot hashes exact root/member manifest bytes and path labels, excluding `Cargo.lock`, `.cargo/config*`, source files, toolchain and Git identity. Identical manifests alone do not bind all inputs to Cargo metadata.
- Local reports are unsigned. Trusted CI must independently re-run on a pinned checkout with protected contract input. Recomputing a report checks consistency, not source authenticity, approval or merge authority.

## CLI and result contract

Only `check` is implemented. `--project` and `--contract` are required; `--facts` and `--report` are optional output paths. Reports are pretty JSON on stdout **unless `--report` is supplied**, in which case the report is written only to that file. `--facts` writes a separate JSON fact set before evaluation. Output directories must already exist; writes overwrite existing files and are not atomic. Avoid pointing outputs at inputs or reusing stale files after errors. Runtime/input diagnostics go to stderr with `archguard:`.

| Exit | Meaning |
|---|---|
| 0 | `ALLOW` within the analyzed scope; advisory matches may exist |
| 2 | `BLOCK`: enforced relation or incomplete analysis |
| 3 | `REQUIRE_APPROVAL`: matching `review` rule with no blocking rule |
| 4 | Invalid arguments/contract, inaccessible root, output failure or other runtime error; no guaranteed fresh report |

The parser takes the first matching flag/value pair; it does not implement conventional `--help`, reject all unknown flags, or validate duplicate flags. Use the exact forms above. The namespace `guard.partme.ai/v1alpha1` is retained for wire compatibility. Contract, facts and report models reject unknown fields; only exact `forbid_relation` exists today. An `enforce` match blocks, a `review` match requests approval, and an `advise` match remains `PASS` with matched evidence. No current approval command exists.

## Interfaces

`CargoWorkspaceAnalyzer` implements `guardengine::GuardAnalyzer`, returning `GuardFacts`. The engine has no Cargo-specific knowledge. This is the first adapter, not a general code graph.

Read [Architecture](docs/architecture.md), [OpenSpec implementation](openspec/changes/add-cargo-workspace-guard/), and [GuardEngine protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md).

## Architectural governance roadmap

The complete ArchGuard design includes **System Guard** (layering, module dependencies and cycles), **Domain Guard** (bounded contexts, aggregates and state invariants), **Object Guard** (types, ownership and public APIs), and **Method Guard** (signatures, call relationships and side effects). These are **future capabilities**, not features of the current Cargo-only analyzer.

The distinction between deterministic **ENFORCE** checks and human **REVIEW** of architectural trade-offs is part of the architecture contract. Design Diff and the code graph must retain source locations, analyzer coverage, unknown relations and the approved baseline rather than producing a universal quality score.

Read the expanded [architecture and ADRs](docs/architecture.md) and the [technical implementation design](docs/technical-design.md) for detailed module boundaries, Java/Rust/TypeScript adapters, rule semantics, CI trust conditions, milestones and negative-test criteria.

## Integration and next steps

SpecGuard, ArchGuard, CodeGuard, TestGuard, GitGuard and FlowGuard are independent domain guards using GuardEngine; there is no GuardCore. Engine owns generic contracts, neutral rule evaluation and deterministic evidence, while ArchGuard owns extraction and architecture meaning. Cross-guard orchestration, MCP, approvals and Design Diff remain targets. The opt-in local Git bridge binds actual candidate/base/group/member objects, without queue authority. The [integration contract](docs/integration-contract.md) is implemented as a separate strict envelope; native GuardFacts/GuardReport schemas remain unchanged. External CodeGraph/codegraph-plugin and codereview-plugin integrations are unverified compatibility targets, not inspected installed components.

## Test

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

License: Apache-2.0.


## OpenSpec implementation backlog

The incremental [proposal](openspec/changes/extend-architecture-analysis-and-evidence/proposal.md), [design](openspec/changes/extend-architecture-analysis-and-evidence/design.md), [requirements](openspec/changes/extend-architecture-analysis-and-evidence/specs/) and [tasks](openspec/changes/extend-architecture-analysis-and-evidence/tasks.md) translate the architecture into pending implementation work. See the [cross-repository dependency roadmap](openspec/guard-roadmap.md) and [structural validation record](openspec/validation-2026-10-09.md). Every new implementation task remains unchecked; this branch adds planning artifacts, not product features. Earlier source-tree inventories and validation limitations describe the inspected baseline or earlier architecture-review stage; this planning stage adds OpenSpec artifacts and separately records actual CLI validation. Existing change ownership and historical completion evidence remain intact.

The library now also provides a fixed OpenJDK 21 static bytecode adapter, submitted for independent review. It preserves required type scope, overload descriptors and native capture provenance; reflection and dynamic behavior remain unknown. See the [capability matrix](docs/analyzer-capabilities.md) and [JDK decision](docs/decisions/0007-fixed-jdk-classfile-provider.md). The existing CLI remains Cargo-focused.
