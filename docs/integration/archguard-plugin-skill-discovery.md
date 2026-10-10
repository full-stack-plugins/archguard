# ArchGuard-Plugin task 4: wrapper discovery

## 2026-10-10 local wrapper delivery

The already approved implementation scope includes a reversible, independent cloud-local wrapper directory. No further location approval is required. The wrapper now lives at `/workspace/guard-implementation/archguard-plugin`, separate from this Rust engine, on `impl/archguard-plugin-routing-20261010`. It has no remote repository and makes no marketplace changes.

The local package provides three platform-shaped manifests, one `skills/archguard/SKILL.md` entry and a pinned local provider reference to `java-skills/archunit` at `64847f09215e05b4c92946ad365cfeca4d0c63e7`. It introduces no runtime, MCP, controller, state owner or duplicate Java rule implementation. Java builds use their wrappers directly; existing supported engine contracts use the engine's documented CLI.

Fresh agent application followed manifest → entry skill → provider registration/revision/content hashes → Maven and Gradle native checks. Each returned exit 0 and fresh XML with three executed tests, zero failures/errors/skips, and passing import coverage. All 562 tracked provider files were unchanged. Five static discovery/provenance checks passed; the same fresh agent found no important wrapper review blocker. Missing-provider, ambiguous-scope, non-Java, unauthorized-write, stale/empty-test and incomplete-coverage responses were examined as explicitly labeled application scenarios, not repeated native negative builds. Existing full Java/core qualification was not rerun.

Evidence: `/workspace/guard-plugin-routing-20261010/wrapper-application-report.md` and adjacent `wrapper-application-*` raw records. This is a local agent execution profile. Actual Codex/ZCode/Kimi plugin installation and official host schema validation remain **UNVERIFIED**. Codex CLI help is available; ZCode/Kimi executables are absent. No installed-host end-to-end success or public availability is claimed. The provider is an explicit sibling checkout prerequisite, not a self-contained published package.

This delivery supersedes the location blocker below. Historical discovery findings remain useful evidence of why no remote wrapper or marketplace entry was assumed.

## 2026-10-10 correction: missing wrapper source, not a missing host API

The earlier requirement to find a separate native skill API was too broad. The actual ecosystem uses plugin manifests and progressively loaded `skills/<name>/SKILL.md`; an agent can select and read `java-skills/archunit` through those conventions. No new runtime, MCP server, controller or Rust-to-skill API is necessary. Per the confirmed engine/wrapper separation (PAPR-001), `full-stack-plugins/archguard` remains the independent engine; a plugin wrapper is a separate delivery target.

Read-only recheck found:

- `full-stack-plugins/archguard` remote main/tree `566fda92c7186a38e53ac81347ba23e0a8107443` contains the Rust crate and no `skills/`, `.codex-plugin/plugin.json`, `.zcode-plugin/plugin.json` or `kimi.plugin.json`. The retained local engine implementation is `6fbcb9e6c51717a396e6e83ee8cc706b5253d655` before this documentation correction.
- Exact authenticated `full-stack-plugins/archguard-plugin` lookup returned 404. Organization-scoped repository search returned 19 repositories including `archguard`, `codeguard-plugin` and `flowguard-plugin`, but no `archguard-plugin`. This establishes no accessible exact target, not universal nonexistence of private repositories. The installed-repository query returned unrelated organizations and is not treated as a complete organization inventory.
- The [organization profile](https://github.com/full-stack-plugins/.github/blob/e0c8b638441915241583cedd4d5c1cc549ccab55/profile/README.md) identifies the three platform manifests and skill directories. [CodeGuard's actual manifest](https://github.com/full-stack-plugins/codeguard-plugin/blob/58ecd1f923609e980dc47266a741f7139471917a/.codex-plugin/plugin.json) registers `"skills": "./skills/"`; its entry skill demonstrates agent-selected routing. These are conventions, not an ArchGuard-specific API.
- The marketplace [catalog](https://github.com/partme-ai/full-stack-plugins/blob/86071c52c6a59c96f861239e4319650e73d101ba/catalog.json) and generated `.agents/plugins/marketplace.json` contain no ArchGuard entry. No installed ArchGuard path exists under this environment's empty `/workspace/.agents` directory. No marketplace or installation configuration was changed.
- The completed local provider `java-skills` at `64847f09215e05b4c92946ad365cfeca4d0c63e7` registers `./skills/archunit`. This local, unpushed skill and its verified examples are usable by an authorized agent with that checkout; they are not yet an installed or publicly released ArchGuard plugin.

**Remaining location question:** identify the existing independent wrapper repository/directory, or confirm a new cloud-local `archguard-plugin` directory. Do not require another host API and do not add wrapper manifests to the engine to bypass this boundary. Once the wrapper location is selected, use the existing manifest/skill conventions, preserve provider revision and native build evidence, and verify agent discovery/routing. Actual Codex/ZCode/Kimi installation remains a separate validation claim. No previous Java/core acceptance needs repeating.

The approved route remains: Java + one resolved Maven/Gradle module → load the available registered `archunit` skill → pass project/package scope and existing write authorization → run that skill's native checks → report selected skill revision, diff, command/exit, test discovery/coverage and PASS/VIOLATION/UNVERIFIED. Missing skill, ambiguous module, unauthorized edits and empty coverage must not become PASS; non-Java projects do not select ArchUnit. This is an integration recipe, not a claim that wrapper execution has occurred.

Read-only evidence is retained in the cloud delivery folder `guard-plugin-routing-20261010/discovery-evidence.json`. The prior investigation below remains historical; its "native skill API" gate is superseded by this correction.

## Historical 2026-10-09 investigation

Date: 2026-10-09. Outcome: discovery completed; plugin implementation and host end-to-end validation remain BLOCKED because no actual ArchGuard-Plugin host source, manifest, or native skill API was located. This does not block the independent Java skill or Rust core work.

## Verified sources

- `/workspace/.agents` is empty. A workspace manifest scan excluding build outputs, Git internals, and gpui found Java Skills manifests in the audit and implementation checkouts, CodeGuard plugin manifests, and the legacy FlowGuard marketplace. None identifies ArchGuard-Plugin. Existing CodeGuard/FlowGuard plugins are not substitutions.
- Existing canonical ArchGuard checkout: `/workspace/workspace-agent-skills/full-stack-plugins-repositories/archguard`, origin `https://github.com/full-stack-plugins/archguard.git`, local HEAD `9ed191acf3c3b1f9c0ca9edf3d0621b6a7c53f04`. Its root contains Cargo.toml, src, tests, docs, examples, fixtures, openspec, and .github; no plugin manifest or skill directory.
- Active Rust implementation checkout: `/workspace/guard-implementation/archguard`, observed HEAD `583e0600781a931f79aed828e4f9d73d726cb477` (another agent owns ongoing changes). Cargo package is `archguard`; source/test search did not identify a native skill discovery/invocation API. Its only source/test match for `skill|plugin` was a TypeScript compiler-plugin rejection test. No changes were made there by this task.
- Authenticated GitHub `get_repo(full-stack-plugins/archguard)` succeeded, confirming repository access and default branch `main`. Current `commits/main` returned `566fda92c7186a38e53ac81347ba23e0a8107443`, commit tree `6a2ee0186dd24f687b421ae69e03ec74d2b0a679`. The recursive `git/trees/main` response was not truncated and listed the Rust crate, docs, fixtures, and tests; it contains no plugin manifest, skill directory, or host invocation files. Raw responses are preserved, including the tree endpoint's returned SHA.
- Authenticated direct lookup of the candidate `full-stack-plugins/archguard-plugin` returned `404 NOT_FOUND`. This means unavailable under that exact name to this connection, not proof that no private or differently named host exists. The owner-filtered repository listing returned an empty list despite successful direct access to ArchGuard, so listing/search absence is not treated as authoritative proof of nonexistence.
- Existing Java skill provider: `https://github.com/full-stack-skills/java-skills`, audit checkout `/workspace/java-skills-design-audit-20261009`, immutable baseline `d981268c4baaa32e4e068a47b95db13e5a1c526e`. Registration is the `skills` array in `.claude-plugin/plugin.json`, with relative entries `./skills/<name>`. The baseline has no `archunit` entry. Tasks 1–3 own adding `./skills/archunit` in `/workspace/guard-implementation/java-skills`; the provider manifest is evidence of provider registration, not evidence of a host discovery API.

Raw local outputs and authenticated GitHub responses: `/workspace/guard-approved-implementation-20261009/archguard-plugin-discovery-evidence.json`.

## Exact remaining gate

Supply or identify the real host repository/check-out and readable revision. Before host code can be changed, record its instructions, manifest, native discovery and invocation files, and actual test command. Coordinate against the final Java skill commit and `archunit` identity; consume real build/module/package scope through the host's native interface. No replacement runtime, invented API, or Rust host scaffold was created.

Once that gate is satisfied, task 4 requires native RED/GREEN coverage for Maven and Gradle selection, missing skill diagnostics, ambiguous build/module scope, non-Java exclusion, unauthorized-write prevention, and empty/insufficient-test false-pass rejection. Then run actual host discovery → invocation → native Maven/Gradle execution against tasks 2–3 fixtures and preserve the skill revision, command, diff, reports, and pass/violation/unverified status.

## Delivered and not executed

This report is ready to copy into the core-owned checkout as `docs/integration/archguard-plugin-skill-discovery.md`; the root/core owner performs that integration to avoid shared-worktree conflicts. No host branch/commit was created because no host was found. No host tests or end-to-end runs were executed or claimed. No push, PR, remote mutation, gpui change, or modification to the core/Java checkouts was made.
