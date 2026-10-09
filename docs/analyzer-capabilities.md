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

## Output safety tightening after review

`--facts` and `--report` still support new output paths. Overwriting an existing file inside the analyzed project or its enclosing Cargo workspace is now deliberately rejected: such files, including old reports, are part of the conservative input scope. Contract aliases and project-input symlinks/hardlinks are protected as well. A rejected path is left intact, with exit 4 and a `not modifying` diagnostic. For repeated report publication, choose a directory outside the analyzed workspace.

Every independently safe requested output is invalidated even when another destination has a missing parent, is a directory, or aliases an input. If alias inspection fails, existing paths are preserved with an explicit error rather than risking source deletion. Inspection follows source-tree aliases, deduplicates canonical paths, and is capped at 100,000 entries and depth 64; ancestor workspace manifests have a 16 MiB limit. Non-Unix platforms without file-identity support conservatively reject existing-file replacement. This validation assumes a stable tree and is not an OS sandbox.

This intentionally changes the former ability to overwrite an existing in-project report. A retained file's existence is never evidence that the current attempt succeeded: consumers must use the current exit status and, when future integration is available, its run binding. The CLI does not yet publish such bindings.

### Declared input roots outside the workspace directory

The protection scope now follows the read-only Cargo manifest reference closure, not just directory descendants. It expands `workspace.members` and `default-members` patterns (fixed `glob` 0.3.3), follows `package.workspace`, and recursively protects local dependency/patch paths and explicit source/readme/license/build paths. External nonmember path dependencies are protected too. Member exclusions are not used to weaken this conservative preservation set. The inspector may protect additional metadata `path` values; malformed or unresolvable inspection causes existing destinations to be preserved with an error.

For a symlinked manifest, both its canonical storage parent and the manifest's source parent participate in reference resolution. The inspected `(canonical manifest, source parent)` pair prevents cycles without discarding distinct alias origins. Real Cargo metadata is used only in regression fixtures to verify the source-origin behavior; output preparation itself never invokes Cargo.

The existing entry/depth/manifest limits remain, with a 100,000-match limit per member pattern. Glob expansion and static filesystem inspection do not claim a wall-clock or OS-containment bound. Repeated output publication should use a location outside **all declared protected input roots**, including external workspace members and referenced local packages, not merely outside the workspace directory.

### Literal roots and unresolved member patterns

The literal filesystem root is escaped with `glob::Pattern::escape` before joining a relative member declaration. Only the declaration contributes pattern syntax; absolute declarations retain their own pattern without a prepended root. Ancestor names containing `[]`, `?` or `*` therefore cannot silently alter protection scope. Explicit local dependency paths continue to be interpreted literally.

A declared member pattern with no matches, a matched member without Cargo.toml, or any manifest/glob parsing failure now makes output preparation fail closed, including for newly requested output paths. No artifact is published on that failure. Existing destinations whose safety cannot be established are retained with an explicit error; consumers must use the current status. This intentionally tightens publication behavior for invalid/unresolved input scope. Legacy scans without requested file outputs retain their existing analysis behavior.
