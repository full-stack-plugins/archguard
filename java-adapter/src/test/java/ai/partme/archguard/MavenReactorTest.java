package ai.partme.archguard;

import org.junit.Test;
import org.junit.Rule;
import org.junit.rules.TemporaryFolder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.FileTime;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.TimeUnit;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;
import static org.junit.Assert.assertFalse;

/** 用已安装真实Maven编译双模块，再调用真实ArchUnit检查。 */
public class MavenReactorTest {
    @Rule public TemporaryFolder temporary = new TemporaryFolder();
    private Path fixture() throws Exception {
        Path source = Path.of(getClass().getClassLoader().getResource("reactor/pom.xml").toURI()).getParent();
        Path root = temporary.newFolder().toPath();
        try (var walk = Files.walk(source)) {
            for (Path path : walk.toList()) {
                Path destination = root.resolve(source.relativize(path));
                if (Files.isDirectory(path)) { Files.createDirectories(destination); }
                else { Files.copy(path, destination); }
            }
        }
        return root;
    }
    private void compile(Path root) throws Exception {
        Path maven = Path.of(System.getProperty("archguard.maven.home"));
        Path bootstrap;
        try (var list = Files.list(maven.resolve("boot"))) { bootstrap = list.filter(p -> p.getFileName().toString().startsWith("plexus-classworlds-") && p.toString().endsWith(".jar")).findFirst().orElseThrow(); }
        List<String> arguments = new ArrayList<>(List.of(Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java").toString(),
            "-Duser.home=" + System.getProperty("user.home"), "-Djava.io.tmpdir=" + System.getProperty("java.io.tmpdir"), "-Dfile.encoding=UTF-8",
            "-Dmaven.home=" + maven, "-Dmaven.multiModuleProjectDirectory=" + root,
            "-Dclassworlds.conf=" + maven.resolve("bin/m2.conf"), "-cp", bootstrap.toString(), "org.codehaus.plexus.classworlds.launcher.Launcher",
            "-o", "-B", "-ntp", "-Dmaven.repo.local=" + System.getProperty("archguard.maven.repository"), "compile"));
        Path output = root.resolve("maven.log");
        Process process = new ProcessBuilder(arguments).directory(root.toFile()).redirectErrorStream(true).redirectOutput(output.toFile()).start();
        assertTrue("Maven test fixture deadline", process.waitFor(90, TimeUnit.SECONDS));
        assertEquals(Files.readString(output), 0, process.exitValue());
    }
    private BytecodeRequest request(Path root) {
        return new BytecodeRequest(1, List.of(root.resolve("domain/target/classes").toString(), root.resolve("infra/target/classes").toString()),
            List.of("demo.domain.Order", "demo.infra.SqlStore"), List.of(new LayerRule("APPROVED-DOMAIN-NO-INFRA", "demo.domain", List.of("demo.infra"))));
    }
    @Test public void actualMavenAllowedForbiddenRepairAndIncomplete() throws Exception {
        Path root = fixture(); compile(root);
        assertEquals("complete", BytecodeChecker.evaluate(request(root)).completeness());
        assertTrue(BytecodeChecker.evaluate(request(root)).violations().isEmpty());
        Path order = root.resolve("domain/src/main/java/demo/domain/Order.java");
        Files.writeString(order, "package demo.domain; public class Order { public String id() { return new demo.infra.SqlStore().load(); } }");
        Files.setLastModifiedTime(order, FileTime.fromMillis(System.currentTimeMillis() + 2000)); compile(root);
        BytecodeResult forbidden = BytecodeChecker.evaluate(request(root));
        assertEquals("complete", forbidden.completeness()); assertFalse(forbidden.violations().isEmpty());
        assertEquals("APPROVED-DOMAIN-NO-INFRA", forbidden.violations().get(0).ruleId());
        assertEquals("Order.java", forbidden.violations().get(0).file()); assertTrue(forbidden.violations().get(0).line() > 0);
        Files.writeString(order, "package demo.domain; public class Order { public String id() { return \"repaired\"; } }");
        Files.setLastModifiedTime(order, FileTime.fromMillis(System.currentTimeMillis() + 4000)); compile(root);
        assertEquals("complete", BytecodeChecker.evaluate(request(root)).completeness());
        assertTrue(BytecodeChecker.evaluate(request(root)).violations().isEmpty());
        Files.delete(root.resolve("infra/target/classes/demo/infra/SqlStore.class"));
        assertEquals("incomplete", BytecodeChecker.evaluate(request(root)).completeness());
    }
}
