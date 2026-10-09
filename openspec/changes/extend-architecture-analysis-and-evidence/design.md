# Design: ArchGuard 分阶段分析与证据演进

## Context and Baseline

源码基线 `566fda92c7186a38e53ac81347ba23e0a8107443`。当前 `CargoWorkspaceAnalyzer` canonicalize 根目录，运行离线 `cargo metadata --no-deps`，从真实成员抽取直接 path 边，按原始清单字节计算 manifest digest。facts 不区分 dev/build/optional/target，也不包含非成员/传递/源码关系；未知合同对象可能因无匹配而 PASS。根目录错误为 4，metadata/提取错误为 partial/BLOCK 2。CLI `--facts` 先于求值写文件，`--report` 抑制 stdout，无原子写入与超时。未来必须保留 legacy profile 的语义，显式添加能力而非默改归档证据。

依据：[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[集成草案](../../../docs/integration-contract.md)、[跨仓路线图](../../guard-roadmap.md)。旧 [change tasks](../add-cargo-workspace-guard/tasks.md) 不修改：发布依赖、语言适配、受信 CI 三个 umbrella 分别由新任务 5.5、2–3 组、4–5 组细化，未宣称旧 change 完成。

## Goals and Non-goals

提供可检查的 scope/能力、四层架构语义、可审计候选证据与明确失败状态。ArchGuard 拥有架构解释和领域规则；GuardEngine 只拥有一般 Rule/Contract/Evidence。六个独立 Guard 无 GuardCore。ArchGuard 不执行合并、不签发审批、不替代 TestGuard 行为验证，不凭模型置信度认定设计唯一正确。

## Proposed Modules and Interfaces

以下均为拟新增路径和内部接口，具体 Rust 类型随第一组 profile 决策冻结，不是已公开 API：

| 模块 | 输入 → 输出与责任 |
|---|---|
| `src/analysis/profile.rs` | ProtectedScope + requested capabilities → FrozenAnalysisProfile；固定必查成员/关系/配置与预算 |
| `src/analysis/cargo.rs` | FrozenAnalysisProfile + frozen project → ArchitectureObservation；直接声明边保留 kind/optional/target，legacy projection 仍为原四元组 |
| `src/analysis/snapshot.rs` | declared input inventory → ManifestSnapshot / AnalysisSnapshot；明确字节、配置、环境身份和遗漏 |
| `src/analysis/runner.rs` | tool invocation + budget → completed observation 或 typed failure；目录/时间/输出限额 |
| `src/analysis/{java,typescript,rust_symbols}.rs` | versioned language capability → 符号/依赖/覆盖缺口；选择依赖后逐语言发布 |
| `src/domain/{model,rules,baseline,diff}.rs` | Observation + approved ArchitectureBaseline → domain findings / obligations / diff；来源与能力检查由本域负责 |
| `src/integration/{projection,binding,evidence,cli}.rs` | frozen binding + typed result → schema-validated GuardRunEnvelope + immutable artifact refs；不拥有身份/审批签发 |

现有 `src/lib.rs` 保留 GuardAnalyzer 入口，`src/main.rs` 保留当前 check profile。新增深层模型/coverage 放在独立领域 artifact，不给当前严格 schema 加字段。领域关系确可映射 exact forbid_relation 时才投影；不可表达的要求须协商新能力/版本或明确 unsupported，不能伪造当前引擎规则。

## Analysis and State Model

必查范围来自受保护配置，先冻结再提取，候选删除模块/规则不能缩小义务。域 observation 保留 Module/Context/Aggregate/Type/Method、namespace/signature、SourceSpan、边及 coverage；符号 ID 必须区分重载、语言和索引版本。当前 Cargo 兼容输出继续合并重复四元组，增强 artifact 保留每项声明，避免把声明图称为 active feature graph。

集成运行先解析 repo/task/worktree/requirementIds/candidate/base/mergeGroup、producer 和 required coverage。此前无效参数或无法读保护配置只返回独立诊断，不补空 OID，不输出 envelope。绑定后 `queued → running → completed | error | cancelled`；完整或有效 partial facts 可完成引擎求值，partial 为 BLOCK；工具崩溃/超时/取消的增强执行路径为 error/cancelled、decision=null。legacy metadata 失败仍保持 partial/BLOCK，不在原命令中悄悄改退出码。结果 envelope 的 decision 必须等于所引用引擎报告；输出失败不允许复用旧文件成功。

ArchitectureBaseline 为不可变合同/ADR/source ref/digest 和外部审批引用，状态 draft → in_review → approved → superseded/revoked。本域定义“边界变弱”和例外范围，控制器认证权限/期限/撤销。Markdown accepted、布尔值、签名字段不能自行构成权威。审批不改写 REQUIRE_APPROVAL，也不覆盖 partial/执行错误。对 AppService.cancelTask 与 Aggregate.cancel 的职责差异先看批准契约与副作用，不以同名判重复。

## Domain Evolution and Design Diff

System 首先扩展清晰能力下的分层/环检测；Domain 关联状态转换、不变量与 TestObligation，但不承担测试执行；Object 验证可见性、依赖/继承与 API 契约；Method 验证签名、调用、事务/IO 与错误传播。每个 enforce 规则须有合法、违规、缺覆盖三类案例；启发式保持 review/advise。静态无法解析动态分派、宏/反射时保留 unknown，必查范围未覆盖不满足门禁。

Design Diff 比较批准基线和候选的图/API，输出新增禁止边、所有权改变、API 消费方影响及来源。不把未索引消费者当成零影响；索引需绑定源码/配置/工具身份，过期拒绝复用。CodeGraph/SCIP 是未来适配候选，先做格式/许可/覆盖调查，不假定现有外部插件可调用。

## Integration, Trust and Concurrency

[GuardEngine change](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts) 的 GE-CONTRACT/ADAPTER 固定 schema、golden 和通用适配后才发布 interoperable producer；本地分析先行。`guard.integration/v1alpha1` 与 `guard.partme.ai/v1alpha1` 独立，不支持隐含 N/N-1。trusted use 等待 GE-TRUST，独立固定发行等待 GE-RELEASE。Spec/ADR trace 仅等待 [SG-BASELINE](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis)，basic scan 不依赖它。

技术复用键包括不可变 repo/task/requirement scope、candidate/base/merge group、snapshot/baseline/contract、analyzer/profile/coverage。runId 为每次尝试唯一，重试不复用 runId。两 requirement 可以在相同提取配置下共享缓存，但运行、义务和审批独立。迟到完成只归档原 binding，compare-and-set 防覆盖新候选。candidate/base/queue membership、基线/规则集/分析器/coverage 变化或审批过期/撤销触发失效。真正合并门禁必须由控制器在 GitGuard 解析的 synthetic candidate 上重跑；head 报告不能替代。

输出给 GitGuard/FlowGuard 的 AG-EVIDENCE 是范围内技术证明，不授予合并。只读 CLI/MCP 使用同一分析服务，能力协商明确错误/profile；无自我批准或 ref-write endpoint。记录认证 producer、输入绑定、artifact digests、来源/覆盖、审批引用和失效因果；签名凭据/短期凭据只在外部控制器，分析进程无写 Git/审批权限。

## Security and Failure Handling

runner 在隔离快照读取，受保护合同在候选之外。白名单可读目录与证据目的地，限制符号链接逃逸、单文件/总输出、深度和时间；不执行文档指令或任意 policy script。Cargo 的配置/环境与可能更新锁文件的行为通过临时工作副本隔离，不以 offline 宣称只读沙箱。诊断脱敏，artifact URI 必须是许可存储而非可执行任意 URL。预算耗尽和工具不支持产生具体缺口/错误，不能删边后 ALLOW。

## Phase Gates, Acceptance and Rollback

组 1 完成现有 Cargo 加固；组 2 逐语言 profile；组 3 领域与基线/Design Diff；组 4 集成 transport/binding；组 5 信任、只读接口、联合验证与发行。任务可按明确 gate 并行，不把整仓依赖变成循环。每个发布 profile 列出真实矩阵；未支持语言保留 unsupported，不宣称一次发布已完成全部语言。

先保持 5 个旧测试与 JSON/退出码 golden，再完成新矩阵。联合验收包含同仓两 requirement、精确队列 candidate、迟到结果、覆盖缺失、审批过期/撤销、规则变弱和恢复旧 CLI。rollout 依次 advisory/shadow/opt-in protected/enforce；单独回退新适配器而不篡改历史报告或取消必需义务。任务全部未勾选，实际测试/CI 与审批验证记录才是通过凭据。

## Decisions Still Required

第一阶段明确采用声明图还是另立 active build profile，默认保留声明语义；输入 inventory 是否包括锁文件/config/toolchain 需按 capability 冻结并给变动用例；默认新 inventory 不改变旧 manifest digest。Java ArchUnit/字节码、TS compiler API、Rust 索引器及 SymbolId 方案须用最小 fixture/性能评估选择后锁版本。OPA 为可选研究，不是任意脚本执行承诺。存储/签名/审批提供商、保留期限与 MCP 库在 GE-TRUST/RELEASE 确认后选用；未选定时只提供 typed ports 和本地 fixture，不发布受信完成声明。
