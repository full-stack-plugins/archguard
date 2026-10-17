package ai.partme.archguard;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayInputStream;
import java.nio.ByteBuffer;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.Base64;
import java.util.List; import java.util.Objects;
import java.util.HashSet;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.jar.JarFile;
import java.util.zip.GZIPInputStream;
import static ai.partme.archguard.PipelineFiles.require;

/** K06调用的原生Java产品入口；stdout只有一个本地技术结果JSON。 */
public final class PipelineMain {
    private PipelineMain() { }
    static void verifyLaunch(PipelineRequest request) throws Exception {
        require(PipelineFiles.safe(Path.of(request.projectRoot())).equals(Path.of("").toRealPath()), "requested candidate differs from actual process cwd");
        Set<Path> pins = new HashSet<>();
        for (ArtifactPin pin : request.toolArtifacts()) { require(pins.add(PipelineFiles.safe(Path.of(pin.path()))), "duplicate tool launch pin"); }
        Path currentJava = Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java");
        require(PipelineFiles.safe(currentJava).equals(PipelineFiles.safe(Path.of(request.javaExecutable()))) && pins.contains(PipelineFiles.safe(currentJava)), "pipeline Java must match pinned running JVM");
        for (String item : System.getProperty("java.class.path").split(Pattern.quote(java.io.File.pathSeparator), -1)) {
            Path artifact = PipelineFiles.safe(Path.of(item));
            require(java.nio.file.Files.isRegularFile(artifact) && artifact.toString().endsWith(".jar") && pins.contains(artifact), "each launch classpath JAR must be explicitly pinned");
            try (JarFile jar = new JarFile(artifact.toFile())) {
                var manifest = jar.getManifest();
                require(Objects.isNull(manifest) || Objects.isNull(manifest.getMainAttributes().getValue("Class-Path")), "implicit manifest Class-Path unsupported");
            }
        }
    }
    static PipelineRequest decodePacked(String root, String encoded) throws Exception {
        require(Path.of(root).isAbsolute() && encoded.length() <= 24000, "absolute root and bounded packed configuration required");
        byte[] packed = Base64.getDecoder().decode(encoded); byte[] raw;
        try (var gzip = new GZIPInputStream(new ByteArrayInputStream(packed))) {
            raw = gzip.readNBytes(1024 * 1024 + 1); require(raw.length <= 1024 * 1024 && gzip.read() == -1, "decompressed request exceeds 1 MiB");
        }
        String json = StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(raw)).toString();
        PipelineRequest request = RequestJson.read(json, PipelineRequest.class);
        require("{{subject_root}}".equals(request.projectRoot()), "packed request must bind exact subject_root template");
        PipelineRequest rendered = new PipelineRequest(request.schemaVersion(), root, request.javaExecutable(), request.mavenHome(), request.repository(), request.home(), request.temporaryDirectory(), request.toolArtifacts(), request.modules(), request.rules());
        String renderedJson = new ObjectMapper().writeValueAsString(rendered);
        require(!renderedJson.contains("{{") && !renderedJson.contains("}}"), "only projectRoot subject_root template is supported"); return rendered;
    }
    /** 参数pipeline JSON或pipeline-packed ROOT GZIP_BASE64_JSON；不签发受信资格。 */
    public static void main(String[] args) throws Exception {
        PipelineResult result;
        try {
            PipelineRequest request;
            if (args.length == 2 && args[0].equals("pipeline")) {
                require(args[1].length() <= 24000, "inline request exceeds portable argv budget; use pipeline-packed"); request = RequestJson.read(args[1], PipelineRequest.class);
            } else if (args.length == 3 && args[0].equals("pipeline-packed")) { request = decodePacked(args[1], args[2]); }
            else { throw new IllegalArgumentException("expected pipeline JSON or pipeline-packed ROOT GZIP_BASE64_JSON"); }
            verifyLaunch(request);
            result = MavenPipeline.evaluate(request);
        } catch (Exception | AssertionError failure) {
            result = new PipelineResult(1, "incomplete", 0, List.of(), List.of(failure.getClass().getSimpleName() + ": " + failure.getMessage()), null, null, null, null, "archguard-bytecode/1", "1.5.1", List.of());
        }
        System.out.println(new ObjectMapper().writeValueAsString(result));
        System.exit(result.completeness().equals("complete") ? (result.violations().isEmpty() ? 0 : 2) : 4);
    }
}
