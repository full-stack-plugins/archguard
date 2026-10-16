---
name: archguard-improve-codebase-architecture
description: 从现有代码找出架构改进候选，经决策转为可执行约束并复验；不自动大规模重构或改变已确认规则。
license: MIT
disable-model-invocation: true
compatibility: ArchGuard 0.5.0 CLI/MCP；项目模型与 Maven/Cargo 声明图，源码语义需人工评审。
---

# 架构改进到检查闭环

改编自 Matt Pocock improve-codebase-architecture；原始来源与许可位于 `org.partme.guard/sources/`。
仅在用户调用或明确授权架构改进活动后进入；自动路由只能推荐。不得因发现热点擅自启动重构。

先读 [公共协议](references/guard-workflow.md)、本包 [archguard-codebase-design](../archguard-codebase-design/SKILL.md)。需要澄清时使用本包 [archguard-grill-with-docs](../archguard-grill-with-docs/SKILL.md)，需要核实外部事实时使用本包 [archguard-research](../archguard-research/SKILL.md)。没有专用 Skill API 时读取上述路径。

1. 从用户指定区域、变更热点和测试困难定位候选；已有 CodeGraph 索引优先使用，不自行索引。
2. 运行现有架构检查，区分真实违规、模型覆盖不足和设计建议。阅读边链的源位置，不把声明图解释成 Java 调用图。
3. 给每个候选提供问题、代码依据、改造前后 Mermaid 图、收益、风险和信心（明确/待验证/推测）。图和报告放项目允许目录；不依赖外部 CDN 或系统默认临时盘。
4. 候选选择复用已有用户授权；关键取舍未确认才澄清。尊重 ADR，冲突须明确记录，不能静默覆盖。
5. 将确认的依赖方向转为项目契约及原生需求绑定；对无法自动检查的意图保留人工验收，不写虚假的机器规则。
6. 先构造违反目标约束的可观察案例，确认 BLOCK 与见证路径，再在授权范围内修复源码/声明。保持已确认的契约与场景不变。
7. 执行相关测试；已有项目基线时针对需求增量运行 change impact/check/verify，审阅并在授权范围内 baseline update --apply，保留历史。无模型项目沿用 check/verify。证明旧报告在修改后失效，Cargo 项目走相同流程。

交付：确认的改进、对应规则、修复差异、测试结果、当前报告及未覆盖项；本技能不自动提交、推送或发布。

Java/Maven 源码治理使用公共协议中的 java 策略和 java_type 映射；复用项目基线及需求增量，以 java_analysis 的源证据检查包、层和归属。修复后重新 check/verify，不把构建子报告 ALLOW 或模型声明当作完整 Java 语义证明。
