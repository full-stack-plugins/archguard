# Local trusted Cargo consumption and rollout

This is the original AG4.5/5.1/5.4/5.5 local Cargo slice. It adds no controller,
signing system, state owner, MCP, language provider or Git write capability.
The independent core still depends only on GuardEngine; verified Git candidates
and SpecGuard trace remain in the optional `archguard-git-spec` package.

## Supported profile and protected inputs

`integration::trust::CAPABILITIES` advertises exactly `cargo.declarations` and
`cargo.depends_on`. `cargo-declarations-v1` retains actual dependency declarations
(kind, target, optional, source and member resolution) and required coverage.
It publishes the existing strict contract/facts/report/domain artifact set using
canonical `artifact://archguard/<kind>/<sha256>` references. Missing artifacts,
unsupported requested capabilities, unknown fields, URI substitutions and missing
coverage cannot become eligible. This is not a claim for Java/TS/Rust symbol,
method/purity, domain transition, or all-language evidence consumption.

The protected caller freezes `ConsumptionPolicy` before dispatch: GE binding,
producer, required scopes, contract digest, action/principals, required capabilities
and required approval purpose/reference map. Never derive protected expectations
from candidate evidence as a deployment policy. The tests do so only to construct
fixture expectations for subsequently mutated observations. Core `CandidateContext`
is caller-supplied; use `GitEvidenceBundle::verify` in the optional integration
before consuming its Cargo bundle for authenticated object identity.

`consume` performs existing bundle integrity/URI checks and GE eligibility with
actual artifact bytes. Every call queries the external `AuthorityProvider` again.
`required_approvals` makes externally authenticated architecture baseline and
exception purposes mandatory even when the technical report is ALLOW; records
must match the exact binding (including baseline digest when present), contract,
action, purpose, permitted principal and time window. A Cargo run with null
baseline digest does not establish an approved domain baseline. Original
REQUIRE_APPROVAL, partial/BLOCK and execution status remain unchanged. A fixture
provider or integrity verification is never production authentication.

## Stage routing and rollback

`integration::rollout::check_stage` is stateless. Its protected caller supplies
`Selection { stage, adapter_enabled, native_facts }` on each consumption.
The comparison is explicitly **actual legacy analyzer facts/completeness parity**,
not observed CLI/report parity or equal snapshot identity: the legacy manifest
identity differs from the enhanced inventory. The same-input native observation
must be supplied by the caller; this library does not authenticate that input.

| Selection | Required architecture check |
| --- | --- |
| Advisory | Unsatisfied; evidence remains observable |
| Shadow | Unsatisfied; actual native fact comparison is observable |
| OptInProtected | Satisfied only with enabled adapter, native fact parity and fresh eligibility |
| Enforce | Same fresh checks; caller explicitly selects enforcement |
| Disabled adapter or rollback to advisory/shadow | Unsatisfied regardless of historical success |

There is no automatic phase promotion or external enforcement. A missing native
observation or mismatch refuses protected satisfaction. Rollback retains original
artifact/envelope bytes and GE attempt history. The caller must honor
`required_check_satisfied`, never the historical eligibility field alone. The
existing GE `AttemptStore` continues owning attempt generations and currentness;
late old publication is rejected after advancing the protected target.

## Audit and CI boundary

GE's redacted `AuditRecord` is returned without a new audit store. It carries
binding/envelope digests, the complete consumption policy digest (including
capabilities and mandatory approvals), actor/approver hashes, timestamp and hashed
invalidation cause. Stage output records selection, disabled state and required
check result separately from eligibility. The caller must retain these records
under its existing protected append-only storage/retention policy. No durable
retention provider, deletion policy or controller attestation issuer was selected.

Core CI now copies its already pinned vendored GE source instead of fetching
GG/SG into the core graph. Its checkout does not persist credentials, permissions
are read-only, and no signing/merge secret is supplied. The existing bounded
analysis runner clears its environment. These repository configuration changes
are not evidence of protected hosted CI, process isolation qualification or real
credential separation on a deployment. Those remain unverified inputs for AG5.2.

## Reproduce the local slice

Use Rust 1.90, existing Cargo cache and the source pins documented in
[independent-distribution.md](independent-distribution.md). No remote destination
or credentials are used:

```sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_DEBUG=0 \
  cargo test --locked --offline --test trust --test end_to_end
ARCHGUARD_ROLLOUT_CAPTURE=/absolute/output.json \
  cargo test --locked --offline --test end_to_end \
  actual_cargo_four_stage_rollout_preserves_required_check_and_history -- --exact
python3 integrations/git-spec/verify-source-pins.py
cargo test --locked --offline --manifest-path integrations/git-spec/Cargo.toml \
  --test candidate_binding \
  exact_optional_candidate_flows_through_core_trust_without_importing_git_into_core -- --exact
```

The capture records actual Cargo artifacts/binding, all four selections, rollback,
disablement, revocation and retained GE history with explicit fixture authority.
The optional test verifies the real candidate before core consumption and rejects
changed candidate/requirement expectations. Existing legacy CLI tests establish
0/2/3/4 and report/stdout compatibility separately. No old joint matrix is replayed.
Whole-task acceptance is reviewed against the original tasks; this document does
not mark absent all-domain, host, joint-consumer or production distribution
requirements complete. Packaging remains the existing fixed-source distribution,
not a new public release or signing profile.
