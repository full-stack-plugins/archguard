# Analyzer capabilities — implementation slice, pending review

## Cargo

| Profile | What is actually observed | Limitations |
|---|---|---|
| `legacy` (default) | Cargo workspace member direct path declarations, deduplicated `(subject, depends_on, object, source)` | Original manifest-only digest and metadata-error partial/BLOCK semantics; no timeout or isolation guarantee |
| `cargo-declarations-v1` (experimental, explicit) | Same direct-member relation, with required scope copied from the caller-supplied contract before extraction; missing members/unknown predicates yield partial/BLOCK | Declaration graph, never active feature/target or source/call graph; caller must obtain protected policy externally; no candidate authentication |

The library's `analysis::cargo::observe` preserves each Cargo declaration's actual member target, package name, rename, kind, optional flag, target condition, path and registry/git source, plus its source manifest. It remains the direct legacy extraction path. Enhanced `analysis::analyze` uses an isolated copy and exports only the neutral facts; a serialized domain artifact and evidence envelope are not published in this slice.

The enhanced inventory hashes source-tree files conservatively, excluding `.git` and `target`, plus frozen scope, resolved tool paths/version output and declared ambient `CARGO_*`, `RUST*`, `PATH`, `HOME` identity. Keys/lengths/bytes are ordered deterministically. Raw identity/environment bytes are not exported. Lock/config/toolchain/source changes cannot be represented as the old manifest-only digest. This is not a full repository identity, candidate binding, or controller cache key.

Execution uses fixed Cargo/rustc paths resolved outside the candidate, `env_clear`, an empty temporary HOME/CARGO_HOME, no stdin, offline no-deps metadata and a private copied project. Candidate Cargo config, temporary-ancestor Cargo config, absolute/escaping manifest paths, symlinks and special files are rejected. The current declaration profile does not honor candidate toolchain switching, Cargo aliases, wrappers, arbitrary scripts, target/feature activation, or external path dependencies. Nonmember dependencies contained inside the copied tree retain declaration provenance and produce no member edge.

Default per-tool budget: 20 seconds, 8 MiB combined stdout/stderr. Copy budget: 16 MiB per file, 128 MiB total, 10,000 directory/file entries, depth 64. Unix process-group termination and nonblocking pipe reads bound normal tool execution and cleanup. Non-Unix execution is unsupported.

**Security acceptance remains partial:** the copy/validation mechanism is not an OS filesystem sandbox. Controllers must provide immutable input trees and trusted executables/environment; concurrent ancestor-directory substitution and deliberate process-session escapes are not contained. A real hostile-worktree runner needs an OS isolation backend and cgroup/process containment. This host's `bwrap --ro-bind / / --unshare-net -- /bin/true` fails with `setting up uid map: Read-only file system`. Do not use this experimental profile as authenticated protected-merge evidence.

## Language provider decisions

No Java, TypeScript, deep Rust, cycle, layer, domain, object or method capability is published. Requesting their profile names returns unsupported rather than Cargo-complete coverage.

| Candidate | Actual evaluation | Decision |
|---|---|---|
| TypeScript compiler API 5.9.3 | Real pinned npm package; resolves `@core/*` alias to a cross-folder type; preserves two method overload signatures; nonliteral dynamic import is explicitly unknown | Preferred candidate for a future semantic adapter; evaluation fixture only, no production provider |
| Java bytecode/ArchUnit | OpenJDK runtime 21.0.12.1 present; `javac` absent from PATH | No adapter/version selected; bytecode compiler, license/dependency study and reflection coverage fixture remain required |
| Rust symbol index | Rust/cargo 1.99.0 available; rust-analyzer shim reports component absent | No symbol-index adapter selected; module/type/method, cfg/features/target, macro and dispatch matrix remains required |

The TypeScript fixture is `fixtures/languages/typescript`, with exact package and npm lock integrity. Source: [official npm 5.9.3 metadata](https://registry.npmjs.org/typescript/5.9.3), license Apache-2.0, package unpacked size 23,625,066 bytes, declared Node requirement >=14.17. Tested on Node 24.19.0. Local compiler run: 1.296 seconds and 294,048 KiB peak RSS (single small fixture, not a bound or benchmark). The Rust runner's input budget is not a JavaScript process-memory limit. Project references, unresolved aliases, package export conditions, cross-language symbol IDs and production resource containment still need executable acceptance matrices.

Reproduce with `npm ci --ignore-scripts --no-audit --no-fund --cache=/tmp/archguard-npm-cache --prefix fixtures/languages/typescript`, then `npm test --prefix fixtures/languages/typescript`. Package installation does not publish the adapter or authorize anything.
