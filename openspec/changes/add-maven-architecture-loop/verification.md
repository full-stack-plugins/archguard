# 验证记录：add-maven-architecture-loop

日期：2026-10-15；Windows PowerShell，Rust/Cargo 1.99。所有构建缓存、输出、临时 fixture 在 E:\workspaces\workspace-agent-skills\.guard-build。未安装/升级全局工具，未改真实宿主配置，未修改 ddd4j 项目，未切分支、提交或推送。保留此前安装工作未提交的修改。

## 测试驱动证据

首次运行新测试：GuardEngine 4 个图规则用例因未知 v2/断言失败；ArchGuard 3 个 CLI 用例因缺少默认契约、explain/verify 与 JSON 错误输出失败。补齐实现后通过。实际 ddd4j 诊断暴露 jvm.config 过度阻断，再补 JVM 内存参数用例，确认失败后修复；系统属性覆盖仍阻断。

## 已执行

| 验证 | 实际结果 |
|---|---|
| GuardEngine cargo test --locked --all-targets | 19 通过（原有 10 + 图规则 9） |
| ArchGuard 集成测试 | 17 通过（Cargo 原有 5 + 架构闭环 12） |
| guard-install --features test-fixture | 3 通过（回滚、无写预览、幂等安装/卸载） |
| 六工具真实进程验收 verify_guard_installation.py | 9 通过，19.240 秒 |
| ArchGuard / GuardEngine / guard-install clippy | -D warnings 通过；共享层含 test-fixture |
| 三个 crate cargo fmt | 通过 |
| OpenSpec strict validate | add-maven-architecture-loop 有效 |
| 六个 CLI 编译 | 成功；CodeGuard 明确使用 --no-default-features 仅接入构建 |
| 真实 ddd4j 源诊断 | INDETERMINATE/BLOCK：ddd4j-parent/pom.xml 存在模型相关 profile |

合计 48 个已执行测试。对新增测试最后单独运行 architecture_loop 的 12 个用例均通过；未因仅修改文档而重复所有测试。

## 行为证据

- Maven 4 命名空间/subprojects、本地父 POM（含不在 reactor 的父模型）、继承属性与依赖管理、间接路径与循环见证边均有用例。
- 直接 dependencyManagement 不产生依赖边。模块逃逸、缺失、重复坐标、版本冲突、未知属性、外部父模型、DTD、异常命名空间/profile 不能默认通过。
- 源修改、契约修改、报告 signed 等字段篡改均被 verify 拒绝；修复后新报告可以验证。
- 新文件导出预检全部目标；目标与现有 POM 冲突时不写前面的事实输出。
- 实际子进程完成 MCP initialize/initialized/list/check/explain/verify；输出与 CLI 相同，拒绝相对项目路径与输出文件参数。
- plugin/mcp.json 的真实 args 驱动同一测试二进制，manifest 版本与 CLI 一致；动态提示包含项目规则和专业入口。
- 获取并人工逐项核对 Agent Plugins 1.0.0 官方 plugin/mcp schemas；本机没有通用 JSON Schema 验证器，未静默安装，也不声称已运行完整 Schema 验证器。
- Maven 4 结构依据官方模型文档：https://maven.apache.org/ref/4-LATEST/api/maven-api-model/maven.html
- 插件结构依据：https://agent-plugins.org/schemas/1.0.0/plugin.schema.json 与 https://agent-plugins.org/schemas/1.0.0/mcp.schema.json

## 兼容性和未验证边界

v1alpha1 精确关系契约保持历史判定；v1alpha2 才要求严格覆盖。Rust GuardFacts 字面量新增 coverage 字段，已更新本工作区直接调用测试。引擎/分析器版本提升至 0.2.0，旧报告须重跑生成。

当前结果是声明模块图，不是 Java import/方法、有效 Maven 编译模型或 DDD 领域不变量证明。ddd4j 的诊断契约仅用于验证覆盖不足行为，不代表业务架构规则已经批准或通过。

CI 已补 Ubuntu/Windows 目标测试步骤，但未在远程运行。插件是可移植源包；未安装到真实智能体宿主或发布市场。当前仍需同级 guardengine 源码依赖，跨仓发布须协调。CodeGuard 全量 Windows CLI 的既有平台问题本轮未修复；其 integration-only 构建不能证明专业检测器通过。

## 规格状态与下一步

本 change 实施及本地验证完成，保留 active change，尚未同步/归档或发布；上一批安装 change 保持独立。后续先实现有明确激活参数和可信来源证据的 Maven effective-model 适配器，再补 Java 源级边界。其余 Guard 的排序见 guardengine/docs/professional-roadmap.md。
