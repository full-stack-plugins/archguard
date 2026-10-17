package ai.partme.archguard;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.TreeMap;
import java.util.Objects;
import static ai.partme.archguard.PipelineFiles.require;

/** 一个受管Java进程内编译当前私有候选再评估；总树预算由K06负责。 */
public final class MavenPipeline {
    private MavenPipeline() { }
    private static String digest(byte[] input) throws Exception { return "sha256:" + HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(input)); }
    private static void compile(PipelineRequest request, Path root) throws Exception {
        Path maven = PipelineFiles.safe(Path.of(request.mavenHome()));
        Path boot;
        try (var files = Files.list(maven.resolve("boot"))) {
            List<Path> matches = files.filter(p -> p.getFileName().toString().startsWith("plexus-classworlds-") && p.toString().endsWith(".jar")).toList();
            require(matches.size() == 1, "exact Maven classworlds bootstrap required"); boot = matches.get(0);
        }
        Path build = Files.createDirectory(root.resolve(".archguard-bytecode-build"));
        Path settings = build.resolve("settings.xml");
        Files.writeString(settings, "<settings xmlns=\"http://maven.apache.org/SETTINGS/1.2.0\"/>", StandardCharsets.UTF_8);
        List<String> args = List.of(request.javaExecutable(), "-Duser.home=" + request.home(), "-Djava.io.tmpdir=" + request.temporaryDirectory(), "-Dfile.encoding=UTF-8",
            "-Dmaven.home=" + maven, "-Dmaven.multiModuleProjectDirectory=" + root, "-Dclassworlds.conf=" + maven.resolve("bin/m2.conf"),
            "-cp", boot.toString(), "org.codehaus.plexus.classworlds.launcher.Launcher", "-o", "-B", "-ntp", "-s", settings.toString(), "-gs", settings.toString(),
            "-Dmaven.repo.local=" + request.repository(), "-Dmaven.compiler.proc=none", "-Dmaven.compiler.debug=true", "compile");
        ProcessBuilder builder = new ProcessBuilder(args).directory(root.toFile()).redirectErrorStream(true);
        for (String key : List.of("JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "JDK_JAVA_OPTIONS", "MAVEN_OPTS", "MAVEN_ARGS")) { builder.environment().remove(key); }
        Process process = builder.start();
        // 原始Maven双流合并输出到adapter stderr，stdout保留一个JSON；不另实现超时执行器。
        process.getInputStream().transferTo(System.err); require(process.waitFor() == 0, "Maven compilation failed");
    }
    /** 真编译、产物完整性和真实ArchUnit；输入错误均返回不完整。 */
    public static PipelineResult evaluate(PipelineRequest request) {
        String source = null, bytecode = null, tools = null, rules = null;
        BytecodeResult checked = new BytecodeResult(1, "incomplete", 0, List.of(), List.of());
        List<String> modules = new ArrayList<>();
        try {
            require(Objects.nonNull(request) && request.schemaVersion() == 1, "unknown pipeline request version");
            Path root = PipelineFiles.safe(Path.of(request.projectRoot())); require(Files.isDirectory(root), "candidate directory required");
            for (String external : List.of(request.repository(), request.home(), request.temporaryDirectory(), request.mavenHome())) {
                Path path = PipelineFiles.safe(Path.of(external)); require(Files.isDirectory(path) && !path.startsWith(root), "external tool/cache directory must be outside candidate");
            }
            Map<String, String> pins = PipelineTools.verify(request); tools = PipelineFiles.manifestDigest(pins);
            PipelineProject.validate(root, request.modules());
            Map<String, String> originals = PipelineFiles.inventory(root, 4L * 1024 * 1024, 64L * 1024 * 1024);
            source = PipelineFiles.manifestDigest(originals);
            require(Objects.nonNull(request.rules()) && request.rules().size() <= 1024, "bounded rules required");
            rules = digest(new ObjectMapper().writeValueAsBytes(request.rules()));
            compile(request, root);
            List<String> classRoots = new ArrayList<>(), expected = new ArrayList<>();
            Map<String, String> compiled = new TreeMap<>(); Map<String, String> locations = new TreeMap<>();
            for (ModuleInput module : request.modules()) {
                modules.add(module.path()); Path directory = root.resolve(module.path());
                if (module.expectedTypes().isEmpty()) {
                    require(!Files.isDirectory(directory.resolve("src/main/java")), "source-bearing module requires explicit expected class set");
                    Path unlisted = directory.resolve("target/classes");
                    if (Files.isDirectory(unlisted)) {
                        require(PipelineFiles.inventory(unlisted, 4L * 1024 * 1024, 64L * 1024 * 1024).keySet().stream().noneMatch(p -> p.endsWith(".class")), "unexpected classes in empty module");
                    }
                    continue;
                }
                Path classes = PipelineFiles.safe(directory.resolve("target/classes")); classRoots.add(classes.toString());
                Path status = PipelineFiles.safe(directory.resolve("target/maven-status/maven-compiler-plugin/compile/default-compile"));
                require(Files.size(status.resolve("createdFiles.lst")) <= 1024 * 1024 && Files.size(status.resolve("inputFiles.lst")) <= 1024 * 1024, "compiler manifest budget exceeded");
                Set<String> actual = new HashSet<>();
                for (String output : Files.readAllLines(status.resolve("createdFiles.lst"))) {
                    String normalized = output.replace('\\', '/');
                    require(normalized.endsWith(".class") && !normalized.contains("..") && !Path.of(normalized).isAbsolute(), "invalid compiler output manifest");
                    actual.add(normalized.substring(0, normalized.length() - 6).replace('/', '.'));
                }
                require(actual.equals(new HashSet<>(module.expectedTypes())) && module.expectedTypes().size() == actual.size(), "compiled types differ from explicit approved type set");
                Set<Path> consumed = new HashSet<>();
                for (String input : Files.readAllLines(status.resolve("inputFiles.lst"))) { Path path = PipelineFiles.safe(Path.of(input)); require(path.startsWith(root), "compiler read outside candidate"); consumed.add(path); }
                Set<Path> allSources = new HashSet<>();
                Path sourceRoot = PipelineFiles.safe(directory.resolve("src/main/java"));
                for (String relative : originals.keySet()) {
                    Path original = root.resolve(relative);
                    if (original.startsWith(sourceRoot) && relative.endsWith(".java")) { allSources.add(original.toRealPath()); }
                }
                require(consumed.equals(allSources) && !consumed.isEmpty(), "compiler input coverage differs from source manifest");
                for (String name : actual) {
                    expected.add(name);
                    String outer = name.split("\\$", 2)[0]; String relativeSource = module.path() + (module.path().isEmpty() ? "" : "/") + "src/main/java/" + outer.replace('.', '/') + ".java";
                    require(originals.containsKey(relativeSource), "source location for compiled type is not an original input"); locations.put(name, relativeSource);
                }
                for (var entry : PipelineFiles.inventory(classes, 4L * 1024 * 1024, 64L * 1024 * 1024).entrySet()) {
                    compiled.put(module.path() + "/target/classes/" + entry.getKey(), entry.getValue());
                }
            }
            bytecode = PipelineFiles.manifestDigest(compiled);
            checked = BytecodeChecker.evaluate(new BytecodeRequest(1, classRoots, expected, request.rules()));
            List<BytecodeViolation> located = new ArrayList<>();
            for (BytecodeViolation edge : checked.violations()) {
                require(locations.containsKey(edge.from()), "violation origin source mapping missing");
                located.add(new BytecodeViolation(edge.ruleId(), edge.from(), edge.to(), locations.get(edge.from()), edge.line(), edge.description()));
            }
            checked = new BytecodeResult(1, checked.completeness(), checked.classCount(), List.copyOf(located), checked.diagnostics());
            for (var input : originals.entrySet()) { require(PipelineFiles.hash(root.resolve(input.getKey()), 4L * 1024 * 1024).equals(input.getValue()), "source input changed during compilation"); }
            require(PipelineTools.verify(request).equals(pins), "tool closure changed during compilation");
        } catch (Exception | AssertionError failure) {
            checked = new BytecodeResult(1, "incomplete", checked.classCount(), checked.violations(), List.of(failure.getClass().getSimpleName() + ": " + failure.getMessage()));
        }
        return new PipelineResult(1, checked.completeness(), checked.classCount(), checked.violations(), checked.diagnostics(), source, bytecode, tools, rules, "archguard-bytecode/1", "1.5.1", List.copyOf(modules));
    }
}
