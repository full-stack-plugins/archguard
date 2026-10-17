use archguard::bytecode_policy::BytecodePolicy;

#[test]
fn local_policy_has_explicit_scope_and_preserves_rule_identity() {
    let bytes = br#"{"schemaVersion":1,"rules":[{"ruleId":"ARCH-001","from":"demo.domain","forbiddenTo":["demo.infra"]}]}"#;
    let policy = BytecodePolicy::from_local_json(bytes).unwrap();
    assert_eq!(policy.rules().len(), 1);
    assert_eq!(policy.rules()[0].rule_id(), "ARCH-001");
    assert_eq!(policy.rules()[0].from(), "demo.domain");
    assert_eq!(policy.rules()[0].forbidden_to(), ["demo.infra"]);
    assert!(!policy.is_contract_bound());
}

#[test]
fn local_json_cannot_claim_approval_or_hide_duplicate_fields() {
    for bytes in [
        br#"{"schemaVersion":1,"rules":[],"approved":true}"#.as_slice(),
        br#"{"schemaVersion":1,"schemaVersion":1,"rules":[]}"#,
        br#"{"schemaVersion":2,"rules":[]}"#,
        br#"{"schemaVersion":1,"rules":[]} {}"#,
    ] {
        assert!(BytecodePolicy::from_local_json(bytes).is_err());
    }
}

#[test]
fn empty_wildcard_duplicate_and_unsupported_prefixes_are_rejected() {
    for rules in [
        "[]",
        r#"[{"ruleId":"R","from":"demo..domain","forbiddenTo":["demo.infra"]}]"#,
        r#"[{"ruleId":"R","from":"demo.*","forbiddenTo":["demo.infra"]}]"#,
        r#"[{"ruleId":"R","from":"demo.domain","forbiddenTo":[]}]"#,
        r#"[{"ruleId":"R","from":"demo.domain","forbiddenTo":["demo.infra","demo.infra"]}]"#,
        r#"[{"ruleId":"R","from":"demo.domain","forbiddenTo":["demo.infra"]},{"ruleId":"R","from":"demo.api","forbiddenTo":["demo.infra"]}]"#,
    ] {
        let bytes = format!(r#"{{"schemaVersion":1,"rules":{rules}}}"#);
        assert!(
            BytecodePolicy::from_local_json(bytes.as_bytes()).is_err(),
            "{rules}"
        );
    }
}
