# Bounded Cargo Runner Implementation Plan

> Execute directly with executing-plans and test-driven-development; root already authorized implementation and independent review. No subagents or additional approval step.

**Goal:** Complete task1.4's bounded fixed-tool declaration runner for a precisely supported Linux profile.

**Architecture:** Keep the existing public Budget/run/IsolatedProject interfaces and typed failures. Add inherited Linux process-group confinement and descriptor-relative source copying, retaining private working-copy execution and fixed Cargo/rustc discovery. No candidate script execution or change to legacy CLI partial behavior.

**Tech Stack:** Rust1.90, existing libc, Linux x86_64/aarch64 seccomp, openat/O_NOFOLLOW, existing Cargo metadata.

**Spec:** openspec/changes/extend-architecture-analysis-and-evidence/specs/architecture-analysis-profile/spec.md — Bounded Architecture Execution; task1.4.

## Global constraints

Source/candidate files cannot choose executables or weaken budgets; enhanced tool/budget/path errors are failures, never empty complete graphs. Worktree lockfiles unchanged. Keep legacy CLI/schema/exits, actual GE/GG source semantics and pending SG review separate. CARGO_INCREMENTAL=0, reuse target after reviewer releases it. No checkboxes before independent acceptance.

## Review focus

- A child calls setsid/setpgid, outlives timeout and writes a late marker: deny escape and terminate the entire retained group.
- A leader exits early while children hold pipes: retain leader PID until cleanup, deadline remains bounded.
- Source directory or file swaps to a symlink/FIFO during copy: descriptor-relative nofollow opens and fstat reject links/special files without blocking or reading outside.
- Caller uses zero/oversized/overflow budgets: typed refusal before execution or allocation; default values remain unchanged.
- Too many/too large/deep files and copy deadline: stop before byte/count/time amplification; real Cargo metadata and isolated fake-tool writes cannot change original lock.

## Task1: Process boundary (src/analysis/runner.rs, new runner/linux.rs; tests/runner_failures.rs)

- [x] Add finite-lived setsid late-marker regression; retain existing timeout/nonzero/output tests. Observe RED before adding filter.
- [x] Install allocation-free prepared seccomp filter in pre_exec after initial process_group setup; deny session/group escape and namespace changes, inherited by descendants; reject unsupported Linux ABI/platform explicitly.
- [x] Observe leader using waitid WNOWAIT, kill group before reaping so numeric PID cannot be reused during cleanup. Test early parent exit/closed pipes plus timeout paths.
- [x] Validate finite bounded budgets before spawn, preserving Failure variants and exact ordinary exit codes.

## Task2: Source boundary (src/analysis/runner.rs, new runner/copy.rs; tests/runner_failures.rs)

- [x] RED zero/overflow budget and controlled copy deadline tests; concurrent symlink replacement probe asserts no external marker is ever copied.
- [x] Open absolute canonical source one component at a time without following links. Iterate pinned directory fds and open children relative to each fd; nonblocking nofollow open, fstat regular/directory only. Enumerate /proc/self/fd only on supported Linux, never candidate-provided proc paths.
- [x] Enforce count/depth/file/total/deadline during bounded chunk reads, including mutation after initial metadata. Use private random temporary directories with mode0700 and existing manifest/config rejection.
- [x] Test every budget, special input, manifest escape, actual Cargo lock invariance and precise failed-vs-partial behavior.

## Verification and handoff

- [x] Focused RED/GREEN logs and full `cargo test --locked --offline --all-targets`.
- [x] Rust1.90 targeted/full as justified, Clippy-Dwarnings, fmt/diff and strict OpenSpec.
- [x] Document actual Linux trusted-fixed-tool profile, unavailable external sandbox capabilities and exact limits without equating copy isolation to arbitrary native-tool confinement. Local commit plus ledger report for independent task1.4 review.

Local capability probe: bwrap user namespace setup fails because uid map is read-only; Landlock ABI query returns ENOSYS. This implementation therefore does not promise a general hostile-native-tool sandbox. Seccomp/group containment and descriptor-bound input copying are executable controls within the existing fixed-tool declaration profile.
