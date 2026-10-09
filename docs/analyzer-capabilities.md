# Analyzer capabilities — local implementation, explicit profiles

## Current boundaries

| Profile/API | Observed capability | Limits |
|---|---|---|
| legacy CLI | Original Cargo direct-member path declarations | Original manifest digest and metadata-error partial/BLOCK; unchanged CLI/schema/exits |
| cargo-declarations-v1 / CargoEvidence | Frozen protected Cargo scope, actual bounded metadata, isolated inventory and GE native evidence | Declared dependencies only; public CandidateContext remains unverified |
| GitCargoEvidence | Actual GG candidate/private-tree source equality and scoped Cargo evidence | Local verification, not production identity or admission |
| TracedCargoEvidence | Actual SG fixture-profile references reverse checked against candidate blobs | Historical baseline obligations only; no production approval or candidate test execution |
| System rules | Layers, forbidden direction and real-edge cycles over local LanguageObservation | Source provider scope/completeness remains explicit |
| Java21 classfile API | Actual selected classfiles → Module/Type/Method identities and static DependsOn | Exact installed JDK/profile below; no complete Calls or runtime graph |
| TypeScript5.9.3 compiler API | Required source aliases, top-level type/method identities and static declaration dependencies | Exact fixed tools/options; project references rejected, Calls incomplete |
| Rust symbols | Model only | No rust-analyzer adapter/profile implemented |

Cargo enhanced inventory includes isolated files, frozen profile and resolved tool/environment identity. Actual Git and SG wrappers retain their own scoped digests; none is silently substituted for the measured Cargo inventory. Candidate Cargo config, symlinks/special files, escaped manifest paths and ambient ancestor config are rejected. Bounded Linux execution now uses inherited group confinement, unreaped leader cleanup and descriptor-relative source copying (ADR0006, task1.4 independently accepted for the tested Linux x86_64 profile); general malicious-native-tool filesystem/network isolation remains outside this fixed trusted-tool profile.

The existing TypeScript evaluation fixes npm5.9.3 (Apache-2.0) with package/lock integrity in fixtures/languages/typescript. Its earlier actual run on Node24.19.0 took1.296seconds and294048KiB peak RSS, an observation rather than a resource guarantee. No JavaScript production adapter or project-reference acceptance is implied. Rust compiler availability does not imply rust-analyzer capability.

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

## Frozen in-memory language model

`archguard.language-model/v1alpha1` remains the existing model; no wire schema or cross-language resolver was invented. SymbolId structurally separates language/module/owner/kind/name/canonical signature. JVM descriptors now supply real Java method identity, preserving overloads and generated binary methods. SourceSpan identifies snapshot-relative one-based line/column positions with exclusive end; Java spans refer to actual native capture records, not fabricated Java source lines. Confirmed endpoints must be registered. Unknown relations preserve source and reason without guessed targets, and cannot be cleared by marking scope complete. Models remain scoped local provider observations, not authentication or cross-candidate authority.

## Java21 bytecode profile — tasks2.1/2.2 submitted for review

`JavaToolchain::freeze(jdk_home)` requires the exact local Debian OpenJDK21.0.12.1 installation pinned in fixtures/languages/java/jdk-profile.json (release,java,javac,lib/modules and libjvm SHA256). The helper uses installed `jdk.jdeps/com.sun.tools.classfile` APIs; this is a deliberately fixed internal API, not a general JDK-compatible release. Its source and actual compiled class digest plus installed tool identity participate in provider ID. Build args/version/result are available through build_log. Native artifact and module hashes are compatibility checks, not producer authentication.

`JavaProfile::freeze` owns required internal binary name→logical module mappings and explicit external type names before extraction. Logical Module symbols do not claim JPMS semantics. Required binary names use portable ASCII slash names, at most64types; modules≤256bytes; external names≤256entries. Java platform references under java/ are explicitly outside the managed type graph unless listed as required. Other references outside required/external scope produce Unknown. Missing required bytecode or unsupported major/minor creates a retained gap; no empty complete result. Only classfile65.0 (Java21, no preview) is supported. JAR/ZIP discovery, module paths, multi-release archives and classloading are not implemented.

The native dependency finder supplies constant-pool, descriptor/signature, superclass/interface and supported declaration references. Methods retain JVM descriptors. Unknown/unsupported attributes (including annotations, records/module metadata and bootstrap metadata not covered by this adapter) conservatively add DependsOn gaps. Reflection, dynamic constants/invokedynamic and native implementations prevent complete DependsOn. Virtual/interface dispatch stays Unknown Calls even when static type references are complete; Calls is never declared complete. This profile does not establish runtime targets, JVM verification, source-language semantics, transitive external library closure or runtime completeness.

Candidate bytecode is copied through the same fd-bound input boundary with a separate artifact policy; Cargo-specific target-directory/config rules are not applied to Java packages. The source digest is an ordered length-framed hash of exact required .class bytes (including unavailable/version markers), frozen scope and tool/helper profile, not a whole-repository digest. Per-class hashes are exposed. Class internal names must match frozen file paths; trailing bytes and malformed parse/tool output fail, rather than changing coverage to success. Source spans identify lines in returned `captures/java-bytecode.tsv`; records refer to actual class names and instruction offsets. Raw capture is retained so line provenance can be inspected.

The helper is compiled with -proc:none; the runtime classpath contains only that trusted helper. Candidate classes are never loaded or executed, even static initializers. Java environment is cleared, input is explicit classfile paths, no candidate JVM options/agents/processors/scripts are accepted. Resource profile:512KiB/class,4MiB copied input,256directory/file entries,depth32,64required classes;128MiB heap/64MiB metaspace/2active CPUs/SerialGC;20seconds per invocation and1MiB combined output. Parsed native structures are capped at8192constant-pool entries,256methods and512fields/class;4096native records. Limits are checked before dependency/output expansion; JVM caps bound the underlying parser allocation. No OS-wide memory/process-count guarantee is implied.

See ADR0007 for exact library/license and native capture provenance. Valid complete scoped type graphs can satisfy local system rules; missing/dynamic/unsupported coverage remains partial/BLOCK. This library adapter does not add a Java CLI, GE envelope, production identity or release. TypeScript and Rust adapters remain unsupported.

## TypeScript5.9.3 provider — task2.3 submitted for review

`analysis::typescript::{TypeScriptProfile,TypeScriptToolchain}` uses the actual fixed compiler API in a closed in-memory CompilerHost. Required .ts/.d.ts files→logical modules and paths aliases are frozen before extraction; ES2022/ESNext/Bundler/strict/noEmit options are fixed. Project references are explicitly rejected, as are candidate tsconfig files and unknown controller configuration fields. Standard libraries and compiler JS are copied only after exact pinned hash verification; no candidate module is executed or emitted.

Supported observations: file/module declaration dependencies through resolved import/export/type-reference, constructor and type-query declarations, top-level types and canonical overloaded method signatures, all with real source positions. Literal dynamic imports may have confirmed static targets; nonliteral imports, missing source, compiler diagnostics, recursive aliases and unsupported local/nested identity scopes retain partial/Unknown. Calls never becomes complete. This does not claim whole-program type dependency closure, cross-language edges or runtime behavior. See ADR0008 for exact budgets, tool/license scope and source digest semantics. Real fixtures and native/GE artifacts are generated by `export_typescript_evidence`; the earlier `evaluate.cjs` remains an evaluation fixture rather than the production adapter.
