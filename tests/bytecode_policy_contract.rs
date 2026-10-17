use archguard::bytecode_policy::BytecodePolicy;
use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CapabilityProbe, CompileContext, CompileRequest,
    CompiledContract, ContractLayer, LayerLevel, SourceMaterial, TaskAuthorization,
    VerifierDescriptor, VerifierKind, VerifierObservation, VerifierRegistry, compile,
    snapshot_digest,
};
use guard_protocol::{Action, Digest, DomainOwner, Identifier, ProtocolDocument};
use serde_json::{Value, json};
use sha2::{Digest as Hasher, Sha256};

fn id(value: &str) -> Identifier {
    Identifier::new(value).unwrap()
}
fn digest(bytes: &[u8]) -> Digest {
    Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).unwrap()
}

// 测试专用独立端口；不实现或假冒生产批准服务。
struct FixtureAuthority;
impl ApprovalAuthority for FixtureAuthority {
    fn verify_layer(
        &self,
        context: &CompileContext,
        level: LayerLevel,
        approval: &Identifier,
        digest: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String> {
        Ok(ApprovalAttestation {
            authority_id: id("fixture-authority"),
            approval_ref: approval.clone(),
            contract_digest: digest.clone(),
            context: context.clone(),
            level,
            action,
        })
    }
    fn authorize_task(
        &self,
        context: &CompileContext,
        digest: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String> {
        Ok(TaskAuthorization {
            authority_id: id("fixture-authority"),
            authorization_ref: id("AUTH-1"),
            contract_digest: digest.clone(),
            context: context.clone(),
            action,
        })
    }
}
struct FixtureProbe;
impl CapabilityProbe for FixtureProbe {
    fn observe(&self, pin: &guard_contract::VerifierPin) -> VerifierObservation {
        VerifierObservation::Available(pin.clone())
    }
}

fn fixture(parameters: Value, owner: DomainOwner) -> CompiledContract {
    let raw_source = b"independently provided fixture constraints";
    let mut layers = Vec::new();
    for level in [
        LayerLevel::Organization,
        LayerLevel::Project,
        LayerLevel::Task,
    ] {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../guardengine/crates/guard-protocol/fixtures/valid/contract-snapshot.json"
        ))
        .unwrap();
        value["contractId"] = json!(format!("fixture-{level:?}"));
        value["sourceRefs"][0]["contentDigest"] = json!(digest(raw_source));
        value["rules"][0]["parameters"] = parameters.clone();
        value["rules"][0]["owner"] = json!(owner);
        let parsed = guard_protocol::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        let ProtocolDocument::ContractSnapshot(mut snapshot) = parsed.document().clone() else {
            panic!("contract fixture")
        };
        snapshot.digest = snapshot_digest(&snapshot).unwrap();
        layers.push(ContractLayer {
            level,
            sources: vec![SourceMaterial {
                source: snapshot.source_refs[0].clone(),
                bytes: raw_source.to_vec(),
            }],
            document: serde_json::to_vec(&ProtocolDocument::ContractSnapshot(snapshot)).unwrap(),
        });
    }
    let mut registry = VerifierRegistry::new(id("opa"));
    for policy in [false, true] {
        registry
            .register(VerifierDescriptor {
                verifier_id: id(if policy { "opa" } else { "java-package-check" }),
                version: id("1"),
                artifact_digest: digest(b"fixture artifact"),
                config_digest: digest(b"fixture config"),
                adapter_version: id("1"),
                kind: if policy {
                    VerifierKind::PolicyEngine
                } else {
                    VerifierKind::Domain
                },
                owner: if policy { None } else { Some(owner) },
                evidence_types: vec![id(if policy {
                    "policy-decision"
                } else {
                    "source-report"
                })],
                parameter_schema: serde_json::to_vec(&json!({"type":"object"})).unwrap(),
            })
            .unwrap();
    }
    let request = CompileRequest {
        layers,
        context: CompileContext {
            repository_id: id("repo-example"),
            task_id: id("REQ-1"),
            authorization_refs: vec![id("AUTH-1")],
        },
        action: Action::Merge,
    };
    let result = compile(&request, &registry, &FixtureProbe, Some(&FixtureAuthority)).unwrap();
    result
        .compiled()
        .unwrap_or_else(|| panic!("{:?}", result.diagnostics()))
        .clone()
}
fn params() -> Value {
    json!({"schemaVersion":1,"kind":"forbid_package_dependency","from":"demo.domain","forbiddenTo":["demo.infra"]})
}

#[test]
fn compiled_architecture_parameters_keep_contract_and_rule_identity() {
    let contract = fixture(params(), DomainOwner::Archguard);
    let policy = BytecodePolicy::from_contract(&contract, &id("java-package-check")).unwrap();
    assert_eq!(policy.contract_digest(), Some(contract.digest()));
    assert!(policy.is_contract_bound());
    assert_eq!(policy.rules()[0].rule_id(), "ARCH-1");
    assert_eq!(policy.rules()[0].forbidden_to(), ["demo.infra"]);
    assert!(BytecodePolicy::from_contract(&contract, &id("another-verifier")).is_err());
}

#[test]
fn unsupported_parameters_and_other_professional_owner_cannot_be_projected() {
    for parameters in [
        json!({"schemaVersion":1,"kind":"layer_allowlist","from":"demo.domain","forbiddenTo":["demo.infra"]}),
        json!({"schemaVersion":1,"kind":"forbid_package_dependency","from":"demo.domain","forbiddenTo":["demo.infra"],"approved":true}),
    ] {
        let contract = fixture(parameters, DomainOwner::Archguard);
        assert!(BytecodePolicy::from_contract(&contract, &id("java-package-check")).is_err());
    }
    let contract = fixture(params(), DomainOwner::Codeguard);
    assert!(BytecodePolicy::from_contract(&contract, &id("java-package-check")).is_err());
}
