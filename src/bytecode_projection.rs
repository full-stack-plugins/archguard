use crate::{
    bytecode_artifact_ref::BytecodeArtifactRef, bytecode_config::BytecodeConfig,
    bytecode_projection_context::BytecodeProjectionContext,
    bytecode_public_outcome::BytecodePublicOutcome, bytecode_public_result::BytecodePublicResult,
    bytecode_receipt::BytecodeReceipt, bytecode_run_outcome::BytecodeRunOutcome, bytecode_service,
};
use guard_contract::CompiledContract;
use guard_plan::RegisteredPlan;
use guard_protocol::{
    Assessment, Classification, Coverage, DomainOwner, EvidenceRef, Execution, Extensions,
    GuardResult, Identifier, ProtocolDocument, Remediation, Retention, RunSequence, SchemaVersion,
    UtcTimestamp, Verdict,
};
use guard_run_ledger::{RunIdentity, RunLedger, RunScope, RunState};
use guard_runner::Cancellation;
use guard_store::EvidenceStore;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, sync::Arc};
fn digest(value: &impl serde::Serialize) -> Result<guard_protocol::Digest, String> {
    guard_protocol::digest_json(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
fn rule_findings(
    result: Option<&crate::bytecode_pipeline_result::BytecodePipelineResult>,
    rule: &Identifier,
) -> Vec<crate::bytecode_violation::BytecodeViolation> {
    result
        .map(|r| {
            r.violations
                .iter()
                .filter(|v| v.rule_id == rule.as_str())
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}
/// 回读真实当前账本和原件，将指定批准义务投影为公共GuardResult；不证明身份签名、OS隔离或交付授权。
/// 只验证冻结身份与当前运行头；调用端消费前仍须核当前候选和独立信任策略。
pub fn project_registered_result(
    project: &Path,
    receipt: &BytecodeReceipt,
    store: &EvidenceStore,
    contract: &CompiledContract,
    plan: &RegisteredPlan,
    obligation_id: &Identifier,
    context: &BytecodeProjectionContext,
) -> Result<GuardResult, String> {
    let stamp = bytecode_service::now()?;
    plan.assert_current(store, project, stamp)?;
    receipt.verify(store, stamp)?;
    let report = &receipt.report;
    if report.mode != "registered"
        || &plan.plan().verification().contract_digest != contract.digest()
        || receipt.artifact.operation != format!("{}.report", context.operation_id.as_str())
    {
        return Err("public projection requires matching registered execution operation".into());
    }
    let obligation = plan
        .plan()
        .verification()
        .obligations
        .iter()
        .find(|o| &o.obligation_id == obligation_id)
        .ok_or("unknown projection obligation")?;
    let rule = contract
        .rules()
        .iter()
        .find(|r| r.rule_id == obligation.rule_id)
        .ok_or("approved rule absent")?;
    let pin = contract
        .verifiers()
        .iter()
        .find(|p| p.verifier_id == obligation.verifier_id)
        .ok_or("approved verifier absent")?;
    let producer = &context.producer;
    if pin.adapter_version.as_str() != "archguard-bytecode/1"
        || report
            .result
            .as_ref()
            .is_some_and(|r| r.adapter_version != pin.adapter_version.as_str())
        || rule.owner != DomainOwner::Archguard
        || rule.verifier_id != obligation.verifier_id
        || producer.version != pin.version
        || producer.artifact_digest != pin.artifact_digest
        || producer.config_digest != pin.config_digest
        || producer.adapter_version != pin.adapter_version
        || producer.execution_environment_digest != plan.capture_binding().environment_digest
        || report.config_digest != pin.config_digest
    {
        return Err("producer or professional rule differs from compiled pin".into());
    }
    guard_protocol::encode(&ProtocolDocument::ProducerIdentity(producer.clone()))?;
    let reference = report
        .execution_observation
        .as_ref()
        .ok_or("native registered observation missing")?;
    let raw = store.read_private(&reference.operation, stamp)?;
    let canonical = guard_protocol::canonicalize(&raw)?;
    let meta: Value = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
    let identity = RunIdentity {
        binding_digest: digest(
            &json!({"taskBinding":plan.binding(),"planDigest":plan.plan().digest(),"activeHead":plan.head()}),
        )?,
        subject_digest: plan.plan().verification().subject_digest.clone(),
        contract_digest: contract.digest().clone(),
        policy_digest: plan.plan().verification().policy_digest.clone(),
        verifier_digest: guard_plan::verifier_digest(contract)?,
        environment_digest: plan.capture_binding().environment_digest.clone(),
    };
    let actual: RunIdentity =
        serde_json::from_value(meta["identity"].clone()).map_err(|e| e.to_string())?;
    let sequence: RunSequence =
        serde_json::from_value(meta["sequence"].clone()).map_err(|e| e.to_string())?;
    if actual != identity
        || meta["schemaVersion"] != "guard.execution-observation/v1"
        || meta["trust"] != "local_unverified"
        || meta["obligationId"] != json!(obligation_id)
        || meta["verifierId"] != json!(pin.verifier_id)
        || meta["planDigest"] != json!(plan.plan().digest())
        || report.subject_digest != identity.subject_digest
        || meta["executionStatus"] != json!(report.execution_status)
        || meta["exitCode"] != json!(report.exit_code)
        || meta["startedAt"] != json!(report.started_at)
        || meta["finishedAt"] != json!(report.finished_at)
        || meta["inputDrifted"] != json!(report.input_drifted)
        || meta["outputTruncated"] != json!(report.output_truncated)
        || meta["stopReason"] != json!(report.stop_reason)
        || meta["stdout"] != json!(report.stdout)
        || meta["stderr"] != json!(report.stderr)
    {
        return Err("professional report differs from frozen actual execution observation".into());
    }
    let scope = RunScope {
        repository_id: contract.context().repository_id.clone(),
        task_id: plan.binding().task_id.clone(),
        obligation_id: obligation_id.clone(),
        approved_scope_digest: digest(&plan.binding().scope)?,
    };
    let current = RunLedger::new(store)
        .inspect(&scope, &identity, stamp)?
        .ok_or("current execution head absent")?;
    if matches!(current.state, RunState::Running | RunState::Stale)
        || current.sequence != sequence
        || current.result_digest.as_ref()
            != Some(&crate::bytecode_pipeline_request::content_digest(
                &canonical,
            ))
    {
        return Err("execution observation is not current finalized ledger result".into());
    }
    let findings = rule_findings(report.result.as_ref(), &rule.rule_id);
    let finding_ids = findings
        .iter()
        .map(|v| {
            let d = digest(&json!({"obligation":obligation_id,"rule":rule.rule_id,"edge":v}))?;
            Identifier::new(format!("FIND-{}", &d.as_str()[7..]))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let complete = matches!(report.status.as_str(), "passed" | "violations")
        && report
            .result
            .as_ref()
            .is_some_and(|r| r.completeness == "complete");
    let coverage = if complete {
        Coverage::Complete
    } else if report
        .result
        .as_ref()
        .is_some_and(|r| !r.modules.is_empty() || !findings.is_empty())
    {
        Coverage::Partial
    } else {
        Coverage::None
    };
    let verdict = if !findings.is_empty() {
        Verdict::Violation
    } else if complete && report.execution_status == guard_protocol::ExecutionStatus::Completed {
        Verdict::Satisfied
    } else {
        Verdict::Unknown
    };
    let retain = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(
        i64::try_from(report.retain_until_ms).map_err(|e| e.to_string())?,
    )
    .ok_or("retention time range")?;
    let retention = Retention {
        policy_id: context.retention_policy_id.clone(),
        retain_until: UtcTimestamp::new(retain.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string())?,
    };
    let mut evidence = BTreeMap::new();
    for (reference, media) in [
        (&report.stdout, "application/octet-stream"),
        (&report.stderr, "application/octet-stream"),
        (&receipt.artifact, "application/json"),
        (reference, "application/json"),
    ] {
        evidence
            .entry(reference.digest.clone())
            .or_insert(EvidenceRef {
                digest: reference.digest.clone(),
                media_type: Identifier::new(media)?,
                size: reference.size,
                classification: Classification::Private,
                retention: retention.clone(),
            });
    }
    let extensions:Extensions=serde_json::from_value(json!({"archguard:trust":"UNVERIFIED","archguard:obligationId":obligation_id,"archguard:findings":findings,"archguard:rawReferences":{"stdout":report.stdout,"stderr":report.stderr,"professional":receipt.artifact,"execution":reference},"archguard:evidenceTypes":rule.required_evidence})).map_err(|e|e.to_string())?;
    let result = GuardResult {
        schema_version: SchemaVersion::new("1.0.0")?,
        result_id: context.result_id.clone(),
        run_id: context.run_id.clone(),
        operation_id: context.operation_id.clone(),
        task_id: plan.binding().task_id.clone(),
        subject_digest: identity.subject_digest,
        contract_digest: identity.contract_digest,
        policy_digest: identity.policy_digest,
        producer: producer.clone(),
        guard: DomainOwner::Archguard,
        rule_id: rule.rule_id.clone(),
        mode: rule.mode,
        severity: rule.severity,
        execution: Execution {
            status: report.execution_status,
            exit_code: report.exit_code,
            started_at: report.started_at.clone(),
            finished_at: report.finished_at.clone(),
            sequence,
            lineage_id: context.lineage_id.clone(),
        },
        assessment: Assessment {
            verdict,
            coverage,
            applicability_evidence_refs: vec![],
            finding_ids,
        },
        evidence_refs: evidence.into_values().collect(),
        remediation: Remediation {
            summary: Identifier::new(
                "Review the forbidden dependency and recheck the current candidate",
            )?,
            argv: vec![],
        },
        limitations: vec![
            Identifier::new("UNVERIFIED")?,
            Identifier::new("main-java-only-exact-types")?,
            Identifier::new("local-execution-not-os-sandbox")?,
            Identifier::new("independent-authority-and-gate-still-required")?,
        ],
        extensions,
    };
    guard_protocol::encode(&ProtocolDocument::GuardResult(result.clone()))?;
    Ok(result)
}
/// 真实注册执行后生成并持久化逐义务公共结果；Existing不执行、不合成通过或提升信任。
#[allow(clippy::too_many_arguments)]
pub async fn check_registered_result(
    project: &Path,
    scratch: &Path,
    config: &BytecodeConfig,
    contract: &CompiledContract,
    plan: &RegisteredPlan,
    obligation: &Identifier,
    context: &BytecodeProjectionContext,
    store: Arc<EvidenceStore>,
    operation: &str,
    retain: u64,
    cancel: Cancellation,
) -> Result<BytecodePublicOutcome, String> {
    if context.operation_id.as_str() != operation {
        return Err("projection operation ID mismatch".into());
    }
    let frozen = plan
        .plan()
        .verification()
        .obligations
        .iter()
        .find(|o| &o.obligation_id == obligation)
        .ok_or("unknown projection obligation")?;
    let pin = contract
        .verifiers()
        .iter()
        .find(|p| p.verifier_id == frozen.verifier_id)
        .ok_or("projection verifier missing")?;
    let producer = &context.producer;
    if pin.adapter_version.as_str() != "archguard-bytecode/1"
        || producer.version != pin.version
        || producer.artifact_digest != pin.artifact_digest
        || producer.config_digest != pin.config_digest
        || producer.adapter_version != pin.adapter_version
        || producer.execution_environment_digest != plan.capture_binding().environment_digest
    {
        return Err("producer declaration differs from compiled pin before dispatch".into());
    }

    match bytecode_service::check_registered(
        project,
        scratch,
        config,
        contract,
        plan,
        obligation,
        store.clone(),
        operation,
        retain,
        cancel,
    )
    .await?
    {
        BytecodeRunOutcome::Existing { sequence } => {
            Ok(BytecodePublicOutcome::Existing { sequence })
        }
        BytecodeRunOutcome::Finished(receipt) => {
            let projection_receipt = receipt.clone();
            let projection_store = store.clone();
            let projection_contract = contract.clone();
            let projection_plan = plan.clone();
            let projection_obligation = obligation.clone();
            let projection_context = context.clone();
            let projection_project = project.to_path_buf();
            let result = bytecode_service::blocking(move || {
                project_registered_result(
                    &projection_project,
                    &projection_receipt,
                    &projection_store,
                    &projection_contract,
                    &projection_plan,
                    &projection_obligation,
                    &projection_context,
                )
            })
            .await?;
            let bytes = guard_protocol::encode(&ProtocolDocument::GuardResult(result.clone()))?;
            let op = format!("{operation}.guard-result");
            store.put(&op, &bytes, retain, bytecode_service::now()?)?;
            let artifact = BytecodeArtifactRef {
                operation: op,
                digest: crate::bytecode_pipeline_request::content_digest(&bytes),
                size: bytes.len() as u64,
            };
            Ok(BytecodePublicOutcome::Finished(Box::new(
                BytecodePublicResult {
                    receipt: *receipt,
                    result,
                    artifact,
                },
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_verifier_does_not_attribute_another_rules_edge_to_this_obligation() {
        let make = |rule: &str| crate::bytecode_violation::BytecodeViolation {
            rule_id: rule.into(),
            from: "demo.Model".into(),
            to: "infra.Store".into(),
            file: "src/main/java/demo/Model.java".into(),
            line: 1,
            description: "actual edge".into(),
        };
        let result = crate::bytecode_pipeline_result::BytecodePipelineResult {
            schema_version: 1,
            completeness: "incomplete".into(),
            class_count: 1,
            violations: vec![make("RULE-A"), make("RULE-B")],
            diagnostics: vec![],
            source_digest: None,
            bytecode_digest: None,
            tool_digest: None,
            rule_digest: None,
            adapter_version: "archguard-bytecode/1".into(),
            archunit_version: "1.5.1".into(),
            modules: vec![],
        };
        let selected = super::rule_findings(
            Some(&result),
            &guard_protocol::Identifier::new("RULE-A").unwrap(),
        );
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].rule_id, "RULE-A");
        assert!(
            super::rule_findings(
                Some(&result),
                &guard_protocol::Identifier::new("RULE-C").unwrap()
            )
            .is_empty()
        );
    }
}
