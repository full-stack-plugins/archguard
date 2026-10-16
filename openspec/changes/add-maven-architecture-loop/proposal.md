# Maven 架构规范与检查闭环

## Why
用户要求六 Guard 分阶段落地，优先 Java/Maven 与 ArchGuard 架构约束闭环，同时保留 Cargo。既有 add-cargo-workspace-guard 的已验收范围保留；它明确排除 Java，本 change 是用户授权的新阶段，不覆盖旧验收。

## What Changes
- 新增只读 Maven reactor 声明依赖分析器，支持模块聚合、坐标属性和本地父模型继承；未覆盖模型能力必须披露并 fail closed。
- GuardEngine 增加 v1alpha2 覆盖声明与图规则：禁止直接关系、禁止间接路径、允许依赖范围、必须存在关系、无环；保留 v1alpha1 兼容。
- CLI 完成 contract validate、check、explain、verify；verify 必须重新分析当前输入而非只比较自报摘要。
- 共享 MCP 支持由工具显式注册的只读专业操作，ArchGuard 插件只调用相同实现。
- 给六 Guard 明确分阶段路线与交接契约；本轮不声称其他 Guard 的领域引擎已实现。

## Impact
唯一增量规格在本 change；实现涉及 archguard、guardengine 及 archguard/plugin。保留上一轮未提交改动，不发布、不安装真实宿主、不修改业务项目规范或源码。

## Non-goals
Java 类型/方法调用语义、反射、运行时图、完整 Maven effective-model 解析、自动修改架构规范或源码、远程签名及保护分支审批。
