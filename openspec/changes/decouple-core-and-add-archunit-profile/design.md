# Design: 技能优先与独立架构核心

状态：待书面审阅；对应 [proposal](proposal.md)。详细内容以 [完整短设计](../../../docs/superpowers/specs/2026-10-09-architecture-contract-and-checker-adapters-design.md) 为准。

## Decisions

1. ArchGuard-Plugin 负责项目识别及选择调用语言技能；Java 集成知识归 full-stack-skills/java-skills 的 skills/archunit。技能指导项目使用原生构建与测试，不新增专用运行时或数据库。
2. Java 技能包括 SKILL.md、Maven/Gradle/规则/排错 references、两套 order-service 示例，遵循现有 lint、插件注册和双语 README 规范。
3. 规则使用稳定 ruleId，目标项目保存版本、作用域和架构测试。原生退出码、JUnit 报告与日志为证据；不新增旧稿提出的 0/2/4 插件退出码协议。
4. 首批各自验证 Maven 与 Gradle 的通过→可编译越层依赖违规→修复通过；缺工具、编译失败、空测试/导入不能假通过。版本兼容矩阵需实测，当前不宣称已验证。
5. 核心保留既有分析和纯架构基线，GG/SG 候选及追溯进入可选包；独立分发不依赖任何仓库外 sibling。GE 采用固定可分发依赖，或明确标注的自包含源码构建包。
6. 可选集成保留真实候选字节验证，普通目录不冒充 Git 候选。Java 技能不以核心解耦完成为前提。例外需明确范围、责任、审批和到期信息，不自动弱化规则。

## Acceptance and Limits

先交付 Java 技能、注册和双语索引及两套三态验证，再独立验收核心发行、旧能力回归与候选防伪。其他语言先只读核实实际仓库、规范与工具后分批设计。不修改 CodeGuard，不创建全语言执行器平台，不删旧能力。

本轮仅更新既有文档；java-skills 保持只读，未安装或实现技能、未创建实施 tasks、未宣称 OpenSpec 或功能测试通过。
