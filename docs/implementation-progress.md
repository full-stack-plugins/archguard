# ArchGuard implementation ledger

Base: 9ed191acf3c3b1f9c0ca9edf3d0621b6a7c53f04. Local branch only. Root acceptance checkpoint: 5/25 tasks accepted (1.1, 1.2, 1.3, 1.5, 4.1); restricted Cargo scope only. Producer 01f2f13 independently accepted for 4.1 in external archguard-producer-independent-review.md. Task 4.2 remains partial because public candidate context does not validate Git objects/source equality. Task 1.4 remains partial. New task 2.1 typed model slice is partial and pending review.

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

## Historical slice 1 status (superseded by acceptance checkpoint above)

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

## External member follow-up — task 1.5 still awaiting re-review

A second independent review found that directory-only protection missed valid `workspace.members = ["../member"]` layouts. New test-first regressions cover literal/`*`/bracket member patterns, root/member entrypoints, both output flags with independent safe cleanup, transitive external nonmember path dependencies, and symlinked manifest source-relative references confirmed by real Cargo metadata.

The implementation now discovers a conservative manifest-reference closure without invoking Cargo during preparation. Existing external declared inputs and their aliases remain unchanged. Fixed `glob = 0.3.3` supplies Cargo-style pattern expansion; its official metadata declares MIT OR Apache-2.0 and Rust 1.63.0 minimum. Uncertain inspection preserves existing files with an explicit error. This does not change the previously documented OS isolation limitations or add any language/domain/evidence capability. No task checkbox changed.

## Literal-root glob correction — task 1.5 pending re-review

The next review reproduced an external-member omission when a literal ancestor directory contained `[1]`. New test-first regressions cover Unix ancestor names containing brackets, question marks and stars; literal member declarations and absolute declared patterns; literal metacharacter path dependencies; input preservation plus independent safe cleanup. The literal root prefix is now escaped separately from the declared pattern.

Unmatched member patterns, matched members missing manifests, and manifest/glob parse errors now reject publication to both existing and new destinations. Previously a protection error could be ignored when every requested destination was new; the test reproduced that path before correction. This is an intentional safety tightening and does not change task 1.4's partial status or mark task 1.5 accepted.

## Next independent slice: task 2.1 typed language model

Implement versioned Module/Type/Method symbol identity, provider-supplied canonical signature, source span and confirmed/unknown relation coverage in `src/domain/model.rs`. Tests in `tests/language_matrix.rs` must distinguish overloads, languages, modules and delimiter-like names, preserve source provenance, and prevent unresolved dispatch from presenting complete coverage or invented confirmed edges. This is the model portion of 2.1 only: no Java/TypeScript/Rust provider or serialized artifact is published. The earlier TS 5.9.3 evaluation remains the actual fixed-tool assessment. Full provider resource/configuration matrices and ADR review remain open. Task 1.5 awaits separate review; no checkboxes change.

Current model slice validation: 43 Rust tests, fmt, warning-free Clippy and strict OpenSpec 2/2 passed. Initial model tests failed compilation on the absent domain API; a separate blank unknown-reason regression then failed behaviorally before validation was added. No production language provider is claimed.

## Evidence producer slice plan — tasks4.1/4.2, basec5cb983

Root delegates this slice to GitGuard implementer while prior AGowner works elsewhere; no concurrent AG writer. Use executing-plans/TDD. Preserve legacy CLI/schema/exits and existing43tests. Add private prepared Cargo analysis that freezes protected caller policy, isolated input inventory/tool identity and required coverage before GE BoundAttempt; run actual Cargo metadata after binding, retaining declaration provenance as domain attachment. New opt-in library producer uses actual GE bounded evaluation and exact byte artifacts/strict envelope. Missing context/preparation errors yield TransportDiagnostic; post-bind tool failure/cancel produce error/cancelled null; valid coverage gaps stay completed partial/BLOCK. No caller-provided facts or completed decision enters producer.

Candidate/base context is controller-resolved local input, structurally checked, explicitly advisory/no Git object proof or producer authenticity. Actual GG candidate proof belongs4.4. Complete4.1 restricted Cargo-declarations profile is the target;4.2 acceptance depends on exact reviewer reading of verified candidate prerequisites. No provider/grant/remote operations, no replacement CLI, no fields added to legacy GuardFacts/Report. Contract/member limits apply before profile/coverage expansion; output serialization bounded; GE shared evaluate_bounded supplies report expansion guard. Tests real Cargo fixtures, schema/digest/decision tampering, profile drift, error/cancel/partial, native parity and oldCLI regressions. Update Cargo description to ArchGuard. No task checkboxes before root review.

Evidence producer slice implemented for independent4.1/4.2 review: actual Cargo metadata only, private prepared inventory, actual GE BoundAttempt/evaluate_bounded/finish, strict byte-referenced native+domain bundle, explicit profile scopes and declaration provenance, no legacyCLI change. ADR0002 states controller-resolved advisory candidate labels are not Git object proof;4.4 remains unimplemented.4.1 submitted as complete restricted Cargo-declarations profile;4.2 execution-state behavior implemented, verified-candidate prerequisite requires reviewer scope ruling. No task checked by implementer.

TDD: initial producer/budget scaffold RED0/2→GREEN2/2; rehashed invalid manifest_digest RED0/1→rejected; duplicate native fields lost through Value RED0/1→typed artifact deserialization rejected; foreign artifact namespace with intact bytes RED0/1→canonical reference rejected. Logs archguard-evidence-{red-producer,green-producer,red-domain,green-domain,red-duplicate,green-duplicate,red-uri}.log in external ledger. Expanded tests cover real Git context+Cargo declaration parity, protected input freeze, missingOID/root diagnostics, real metadata failure versus legacypartial, cancellation, validpartialBLOCK, review/advise unchanged and actual600facts×256rules bounded refusal. Initial test mistakenly named nonexistent load_report_json, corrected to actual strict serde GuardReport before behavioral assertions; not counted as RED.

Final locked offline all-target suite56passed (original43 plus13new), Clippy-Dwarnings and AG-package fmt/diff clean; strict OpenSpec1/1. Source GE observedcc0e4e0773ce797235931f379089408b2bcd3bee. Explicit serde1.0.229/tempfile3.25.0/time0.3.44 reuse preexisting lock versions; only direct dependencies/formatting feature added. Cargo package description now ArchGuard-focused; historical wire namespace unchanged. Actual producer exporter emits exact contract/facts/report/domain/envelope/bundle and real source.bundle; root replaces synthetic AG wrapper in shared matrix. Detailed archguard-evidence-slice-report.md and golden provenance in external ledger.

## Next independently actionable slice plan — complete2.5 system rules

Root authorized proceeding after evidence commit/review handoff. Use existing LanguageObservation as explicit local provider/fixture input, never claim Java/TS/Rust extraction exists. Freeze controller-owned rule profile with required module/relation/provider coverage and budgets before evaluation. Implement three deterministic rules in src/domain/rules.rs: layer direction, explicit forbidden module direction, dependency cycles. Add minimal read-only SymbolId getters if required; preserve structured cross-language identity. Return source-bearing findings; cycle witnesses must follow actual observed edges and retain every edge SourceSpan. Unknown relations, absent required module or incomplete DependsOn coverage remain partial/missing and cannot become empty success. Bound nodes/edges/rules before graph expansion; use iterative deterministic traversal and bounded witness count, not unbounded all-simple-cycle enumeration.

TDD tests/system_rules.rs: each rule has legal, violating and missing-coverage fixtures; graph-order permutations stable; actual cycle path+source checked, disconnected/incomplete graph unknown; unsupported relation/profile refused; rules cannot weaken via observation edits. Project only explicit AG-owned violation facts via exact existing forbid_relation into actual GEbounded evaluation, with partial mappedBLOCK/INDETERMINATE. Domain semantics stay in AG. Do not mark2.5 from model alone: deliver executable evaluator+projection+all three rule matrices, docs and real tests before review. No production provider, protected baseline, remote execution or shared GE changes.

## System rules slice — task 2.5 submitted for independent review

Implemented complete local evaluator over existing LanguageObservation (no language provider claim): frozen module/provider/rule policy; layers, exact forbidden direction, iterative deterministic actual-edge cycle witness; missing module/provider/relation coverage stays partial/BLOCK. Exact native forbid_relation projection delegates enforcement to GE evaluate_bounded. Typed read-only result retains complete source-bearing witnesses plus native contract/facts/report. Unsupported Calls policy rejected. ADR0003 defines internal/self/cross-language edge semantics, local trust limits, global completeness and budgets.

Initial absent-module test RED, then six behavioral tests GREEN. Separate 64 repeated 1024-module layer-map test reproduced missing pre-serialization policy byte limit; fixed before hash/allocation amplification. Expanded tests cover each enforce legal/violation/missing case, review/advise, contiguous source witnesses, permutation stability, unknown/provider loss, self/disconnected/internal edges, protected scope/profile hashing, graph/policy budgets and multilingual module separation. Task 2.5 remains unchecked pending review. Acceptance remains 5/25 (root accepted 4.1 at 01f2f13; archguard-producer-independent-review.md). 4.2 stays partial for actual Git/source proof; this slice does not weaken that requirement.

Final slice verification: 67 tests pass (prior 56 + 11 system-rule tests), warning-free all-target Clippy, package fmt/diff clean and strict OpenSpec 1/1. Reused existing target, no new dependencies/cache. Detailed commands/RED evidence are in external archguard-system-slice-report.md. Root's separately committed frozen capability document is unchanged by this slice.
