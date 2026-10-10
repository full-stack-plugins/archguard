# Change: decouple-core-and-add-archunit-profile

状态：下文保留 2026-10-09 设计阶段范围；技能优先方向及本地交付现已审阅，当前证据见[本地验收记录](../../../docs/reviews/local-closure-20261010.md)。未推送、安装宿主或公开发布。保留现有 change 标识以维持引用。

## Why

ArchGuard 应帮助项目定义和验证架构约束。成熟工具的接入知识应放在 full-stack-skills 语言仓库，由 ArchGuard-Plugin 选择调用；当前 GitGuard/SpecGuard 必选路径依赖也需要与核心分离。

## What Changes

- ArchGuard-Plugin 选择并调用语言架构技能，不逐个建立专用执行器平台。
- 首批在 java-skills 增加 archunit 技能、Maven/Gradle 接入说明、架构规则与两套实际示例；同步注册和双语索引。
- 目标项目原生测试保存规则、版本和证据，沿用构建工具退出码及报告，区分通过/违规/未验证。
- 独立核心保留既有分析、契约与基线；Git 候选绑定及规格追溯移至可选集成。分发不得依赖外部 sibling 路径。

## Scope and Compatibility

java-skills 本轮只读，源码核验 commit 为 d981268c4baaa32e4e068a47b95db13e5a1c526e。Java 技能接入无需 ArchGuard 二进制，核心解耦单独验收。不改 CodeGuard，不新增运行时、数据库、控制器或扩张原路线图。旧分析与精确候选验证保留，其他语言按实际仓库逐批调查。

## Review Deliverable

[设计决策](design.md)；[完整短设计](../../../docs/superpowers/specs/2026-10-09-architecture-contract-and-checker-adapters-design.md)。未创建实施 tasks，未宣称功能测试或 OpenSpec 验证通过。

## Specification scope declaration — 2026-10-10

This change reorganizes delivery and tooling: it separates the existing core from optional integration and places native-tool guidance/examples in the language skill provider. Existing ArchGuard analysis, domain decisions, immutable baselines, candidate verification and wire contracts retain their requirements in `extend-architecture-analysis-and-evidence`. No new engine envelope, analyzer semantics, runtime controller or MCP contract is introduced by this change. Accordingly `.openspec.yaml` declares `skip_specs: true` for this refactor/tooling change rather than inventing duplicate normative deltas. Java examples have their explicit provider-local rule contracts; their tests and compatibility scope are recorded independently. A future change to ArchGuard normative behavior must introduce its own deltas and must not rely on this declaration.

The original design-stage “not implemented/not validated” statements above describe that historical phase, not the current delivery. The implementation plan and current acceptance record preserve actual evidence and remaining host/distribution limits. OpenSpec format validation does not qualify runtime behavior.
