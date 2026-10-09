## ADDED Requirements

### Requirement: Separate Versioned Architecture Evidence

ArchGuard SHALL 在 GE-CONTRACT/GE-ADAPTER 后发布独立 GuardRunEnvelope producer，拒绝未知版本/字段且不隐含 N/N-1。completed engine-backed envelope 的 decision MUST 等于引用 GuardReport，并保留 contract/facts/report、coverage 和领域 artifact 引用。

#### Scenario: Envelope field supplied to current engine object
- **WHEN** 消费者把 coverage 或 binding 放入当前 GuardReport
- **THEN** 兼容测试确认严格引擎拒绝未知字段，新字段仅存在独立版本化包装

#### Scenario: Referenced report disagrees
- **WHEN** envelope 声称 ALLOW 而引用报告为 BLOCK 或摘要不匹配
- **THEN** 验证拒绝整个集成结果，不选择较宽松结论

### Requirement: Bound Execution State and Diagnostics

ArchGuard SHALL 在完整 binding、producer 与 required coverage 冻结后才输出 envelope；此前错误仅输出独立诊断。绑定后执行错误/取消 MUST 使用 error/cancelled 和 null decision；有效 partial facts 求值为 completed/BLOCK，不得被审批放行。

#### Scenario: Repository or candidate cannot be resolved
- **WHEN** 调用缺少可验证 repo/candidate/base 或必要保护策略
- **THEN** 输出失败诊断而非伪造绑定或不完整 envelope

#### Scenario: Cancelled run versus valid partial facts
- **WHEN** 已绑定运行被取消，或另一次运行提交有效 partial facts
- **THEN** 前者为 cancelled/null decision，后者保留 INDETERMINATE/BLOCK，二者均不满足完整门禁

### Requirement: Immutable Candidate Isolation and Invalidation

ArchGuard SHALL 绑定 repo/task/worktree/requirement set、candidate/base/merge group、snapshot/baseline/contract 和 analyzer/coverage；尝试使用独立 runId。输入漂移及审批过期/撤销 MUST 使资格失效，迟到结果不得覆盖新候选。

#### Scenario: Parallel requirements finish out of order
- **WHEN** 同仓两个 requirement 的运行和旧候选重试乱序完成
- **THEN** 结果各归原绑定，compare-and-set 拒绝旧结果覆盖当前状态或跨义务满足

#### Scenario: Synthetic merge candidate changes
- **WHEN** 合并队列重组或 base 变化生成新 candidate
- **THEN** 必须针对精确新对象重跑，旧 PR head 或队列报告不可替代

### Requirement: Authenticated Architecture Evidence Audit

ArchGuard SHALL 通过 GE-TRUST ports 由外部控制器验证 producer、受保护合同、证据引用和审批身份/范围/新鲜度，并保存最小可审计记录。分析器不得保管签名或合入凭据；本地重算 MUST NOT 被视为认证。

#### Scenario: Forged approval or evidence URI
- **WHEN** 报告提供伪造审批、自报 signed=true 或许可存储外 URI
- **THEN** 受信消费拒绝该引用，不授予动作权限，也不改写原技术 decision

#### Scenario: Protected policy is weakened in candidate
- **WHEN** 候选删除禁止边规则并产生本地 ALLOW
- **THEN** CI 使用冻结的受保护合同重新检查，审计记录实际 policy digest 与结果

### Requirement: Read Only Architecture Interfaces

ArchGuard SHALL 使增强 CLI、library 和未来 MCP 共用分析语义及能力协商，明确输入、输出、错误与预算；接口只读且不包含审批签发或 Git ref 写入。命令发布前 MUST 有协议/退出码和权限测试。

#### Scenario: MCP client asks for self approval
- **WHEN** 客户端要求批准自己的架构例外或合并变更
- **THEN** 接口拒绝未提供的能力，仅返回允许的分析/解释结果

### Requirement: Phased Architecture Release and Rollback

ArchGuard SHALL 经 advisory、shadow、opt-in protected、显式 enforce 分阶段发布；独立生产发行等待 GE-RELEASE，并使用固定依赖与明确 capability 矩阵。回滚 MUST 保留 legacy CLI、不可变证据及必查义务，不把禁用适配器变成门禁成功。

#### Scenario: Adapter rollback after shadow mismatch
- **WHEN** 新投影与 legacy golden 不一致而回退增强适配器
- **THEN** 原命令恢复原语义，历史证据保留，要求增强 evidence 的门禁保持未满足

#### Scenario: Production release requested before engine gate
- **WHEN** 开发已使用固定源码但 GE-RELEASE 尚未通过
- **THEN** 可继续本地验证，不宣称独立生产包与兼容矩阵已交付
