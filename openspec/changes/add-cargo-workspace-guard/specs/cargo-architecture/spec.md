# Specification: Cargo Workspace Dependency Guard

## Requirements

### Real extraction
The analyzer MUST derive edges from successful Cargo metadata, not from hard-coded fixture names.

### Workspace scope
Each generated \`depends_on\` fact MUST relate two locally resolved workspace member package paths.

### Provenance
Each fact MUST provide the source manifest location; the fact set MUST bind inspected manifest content to a deterministic digest.

### Fail closed
If Cargo metadata fails, the fact set MUST be \`partial\` and yield global \`BLOCK\` on evaluation.

### CLI
The \`check\` subcommand MUST support project root, contract path, JSON report and optional fact export. It MUST produce blocking exit code 2 for violations.

### Consumer independence
All contract parsing, evaluation and report digest logic MUST be delegated to GuardEngine.
