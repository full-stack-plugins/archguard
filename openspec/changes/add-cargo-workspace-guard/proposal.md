# Change: add-cargo-workspace-guard

## Why

A first real analyzer is necessary to validate Guard Protocol's generality and prove deterministic constraints can block AI-authored architecture violations.

## Goal

Analyze actual Cargo workspace member dependencies; normalize facts; feed GuardEngine; produce evidence and CI-friendly exit codes.

## Not in scope

Method symbols, source-level call graphs, Java/TypeScript, transitive dynamic dependencies, designing optimal domain boundaries, signing or Git permissions.

## Acceptance

Allowed Cargo dependency yields ALLOW and exit 0. Prohibited \`agent-job -> agent-saas\` yields BLOCK and exit 2 with a source manifest reference. Missing manifest yields INDETERMINATE and BLOCK. Analyzer never evaluates the rule itself.
