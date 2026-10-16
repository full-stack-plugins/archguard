---
name: archguard-grill-with-docs
description: 对架构需求开展带术语和文档落点的澄清，复用已有规格；不初始化新 SDD 体系或创建重复需求。
license: MIT
disable-model-invocation: true
compatibility: 可读取包内依赖文件的智能体；ArchGuard 0.5.0 提供项目模型和原生规格上下文。
---

# 有文档落点的架构澄清

改编自 Matt Pocock grill-with-docs；原始来源与许可位于 `org.partme.guard/sources/`。

此技能是组合入口，必须先读取三项内容，不能只读入口后声称已完成访谈：
仅在用户调用或明确授权此澄清活动后进入；自动路由只推荐入口。宿主不支持该元数据时仍遵守此正文限制。

- [公共协议](references/guard-workflow.md)
- 本包 [guard-grilling](../guard-grilling/SKILL.md)
- 本包 [archguard-domain-modeling](../archguard-domain-modeling/SKILL.md)

1. 明确实际目标目录、现有规格体系、项目基线摘要、当前需求增量与执行权限。存在多个候选时显式选择；不要为当前需求另建完整架构。
2. 用 guard-grilling 处理问题依赖，用领域建模维护术语及边界。已确认的选择直接复用。
3. 对每个决定记录：问题、结论、依据、需求标识、目标文件、可能的检查方式。明确事实、建议和已确认约束。
4. 在授权范围内更新所选原生规格及已有词汇表/ADR；只读请求则返回建议修改内容。不得另建平行规格或发送外部消息。
5. 写入后重新读取目标文件，确认决定确实保存，检查原生格式与链接。对需要机器验证的规则交接架构契约和 spec-binding。

结束条件：关键问题已解决，决定能定位到原生产物；未覆盖、未批准、未写入的部分明确列出。

Java/Maven 源码治理使用公共协议中的 java 策略和 java_type 映射；复用项目基线及需求增量，以 java_analysis 的源证据检查包、层和归属。修复后重新 check/verify，不把构建子报告 ALLOW 或模型声明当作完整 Java 语义证明。
