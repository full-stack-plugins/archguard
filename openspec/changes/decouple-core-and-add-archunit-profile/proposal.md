# Change: decouple-core-and-add-archunit-profile

状态：设计待审阅，仅文档；不启动实现、安装、发布或任务勾选。

## Why

ArchGuard 应定义和检查项目语言、框架、MVC/DDD模式、模块、目录、命名及依赖规则。目前候选证据桥使GitGuard/SpecGuard成为无条件Cargo路径依赖；运行时不调用Git并不等于核心构建与发行独立。需保留已有分析能力，同时把候选与追溯隔离为可选集成。

## What Changes

- 核心可从无 `.git` 的源码目录运行，默认构建/分发不依赖GG/SG或外部sibling仓库；GE也必须采用固定可分发依赖或显式自包含构建包。
- 版本化架构契约映射到项目选择的成熟工具，规则不支持、缺工具、构建失败或覆盖缺失均为未验证，不能假通过。
- 首先接入Java ArchUnit，实际验证通过→可编译越层依赖失败→修复通过。TSArch与Rust独立后续切片，不称跨语言统一官方插件。
- 保留Cargo/Java/TS分析、架构baseline和精确候选字节验证。初始化、安装、项目修改及存量违规baseline均需显式触发/确认。

## Scope and Compatibility

不改CodeGuard，不扩张原193项任务或通用安全体系。旧 `extend-architecture-analysis-and-evidence` 及源码历史保留；本变更修正其默认打包边界与成熟工具接入方向，不追溯勾选未完成功能。普通CLI语义保持，可选集成调用者渐进迁移。

## Review Deliverable

[设计决策及验收摘要](design.md)；[完整短设计](../../../docs/superpowers/specs/2026-10-09-architecture-contract-and-checker-adapters-design.md)。本轮未创建实施tasks；书面设计确认后才进入实施计划。
