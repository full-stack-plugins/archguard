# Java 源码架构检查验证记录

日期：2026-10-16。ArchGuard CLI / plugin 0.5.0。本 OpenSpec change 为唯一规格事实源；实现已在本地验证，未归档、未发布、未安装到真实宿主。

## 实现

- 项目模型可选 java 策略：包前缀归属、跨层允许表、显式禁止包依赖及测试源码范围。
- 领域元素 java_type 映射实际顶层或成员类型；校验唯一存在、真实 Maven 模块和声明归属。
- 使用固定 Tree-sitter Java 0.23.5 / Tree-sitter 0.25.10 解析源码；真实包引用与源行号进入 java_analysis。
- 需求增量复用原生规格和 base_digest，支持 java 策略更新与 java_type 元素更新。
- 源码内容与文件清单绑定候选报告和普通报告；check → 修复 → verify → 已授权 --apply 复用现有基线发布机制。
- 五项架构方法及公共依赖已从源库更新、重锁并生成插件；原有 10 个只读 MCP 共用检查实现。

## 验证命令与结果

所有 Cargo home、target、测试临时目录均位于工作区 `.guard-build`，Python 使用 -B，未修改 F 盘或全局宿主安装。

| 执行目录及命令 | 结果 |
|---|---|
| archguard: cargo test --locked | 53 项通过：原架构循环 18、旧集成 5、项目模型 15、Java 源码 15 |
| archguard: cargo fmt --check | 通过 |
| archguard: cargo clippy --locked --all-targets -- -D warnings | 通过 |
| archguard: openspec validate java-source-architecture --strict | 通过 |
| guardengine: python -B scripts/skill_bundle.py --library skill-library --plugin ../archguard/plugin --check | verified，无缺失或陈旧文件 |
| guardengine: python -B -m unittest discover -s scripts/tests -v | 5 项通过 |
| guardengine: python -B scripts/verify_guard_installation.py --bin-dir E:/workspaces/workspace-agent-skills/.guard-build/target/debug --gitflow-root ../gitflow-plugin | 六工具安装/共存/提示/MCP 回归 10 项通过 |
| archguard model validate --project 本仓库绝对路径 --model examples/java-project-model.json | valid；implementation_checked=false |
| git diff --check | 通过；仅已有 LF/CRLF 提示 |

测试运行在 Windows；未声称已在 Linux、macOS、真实 ddd4j 项目或宿主插件加载后复测。GuardEngine 核心源码未改动，不把上批的 19 项引擎测试写成本批新结果。

## Java 验收证据

15 项真实 CLI/MCP 测试覆盖：同模块跨层违规及修复复验发布、static/wildcard/FQ 引用、外部禁止包、领域类型缺失/错误实际模块/重复声明、record 和注释字符串、POM 不变时的源码内容与新增文件漂移、策略及类型映射需求增量、未知层/包、包级注解、Windows 路径与 Unicode 转义区分、测试源范围和自定义源根、普通报告源码漂移、真实 stdio MCP 决策一致、深层语法树预算。

最初 5 项因缺少 java_type/java 模型支持失败后实现转绿。外部全限定禁止引用绕过测试先失败后修复。独立审查发现两个 P2：package-info 注解被跳过、合法 Windows 路径误判 Unicode；两项均通过失败测试复现、修复并由审查者复核关闭。深层字段链预算测试也先失败后修复。Clippy 曾发现 collapsible_if，修正后严格检查通过。

## 实际边界

- 显式配置才启用 Java，旧模型和 Cargo 的既有循环保持兼容；旧模型省略字段不改变其序列化格式。分析器升级后需新报告。
- 范围是 reactor 标准 main 源根，可选 test 根。自定义/已识别生成根、JPMS、有效 Unicode 转义、不完整解析等阻断；任意 Maven 插件执行后的完整有效源模型未实现。
- 导入和已知全限定引用属于语法事实，通配与外部禁止前缀保守处理；完整简单名/类型绑定、运行时调用、反射、领域业务语义未验证。
- 影响分析仍用现有保守模型范围，策略变化影响全部模块；源码检查不缩小全项目架构门禁。
- 修复是按源证据由智能体执行后复验，没有自动重构 CLI。局部 ALLOW、哈希或显式 apply 都不等于人工审批。
- 未初始化真实业务项目基线、未切分支、未提交或发布；工作区原有修改保留。
