# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard is the architecture-focused specialist Guard in [Partme Guard](https://github.com/full-stack-plugins).
Version **0.5.0** adds opt-in Java source package dependencies, layer boundaries and mapped domain type ownership to the unified project model, requirement delta and repair/verification lifecycle. Maven/Cargo checks and existing contract-only projects remain supported. See the [Java source guide](docs/java-source.md), [project model guide](docs/project-model.md) and [portable plugin](plugin/).

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

- Uses `cargo metadata --no-deps --offline --locked --format-version 1` to inspect **direct Cargo workspace member package dependencies**; Maven reads local POMs without executing builds.
- Detects local path dependencies (including declared optional/dev/build dependencies) between workspace members.
- Supports Maven declared module edges, forbidden paths, allowlists, required edges and cycles. Configured model Java policies additionally check imports, known qualified references, package layers and declared type ownership. Rust `use`, full Java compiler binding, runtime calls and domain business invariants remain outside coverage.
- On Cargo metadata failure or incomplete inspection, it reports `partial`, `INDETERMINATE` and `BLOCK` rather than assuming success.
- Evidence includes an SHA-256 snapshot over inspected build inputs, **not a full repository commit**. `verify` reanalyzes current sources and contract. Export targets must be new files.
- Local reports are unsigned. Trusted CI must independently re-run on a pinned checkout with protected contract input.

## Interfaces

`CargoWorkspaceAnalyzer` and `MavenWorkspaceAnalyzer` implement `guardengine::GuardAnalyzer`, returning coverage-aware `GuardFacts`. The engine has no build-system-specific knowledge. Ten read-only MCP tools expose context, instructions, check, explain, verify, model_show, model_validate, change_impact, change_check and change_verify (prefixed `archguard_`). Baseline initialization and updates are explicit CLI operations.

Read [Architecture](docs/architecture.md), [OpenSpec implementation](openspec/changes/add-cargo-workspace-guard/), and [GuardEngine protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md).

## Architectural governance roadmap

The complete design includes System, Domain, Object and Method guards. Version 0.5.0 implements module dependency constraints, Java syntax package/layer/ownership checks and project baseline delta verification; full compiler and domain/method semantics remain future work. Unsupported effective Maven models block with an explicit diagnostic.

The distinction between deterministic **ENFORCE** checks and human **REVIEW** of architectural trade-offs is part of the architecture contract. Design Diff and the code graph must retain source locations, analyzer coverage, unknown relations and the approved baseline rather than producing a universal quality score.

Read the expanded [architecture and ADRs](docs/architecture.md) and the [technical implementation design](docs/technical-design.md) for detailed module boundaries, Java/Rust/TypeScript adapters, rule semantics, CI trust conditions, milestones and negative-test criteria.

## Test

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

License: Apache-2.0.

## Agent installation and managed instructions

The shared installer provides `install`, `uninstall`, `context`, `instructions render|sync`, `doctor --integration`, and `serve --mcp` for Codex, Claude Code, and Gemini CLI. Each tool renders its own managed block.

```sh
cargo build --locked
archguard install --agents codex,claude,gemini --project /absolute/project --bin-dir /absolute/bin --dry-run
archguard context --project /absolute/project
```

After building, invoke the executable under `target/debug/` (with `.exe` on Windows); short commands above assume PATH is configured. `--bin-dir` copies the current executable; PATH is not changed automatically. Remove `--dry-run` to apply. A sibling `../guardengine/crates/guard-install` source checkout is required.

[Shared design, host mapping, update/uninstall and verification](https://github.com/full-stack-plugins/guardengine/blob/main/docs/agent-installation.md). Configuration success does not prove host loading or quality approval.

## Architecture methods / 架构方法

[原生规格适配、五项技能与 Maven profile 检查闭环](docs/method-integration.md)：0.3.0 新增只读 spec context、可选规则到原生规格绑定、绑定产物漂移复验和显式 profile 场景。插件方法由固定来源依赖闭包生成，不代替确定性检查或授权。
