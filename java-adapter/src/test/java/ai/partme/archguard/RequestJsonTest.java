package ai.partme.archguard;
import org.junit.Test;
import static org.junit.Assert.assertThrows;
import static org.junit.Assert.assertEquals;
/** 闭合请求不能接受未知字段、重复键或尾随JSON。 */
public class RequestJsonTest {
    @Test public void duplicateUnknownAndTrailingInputFail() {
        assertThrows(Exception.class, () -> RequestJson.read("{\"schemaVersion\":1,\"schemaVersion\":1}"));
        assertThrows(Exception.class, () -> RequestJson.read("{\"schemaVersion\":1,\"approved\":true}"));
        assertThrows(Exception.class, () -> RequestJson.read("{} {}"));
    }
    @Test public void versionCoercionAndDeepJsonFail() {
        assertThrows(Exception.class, () -> RequestJson.read("{\"schemaVersion\":\"1\"}"));
        assertThrows(Exception.class, () -> RequestJson.read("[".repeat(100) + "0" + "]".repeat(100)));
    }
    @Test public void packedInputRequiresOnlyExplicitRootTemplate() {
        assertThrows(Exception.class, () -> PipelineMain.decodePacked("not absolute", "invalid"));
    }
    private String packed(String json) throws Exception {
        var bytes = new java.io.ByteArrayOutputStream();
        try (var gzip = new java.util.zip.GZIPOutputStream(bytes)) { gzip.write(json.getBytes(java.nio.charset.StandardCharsets.UTF_8)); }
        return java.util.Base64.getEncoder().encodeToString(bytes.toByteArray());
    }
    @Test public void packedRootReplacementIsExactAndRejectsBombAndOtherTemplates() throws Exception {
        String root = java.nio.file.Path.of(System.getProperty("java.io.tmpdir")).toAbsolutePath().toString();
        String json = "{\"schemaVersion\":1,\"projectRoot\":\"{{subject_root}}\",\"javaExecutable\":\"java\",\"mavenHome\":\"maven\",\"repository\":\"repo\",\"home\":\"home\",\"temporaryDirectory\":\"tmp\",\"toolArtifacts\":[],\"modules\":[],\"rules\":[]}";
        assertEquals(root, PipelineMain.decodePacked(root, packed(json)).projectRoot());
        assertThrows(Exception.class, () -> PipelineMain.decodePacked(root, packed(json.replace("\"home\":\"home\"", "\"home\":\"{{subject_root}}\""))));
        assertThrows(Exception.class, () -> PipelineMain.decodePacked(root, packed(" ".repeat(1024 * 1024 + 1))));
    }
    @Test public void launchClosureRejectsMissingClasspathEvenWhenJavaPinned() throws Exception {
        String javaExecutable = java.nio.file.Path.of(System.getProperty("java.home"), "bin", System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java").toString();
        var request = new PipelineRequest(1, java.nio.file.Path.of(".").toAbsolutePath().normalize().toString(),
            javaExecutable, "maven", "repo", "home", "tmp",
            java.util.List.of(new ArtifactPin(javaExecutable, "sha256:" + "0".repeat(64))), java.util.List.of(), java.util.List.of());
        assertThrows(Exception.class, () -> PipelineMain.verifyLaunch(request));
    }
}
