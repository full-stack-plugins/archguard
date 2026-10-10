# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard 是 Guard 生态的专业**架构守卫**。当前 V0.1 原型从真实 Rust Cargo 工作区提取包依赖事实，再交给独立的 GuardEngine 执行约束规则。

当前本地实现（crate `0.1.0`）包括有界 Cargo 声明、确定性系统规则、已独立验收的 JDK 21 静态字节码与 TypeScript 5.9.3 语义 provider、不可变批准基线、批准的领域静态访问规则及 `NotRun` 测试义务、Java 观测差异 Design Diff，以及严格 Cargo evidence/trust ports。原生 CLI 仍仅支持 Cargo。批准 authority 来自外部；本地校验不安装生产身份服务，也不授予合并权。详见[当前验收与剩余缺口](docs/reviews/local-closure-20261010.md)、[实施历史](docs/implementation-progress.md)和[能力矩阵](docs/analyzer-capabilities.md)。

## 工程准备

要求 Rust 1.90+ 和 Cargo。核心开发只依赖同级 `../guardengine`；不依赖 GitGuard 或 SpecGuard。[独立源码打包流程](docs/independent-distribution.md)将固定 GuardEngine 源码放入带锁文件的 archive，可在无同级仓库的目录构建。Registry/工具链缓存和文档列出的 Java/TypeScript 测试工具仍需准备；这不是已公开的生产发行包。

可选 Git/Spec 集成使用独立的 `integrations/git-spec/Cargo.toml` 及文档中的固定 GG/SG/GE 源码。实际 Git 候选桥接和 trace reader 不提供生产 issuer 或队列控制器。

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

## 默认 legacy CLI 检测范围

默认 legacy 分析器调用 `cargo metadata --no-deps --offline --format-version 1`，检查**同一 Cargo 工作区内本地成员包的直接依赖**。包含声明的普通、可选、开发和构建依赖。

它暂时**不支持** Rust 方法调用图、外部依赖运行期调用、Java 模块规则、DDD 聚合归属或 AI 主观设计质量评估。

如果 Cargo 元数据/清单分析失败，事实状态为 `partial`，规则状态为 `INDETERMINATE`，最终 `BLOCK`；项目根目录无法规范化则直接输入错误（退出 4）。

依赖以实际包名与规范化目录匹配，重命名依赖不产生别名节点；普通/dev/build/optional/target 条件声明在事实中不作区分。因此不是指定 feature/target 下实际启用的构建图。不含 registry/git 外部包或非成员 path 边，不计算传递依赖与环。

`complete` 仅表示该有限范围提取成功。合同中的不存在包或不支持关系可能因没有精确匹配而 PASS，当前没有规则适用性验证。摘要覆盖根与成员 `Cargo.toml` 的原始字节和路径标签，不含 `Cargo.lock`、`.cargo/config*`、源码、工具链与 Git 身份；不能绑定 Cargo 元数据的全部影响因素或冒充提交证明。

ArchGuard 只提取事实，GuardEngine 负责契约与判定。可信合并门禁仍需在受保护 CI 上独立运行。

本地报告无签名；重算只验证报告与给定输入的一致性，不证明源码真实、审批有效或授予合入权限。

显式 `--profile cargo-declarations-v1` 选择有界声明 profile，包含受保护的规则适用性、声明来源与输入 inventory。默认 `legacy` 保留上述边界；两者均不分析 Rust 符号或运行时调用。

## CLI 与结果契约

仅实现 `check`，必需 `--project`、`--contract`；可选 `--facts`、`--report` 输出路径；`--profile` 接受 `legacy`（默认）或 `cargo-declarations-v1`。默认在 stdout 输出格式化 JSON 报告；**传入 `--report` 后仅写文件，不再向 stdout 打印报告**。`--facts` 在求值前写入独立 JSON。父目录必须已存在；每个输出使用暂存后原子替换，拒绝受保护项目/合同输入及其别名，并在执行前清理可安全识别的旧输出。多个输出不是一个事务；必须检查本次退出状态，错误不能当作新证据。输入/运行时错误在 stderr 输出 `archguard:` 诊断。

| 退出码 | 含义 |
|---|---|
| 0 | 范围内 `ALLOW`，可能仍有建议级匹配 |
| 2 | `BLOCK`：强制规则违规或分析不完整 |
| 3 | `REQUIRE_APPROVAL`：存在 review 匹配且无阻断项 |
| 4 | 参数/合同错误、根目录不可访问、输出失败等；不保证有新报告 |

参数解析拒绝未知/重复参数及缺失的值；不提供常规 `--help`。请使用上面的准确命令。`guard.partme.ai/v1alpha1` 是为兼容保留的协议名称；合同/事实/报告拒绝未知字段，仅支持精确 `forbid_relation`。`enforce` 匹配阻断，`review` 匹配要求审批，`advise` 匹配仍为 `PASS` 并保留证据；当前没有审批命令。

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

未来 ArchGuard 以四层守卫为完整产品范围：**System Guard（系统/模块边界）**、**Domain Guard（领域/聚合不变量）**、**Object Guard（类型/对象职责）**、**Method Guard（方法调用、签名和副作用）**。已实现上述静态系统/领域库能力、Java/TypeScript provider 和观测差异；深度 Rust 符号、完整对象/方法语义、通用 API 兼容分析及生产可信 CI 仍未完成。

**原生 CLI 仍面向 Cargo；库能力按显式静态 profile 发布**。对象/方法合理性不能通过 AI 的“高置信度”得出确定性 PASS，必须以批准的架构契约、真实语言事实和必要的人工评审为依据。

完整方案见 [详细架构与 ADR](docs/architecture.md) 和 [技术方案、阶段实施与测试](docs/technical-design.md)。

## 集成边界

SpecGuard、ArchGuard、CodeGuard、TestGuard、GitGuard、FlowGuard 是六个独立领域守卫，共用 GuardEngine，不存在 GuardCore。引擎负责通用合同校验、中立规则求值与确定性证据，ArchGuard 负责架构事实与语义。支持的 Cargo profile 已验证实际 GG/FG 消费与本地分阶段回滚；生产编排/authority、统一 MCP 接口和完整语义 Design Diff 仍是目标。可选本地 Git 桥接已绑定实际候选/base/group/member 对象，不提供队列授权。[集成契约](docs/integration-contract.md) 已作为独立严格 envelope 实现，原生 GuardFacts/GuardReport schema 不变。保留的 GG CodeGraph adapter 另有受限 JavaScript export 验证范围，不是 AG Java 索引 importer 或通用已安装索引器。AG 外部 CodeGraph/codereview-plugin 集成和宿主安装仍未验证。

## 验证

运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`。
许可证：Apache-2.0。


## OpenSpec 实施待办

[proposal](openspec/changes/extend-architecture-analysis-and-evidence/proposal.md)、[design](openspec/changes/extend-architecture-analysis-and-evidence/design.md)、[规范](openspec/changes/extend-architecture-analysis-and-evidence/specs/)与[tasks](openspec/changes/extend-architecture-analysis-and-evidence/tasks.md)记录截至 2026-10-10 的 **19/25 项验收**。2.4、3.3、3.5、5.2、5.3、5.5 仍未勾选，精确缺口见[本地闭环审阅](docs/reviews/local-closure-20261010.md)。历史验证记录保留原检查点的含义。

已批准的 skills-first 计划已交付独立核心、可选 Git/Spec 包和本地 Java 原生 provider/wrapper。Provider `fad6395` 保留原三项测试，增加四项结构测试（环、命名、静态状态访问、导入覆盖）；wrapper `1ed1beb` 固定其来源。原生证据只验收这些明确结构示例，不证明运行时不变量、纯度、GE 认证证据或真实宿主安装。此前 wrapper 应用运行只覆盖原三项测试；完整 pin 与证据边界见闭环审阅。
