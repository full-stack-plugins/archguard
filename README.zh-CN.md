# ArchGuard

[English](README.md) | [简体中文](README.zh-CN.md)

ArchGuard 是 Partme Guard 的专业**架构守卫**。当前 V0.1 原型从真实 Rust Cargo 工作区提取包依赖事实，再交给独立的 GuardEngine 执行约束规则。

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

当前分析器调用 `cargo metadata --no-deps --offline --format-version 1`，检查**同一 Cargo 工作区内本地成员包的直接依赖**。包含声明的普通、可选、开发和构建依赖。

它暂时**不支持** Rust 方法调用图、外部依赖运行期调用、Java 模块规则、DDD 聚合归属或 AI 主观设计质量评估。

如果 Cargo 分析失败，事实状态为 `partial`，规则状态为 `INDETERMINATE`，最终 `BLOCK`。证据摘要仅覆盖实际检查的 `Cargo.toml`，不能冒充 Git 提交证明。

ArchGuard 只提取事实，GuardEngine 负责契约与判定。可信合并门禁仍需在受保护 CI 上独立运行。

详见 [技术架构](docs/architecture.md)、[OpenSpec](openspec/changes/add-cargo-workspace-guard/) 和 [Guard Protocol](../guardengine/docs/protocol.md)。

运行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`。
许可证：Apache-2.0。
