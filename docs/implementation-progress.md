# ArchGuard implementation ledger

Base: 9ed191acf3c3b1f9c0ca9edf3d0621b6a7c53f04. Local branch only; all approved task checkboxes remain pending review.

| Task | Executable slice / verification | Prerequisite |
|---|---|---|
| Legacy format | Preserve normative paragraphs, add scenarios, strict validate both changes | none |
| 1.1 | Immutable protected Cargo profile; missing member/relation/candidate removal tests | none |
| 1.2 | Cargo declaration model + real rename/dev/build/optional/target/nonmember fixture and unchanged projection | 1.1 |
| 1.3 | Canonical explicit input inventory, config/lock/tool invalidation tests, legacy digest golden | 1.2 |
| 1.4 | Isolated bounded Cargo runner; timeout/output/path/nonzero test doubles, original-tree lock test | 1.1 |
| 1.5 | Explicit CLI legacy selection + safe publication; exit/status/output regressions | 1.1–1.4 for enhanced CLI |
| 2.1 | Scoped language symbol/coverage model and measured tool capability decision | group 1 |
| 2.2–2.4 | Real language adapters only after pinned tool and coverage evaluation | tool selection |
| 2.5 | System rule legal/violation/coverage matrices | 2.1 |
| 3–5 | Not in slice; no invented integration API | GE freeze / other gates |

TDD evidence and exact acceptance status are recorded in the external slice report. Baseline: original five tests passed before changes.

## Slice 1 status (pending root review)

- 1.1: implemented locally. Owned protected scope, unknown-member/unsupported-relation gaps, candidate scope-copy isolation, ordinary complete ALLOW, explicit profile selection.
- 1.2: implemented locally. Real Cargo declaration fixture plus canonical declaration ordering; legacy tuple and manifest golden preserved. Domain artifact serialization/envelope belongs to later integration.
- 1.3: implemented for the restricted declaration profile. Conservative copied-tree inventory and tool/environment identity; canonicalization and lock/environment invalidation tests. Cargo configuration is explicitly unsupported, not an active-build profile.
- 1.4: partial. Copy/allowlist/path/config/file budgets and bounded Unix tools are implemented, but no OS sandbox/hostile concurrent-input or escaped-process containment. Cannot count the full task accepted.
- 1.5: implemented locally. Explicit legacy/enhanced selection; original exits, review/advise/report-output semantics, stale-output invalidation and atomic publication. No evidence transport is implied.
- 2.1: partial dependency/capability study only; SymbolId/domain coverage model not implemented.
- 2.2: not implemented; JRE present, javac unavailable in PATH; bytecode adapter/version unresolved.
- 2.3: partial evaluation only. Pinned TypeScript compiler fixture resolves aliases and overloads and retains unknown dynamic imports. No production adapter or project-reference matrix.
- 2.4: not implemented; rust-analyzer component absent; compiler availability is not symbol-provider support.
- 2.5: not implemented; no system-rule engine added.
- Groups 3–5: deferred beyond this slice. Root supplied emerging GE API and SpecGuard fixture handoff; neither is consumed or misrepresented as reviewed interoperability.

Final Rust verification: 27 tests pass, including all original five. Formatting, warning-free clippy and strict old/new OpenSpec validation passed before the local commit. All OpenSpec checkboxes remain unchanged. See external `archguard-slice1-report.md` for commands, RED/GREEN history, TDD evidence limits, commits and remaining acceptance gates.

## Review corrections — task 1.5

Independent review accepted 1.1/1.2/1.3 only within the documented restricted Cargo scope and left 1.4 partial. Two task-1.5 findings reproduced in new tests: one bad output destination preserved another old success file, and output invalidation could delete member manifests/lockfiles/source.

The correction separates per-destination classification from best-effort safe invalidation. Existing project/workspace input entries, contract aliases, symlinks and hardlinks are preserved; independently safe external destinations are still cleaned when another destination fails. New output paths remain usable. Existing in-project output replacement is intentionally no longer supported and returns exit 4 with `not modifying` diagnostics. Consumers must use this attempt's status, not old file existence. Full acceptance remains pending re-review; no new task checkboxes or language capability claims are added.
