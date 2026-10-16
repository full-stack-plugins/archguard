---
name: archguard-domain-modeling
description: 澄清领域术语、上下文与架构边界，并关联已有规格；不替代完整 DDD 设计或自动生成强制规则。
license: MIT
compatibility: ArchGuard CLI 0.5.0 或对应 MCP；支持 Maven/Cargo，纯分析可仅读取源码。
---

# 领域建模与约束来源

改编自 Matt Pocock domain-modeling；原始来源与许可位于 `org.partme.guard/sources/`。

先读 [公共协议](references/guard-workflow.md)，用 spec context 选择已有规格。需要交互澄清时读取本包 [guard-grilling](../guard-grilling/SKILL.md)；没有 Skill API 时直接读取该文件，不调用同名全局技能。

1. 读取项目统一模型基线（model show）、已有词汇表、领域上下文映射、ADR 和实际代码；同一个词在不同上下文的含义必须分清。按需求维护稳定 ID 的增量，不创建独立完整架构。
2. 从用户目标中提取实体、行为和业务边界，检验歧义和反例。术语与代码不一致时列出证据，不自行改业务含义。
3. 在授权范围内更新已有词汇表；只保存术语定义，不把实现计划塞进词汇表。跨上下文保持映射，不强制全局同名等义。
4. 仅对难以逆转、容易令人意外并存在真实权衡的决定写 ADR，引用原生需求标识；轻量决定留在现有规格。
5. 将领域边界映射到实际 Maven 坐标或 Cargo 包名；列出允许/禁止的依赖方向、例外、负责模块及未覆盖的源码行为。
6. 经项目决策确认后，把适用约束交给 architecture-check，补充规则与原生需求的 spec-binding 引用。增量绑定当前基线和原生需求，运行 change impact/check；复验后按授权更新基线。禁止把未经确认的推断写成 enforce。

交付：统一术语、模块映射、决策依据和未决问题。术语一致不等于架构检查通过。

Java/Maven 源码治理使用公共协议中的 java 策略和 java_type 映射；复用项目基线及需求增量，以 java_analysis 的源证据检查包、层和归属。修复后重新 check/verify，不把构建子报告 ALLOW 或模型声明当作完整 Java 语义证明。
