# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard is the architecture-focused specialist Guard in the [Guard ecosystem](https://github.com/full-stack-plugins).
Its **v0.1 proof of concept** extracts real Rust Cargo workspace relationships and evaluates an independent GuardEngine contract.

Current local implementation (crate `0.1.0`) includes bounded Cargo declarations, deterministic system rules, independently accepted JDK 21 static bytecode and TypeScript 5.9.3 semantic providers, immutable approved baselines, approved static domain access with `NotRun` obligations, observed Java Design Diff, and strict Cargo evidence/trust ports. The native CLI remains Cargo-only. Approval authorities are external inputs; local verification does not install a production authority or grant merge permission. See [current acceptance and remaining gaps](docs/reviews/local-closure-20261010.md), [implementation history](docs/implementation-progress.md), and the [capability matrix](docs/analyzer-capabilities.md).

## Prerequisites

Rust 1.90+ and Cargo. Core development uses only the sibling `../guardengine` source dependency; GitGuard and SpecGuard are not core dependencies. The [independent source distribution workflow](docs/independent-distribution.md) creates a locked archive with the fixed GuardEngine source vendored inside it, buildable without sibling repositories. Registry/toolchain caches and the documented Java/TypeScript test tools remain prerequisites; this is not a published production package.

Optional Git/Spec interoperability lives under its own `integrations/git-spec/Cargo.toml`; it uses the separately pinned GG/SG/GE checkouts described in the distribution guide. Its actual Git candidate bridge and trace reader do not supply a production issuer or queue controller.

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

## Default legacy CLI scope

- Uses `cargo metadata --no-deps --offline --format-version 1` to inspect **direct Cargo workspace member package dependencies**.
- Detects local path dependencies (including declared optional/dev/build dependencies) between workspace members.
- Does **not** analyze Rust `use` or functions, external crate call graphs, runtime edges, Java modules, domain aggregate design, or interface ownership.
- Cargo metadata/manifest inspection failures produce `partial`, `INDETERMINATE` and `BLOCK`. Failure to canonicalize the project root is instead an input error (exit 4).
- Evidence includes an SHA-256 snapshot over the inspected `Cargo.toml` files, **not a full repository commit**.
- Edges use actual package names and canonical dependency paths; renamed dependencies do not create alias nodes. Ordinary/dev/build/optional/target-conditioned declarations are not distinguished in facts. This is not a resolved feature/target build graph. External registry/git and non-member path edges are omitted; transitive edges and cycles are not computed.
- `complete` means extraction succeeded within that limited scope. A contract naming a nonexistent package or unsupported relation can still pass because the engine matches exact triples; it does not prove rule applicability.
- The snapshot hashes exact root/member manifest bytes and path labels, excluding `Cargo.lock`, `.cargo/config*`, source files, toolchain and Git identity. Identical manifests alone do not bind all inputs to Cargo metadata.
- Local reports are unsigned. Trusted CI must independently re-run on a pinned checkout with protected contract input. Recomputing a report checks consistency, not source authenticity, approval or merge authority.

The explicit `--profile cargo-declarations-v1` selects the bounded declaration profile with protected rule applicability, declaration provenance and measured input inventory. The default `legacy` behavior above remains available; neither profile is a Rust symbol or runtime call analyzer.

## CLI and result contract

Only `check` is implemented. `--project` and `--contract` are required; `--facts` and `--report` are optional output paths. `--profile` accepts `legacy` (default) or `cargo-declarations-v1`. Reports are pretty JSON on stdout **unless `--report` is supplied**, in which case the report is written only to that file. `--facts` writes a separate JSON fact set before evaluation. Output directories must already exist. Publication uses staged atomic replacement per output; protected project/contract inputs and aliases are rejected, and safely identified old outputs are invalidated before execution. Multiple outputs are not one transaction. Always use the current exit status; an error is not fresh evidence. Runtime/input diagnostics go to stderr with `archguard:`.

| Exit | Meaning |
|---|---|
| 0 | `ALLOW` within the analyzed scope; advisory matches may exist |
| 2 | `BLOCK`: enforced relation or incomplete analysis |
| 3 | `REQUIRE_APPROVAL`: matching `review` rule with no blocking rule |
| 4 | Invalid arguments/contract, inaccessible root, output failure or other runtime error; no guaranteed fresh report |

The parser rejects unknown or duplicate flags and missing values; conventional `--help` is not implemented. Use the exact forms above. The namespace `guard.partme.ai/v1alpha1` is retained for wire compatibility. Contract, facts and report models reject unknown fields; only exact `forbid_relation` exists today. An `enforce` match blocks, a `review` match requests approval, and an `advise` match remains `PASS` with matched evidence. No current approval command exists.

## Interfaces

`CargoWorkspaceAnalyzer` implements `guardengine::GuardAnalyzer`, returning `GuardFacts`. The engine has no Cargo-specific knowledge. This is the first adapter, not a general code graph.

Read [Architecture](docs/architecture.md), [OpenSpec implementation](openspec/changes/add-cargo-workspace-guard/), and [GuardEngine protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md).

## Architectural governance roadmap

The complete ArchGuard design includes **System Guard** (layering, module dependencies and cycles), **Domain Guard** (bounded contexts, aggregates and state invariants), **Object Guard** (types, ownership and public APIs), and **Method Guard** (signatures, call relationships and side effects). System rules and the scoped static domain/library capabilities above are implemented. Deep Rust symbols, full object/method semantics and general API compatibility remain incomplete; the CLI is still Cargo-only.

The distinction between deterministic **ENFORCE** checks and human **REVIEW** of architectural trade-offs is part of the architecture contract. Design Diff and the code graph must retain source locations, analyzer coverage, unknown relations and the approved baseline rather than producing a universal quality score.

Read the expanded [architecture and ADRs](docs/architecture.md) and the [technical implementation design](docs/technical-design.md) for detailed module boundaries, Java/Rust/TypeScript adapters, rule semantics, CI trust conditions, milestones and negative-test criteria.

## Integration and next steps

SpecGuard, ArchGuard, CodeGuard, TestGuard, GitGuard and FlowGuard are independent domain guards using GuardEngine; there is no GuardCore. Engine owns generic contracts, neutral rule evaluation and deterministic evidence, while ArchGuard owns extraction and architecture meaning. Supported Cargo consumption through actual GG/FG boundaries and local phased rollback are verified; production orchestration/authority, unified MCP interfaces and full semantic Design Diff remain targets. The opt-in local Git bridge binds actual candidate/base/group/member objects, without queue authority. The [integration contract](docs/integration-contract.md) is implemented as a separate strict envelope; native GuardFacts/GuardReport schemas remain unchanged. The retained GG CodeGraph adapter has a separately qualified bounded JavaScript export scope; it is not an AG Java index importer or a universal installed indexer. AG external CodeGraph/codereview-plugin integration and host installation remain unverified.

## Test

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

License: Apache-2.0.


## OpenSpec implementation backlog

The [proposal](openspec/changes/extend-architecture-analysis-and-evidence/proposal.md), [design](openspec/changes/extend-architecture-analysis-and-evidence/design.md), [requirements](openspec/changes/extend-architecture-analysis-and-evidence/specs/) and [tasks](openspec/changes/extend-architecture-analysis-and-evidence/tasks.md) track **19/25 accepted tasks** as of 2026-10-10. Tasks 2.4, 3.3, 3.5, 5.2, 5.3 and 5.5 remain open with exact gaps in the [closure review](docs/reviews/local-closure-20261010.md). Historical validation records describe their original checkpoints.

The approved skills-first plan has delivered an independent core, an optional Git/Spec package, and local Java-native provider/wrapper delivery. Provider `fad6395` preserves the original three tests and adds four structural tests (cycle, naming, static state access and import coverage); wrapper `1ed1beb` pins its provenance. Native evidence is qualified for these explicit structural examples, not runtime invariants, purity, authenticated GE evidence or an installed host. Earlier wrapper application runs covered only the original three tests. See the closure review for exact pins and evidence boundaries.
