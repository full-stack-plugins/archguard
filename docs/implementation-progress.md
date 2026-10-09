# ArchGuard implementation ledger

Base: 9ed191acf3c3b1f9c0ca9edf3d0621b6a7c53f04. Local branch only. Root acceptance checkpoint: 9/25 tasks accepted (1.1, 1.2, 1.3, 1.5, 2.5, 3.4, 4.1, 4.2, 4.4); restricted Cargo scope only. Producer 01f2f13 independently accepted for 4.1 in external archguard-producer-independent-review.md. Tasks 4.2/4.4 were independently accepted for the actual Git wrapper after acf6116; the older public candidate context remains unverified. Task 1.4 remains partial. New task 2.1 typed model slice is partial and pending review.

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

## Next slice plan — actual Git candidate source, tasks 4.2/4.4

Use actual GitGuard Repository/CandidateSnapshot validation and a separately reviewed additive bounded tree/blob reader from its isolated object store. No reads from mutable caller working tree to construct analyzed source. GG runner owner is finishing 1.2 first; no concurrent GG edits. Protected AG policy digest must equal GG task scope policy digest. Materialize regular UTF-8 candidate tree files into a private temporary directory, reject unsupported modes/config/ignored target paths, and compare the exact copied file inventory before analysis. Freeze GG snapshot (including base/group/members/requirements/allowed paths), object format and source file identity into analysis inventory before GE prepare_attempt. Old Cargo producer/profile/wire stays explicitly advisory; a distinct strict Git wrapper exposes verified local candidate/source binding without a production identity claim.

TDD actual SHA1/SHA256 repositories and non-HEAD queue candidate; nonexistent/corrupt/cross-repo objects rejected before envelope, changed worktree irrelevant after preparation, policy/base/group/member/requirements changes invalidate wrapper identity, cancellation/tool error retain bound null. Exact source blob mapping plus copied inventory equality, including manifest and Cargo.lock, prevents independent caller OID labels. No worktree checkouts/hooks/submodule/network or remote writes. GitGuard remains advisory, and reader's own limitations are documented. AG's direct versions align with actual GG exactserde1.0.228/tempfile3.23 dependencies; required MSRV moves1.85→1.90 because the real dependency declares1.90, not because the current compiler is newer. Full native regression and official1.90 check before acceptance. New binding/checks do not retroactively bless the legacy context constructor.

Root independently accepted 2.5 local LanguageObservation profile at 50ca45e after exhaustive 512 three-module directed graphs against independent Kahn cycle oracle, actual source paths, permutations and GE verification. See archguard-system-independent-review.md. Current accepted count 6/25. Real language provider and Git/source proof remain separate.

Actual candidate bridge implemented for independent 4.2/4.4 review: GG c33b28a bounded regular-file reader over discovery-fsck-verified private objects, exact candidate-only materialization and copied file inventory equality, full GG snapshot/object format/file identity frozen before GE binding. Separate strict Git wrapper requires actual repository plus expected snapshot; ordinary Cargo context remains explicitly unverified. AG measured source digest is preserved, not replaced with GG's differently scoped tree digest. No provider/authentication/CAS/write capability added.

Final verification: 74 tests pass on both default Rust1.99 and official1.90, all-target Clippy/fmt/diff and strict OpenSpec1/1 GREEN. Seven new actual candidate tests cover SHA1/SHA256, mutable/nonHEAD isolation, real two-parent merge vs member head, invalid objects/repo/policy, base/group/member/requirement/source invalidation, cancelled null, actual metadata bound error and completed partial. Legacy tests and producer golden remain intact. Actual new exporter example and independently generated Git bundle/source.bundle support FG scoped-source handoff. AG directserde/tempfile versions align GG exactpins; MSRV1.90 follows real dependency, with actual test evidence. Source dependencies at handoff: GG95eb3d9 (reader c33b28a), GEdcfd197 (code unchanged by later docs). Task4.2/4.4 unchecked; accepted count6/25.

Independent review found P2 in Git wrapper: rehashed domain could omit file:Cargo.toml or add a foreign file/Git identity while passing two-key presence checks. TDD reproduced all three; verify now compares exact actual candidate file-key sequence and exact reserved Git identity namespace. Five mutation variants tested. No producer/golden/source digest change and no task accepted by this correction;4.2/4.4 still await independent re-review.


## SG architecture handoff reader plan — SG2.7 / AG3.4 local profile

Root accepted local4.2/4.4 after independent inventoryfix acf6116, report archguard-candidate-independent-review.md, total8/25. Getter95c353b lets controllers pin actual AG source beforeexecution. Neither supplies productionidentity.

Next: actual SG architecture::decode with externally owned exact ExpectedHandoff; immutable explicit requirement/ADR/GGrequirementalias→existingprotectedCargoRule mapping, budgets before expansion. Require exact scope/reference pair coverage, stable SGTestObligation export under existing fixture-only approval boundary, fresh time/revocation validation. SG supplied text/ref data must not create or edit Cargo policy. Read every SGsnapshot.content path backward from actual GG candidate blob to reject dirty/missing/noncandidate source. Freeze full trace receipt/baseline/mapping digests as additional AG inventory identity before real Cargo producer. Separate strict traced wrapper retains rawhandoff digest, fullbaseline, source refs and actual SG obligations; independent read/verify requires expectedmapping/rawSGbytes/repo/GGsnapshot/currentfixtureport/clock. Keep native/basicCargo scan independent of SG service availability. No production provider, approved baseline promotion or newGE/SGpolicy logic.

TDD actualcommitted Cargo+Markdown repository→SGdiscover/freeze/export→AGread+GGsourcechecks→realCargo/GE; stablebaseline/requirement/ADR/obligation IDs, scope/version/context/digest/source tampering, time/revocation/unavailableport, invalid/missing/mismatched mappings and no policy rewrite. OldSGdirty-empty-tree golden cannot prove Git correspondence. Do not edit SG; owner separately implements its Git source wrapper. Existing baseline fixture remains explicitly fixture-only; its historical placeholder source revision is not claimed actual Git proof. AGtask3.4 acceptance localprofile/production limitations to reviewer; no implementercheckbox.

SG reader slice implemented for independent interoperability review. Seven actual temporary-repository tests exercise real SG export/decode and backward GG commit-byte verification, explicit protected rule mappings, preserved shared baseline/obligation IDs, dirty source rejection, expiry/revocation/unavailable port, strict receipt/nested native JSON and rehashed SG inventory mutations, cancelled null and SG-independent basic Cargo. ADR0005 defines fixture-only authentication and historical baseline_obligations; neither is production approval or current candidate test coverage. Full trace receipt participates in actual AG measured inventory before the GE attempt; scoped source digests remain distinct.

Initial absent-API compile RED and a separate oversized external-context behavioral RED are retained in the external ledger. Default and official Rust1.90 locked offline all-target suites each pass82tests; warning-free Clippy, fmt and strict OpenSpec pass. Actual exporter example produces SG handoff plus traced/Git/native evidence and source.bundle. Stable dependencies SG e0c7559, GG95eb3d9 (reader c33b28a), GE6527e2a. Details in archguard-sg-slice-report.md. Only already accepted4.2/4.4 checked in this commit; total8/25. Task3.4 remains unchecked pending independent review of this local interoperability profile;3.1/production approval/providers remain separate.


Root accepted task3.4 local fixture-profile interoperability at1d6d1e3 after independent82tests+2probes, SG7writer tests, real source.bundle reconstruction and all15 raw artifact hashes; report archguard-sg-independent-review.md. Total9/25. Scope excludes production authentication, historical baseline Git proof and TestGuard execution; task3.1 and trusted-operation groups remain separate. New task1.4 work follows docs/superpowers/plans/2026-10-09-bounded-cargo-runner.md and is not accepted yet.


## Task1.4 bounded fixed-tool Linux slice — pending independent review

Behavioral RED reproduced escaped setsid descendant writing after timeout, invalid budgets reaching execution and copy ignoring deadline. The Linux runner now installs inherited process-group/namespace confinement, observes leader exit without reaping before group cleanup, limits cleanup wait, and validates all budgets before spawn/copy. Source traversal pins directory descriptors, rejects symlink/special inputs without blocking and enforces byte/count/depth/deadline checks during copy into private random temporary directories. Actual Cargo and controlled fake tool tests preserve source lockfiles and enhanced error versus legacy partial exits. ADR0006 precisely scopes Linux x86_64/aarch64 fixed trusted tools; no general hostile-tool OS sandbox or production authentication claim. Task1.4 remains unchecked until review; accepted count9/25.


Task1.4 validation:91all-target tests pass on default Rust and official1.90, including15runner tests plus the pinned-directory replacement unit test; Clippy-Dwarnings, fmt/diff and strict OpenSpec pass. Exact RED/GREEN commands, final dependency archive verification and platform limitations are recorded in external archguard-runner-slice-report.md. No new dependency or MSRV increase. Task1.4 remains unchecked for independent review.


## Bounded fixed-tool runner accepted

Task 1.4 accepted at `22c1716183840a27c93400f9a2b8b0c074cdb328` after independent fixed-source review. All 91 maintained tests and an additional raw-syscall probe passed. Actual setsid escape attempts, pipe-holding/background descendants, invalid budgets, dual-stream output bounds, FIFO/path-replacement copying and Cargo lockfile isolation were exercised. WNOWAIT keeps the owned leader unreaped until process-group cleanup; inherited seccomp restricts group/namespace escape. Descriptor-relative copying rejects symlink/FIFO escapes and checks count/bytes/depth/time budgets.

Acceptance is the tested Linux x86_64 trusted fixed-tool/isolated-copy profile. It does not qualify arbitrary hostile executables, general filesystem/network sandboxing, process-count limits, hard interruption of filesystem syscalls, aarch64 execution or production authority. Enhanced tool failure remains error4 and legacy partial remains BLOCK2. Evidence: cloud ledger `archguard-runner-independent-review.md` and associated fixed-source suite/probe logs.


## Fixed JDK static bytecode provider — pending independent review

Tasks 2.1/2.2 retain the existing typed language model and add actual OpenJDK 21 classfile observations under a frozen required binary-type → logical-module profile. Exact installation and helper bytes participate in identity. The helper parses bytes without loading candidate classes, emits bounded native captures, and preserves JVM method descriptors and actual capture provenance. Static declared dependencies feed the existing system-rule/GE projection. Missing/version/opaque/reflection gaps stay explicit; Calls never claims complete runtime coverage. ADR0007 and analyzer-capabilities.md specify supported relations, tool license and resource/platform limits.

Initial absent-API RED plus behavioral REDs caught non-ASCII record panics, trailing bytes, swapped classfile names, Cargo-specific target-directory omission and lossy UTF-16 identities. Default and official Rust 1.90 all-target suites pass 103 tests; strict Clippy, fmt and OpenSpec pass. Real fixtures cover allowed/forbidden/missing type dependencies, overloads, nonexecuted static initializers, reflection, dynamic dispatch, unknown versions, malformed input and pre-expansion budgets. Actual native exporter retains compiler/helper receipts, bytecode, capture, javap and GE contract/facts/report. No Java envelope, production authority, JAR expansion or runtime call graph is claimed. Tasks remain unchecked pending independent review; accepted total remains 10/25.


## Reviewed language model and Java static provider

Tasks 2.1 and 2.2 accepted at `ff601c0a55035616cdc1ebccb33b36efc36a017b` for the versioned model and fixed Linux x86_64 / Debian OpenJDK21.0.12.1 static classfile profile. Independent Rust1.90 regression passed103 tests; the actual exporter regenerated four cases with byte-identical observations and GE artifacts. Legal and NeverRun yield ALLOW, Forbidden and Reflective yield BLOCK. An independent Class.forName control proves the same NeverRun class would fail on initialization, while the provider reads it without loading candidate code.

The helper uses the pinned JDK classfile library and fixed tool/helper bytes. JVM descriptors distinguish overloads; native-record provenance is not fabricated Java source positioning. Missing bytecode and unresolved/dynamic/reflection behavior retain unknown coverage; Calls is never complete. Unsupported format, malformed/trailing/swapped classfiles and expansion limits fail. This does not qualify Java source-to-candidate authentication, Java CLI/envelope, runtime completeness or other language providers. Evidence: cloud ledger `archguard-java-independent-review.md`, real golden and independent regenerated captures.


## TypeScript semantic provider — task2.3 pending review

Actual TypeScript5.9.3 compiler API on fixed Node24.19.0 now consumes protected source/path mappings through a closed CompilerHost. Exact compiler/standard-library hashes and required source bytes participate in identity. Real aliases, cross-package declarations, top-level types, overload signatures, constructor/type-query origins and source spans feed the existing system-rule/GE evaluator. Project references and unknown controller options are explicitly rejected; candidate tsconfig cannot expand scope. Dynamic/unresolved imports, missing source, compiler diagnostics, recursive aliases and unsupported nested identities remain partial. Calls never becomes complete. No new package download, parser, JavaScript execution, evidence envelope or authority claim.

Initial absent-helper/API RED plus real nested-identity and global-constructor REDs are preserved externally. Eleven real provider tests cover legal/forbidden/missing GE decisions, literal/nonliteral imports, scoped alias failures, source identity/determinism, recursive types, config rejection, source symlinks, tool identity and input/record budgets. ADR0008 records licensing and exact limits, including the unavailable exact installed Node license notice. Full fixed-source verification and native exporter artifacts are recorded in archguard-typescript-slice-report.md. Task2.3 remains unchecked; accepted total remains12/25.
