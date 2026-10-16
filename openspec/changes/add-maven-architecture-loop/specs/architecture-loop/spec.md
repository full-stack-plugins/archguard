## ADDED Requirements

### Requirement: Maven reactor extraction
The analyzer SHALL read bounded local POMs without running Maven, resolve supported local inheritance and properties, emit reactor dependency evidence and bind every inspected POM to a deterministic snapshot.
#### Scenario: Valid layered reactor
- **WHEN** a reactor declares application to domain dependencies and dependencyManagement contains an unrelated entry
- **THEN** only real declared dependencies become edges, with module identities and source locations
#### Scenario: Unsupported or unsafe model
- **WHEN** a module escapes the root, a required model is missing, a coordinate is unresolved, or a model-affecting profile/external parent cannot be resolved
- **THEN** completeness is partial with diagnostics and checks cannot ALLOW
#### Scenario: JVM configuration boundaries
- **WHEN** jvm.config contains only numeric Xmx/Xms/Xss memory limits
- **THEN** the analyzer includes it in the snapshot and continues; system properties and other unsupported flags remain partial

### Requirement: Coverage-aware graph constraints
The engine SHALL support versioned direct, transitive, allowlist, required-relation and acyclic constraints with reproducible edge evidence and preserve legacy exact-rule parsing.
#### Scenario: Indirect boundary or cycle violation
- **WHEN** a covered graph contains a forbidden reachable target or cycle
- **THEN** the result contains an actual witness edge chain and respects enforce/review/advise severity
#### Scenario: Misspelled or unsupported scope
- **WHEN** a rule references an absent entity, unsupported predicate, or v2 coverage is absent
- **THEN** it is INDETERMINATE and BLOCK rather than silently PASS

### Requirement: Repair and fresh verification loop
CLI and MCP SHALL use the same checking service, expose explanations, and verify evidence by re-analyzing the current project and contract.
#### Scenario: Repair without weakening policy
- **WHEN** a forbidden POM dependency is removed and check is rerun with the same approved contract
- **THEN** the current graph passes and fresh evidence verifies, while the old report is rejected
#### Scenario: Report tampering or input drift
- **WHEN** report fields, POM inputs, or contract fields differ from the verified inputs
- **THEN** verify rejects stale or altered evidence without accepting a self-reported digest
#### Scenario: Read-only MCP and plugin
- **WHEN** an agent invokes ArchGuard check/explain/verify through the declared plugin MCP entry
- **THEN** it receives the CLI-equivalent decision and no project outputs or rules are written

### Requirement: Safe public interface
Commands SHALL reject unknown or duplicate options, preserve existing inputs and artifacts, and retain exit codes 0/2/3/4.
#### Scenario: Output collision
- **WHEN** an output path already exists or resolves to project input
- **THEN** the command refuses without modifying the file
