package ai.partme.archguard;
import java.util.List;
/** 同一私有候选内的固定原生流水线配置，不签发授权。 */
public record PipelineRequest(int schemaVersion, String projectRoot, String javaExecutable, String mavenHome, String repository, String home, String temporaryDirectory, List<ArtifactPin> toolArtifacts, List<ModuleInput> modules, List<LayerRule> rules) { }
