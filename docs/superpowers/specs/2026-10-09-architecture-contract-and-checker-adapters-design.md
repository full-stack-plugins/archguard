# ArchGuard：技能优先的架构检查设计

状态：2026-10-09 修订，待书面审阅；本轮只更新设计，不实施技能、产品代码或依赖，不推送。

## 1. 定位与职责

ArchGuard 帮助项目定义语言、框架、MVC/DDD 等模式、模块职责、目录、命名和依赖边界，并检查可自动验证的约束。职责描述与业务语义仍需评审，不能把静态依赖通过等同于完整架构正确。

采用技能优先的分工：

```text
ArchGuard-Plugin → 选择并调用 full-stack-skills 中的语言架构技能
                → 技能指导目标项目配置原生测试 → Maven/Gradle/CI 运行
ArchGuard 核心  → 保留已有分析、架构契约与基线能力
可选 Git 集成 → 候选字节绑定及规格追溯
```

插件负责识别项目、选择技能、说明修改及结果；接入知识、规则示例、排错和版本兼容说明归语言技能仓库。技能不是常驻执行器。Java 接入无需先调用 ArchGuard 二进制，不另建运行时、数据库或控制器，不为每个检查器包装专用执行平台。不改 CodeGuard。

## 2. 已核验事实

ArchGuard 源码基线为 `001d40c4243a920c49834da4139f51f79845eeff`：已有 Cargo、Java 字节码、TS 分析及架构基线；ArchUnit 集成尚未完成。GitGuard/SpecGuard 仍是必选路径依赖，核心独立发行尚未实现。

只读核验 [java-skills](https://github.com/full-stack-skills/java-skills/tree/d981268c4baaa32e4e068a47b95db13e5a1c526e)：未发现 AGENTS.md，存在 `skills/<name>/SKILL.md`、插件注册、双语 README、技能 lint 和 OpenSpec 技能约定，当前无 ArchUnit 技能。lint 要求目录与 frontmatter name 一致、名称小写 kebab-case、description 为单行 20–1024 字符。以下均为目标交付，不能称为现有能力。

## 3. Java 首批交付

在 `full-stack-skills/java-skills` 新增：

```text
skills/archunit/
  SKILL.md
  references/maven.md
  references/gradle.md
  references/architecture-rules.md
  references/troubleshooting.md
  examples/maven-order-service/
  examples/gradle-order-service/
```

SKILL.md 说明触发场景、输入、适用边界、接入流程、结果与证据；遵循仓库 frontmatter 和许可惯例。同步注册 `.claude-plugin/plugin.json`、中英文 README 索引；如引入第三方材料，核对许可证与 notices。示例属于技能仓库，实际架构测试保存在目标项目。

内容覆盖 [TNG/ArchUnit](https://github.com/TNG/ArchUnit) 的 Maven test 依赖、Gradle testImplementation、JUnit 5 测试发现、类导入范围、包/分层/循环/命名规则，以及项目自定义规则。沿用目标项目构建方式；不强制 Maven 或引入双构建系统。

## 4. 项目接入与规则契约

输入是项目构建文件、JDK/框架版本、源码及包结构、团队确认的架构边界；输出是可审阅的依赖配置、ArchitectureRulesTest、规则说明和运行证据。规则使用稳定 ruleId，记录作用域、允许/禁止依赖、排除项及契约版本；项目文件承载配置，不新增中央配置服务。

示例采用 order-service 的 web、application、domain、infrastructure 分层：web→application、application→domain、infrastructure→domain；domain 不依赖其余层或框架。首条 ARCH-JAVA-001 禁止 web→infrastructure。规则显式写入 ArchUnit 测试，并确认导入覆盖；无对应自动规则的职责描述标为需评审。

接入时展示目标项目差异，按用户授权修改和运行；检查不隐式安装、升级或改写规则。存量违规只有经确认的具体条目才能成为例外，记录原因、责任人、范围、审批及到期时间；过期或规则/配置变化需重审，新增违规继续失败。既有不可变架构基线与违规豁免分别处理。

## 5. 执行、版本与证据

使用目标项目原生命令，以下是待验证的示例入口：

- Maven：`./mvnw -B -Dtest=ArchitectureRulesTest test`。
- Gradle：`./gradlew test --tests 'example.order.ArchitectureRulesTest'`，配置 JUnit Platform。

保留 Maven/Gradle 原生退出码和测试报告，不新增插件专用退出码协议。结果说明区分通过、规则违规、未验证：构建失败、缺工具、空测试/类导入、覆盖不足或不支持必需规则都不能称为通过。CI 必須检查测试确已执行，不启用忽略失败；优先保留原生 JUnit XML 与日志，不假设工具提供统一 JSON。

分别记录技能仓库版本/commit 与目标项目的 ArchUnit、JUnit、构建插件、wrapper 和 JDK 版本。初始 Java 17/JUnit 5 组合是验证候选；通过 Maven、Gradle 两条实际验证后才公布兼容矩阵。禁用动态依赖版本，升级需重跑正反例。证据记录输入修订或摘要、规则与配置版本、命令、退出状态、测试数量、类导入覆盖和原始报告；目录扫描不冒充 Git 候选证据。

## 6. 独立核心与可选集成

保留 ArchGuard 现有分析，不重写为空壳。默认核心依赖图移除 GitGuard/SpecGuard，其候选提取和追溯归可选集成包；纯架构基线留核心。集成包保留真实候选对象及内容清单验证，自报 OID 不能成为已验证来源。

GuardEngine 只提供中立规则、契约、证据能力，语言策略留守卫或技能。独立发行必须从无外部 sibling 仓库的干净分发物构建，并运行无 .git 扫描；GE 使用固定可分发依赖，尚不可发行时可采用含固定源码与许可的自包含构建包，但不能声称独立 crate 已发布。此解耦可独立验收，不阻塞 Java 技能原生测试。

## 7. 分阶段验收

第一阶段交付 Java 技能、双语索引和两套示例，各自完成相同三态验证：正常分层通过；添加仍可编译的 OrderController→JdbcOrderRepository 依赖，使 ARCH-JAVA-001 真实失败；只修复越层依赖，在同版本、同规则下恢复通过。

同时验证缺工具、编译失败、空测试和空导入均不得误报通过；保存命令与原始报告，并运行仓库技能 lint。不能用生成配置或文档检查代替实际测试。

第二阶段验收独立核心分发、旧分析/基线回归，以及可选候选集成的字节一致性与伪造来源拒绝。其他语言按 full-stack-skills 实际仓库、规范和成熟工具逐批调查后设计；不预设仓库名或承诺统一检查器。Rust 保留已有 Cargo 声明分析边界。

本轮仅修订本设计及关联 OpenSpec proposal/design；java-skills 保持只读，未创建实施任务、运行功能测试或宣称 OpenSpec 验证通过。书面设计审阅后再制定实施计划。
