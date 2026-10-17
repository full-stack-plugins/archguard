package ai.partme.archguard;
import org.junit.Rule;
import org.junit.Test;
import org.junit.rules.TemporaryFolder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertFalse;

/** 同一私有候选目录内真实编译再验证，不复用旧target。 */
public class PipelineTest {
    @Rule public TemporaryFolder temporary = new TemporaryFolder();
    private PipelineRequest request(boolean forbidden) throws Exception {
        Path source = Path.of(getClass().getClassLoader().getResource("reactor/pom.xml").toURI()).getParent();
        Path root = temporary.newFolder().toPath();
        try (var walk = Files.walk(source)) {
            for (Path path : walk.toList()) {
                Path target = root.resolve(source.relativize(path));
                if (Files.isDirectory(path)) { Files.createDirectories(target); } else { Files.copy(path, target); }
            }
        }
        if (forbidden) { Files.writeString(root.resolve("domain/src/main/java/demo/domain/Order.java"), "package demo.domain; public class Order { public String id() { return new demo.infra.SqlStore().load(); } }"); }
        Path maven = Path.of(System.getProperty("archguard.maven.home"));
        Path java = Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java");
        List<ArtifactPin> pins = new ArrayList<>();
        pins.add(new ArtifactPin(java.toString(), "sha256:" + HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(java)))));
        try (var walk = Files.walk(maven)) {
            for (Path file : walk.filter(Files::isRegularFile).toList()) {
                pins.add(new ArtifactPin(file.toString(), "sha256:" + HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(file)))));
            }
        }
        try (var walk = Files.walk(Path.of(System.getProperty("archguard.maven.repository")))) {
            for (Path file : walk.filter(Files::isRegularFile).filter(p -> p.toString().endsWith(".jar") || p.toString().endsWith(".pom")).toList()) {
                pins.add(new ArtifactPin(file.toString(), "sha256:" + HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(file)))));
            }
        }
        return new PipelineRequest(1, root.toString(), java.toString(), maven.toString(), System.getProperty("archguard.maven.repository"), System.getProperty("user.home"), System.getProperty("java.io.tmpdir"), pins,
            List.of(new ModuleInput("", List.of()), new ModuleInput("infra", List.of("demo.infra.SqlStore")), new ModuleInput("domain", List.of("demo.domain.Order"))),
            List.of(new LayerRule("DOMAIN-NO-INFRA", "demo.domain", List.of("demo.infra"))));
    }
    @Test public void sameCandidateFreshCompileAndBytecodeEvaluate() throws Exception {
        PipelineRequest allowed = request(false);
        PipelineResult result = MavenPipeline.evaluate(allowed);
        assertEquals(result.diagnostics().toString(), "complete", result.completeness()); assertTrue(result.violations().isEmpty());
        assertTrue(result.sourceDigest().startsWith("sha256:")); assertTrue(result.bytecodeDigest().startsWith("sha256:"));
        // 同一物化副本第二次调用已有target；拒绝而不是默默复用旧class。
        assertEquals("incomplete", MavenPipeline.evaluate(allowed).completeness());
        PipelineResult forbidden = MavenPipeline.evaluate(request(true));
        assertEquals(forbidden.diagnostics().toString(), "complete", forbidden.completeness()); assertFalse(forbidden.violations().isEmpty());
        assertEquals("domain/src/main/java/demo/domain/Order.java", forbidden.violations().get(0).file());
    }
    @Test public void changedToolPinAndMissingExpectedTypeBlock() throws Exception {
        PipelineRequest original = request(false);
        List<ArtifactPin> bad = new ArrayList<>(original.toolArtifacts());
        bad.set(0, new ArtifactPin(original.javaExecutable(), "sha256:" + "0".repeat(64)));
        PipelineRequest tampered = new PipelineRequest(1, original.projectRoot(), original.javaExecutable(), original.mavenHome(), original.repository(), original.home(), original.temporaryDirectory(), bad, original.modules(), original.rules());
        assertEquals("incomplete", MavenPipeline.evaluate(tampered).completeness());
        assertFalse(Files.exists(Path.of(original.projectRoot()).resolve("domain/target")));
        List<ModuleInput> missing = List.of(new ModuleInput("", List.of()), new ModuleInput("infra", List.of("demo.infra.Missing")), new ModuleInput("domain", List.of("demo.domain.Order")));
        PipelineRequest incomplete = new PipelineRequest(1, original.projectRoot(), original.javaExecutable(), original.mavenHome(), original.repository(), original.home(), original.temporaryDirectory(), original.toolArtifacts(), missing, original.rules());
        assertEquals("incomplete", MavenPipeline.evaluate(incomplete).completeness());
    }
    @Test public void actualMavenCompilerFailureCannotPass() throws Exception {
        PipelineRequest input = request(false);
        Files.writeString(Path.of(input.projectRoot()).resolve("domain/src/main/java/demo/domain/Order.java"), "package demo.domain; public class Order { BROKEN JAVA }");
        PipelineResult result = MavenPipeline.evaluate(input);
        assertEquals("incomplete", result.completeness());
        assertTrue(result.diagnostics().toString().contains("Maven compilation failed"));
    }
    @Test public void externalSystemJarMustNotBecomeUnpinnedCompileInput() throws Exception {
        PipelineRequest input = request(false);
        Path external = temporary.newFolder().toPath(); Path source = external.resolve("Marker.java");
        Files.writeString(source, "package external; public class Marker { public String id() { return \"external\"; } }");
        assertEquals(0, javax.tools.ToolProvider.getSystemJavaCompiler().run(null, null, null, "-d", external.toString(), source.toString()));
        Path jar = external.resolve("external.jar");
        try (var output = new java.util.jar.JarOutputStream(Files.newOutputStream(jar))) {
            output.putNextEntry(new java.util.jar.JarEntry("external/Marker.class")); output.write(Files.readAllBytes(external.resolve("external/Marker.class"))); output.closeEntry();
        }
        Path root = Path.of(input.projectRoot()), pom = root.resolve("domain/pom.xml");
        String dependency = "<dependency><groupId>external</groupId><artifactId>outside</artifactId><version>1</version><scope>system</scope><systemPath>" + jar.toString().replace('\\', '/') + "</systemPath></dependency>";
        Files.writeString(pom, Files.readString(pom).replace("</dependencies>", dependency + "</dependencies>"));
        Files.writeString(root.resolve("domain/src/main/java/demo/domain/Order.java"), "package demo.domain; public class Order { public String id() { return new external.Marker().id(); } }");
        assertEquals("incomplete", MavenPipeline.evaluate(input).completeness());
    }
    @Test public void parentOutsideCandidateMustNotBecomeUnpinnedCompileInput() throws Exception {
        PipelineRequest input = request(false); Path root = Path.of(input.projectRoot());
        Path outside = temporary.newFolder().toPath().resolve("parent.xml");
        Files.writeString(outside, "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><groupId>external</groupId><artifactId>parent</artifactId><version>1</version><packaging>pom</packaging><properties><maven.compiler.release>17</maven.compiler.release></properties><build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-resources-plugin</artifactId><version>3.3.1</version></plugin><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><version>3.13.0</version></plugin></plugins></build></project>");
        String relative = root.resolve("infra").relativize(outside).toString().replace('\\', '/');
        Files.writeString(root.resolve("infra/pom.xml"), "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><parent><groupId>external</groupId><artifactId>parent</artifactId><version>1</version><relativePath>" + relative + "</relativePath></parent><groupId>demo</groupId><artifactId>infra</artifactId></project>");
        assertEquals("incomplete", MavenPipeline.evaluate(input).completeness());
    }
}
