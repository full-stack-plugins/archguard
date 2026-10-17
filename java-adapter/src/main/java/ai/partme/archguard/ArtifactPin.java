package ai.partme.archguard;
/** 显式工具原件摘要；调用者仍需外部批准整个配置。 */
public record ArtifactPin(String path, String sha256) { }
