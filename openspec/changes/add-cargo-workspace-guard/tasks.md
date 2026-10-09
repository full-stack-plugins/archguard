# Tasks: add-cargo-workspace-guard

- [x] Implement \`CargoWorkspaceAnalyzer\` as \`guardengine::GuardAnalyzer\`.
- [x] Extract canonical local Cargo workspace package edges.
- [x] Produce normalized GuardFacts and manifest-level snapshot digest.
- [x] Implement CLI \`archguard check\`.
- [x] Add allowed/forbidden/missing-manifest fixtures.
- [x] Add 5 end-to-end tests including actual CLI process exits.
- [x] Document analyzer's explicitly bounded scope.
- [ ] Publish guardengine and remove temporary path dependency.
- [ ] Add Java, symbol-level Rust and TypeScript analyzer adapters.
- [ ] Add protected CI policy source and signed build attestations.
