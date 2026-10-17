package ai.partme.archguard;
import java.util.List;
/** 精确包前缀禁止边；此技术 DTO 自身不构成批准。 */
public record LayerRule(String ruleId, String from, List<String> forbiddenTo) { }
