## ADDED Requirements

### Requirement: Versioned Language Fact Providers

ArchGuard SHALL 为 Java、TypeScript 和深层 Rust 逐一发布版本化 capability、配置矩阵、稳定符号身份和来源；解析器依赖先评估再固定版本。未支持语法或动态语义 MUST 显式报告缺口，领域解析不得迁入 GuardEngine。

#### Scenario: Overloads and dynamic dispatch
- **WHEN** 同名重载和无法静态确认的动态调用同时存在
- **THEN** 已确认符号保持独立身份，未知调用保留未知范围而非猜测确定边

#### Scenario: Language capability unavailable
- **WHEN** 用户要求尚未发布的语言或配置
- **THEN** 返回 unsupported capability，不复用 Cargo complete 作为该语言完整覆盖

### Requirement: Deterministic System Architecture Rules

ArchGuard SHALL 在具备完整所需关系的 profile 中验证批准的层级、禁止方向和依赖环规则，输出规则 ID、路径与来源证据；每条 enforce 规则 MUST 有合法、违规和缺覆盖 fixture。

#### Scenario: Cycle introduced across modules
- **WHEN** 候选在完整模块图中引入批准规则禁止的环
- **THEN** finding 给出具体环路径和边来源，并按受保护 enforcement 求值

#### Scenario: Missing module in cycle analysis
- **WHEN** 必查模块无法解析
- **THEN** 环规则不把不完整图的无环结果当作满足义务

### Requirement: Approved Domain Object and Method Contracts

ArchGuard SHALL 将领域状态/不变量、类型职责/API、方法签名/副作用关联到批准契约与行为测试义务；启发式设计建议 MUST 保持 review/advise，不凭模型置信度升级 enforce，也不代替 TestGuard 执行测试。

#### Scenario: Legitimate coordination methods
- **WHEN** ApplicationService.cancelTask 负责协调而 Aggregate.cancel 负责批准的状态不变量
- **THEN** 不因名称相似判定重复设计，报告以契约、调用与副作用事实为依据

#### Scenario: State invariant is bypassed
- **WHEN** 完整语义证据显示候选绕过批准的状态入口
- **THEN** 产生可定位违规及对应 TestObligation 引用，不声称该行为测试已执行

### Requirement: Immutable Architecture Baseline

ArchGuard SHALL 绑定不可变架构合同/ADR 摘要、来源 ref 与外部审批记录，支持 draft、in_review、approved、superseded/revoked 生命周期。Requirement/ADR trace SHALL 对接 SG-BASELINE；审批认证属于受信控制器。

#### Scenario: Candidate edits approved ADR
- **WHEN** 候选修改标记 accepted 的 ADR 或提供 approved=true
- **THEN** 该内容不能覆盖原批准基线，必须成为新修订与待审核差异

#### Scenario: Approval expires or is revoked
- **WHEN** 基线或例外审批过期、撤销或认证服务无法确认
- **THEN** 原证据失去对应授权资格，技术报告保留且 REQUIRE_APPROVAL 不被改写

### Requirement: Source Bound Design Diff

ArchGuard SHALL 对批准基线与候选比较模块、所有权、公共 API 和方法变化，输出受影响消费者、来源及缺覆盖。索引 MUST 绑定输入/config/analyzer 身份；过期或不完整索引不得证明零影响。

#### Scenario: Public method renamed with stale index
- **WHEN** 候选重命名公共方法而消费方索引来自旧 snapshot
- **THEN** 拒绝把旧索引作为当前完整影响证明，并列出需重新索引范围

#### Scenario: Independent tasks share an API
- **WHEN** 两个 requirement 改动不同文件但共享公共 API
- **THEN** 差异包含共享依赖与各自测试义务，不将文件不冲突等同于架构兼容
