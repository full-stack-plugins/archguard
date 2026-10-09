# ArchGuard Architecture v0.1

## Data flow

```text
Cargo workspace (Cargo.toml files)
             |
   cargo metadata --no-deps --offline
             |
     CargoWorkspaceAnalyzer
             |
      GuardFacts (neutral relation triples)
             |
      GuardEngine::evaluate
             |
      GuardReport + exit code
```

The adapter maps each local workspace-member path dependency to a neutral fact:

```json
{"subject":"agent-job","predicate":"depends_on","object":"agent-saas","source":"agent-job/Cargo.toml"}
```

The fact source identifies the declaring manifest, not a precise code span. Cargo metadata is authoritative only for **declared package edges** and will not discover method calls or domain design flaws.

## Guard/Engine responsibilities

ArchGuard knows Cargo metadata, path dependencies and package identity. GuardEngine knows only structured facts, contracts, rule matching, results and evidence hashes.

The CLI directly uses the library trait; additional analyzers (Java ArchUnit, Rust HIR, TypeScript, SCIP) are future plugins, not hidden capabilities in v0.1.

## Source completeness

For successful metadata resolution, `complete` means only that the analyzer enumerated direct local workspace Cargo package dependencies it can discover. It does not imply all program dependencies have been analyzed. Cargo metadata failures yield `partial` with diagnostics and no ALLOW result.

## Exact example

Policy: `agent-job` must not have a `depends_on` relation with `agent-saas`.

Fixtures: `fixtures/allowed` permits only `agent-job → agent-contracts`; `fixtures/forbidden` adds the prohibited package and dependency. Integration tests assert that the CLI returns 0 and 2 respectively and contains a source reference.

## Release and CI

The temporary path dependency is for this pair of unpublished sibling repositories. CI should check out `full-stack-plugins/guardengine` beside this repo. Once the GuardEngine crate has a published, immutable version, pin that version and test compatibility.

Never use PR-modified policy as the sole authorization source. Fetch it from a protected baseline in trusted CI.
