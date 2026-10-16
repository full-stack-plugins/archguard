# 项目统一模型与需求增量

0.5.0：模型和需求增量允许可选 java 策略，元素允许 java_type；启用后的源码包/分层/领域类型归属检查遵循 [Java 工作流](java-source.md)。同时检查 java_analysis.findings，不能只看构建子报告。

1. 查询 context 的 domain.model。不存在时先阅读已有架构、契约与原生规格，形成可审阅的项目模型；不得把当前扫描结果直接认作正确设计，也不自动初始化。
2. 已有基线使用 `archguard model show --project PROJECT`；按需增加 `--entity ID` 获取一跳视图。视图不能覆盖完整模型。MCP 为 archguard_model_show / archguard_model_validate。
3. 需求保存在 `.archguard/changes/ID.json`，绑定当前 base_digest 和原生需求引用。只管理 upsert/remove 与 touched_modules；完整目标模型只在基线保存。不得复制第二套需求正文。
4. 先 `change impact --change ID --project PROJECT`，审阅直接变化、受影响模块/上下文和 paths；路径样本可能截断，predecessors 与 affected_elements 保留完整可达信息。影响是声明关系与归属的保守范围，不是 Java 调用图。
5. `change check --change ID --project PROJECT` 始终对整个 Maven/Cargo 项目执行契约检查，并核对候选模型全部模块和依赖。MCP 对应 archguard_change_impact / archguard_change_check。
6. 根据 model_findings 和 architecture_report 修复已授权源码/声明；契约问题可用原有 explain。修改模型必须符合已确认的需求决策，不能通过删除关系、缩小模型或关闭场景来“修复”违规。
7. 执行业务相关测试，再运行 `change check --change ID --project PROJECT --report NEW_FILE`，然后 `change verify --change ID --project PROJECT --report FILE`。MCP archguard_change_verify 接受绝对 report；所有文件必须位于项目内。
8. 预览 `baseline update --change ID --project PROJECT --report FILE`。只有用户/任务已授权更新项目基线，才追加 `--apply`。工具内部重新全量检查和比对报告，保留旧快照，拒绝过期基线。该 CLI 操作不是可验证人工审批，也不自动 commit/push。
9. 更新后运行普通 check；后续需求必须基于新 digest。并行需求变陈旧时重新审阅差异，禁止仅改 base_digest 掩盖冲突。

初始化：`model validate --project PROJECT --model FILE` 后，`baseline init --project PROJECT --model FILE` 预览，已授权时加 --apply。初始化也要求模型与当前完整构建图一致且契约 ALLOW。不能覆盖已存在的基线。

模型根字段 schema_version=1、project、elements、relations。元素字段 id/kind/name/responsibility，以及按类型设置的 context/module/implementation；模块 implementation 为 Maven 坐标或 Cargo 包名。非 context 必须归属 context，领域对象须归属同上下文模块。关系字段 id/from/to/kind；支持 depends_on、uses、publishes、subscribes。

增量必需字段：schema_version=1、id、base_digest、requirement={system,target,artifact,requirement}、upsert_elements、remove_elements、upsert_relations、remove_relations、touched_modules。无结构变化的需求用 touched_modules 声明稳定模块 ID。未知字段、悬空引用、重复/冲突操作、过期来源都阻断。

当前模型声明一致性和 Maven/Cargo 构建依赖可确定性检查；Java 类/方法、业务不变量、运行时调用仍须单独验证。无基线的旧项目保持原有契约检查。基线绑定当前契约摘要，自动契约迁移未实现，不得手改不可变快照绕过检查。
