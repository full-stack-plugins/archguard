# Change: extend-architecture-analysis-and-evidence

## Why

ArchGuard 0.1.0 已有 Cargo 成员直接 path 边原型，但尚无规则适用性检查、显式覆盖配置、独立发布、深层语言分析或受信候选证据。本增量将现有架构 A1–A5 路线转为可实施的新增要求，避免把 manifest-scope ALLOW 误解为源码/业务架构全覆盖。

源码审阅基线 `566fda92c7186a38e53ac81347ba23e0a8107443`：[src/lib.rs](../../../src/lib.rs)、[src/main.rs](../../../src/main.rs)、[tests/integration.rs](../../../tests/integration.rs)。现有 5 项测试覆盖 allowed、forbidden、missing manifest 与两项真实 CLI 退出；不证明新的覆盖、审批或符号功能已实现。当前 metadata 失败成为 partial/BLOCK，根目录无法 canonicalize 则错误 4；`--report` 仅写文件。新任务全部未完成，本提案不改变这些事实。

设计依据：[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[集成草案](../../../docs/integration-contract.md)、[跨仓路线图](../../guard-roadmap.md)。

## What Changes

新增三个独立 capability，不替换旧 `cargo-architecture` 规格：

- `architecture-analysis-profile`：在兼容原 CLI 的前提下冻结必查范围、验证规则适用性、保存依赖声明区别、声明输入摘要和执行预算。
- `architecture-domain-evolution`：逐语言扩展 System/Domain/Object/Method 模型、批准的架构基线、Design Diff 与过期索引处理；确定性约束与人工设计评审分开。
- `architecture-evidence-integration`：通过独立 versioned envelope 绑定候选、证据、覆盖和外部审批；处理并行任务、merge queue、审计、只读接口与发布回滚。

`guard.partme.ai/v1alpha1` 严格 schema 和 exact `forbid_relation` 保持不变。`guard.integration/v1alpha1` 仍是待冻结的独立集成版本，不能给当前 GuardFacts/GuardReport 增添字段。包 semver、分析器版本、协议版本与 policy revision 分别管理。

## Ownership and Dependencies

旧 [add-cargo-workspace-guard/tasks.md](../add-cargo-workspace-guard/tasks.md) 三项未完成 TODO 保留，不改勾选、不另起重复实现：

| 旧 umbrella TODO | 新细化任务归属 | 完成条件 |
|---|---|---|
| Publish guardengine and remove temporary path dependency | 5.5；GuardEngine 持有发布动作 | GE-RELEASE 通过后消费固定版本并验证回退；本项目不自行发布引擎 |
| Add Java, symbol-level Rust and TypeScript analyzer adapters | 2.1–2.5、3.1–3.5 | 各语言正/负/缺覆盖矩阵和领域映射实际通过 |
| Add protected CI policy source and signed build attestations | 4.1–4.5、5.1–5.4 | 外部控制器认证、最终候选与证据验证仅完成 protected CI 部分；signed-attestation 部分仍须独立签名方案和真实签发/验签 profile 证据，未落地前旧 umbrella 保持未完成；ArchGuard 不保管签名凭据 |

1. 本地 Cargo 加固和语言 fixture 可并行开发，不等待 SpecGuard。
2. 可互操作 evidence 等待 [GuardEngine change](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts) 的 GE-CONTRACT、GE-ADAPTER；受信使用再等待 GE-TRUST；独立生产发布等待 GE-RELEASE。可先用固定源码 revision 开发。
3. Requirement/ADR trace 和批准规格基线只在对应阶段等待 [SpecGuard change](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis) 的 SG-BASELINE，不形成全仓串行屏障。
4. AG-EVIDENCE 输出供 [GitGuard](https://github.com/full-stack-plugins/gitguard/tree/docs/guard-design-20261009/openspec/changes/add-candidate-bound-git-governance) 与 [FlowGuard](https://github.com/full-stack-plugins/flowguard/tree/docs/guard-design-20261009/openspec/changes/add-evidence-bound-workflow-gates) 消费；行为义务与 [TestGuard](https://github.com/full-stack-plugins/testguard/tree/docs/guard-design-20261009/openspec/changes/add-test-obligation-evidence-pipeline) 对接。最终联合场景在相关 gates 后执行，不要求 GitGuard 写操作先于本地分析。

## Impact and Non-goals

未来改动面为现有 `src/lib.rs`/`src/main.rs` 的窄集成点、新的 `src/analysis/`、`src/domain/`、`src/integration/`、fixture 与测试；这些模块目前不存在。没有功能代码随此提案提交。不实现 Git ref 写入、审批签发、通用测试执行或领域解析下沉引擎；外部 CodeGraph/codegraph-plugin、codereview-plugin 仅为未核验兼容目标。OPA、具体 Java/Rust 索引库、MCP 库、签名/审批提供商均需后续选择，不能视为已安装。

## Validation and Rollout

先运行原五项回归与 CLI/协议 golden，再逐阶段增加合法、违规、覆盖不足和资源失败矩阵；最终验证双 requirement、队列候选漂移、审批撤销/过期、迟到结果和回滚。按 advisory capture → shadow comparison → opt-in protected check → 显式 enforce 发布。任何生产适配器关闭均保留旧 CLI 和不可变审计；禁用适配器不能让本来要求 evidence 的门禁自动通过。本文仅规划，未来验收运行记录不能由文档/任务勾选替代。
