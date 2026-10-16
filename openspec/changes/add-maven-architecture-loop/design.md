# 设计

Maven 静态分析不执行 mvn、构建插件或下载依赖。以 reactor 中模块的 groupId:artifactId 为实体 ID，depends_on 为直接声明依赖关系，包含本地父模型继承的声明依赖；test/provided/optional 等声明保守纳入图，dependencyManagement 本身不是依赖边。模块声明支持 modules/module 和 subprojects/subproject。属性递归解析有深度界限，重复坐标、模块逃逸、模型循环、缺失文件、无法解析坐标、影响依赖的 profile、未知外部父模型等产生 partial，不能 ALLOW。外部三方依赖不构成 reactor 内部图，但输入仍绑定整个 POM 内容。

新协议 v1alpha2 要求 coverage 的 entities、predicates、scope。规则引用不存在实体、不支持 predicate、缺 coverage 均 INDETERMINATE/BLOCK。图规则由独立 GuardEngine 执行，确定性排序，输出实际边链证据。v1alpha1 旧精确规则继续解析；旧事实缺覆盖不能用于 v2 放行。

check 默认发现 .archguard/contract.yaml，显式 --contract 优先；不自动生成或批准规范。auto 只在唯一构建入口时选择 Maven/Cargo，混合项目需指定。explain 重跑检查，提供规则、证据和修复方向。verify 重跑当前分析与规则，比较完整报告；源或契约漂移返回错误，保存报告仍是 unsigned。

CLI 输出路径不能覆盖输入源码或契约，默认输出 stdout；报告显式导出仅允许新建文件，避免破坏已有人工产物。MCP 不支持输出路径，仅返回数据；插件 package 在 archguard/plugin，不包含独立引擎。旧 context/instructions 与其他五个 Guard 保持兼容。

限制：静态声明图不是编译期有效依赖图；不扫描 Java import，也不把源码语义纳入 POM 快照。生产模型存在不支持部分时阻断并解释，后续增加可信 effective-model 适配器。
