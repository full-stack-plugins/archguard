# Java bytecode checks and repair verification

ArchGuard 0.6 adds an explicit `bytecode check|verify` CLI for JDK17/default Maven reactor package dependency rules. Existing `archguard_check`, project-model MCP operations and Cargo checks keep their original coverage. No bytecode MCP operation is exposed in this release.

1. Read the single project architecture/model baseline and the current requirement delta first. Keep their native requirement identity; bytecode findings supplement this workflow and do not approve or publish its baseline.
2. Require explicit existing, pinned Java/Maven/ArchUnit/helper artifacts, a private cache/home/temp configuration and the full declared reactor/type scope. Missing setup is incomplete; do not install tools or synthesize a permissive configuration. The local policy is a closed schema containing `ruleId`, exact package `from` and `forbiddenTo` prefixes. A local JSON file cannot grant approval.
3. Invoke the CLI with the reviewed configuration. Every path below is an absolute path supplied for this project, and the operation ID must be new:

```text
archguard bytecode check --project PROJECT --scratch PRIVATE_PARENT --store EVIDENCE_STORE --config CONFIG --policy POLICY --subject worktree --base BASE_OID --operation NEW_OPERATION --report NEW_RECEIPT
```

Compilation and ArchUnit evaluation happen in one private materialized candidate. Do not compile the original checkout or delete its `target` directory. Existing outputs, custom build profiles, processors, source roots and unsupported Java scopes remain explicit capability gaps. Full support details are in the repository's `docs/bytecode.md` and `java-adapter/README.md`.

4. Read technical status, completeness, diagnostics and each rule's actual source/edge. A partial result can still contain a confirmed violation. Fix the authorized source; do not weaken the rule or change the baseline to hide a failure.
5. Recompile and verify the current candidate, using the previous receipt, the same configuration/policy/selection kind/base and a new operation ID:

```text
archguard bytecode verify --project PROJECT --scratch PRIVATE_PARENT --store EVIDENCE_STORE --config CONFIG --policy POLICY --subject worktree --base BASE_OID --operation NEXT_OPERATION --previous OLD_RECEIPT --report NEW_RECEIPT
```

An old receipt must still match its private stored original. Repairs may change source bytes; changing the selection base, rule set or configuration is a new scope and is rejected by repair verification. Never reuse old compiled classes or another run's logs.

The receipt stays `local_unverified`, with no approval. Exit 0 is a complete local technical observation; exit 2 covers both violations and incomplete observations, distinguished by the receipt's `status` and diagnostics; exit 4 is an invalid invocation. Registered integration requires an independently compiled contract and frozen plan and returns an `UNVERIFIED` public GuardResult with private evidence references. Neither route creates a GateDecision or execution grant. Domain invariants, method/runtime calls, reflection, automatic model synchronization and baseline publication are separate obligations.
