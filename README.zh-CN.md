# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard 是 Guard 生态的专业**架构守卫**。当前 V0.1 原型从真实 Rust Cargo 工作区提取包依赖事实，再交给独立的 GuardEngine 执行约束规则。

当前本地实现（crate `0.1.0`）包括有界 Cargo 声明分析、严格 GE evidence、基于输入 LanguageObservation 的确定性系统规则，以及可选 GitGuard 实际候选源码桥接。桥接从不可变候选 blob 构造私有分析副本，不认证 producer、控制器或队列。原生 CLI/schema 保持兼容。详见[进度](docs/implementation-progress.md)、[系统规则](docs/decisions/0003-local-system-rules.md)和[Git 源码边界](docs/decisions/0004-git-candidate-source.md)。真实语言 provider、受信基线/身份服务、合入授权和独立发行尚未完成。

## 工程准备

要求 Rust 1.90+ 和 Cargo；将 `guardengine`、`gitguard`、`archguard` 独立仓库放在同级目录。实际 GitGuard 依赖要求 Rust 1.90。目前是本地 path 依赖，固定公共包发行是单独任务。

## 可运行示例

在 `archguard` 目录执行：

```sh
cargo run -- check --project fixtures/allowed --contract examples/agent-job-contract.yaml
# 合法依赖，ALLOW，退出码 0

cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml
# 违规依赖 agent-job -> agent-saas，BLOCK，退出码 2

cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml --facts facts.json --report evidence.json
# 违规情况下也会输出事实和证据报告。
```

## 守卫检测范围

当前分析器调用 `cargo metadata --no-deps --offline --format-version 1`，检查**同一 Cargo 工作区内本地成员包的直接依赖**。包含声明的普通、可选、开发和构建依赖。

它暂时**不支持** Rust 方法调用图、外部依赖运行期调用、Java 模块规则、DDD 聚合归属或 AI 主观设计质量评估。

如果 Cargo 元数据/清单分析失败，事实状态为 `partial`，规则状态为 `INDETERMINATE`，最终 `BLOCK`；项目根目录无法规范化则直接输入错误（退出 4）。

依赖以实际包名与规范化目录匹配，重命名依赖不产生别名节点；普通/dev/build/optional/target 条件声明在事实中不作区分。因此不是指定 feature/target 下实际启用的构建图。不含 registry/git 外部包或非成员 path 边，不计算传递依赖与环。

`complete` 仅表示该有限范围提取成功。合同中的不存在包或不支持关系可能因没有精确匹配而 PASS，当前没有规则适用性验证。摘要覆盖根与成员 `Cargo.toml` 的原始字节和路径标签，不含 `Cargo.lock`、`.cargo/config*`、源码、工具链与 Git 身份；不能绑定 Cargo 元数据的全部影响因素或冒充提交证明。

ArchGuard 只提取事实，GuardEngine 负责契约与判定。可信合并门禁仍需在受保护 CI 上独立运行。

本地报告无签名；重算只验证报告与给定输入的一致性，不证明源码真实、审批有效或授予合入权限。

## CLI 与结果契约

仅实现 `check`，必需 `--project`、`--contract`；可选 `--facts`、`--report` 输出路径。默认在 stdout 输出格式化 JSON 报告；**传入 `--report` 后仅写文件，不再向 stdout 打印报告**。`--facts` 在求值前写入独立 JSON。父目录必须已存在，写入覆盖旧文件且不具原子性；不要覆盖输入，也不要在失败后使用旧结果。输入/运行时错误在 stderr 输出 `archguard:` 诊断。

| 退出码 | 含义 |
|---|---|
| 0 | 范围内 `ALLOW`，可能仍有建议级匹配 |
| 2 | `BLOCK`：强制规则违规或分析不完整 |
| 3 | `REQUIRE_APPROVAL`：存在 review 匹配且无阻断项 |
| 4 | 参数/合同错误、根目录不可访问、输出失败等；不保证有新报告 |

参数解析采用首个匹配的 flag/value，不提供常规 `--help`，也不会拒绝所有未知/重复参数。请使用上面的准确命令。`guard.partme.ai/v1alpha1` 是为兼容保留的协议名称；合同/事实/报告拒绝未知字段，仅支持精确 `forbid_relation`。`enforce` 匹配阻断，`review` 匹配要求审批，`advise` 匹配仍为 `PASS` 并保留证据；当前没有审批命令。

规则 DSL 与仓库示例一致：

```yaml
apiVersion: guard.partme.ai/v1alpha1
kind: GuardContract
metadata: { id: agent-job-boundary, revision: "1" }
spec:
  rules:
    - id: AGJ-ARCH-001
      enforcement: enforce
      assertion:
        type: forbid_relation
        subject: agent-job
        predicate: depends_on
        object: agent-saas
```

`CargoWorkspaceAnalyzer` 实现 `guardengine::GuardAnalyzer` 并返回 `GuardFacts`；引擎没有 Cargo 专属知识。

详见 [技术架构](docs/architecture.md)、[OpenSpec](openspec/changes/add-cargo-workspace-guard/) 和 [Guard Protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)。

## 四层架构治理与实施规划

未来 ArchGuard 以四层守卫为完整产品范围：**System Guard（系统/模块边界）**、**Domain Guard（领域/聚合不变量）**、**Object Guard（类型/对象职责）**、**Method Guard（方法调用、签名和副作用）**。后续计划扩展 Java ArchUnit、TypeScript 语义与 Rust 符号索引、Design Diff 和可信 CI。

**当前原型只覆盖 Cargo workspace 的直接本地成员依赖**。对象/方法合理性不能通过 AI 的“高置信度”得出确定性 PASS，必须以批准的架构契约、真实语言事实和必要的人工评审为依据。

完整方案见 [详细架构与 ADR](docs/architecture.md) 和 [技术方案、阶段实施与测试](docs/technical-design.md)。

## 集成边界

SpecGuard、ArchGuard、CodeGuard、TestGuard、GitGuard、FlowGuard 是六个独立领域守卫，共用 GuardEngine，不存在 GuardCore。引擎负责通用合同校验、中立规则求值与确定性证据，ArchGuard 负责架构事实与语义。跨守卫编排、MCP、审批和 Design Diff 仍是目标。可选本地 Git 桥接已绑定实际候选/base/group/member 对象，不提供队列授权。[集成契约](docs/integration-contract.md) 已作为独立严格 envelope 实现，原生 GuardFacts/GuardReport schema 不变。CodeGraph/codegraph-plugin、codereview-plugin 是尚未检查的外部兼容目标，不能视为已安装或验证的组件。

## 验证

运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`。
许可证：Apache-2.0。


## OpenSpec 实施待办

新增增量 [proposal](openspec/changes/extend-architecture-analysis-and-evidence/proposal.md)、[design](openspec/changes/extend-architecture-analysis-and-evidence/design.md)、[规范](openspec/changes/extend-architecture-analysis-and-evidence/specs/) 与 [tasks](openspec/changes/extend-architecture-analysis-and-evidence/tasks.md)，将架构方案拆成待实施工作。参阅[跨仓依赖路线图](openspec/guard-roadmap.md)与[结构验证记录](openspec/validation-2026-10-09.md)。任务勾选仅登记独立验收结果；当前实现和验收进度见 [implementation-progress](docs/implementation-progress.md)。前文源码树清单和验证限制对应检查基线或较早的架构审阅阶段；本次另行新增 OpenSpec 文档并记录实际 CLI 校验。既有 change 的任务归属和历史完成证据继续保留。

库接口现已增加固定 OpenJDK 21 的静态字节码适配器，已通过受限静态 profile 独立审查。它保留必需类型范围、重载描述符与真实 native capture 来源；反射和动态行为仍标记为未知。详见[能力矩阵](docs/analyzer-capabilities.md)与[JDK 决策](docs/decisions/0007-fixed-jdk-classfile-provider.md)。原有 CLI 仍面向 Cargo。

固定 TypeScript5.9.3 compiler API 适配器已实现，待独立审查。它支持受保护 paths 别名、跨包声明来源和重载签名；project references 明确拒绝，动态导入与缺失来源保持缺覆盖，不宣称运行时调用完整性。
