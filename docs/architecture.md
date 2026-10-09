# ArchGuard — 架构设计

> 文档版本：0.2.0（目标架构）。当前软件：0.1.0 Cargo 依赖原型。日期：2026-10-09。项目名统一为 **ArchGuard**。

## 1. 定位与原则

ArchGuard 是独立的**架构约束验证工具**：让编程 Agent 在已有系统边界、领域设计、对象职责和方法契约内演进代码。AI 可以提出方案和修复建议，但不能自行改写已批准架构规则或将设计意见冒充确定性证明。

**拥有**：架构规则与包、系统/模块依赖、领域模型契约、类型/对象设计、方法签名与副作用约束、Architecture Fitness Functions、Design Diff 与架构风险证据。

**不拥有**：需求基线（SpecGuard）、普通 lint/security 规则（CodeGuard）、真实测试执行与覆盖（TestGuard）、Git 分支/合入（GitGuard）、阶段审批（FlowGuard）、通用协议/规则计算/证据框架（GuardEngine）。

架构设计“符合已批准约束”不等于“设计是唯一或最优的”：可检查关系为 **ENFORCE**；职责归属争议、抽象合理性、演进成本等需基于事实进入 **REVIEW**；无需立即处理的优化为 **ADVISE**。自然语言启发式评分不得变成自动强制阻断。

## 2. 产品总体架构

~~~text
     Approved ADR / Architecture Contract / Domain Invariants
                              │
            ┌─────────────────┴─────────────────┐
            ▼                                   ▼
       Rule Packages                       Source Snapshot
  system/domain/object/method                    │
            │                         Language Fact Providers
            │                     Cargo / ArchUnit / TS / SCIP
            │                                   │
            │                              Source Graph
            │                     Module → Type → Method
            │                         calls / depends / owns
            │                                   │
            └─────────────────┬─────────────────┘
                              ▼
             Architecture Analysis / Design Diff
      forbidden edges / state contracts / API drift / risks
                              │
                  GuardFacts + Findings
                   source + scope + digest
                              │
                          GuardEngine
                 Contract / Rule / Evidence
                              │
             CLI / CI / MCP / FlowGuard / CodeReview
~~~

事实提供者必须报告“实际分析了什么”：语言、模块、文件、feature/target、工具版本、静态/动态分派局限以及缺失项。CodeGraph 和 SCIP 只提供符号/调用事实，不自行判定设计优劣。无法解析的边不能作为“没有违规边”的证明。

## 3. 四层架构治理

### 3.1 System Guard

规则包括：组件职责/分层、禁止反向依赖、领域隔离、禁止循环、依赖方向、跨服务 API 和公共契约、数据库/事件所有权约束。现有 0.1.0 仅实现**同 Cargo 工作区成员的直接本地 path 依赖**精确禁止边检查；尚未实现传递依赖、Java、JS/TS 或源码级导入关系。

### 3.2 Domain Guard

领域契约为 Context、Aggregate、Entity、ValueObject、DomainService、ApplicationService、Event、Invariant、Transition。可以硬检查批准的状态转换、不变量所关联的行为测试、明确禁止的依赖及数据库访问边界，但“哪个对象应拥有某行为”未写成契约之前，应形成待评审 ADR，而不是依据名称打分后自动阻断。

举例：应用服务中有 cancelTask() 可负责鉴权、事务和对外协调，AgentTask 聚合内的 cancel() 管理状态不变量，执行器负责资源释放；名称相似**不等于设计重复**。检查真实职责与副作用。

### 3.3 Object Guard

输入包括类型定义、接口/继承/组合、可变字段、公共 API、使用者、跨模块类型依赖、变更前后图谱。已批准的类型可见性、禁止继承、依赖边可以硬检查；类过长、缺少接口、接口实现过多等指标只能触发 REVIEW，不机械使用行数分数宣称不合格。

### 3.4 Method Guard

检查方法签名、可见性、调用方向、纯度/事务/IO 副作用契约、错误与异常传播、状态转换入口、公共 API 兼容。语言适配器需要足够的类型精度；宏、反射、动态分派、跨语言调用无法完整解析时返回 UNKNOWN/partial，不补猜测调用边。

## 4. 当前实现的真实能力（v0.1.0）

源代码 [src/lib.rs](../src/lib.rs) 中 **CargoWorkspaceAnalyzer** 实现 GuardEngine 的 GuardAnalyzer：

1. 对目标工作区运行 `cargo metadata --no-deps --offline --format-version 1`；
2. 识别实际 workspace_members；以规范化目录定位成员包；
3. 从成员包的声明中提取 **path 指向另一成员** 的直接依赖；
4. 生成中立关系事实 `(subject, depends_on, object, source)`，例如 `agent-job → agent-saas`；
5. 把读取的 Cargo.toml 内容规范化哈希为 `snapshotDigest`（**不是完整 Git Tree/Commit**）；
6. 通过 GuardEngine 载入合同并执行 `forbid_relation`，输出 JSON 报告与 CLI 退出码。

CLI 和示例见 [README](../README.md) 与 [测试](../tests/integration.rs)。该原型无 Rust use/函数级语义、无 Java/TS 静态索引、无可信签名/用户审批。依赖 `../guardengine` 是临时本地 path 依赖，需改为带版本的不可变发布包。

## 5. 规则、证据、决策与信任边界

同一 Guard Protocol v1alpha1 用 GuardContract、GuardFacts、GuardReport 表达约束、事实和报告；完整分析声明只对**该分析器公布的 scope**有效。必需扫描失败、工具丢失或只分析了部分源码，状态必须 INDETERMINATE，最终 BLOCK；不能把环境错误冒充目标代码的架构违规。

模式 `enforce`、`review`、`advise` 表示处置策略，不表示规则精度。架构风险不能靠模型置信度升级为 ENFORCE；有审批例外时保留原始 Finding，记录有时限的可验证例外。

可信合并需要：受保护规则版本、当前最终候选 SHA/Tree、独立分析器执行、匹配签名/权限、required CI 和 GitGuard 最终版本核验。**本地 unsigned report 只对开发者提供指导，不能给 Agent 直接合入权限。**

## 6. 与其他组件的协议

- GuardEngine：纯粹通用规则、契约及证据模型，不能导入 ArchGuard 领域代码。
- SpecGuard：提供已批准的 Requirement 和 ADR 关联，不评价架构实现。
- CodeGuard：执行编译、lint/security；复杂度分数是架构风险的辅助事实，非证明。
- TestGuard：执行状态机、不变量及消费者契约测试，回传对同一候选的证据。
- GitGuard：提供候选变更集、分支基线和符号冲突观察，保证最终候选重新验证。
- FlowGuard：管理架构评审、例外与下一阶段审批，不代替架构检查。
- CodeGraph/codegraph-plugin：提供索引及引用/调用关系，索引版本和覆盖缺口必须同时传递。
- codereview-plugin：提交 DESIGN REVIEW/ADVISE 建议，不能自行变更批准 ADR。

## 7. Architecture Fitness Functions 与 Design Diff

不输出万能“架构质量分数”，而是维护可以复核的指标：新增禁止边数量、依赖环、公共 API 扩张、模块拥有权变化、聚合不变量回归、类型依赖 fan-in/out 和重大设计风险。报告同时显示**基线和候选**，给出具体新增边、受影响类型/方法/消费者、规则 ID 与复核路径。

CodeGraph/SCIP 的索引不完整时显示“本次影响分析无法覆盖哪些调用方”，不隐瞒未知。两个 Agent 不改同一文件仍可能共享 API；风险分析可提前预警，但最终合并仍由 GitGuard + TestGuard 验证。

## 8. ADR 基线

- ARCH-ADR-001：规则属于 ArchGuard，GuardEngine 只执行通用机制。
- ARCH-ADR-002：语言精确性与覆盖明确声明；不支持的语义绝不默认通过。
- ARCH-ADR-003：对象/方法合理性以批准契约+事实为证，启发式仅 REVIEW。
- ARCH-ADR-004：Design Diff 面向候选与批准基线，不机械使用代码行数阈值。
- ARCH-ADR-005：真实信任边界位于独立 CI、受信策略和受保护 Git，而非模型/Hooks。

## 9. 分阶段实施与验收

| 阶段 | 实际/目标 | 验收 |
|---|---|---|
| A0 | **已有**：Cargo 直接 path 依赖事实与禁止边 | allowed=0、forbidden=2、partial=BLOCK |
| A1 | 完善 rule scope、图谱覆盖、版本化 analyzer 能力 | 缺少被检成员/配置导致 INDETERMINATE |
| A2 | Java ArchUnit、TS dependency-cruiser、Rust 模块/类型分析 | 多语言合法/违规/不支持真实差分 |
| A3 | Domain 状态机与对象/方法设计契约 | 合法协调模式不误报，绕过聚合状态测试阻断 |
| A4 | Design Diff、SCIP/CodeGraph、API 影响与跨项目审查 | 图谱过期/部分解析仍显式 unknown |
| A5 | MCP/CI、签名证据、真实受保护候选 | 无权限绕过分支保护失败 |

技术模块、DSL/CLI、错误和测试设计见 [技术方案](technical-design.md)。

**注意：只有 A0 有当前实现支撑，后续均为 OpenSpec/代码实施目标。**
