# ArchGuard-Plugin task 4: host discovery

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
