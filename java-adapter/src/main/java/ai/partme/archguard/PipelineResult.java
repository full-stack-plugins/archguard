package ai.partme.archguard;
import java.util.List;
/** 绑定当前源、字节码与工具的技术结论；早期失败摘要可为空。 */
public record PipelineResult(int schemaVersion, String completeness, int classCount, List<BytecodeViolation> violations, List<String> diagnostics,
    String sourceDigest, String bytecodeDigest, String toolDigest, String ruleDigest, String adapterVersion, String archunitVersion, List<String> modules) { }
