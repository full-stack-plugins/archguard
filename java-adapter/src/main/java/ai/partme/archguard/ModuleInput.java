package ai.partme.archguard;
import java.util.List;
/** 原项目相对模块路径及编译前冻结的声明类型集合。 */
public record ModuleInput(String path, List<String> expectedTypes) { }
