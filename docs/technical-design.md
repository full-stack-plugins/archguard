# ArchGuard — 技术方案与实施设计

> 目标版本 V0.2；软件基线 Rust 0.1.0。已实现功能仅限 Cargo workspace 本地直接依赖禁止边检查，其他均为目标设计。

## 1. 技术选型

| 组件 | 已有 / 建议 | 责任及限制 |
|---|---|---|
| 核心运行时 | **已有** Rust 2024，Serde/JSON，sha2 | 模块职责与类型语义规则属于 ArchGuard |
| 规则和报告 | **已有** 依赖同级 GuardEngine 0.1.0，v1alpha1 | 该路径依赖是 PoC，后续升级成固定版本发布 |
| Rust 包依赖 | **已有** `cargo metadata --no-deps --offline` | 仅 workspace 成员间本地 path 依赖 |
| Java | **规划** ArchUnit + JVM 字节码/Gradle/Maven 适配 | 架构层和类型依赖，不替代业务规则 |
| TypeScript | **规划** dependency-cruiser + TypeScript compiler API | 模块/类型关系，跨包路径别名要精确解析 |
| Rust 深度语义 | **规划** rust-analyzer/编译器数据或 SCIP 索引 | 类型/方法/调用关系；宏和 cfg/feature 限制须披露 |
| 通用结构 | **规划** ast-grep/Tree-sitter | 结构匹配不是完整类型语义或运行时调用图 |
| 策略 | **规划** GuardEngine OPA 适配器 | REGO 执行已批准规则，不重写 AST |
| CLI/MCP/CI | **已有** `archguard check` CLI；**规划** rmcp、Action | 每项接口独立发布/验证 |

首版不要把所有语言强制塞进一个 AST API。各语言提取**同一标准化语义事实**和精确覆盖声明，专业规则以语言 capability 检查可执行性，不支持/不完整则 UNKNOWN。

## 2. 模块拆分（逐阶段演进，不强制一次重构）

~~~text
archguard/
├── src/lib.rs                 # 已有 CargoWorkspaceAnalyzer
├── src/main.rs                # 已有 check 命令
├── analyzers/
│   ├── cargo-workspace/       # A0：提取直接成员边
│   ├── rust-symbols/          # A2：cfg、HIR/符号引用
│   ├── java-archunit/         # A2：JVM 架构
│   ├── typescript/            # A2：TS 模块和类型
│   └── codegraph/             # A4：索引与 Design Diff
├── domain/
│   ├── system/                # 分层、循环、模块所有权
│   ├── bounded-context/       # 上下文、聚合、事件
│   ├── object/                # 类型、接口、字段、继承
│   └── method/                # 方法签名、调用和副作用
├── schemas/
├── fixtures/
├── tests/
├── docs/
└── openspec/
~~~

上述新目录为目标模块规划，当前并不存在；不要求每个目录都成为独立 crate。现有原型继续可运行，新增适配器遵守已发布 GuardAnalyzer trait 或它的版本化扩展。

## 3. 事实、合同、协议

当前 Guard Protocol v1alpha1 支持：

~~~yaml
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
~~~

已验证范例来源：[examples/agent-job-contract.yaml](../examples/agent-job-contract.yaml)。

输入 `GuardFacts` 的 AnalyzerIdentity、GuardSubject(snapshotDigest)、completeness、facts、diagnostics 由 ArchGuard 创建；GuardEngine 执行具体关系断言。**v1alpha1 无领域不变量通用表达式、类型/方法 DSL、验证器注册和强制身份认证**，不能通过扩展 YAML 想象这些能力已存在。

规划新增专业事实：ModuleId、TypeId、SymbolId、CallEdge、OwnershipEdge、StateTransition、ExternalIOEffect、PublicApiSignature、SourceSpan（含解析语言与文件相对路径）、AnalyzerScope/Capabilities。跨语言 SymbolId 需命名空间、文件摘要、签名及索引器版本，避免误将同名重载方法合并。facts 必须可追溯到真实代码或批准 ADR。

## 4. 分析执行流程

1. **Discover**：检查项目语言、构建系统、模块、features/targets、可用索引器；只读，不能启动未经授权的构建脚本。
2. **Freeze Scope**：从批准的 Architecture Contract 取得必查边、模块、符号和义务；分析器必须声明可覆盖范围。
3. **Extract**：分语言解析，并同时报告每一个未解析模块/文件/调用范围；工具崩溃与不支持为 partial。
4. **Normalize**：跨平台路径归一，按稳定顺序生成事实；原生诊断和来源位置不能丢失。
5. **Evaluate**：确定性规则可 ENFORCE；领域/设计启发式只 REVIEW；缺强制能力或覆盖不足无法 ALLOW。
6. **Design Diff**：基线、候选依赖图和公开 API 差异，关联相应 Requirement/ADR/TestId。
7. **Emit Evidence**：规则 ID、source spans、subject digest、contract digest、analyzer/verifier 版本、检查范围和未满足原因。
8. **Trusted Recheck**：CI 在保护的规则快照和最终候选重新分析；不能信任 PR 更改后的自签 YAML。

## 5. 工程 CLI / MCP / CI

**真实可用（PoC）：**

~~~sh
cargo run -- check --project fixtures/allowed --contract examples/agent-job-contract.yaml
cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml
cargo test --all-targets
~~~

规则不违反时退出码为 0；发现违规时为 2，REQUIRE_APPROVAL 为 3，无效输入为 4。partial 事实返回 BLOCK；本地报告 unsigned。

**规划命令（尚不可执行）：**

~~~sh
archguard doctor --project .
archguard scan --project . --language java --format sarif
archguard design diff --base <approved-ref> --head HEAD
archguard domain verify --aggregate AgentTask
archguard symbols impact --target AgentTask.cancel
archguard explain --rule ARCH-001 --format json
~~~

MCP 未来暴露 doctor/scan/explain/diff/impact 的只读检查能力；报告转原生问题和修复建议，但不得提供“AI 自我批准架构例外”入口。GitHub Actions 应由受保护策略和已验证工具链执行，合并要求包括本域结果、TestGuard 不变量回归和 GitGuard 对真实 merge candidate 的核对。现有 .github/workflows/ci.yml 只验证当前 Rust 仓源码，不代表已经强制用户业务仓库的合并。

## 6. 并行研发/对象方法治理

一条任务可能把方法从 Aggregate 移到 ApplicationService：此时捕捉类型所有权、调用方、事务/IO 副作用、可见性和领域状态规则的**具体差异**。只有批准契约明确禁止的模式才 BLOCK；没有充分语义数据而模型只是认为“应该放在另一个类”时 REVIEW_REQUIRED。

多 Agent 不同文件变更同一 API，ArchGuard 输出 providedApi/consumedApi 与受影响测试义务；GitGuard 负责候选冲突与合并，TestGuard 验证真实行为。CodeGraph 索引需与当前源码绑定，过期时不给“无影响”结论。

## 7. 安全可靠性

- 范围边界：对只读分析器限制访问目录，符号链接逃逸、外部路径和非预期构建脚本需要额外授权或阻断。
- 事实可靠性：source span、摘要和输入树绑定；缓存以语言工具/规则/源码哈希组合键控。
- 输出安全：日志限制大小、截断声明、敏感源码脱敏；不能执行从代码注释或模型报告中提取的指令。
- 超时和取消：子进程资源预算、逐分析器超时、失败事实 partial；不可将工具失败降级为通过。
- Provenance：本地 SHA-256 一致性不是受信供应链 attestation；签名、可信运行身份及权限控制后续实施。

## 8. 实施计划与真实验收

| Wave | 交付物 | 正负例测试 |
|---|---|---|
| A0（已有） | Cargo workspace 精确直接依赖规则、真实 CLI | 现有 5 个集成测试和证据重算 |
| A1 | 分析目标覆盖、规则适用性、真实 Git candidate hash | 未解析源模块/漏查文件必须 BLOCK |
| A2 | Java/TS/Rust 模块分析适配 | 合法分层/禁止层/循环/部分解析 |
| A3 | 聚合状态机、不变量测试契约、对象/方法规则 | 合法 ApplicationService 协调不误报；非法状态转换失败 |
| A4 | 公共 API / Method Design Diff、CodeGraph 符号影响 | 过期索引、动态分派、重命名均准确披露 |
| A5 | MCP/CLI/CI/OPA/受信策略、跨语言发布与兼容 | Agent 改弱策略或重用旧证据不能合入 |

测试分单元（关系提取、路径、摘要）、属性（顺序不影响规范化事实）、集成（真实 Cargo/JVM/TS 编译）、故障（工具退出、未知语法、超时）与安全（策略篡改、旧提交、跨租户）。所有硬规则至少配合法、违规、覆盖不足三类负例，并实际验证服务端门禁。

## 9. 规格与责任

本技术方案是 ArchGuard 目标技术边界，需求/任务另由 [OpenSpec](../openspec/changes/add-cargo-workspace-guard/) 跟踪。不要把设计文档当成已实现能力。

对系统/领域/对象/方法组织合理性的最终重大取舍，仍需经过批准的 ADR 与实际工程评审；工具负责提供可复现的事实和约束验证，而不是替人决定所有设计。 
