package ai.partme.archguard;

import org.junit.Test;
import org.junit.Rule;
import org.junit.rules.TemporaryFolder;
import javax.tools.ToolProvider;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

/** 验证真实 javac 字节码，不以模拟图代替 ArchUnit。 */
public class BytecodeCheckerTest {
    @Rule public TemporaryFolder temporary = new TemporaryFolder();

    private Path compile(boolean forbidden) throws Exception {
        Path root = temporary.newFolder().toPath();
        Path classes = Files.createDirectory(root.resolve("classes"));
        Path domain = root.resolve("Order.java");
        Path infra = root.resolve("SqlStore.java");
        Files.writeString(domain, "package demo.domain; public class Order { " + (forbidden ? "public demo.infra.SqlStore store;" : "public String id;") + " }");
        Files.writeString(infra, "package demo.infra; public class SqlStore { public demo.domain.Order order; }");
        assertEquals(0, ToolProvider.getSystemJavaCompiler().run(null, null, null, "-g", "-d", classes.toString(), domain.toString(), infra.toString()));
        return classes;
    }
    private BytecodeRequest request(Path classes) {
        return new BytecodeRequest(1, List.of(classes.toString()), List.of("demo.domain.Order", "demo.infra.SqlStore"),
            List.of(new LayerRule("DOMAIN-NO-INFRA", "demo.domain", List.of("demo.infra"))));
    }
    @Test public void allowedRealBytecodePasses() throws Exception {
        BytecodeResult result = BytecodeChecker.evaluate(request(compile(false)));
        assertEquals("complete", result.completeness()); assertTrue(result.violations().isEmpty());
        assertEquals(2, result.classCount());
    }
    @Test public void forbiddenRealBytecodeLocatesRuleEdgeAndSource() throws Exception {
        BytecodeResult result = BytecodeChecker.evaluate(request(compile(true)));
        assertEquals("complete", result.completeness()); assertFalse(result.violations().isEmpty());
        var edge = result.violations().get(0);
        assertEquals("DOMAIN-NO-INFRA", edge.ruleId()); assertEquals("demo.domain.Order", edge.from());
        assertEquals("demo.infra.SqlStore", edge.to()); assertEquals("Order.java", edge.file());
        assertTrue(edge.line() >= 0);
    }
    @Test public void absentClassCannotBecomePassingStub() throws Exception {
        Path classes = compile(true); Files.delete(classes.resolve("demo/infra/SqlStore.class"));
        assertEquals("incomplete", BytecodeChecker.evaluate(request(classes)).completeness());
    }
    @Test public void corruptClassAndEmptySelectionAreIncomplete() throws Exception {
        Path classes = compile(false); Files.write(classes.resolve("demo/infra/SqlStore.class"), new byte[]{1,2,3});
        assertEquals("incomplete", BytecodeChecker.evaluate(request(classes)).completeness());
        assertEquals("incomplete", BytecodeChecker.evaluate(new BytecodeRequest(1, List.of(classes.toString()), List.of("demo.domain.Order"), List.of(new LayerRule("EMPTY", "missing", List.of("demo.infra"))))).completeness());
    }
    @Test public void repairRequiresNewBytecodeEvaluation() throws Exception {
        assertFalse(BytecodeChecker.evaluate(request(compile(true))).violations().isEmpty());
        assertTrue(BytecodeChecker.evaluate(request(compile(false))).violations().isEmpty());
    }
    @Test public void emptyRuleSelectionFailsOnOtherwiseValidClasses() throws Exception {
        Path classes = compile(false);
        assertEquals("incomplete", BytecodeChecker.evaluate(new BytecodeRequest(1, List.of(classes.toString()), List.of("demo.domain.Order", "demo.infra.SqlStore"), List.of(new LayerRule("EMPTY", "demo.missing", List.of("demo.infra"))))).completeness());
    }
    @Test public void prefixBoundaryDoesNotMatchSimilarPackage() throws Exception {
        Path classes = compile(true);
        BytecodeRequest input = new BytecodeRequest(1, List.of(classes.toString()), List.of("demo.domain.Order", "demo.infra.SqlStore"), List.of(new LayerRule("BOUNDARY", "demo.domain", List.of("demo.infr"))));
        assertEquals("complete", BytecodeChecker.evaluate(input).completeness());
        assertTrue(BytecodeChecker.evaluate(input).violations().isEmpty());
    }
    @Test public void laterIncompleteRuleKeepsEarlierCertainViolation() throws Exception {
        Path classes = compile(true);
        BytecodeRequest input = new BytecodeRequest(1, List.of(classes.toString()), List.of("demo.domain.Order", "demo.infra.SqlStore"),
            List.of(new LayerRule("FIRST", "demo.domain", List.of("demo.infra")), new LayerRule("MISSING", "demo.missing", List.of("demo.infra"))));
        BytecodeResult result = BytecodeChecker.evaluate(input);
        assertEquals("incomplete", result.completeness());
        assertFalse("confirmed positive must survive later incomplete rule", result.violations().isEmpty());
        assertEquals("FIRST", result.violations().get(0).ruleId());
    }
}
