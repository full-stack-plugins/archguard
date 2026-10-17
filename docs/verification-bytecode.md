# Java 字节码闭环验证记录

日期：2026-10-17。对应既有总体任务 EG-A01 / EG-ARCH-001 / EG-PROD-001 / EG-PROD-006，不另建规格体系。本记录只覆盖首批默认 Maven reactor 的 main-Java 精确包禁止依赖，不关闭领域行为、方法契约、完整平台或发布路线。

## 实现与验证范围

| 原任务义务 | 当前实现及证据 |
|---|---|
| 独立产品入口 | 实际 bytecode check/verify CLI；旧 check/explain/verify、项目模型、需求增量及 Cargo 检查保留 |
| 真 ArchUnit | 固定 ArchUnit 1.5.1；同一私有候选中先离线 Maven 编译，再实际导入 class、调用 ArchRule.evaluate 并产生具体规则/边/源码位置 |
| 批准规则解释 | 本域 BytecodePolicy 从真实 CompiledContract 提取精确禁止边；普通 JSON 不可生成批准计划；未知规则和 owner 不匹配拒绝 |
| 公共执行接线 | RegisteredPlan + Runtime::execute_v2，固定配置/工具/环境、真实时间及运行序列；私有原始双流与执行观察分别保留 |
| 公共结果 | 指定义务的 GuardResult 从当前账本与不可变原件投影，继承批准 rule 的 mode/severity，筛选当前 rule 的违规；固定 UNVERIFIED，不签 GateDecision/Grant |
| 修复复验 | 旧收据原件复核、重新物化及编译当前候选；允许源码修复，拒绝基线/配置/规则/身份范围变更；不读旧 class 或在原仓 clean |

## 实际失败与修复

- 私有 Windows verbatim cwd 曾使真实 JDK ZipFile 失败；改为同一目录的等价原生 DOS/UNC cwd，未改变 GitPath 字节身份。
- Partial 结果曾提前返回而绕过阳性归属核验；真实反例失败后，增加 source/tool/rule/bytecode 摘要、已批准规则、当前源路径和依赖边验证。有效阳性仍保留，未知范围保持 Partial。
- 原 verify 漏比基线 OID；真实 CLI 已先完成 Commit/Worktree 检查后，更换有效 base 并预取消，旧实现仍接受。反例运行实际失败，148.87秒；新增 selectionScopeDigest 比较后恢复正确拒绝，允许普通源码修复。
- 两个真实外部输入闭包反例曾编译成功却错误返回 complete：systemPath JAR、候选外相对父 POM。Java adapter 已拒绝这些不受支持路由，同时保留正常 reactor 正例。
- 插件注入提示曾缺新入口；root 新增真实提示断言先失败，再补 bytecode check/verify 及 local_unverified 限制，同一测试通过。

## 已执行证据

- Windows / JDK17.0.12 / Maven3.9.9：Java 全19项测试、package成功，无失败、错误或跳过；修正三个测试的跨平台 launcher 名称后再次19/19通过，29.722秒。
- 跨平台测试修复时 helper JAR SHA256 为 `b17de6f737e6e740f2a578aa2795b784b4818ec0c39db5335d767a41c55e9fdf`。root随后按项目约定将Objects显式import，当前JAR SHA256为 `b041b56c939e229b68464bae851620dfa0fbf26245cb5d8573f9ea8e20ae5ad8`；重新离线package，Java19/19无跳过通过（50.513秒），并重新核验14依赖锁及全部275工具pin。
- 当前 Rust 接线最后两个真实 E2E 全通过，226.28秒：实际 CLI Commit忽略dirty Worktree、Worktree违规、篡改旧收据拒绝、换base拒绝、源码修复后verify；真实 CompiledContract/RegisteredPlan/Runtime 到逐规则公共结果及 Private refs、时间、sequence、错误producer/config、终态同operation不重跑。
- root 对 0.6.0 候选独立执行 Rust1.85 `--locked --offline --all-targets`：67项通过；旧 Cargo/模型/Java源码回归包含在内。严格 Clippy all-targets 且启用 bytecode-java-tests 通过；格式检查通过。新提示断言另有同一测试 RED→GREEN。
- root在最终0.6.0源码及上述新JAR/pin配置上独立执行两个真实端到端用例，2/2通过，325.13秒，无失败或跳过；没有沿用旧JAR摘要。
- CI配置准备器读取现有明确工具及缓存，实际生成275个 pin，与原本地配置逐项一致并重新核验；拒绝相对工具路径或覆盖已有配置。它不安装或联网、不构成运行验收。

当前已新增 Ubuntu/Windows 独立 Java CI 作业，执行真实 Java tests/package、14依赖锁、19测试零skip门禁及两个真实 Rust E2E。发布后必须检查具体运行，不把本地结果推广为 Linux 已通过。现阶段 EG-A01 仍待该提交 CI 与最终审查，不提前勾选。

## 保留边界

main-Java 默认源码目录、完整 reactor 和明确预期类型集合；缺失/重复 class、空输入、损坏字节码、编译失败、输入/工具漂移保持不完整。测试源码、注解处理、自定义编译、profiles、反射和领域不变量不获 complete。JDK 可执行文件及部分运行原件 pin 不等于完整受信镜像；本地文件完整性不等于平台身份或批准。已有项目基线仍通过原本显式授权流程更新，字节码收据不会自动发布基线。
