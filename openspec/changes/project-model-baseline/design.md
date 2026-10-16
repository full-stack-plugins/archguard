# 项目模型闭环设计

## 权威与文件

本 OpenSpec change 是该实现的唯一规格源；需求内容继续存放在使用项目已采用的 OpenSpec/Spec Kit/Superpowers 中。

- `.archguard/baseline.json`：当前快照摘要指针。
- `.archguard/baselines/<sha256>.json`：不可变版本，包含 revision、parent、model、contract_digest 及变更/报告摘要。旧版本保留。
- `.archguard/changes/<id>.json`：schema_version、id、base_digest、requirement 引用、元素/关系 upsert/remove、touched_modules。
- `.archguard/contract.yaml`：已有可执行规则，模型更新不能暗中修改或放宽规则；快照绑定其语义摘要，契约迁移不在本版本自动执行。

模型 schema_version=1，包含 project、elements、relations。元素含稳定 id、kind、name、responsibility，可选 context/module/implementation。kind 支持 context/module/capability/aggregate/entity/value_object/interface/event；模块 implementation 对应 Maven groupId:artifactId 或 Cargo package name，必须完整覆盖当前分析器全部模块且唯一。上下文和模块归属必须有效，业务对象模块与上下文一致。relations 为 id/from/to/kind，支持 depends_on、uses、publishes、subscribes；depends_on 只连接模块，事件关系的目标必须是 event。

模型是目标基线，不能直接把扫描事实当作正确设计。depends_on 表达当前目标依赖，候选检查与实际内部声明边集合双向比较；领域对象仅检查声明结构和归属，不宣称验证了 Java 类或业务不变量。

## 增量与影响

增量绑定 base_digest，验证原生需求 artifact/requirement 标识并绑定规格摘要。旧基线不能直接重放；必须在当前基线重新审阅增量。删除不存在的 ID、删除后悬空引用、冲突操作、无效果且未声明 touched_modules 的变更拒绝。

影响分析在旧/新模型的关系与归属联合图上保守扩展，覆盖增加、修改、删除元素和关系，返回受影响模块/上下文及路径。touched_modules 用于模型不变的实现需求。它不是精确 Java 调用图，可能扩展至全项目；实际检查始终全项目执行，不因影响子集缩小而跳过循环或原有契约规则。

## 检查、复验和更新

`model validate/show`、`change impact/check/verify` 只读；check 可导出新报告。候选模型、基线、增量、原生规格、实际构建输入、契约与分析器版本均进入报告。verify 对当前输入重新计算后完整比较，旧报告与伪造 ALLOW 不可复用。

`baseline init --model FILE` 和 `baseline update --change ID --report FILE` 默认只返回预览；只有 `--apply` 才更新本地。初始化也必须通过当前全项目检查，不能覆盖现有基线。更新拒绝 BLOCK、REQUIRE_APPROVAL 和陈旧证据；不承认报告文本中的批准字段。

更新使用 create_new 锁、只新建的不可变快照以及临时指针原子替换。写入前重新核对指针、增量与报告；失败保留原指针，孤立快照可留存，不自动清理未知文件。不提供强制覆盖、自动重基、自动放宽契约、自动 Git 提交或审批签发。锁只协调本 CLI；恶意并发文件替换及掉电恢复需要受控 CI/文件系统保障。

## 兼容与边界

没有 baseline.json 时原有架构检查保持不变；存在基线时普通 check 同时校验模型与当前实际模块图，并将基线摘要绑定到报告，模型输入错误失败关闭。需求开发使用 change check 检查候选，合并/发布检查当前基线。

首次采用为单个有界 JSON 模型（4 MiB），通过 model show --entity 提取邻域视图；物理分片、多项目联合、Java import/类型/方法语义及自动 EA 同步不在此版本。Maven 保留显式 profile/局部模型限制，Cargo 保留离线 locked metadata。每项输出声明覆盖边界，哈希不是签名，本地更新不是审批或交付授权。
