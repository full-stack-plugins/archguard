package ai.partme.archguard;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import java.util.TreeMap;
import java.util.Objects;
import static ai.partme.archguard.PipelineFiles.require;

/** 固定Maven发行文件与离线仓库编译依赖，前后各复验一次。 */
final class PipelineTools {
    private PipelineTools() { }
    static Map<String, String> verify(PipelineRequest request) throws Exception {
        require(Objects.nonNull(request.toolArtifacts()) && !request.toolArtifacts().isEmpty() && request.toolArtifacts().size() <= 8192, "bounded tool artifact pins required");
        Map<String, String> pins = new TreeMap<>();
        for (ArtifactPin pin : request.toolArtifacts()) {
            require(Objects.nonNull(pin) && Objects.nonNull(pin.sha256()) && pin.sha256().matches("sha256:[a-f0-9]{64}"), "invalid tool pin");
            String path = PipelineFiles.safe(Path.of(pin.path())).toString();
            require(Objects.isNull(pins.put(path, pin.sha256())), "duplicate tool pin");
            require(PipelineFiles.hash(Path.of(path), 512L * 1024 * 1024).equals(pin.sha256()), "tool artifact pin mismatch: " + path);
        }
        require(pins.containsKey(PipelineFiles.safe(Path.of(request.javaExecutable())).toString()), "Java executable pin missing");
        Path maven = PipelineFiles.safe(Path.of(request.mavenHome()));
        for (String relative : PipelineFiles.inventory(maven, 64L * 1024 * 1024, 512L * 1024 * 1024).keySet()) {
            require(pins.containsKey(PipelineFiles.safe(maven.resolve(relative)).toString()), "Maven distribution pin missing: " + relative);
        }
        Path repository = PipelineFiles.safe(Path.of(request.repository()));
        try (var walk = Files.walk(repository, 64)) {
            var iterator = walk.iterator(); int count = 0;
            while (iterator.hasNext()) {
                Path file = iterator.next(); require(++count <= 50000, "repository traversal budget exceeded"); PipelineFiles.safe(file);
                if (Files.isRegularFile(file) && (file.toString().endsWith(".jar") || file.toString().endsWith(".pom"))) {
                    require(pins.containsKey(file.toRealPath().toString()), "offline repository artifact pin missing: " + file.getFileName());
                }
            }
        }
        return pins;
    }
}
