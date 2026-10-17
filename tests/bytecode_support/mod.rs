#![cfg(feature = "bytecode-java-tests")]
use archguard::{
    bytecode_config::BytecodeConfig, bytecode_pipeline_request::BytecodePipelineRequest,
    bytecode_policy::BytecodePolicy,
};
use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CapabilityProbe, CompileContext, CompileRequest,
    CompiledContract, ContractLayer, LayerLevel, SourceMaterial, TaskAuthorization,
    VerifierDescriptor, VerifierKind, VerifierObservation, VerifierRegistry, compile,
    snapshot_digest,
};
use guard_protocol::{
    Action, Digest, DomainOwner, Extensions, GitPath, Identifier, ProtocolDocument, SchemaVersion,
    TaskBinding, TaskScope,
};
use guard_runtime::NativeVerifierCommandV2;
use guard_store::EvidenceStore;
use guard_subject::{CaptureBinding, Selection};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
fn id(v: &str) -> Identifier {
    Identifier::new(v).unwrap()
}
fn digest(v: &[u8]) -> Digest {
    archguard::bytecode_pipeline_request::content_digest(v)
}
// 真实编译器测试专用批准端口，仅接受预先记录的精确fixture摘要和context，不是生产批准服务。
struct Approved {
    layers: BTreeMap<LayerLevel, (Digest, Identifier)>,
    context: CompileContext,
}
impl ApprovalAuthority for Approved {
    fn verify_layer(
        &self,
        c: &CompileContext,
        l: LayerLevel,
        a: &Identifier,
        d: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String> {
        if c != &self.context
            || action != Action::Merge
            || self.layers.get(&l) != Some(&(d.clone(), a.clone()))
        {
            return Err("fixture not independently approved".into());
        }
        Ok(ApprovalAttestation {
            authority_id: id("fixture-authority"),
            approval_ref: a.clone(),
            contract_digest: d.clone(),
            context: c.clone(),
            level: l,
            action,
        })
    }
    fn authorize_task(
        &self,
        c: &CompileContext,
        d: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String> {
        if c != &self.context || action != Action::Merge {
            return Err("fixture context mismatch".into());
        }
        Ok(TaskAuthorization {
            authority_id: id("fixture-authority"),
            authorization_ref: id("AUTH-1"),
            contract_digest: d.clone(),
            context: c.clone(),
            action,
        })
    }
}
struct Present;
impl CapabilityProbe for Present {
    fn observe(&self, p: &guard_contract::VerifierPin) -> VerifierObservation {
        VerifierObservation::Available(p.clone())
    }
}
pub fn prepare(
    project: &Path,
    config: &BytecodeConfig,
    policy: &BytecodePolicy,
    selection: &Selection,
    store: &EvidenceStore,
    retain: u64,
    now: u64,
) -> (CompiledContract, guard_plan::RegisteredPlan) {
    let command = BytecodePipelineRequest::new(config, policy)
        .unwrap()
        .command(config, project)
        .unwrap();
    let config_digest = NativeVerifierCommandV2::configuration_digest(&command).unwrap();
    let context = CompileContext {
        repository_id: config.repository_id.clone(),
        task_id: config.task_id.clone(),
        authorization_refs: vec![id("AUTH-1")],
    };
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["ls-files", "-z"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let paths: Vec<_> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| GitPath::from_bytes(p).unwrap())
        .collect();
    let raw = b"explicit fixture architecture constraints";
    let mut layers = Vec::new();
    let mut approved = BTreeMap::new();
    for level in [
        LayerLevel::Organization,
        LayerLevel::Project,
        LayerLevel::Task,
    ] {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../../guardengine/crates/guard-protocol/fixtures/valid/contract-snapshot.json"
        ))
        .unwrap();
        value["contractId"] = json!(format!("fixture-{level:?}"));
        value["sourceRefs"][0]["repoId"] = json!(config.repository_id);
        value["sourceRefs"][0]["contentDigest"] = json!(digest(raw));
        value["rules"][0]["ruleId"] = json!(policy.rules()[0].rule_id());
        value["rules"][0]["verifierId"] = json!(config.verifier_id);
        value["rules"][0]["parameters"] = json!({"schemaVersion":1,"kind":"forbid_package_dependency","from":policy.rules()[0].from(),"forbiddenTo":policy.rules()[0].forbidden_to()});
        value["obligations"][0]["ruleId"] = value["rules"][0]["ruleId"].clone();
        value["obligations"][0]["verifierId"] = json!(config.verifier_id);
        value["obligations"][0]["targets"] = json!(
            paths
                .iter()
                .map(|p| json!({"repositoryId":config.repository_id,"path":p}))
                .collect::<Vec<_>>()
        );
        let ProtocolDocument::ContractSnapshot(mut snapshot) =
            guard_protocol::decode(&serde_json::to_vec(&value).unwrap())
                .unwrap()
                .document()
                .clone()
        else {
            panic!("fixture")
        };
        snapshot.digest = snapshot_digest(&snapshot).unwrap();
        approved.insert(
            level,
            (snapshot.digest.clone(), snapshot.approval_ref.clone()),
        );
        layers.push(ContractLayer {
            level,
            sources: vec![SourceMaterial {
                source: snapshot.source_refs[0].clone(),
                bytes: raw.to_vec(),
            }],
            document: serde_json::to_vec(&ProtocolDocument::ContractSnapshot(snapshot)).unwrap(),
        });
    }
    let mut registry = VerifierRegistry::new(id("opa"));
    for opa in [false, true] {
        registry
            .register(VerifierDescriptor {
                verifier_id: if opa {
                    id("opa")
                } else {
                    config.verifier_id.clone()
                },
                version: id("1"),
                artifact_digest: command.executable_digest.clone(),
                config_digest: config_digest.clone(),
                adapter_version: id(if opa { "1" } else { "archguard-bytecode/1" }),
                kind: if opa {
                    VerifierKind::PolicyEngine
                } else {
                    VerifierKind::Domain
                },
                owner: if opa {
                    None
                } else {
                    Some(DomainOwner::Archguard)
                },
                evidence_types: vec![id(if opa {
                    "policy-decision"
                } else {
                    "source-report"
                })],
                parameter_schema: serde_json::to_vec(&json!({"type":"object"})).unwrap(),
            })
            .unwrap();
    }
    let outcome = compile(
        &CompileRequest {
            layers,
            context: context.clone(),
            action: Action::Merge,
        },
        &registry,
        &Present,
        Some(&Approved {
            layers: approved,
            context,
        }),
    )
    .unwrap();
    let contract = outcome
        .compiled()
        .unwrap_or_else(|| panic!("{:?}", outcome.diagnostics()))
        .clone();
    let binding = TaskBinding {
        schema_version: SchemaVersion::new("1.0.0").unwrap(),
        task_id: config.task_id.clone(),
        requirement_refs: contract.source_refs().to_vec(),
        contract_digest: contract.digest().clone(),
        scope: TaskScope {
            repository_ids: vec![config.repository_id.clone()],
            paths,
            actions: vec![Action::Merge],
        },
        session_id: id("session-fixture"),
        worktree_id: config.worktree_id.clone(),
        authorization_refs: vec![id("AUTH-1")],
        depends_on: vec![],
        extensions: Extensions::default(),
    };
    let capture_binding = CaptureBinding {
        repository_id: config.repository_id.clone(),
        task_id: config.task_id.clone(),
        worktree_id: config.worktree_id.clone(),
        contract_digest: contract.digest().clone(),
        policy_digest: digest(b"fixture policy"),
        verifier_digest: guard_plan::verifier_digest(&contract).unwrap(),
        environment_digest: NativeVerifierCommandV2::environment_digest(&config.environment)
            .unwrap(),
    };
    let captured = guard_subject::capture(project, &capture_binding, selection).unwrap();
    let registered = guard_plan::freeze(
        project,
        &contract,
        &binding,
        &captured,
        config.budget.clone(),
    )
    .unwrap()
    .register(store, retain, now)
    .unwrap();
    (contract, registered)
}
