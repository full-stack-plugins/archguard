## ADDED Requirements

### Requirement: Extract bounded Java source dependencies
ArchGuard SHALL parse Java syntax in Maven reactor source roots, report package references with source locations and explicit coverage, and reject unsupported or incomplete analysis instead of reporting compliance.

#### Scenario: Import and qualified reference
- **WHEN** source imports another package or explicitly references a known project type
- **THEN** the package edge is reported, including static imports, with the actual source location

#### Scenario: Comments and broken sources
- **WHEN** comments or literals mention imports
- **THEN** they create no edges, while malformed Java blocks analysis

### Requirement: Enforce source package layers and domain ownership
The project model SHALL contain optional reviewed Java policy and explicit domain type mappings. Package ownership, forbidden references and allowed layer dependencies SHALL be enforced against actual declarations.

#### Scenario: Layer violation and repair
- **WHEN** a domain source depends on a prohibited infrastructure package
- **THEN** the check blocks with source evidence, and removing the violation permits a fresh check

#### Scenario: Missing or moved domain type
- **WHEN** a mapped type is missing, duplicated or belongs to another module
- **THEN** the check blocks without publishing a new baseline

### Requirement: Bind source checks to the baseline lifecycle
Candidate and ordinary checks SHALL bind Java sources and policy. Requirement increments SHALL update mappings and policy through the existing native specification reference. Verify and update SHALL reject stale evidence.

#### Scenario: Java source drift
- **WHEN** Java content or the file inventory changes after report export
- **THEN** verify and baseline update reject the old report even if POM files are unchanged

### Requirement: Preserve compatibility and truthful plugins
Models without Java policy and Cargo checks SHALL retain existing behavior. CLI and read-only MCP SHALL share implementation, and plugin guidance SHALL state unsupported semantic coverage.

#### Scenario: Cargo project
- **WHEN** a Cargo model without Java policy is checked or updated
- **THEN** its existing lifecycle continues to work
