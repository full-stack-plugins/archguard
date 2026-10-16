## ADDED Requirements

### Requirement: Maintain one versioned project model
ArchGuard SHALL validate stable identities, ownership and relation endpoints in the project model and preserve immutable baseline history. Initialization SHALL NOT overwrite an existing baseline.

#### Scenario: Dangling domain relation
- **WHEN** a model references a removed or unknown element
- **THEN** validation fails and baseline state remains unchanged

### Requirement: Bind each requirement delta to its baseline and native source
ArchGuard SHALL reject stale baseline references, conflicting operations and missing native requirement references without creating a second specification source.

#### Scenario: Concurrent requirements
- **WHEN** one requirement updates the baseline and another still references the old digest
- **THEN** the second requirement must be reviewed against the current baseline before update

### Requirement: Analyze impact without reducing global checks
ArchGuard SHALL compute conservative impact paths over old and candidate relations and ownership, including removals. It SHALL execute full Maven/Cargo contract checks and compare the candidate module/dependency model to actual declarations.

#### Scenario: Dependency violation outside edited module
- **WHEN** the changed scope is narrow but the project contains a forbidden path or cycle
- **THEN** the candidate cannot pass merely because that violation is outside the direct changes

### Requirement: Recompute evidence before baseline updates
ArchGuard SHALL recompute current checks during verify and update. Only matching ALLOW evidence may update the pointer with explicit --apply; preview SHALL write nothing.

#### Scenario: Repair then update
- **WHEN** an implementation mismatch is repaired, a fresh report exported, and that report verified
- **THEN** explicit baseline update preserves the previous snapshot and switches the current pointer to the checked candidate

#### Scenario: Input drift or forged success
- **WHEN** sources, delta, model, contract, native specification or report have changed
- **THEN** old verification evidence is rejected without updating the pointer

### Requirement: Preserve existing interfaces and truthful plugin guidance
Existing contract-only projects SHALL continue working. New read-only MCP operations SHALL share CLI behavior; plugins SHALL distinguish declared domain structure, build-dependency conformance and unverified semantic implementation or approval.

#### Scenario: Current baseline check
- **WHEN** a project has an active baseline and the implementation drifts
- **THEN** ordinary check blocks and existing verify cannot reuse a report from an older baseline
