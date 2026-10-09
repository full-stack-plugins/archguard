# Design

\`CargoWorkspaceAnalyzer\` implements \`GuardAnalyzer\` from GuardEngine.

1. Invoke \`cargo metadata --no-deps --offline\` for the target workspace.
2. Identify actual workspace members and map canonical package paths to names.
3. Extract direct path dependencies between those members, normalize into \`(subject, depends_on, object, manifest-source)\`.
4. Hash the exact workspace and member manifests inspected, in sorted relative-path order.
5. Mark incomplete observations \`partial\` with diagnostics.
6. Run the independent \`guardengine::evaluate\` with the approved contract.
7. Print report and return exit codes 0/2/3/4.

No raw GitHub API credential or model-generated JSON facts are required. Local evidence is unsigned.
