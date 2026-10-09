## ADDED Requirements

### Requirement: Frozen Architecture Analysis Profile

ArchGuard SHALL 从受保护配置冻结必查成员、关系、语言工具版本及 feature/target 语义；在求值前验证规则适用性。缺少必查对象或能力 MUST 表达覆盖缺口，不得仅因事实无匹配而满足义务。

#### Scenario: Unknown required package
- **WHEN** 合同要求检查的成员不在实际观察范围内
- **THEN** 增强 profile 输出缺失 scope 并阻止满足完整分析义务，不以空匹配 PASS

#### Scenario: Candidate deletes scope configuration
- **WHEN** 候选删除模块或修改自带配置以缩小必查范围
- **THEN** 分析仍按先前冻结的受保护范围检测缺失项

### Requirement: Cargo Declaration Provenance

ArchGuard SHALL 保留声明依赖的实际成员身份、kind、optional、target 条件与清单来源；legacy 投影 SHALL 继续产生直接成员 path 四元组。声明图 MUST NOT 被称为已启用 feature/target 构建图。

#### Scenario: Renamed and duplicated member declarations
- **WHEN** 同一实际成员通过重命名及 dev/build 声明被多次引用
- **THEN** 增强 artifact 保留声明区别，legacy facts 使用真实包名并合并重复四元组

#### Scenario: External or non-member dependency
- **WHEN** 依赖指向 registry/git 或非成员 path
- **THEN** 当前成员图 profile 不生成成员边，并在能力声明中说明排除范围

### Requirement: Declared Snapshot Inventory

ArchGuard SHALL 区分现有 manifest digest 与增强分析输入 inventory；后者 MUST 声明清单、锁文件、配置、源码、工具链与环境身份中哪些影响当前 capability，并绑定实际输入。增强摘要不得静默改变既有 digest 语义。

#### Scenario: Configuration changes with identical manifests
- **WHEN** 清单未变但已声明影响分析的配置或工具版本变化
- **THEN** 增强分析缓存键和证据输入摘要变化，旧 manifest digest 不被冒充全输入证明

#### Scenario: Equivalent extraction order
- **WHEN** 输入字节和配置相同，仅提取顺序不同
- **THEN** 对应 profile 的规范化事实与摘要保持一致

### Requirement: Bounded Architecture Execution

ArchGuard SHALL 在隔离工作副本中限制可读路径、子进程时间、输入/输出规模和可执行工具，不执行任意策略脚本。预算、工具或路径失败 MUST 保留具体诊断，不得转为成功的空图。

#### Scenario: Symlink escapes permitted roots
- **WHEN** 输入路径解析到未授权根目录之外
- **THEN** runner 拒绝该读取并记录范围失败，不采集越界内容

#### Scenario: Metadata timeout or oversized output
- **WHEN** 增强 runner 的工具超时或输出超过冻结预算
- **THEN** 停止提取并返回执行错误，不发布 ALLOW；legacy profile 的既有 partial 行为保持兼容

### Requirement: Legacy Cargo CLI Compatibility

ArchGuard SHALL 保留现有 check 参数、strict engine protocol 和 0/2/3/4 退出语义；增强 profile 必须显式选择。`--report` 仍仅写文件；无该参数时输出 JSON 到 stdout。输出失败 MUST NOT 使用旧文件宣称本次成功。

#### Scenario: Existing commands are replayed
- **WHEN** 运行既有 allowed、forbidden、missing-manifest 及 review 合同 fixture
- **THEN** legacy 分别保持 0、2、2、3，报告语义与 golden 一致

#### Scenario: Root or output cannot be accessed
- **WHEN** 根目录无法 canonicalize 或指定报告不能写入
- **THEN** legacy 返回 4 并向 stderr 诊断，消费者不能采用旧结果文件
