# Java 源码架构检查（0.5.0）

先读取 context 的 domain.model.java_source_configured 和 model show。已有基线在需求增量中通过 java 字段显式提供经审阅的完整 Java 策略，java_type 通过 upsert_elements 维护；不创建第二套模型。没有策略的旧模型继续只检查构建声明，不宣称 Java 源码已通过。

- java.packages 为 prefix/module/layer，按点边界最长前缀匹配，module 是模型稳定 ID。
- java.layers 为 id/allowed_dependencies；同层引用允许，跨层必须在允许表内。
- java.forbidden_dependencies 为 id/from/to，匹配包前缀，包含外部包禁止规则。
- java.include_tests 默认 false；范围为所有 Maven reactor 的 src/main/java，可显式加入 src/test/java。
- 启用后，aggregate/entity/value_object/interface/event 需要 java_type；采用 Java 全限定名，成员类型用点分名称。实际声明必须唯一存在于预期模块，包映射也必须与实际模块一致。

执行 change impact → change check。根据 java_analysis.findings 的 code/file/line 修复，而非删除 java 字段、扩大允许表或迁移映射来掩盖违规。策略变更必须有已确认需求依据。修复后执行业务测试，导出新报告并 change verify，最后在已有授权下 baseline update --apply。MCP 共用同一只读检查，不发布基线。

java_analysis.complete 表示声明范围提取完成，不等于 findings 为空。遵循顶层 decision；architecture_report 是构建子报告，其 ALLOW 不能盖过 Java BLOCK。普通 check 的 Java 违规表现为 INDETERMINATE/BLOCK，并在诊断中附证据。

语法树覆盖 import（包括 static/wildcard）、项目全限定类型/成员引用与包注解；通配引用与外部禁止前缀采用保守策略。Java 内容、文件清单、策略变化均需重新生成报告。新增文件不能靠未变的 POM 复用旧证据。

默认不包含测试源码；自定义/生成源码根、JPMS、有效 Unicode 转义和解析错误阻断，不能静默跳过。完整编译器绑定、简单名/类型推导的全部依赖、反射、动态调用、领域不变量不在覆盖范围。自动修复没有 CLI 入口；智能体仅在授权范围内修改源码并复验。
