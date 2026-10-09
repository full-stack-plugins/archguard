# Fixed Java Bytecode Provider Implementation Plan

> Execute directly with executing-plans/TDD; root approved the concrete JDK classfile proposal. No downloads or subagents.

Goal: finish2.1's existing model/capability documentation and deliver2.2 actual Java21 static bytecode type dependencies with real provenance, required-scope gaps and system-rule tests.

Architecture: preserve LanguageObservation/SymbolId/SourceSpan and the reviewed SystemPolicy evaluator. A fixed trusted Java helper uses installed OpenJDK21.0.12.1 `jdk.jdeps/com.sun.tools.classfile.ClassFile` and `Dependencies.getClassDependencyFinder`, never loads candidate classes. Rust freezes controller type→module plus explicit external-class scope, isolates bytecode input, invokes the helper with the bounded runner, and converts strict bounded records to existing typed observations. Helper/capture/input/tool bytes and exact configuration receive digests. No Java evidence envelope or authentication claim.

Spec: extend-architecture-analysis-and-evidence, tasks2.1/2.2; Versioned Language Fact Providers. Resource profile:64required classes,512KiB/class,4MiB copied input,1MiB combined tool output,20seconds/invocation,128MiB JVM heap,64MiB metaspace,2active CPUs/SerialGC. Linux runner limitations unchanged. JAR/ZIP/module-paths/preview classes unsupported. Class version65.0 only. Source spans identify actual generated native capture lines with binary class provenance, never invented Java source lines.

## Steps and review focus

- [x] Add real javac fixtures with legal and forbidden type edges, overloaded methods, reflection/lambda/virtual calls and static initializer payload that must never execute. Watch absent adapter test RED.
- [x] Freeze a JavaProfile with exact controller type→module mapping and explicit external type names; preserve stable structural IDs and all required classes. Add byte/count/scope failure tests before implementation.
- [x] Implement trusted helper compilation with -proc:none and private classpath; fixedJDKrelease/module hashes and build log retained. Invoke only classfile read/dependencyfinder/instruction APIs, no Class.forName or candidate execution.
- [x] Strict record decoder owns bounded native capture, normalizes deterministic ordering, generates Module/Type/Method symbols using JVM descriptors. Scoped DependsOn edges require real parsed endpoints; missing declared class or undeclared nonplatform reference prevents complete coverage.
- [x] Missing/unsupported classfile version yields explicit coverage gap; malformed bytecode/tool/budget failures yield errors. invokevirtual/interface are Unknown Calls; invokedynamic/dynamic constants/native/reflection preserve conservative DependsOn gaps where static type targets cannot be determined. Calls is never declared complete. Unsupported annotation/module metadata cannot silently yield complete broad claims.
- [x] Run real observations through existing system rules and GE: legalALLOW, forbiddenBLOCK, missing/dynamicpartialBLOCK with actual capture path. Preserve overload/language identity tests; no model rewrite.
- [x] Publish precise capability matrix and actual installed/official-source licensing, version/resource boundaries and unsupported cases. Export native captures/provenance from actual JDK tools, then fullRust/MSRV/Clippy/OpenSpec and independentreview. No task checkboxes before acceptance.

Actual evaluation is in ledger/archguard-java-evaluation: fixed javac --release21 compiles five classes; javap native captures and a throwaway classfile API probe show legal core.Port vs forbidden infra.Store references, two distinct JVM method descriptors and invokedynamic/reflective calls. This probe is evaluation evidence, not production adapter code.
