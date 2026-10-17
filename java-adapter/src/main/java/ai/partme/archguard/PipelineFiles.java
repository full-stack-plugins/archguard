package ai.partme.archguard;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Map;
import java.util.Objects;
import java.util.TreeMap;

/** 有界原件读取和稳定清单摘要，不解引用链接。 */
final class PipelineFiles {
    private PipelineFiles() { }
    static void require(boolean condition, String error) { if (!condition) { throw new IllegalArgumentException(error); } }
    static Path safe(Path path) throws Exception {
        require(path.isAbsolute(), "absolute path required");
        for (Path ancestor = path; Objects.nonNull(ancestor); ancestor = ancestor.getParent()) {
            require(!Files.isSymbolicLink(ancestor), "linked input rejected");
            require(!Files.readAttributes(ancestor, java.nio.file.attribute.BasicFileAttributes.class, LinkOption.NOFOLLOW_LINKS).isOther(), "special or reparse input rejected");
        }
        return path.toRealPath(LinkOption.NOFOLLOW_LINKS);
    }
    static String hash(Path path, long budget) throws Exception {
        safe(path); require(Files.isRegularFile(path, LinkOption.NOFOLLOW_LINKS) && Files.size(path) <= budget, "bounded regular artifact required");
        MessageDigest hash = MessageDigest.getInstance("SHA-256"); long total = 0;
        try (InputStream input = Files.newInputStream(path)) {
            byte[] buffer = new byte[65536]; int read;
            while ((read = input.read(buffer)) != -1) { total += read; require(total <= budget, "artifact grew beyond budget"); hash.update(buffer, 0, read); }
        }
        return "sha256:" + HexFormat.of().formatHex(hash.digest());
    }
    static Map<String, String> inventory(Path root, long individualLimit, long totalLimit) throws Exception {
        safe(root); Map<String, String> result = new TreeMap<>(); long total = 0; int count = 0;
        try (var walk = Files.walk(root, 64)) {
            var iterator = walk.iterator();
            while (iterator.hasNext()) {
                Path file = iterator.next(); require(++count <= 20000, "directory budget exceeded"); safe(file);
                if (Files.isRegularFile(file, LinkOption.NOFOLLOW_LINKS)) {
                    total += Files.size(file); require(total <= totalLimit, "input total budget exceeded");
                    result.put(root.relativize(file).toString().replace('\\', '/'), hash(file, individualLimit));
                }
            }
        }
        return result;
    }
    static String manifestDigest(Map<String, String> items) throws Exception {
        MessageDigest hash = MessageDigest.getInstance("SHA-256");
        for (var entry : new TreeMap<>(items).entrySet()) {
            for (String part : new String[]{entry.getKey(), entry.getValue()}) {
                byte[] value = part.getBytes(StandardCharsets.UTF_8); hash.update(ByteBuffer.allocate(8).putLong(value.length).array()); hash.update(value);
            }
        }
        return "sha256:" + HexFormat.of().formatHex(hash.digest());
    }
}
