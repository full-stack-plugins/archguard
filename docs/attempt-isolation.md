# Local candidate attempt isolation

`integration::binding::attempts` adds an explicit controller-owned history for the existing Git Cargo declaration profile. It does not change the legacy CLI, generic Cargo evidence, domain-contract or traced-baseline APIs. Its consistency checks do not authenticate a producer or approval and do not grant merge eligibility.

## Registration and completion

Prepare `PreparedGitAttempt` with the independently selected Repository, CandidateSnapshot and ProtectedCargoPolicy. This runs the existing immutable source preparation. The generated run ID is retained before execution; its existing generation scheme is unchanged. The full-work dedup key includes the complete Cargo binding, actual GG candidate binding digest, protected contract/profile, required coverage and analyzer/version. The source digest already binds the declared source, configuration and tool inventory. It remains the Cargo-scoped identity, not a substituted GG tree digest.

Register the prepared value with `AttemptHistory::register(prepared, expected_generation)`. Registration uses GE's existing advance operation and clears any previous current result immediately. It returns an opaque `RegisteredGitAttempt`; executing it consumes the prepared value and yields a sealed `CompletedGitAttempt`. No deserialization, relabeling or mutable artifact access is exposed for these controller-local objects. Repeated preparation produces a distinct run ID even when the dedup key is identical.

The private full target is repo/task/worktree/requirements. Separate GE stores retain worktree isolation without changing GE's wire Target. Candidate, base, queue members/group, source, profile and coverage changes alter the work identity; advancing the same target invalidates old publication. There is no source-only extraction cache in this API. A source digest or identical extraction cannot satisfy another obligation.

## Import, publication and read checks

`import` verifies the existing Git evidence bundle against the actual repository and independent expected candidate on every call. It checks exact run, full binding, frozen coverage, work digest and completed contract digest before GE append. Identical completion replay is idempotent. Only a completion produced by the registered attempt in this history can be imported; arbitrary changed bytes are not an accepted input type.

`publish` requires the identical imported record and GE's latest generation. An older ALLOW may be recorded historically but cannot overwrite a newer BLOCK. `current_matches` additionally requires the independent protected policy and complete expected Cargo RunBinding, re-verifies candidate/artifacts, and compares the complete current record. The protected controller must supply that binding from its independently frozen expected work, including the expected Cargo-scoped source/tool/config identity; copying it from an uploaded result is not independent verification. The check does not implicitly rerun extraction to discover expectations. It is a local integrity/current-state check: true is not eligibility. Error, cancelled and partial attempts can be current observations; history records always carry `eligible=false`. The returned history contains bounded metadata, not raw evidence or a promise of continuing availability.

There is no approval/producer cache or authority provider. Trace receipt and external baseline approval checks remain on their existing independent interfaces. This basic Git Cargo entry does not requalify traced evidence by dropping its trace. Task 5.1 authority and expiry/revocation semantics remain separate.

## Limits and concurrency

Before source preparation, cloning or hashing, a nonallocating serialization counter admits the candidate within 64 KiB and at most 64 requirement IDs, and the protected policy/profile inputs within 256 KiB. Existing ProtectedCargoPolicy limits still apply. The final frozen descriptor is capped at 256 KiB, completed bundle admission at four GE artifact caps plus 2 MiB, and each history at 256 registrations. Failed registration cannot retain an empty target or replace current state. The record run ID cap is 128 bytes; actual generated IDs use the existing SHA-256 format.

Mutating operations require exclusive mutable access. A caller Mutex can serialize history operations while registered runs execute concurrently outside the lock. Tests use real threads for two requirements × two worktrees and competing same-generation registration, real SHA-1/SHA-256 Git candidates and actual Cargo declaration analysis. An allocator probe rejects a 17 MiB candidate field without any allocation of 4096 bytes or more inside preparation.

This is process-local state, not durable or multiprocess CAS. Restart or constructing a new history gives an empty independent controller session; old completions belong to their original history. There is no global-current claim, automatic recovery, persisted artifact store, network access, approval issuance or ref write. Explicit preparation and execution retain the existing read-only Git and bounded Cargo processes; history checks may reread actual Git objects but do not rerun Cargo.
