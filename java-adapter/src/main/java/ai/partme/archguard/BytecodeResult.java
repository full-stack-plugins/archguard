package ai.partme.archguard;
import java.util.List;
/** 纯技术检查结果，不签发许可，也不代表受信质量通过。 */
public record BytecodeResult(int schemaVersion, String completeness, int classCount, List<BytecodeViolation> violations, List<String> diagnostics) { }
