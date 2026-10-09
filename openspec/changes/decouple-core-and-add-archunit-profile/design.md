# Design: 独立架构核心与成熟检查器适配

状态：待用户审阅；对应 [proposal](proposal.md)。唯一详细设计来源为 [架构契约与检查器短设计](../../../docs/superpowers/specs/2026-10-09-architecture-contract-and-checker-adapters-design.md)，本文件固定本次OpenSpec变更的关键决策。

## Decisions

1. 默认核心拥有架构契约、源码分析、规则/覆盖结果和纯架构baseline；GG候选提取及SG追溯进入独立可选集成包。不仅给path依赖加optional：核心必须在无外部sibling目录的干净环境从分发物构建、运行。
2. 集成包保持真实Git对象文件与来源清单一致性，使用既有中立绑定契约；自报OID不得成为已验证输入。普通目录结果不冒充候选证据。纯baseline与SG/GG trace逻辑分开迁移。
3. 保留现有Cargo、Java、TS分析，以成熟工具适配增加能力。ArchUnit负责其明确支持的Java字节码架构规则；目录规则由核心清单检查；无法判定的DDD职责/框架语义明确需评审或未验证。
4. 目标项目持有精确依赖版本、wrapper/JDK配置、架构测试与明确入口。扫描不自动安装或初始化，不重写规则或baseline消除失败；存量例外需用户确认具体范围。
5. 第一条采用真实order-service、Java17/Maven/JUnit5/ArchUnit组合；具体兼容版本在实施计划验证固定，当前工具未安装。新适配结果为0通过、2违规、4未验证/执行错误，保留原CLI既有语义。

## First Acceptance Boundary

在无.git项目上完成：正常分层通过；只让OrderController引用JdbcOrderRepository且仍可编译，ARCH-JAVA-001真实失败；修复该边后同规则通过。验证非零测试/类导入，以及缺工具、编译失败、空覆盖、不支持规则均非成功。独立发行构建、已有分析能力回归、可选候选防伪验收见短设计第6节。不得用生成配置或文档检查代替实际执行。

## Follow-on and Migration Limits

TSArch仅后续固定TS项目的依赖/目录/环与覆盖边界；Rust先保留Cargo声明分析，再选择成熟工具验证。各工具独立，不承诺统一官方插件。迁移仅改变核心与集成边界，不删除分析实现、不改CodeGuard、不新增队列/签名系统。

当前GE仍有路径开发依赖，核心独立分发尚未实现；精确发行依赖未可用时，自包含固定源码构建包必须明确标识，不能宣称已发布独立crate。现有GG/SG API调用者需要显式迁移至可选包。这些是后续实施计划必须处理的前置项，不是本轮完成声明。
