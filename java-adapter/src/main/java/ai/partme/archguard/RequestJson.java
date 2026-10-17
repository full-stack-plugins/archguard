package ai.partme.archguard;
import com.fasterxml.jackson.core.JsonFactory;
import com.fasterxml.jackson.core.StreamReadConstraints;
import com.fasterxml.jackson.core.StreamReadFeature;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.MapperFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.json.JsonMapper;
/** 有界闭合JSON解码；不加载候选代码或信任标签。 */
public final class RequestJson {
    private RequestJson() { }
    /** 将至多1MiB请求解码为严格技术输入，未知/重复/尾随字段均拒绝。 */
    public static BytecodeRequest read(String json) throws Exception {
        return read(json, BytecodeRequest.class);
    }
    static <T> T read(String json, Class<T> type) throws Exception {
        if (json.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > 1024 * 1024) { throw new IllegalArgumentException("request exceeds 1 MiB"); }
        JsonFactory factory = JsonFactory.builder().enable(StreamReadFeature.STRICT_DUPLICATE_DETECTION)
            .streamReadConstraints(StreamReadConstraints.builder().maxNestingDepth(32).maxStringLength(32768).maxNumberLength(128).build()).build();
        ObjectMapper mapper = JsonMapper.builder(factory).disable(MapperFeature.ALLOW_COERCION_OF_SCALARS).build();
        return mapper
            .enable(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES)
            .enable(DeserializationFeature.FAIL_ON_TRAILING_TOKENS)
            .enable(DeserializationFeature.FAIL_ON_NULL_FOR_PRIMITIVES)
            .readValue(json, type);
    }
}
