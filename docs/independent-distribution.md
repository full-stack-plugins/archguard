# Independent core source distribution

The default `archguard` package retains Cargo declarations, Java bytecode, TypeScript
semantic analysis, architecture contracts, design diffs and the immutable
`domain::baseline::{ArchitectureBaseline, ArchitectureBaselineState}` lifecycle.
It has no GitGuard or SpecGuard dependency, feature, or workspace member.

## API migration

| Previous ArchGuard API | Optional `archguard_git_spec` API |
| --- | --- |
| `integration::binding` (including `attempts`) | `integration::binding` (including `attempts`) |
| `integration::trace` | `integration::trace` |
| `domain::baseline::{ArchitectureTrace, ProtectedTraceMapping, RuleTraceLink}` | `domain::baseline::{ArchitectureTrace, ProtectedTraceMapping, RuleTraceLink}` |

`candidate_binding`, trace `baseline`, `attempt_concurrency`, their shared fixture
helpers and `export_git_evidence`/`export_spec_trace` move to `integrations/git-spec`.
Core `domain_contracts` and `design_diff` continue importing the core lifecycle.
The optional crate depends one way on core. Actual Repository/CandidateSnapshot
verification and exact byte/inventory checks remain in the optional crate.
Neutral preparation/data access APIs convey no Git candidate authentication.

## Build and verify

Development uses one shared local GuardEngine path so sibling consumers and the
optional integration share Rust type identity. This checkout is not itself a
standalone published Cargo crate. `python3 scripts/package-core.py /tmp/core.tar.gz`
creates the independently buildable **source distribution**, rewrites only that
manifest's GE path to `vendor/guardengine`, includes Cargo.lock, Apache-2.0 license,
GE NOTICE, source hashes and package SHA256SUMS, and excludes the optional package.
It rejects changed GE bytes and symlinks. Registry dependencies resolve using the
lockfile; an offline registry vendor directory is not included.

Unpack into a new directory with no sibling repositories, then run:

```sh
tar -xf /tmp/core.tar.gz -C /tmp/new-empty-directory
cd /tmp/new-empty-directory
cargo build --locked
cargo test --locked
cargo tree --locked
```

The isolated distribution test performs these commands and checks the dependency
tree for GitGuard/SpecGuard. Cargo's downloaded registry/toolchain cache may be
reused; no repository source path or symlink is used. Real Java tests require JDK
21; `domain_contracts`/`design_diff` accept `ARCHGUARD_TEST_JDK`. The existing Java
suite and Node executable retain their tested toolchain locations. TypeScript
5.9.3 must be installed with `npm ci` in `fixtures/languages/typescript` before
packaging to include the test compiler and its license/notices. Its path is
resolved from the unpacked package, never a development checkout. Missing external
tools fail these tests; a missing tool is not an analysis pass.

Fixed GE source: `a47ae91811f2f31f9d6438f382affbccd9cf4426`; Cargo.toml, Cargo.lock
and src are byte-identical to reviewed source
`b0e4b445e1a20a9354d55ebb41bbedaf16ec8a74`. Upstream declares Apache-2.0 but has no
LICENSE/NOTICE file at that pin; the bundled standard license text and added
provenance NOTICE explain this without inventing upstream attribution.

Optional local integration, outside default core build and archive:

```sh
python3 integrations/git-spec/verify-source-pins.py
cargo test --locked --manifest-path integrations/git-spec/Cargo.toml
```

Its exact local GG/SG/GE commits and file hashes are in `source-pins.json`.
GG `6be082a4b1ae91fce1075ba4fadcb7c17bbc21bc` and SG
`ac580fd643e36a35bdfdfbb1e5001f3a47745214` are local, not claimed remotely
fetchable or published. GG/SG have no license metadata/files at these pins; this
work does not supply a new license or distribute them in the core archive.
Consumers such as FlowGuard must import the optional binding API explicitly;
their shared sibling sources avoid duplicate Rust crate identities.
