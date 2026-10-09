# Specification: Cargo Workspace Dependency Guard

## ADDED Requirements

### Requirement: Real extraction
The analyzer MUST derive edges from successful Cargo metadata, not from hard-coded fixture names.

#### Scenario: Real workspace metadata
- **WHEN** a Cargo workspace is analyzed successfully
- **THEN** edges reflect its metadata declarations

### Requirement: Workspace scope
Each generated \`depends_on\` fact MUST relate two locally resolved workspace member package paths.

#### Scenario: Nonmember dependency
- **WHEN** a path dependency is not a workspace member
- **THEN** no member dependency fact is emitted

### Requirement: Provenance
Each fact MUST provide the source manifest location; the fact set MUST bind inspected manifest content to a deterministic digest.

#### Scenario: Manifest bytes change
- **WHEN** inspected manifest bytes change
- **THEN** the deterministic manifest digest changes and facts retain source locations

### Requirement: Fail closed
If Cargo metadata fails, the fact set MUST be \`partial\` and yield global \`BLOCK\` on evaluation.

#### Scenario: Metadata fails
- **WHEN** Cargo metadata exits unsuccessfully
- **THEN** partial facts yield BLOCK

### Requirement: CLI
The \`check\` subcommand MUST support project root, contract path, JSON report and optional fact export. It MUST produce blocking exit code 2 for violations.

#### Scenario: Forbidden workspace
- **WHEN** check evaluates a forbidden member dependency
- **THEN** the process exits with code 2 and exports the requested JSON

### Requirement: Consumer independence
All contract parsing, evaluation and report digest logic MUST be delegated to GuardEngine.

#### Scenario: Shared engine evaluation
- **WHEN** a contract and extracted facts are evaluated
- **THEN** GuardEngine performs parsing, evaluation and report digest computation
