# ArchGuard Skills-first Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 先交付 java-skills/archunit 及 Maven、Gradle 真实三态验证，再接入实际 ArchGuard-Plugin；核心独立发行单独验收。

**Architecture:** 技能指导目标项目配置原生架构测试，插件发现并调用技能、解释差异和结果。Rust 核心保留分析、契约和不可变基线，Git 候选与规格追溯移至可选集成包；两条交付线互不阻塞。

**Tech Stack:** Agent Skills Markdown、Java、ArchUnit、JUnit 5、Maven/Gradle；独立核心为 Rust 1.90（edition 2024）与 GuardEngine。

**Spec:** [完整设计](../specs/2026-10-09-architecture-contract-and-checker-adapters-design.md)，关联 [proposal](../../../openspec/changes/decouple-core-and-add-archunit-profile/proposal.md) / [design](../../../openspec/changes/decouple-core-and-add-archunit-profile/design.md)。本计划承接已审阅方向；旧文档“本轮仅设计”描述的是此前交付。

## Global Constraints

- 本次只写计划与本地文档提交；不修改 java-skills、产品代码或 CodeGuard，不安装、不推送。以下复选项全部是未来执行任务，不是已完成结果。
- 技能不是常驻执行器；不新增运行时、数据库、控制器或统一检查器平台；Java 接入无需 ArchGuard 二进制。
- 沿用项目构建工具；原生退出码、JUnit XML 与日志为证据，不新增 0/2/4 协议；失败、空测试/导入、覆盖不足均不得称为通过。
- 技能目录与 frontmatter name 一致、小写 kebab-case；description 单行 20–1024 字符；遵循 Apache-2.0 与第三方 notices 惯例。
- 固定依赖版本；记录技能 commit 与项目/规则/配置修订，例外须有原因、责任人、范围、审批和到期；不得自动冻结现有违规。
- Java 17/JUnit 5 只是初始验证候选；未经两种构建实测，不发布兼容声明。目录扫描不冒充 Git 候选证据。

## Review Focus

- 缺工具或编译失败应为未验证，不能冒充架构违规（任务 2、3）。
- 零测试、空导入、遗漏任一层应拒绝通过（任务 2、3）。
- 违规样例必须仍可编译，修复不能变更规则、版本或排除范围（任务 2、3）。
- 多构建/多模块项目或未安装技能不能猜测调用入口、覆盖与成功状态（任务 4）。
- 没有 sibling 仓库也能分发构建；可选集成拒绝伪造 OID/字节（任务 5）。

## 已核实边界与依赖

2026-10-09 只读核验：ArchGuard `139d1f3`、分支 `impl/guard-roadmap-20261009`；java-skills 位于 `/workspace/java-skills-design-audit-20261009`，pin `d981268c4baaa32e4e068a47b95db13e5a1c526e`，无 AGENTS.md。现有注册为 `.claude-plugin/plugin.json` 的 `skills` 数组，lint 为 `python3 scripts/lint_skills.py`；无 archunit 技能。

当前 ArchGuard 是 Rust crate：`Cargo.toml`、`src/main.rs`、`src/lib.rs`；工作区已检索的插件 manifest 中未找到 ArchGuard-Plugin 宿主（发现的 CodeGuard/旧 FlowGuard 插件不是替代目标）。故任务 4 先定位真实宿主及契约，再补准确文件/API/原生测试命令；此缺口仅阻塞插件代码，不能在 Rust 仓库凭空新建宿主入口。

本环境默认 `java -version` 为 OpenJDK 21.0.12.1，默认 PATH 未找到 mvn/gradle；`/workspace/guard-toolchain` 已有 JDK 21、Maven、Gradle，可按其 `native-env.sh` 配置复用。默认 PATH 状态不是外部阻塞；未来在任务授权内创建可写隔离 checkout、使用或准备所需工具并记录来源，本轮仅因 plan-only 不执行。未执行 Java 功能测试或技能 lint。官方 [ArchUnit 指南](https://www.archunit.org/userguide/html/000_Index.html) 确认原生 Maven/Gradle test 依赖及 JUnit 5 接入；[JUnit 5.12.2 指南](https://docs.junit.org/5.12.2/user-guide/) 作为候选版本资料。文档查询不等于兼容实测，Gradle 版本文档本次抓取失败；具体 ArchUnit/JUnit/构建插件/wrapper/JDK 组合必须在任务 1 固定并经任务 2、3 验证。

依赖：`1 → 2 → 3` 形成可独立交付的 Java 技能；`1 → 4（定位），2+3+宿主定位 → 4（接入）`；任务 5 独立执行，不阻塞任务 1–4，也不等待其他守卫实现流。

### Task 1: 技能契约、版本与注册

**Files（未来可写的 java-skills checkout，相对仓库根；当前只读副本不改）:** 新建 `skills/archunit/SKILL.md`、`references/{maven,gradle,architecture-rules,troubleshooting,compatibility}.md`（均位于 `skills/archunit/`）；修改 `.claude-plugin/plugin.json`、`README.md`、`README.zh-CN.md`；涉及第三方复制时修改 `THIRD-PARTY-NOTICES.md`。

**Interfaces:** 输入构建文件、wrapper、JDK/框架、包结构及确认的架构边界；输出目标项目依赖差异、`example.order.ArchitectureRulesTest`、规则契约及证据。`ARCH-JAVA-001` 固定为禁止 `example.order.web.. → example.order.infrastructure..`；契约记录版本、作用域、允许/禁止依赖与排除，不引入中央服务。

- [ ] 先记录技能/注册/双语索引缺失这一基线；在未来工作 checkout 运行现有 lint，区分旧错误与新增错误。
- [ ] 在 `compatibility.md` 记录候选的完整精确版本：JDK 发行版/版本、ArchUnit、JUnit BOM、Maven、Surefire/Compiler 插件、Gradle 与 wrapper；用对应官方版本文档核验运行 JDK/字节码与测试发现要求，保留来源。版本选择完成才进入任务 2；优先复用 `/workspace/guard-toolchain`，按任务授权准备所需工具并记录来源，无法获得时再报告具体阻塞。
- [ ] 编写技能及 references：包、分层、循环、命名和自定义规则；职责/业务语义标为需评审。明确用户授权内展示差异和运行，不隐式升级、改规则或新增豁免；到期或规则/配置变化使例外待重审，新增违规仍失败。
- [ ] 注册 `./skills/archunit` 并更新双语索引；运行 `python3 scripts/lint_skills.py`，确认新增技能无错误、引用路径存在、JSON 可解析；仅该任务变更提交 `docs: add archunit skill and native build guidance`。

### Task 2: Maven 真实闭环与防假通过

**Files:** 新建 `skills/archunit/examples/maven-order-service/{pom.xml,README.md}`、固定 wrapper 的 `mvnw`、`mvnw.cmd`、`.mvn/wrapper/maven-wrapper.properties`（及所选 wrapper 分发要求的文件）；新建该目录下 `src/main/java/example/order/{web/OrderController,application/OrderService,domain/Order,infrastructure/JdbcOrderRepository}.java`、`src/test/java/example/order/ArchitectureRulesTest.java`、`architecture-contract.md`。证据保存到 `skills/archunit/examples/verification/maven/`，不提交构建产物。

**Interfaces:** 使用任务 1 固定版本与规则。四个生产类均可独立编译；web→application、application→domain、infrastructure→domain。测试包含 `importCoverage`（导入四个预期生产类、排除测试类）及 `webMustNotDependOnInfrastructure`（失败文本含 ARCH-JAVA-001）；domain 纯净规则同时覆盖其他层与框架依赖。

- [ ] 先写上述测试与未完成的示例，运行 `./mvnw -B -Dtest=ArchitectureRulesTest test`，记录失败原因；配置固定 test 依赖、JUnit Platform 发现、Java release、Surefire 零测试失败。
- [ ] 完成正常分层，执行同一命令，要求退出 0、目标测试实际执行、无失败/错误/跳过，覆盖四个生产类；保存当前原始 `target/surefire-reports/*.xml` 和日志，标为 pass。
- [ ] 只向 OrderController 添加使用 JdbcOrderRepository 的真实类型依赖；执行 `./mvnw -B -DskipTests compile` 必须成功，再执行原测试命令必须非零且 XML 指向 ARCH-JAVA-001，保存 violation 证据。
- [ ] 只撤回该越层依赖，同一测试命令恢复 pass；记录三阶段源码 diff/摘要并核对规则、版本、配置摘要相同。
- [ ] 在独立临时副本依次制造缺 wrapper/工具、语法编译错误、零测试、空导入、漏导 infrastructure；每次先清理旧报告，用实际命令验证不能判通过。缺工具/编译错误/零测试标未验证；导入覆盖失败也不能作完整架构结论，不以旧 XML 补证。
- [ ] README/证据记录命令、退出状态、测试数、导入覆盖、源码及规则/配置摘要、工具版本和 XML 路径；lint 通过后提交 `feat: add verified Maven architecture example`。工具缺失则本任务未完成，不拿文档代替实测。

### Task 3: Gradle 独立闭环与兼容矩阵

**Files:** 新建 `skills/archunit/examples/gradle-order-service/{build.gradle,settings.gradle,README.md,gradlew,gradlew.bat,gradle/wrapper/gradle-wrapper.properties,gradle/wrapper/gradle-wrapper.jar}`；源文件与契约路径同任务 2 的 `src/`、`architecture-contract.md`；证据为 `skills/archunit/examples/verification/gradle/`；更新 `references/compatibility.md`。

**Interfaces:** 复用同一规则语义、预期类集合和版本记录结构；每个示例只有自己的构建系统。Gradle `useJUnitPlatform()`，显式固定 launcher 等测试依赖、拒绝零测试与忽略失败。

- [ ] 先用同一测试类验证缺失配置失败，再完成固定版本 Gradle 配置；正常运行 `./gradlew clean test --tests 'example.order.ArchitectureRulesTest' --rerun-tasks --no-build-cache`，核对实际执行数和新 XML，不接受 UP-TO-DATE/NO-SOURCE 作为测试证据。
- [ ] 与任务 2 相同的 OrderController→JdbcOrderRepository 修改后，`./gradlew clean compileJava` 成功，前述测试命令非零且报告 ARCH-JAVA-001；只修复依赖后原命令恢复 pass，规则/版本不变。
- [ ] 在临时副本重做缺工具、编译失败、零测试、空导入和遗漏层五例；保存 `build/test-results/test/*.xml` 与日志，判定要求同任务 2。
- [ ] 把两种构建各三阶段及负例真实结果写入 compatibility；只有实际跑过的组合标 verified，其他组合写未验证。运行技能 lint、检查注册/双语链接后提交 `feat: verify Gradle architecture example and compatibility matrix`。

### Task 4: ArchGuard-Plugin 发现与调用（宿主定位门）

**Files:** 先在本仓库新增 `docs/integration/archguard-plugin-skill-discovery.md` 记录宿主调查；实际宿主文件尚未找到，禁止虚构代码路径/API。宿主定位后在该文档补 repository、commit、manifest、技能发现/调用文件及实际测试文件/命令，再修改那些经核实的文件。

**Interfaces:** 消费任务 1–3 的 `archunit` 标识、技能修订、项目构建与包信息；产出选中技能、目标差异、原生命令及通过/违规/未验证说明。沿用宿主真实技能 API，不另设运行协议。

- [ ] 查明实际宿主来源与 checkout、读取其说明及权限边界，记录如何发现 `java-skills/archunit`；若仍无来源，只完成调查文档，插件实现标阻塞，Java 交付继续。
- [ ] 定位后先写宿主原生失败测试：Java+Maven、Java+Gradle 各选 archunit；技能缺失给出具体缺口；多个构建/模块要求明确作用域；非 Java 不误选；未授权修改不写文件；空测试/覆盖不足不报通过。
- [ ] 最小接入现有技能发现/调用机制，向技能传递实际项目上下文并展示 diff、原生运行结果；运行同一组测试直到通过。
- [ ] 用任务 2、3 已完成样例做真实宿主发现→调用→原生执行演练，保存技能 commit、调用证据和报告；宿主修改独立提交，不以 mock 测试宣称端到端完成。

### Task 5: 独立核心与可选集成（独立验收线）

**Files（ArchGuard）:** 修改 `Cargo.toml`、`src/lib.rs`、`src/domain/{mod.rs,baseline.rs}`；纯生命周期保留 `src/domain/baseline/lifecycle.rs`。新建 `integrations/git-spec/{Cargo.toml,src/lib.rs}`；把 `src/integration/binding.rs`、`binding/attempts.rs`、`trace.rs` 及 `src/domain/baseline.rs` 中 GG/SG 追溯部分移入该包。中立 `src/integration/{projection,evidence,cli}.rs` 留核心。相关候选/追溯测试和 examples 移至集成包；新增 `tests/independent_distribution.rs`，修改 `tests/{domain_contracts,design_diff}.rs` 与发行说明 `docs/independent-distribution.md`。

**Interfaces:** 核心保持现有分析与 `ArchitectureBaseline` 生命周期语义；可选包单向依赖 archguard、固定 GE/GG/SG，对外候选入口仍要求真实 `Repository`/`CandidateSnapshot`，保留字节清单核验。不把集成变成默认 feature/path 依赖；不能只关 feature 而保留分发时必须解析的外部 sibling。

- [ ] 先添加失败的独立分发检查：在新临时目录解包，仅用包内源码/固定依赖执行 `cargo build --locked`、`cargo test --locked`、`cargo tree`；无任何仓库外 sibling、核心图无 GG/SG。检查不得靠开发工作区缓存路径或临时软链通过。
- [ ] 按上列文件拆分，保留核心生命周期；追溯类型移至可选包，必要时仅开放中立数据访问接口，禁止向核心反向引入 GG/SG。先记录迁移后 API/调用方清单，更新包内测试与 examples。
- [ ] 保留并运行 Cargo/Java/TS 分析及基线回归：现有 `analysis_profile`、`java_bytecode`、`typescript_semantic`、`domain_contracts`、`design_diff`；在无 `.git` 的 allowed/denied 项目运行现有 CLI，核对与拆分前结果一致。外部分析工具缺失应明确未验证，不跳过后声称全通过。
- [ ] 集成包运行迁移后的 `candidate_binding`、`baseline`、`attempt_concurrency` 及相关 trace 测试；增加/保留伪造 OID、相同 OID 不同字节、错误清单/追溯源拒绝案例，普通目录不得获得候选认证。
- [ ] 固定 GE 可分发来源；如尚未发行，交付包含固定源码、lockfile、许可证及 notices 的自包含构建包，在隔离目录重跑第一步，明确仅源码包而非已发布 crate。可选集成另测，不进入默认发行依赖图；结果与打包清单随独立提交交付。

## 完成条件与交付边界

Java 完成 = 注册/双语索引/lint + Maven、Gradle 各自真实 pass→可编译违规→修复 pass + 防假通过负例 + 完整版本/报告证据。插件完成另需真实宿主发现/调用验证；宿主未知不能称完成。核心完成另需独立分发、旧能力回归和可选集成防伪证据；其阻塞不撤销 Java 已完成交付。

本计划没有声称执行上述测试或验证任何兼容组合。待落实前置为实际宿主仓库与调用契约、GE 可分发来源及尚未经实际验证的版本矩阵；各自在所属任务闭合。可写隔离 checkout 与既有工具链复用是后续已授权工程步骤，不是外部未定点；不扩张到其他语言或其他守卫产品实现。
