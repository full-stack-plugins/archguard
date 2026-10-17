# ArchGuard plugin 0.6.0

Portable Agent Plugins 1.0.0 package: root plugin.json, mcp.json, and seven discoverable skills (architecture-check, five architecture methods and guard-grilling).

Prerequisite: an ArchGuard 0.6.0 CLI built from this repository and available as `archguard` on the host's PATH. This package does not bundle or install an executable. `archguard install --bin-dir <directory> --project <project> --agents codex,claude,gemini` can copy the executable and generate native host configurations; adding the directory to PATH is an explicit environment step. Preview using --dry-run.

Load this directory only in a host supporting the portable specification, or use the CLI's native host integration. Avoid registering the same MCP server twice. Package validation and subprocess tests do not prove loading in every host.

The plugin calls the CLI's shared check/explain/verify implementation. It has no duplicate rule evaluator and grants no approval. See ../docs/architecture-loop.md for contracts, limits and exit codes. No global installation or marketplace publication is performed by this source package.

The method bundle is generated from GuardEngine skill-library with pinned source hashes, dependency closure and MIT notices. See [method integration](../docs/method-integration.md). Native host configuration by install does not itself install these method skill files; load the plugin package through the host's supported mechanism.

Project model workflow: model show/validate, change impact/check/verify, and explicit CLI-only baseline init/update. MCP has ten read-only tools including the two context tools; no model publication tool is exposed. See [project model guide](../docs/project-model.md) and the package-local [workflow](skills/architecture-check/references/project-model-workflow.md).

Configured Java policies add source package/layer/domain ownership checks to the same model workflow. See [Java source guide](../docs/java-source.md) and [package-local instructions](skills/architecture-check/references/java-source.md).

Bytecode verification is an explicit CLI-only operation in this release: see [the private-candidate workflow](skills/architecture-check/references/bytecode.md). The existing read-only MCP tools do not execute the compiler.
