---
name: architecture-check
description: Maintain project architecture and domain model baselines, analyze requirement deltas, check Maven/Cargo dependencies, repair violations and verify evidence before explicit baseline updates.
license: Apache-2.0
compatibility: ArchGuard 0.5.0 CLI or MCP; project models, Java syntax policies and Maven/Cargo graph.
---

# ArchGuard

Use this skill when changing module dependencies or validating architecture boundaries.

For Maven projects with a model java policy, follow [Java source checks](references/java-source.md). Read java_analysis.findings for package/layer/domain type ownership evidence; preserve source inventory and policy binding through check/verify/update. Do not claim Java compiler or business semantic validation. Projects without that policy retain build-only coverage.

When domain.model is configured, use the single project baseline rather than creating a complete architecture per requirement. Follow [project model workflow](references/project-model-workflow.md): read model show, maintain a baseline-bound native requirement delta, run change impact/check, repair within authorization, recheck and verify, then preview a local baseline update. Only explicit authorized --apply writes the new baseline; MCP exposes no writer. A requirement report ALLOW proves only declared model/build conformance, not domain semantics or approval.

1. Resolve the actual project path and run `archguard context --project <absolute-path>`.
   Reuse `domain.spec_binding` when configured; otherwise use `archguard spec context` to discover native specifications. Select ambiguous systems/targets explicitly with `--spec-system` / `--spec-target`. Do not initialize or duplicate specifications. `ready` means files exist, not approval.
2. Read the project's `.archguard/contract.yaml`, or the user's explicit contract. If absent, report unconfigured and discuss the intended rules; do not infer compliance or create a permissive contract.
3. Run `archguard contract validate --project <absolute-path>`.
4. Use `archguard_check` with an explicit absolute `project`; optional `contract` must also be absolute. If MCP is unavailable, run `archguard check --project <absolute-path>`. For mixed Maven/Cargo roots, explicitly select `analyzer` / `--analyzer maven|cargo`.
5. For each violation use `archguard_explain` with `project` and `rule`, or CLI `explain --rule ID`. Read the source locations and dependency-chain witnesses. Propose or implement fixes within the user's authorized scope; do not automatically weaken contracts.
6. Re-run check after fixes. CLI can export to new files with `--report FILE --facts FILE`. Existing output files are protected. Then run `archguard_verify` with an absolute existing `report`, or CLI `verify --report FILE`, against current sources and contract.

MCP domain operations never export or modify project files. They call the same implementation as the CLI. Context and instructions only describe configuration; they do not evaluate it.

Interpret ALLOW (exit 0), BLOCK (2), REQUIRE_APPROVAL (3), and invalid invocation/stale report (4) separately. A blocked check is a successful tool invocation with a blocking decision. INDETERMINATE blocks, even for advisory rules. REVIEW is a request for human approval, never an approval grant.

v1alpha2 rules support forbid_relation, forbid_path, allowed_dependencies, require_relation, acyclic. Node IDs are Maven groupId:artifactId or Cargo package names. Unknown entities/predicates do not pass. v1alpha1 retains legacy exact-relation semantics; recommend explicit reviewed migration for strict coverage.

Build analysis covers declarations between reactor/workspace members across all declared scopes, including optional/test/build dependencies. Maven supports local parent models, properties, dependency management, modules/subprojects and explicit profile scenarios in .archguard/maven.json. Every model-affecting profile needs a true/false selection; unknown IDs block. It does not run Maven, resolve external parents/BOMs or infer automatic activation. The optional Java model policy separately inspects syntax imports/qualified references and declared type ownership; it does not provide complete compiler binding, method/runtime calls or domain invariants. Cargo uses offline locked metadata. Do not claim complete system/domain/object/method governance.

The package includes archguard-domain-modeling, archguard-grill-with-docs, archguard-codebase-design, archguard-improve-codebase-architecture, archguard-research and guard-grilling. Load the exact names from this package when needed; no same-named global skill substitution. Their shared protocol describes native rule binding in .archguard/spec-binding.json. Bound native artifact changes invalidate saved reports. Methods propose decisions; they never grant approval or produce deterministic ALLOW by prose.

Local evidence is unsigned, bound to the inspected files and contract. Trusted CI must rerun on a pinned checkout with protected contracts; hashes are not signatures.
