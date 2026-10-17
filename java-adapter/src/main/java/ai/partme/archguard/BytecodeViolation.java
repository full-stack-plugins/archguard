package ai.partme.archguard;
/** ArchUnit 实际评估得到的可定位依赖边。 */
public record BytecodeViolation(String ruleId, String from, String to, String file, int line, String description) { }
