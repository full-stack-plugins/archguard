# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard 是 Partme Guard 的专业**架构守卫**。当前 0.5.0 在统一项目基线中增加可选 Java 源码包依赖、分层边界和领域类型实际归属检查，复用需求增量、影响分析、修复复验及显式基线更新；保留 Maven/Cargo 构建检查。详见 [Java 源码指南](docs/java-source.md)、[项目模型指南](docs/project-model.md) 和 [插件包](plugin/)。

## 工程准备

要求 Rust 1.85+ 和 Cargo；将 `guardengine`、`archguard` 两个独立仓库放在同级目录。当前使用临时 `path = "../guardengine"` 依赖，正式发布应使用固定版本的公共包。

## 可运行示例

在 `archguard` 目录执行：

```sh
cargo run -- check --project fixtures/allowed --contract examples/agent-job-contract.yaml
# 合法依赖，ALLOW，退出码 0

cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml
# 违规依赖 agent-job -> agent-saas，BLOCK，退出码 2

cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml --facts facts.json --report evidence.json
# 违规情况下也会输出事实和证据报告。
```

## 守卫检测范围

Cargo 分析器调用 `cargo metadata --no-deps --offline --locked --format-version 1`，检查本地成员声明依赖。Maven 分析器只读 POM，支持本地父模型、属性、依赖管理与 modules/subprojects，不启动构建。规则包括直接禁止、间接路径禁止、允许依赖清单、必需依赖和循环检查。

模型可校验声明的上下文、模块、聚合/实体等归属与关系，以及内部构建依赖。启用 java 策略后还检查源码导入/显式引用、包分层和领域类型实际归属。完整编译器类型推导、Rust/Java 方法调用、外部有效依赖解析和 DDD 业务不变量仍未覆盖。影响模型的 profile、无法解析父 POM 等返回覆盖不足，不默认放行。

分析失败时事实状态为 `partial`，规则状态为 `INDETERMINATE`，最终 `BLOCK`。新版契约引用不存在模块或不支持关系也会阻断。证据仅绑定实际检查的构建输入；verify 重新分析当前输入和契约，拒绝旧报告和篡改。导出只允许新文件。

ArchGuard 只提取事实，GuardEngine 负责契约与判定。可信合并门禁仍需在受保护 CI 上独立运行。

详见 [技术架构](docs/architecture.md)、[OpenSpec](openspec/changes/add-cargo-workspace-guard/) 和 [Guard Protocol](https://github.com/full-stack-plugins/guardengine/blob/main/docs/protocol.md)。

## 四层架构治理与实施规划

未来 ArchGuard 以四层守卫为完整产品范围：**System Guard（系统/模块边界）**、**Domain Guard（领域/聚合不变量）**、**Object Guard（类型/对象职责）**、**Method Guard（方法调用、签名和副作用）**。后续计划扩展 Java ArchUnit、TypeScript 语义与 Rust 符号索引、Design Diff 和可信 CI。

**当前实现覆盖 Maven/Cargo 声明模块图的约束闭环**，对象/方法治理尚未实现。对象/方法合理性不能通过 AI 的“高置信度”得出确定性 PASS，必须以批准的架构契约、真实语言事实和必要的人工评审为依据。

完整方案见 [详细架构与 ADR](docs/architecture.md) 和 [技术方案、阶段实施与测试](docs/technical-design.md)。

运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`。
许可证：Apache-2.0。

## 智能体安装与提示注入

已接入共享安装器：`install`、`uninstall`、`context`、`instructions render|sync`、`doctor --integration`、`serve --mcp`。支持 Codex、Claude Code 和 Gemini CLI；按本工具职责生成独立标记区块。

```sh
cargo build --locked
archguard install --agents codex,claude,gemini --project /absolute/project --bin-dir /absolute/bin --dry-run
archguard context --project /absolute/project
```

构建后请先通过 `target/debug/工具名`（Windows 加 `.exe`）调用；上面的短命令假定程序已加入 PATH。指定 `--bin-dir` 会复制当前二进制，安装器不自动修改 PATH；去掉 `--dry-run` 才实际安装。需要同级 `../guardengine/crates/guard-install` 源码。

[统一设计、宿主文件映射、更新、卸载和验证说明](https://github.com/full-stack-plugins/guardengine/blob/main/docs/agent-installation.md)。安装成功不代表宿主已加载或专业检查通过。

## Architecture methods / 架构方法

[原生规格适配、五项技能与 Maven profile 检查闭环](docs/method-integration.md)：0.3.0 新增只读 spec context、可选规则到原生规格绑定、绑定产物漂移复验和显式 profile 场景。插件方法由固定来源依赖闭包生成，不代替确定性检查或授权。
