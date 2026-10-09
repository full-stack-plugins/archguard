// ArchGuard trusted adapter, Apache-2.0. Uses installed JDK APIs; no JDK source copied.
import com.sun.tools.classfile.*;
import java.nio.file.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
public final class BytecodeCapture {
    private static int records;
    private static String hex(String value) {
        if(value.length()>2048) throw new IllegalArgumentException("symbol size budget");
        try {
            var encoded=StandardCharsets.UTF_8.newEncoder()
                .onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)
                .onUnmappableCharacter(java.nio.charset.CodingErrorAction.REPORT)
                .encode(java.nio.CharBuffer.wrap(value));
            byte[] bytes=new byte[encoded.remaining()]; encoded.get(bytes);
            return HexFormat.of().formatHex(bytes);
        } catch(java.nio.charset.CharacterCodingException error) {
            throw new IllegalArgumentException("invalid symbol UTF-16",error);
        }
    }
    private static void emit(String... columns) {
        if(++records>4096) throw new IllegalArgumentException("record budget");
        System.out.println(String.join("\t", columns));
    }
    private static void gap(String owner,String name,String desc,String relation,String reason,int pc) {
        emit("U",hex(owner),hex(name),hex(desc),relation,hex(reason),Integer.toString(pc));
    }
    private static void attributes(ClassFile cf, Attributes attrs) throws Exception {
        Set<String> understood=Set.of("SourceFile","Signature","Code","Exceptions","ConstantValue","InnerClasses","NestHost","NestMembers","Synthetic","Deprecated","MethodParameters");
        for(Attribute attr:attrs) {
            String name=cf.constant_pool.getUTF8Value(attr.attribute_name_index);
            if(!understood.contains(name)) gap(cf.getName(),"","","D","unsupported attribute: "+name,-1);
        }
    }
    public static void main(String[] args) throws Exception {
        if(args.length>128||args.length%2!=0) throw new IllegalArgumentException("class count budget");
        emit("ARCHGUARD-JAVA","1");
        for(int index=0;index<args.length;index+=2) {
            String expectedName=args[index];
            Path path=Path.of(args[index+1]);
            if(Files.size(path)>524288) throw new IllegalArgumentException("class byte budget");
            byte[] raw=Files.readAllBytes(path);
            java.io.ByteArrayInputStream inputBytes=new java.io.ByteArrayInputStream(raw);
            ClassFile cf=ClassFile.read(inputBytes);
            if(cf.byteLength()!=raw.length) throw new IllegalArgumentException("trailing classfile bytes");
            if(cf.major_version!=65||cf.minor_version!=0||cf.constant_pool.size()>8192||cf.methods.length>256||cf.fields.length>512)
                throw new IllegalArgumentException("unsupported class or structure budget");
            String owner=cf.getName();
            if(!owner.equals(expectedName)) throw new IllegalArgumentException("binary name/path mismatch");
            emit("C",hex(owner));
            attributes(cf,cf.attributes);
            for(Field f:cf.fields) attributes(cf,f.attributes);
            TreeSet<String> targets=new TreeSet<>();
            for(Dependency d:Dependencies.getClassDependencyFinder().findDependencies(cf))
                if(!d.getTarget().getName().equals(owner)) targets.add(d.getTarget().getName());
            for(String target:targets) emit("D",hex(owner),hex(target));
            for(ConstantPool.CPInfo cp:cf.constant_pool.entries()) {
                if(cp.getTag()==ConstantPool.CONSTANT_Dynamic) gap(owner,"","","D","dynamic constant target unresolved",-1);
                if(cp instanceof ConstantPool.CPRefInfo ref) {
                    String target=ref.getClassName();
                    String method=ref.getNameAndTypeInfo().getName();
                    if(target.startsWith("java/lang/reflect/")||target.startsWith("java/lang/invoke/")
                        ||(target.equals("java/lang/Class")&&Set.of("forName","newInstance","getDeclaredConstructor","getDeclaredMethod","getMethod").contains(method))
                        ||target.endsWith("ClassLoader")) gap(owner,"","","D","reflective or dynamic type target unresolved",-1);
                }
            }
            for(Method method:cf.methods) {
                String name=method.getName(cf.constant_pool),desc=method.descriptor.getValue(cf.constant_pool);
                emit("M",hex(owner),hex(name),hex(desc));
                attributes(cf,method.attributes);
                if(method.access_flags.is(AccessFlags.ACC_NATIVE)) {
                    gap(owner,name,desc,"D","native implementation unavailable",-1);
                    gap(owner,name,desc,"C","native call targets unavailable",-1);
                }
                Code_attribute code=(Code_attribute)method.attributes.get(Attribute.Code);
                if(code!=null) for(Instruction instruction:code.getInstructions()) {
                    String op=instruction.getMnemonic();
                    if(op.equals("invokevirtual")||op.equals("invokeinterface")||op.equals("invokedynamic"))
                        gap(owner,name,desc,"C",op+" dispatch unresolved",instruction.getPC());
                    if(op.equals("invokedynamic")) gap(owner,name,desc,"D","invokedynamic type targets unresolved",instruction.getPC());
                }
            }
        }
        emit("END");
    }
}
