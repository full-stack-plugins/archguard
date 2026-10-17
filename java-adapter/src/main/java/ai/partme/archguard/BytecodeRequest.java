package ai.partme.archguard;
import java.util.List;
/** 显式编译范围和预期类型全集；不包含自报受信字段。 */
public record BytecodeRequest(int schemaVersion, List<String> classDirectories, List<String> expectedClasses, List<LayerRule> rules) { }
