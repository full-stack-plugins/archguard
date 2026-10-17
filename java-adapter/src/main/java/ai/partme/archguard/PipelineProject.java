package ai.partme.archguard;
import javax.xml.XMLConstants;
import javax.xml.parsers.DocumentBuilderFactory;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayDeque;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.Objects;
import static ai.partme.archguard.PipelineFiles.require;

/** 首批默认Maven源码根与完整reactor的保守支持边界。 */
final class PipelineProject {
    private PipelineProject() { }
    static Set<Path> validate(Path root, List<ModuleInput> modules) throws Exception {
        require(Objects.nonNull(modules) && !modules.isEmpty() && modules.size() <= 128, "bounded nonempty modules required");
        require(!Files.exists(root.resolve(".mvn")), "candidate .mvn configuration unsupported");
        Set<Path> configured = new HashSet<>();
        for (ModuleInput module : modules) {
            require(Objects.nonNull(module) && Objects.nonNull(module.path()) && Objects.nonNull(module.expectedTypes()), "module fields required");
            Path relative = Path.of(module.path());
            require(!relative.isAbsolute() && relative.normalize().equals(relative) && !module.path().contains("..") && !module.path().contains(":"), "safe module path required");
            Path directory = PipelineFiles.safe(root.resolve(relative)); require(directory.startsWith(root) && configured.add(directory), "duplicate/escaping module");
            require(!Files.exists(directory.resolve("target")), "preexisting target is frozen input: fresh private candidate required");
        }
        Set<Path> actual = new HashSet<>(); ArrayDeque<Path> pending = new ArrayDeque<>(); pending.add(root);
        while (!pending.isEmpty()) {
            Path directory = pending.remove(); require(actual.size() < 128 && actual.add(directory), "reactor cycle or module budget");
            Path pom = PipelineFiles.safe(directory.resolve("pom.xml")); require(pom.startsWith(root) && Files.size(pom) <= 4 * 1024 * 1024, "bounded project POM required");
            DocumentBuilderFactory factory = DocumentBuilderFactory.newInstance();
            factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
            factory.setFeature("http://xml.org/sax/features/external-general-entities", false);
            factory.setFeature("http://xml.org/sax/features/external-parameter-entities", false);
            factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_DTD, ""); factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_SCHEMA, "");
            var doc = factory.newDocumentBuilder().parse(pom.toFile());
            for (String unsupported : List.of("profiles", "sourceDirectory", "testSourceDirectory", "extensions", "executions", "outputDirectory", "directory", "includes", "excludes", "annotationProcessorPaths", "annotationProcessors", "compilerArgs", "compilerArguments", "compilerId", "jdkToolchain", "proc", "fork", "executable", "maven.compiler.executable", "maven.compiler.fork", "maven.compiler.compilerId", "maven.compiler.proc", "systemPath")) {
                require(doc.getElementsByTagName(unsupported).getLength() == 0, "unsupported Maven configuration: " + unsupported);
            }
            var scopes = doc.getElementsByTagName("scope");
            for (int i = 0; i < scopes.getLength(); i++) { require(!scopes.item(i).getTextContent().trim().equals("system"), "system scoped dependencies are outside pinned repository closure"); }
            var parents = doc.getElementsByTagName("parent");
            require(parents.getLength() <= 1, "ambiguous Maven parent");
            if (parents.getLength() == 1) {
                var parent = (org.w3c.dom.Element) parents.item(0);
                var relatives = parent.getElementsByTagName("relativePath");
                require(relatives.getLength() <= 1, "ambiguous Maven parent relativePath");
                String value = relatives.getLength() == 0 ? "../pom.xml" : relatives.item(0).getTextContent().trim();
                // 空relativePath明确使用已经pin的离线仓库；非空值只能读取本次配置reactor中的POM。
                if (!value.isEmpty()) {
                    require(!Path.of(value).isAbsolute() && !value.contains("${"), "literal relative Maven parent required");
                    Path localParent = directory.resolve(value).normalize();
                    require(localParent.startsWith(root) && configured.stream().anyMatch(p -> p.resolve("pom.xml").equals(localParent)), "Maven parent outside declared candidate reactor");
                    PipelineFiles.safe(localParent);
                }
            }
            var declared = doc.getElementsByTagName("module");
            for (int i = 0; i < declared.getLength(); i++) {
                String value = declared.item(i).getTextContent().trim();
                require(!value.contains("${") && !Path.of(value).isAbsolute(), "literal relative module required");
                Path child = PipelineFiles.safe(directory.resolve(value).normalize()); require(child.startsWith(root), "module escapes candidate"); pending.add(child);
            }
        }
        require(configured.equals(actual), "configured modules differ from full Maven reactor"); return configured;
    }
}
