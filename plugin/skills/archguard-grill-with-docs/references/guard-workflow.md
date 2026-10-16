# 原生规格与架构检查公共协议

## Java 源码约束（ArchGuard 0.5.0）

Maven 项目可在统一模型声明 java={packages,layers,forbidden_dependencies,include_tests}；packages 条目 prefix/module/layer 使用点边界最长匹配，layers 条目 id/allowed_dependencies 定义跨层方向，forbidden_dependencies 条目 id/from/to 禁止包前缀依赖。领域对象 aggregate/entity/value_object/interface/event 用 java_type 映射实际全限定类型。通过需求增量 java 字段更新策略、upsert_elements 更新类型映射，不能为通过检查擅自放宽规则。

change check 同时检查整个构建图与配置的 Java 源码，返回 java_analysis 的 edges、findings、文件行号和 source_digest。顶层 decision 才是整体决策，构建子报告 ALLOW 不能代替 Java 通过。按 findings 修复源码，执行业务测试，再导出新报告、change verify 和已授权的 baseline update --apply；Java 文件新增、删除、内容变化都使旧证据失效。

分析范围是 reactor 的 src/main/java，include_tests=true 才包含 src/test/java；不含策略时保留旧行为。语法树覆盖导入、静态/通配导入、包注解及可识别的全限定引用；通配及外部禁止前缀保守处理。自定义/生成源码根、JPMS、有效 Unicode 转义、语法错误和覆盖不足阻断。完整编译器类型推导、运行时关系、领域业务语义仍未验证。MCP 全部只读，模型结构校验不证明实现正确。

## 项目统一模型与需求增量（ArchGuard 0.4.0）

context 的 domain.model 表示项目是否已有统一模型。已配置时先 model show，按稳定 ID 获取 --entity 邻域；不能为每个需求另建完整项目架构。基线在 `.archguard/baseline.json` 指向不可变 `.archguard/baselines/<sha256>.json`，需求只维护 `.archguard/changes/<ID>.json`。

需求增量绑定 base_digest 和 requirement={system,target,artifact,requirement} 原生引用，使用 upsert_elements/remove_elements/upsert_relations/remove_relations/touched_modules。领域元素通过 context/module 表达归属，模块 implementation 映射 Maven 坐标或 Cargo 包名。先 change impact，再 change check；影响范围用于理解，检查始终完整项目。

修复源码或已确认的模型增量后重新 change check --report NEW_FILE，再 change verify --report FILE。预览 baseline update --change ID --report FILE；仅在已授权更新基线时添加 --apply。基线写入前重新检查并保留旧版本，陈旧增量必须重新审阅，不机械替换摘要。MCP 仅提供 model show/validate 和 change impact/check/verify，不更新文件。上述命令均要求 --project 绝对路径。

缺少基线时先基于现有架构形成可审阅模型，model validate 只校验结构。baseline init --model FILE 默认预览，显式 --apply 仍须完整构建与契约 ALLOW。不要从实际扫描图直接推定正确设计，不擅自初始化、覆盖快照或迁移契约。模型声明一致不等于 Java 类/方法或领域不变量实现正确，本地更新不是审批授权。

## 发现与选择

1. 确认实际业务项目、Git 工作区及项目指令；保护未提交修改。
2. 执行 `archguard context --project <绝对项目路径>`；MCP 对应 `archguard_context`，参数 `project`。
3. 执行 `archguard spec context --project <绝对项目路径>`。自动发现歧义时，显式传 `--spec-system openspec|speckit|superpowers --spec-target <项目相对路径>`。这只读现有文件，不初始化工具。
4. OpenSpec 目标示例 `openspec/changes/order-boundary`；Spec Kit 目标示例 `specs/001-order-boundary`；已有 Superpowers 目标示例 `docs/superpowers/specs/order-boundary.md`。同一变更仅选一个来源。不存在时讨论现有流程，不私自初始化。
5. ready 仅表示必要文件存在且非空；不代表规格语义合格、审批通过或可以发布。原生 CLI 验证与项目授权仍必须遵守。

## 产物归属

- 业务问题、需求标识、验收标准及非目标：写回已选原生规格；不得再生成一套独立 spec。
- 执行步骤：更新该变更现有 tasks/plan；不要复制到第二个任务系统。
- 项目术语：沿用已有词汇表和领域上下文映射；ADR 仅记录难以逆转且存在真实权衡的决定。
- 架构契约：`.archguard/contract.yaml`，这是可执行约束，规则 description 引用原生需求和决策依据。启发式建议先讨论，不直接升级为 enforce。
- 追踪引用：可选 `.archguard/spec-binding.json`，只保存已有文件和需求标识，不复制正文、不存造假的批准记录。

```json
{
  "system": "openspec",
  "target": "openspec/changes/order-boundary",
  "rules": {
    "DOMAIN-NO-INFRA": {
      "artifact": "openspec/changes/order-boundary/specs/layers/spec.md",
      "requirement": "REQ-ARCH-1"
    }
  }
}
```

绑定必须覆盖契约全部规则且无额外规则。artifact 必须属于已选原生产物，requirement 必须是正文中存在的完整标识。绑定只是引用一致性检查，不能证明需求已实现或用户已批准。

## 可执行闭环

```text
archguard contract validate --project <project>
archguard check --project <project>
archguard explain --project <project> --rule <rule-id>
# 在已授权范围内修复业务源码或构建声明，然后重新运行：
archguard check --project <project> --report <新的报告文件路径>
archguard verify --project <project> --report <已存在的报告文件路径>
```

MCP：`archguard_check`、`archguard_explain`（增加 rule）、`archguard_verify`（增加绝对 report）；全部要求绝对 project，可选绝对 contract。MCP 不导出文件。CLI 报告只新建，不覆盖。自定义契约应在 validate/check/explain/verify 每次都传同一 `--contract`；混合根目录每次显式 `--analyzer maven|cargo`。

0=ALLOW；2=BLOCK；3=REQUIRE_APPROVAL；4=输入错误或过期报告。INDETERMINATE 不是架构违规结论，但同样阻断。修复不得靠删除约束、扩大白名单或禁用原有检查达成，除非已有明确的架构变更决策。修复后执行项目相关测试，测试失败与架构检查失败分别报告。

Maven 节点使用 groupId:artifactId，Cargo 使用包名。支持 forbid_relation、forbid_path、allowed_dependencies、require_relation、acyclic；predicate 为 depends_on。匹配结果包含源位置和路径见证。未知实体/谓词不放行。

Maven 显式 profile 场景可使用 `.archguard/maven.json` 的 `{"profiles":{"integration":true,"legacy":false}}`。所有影响模型的 profile 均须明确选择；这里不求值真实 Maven 环境激活，不运行 Maven、不解析远程父/BOM。不得为得到 ALLOW 擅自改变已经确认的场景。Cargo 使用离线锁定 metadata。

## 能力与证据边界

Java import、类型、方法、反射及完整有效 Maven 模型不在当前覆盖中。发现此类问题应给出可核实的人工评审证据并标记未覆盖，不声称机器检查通过。报告绑定读取的构建输入、契约和已绑定原生规格；无 Java 源级分析时，源码测试证据必须另行保存。

只读分析请求不得创建文件。实现请求在授权范围内更新原生产物；外部消息、Git 分支/提交/推送、工具安装和多 Agent 需遵循项目及宿主授权。研究允许串行执行。临时产物使用项目允许目录（本工作区限 D/E 盘）。
