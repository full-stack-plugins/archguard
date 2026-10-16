# Java 源码检查设计

本变更是唯一规格事实源。新增可选模型 java 策略，含 packages（prefix/module/layer）、layers（id/allowed_dependencies）、forbidden_dependencies（id/from/to）、include_tests（默认 false）。包前缀按点边界、最长匹配；相同前缀和未知模块/层拒绝。外部包只参与显式禁止规则，未声明层的外部包不宣称已验证分层。

领域对象增加可选 java_type 全限定声明映射；启用 java 时 aggregate/entity/value_object/interface/event 必须映射，类型必须唯一存在于映射模块及其所属包规则中。嵌套成员类型使用点分名称。不推断领域业务语义。

ModelChange 增加可选 java 策略整体替换字段，省略沿用；null 不关闭已有策略。策略改变将全部模块加入影响种子，java_type 使用既有元素 upsert。旧模型省略新增字段时保持其序列化摘要。

Java 检查仅在模型启用策略时运行且要求 Maven。沿已解析 reactor 扫描各模块 src/main/java，可选 src/test/java；根不存在记入覆盖清单，空扫描不能通过。非标准源码根、生成源码、JPMS、Unicode 转义、无法解析源码、链接、超预算及不完整 Maven 模型拒绝或 BLOCK，不执行构建插件。

Tree-sitter Java 语法树提取 package、import（含 static 和 wildcard）、顶层/成员类型以及可识别的全限定源码引用。内部通配导入保守视为对目标包的引用；内部缺失导入目标拒绝通过。忽略注释和字符串伪引用。包依赖有源文件及行号；这是声明/显式引用图，不是完整编译器语义图。

包映射验证实际 Maven 模块，分层允许表和禁止前缀规则产生具体 finding，映射领域类型缺失/错误模块阻断。源码、文件清单、策略和分析器版本绑定报告，新增、删除和同图源码改动均使旧报告失效。检查前后重新核对清单与内容；原有写锁和基线发布流程复用。

候选报告新增 java_analysis；普通 check 将 Java 结果绑定事实摘要，违规/不完整转 partial 并阻断。模型影响分析继续保守全项目，源码分析不缩小契约检查。MCP 不增加写接口；插件引导先配置经过审阅的映射再检查修复复验，不自动弱化规则。
