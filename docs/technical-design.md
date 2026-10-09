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
| 策略 | **远期可选，尚未选定** OPA/Rego 适配 | 需 GuardEngine 独立 ADR、确定性/资源与安全评审；非当前能力或接入前提 |
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

仓库范例来源：[examples/agent-job-contract.yaml](../examples/agent-job-contract.yaml)。

输入 `GuardFacts` 的 AnalyzerIdentity、GuardSubject(snapshotDigest)、completeness、facts、diagnostics 由 ArchGuard 创建；GuardEngine 执行具体关系断言。**v1alpha1 无领域不变量通用表达式、类型/方法 DSL、验证器注册和强制身份认证**，不能通过扩展 YAML 想象这些能力已存在。

规划新增专业事实：ModuleId、TypeId、SymbolId、CallEdge、OwnershipEdge、StateTransition、ExternalIOEffect、PublicApiSignature、SourceSpan（含解析语言与文件相对路径）、AnalyzerScope/Capabilities。跨语言 SymbolId 需命名空间、文件摘要、签名及索引器版本，避免误将同名重载方法合并。facts 必须可追溯到真实代码或批准 ADR。

## 4. 分析执行流程

1. **Discover**：检查项目语言、构建系统、模块、features/targets、可用索引器；只读，不能启动未经授权的构建脚本。
2. **Freeze Scope**：从批准的 Architecture Contract 取得必查边、模块、符号和义务；分析器必须声明可覆盖范围。
3. **Extract**：分语言解析，并同时报告每一个未解析模块/文件/调用范围；不支持的范围如能形成合法事实则声明 partial；真实崩溃/超时在目标集成层为 error、decision=null，取消为 cancelled。保留已获得的局部诊断，不伪造完成决策。
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

一条任务可能把方法从 Aggregate 移到 ApplicationService：此时捕捉类型所有权、调用方、事务/IO 副作用、可见性和领域状态规则的**具体差异**。只有批准契约明确禁止的模式才 BLOCK；没有充分语义数据而模型只是认为“应该放在另一个类”时产生评审义务（目标；当前只有匹配 `review` 禁边规则会产生 `REVIEW_REQUIRED`）。

多 Agent 不同文件变更同一 API，ArchGuard 输出 providedApi/consumedApi 与受影响测试义务；GitGuard 负责候选冲突与合并，TestGuard 验证真实行为。CodeGraph 索引需与当前源码绑定，过期时不给“无影响”结论。

## 7. 安全可靠性

- 范围边界：对只读分析器限制访问目录，符号链接逃逸、外部路径和非预期构建脚本需要额外授权或阻断。
- 事实可靠性：source span、摘要和输入树绑定；缓存以语言工具/规则/源码哈希组合键控。
- 输出安全：日志限制大小、截断声明、敏感源码脱敏；不能执行从代码注释或模型报告中提取的指令。
- 超时和取消：目标适配器施加子进程资源预算与逐分析器超时；超时/崩溃为 error、decision=null，取消为 cancelled。局部事实或历史 BLOCK 报告只作诊断工件，不冒充已完成运行。当前 CLI 的 metadata 故障仍按第 10 节返回 partial/BLOCK，不在文档变更中改写既有行为。
- Provenance：本地 SHA-256 一致性不是受信供应链 attestation；签名、可信运行身份及权限控制后续实施。

## 8. 实施计划与真实验收

| Wave | 交付物 | 正负例测试 |
|---|---|---|
| A0（已有） | Cargo workspace 精确直接依赖规则、真实 CLI | 现有 5 个集成测试；证据重算由 GuardEngine 提供，当前 ArchGuard 测试未覆盖 |
| A1 | 分析目标覆盖、规则适用性、真实 Git candidate hash | 未解析源模块/漏查文件必须 BLOCK |
| A2 | Java/TS/Rust 模块分析适配 | 合法分层/禁止层/循环/部分解析 |
| A3 | 聚合状态机、不变量测试契约、对象/方法规则 | 合法 ApplicationService 协调不误报；非法状态转换失败 |
| A4 | 公共 API / Method Design Diff、CodeGraph 符号影响 | 过期索引、动态分派、重命名均准确披露 |
| A5 | MCP/CLI/CI/OPA/受信策略、跨语言发布与兼容 | Agent 改弱策略或重用旧证据不能合入 |

测试分单元（关系提取、路径、摘要）、属性（顺序不影响规范化事实）、集成（真实 Cargo/JVM/TS 编译）、故障（工具退出、未知语法、超时）与安全（策略篡改、旧提交、跨租户）。所有硬规则至少配合法、违规、覆盖不足三类负例，并实际验证服务端门禁。

## 9. 规格与责任

本技术方案是 ArchGuard 目标技术边界，需求/任务另由 [OpenSpec](../openspec/changes/add-cargo-workspace-guard/) 跟踪。不要把设计文档当成已实现能力。

对系统/领域/对象/方法组织合理性的最终重大取舍，仍需经过批准的 ADR 与实际工程评审；工具负责提供可复现的事实和约束验证，而不是替人决定所有设计。 

## 10. 当前执行顺序与可观测错误

审阅基线：`566fda92c7186a38e53ac81347ba23e0a8107443`。实现映射：[src/main.rs](../src/main.rs) 负责参数/文件和退出码，[src/lib.rs](../src/lib.rs) 负责领域提取；GuardEngine 的 `protocol.rs`、`engine.rs` 负责类型校验、关系求值及证据重算。以下描述当前实际顺序，不是第 4 节目标流水线的已实现声明。

1. 检查命令为 `check`；从参数窗口取首次出现的 `--project`、`--contract` 对应值。没有 `--help`、通用参数 schema、重复/未知参数拒绝机制。
2. 读取并验证合同，再 canonicalize 项目根目录。失败直接返回错误，不产生 partial。
3. 在根目录执行 `cargo metadata --no-deps --offline --format-version 1 --manifest-path <root>/Cargo.toml`，捕获 stdout/stderr；无 `--locked`、显式 feature/target 配置或超时。
4. 收集根清单、实际 workspace member 清单与成员直接 path 依赖；按规范化目录映射实际包名，来源为声明依赖的包清单。排序去重后生成 complete 事实。
5. 若第 3/4 步失败，丢弃已收集事实，返回 `partial`、空 facts、一个诊断和 fallback digest。该 digest 为 `SHA256("incomplete:" || 可读取的根清单字节)`，不能视为有效候选证明。
6. 若传入 `--facts` 先写 JSON，再调用 engine evaluate，随后写 `--report` 文件，未指定则输出 stdout。事实写失败将阻止求值；报告写失败可留下已写 facts 与旧报告，结果为退出 4。
7. 根据报告 decision 返回 0/2/3；其他错误在 stderr 输出 `archguard: …` 并返回 4。当前没有通用 JSON 错误 envelope、原子文件事务、重试或取消状态。

| 情况 | 当前产物 | 退出/诊断 |
|---|---|---|
| 参数/合同无效，根目录不存在 | 不保证新产物 | 4，stderr |
| 根存在但无 Cargo.toml，Cargo 不可启动/非零/JSON 无效，清单读取失败 | partial 事实 + BLOCK 报告（若写成功） | 2；diagnostics 进入事实与 evaluation.detail，不单独保证 stderr |
| 完整事实，无禁边 | ALLOW 报告 | 0 |
| enforce 禁边 | FAIL + 原始匹配事实 | 2 |
| review 禁边，且无 FAIL/INDETERMINATE | REVIEW_REQUIRED | 3 |
| advise 禁边 | PASS + advisory detail + 匹配事实 | 0（若无更强结果） |
| 输出路径不能写 | 可能部分产物/旧文件 | 4，stderr |

## 11. 现有 wire 字段、摘要和复核合同

以下字段均是当前固定 schema，无自由扩展字典；未列出的新领域对象不能直接加入现有消息：

| 对象 | 字段与约束 |
|---|---|
| GuardContract | `apiVersion, kind, metadata{id,revision}, spec{rules}`；非空规则列表和唯一非空 rule ID |
| Rule | `id, description`（省略默认空字符串）, `enforcement, assertion{type,subject,predicate,object}`；唯一 assertion type 为 `forbid_relation`，精确字符串比较 |
| GuardFacts | `apiVersion, kind, analyzer{id,version}, subject{id,snapshotDigest}, completeness, facts, diagnostics`；partial 必须提供诊断 |
| Fact | `subject,predicate,object,source`，非空字符串；Cargo 当前 predicate 为 `depends_on` |
| GuardReport | `apiVersion,kind,evaluationId,engineVersion,contractId,contractRevision,contractDigest,factsDigest,analyzer,subject,evaluations,decision,signed` |
| RuleEvaluation | `ruleId,status,enforcement,matchedFacts,detail`；当前 Cargo 路径不生成 NOT_APPLICABLE |

原型没有规则对成员/能力适用性的校验、SourceSpan 行列、覆盖声明字段、审批记录或签名。协议支持的 `NOT_APPLICABLE` 枚举不代表当前 evaluator 会产生它；partial 优先于 enforce/review/advise，每条规则 INDETERMINATE，并最终 BLOCK。

manifest 摘要算法：以路径为 BTreeMap 键去重并排序，对每项顺序输入 little-endian u64 路径字节长度、UTF-8 路径字节、little-endian u64 内容长度、原始内容字节，计算 `sha256:<hex>`。路径用 `/`，优先相对根目录；原始 TOML 不做语义规范化，所以注释变动也会改变摘要。根清单与成员清单同路径仅记录一次。`Cargo.lock`、配置、源码、Cargo 版本、环境和 Git ref 不在该摘要内。

GuardEngine 对已解析合同和排序去重事实的 JSON 序列化分别摘要，evaluationId 绑定协议、引擎版本、合同与事实摘要。合同摘要不是原始 YAML 字节哈希。事实 subject.id 包含调用时传入的项目路径字符串，因此等价目录的不同写法也可改变 factsDigest/evaluationId。重算整个报告的一致性不证明事实是真实提取的。

可运行的外部复核（从 ArchGuard 根目录；先生成文件）：

~~~sh
cargo run -- check --project fixtures/allowed --contract examples/agent-job-contract.yaml --facts /tmp/archguard-facts.json --report /tmp/archguard-report.json
cargo run --manifest-path ../guardengine/Cargo.toml -- verify --contract examples/agent-job-contract.yaml --facts /tmp/archguard-facts.json --report /tmp/archguard-report.json
~~~

第二条是 GuardEngine 命令，不是 ArchGuard 子命令。verify 成功时返回原始 decision 的退出码（阻断报告仍为 2），诊断在 stderr；失败为 4，不签名、不访问批准系统、不给合入授权。可信 CI 仍须从同一候选重新分析。

## 12. 目标集成、并发和扩展边界

采用独立的 [GuardRunEnvelope 草案](integration-contract.md)（`guard.integration/v1alpha1`），当前 engine 不解析。它把运行状态 completed/error/cancelled、失败时为空的 decision、repo/task/worktree/requirement/candidate/base/merge-group 绑定、合同/事实/报告摘要引用、分析器覆盖、外部审批与诊断分离。领域 error code 在该包装中版本化，不能给当前 GuardReport 增加 error 字段或虚构 decision=ERROR。

建议控制器先冻结输入绑定，再按绑定创建幂等运行记录；任务输出写隔离目录，完成后原子发布只属于该绑定的结果。发布时 compare-and-set 当前候选，过期运行仅归档。改变 candidate/base/merge-group、合同/规则集、分析器/覆盖、基线 revision 或审批状态时失效重跑。若最终 merge-queue candidate 与被检查 head 不同，必须分析实际队列候选，不能缓存复用旧 head 的 ALLOW。

新增语言适配器需公布稳定 ID、工具版本、输入配置、支持关系与已知盲区。ArchGuard 做领域规则到中立事实/断言的映射；新关系/图算法先制定版本化契约再实现，GuardEngine 不导入 Java/Rust/DDD 专属代码。没有能力或缺覆盖的必查义务阻断，不能仅以空事实使规则通过。签名由可信执行环境/外部审批服务完成，不由本地模型自行产生信任。

## 13. 验证清单、阶段量化门槛与未完成项

现有 [tests/integration.rs](../tests/integration.rs) 的五项分别是 allowed 提取/ALLOW、forbidden 提取及来源/BLOCK、missing manifest partial/INDETERMINATE、真实 CLI forbidden=2、真实 CLI allowed=0。没有现成测试证明重命名依赖、dev/build/optional/target、路径逃逸、manifest 摘要稳定性、review/advise CLI、写文件错误或 verify 行为；阅读到测试不等于本次运行通过。

| 阶段 | 可测验收门槛（目标） |
|---|---|
| A1 覆盖与稳健性 | 每个 dependency kind/rename/target/non-member 有正负例；根缺失=4 与清单缺失=2 可区分；未知必查包不误 PASS；摘要顺序稳定且逐输入变更失效 |
| A2 语言适配 | 每个发布语言至少合法/违规/不支持三类真实 fixture；配置矩阵列出所有承诺 target/features；不支持场景必报 coverage gap |
| A3 领域/对象/方法 | 每个 enforce 规则具合法、违规、覆盖不足案例；合法应用服务协调不会因同名方法误报；非法状态转换有对应 TestGuard 义务 |
| A4 Design Diff | 基线变化、候选变化、重命名、索引过期、动态分派各有案例；报告给出每个变化的来源与未覆盖消费者，未知不冒充零影响 |
| A5 受信集成 | 候选替换、基线修改、审批到期/撤销、迟到运行和跨任务重放各被拒绝；队列候选重新检查；超时/取消不产生成功 decision；无授权无法绕过门禁 |

当前 CI 对 GuardEngine 使用 sibling checkout 且未指定 immutable ref；稳定 toolchain 和 checkout Action 也不是固定二进制供应链证明。现有 CI 只跑本仓 fmt/clippy/test，不是受保护业务仓库 merge gate。发布前应锁定依赖版本、核验新旧 schema 兼容和最小工具链，并保留协议回归 fixture。

OpenSpec task list 中原型任务已勾选，发布 GuardEngine、Java/TS/深层 Rust、受保护策略和签名仍未完成；本文 A1–A5 并不在现有已完成规格中。待决策包括覆盖 schema 的版本演进、源码/环境全量摘要策略、SymbolId、性能预算、审批供应商和审计保留期。默认先隔离扩展与当前协议，避免无版本字段扩张。

本次文档核对（2026-10-09）：已逐项阅读源码、5 个现有测试和 OpenSpec，检查文档相对链接与 `git diff --check`。环境没有可用的 Cargo/OpenSpec 命令，因此未声称本次 fmt/clippy/test 或 OpenSpec validate 通过。上表新增案例仍是待实现的验收目标。


### 集成错误的绑定前置条件

上述目标 error/cancelled 信封只适用于调用身份、精确候选/基底、producer 与必查覆盖已经冻结的尝试。参数非法、仓库不可解析或绑定歧义等前置故障使用独立传输诊断和失败退出状态，不生成 GuardRunEnvelope，不伪造 OID 或空字段；当前各 CLI 的既有行为仍按本文事实表保留。详见[共享契约](integration-contract.md)。
