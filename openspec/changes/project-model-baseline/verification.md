# 项目架构基线验证记录

日期：2026-10-16。版本：ArchGuard CLI / plugin 0.4.0。规格事实源为本 OpenSpec change；状态为本地实现与验证完成、未归档、未发布。

## 验收对应

| 规格 | 实现与证据 |
|---|---|
| 唯一项目基线 | model validate/show、内容寻址快照和摘要指针；初始化不能覆盖、篡改快照拒绝读取 |
| 需求增量 | 稳定 ID、基线摘要、原生需求引用；冲突/悬空/旧版本/错误引用均拒绝 |
| 影响与全局约束 | 旧新关系和归属图保守传播，包含删除的旧邻居；检查仍覆盖完整 Maven/Cargo 图 |
| 修复与复验 | Maven 循环违规→修复→新报告→复验→更新；Cargo 使用同一生命周期 |
| 更新保护 | 预览无状态写入，只有匹配 ALLOW + --apply 才发布；锁、源码/规格/契约/增量漂移、伪造报告均不能推进指针 |
| 兼容与插件 | 无基线兼容旧检查；有基线绑定普通检查；5 个新只读 MCP 与 CLI 共用实现；五项公共架构技能更新并重新锁定打包 |

## 实际执行结果

所有测试临时目录、Cargo home/target 均重定向到工作区 `.guard-build`，未安装宿主插件、未初始化真实业务项目基线。

- ArchGuard `cargo test --locked`：38 项通过（architecture_loop 18、integration 5、project_model 15）。
- ArchGuard `cargo fmt --check`：通过。
- ArchGuard `cargo clippy --locked --all-targets -- -D warnings`：通过。
- GuardEngine `cargo test --locked --all-targets`：19 项通过（cli_evidence 2、contract_tests 8、graph_rules 9）。
- GuardEngine `python -B scripts/skill_bundle.py --library skill-library --plugin ../archguard/plugin --check`：verified，6 项方法及公共依赖，无待生成文件。
- GuardEngine `python -B -m unittest discover -s scripts/tests -v`：5 项通过，含离线依赖闭包、循环/摘要错误及人工修改保护。
- GuardEngine `python -B scripts/verify_guard_installation.py --bin-dir E:/workspaces/workspace-agent-skills/.guard-build/target/debug --gitflow-root ../gitflow-plugin`：六工具安装/MCP/上下文共存回归 10 项通过。
- `openspec validate project-model-baseline --strict`：通过。
- `archguard model validate --project <本仓库绝对路径> --model examples/project-model.json`：valid，明确 implementation_checked=false。
- 实际二进制 `--version` 返回 0.4.0，`--help` 包含模型、需求增量及显式基线更新入口。
- `git diff --check`：通过；已有文件的 LF/CRLF 提示不是测试失败。

安装回归第一次使用错误的 gitflow 路径，9 项通过、1 项因 fixture 不存在报错；更正为实际 `gitflow-plugin` 后完整 10 项通过，未通过修改产品代码规避失败。

## 失败后通过与审查

首先添加 Maven/Cargo 项目基线与增量生命周期测试，因 CLI 能力缺失失败，再实现对应能力；只读 MCP 测试先因工具缺失失败，再接入共享实现。报告写入模型状态目录及影响路径无界测试先失败，随后加入状态目录保护和有界路径样本。

独立只读审查发现一项 P2：报告导出按当前目录解析，而复验按项目根解析。从项目外执行时不能完成闭环。新增两个测试重现失败，随后统一为项目根并在写入前拒绝项目外路径；两项测试通过，审查者复核后关闭 P2。未发现其余可确认的 P1/P2。审查不替代本记录中的实际测试。

另有负例覆盖同图但 POM 内容变化、损坏快照、精确原生需求引用、REQUIRE_APPROVAL 和 partial 分析不得初始化基线。

## 覆盖边界

- 领域模型验证声明结构、稳定身份、归属和关系；Java 类、方法、聚合不变量及实际业务语义尚未检查。
- Maven 使用已有静态声明图与显式 profile 能力，不运行 Maven 或声称完整有效模型；Cargo 保留 offline locked metadata 范围。
- 影响范围为保守旧新图可达集；路径样本有界，不是精确源码调用影响。
- 报告只绑定实际检查输入，不是完整 Git 提交或签名证据；本地 --apply 不构成人工审批。
- 锁只协调本 CLI；未实施恶意并发文件替换、跨进程编辑器或掉电故障注入。
- 模型物理分片、跨项目联合、EA/XML 自动交换及契约迁移未实现。
- 子进程 MCP 与包检查已通过；未声明真实宿主已安装/加载，也未在用户 ddd4j 仓库运行基线写入。
- 保留工作区已有修改；未提交、切分支或发布。
