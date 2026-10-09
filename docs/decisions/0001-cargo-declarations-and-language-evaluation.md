# Decision 0001 — experimental Cargo declaration profile

Status: implemented locally, pending independent review. This is a reversible domain decision, not external approval.

Keep legacy metadata projection and manifest digest unchanged. Add an explicit declaration-only profile that freezes the caller's contract scope before extraction, records richer dependency provenance, and hashes an isolated conservative source inventory. Reject unsupported configuration rather than claiming an active build graph. Registry/git/nonmember dependencies never become member edges.

Use fixed `toml = 0.8.23` to inspect copied manifest paths and `libc = 0.2.177` for Unix process-group termination/nonblocking pipes. Existing serde_json/sha2 and engine protocols remain unchanged. The platform's blocked bubblewrap namespace setup means OS containment is not accepted; the runner is suitable only for trusted tools over immutable input supplied by a controller. Do not describe local digest recomputation as authentication or production release.

Evaluate TypeScript 5.9.3 as a next-language candidate using a real compiler fixture. Its successful alias/overload/unknown-import experiment does not freeze the domain SymbolId protocol or enable a language capability. Java and Rust symbol providers remain unsupported until the missing tool/config/license/coverage decisions and matrices are implemented. No OPA, MCP, approval provider, signature issuer, registry release or external side effect is introduced.
