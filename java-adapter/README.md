# ArchGuard Java 字节码适配器

这是 EG-A01 的专业验证器，不是通用执行内核。采用 ArchUnit core **1.5.1**，实际导入本次编译的 class 文件并调用 `ArchRule.evaluate`。Maven/JDK 必须事先存在；适配器不安装工具、不下载依赖、不写原仓、不自报批准。

## 执行入口

`ai.partme.archguard.PipelineMain` 支持：

```text
pipeline JSON
pipeline-packed ACTUAL_SUBJECT_ROOT GZIP_BASE64_JSON
```

普通 JSON 参数至多 24000 字符。压缩形式使用标准 Base64 编码 GZIP，编码至多 24000 字符，解压 UTF-8 至多 1 MiB；JSON 嵌套深度最多 32。拒绝重复键、未知字段、尾随 JSON、数值字符串强转和非法 UTF-8。

压缩配置的 `projectRoot` 必须精确为 `{{subject_root}}`，只通过独立根参数替换此字段。其他字段不允许模板。原生档位固定压缩原件及根参数模板；不能把临时随机路径写入批准配置。实际根必须等于当前进程工作目录。启动 classpath 每项必须是明确 pin 的普通 JAR，拒绝目录及 JAR manifest 隐式 `Class-Path`。

`PipelineRequest` 是闭合的技术输入：

| 字段 | 含义 |
|---|---|
| schemaVersion | 固定 `1` |
| projectRoot | 当前私有候选实际根 |
| javaExecutable / mavenHome | 显式绝对 Java 程序及 Maven 发行目录 |
| repository / home / temporaryDirectory | 项目外的显式已有缓存、用户目录与临时目录 |
| toolArtifacts | `{path, sha256}` 原件清单；摘要为 `sha256:` 加小写十六进制 |
| modules | `{path, expectedTypes}` 完整 reactor；根模块 path 为空串，聚合模块类型集合为空 |
| rules | `{ruleId, from, forbiddenTo}` 精确包前缀规则；点边界匹配 |

工具清单前后复验全部声明摘要，强制包含 Java 程序、Maven 发行目录全部普通文件、专用离线仓库全部 JAR/POM。调用方还必须把适配器、ArchUnit、Jackson、SLF4J 全部运行 JAR 加入清单。Java 程序摘要不等于整个 JDK 运行时文件闭包；宿主需要自行明确并固定 JDK 镜像或模块/本机库原件，不得把本地技术工具清单当成生产信任证明。

Maven 子进程由显式 Java 启动单一 classworlds 引导 JAR，使用本次候选根、固定字面参数、离线专用仓库和适配器生成的空 user/global settings。宿主 Maven 镜像和用户凭据不参与解析。注解处理关闭。Maven 输出流转发到适配器 stderr，stdout 保留唯一 JSON；进程树、总时限和输出预算由 K06 Runtime/Runner 负责。

## 覆盖边界

支持默认 `src/main/java`、`target/classes` 和标准 compiler 默认 compile 执行的完整 Maven reactor。`expectedTypes` 是批准的明确 class 全集，包含实际内部/匿名类时须显式使用 JVM `$` 名字；不是自动推断的全项目语义承诺。修改方法体不改变此配置，增加类型需要审阅类型范围。

已有 `target` 必须拒绝，不能 `clean` 删除原先冻结的输入。已有 `.mvn`、profiles、自定义源码/输出目录、额外 executions/extensions、includes/excludes、注解处理器、compilerArgs、外部编译器 executable/fork/toolchain、systemPath/system scope 等超出首批支持范围，返回 incomplete。非空 parent relativePath 只允许本次明确 reactor 中的 POM，不能越出候选或指向未核对的模型；显式空 relativePath 才允许从已 pin 离线仓库解析父 POM。默认 reactor 模块路径、编译器 inputFiles/createdFiles 清单、磁盘 class 集合和明确预期集合必须一致；空源码模块出现意外 class 也拒绝。不能只导入 domain 而漏掉 infra。

每个违规包含规则 ID、from/to 全限定类型、当前项目相对源码路径、行号和 ArchUnit 原始描述。字节码没有精确行表的位置保留行号 `0` 表示未知，不编造行号。非公共顶层类型与源文件名不同等不能准确映射的情形返回 incomplete。JDK/外部类型可以是占位类型；明确项目类型不能缺失。反射、运行时动态调用和业务不变量不在本检查范围。

结果 `completeness=complete` 仍可能有违规。失败保留此前已确认的违规，不能因为后续规则不完整抹掉阳性。退出码：完整无违规 `0`、完整有违规 `2`、不完整 `4`。结果不包含自报受信、授权或 `qualityPass`。

`sourceDigest`、`bytecodeDigest`、`toolDigest` 是排序清单摘要：按 Java UTF-16 键顺序逐项，把键和值的 UTF-8 字节分别加 8 字节大端长度前缀，再 SHA-256。源码键是原始项目相对路径；字节码键是 `module/target/classes/文件`；工具键是 canonical 绝对路径。`ruleDigest` 是 Java record 字段顺序 `ruleId/from/forbiddenTo` 的数组 JSON UTF-8 摘要。这些是本适配器技术摘要，不声称为公共协议 JCS；公共契约及批准配置仍使用公共协议规范摘要。

## 本地验证

已使用 Windows、JDK 17.0.12、Maven 3.9.9、ArchUnit 1.5.1 真实执行 19 项测试，无跳过。包含真实双模块 Maven 合法/禁止依赖、修复、缺件、编译失败、保留前序阳性、不完整类、JSON 与压缩预算负例。外部 system JAR 被源码实际调用、外部相对父 POM 这两项曾真实编译成功并错误返回 complete；测试实际失败后增加输入闭包限制，最终拒绝并保留普通 reactor 正例。未据此宣称 JDK 8、Unix 或完整 A01 产品接线已验收。

编译时设置 JAVA_HOME、TEMP、TMP、`-Duser.home`、`-Djava.io.tmpdir`、`-Dmaven.repo.local`、`-Darchguard.build.directory` 到明确 D/E 位置，并明确 `-s` / `-gs`，不继承宿主镜像或凭据。生产适配器始终离线；准备缓存是独立的显式正常 Maven 项目依赖解析步骤。

固定版本依赖的 14 个 JAR/POM 摘要保存在 `dependencies.lock.json`。只读核验：

```text
python -B java-adapter/scripts/dependency_lock.py --repository E:/明确的专用仓库 --check
```

脚本不会联网或安装依赖；去掉 `--check` 会重写锁文件，仅用于明确审阅后的依赖变更。
