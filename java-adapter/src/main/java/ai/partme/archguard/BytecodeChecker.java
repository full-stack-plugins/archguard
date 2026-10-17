package ai.partme.archguard;

import com.tngtech.archunit.ArchConfiguration;
import com.tngtech.archunit.base.DescribedPredicate;
import com.tngtech.archunit.core.domain.Dependency;
import com.tngtech.archunit.core.domain.JavaClass;
import com.tngtech.archunit.core.domain.JavaClasses;
import com.tngtech.archunit.core.importer.ClassFileImporter;
import com.tngtech.archunit.lang.ArchRule;
import com.tngtech.archunit.lang.EvaluationResult;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashSet;
import java.util.List;
import java.util.Objects;
import java.util.Set;
import static com.tngtech.archunit.lang.syntax.ArchRuleDefinition.noClasses;

/** 实际导入字节码、执行 ArchUnit，并拒绝缺失/损坏的项目输入。 */
public final class BytecodeChecker {
    private BytecodeChecker() { }

    private static boolean within(String name, String prefix) {
        return name.equals(prefix) || name.startsWith(prefix + ".");
    }
    private static boolean qualified(String value) {
        return Objects.nonNull(value) && value.length() <= 4096 && value.matches("[A-Za-z_$][A-Za-z0-9_$]*(\\.[A-Za-z_$][A-Za-z0-9_$]*)*");
    }
    private static DescribedPredicate<JavaClass> prefix(String prefix) {
        return new DescribedPredicate<>("package prefix " + prefix) {
            @Override public boolean test(JavaClass type) { return within(type.getPackageName(), prefix); }
        };
    }
    private static void require(boolean condition, String message) {
        if (!condition) { throw new IllegalArgumentException(message); }
    }
    private static void regularPath(Path path, boolean directory) throws Exception {
        require(path.isAbsolute(), "absolute class directory required");
        for (Path ancestor = path; Objects.nonNull(ancestor); ancestor = ancestor.getParent()) {
            require(!Files.isSymbolicLink(ancestor), "linked bytecode input rejected");
            if (System.getProperty("os.name").startsWith("Windows")) {
                require(!Files.readAttributes(ancestor, java.nio.file.attribute.BasicFileAttributes.class, LinkOption.NOFOLLOW_LINKS).isOther(), "reparse input rejected");
            }
        }
        require(directory ? Files.isDirectory(path, LinkOption.NOFOLLOW_LINKS) : Files.isRegularFile(path, LinkOption.NOFOLLOW_LINKS), "regular bytecode input required");
    }
    /** 对显式输入执行完整评估；异常转为 incomplete，不能零发现放行。 */
    public static synchronized BytecodeResult evaluate(BytecodeRequest request) {
        List<BytecodeViolation> violations = new ArrayList<>();
        int importedCount = 0;
        try {
            require(Objects.nonNull(request) && request.schemaVersion() == 1, "unknown request version");
            require(Objects.nonNull(request.classDirectories()) && !request.classDirectories().isEmpty() && request.classDirectories().size() <= 128, "bounded class directories required");
            require(Objects.nonNull(request.expectedClasses()) && !request.expectedClasses().isEmpty() && request.expectedClasses().size() <= 10000, "expected class manifest required");
            require(Objects.nonNull(request.rules()) && !request.rules().isEmpty() && request.rules().size() <= 1024, "nonempty bounded rules required");
            Set<String> expected = new HashSet<>();
            for (String type : request.expectedClasses()) { require(qualified(type) && expected.add(type), "invalid or duplicate expected type"); }
            Set<String> ids = new HashSet<>();
            for (LayerRule rule : request.rules()) {
                require(Objects.nonNull(rule) && Objects.nonNull(rule.ruleId()) && rule.ruleId().matches("[A-Za-z0-9_-]{1,128}") && ids.add(rule.ruleId()) && qualified(rule.from()), "invalid or duplicate rule");
                require(Objects.nonNull(rule.forbiddenTo()) && !rule.forbiddenTo().isEmpty() && rule.forbiddenTo().size() <= 128, "nonempty forbidden prefixes required");
                Set<String> targets = new HashSet<>();
                for (String target : rule.forbiddenTo()) { require(qualified(target) && targets.add(target), "invalid or duplicate target prefix"); }
            }
            List<Path> roots = new ArrayList<>(); Set<Path> files = new HashSet<>(); long bytes = 0;
            for (String directory : request.classDirectories()) {
                Path root = Path.of(directory).normalize(); regularPath(root, true);
                require(roots.stream().noneMatch(p -> root.startsWith(p) || p.startsWith(root)), "duplicate or overlapping class roots"); roots.add(root);
                try (var walk = Files.walk(root, 64)) {
                    int count = 0;
                    var iterator = walk.iterator();
                    while (iterator.hasNext()) {
                        Path item = iterator.next();
                        require(++count <= 20000, "bytecode traversal budget exceeded");
                        regularPath(item, Files.isDirectory(item, LinkOption.NOFOLLOW_LINKS));
                        if (Files.isRegularFile(item) && item.toString().endsWith(".class")) {
                            require(Files.size(item) <= 4 * 1024 * 1024, "class exceeds 4 MiB"); bytes += Files.size(item);
                            require(bytes <= 64 * 1024 * 1024 && files.add(item.toRealPath()) && files.size() <= 10000, "bytecode budget or aliases invalid");
                        }
                    }
                }
            }
            require(files.size() == expected.size(), "class file count differs from expected manifest");
            // 禁止宿主 classpath 自动补齐候选漏件，避免读到旧产物冒充输入。
            ArchConfiguration.get().setResolveMissingDependenciesFromClassPath(false);
            JavaClasses imported = new ClassFileImporter().importPaths(roots);
            Set<String> actual = new HashSet<>();
            for (JavaClass type : imported) {
                require(type.isFullyImported() && actual.add(type.getName()), "incomplete or duplicate imported class");
            }
            require(actual.equals(expected), "imported class set differs from expected manifest");
            importedCount = actual.size();
            for (LayerRule rule : request.rules()) {
                for (String target : rule.forbiddenTo()) {
                    int before = violations.size();
                    ArchRule archRule = noClasses().that(prefix(rule.from())).should().dependOnClassesThat(prefix(target)).as(rule.ruleId()).allowEmptyShould(false);
                    EvaluationResult result = archRule.evaluate(imported);
                    result.<Dependency>handleViolations((dependencies, message) -> {
                        for (Dependency edge : dependencies) {
                            violations.add(new BytecodeViolation(rule.ruleId(), edge.getOriginClass().getName(), edge.getTargetClass().getName(), edge.getSourceCodeLocation().getSourceFileName(), edge.getSourceCodeLocation().getLineNumber(), message));
                        }
                    });
                    require(!result.hasViolation() || violations.size() > before, "unmapped ArchUnit violation");
                }
            }
            violations.sort(Comparator.comparing(BytecodeViolation::ruleId).thenComparing(BytecodeViolation::from).thenComparing(BytecodeViolation::to).thenComparingInt(BytecodeViolation::line));
            return new BytecodeResult(1, "complete", actual.size(), List.copyOf(violations), List.of());
        } catch (Exception | AssertionError failure) {
            return new BytecodeResult(1, "incomplete", importedCount, List.copyOf(violations), List.of(failure.getClass().getSimpleName() + ": " + failure.getMessage()));
        }
    }
}
