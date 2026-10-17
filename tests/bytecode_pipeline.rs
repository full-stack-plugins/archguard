use archguard::bytecode_pipeline_result::BytecodePipelineResult;
#[test]
fn strict_result_rejects_duplicate_and_false_complete() {
    assert!(
        BytecodePipelineResult::from_json(br#"{"schemaVersion":1,"schemaVersion":1}"#).is_err()
    );
    let value=br#"{"schemaVersion":1,"completeness":"complete","classCount":0,"violations":[],"diagnostics":[],"sourceDigest":null,"bytecodeDigest":null,"toolDigest":null,"ruleDigest":null,"adapterVersion":"archguard-bytecode/1","archunitVersion":"1.5.1","modules":[]}"#;
    assert!(BytecodePipelineResult::from_json(value).is_err());
}

#[test]
fn packed_request_preserves_literal_root_and_windows_json_without_nonce() {
    use archguard::bytecode_pipeline_request::BytecodePipelineRequest;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::Read;
    let request = BytecodePipelineRequest {
        schema_version: 1,
        project_root: "{{subject_root}}".into(),
        java_executable: r"D:\Java\java.exe".into(),
        maven_home: r"D:\Java\Maven".into(),
        repository: r"E:\repo".into(),
        home: r"E:\home".into(),
        temporary_directory: r"E:\tmp".into(),
        tool_artifacts: vec![],
        modules: vec![],
        rules: vec![],
    };
    let first = request.packed().unwrap();
    assert_eq!(first, request.packed().unwrap());
    let raw = STANDARD.decode(&first).unwrap();
    assert_eq!(&raw[4..8], &[0, 0, 0, 0]);
    let mut decoded = String::new();
    flate2::read::GzDecoder::new(raw.as_slice())
        .read_to_string(&mut decoded)
        .unwrap();
    let json: serde_json::Value = serde_json::from_str(&decoded).unwrap();
    assert_eq!(json["projectRoot"], "{{subject_root}}");
    assert_eq!(json["javaExecutable"], r"D:\Java\java.exe");
    assert!(!first.contains("{{subject_root}}"));
    let mut large = request;
    large.java_executable = "x".repeat(1024 * 1024);
    assert!(large.packed().is_err());
}
#[test]
fn closed_local_configuration_rejects_ambiguous_environment_and_coverage() {
    use archguard::bytecode_config::BytecodeConfig;
    use serde_json::json;
    let root = std::env::current_dir().unwrap();
    let file = root.join("java.exe").to_str().unwrap().to_string();
    let value = json!({"schemaVersion":1,"repositoryId":"repo","taskId":"task","worktreeId":"worktree","verifierId":"java-bytecode","javaExecutable":file,"classpath":[file],"mavenHome":root,"repository":root,"home":root,"temporaryDirectory":root,"toolArtifacts":[{"path":file,"sha256":format!("sha256:{}","a".repeat(64))}],"modules":[{"path":"","expectedTypes":["demo.Model"]}],"environment":{"GIT_CEILING_DIRECTORIES":"{{subject_parent}}"},"budget":{"timeoutMs":30000,"maxOutputBytes":1048576,"maxConcurrency":1}});
    assert!(BytecodeConfig::from_json(&serde_json::to_vec(&value).unwrap()).is_ok());
    for (key, val) in [
        ("Git_Ceiling_Directories", "{{subject_parent}}"),
        ("GIT_DIR", "E:/other"),
        ("PÄTH", "x"),
        ("JAVA_TOOL_OPTIONS", "-javaagent:E:/unlisted.jar"),
    ] {
        let mut bad = value.clone();
        bad["environment"][key] = json!(val);
        assert!(BytecodeConfig::from_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
    let mut old = value.clone();
    old["environment"]["GIT_CEILING_DIRECTORIES"] = json!("{{subject_root}}");
    assert!(BytecodeConfig::from_json(&serde_json::to_vec(&old).unwrap()).is_err());
    let mut missing = value.clone();
    missing["modules"][0]["expectedTypes"] = json!([]);
    assert!(BytecodeConfig::from_json(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut escalation = value;
    escalation["approved"] = json!(true);
    assert!(BytecodeConfig::from_json(&serde_json::to_vec(&escalation).unwrap()).is_err());
}
#[test]
fn product_command_dispatches_bytecode_without_changing_old_command_namespace() {
    let error = archguard::domain_cli::run(&["bytecode".into(), "check".into()]).unwrap_err();
    assert!(error.contains("--project"));
    assert!(archguard::domain_cli::run(&["bytecode".into(), "serve".into()]).is_err());
}
