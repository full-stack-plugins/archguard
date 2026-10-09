# ArchGuard Analysis and Evidence Implementation Plan

> **For agentic workers:** 后续执行按任务使用 superpowers:subagent-driven-development 或 superpowers:executing-plans；已获后续实现授权；截至独立复核提交 6e71e725，1.1/1.2/1.3/1.5 共 4/25 项获根协调者验收（restricted Cargo 范围），其余未勾选。

**Goal:** 从 Cargo 原型演进出范围明确、领域契约驱动、候选绑定且可审计的架构分析。

**Architecture:** ArchGuard 拥有语言提取、领域模型和规则意义；GuardEngine 保持通用 Contract/Rule/Evidence。现有 CLI 使用 legacy profile，增强领域 artifact 与独立 envelope 通过显式接口引入；认证与审批由控制器提供。

**Tech Stack:** 当前 Rust 2024 / Rust 1.85+、Cargo、Serde JSON、sha2、同级 GuardEngine。Java/TS/Rust 索引器、MCP/OPA 适配和信任 provider 均需任务内评估后选定，不视为已安装。

**Spec:** [analysis profile](specs/architecture-analysis-profile/spec.md)、[domain evolution](specs/architecture-domain-evolution/spec.md)、[evidence integration](specs/architecture-evidence-integration/spec.md)、[本 change design](design.md)、[架构](../../../docs/architecture.md)、[技术方案](../../../docs/technical-design.md)、[集成草案](../../../docs/integration-contract.md)、[跨仓路线图](../../guard-roadmap.md)。

## Global Constraints

- 基线 `566fda92c7186a38e53ac81347ba23e0a8107443`；保留原 5 项集成测试。新模块路径均为拟新增，旧 src/lib.rs/src/main.rs 只作窄集成，不先做无关重构。
- `guard.partme.ai/v1alpha1` 严格结构及 exact forbid_relation 不变；独立 `guard.integration/v1alpha1` 等待 GE-CONTRACT 冻结；无隐含 N/N-1 支持。
- 当前 check 保持 0/2/3/4、--report 写文件抑制 stdout；legacy metadata 失败仍 partial/BLOCK。增强执行错误在绑定后为 error/null decision，取消为 cancelled/null；绑定前独立诊断。
- 全部 scope/contract 在提取前冻结。approval 不改写 REQUIRE_APPROVAL，不覆盖 partial/工具失败；报告一致性不等于认证。
- 当前清单摘要不冒充全仓 identity。最终合并要精确 queue candidate；没有 ref-write、审批签发或默认外部副作用。
- [旧 TODO](../add-cargo-workspace-guard/tasks.md) 不改动：语言 umbrella 映射组 2–3；CI/签名 umbrella 映射组 4–5；引擎发布 umbrella 映射 5.5，发布本身由 GuardEngine 完成。不重复新建同一实现任务，不宣称旧变更完成。

## Review Focus

- 配置/锁文件/工具变化但清单相同，须按增强 inventory 失效：1.3。
- 不存在的必查成员不能因空图而 PASS；候选不能删除义务：1.1、3.1。
- dev/build/optional/target/rename 的区别不能被增强 artifact 丢失：1.2。
- 根错误、工具超时、有效 partial、取消与输出旧文件必须区分：1.4、1.5、4.2。
- 双任务与队列乱序、过期/撤销审批不能重用旧成功：4.3、4.4、5.1、5.4。

## 1. Current Cargo Hardening — 独立于跨仓 gates

**Surfaces:** 拟新增 `src/analysis/{profile,cargo,snapshot,runner}.rs`，现有 `src/lib.rs`/`src/main.rs`，`tests/analysis_profile.rs`/`tests/cli_compatibility.rs`，`fixtures/profile/`。
**Interfaces:** ProtectedScope → FrozenAnalysisProfile；冻结项目/Profile → ArchitectureObservation 或 typed failure；legacy projection 保持 GuardAnalyzer 的 GuardFacts。

- [x] 1.1 在 `src/analysis/profile.rs` 定义 FrozenAnalysisProfile 和规则适用性校验（Requirement: Frozen Architecture Analysis Profile）；`tests/analysis_profile.rs` 验证缺失成员、未支持关系、候选删除 scope，三者不能用无匹配满足完整义务；普通 allowed 仍通过。
- [x] 1.2 在 `src/analysis/cargo.rs` 保存真实成员、依赖 kind/optional/target/source 的声明模型并独立保留 legacy 投影（Requirement: Cargo Declaration Provenance）；`fixtures/profile/` + `tests/analysis_profile.rs` 覆盖 rename/dev/build/optional/target/non-member，断言增强声明不丢失、旧四元组去重且不产生非成员边。
- [x] 1.3 在 `src/analysis/snapshot.rs` 冻结 capability 输入 inventory 与 canonicalization，先以配置/锁文件/工具链影响 fixture 决定 profile（Requirement: Declared Snapshot Inventory）；`tests/snapshot.rs` 断言相同字节顺序变化稳定、每个声明影响输入变动失效，并保留旧 manifest digest golden。
- [x] 1.4 在 `src/analysis/runner.rs` 以隔离副本和 allowlisted roots 实现超时/输出/文件规模预算（Requirement: Bounded Architecture Execution）；`tests/runner_failures.rs` 用受控假工具覆盖超时、超额、路径逃逸与非零退出，断言停止执行且不产生成功空图，原工作树不被 Cargo 锁文件写入影响。
- [x] 1.5 在 `src/main.rs` 和 `src/integration/cli.rs` 的未来集成点冻结显式 profile 选择和安全输出发布（Requirement: Legacy Cargo CLI Compatibility）；`tests/cli_compatibility.rs` 运行原五项及 review/advise、根缺失、输出不可写 golden，断言 legacy 退出 0/2/3/4、stdout/--report 语义不变且失败不复用旧产物。

## 2. Language Providers and System Rules — profile 完成后逐语言推进

**Surfaces:** 拟新增 `src/analysis/{java,typescript,rust_symbols}.rs`、`src/domain/{model,rules}.rs`、`fixtures/languages/`、`tests/language_matrix.rs`/`tests/system_rules.rs`。
**Interfaces:** FrozenAnalysisProfile + language input → versioned ArchitectureObservation{symbols,edges,source,coverage}；observation + protected system rules → findings。三种适配器分别发布，未完成者不宣称支持。

- [x] 2.1 在 `src/domain/model.rs` 和 `docs/analyzer-capabilities.md` 冻结 Module/Type/Method/SymbolId、SourceSpan 与覆盖模型（Requirement: Versioned Language Fact Providers）；`tests/language_matrix.rs` 用同名重载、跨语言同名及未知分派 fixture 证明身份不混淆、未知不伪造确定边，并评估固定版本索引器的许可/资源边界。
- [x] 2.2 在 `src/analysis/java.rs` 选择并固定 Java 字节码/ArchUnit 适配配置，发布真实支持矩阵（Requirement: Versioned Language Fact Providers）；`fixtures/languages/java/` + `tests/language_matrix.rs` 验证合法/禁止类型依赖与反射盲区，缺字节码/不支持版本明确缺覆盖，不声称运行时完整性。
- [x] 2.3 在 `src/analysis/typescript.rs` 选择并固定 TypeScript 语义提取，明确 alias/project-reference 配置（Requirement: Versioned Language Fact Providers）；`fixtures/languages/typescript/` + `tests/language_matrix.rs` 验证别名、跨包类型与动态导入，禁止边有来源，无法解析范围不变成零边成功。
- [ ] 2.4 在 `src/analysis/rust_symbols.rs` 选择并固定 Rust 索引接口与 cfg/feature/target profile（Requirement: Versioned Language Fact Providers）；`fixtures/languages/rust/` + `tests/language_matrix.rs` 覆盖模块/类型/方法、宏和动态分派，支持关系正确归因，缺失配置/索引明确拒绝完整声明。
- [x] 2.5 在 `src/domain/rules.rs` 实现受保护分层、禁止方向与依赖环检查和中立投影边界（Requirement: Deterministic System Architecture Rules）；`tests/system_rules.rs` 为每条 enforce 提供合法、违规、缺覆盖案例，环结果列出实际路径/来源，不能表达于旧 engine 的规则明确协商能力或 unsupported。

## 3. Domain Contracts, Baseline and Design Diff — trace 阶段才依赖 SG-BASELINE

**Surfaces:** 拟新增 `src/domain/{baseline,contracts,diff}.rs`、`src/analysis/codegraph.rs`、`tests/{baseline,domain_contracts,design_diff}.rs`。
**Interfaces:** authenticated external baseline reference → ArchitectureBaseline；baseline + candidate observation → contract findings、TestObligation refs、DesignDiff。基线 authority 为外部输入，不由本域创建审批。

- [x] 3.1 在 `src/domain/baseline.rs` 实现不可变 source/ADR/contract 摘要及 draft/in_review/approved/superseded/revoked 状态（Requirement: Immutable Architecture Baseline）；`tests/baseline.rs` 验证候选改写 accepted/approved=true 不覆盖旧 baseline、修订产生新 digest，批准删除义务也必须有新外部认证记录。
- [ ] 3.2 在 `src/domain/contracts.rs` 定义 Context/Aggregate/Invariant/Transition 与测试义务映射（Requirement: Approved Domain Object and Method Contracts）；`tests/domain_contracts.rs` 验证合法 AppService 协调与绕过状态入口，只有批准确定规则可阻断，输出义务不声称已执行 TestGuard 测试。
- [ ] 3.3 在 `src/domain/contracts.rs` 加入 Object API/可见性/依赖及 Method signature/side-effect 契约（Requirement: Approved Domain Object and Method Contracts）；`tests/domain_contracts.rs` 验证所有权迁移、纯度/IO 和错误传播的合法/违规/缺覆盖三类，启发式长度/名称建议只 review/advise。
- [x] 3.4 在 `src/domain/baseline.rs` 和拟新增 `src/integration/trace.rs` 消费 SG-BASELINE 的 Requirement/ADR/TestObligation 稳定引用（Requirement: Immutable Architecture Baseline）；`tests/baseline.rs` 验证失效/缺失/跨 requirement 引用被拒绝且 basic Cargo scan 不依赖 SpecGuard 可用性；该互操作部分等待 SG-BASELINE 和 GE-CONTRACT/ADAPTER。
- [ ] 3.5 在 `src/domain/diff.rs` 与 `src/analysis/codegraph.rs` 实现 baseline/candidate API/所有权/方法差异及显式未知消费者（Requirement: Source Bound Design Diff）；`tests/design_diff.rs` 覆盖重命名、旧索引、动态分派和跨任务共享 API，断言来源/缺口可追溯；先验证外部索引格式，不假定插件存在。

## 4. Candidate-Bound Evidence — 互操作等待 GE-CONTRACT 和 GE-ADAPTER

**Surfaces:** 拟新增 `src/integration/{projection,binding,evidence,cli}.rs`、`tests/{integration_schema,execution_state,candidate_binding}.rs`、`fixtures/integration/`。
**Interfaces:** FrozenBinding + ProducerProfile + FrozenCoverage + domain outcome → standalone envelope + allowed-storage artifact refs；engine-backed completed decision 等于报告。尚未绑定则只产生 transport diagnostic。

- [x] 4.1 在 `src/integration/projection.rs` 按 GE-CONTRACT golden 实现独立 strict envelope 与领域 artifact 映射（Requirement: Separate Versioned Architecture Evidence）；`tests/integration_schema.rs` 检查未知版本/字段、报告摘要或 decision 不同均拒绝，当前 GuardFacts/GuardReport schema 原样保留，无隐含 N/N-1。
- [x] 4.2 在 `src/integration/evidence.rs` 实现 pre-binding diagnostic 与绑定后 completed/error/cancelled（Requirement: Bound Execution State and Diagnostics）；`tests/execution_state.rs` 验证缺 OID 无 envelope、工具失败/取消 null decision、有效 partial 为 BLOCK、旧输出不可冒充当前结果，并回归 legacy metadata partial 行为。
- [ ] 4.3 在 `src/integration/binding.rs` 定义全量 immutable binding、attempt runId、dedup key 与 compare-and-set 发布（Requirement: Immutable Candidate Isolation and Invalidation）；`tests/candidate_binding.rs` 模拟两个 requirement、重复尝试、迟到旧完成，断言 runId 独立、相同提取可缓存但不能跨义务/候选满足。
- [x] 4.4 在 `src/integration/binding.rs` 消费 GitGuard 的只读候选解析并绑定实际对象格式（Requirement: Immutable Candidate Isolation and Invalidation）；`tests/candidate_binding.rs` 覆盖 synthetic queue candidate、base/成员变化和不同对象格式，旧 head/旧 queue 报告均不能满足新候选；不新增 Git 写权限。
- [ ] 4.5 在 `src/integration/evidence.rs` 发布 AG-EVIDENCE artifact 集合和 explicit capability profile（Requirement: Separate Versioned Architecture Evidence）；`tests/integration_schema.rs` 用 GitGuard/FlowGuard 消费 fixture 验证 missing coverage/artifact、未支持 domain capability 不降级通过，收录真实 contract/facts/report/domain 摘要引用。

## 5. Trusted Operation, Interfaces and Release — GE-TRUST / GE-RELEASE 按阶段进入

**Surfaces:** 拟新增 `src/integration/{trust,audit,mcp}.rs`、`tests/{trust,interface_parity,end_to_end}.rs`，未来更新 `.github/workflows/ci.yml`、`Cargo.toml` 与发布文档。
**Interfaces:** external authenticated evidence/approval ports → derived eligibility + append-only audit；read-only CLI/library/MCP 共用分析服务。无签发审批/签名 key/ref-write API。

- [ ] 5.1 在 `src/integration/trust.rs` 对接 GE-TRUST ports，验证 producer、批准基线/例外范围、期限/撤销及允许 evidence URI（Requirement: Authenticated Architecture Evidence Audit）；`tests/trust.rs` 验证伪造、服务不可用、过期/撤销、策略变弱和自报 signed 字段均不授予资格，REQUIRE_APPROVAL 及 partial 原报告不变。
- [ ] 5.2 在 `src/integration/audit.rs` 及受保护 CI 配置实现脱敏审计、隔离凭据/策略和 controller attestation 引用（Requirement: Authenticated Architecture Evidence Audit）；`tests/trust.rs` 检查真实候选/规则摘要和失效因果可追溯、分析进程没有签名/merge credential；先决定存储保留期/provider，不把本地 verify 当认证；controller attestation 引用不是签名实现，旧 signed-attestation TODO 只有后续独立签名 change 的真实签发/验签证据齐备才可关闭。
- [ ] 5.3 在 `src/integration/cli.rs`、`src/integration/mcp.rs` 共享只读 doctor/scan/explain/diff/impact 的明确 capability contract（Requirement: Read Only Architecture Interfaces）；`tests/interface_parity.rs` 比较同输入输出/错误/预算语义并拒绝 self-approval/ref-write，先冻结 flags 与 MCP 库后才发布可运行命令；OPA 仅决策可选适配，不执行不可信脚本。
- [ ] 5.4 在 `tests/end_to_end.rs` 与 `fixtures/integration/queue/` 验证 AG-EVIDENCE 联合流程（Requirements: Immutable Candidate Isolation and Invalidation; Phased Architecture Release and Rollback）；相关 SG/AG/CG/TG/GG/GE-TRUST gates 后测试双 requirement、精确队列候选、native CodeGuard parity、覆盖缺失、审批到期/撤销、迟到完成与关闭适配器，所有错误保持门禁未满足。
- [ ] 5.5 在 GE-RELEASE 后更新 `Cargo.toml`/CI 为独立固定 GuardEngine 发行并发布兼容矩阵、迁移与回滚记录（Requirement: Phased Architecture Release and Rollback）；`tests/cli_compatibility.rs` + shadow fixture 逐阶段验证 advisory→shadow→opt-in protected→enforce，回退保留 legacy 与旧证据；关闭新 profile 不取消必查义务，仅凭固定源码开发不能宣称生产发布完成。

## Dependency and Acceptance Handoff

[GuardEngine](https://github.com/full-stack-plugins/guardengine/tree/docs/guard-design-20261009/openspec/changes/add-versioned-guard-integration-contracts)：GE-CONTRACT/ADAPTER 控制组 4 与 3.4 的互操作；GE-TRUST 控制 5.1–5.2 受信消费；GE-RELEASE 控制 5.5 独立发行。[SpecGuard](https://github.com/full-stack-plugins/specguard/tree/docs/guard-design-20261009/openspec/changes/add-specification-baseline-analysis) SG-BASELINE 仅阻塞 3.4 trace。AG-EVIDENCE 的基本硬化先组 1，之后按公布 capability 阶段交付，不把全部语言完成作为 Cargo evidence 的先决条件。

执行者为每个任务先建立失败 fixture，再实现最小模块并记录实际通过结果；可用后运行 `cargo test --all-targets`、`cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`，新增外部语言测试记录工具版本/配置。独立 strict OpenSpec validation 只验证规划格式，不完成运行时任务。上述所有待选依赖与 provider 决策必须在对应阶段形成受审 ADR；未解决的能力不列入已发布完整 scope。
