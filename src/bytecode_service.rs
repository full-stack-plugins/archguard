use crate::{
    bytecode_artifact_ref::BytecodeArtifactRef,
    bytecode_config::BytecodeConfig,
    bytecode_paths,
    bytecode_pipeline_request::{BytecodePipelineRequest, content_digest, manifest_digest},
    bytecode_pipeline_result::BytecodePipelineResult,
    bytecode_policy::BytecodePolicy,
    bytecode_receipt::BytecodeReceipt,
    bytecode_report::BytecodeReport,
    bytecode_run_outcome::BytecodeRunOutcome,
};
use guard_contract::CompiledContract;
use guard_plan::RegisteredPlan;
use guard_protocol::{Digest, ExecutionStatus, Identifier};
use guard_runner::{Cancellation, ProcessRunner};
use guard_runtime::{DispatchOutcome, NativeVerifierCommandV2, Runtime};
use guard_store::EvidenceStore;
use guard_subject::{CaptureBinding, CapturedSubject, Selection};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
/// 本机墙钟只用于保留期限，不授信或替代实际进程观察。
pub(crate) fn now() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())
}
fn canonical_digest(value: &impl serde::Serialize) -> Result<Digest, String> {
    Ok(content_digest(&guard_protocol::canonicalize(
        &serde_json::to_vec(value).map_err(|e| e.to_string())?,
    )?))
}
fn policy_digest(policy: &BytecodePolicy) -> Result<Digest, String> {
    canonical_digest(&policy.rules())
}
fn kind(selection: &Selection) -> String {
    match selection {
        Selection::Commit { .. } => "commit",
        Selection::Index { .. } => "index",
        Selection::Worktree { .. } => "worktree",
        Selection::MergeCandidate { .. } => "merge_candidate",
    }
    .into()
}
fn selection_scope_digest(selection: &Selection) -> Result<Digest, String> {
    let value = match selection {
        Selection::Worktree { base } => serde_json::json!({"kind":"worktree","base":base}),
        Selection::Index { base } => serde_json::json!({"kind":"index","base":base}),
        Selection::Commit { base, .. } => serde_json::json!({"kind":"commit","base":base}),
        Selection::MergeCandidate {
            target_ref,
            expected_target,
            ..
        } => {
            serde_json::json!({"kind":"merge_candidate","targetRef":target_ref,"expectedTarget":expected_target})
        }
    };
    canonical_digest(&value)
}
fn ensure_supported_sources(
    config: &BytecodeConfig,
    captured: &CapturedSubject,
) -> Result<(), String> {
    for file in &captured.manifest {
        let name = String::from_utf8(file.path.to_bytes())
            .map_err(|_| "Java adapter requires UTF8 paths")?;
        if name.ends_with(".java")
            && !config.modules.iter().any(|m| {
                name.starts_with(&if m.path.is_empty() {
                    "src/main/java/".into()
                } else {
                    format!("{}/src/main/java/", m.path)
                })
            })
        {
            return Err("bytecode provider supports declared src/main/java only; test/custom/generated Java inputs require another explicit capability".into());
        }
    }
    Ok(())
}
fn source_digest(captured: &CapturedSubject) -> Result<Digest, String> {
    let mut map = BTreeMap::new();
    for file in &captured.manifest {
        let key = String::from_utf8(file.path.to_bytes())
            .map_err(|_| "Java adapter requires UTF8 project paths")?;
        map.insert(key, file.content_digest.as_str().into());
    }
    Ok(manifest_digest(&map))
}
fn put(
    store: &EvidenceStore,
    operation: &str,
    bytes: &[u8],
    retain: u64,
) -> Result<BytecodeArtifactRef, String> {
    store.put(operation, bytes, retain, now()?)?;
    Ok(BytecodeArtifactRef {
        operation: operation.into(),
        digest: content_digest(bytes),
        size: bytes.len() as u64,
    })
}
fn assess(
    config: &BytecodeConfig,
    policy: &BytecodePolicy,
    captured: &CapturedSubject,
    raw: &[u8],
    expected_tools: &Digest,
) -> Result<BytecodePipelineResult, String> {
    let result = BytecodePipelineResult::from_json(raw)?;
    let modules: BTreeSet<_> = config.modules.iter().map(|m| m.path.as_str()).collect();
    let actual: BTreeSet<_> = result.modules.iter().map(String::as_str).collect();
    let types: BTreeSet<_> = config
        .modules
        .iter()
        .flat_map(|m| m.expected_types.iter().map(String::as_str))
        .collect();
    let source = source_digest(captured)?;
    let rules = content_digest(&serde_json::to_vec(policy.rules()).map_err(|e| e.to_string())?);
    // 阳性归属与覆盖完整性独立：Partial也必须验证所有可用身份，不能用incomplete绕过来源。
    for (actual, expected) in [
        (&result.source_digest, &source),
        (&result.tool_digest, expected_tools),
        (&result.rule_digest, &rules),
    ] {
        if actual.as_ref().is_some_and(|value| value != expected) {
            return Err("partial or complete result identity mismatch".into());
        }
    }
    if !result.violations.is_empty()
        && (result.source_digest.is_none()
            || result.tool_digest.is_none()
            || result.rule_digest.is_none()
            || result.bytecode_digest.is_none())
    {
        return Err("unbound partial findings retained only in private raw artifact".into());
    }
    if !actual.is_subset(&modules)
        || result.modules.len() != actual.len()
        || result.class_count as usize > types.len()
    {
        return Err("result contains unconfigured module/type coverage".into());
    }
    if result.completeness == "complete"
        && (modules != actual || result.class_count as usize != types.len())
    {
        return Err("bytecode complete module/type coverage mismatch".into());
    }
    let sources: BTreeSet<_> = captured
        .manifest
        .iter()
        .map(|f| String::from_utf8(f.path.to_bytes()).unwrap_or_default())
        .collect();
    for violation in &result.violations {
        let rule = policy
            .rules()
            .iter()
            .find(|r| r.rule_id() == violation.rule_id)
            .ok_or("unknown violation rule")?;
        let member =
            |name: &str, prefix: &str| name == prefix || name.starts_with(&format!("{prefix}."));
        if !types.contains(violation.from.as_str())
            || !member(&violation.from, rule.from())
            || !rule.forbidden_to().iter().any(|p| member(&violation.to, p))
            || !sources.contains(&violation.file)
            || !config.modules.iter().any(|module| {
                module.expected_types.contains(&violation.from)
                    && violation.file
                        == format!(
                            "{}src/main/java/{}.java",
                            if module.path.is_empty() {
                                String::new()
                            } else {
                                format!("{}/", module.path)
                            },
                            violation
                                .from
                                .split('$')
                                .next()
                                .unwrap_or("")
                                .replace('.', "/")
                        )
            })
            || !violation.file.ends_with(".java")
            || violation.description.len() > 65536
        {
            return Err("violation target/rule/source attribution mismatch".into());
        }
    }
    Ok(result)
}
#[allow(clippy::too_many_arguments)]
fn persist(
    store: &EvidenceStore,
    operation: &str,
    retain: u64,
    mut report: BytecodeReport,
    config: &BytecodeConfig,
    policy: &BytecodePolicy,
    captured: &CapturedSubject,
    raw: &[u8],
    tools: &Digest,
) -> Result<BytecodeReceipt, String> {
    match assess(config, policy, captured, raw, tools) {
        Ok(result) => {
            let expected_exit = if result.violations.is_empty() { 0 } else { 2 };
            if result.completeness == "complete"
                && report.exit_code == Some(expected_exit)
                && report.stop_reason == "Exited"
                && ((expected_exit == 0 && report.execution_status == ExecutionStatus::Completed)
                    || (expected_exit == 2 && report.execution_status == ExecutionStatus::Failed))
                && !report.input_drifted
                && !report.output_truncated
                && report.started_at.is_some()
                && report.finished_at.is_some()
                && report.diagnostics.is_empty()
            {
                report.status = if expected_exit == 0 {
                    "passed"
                } else {
                    "violations"
                }
                .into();
            } else {
                report.diagnostics.push(
                    "execution or coverage incomplete; retained findings cannot imply pass".into(),
                );
            }
            report.result = Some(result)
        }
        Err(error) => report.diagnostics.push(error),
    }
    let bytes = serde_json::to_vec(&report).map_err(|e| e.to_string())?;
    let artifact = put(store, &format!("{operation}.report"), &bytes, retain)?;
    Ok(BytecodeReceipt { report, artifact })
}
/// 在明确声明的本地身份下采集、私有物化并执行单个完整Java流水线；不注册批准或交付权限。
/// 同步准备有界于Subject库60秒；宿主需在阻塞工作线程调用此函数，执行预算仅约束实际进程。
#[allow(clippy::too_many_arguments)]
pub fn check_local(
    project: &Path,
    scratch: &Path,
    config: &BytecodeConfig,
    policy: &BytecodePolicy,
    selection: &Selection,
    store: &EvidenceStore,
    operation: &str,
    retain: u64,
    cancel: Cancellation,
) -> Result<BytecodeReceipt, String> {
    check_local_bound(
        project, scratch, config, policy, selection, store, operation, retain, cancel, None,
    )
}
#[allow(clippy::too_many_arguments)]
fn check_local_bound(
    project: &Path,
    scratch: &Path,
    config: &BytecodeConfig,
    policy: &BytecodePolicy,
    selection: &Selection,
    store: &EvidenceStore,
    operation: &str,
    retain: u64,
    cancel: Cancellation,
    previous_report: Option<BytecodeArtifactRef>,
) -> Result<BytecodeReceipt, String> {
    config.validate()?;
    if policy.is_contract_bound() {
        return Err("contract policy requires registered execution API".into());
    }
    let request = BytecodePipelineRequest::new(config, policy)?;
    let logical = request.command(config, project)?;
    let config_digest = NativeVerifierCommandV2::configuration_digest(&logical)?;
    let tools = bytecode_paths::tools(config)?;
    let pd = policy_digest(policy)?;
    let binding = CaptureBinding {
        repository_id: config.repository_id.clone(),
        task_id: config.task_id.clone(),
        worktree_id: config.worktree_id.clone(),
        contract_digest: pd.clone(),
        policy_digest: pd.clone(),
        verifier_digest: config_digest.clone(),
        environment_digest: NativeVerifierCommandV2::environment_digest(&config.environment)?,
    };
    let captured = guard_subject::capture(project, &binding, selection)?;
    ensure_supported_sources(config, &captured)?;
    let workspace = guard_subject::materialize(project, &binding, selection, &captured, scratch)?;
    let root = bytecode_paths::native(workspace.path())?;
    let parent =
        bytecode_paths::native(workspace.path().parent().ok_or("private parent required")?)?;
    let mut actual = logical;
    actual.directory = std::path::PathBuf::from(&root);
    for argument in &mut actual.args {
        if argument == "{{subject_root}}" {
            *argument = root.clone();
        }
    }
    actual
        .environment
        .insert("GIT_CEILING_DIRECTORIES".into(), parent);
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    workspace.verify_inputs()?;
    guard_subject::verify_current(project, &binding, selection, &captured)?;
    let output =
        executor.block_on(ProcessRunner::new(config.budget.clone())?.run(actual, cancel))?;
    let stdout = put(
        store,
        &format!("{operation}.stdout"),
        &output.stdout,
        retain,
    )?;
    let stderr = put(
        store,
        &format!("{operation}.stderr"),
        &output.stderr,
        retain,
    )?;
    let mut diagnostics = Vec::new();
    let drift = workspace
        .verify_inputs()
        .and_then(|_| guard_subject::verify_current(project, &binding, selection, &captured))
        .err();
    if let Some(error) = &drift {
        diagnostics.push(error.clone());
    }
    match bytecode_paths::tools(config) {
        Ok(after) if after == tools => {}
        Ok(_) => diagnostics.push("tool manifest drift".into()),
        Err(error) => diagnostics.push(error),
    }
    let report = BytecodeReport {
        schema_version: 1,
        mode: "local".into(),
        trust: "local_unverified".into(),
        approval: None,
        status: "incomplete".into(),
        subject_digest: captured.identity_digest.clone(),
        config_digest,
        policy_digest: pd,
        context_digest: canonical_digest(config)?,
        candidate_kind: kind(selection),
        selection_scope_digest: selection_scope_digest(selection)?,
        retain_until_ms: retain,
        previous_report,
        stop_reason: format!("{:?}", output.reason),
        execution_status: output.status,
        exit_code: output.exit_code,
        started_at: output.started_at,
        finished_at: output.finished_at,
        input_drifted: drift.is_some(),
        output_truncated: output.output_truncated,
        diagnostics,
        result: None,
        stdout,
        stderr,
        execution_observation: None,
    };
    persist(
        store,
        operation,
        retain,
        report,
        config,
        policy,
        &captured,
        &output.stdout,
        &tools,
    )
}
pub(crate) async fn blocking<T: Send + 'static>(
    action: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    static SLOTS: std::sync::OnceLock<Arc<tokio::sync::Semaphore>> = std::sync::OnceLock::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    let slots = SLOTS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(16)))
        .clone();
    let permit = tokio::time::timeout_at(deadline, slots.acquire_owned())
        .await
        .map_err(|_| "bounded bytecode preparation queue timeout")?
        .map_err(|e| e.to_string())?;
    let task = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        action()
    });
    tokio::time::timeout_at(deadline, task)
        .await
        .map_err(|_| "bounded bytecode preparation timeout")?
        .map_err(|e| e.to_string())?
}
/// 通过不可反序列化的真实注册计划执行；完整源码与POM目标必须在该义务精确范围中。
#[allow(clippy::too_many_arguments)]
pub async fn check_registered(
    project: &Path,
    scratch: &Path,
    config: &BytecodeConfig,
    contract: &CompiledContract,
    plan: &RegisteredPlan,
    obligation: &Identifier,
    store: Arc<EvidenceStore>,
    operation: &str,
    retain: u64,
    cancel: Cancellation,
) -> Result<BytecodeRunOutcome, String> {
    config.validate()?;
    if config.repository_id != plan.capture_binding().repository_id
        || config.task_id != plan.binding().task_id
        || config.worktree_id != plan.binding().worktree_id
        || config.budget != plan.plan().verification().budget
    {
        return Err("configuration differs from registered binding or budget".into());
    }
    let policy = BytecodePolicy::from_contract(contract, &config.verifier_id)?;
    let captured = plan.captured_subject();
    ensure_supported_sources(config, captured)?;
    let frozen = plan
        .plan()
        .verification()
        .obligations
        .iter()
        .find(|o| &o.obligation_id == obligation)
        .ok_or("obligation missing")?;
    if frozen.verifier_id != config.verifier_id
        || !policy
            .rules()
            .iter()
            .any(|r| r.rule_id() == frozen.rule_id.as_str())
    {
        return Err("obligation rule/verifier differs from bytecode policy".into());
    }
    let mut targets = BTreeSet::new();
    for t in &frozen.targets {
        if t.selector.is_some() || t.repository_id != config.repository_id {
            return Err("bytecode selector/cross-repository target unsupported".into());
        }
        targets.insert(String::from_utf8(t.path.to_bytes()).map_err(|e| e.to_string())?);
    }
    for file in &captured.manifest {
        let name = String::from_utf8(file.path.to_bytes()).map_err(|e| e.to_string())?;
        if (name.ends_with(".java") || name == "pom.xml" || name.ends_with("/pom.xml"))
            && !targets.contains(&name)
        {
            return Err("registered obligation omits actual Java/POM input".into());
        }
    }
    for module in &config.modules {
        let pom = if module.path.is_empty() {
            "pom.xml".into()
        } else {
            format!("{}/pom.xml", module.path)
        };
        if !targets.contains(&pom) {
            return Err("expected module missing from frozen targets".into());
        }
    }
    let request = BytecodePipelineRequest::new(config, &policy)?;
    let command = request.command(config, project)?;
    let config_digest = NativeVerifierCommandV2::configuration_digest(&command)?;
    let owned_config = config.clone();
    let owned_plan = plan.clone();
    let owned_contract = contract.clone();
    let owned_store = store.clone();
    let owned_project = project.to_path_buf();
    let owned_scratch = scratch.to_path_buf();
    let (tools, runtime) = blocking(move || {
        let tools = bytecode_paths::tools(&owned_config)?;
        let runtime = Runtime::new_v2(
            &owned_plan,
            &owned_contract,
            owned_store,
            &owned_project,
            retain,
            &owned_scratch,
        )?;
        Ok((tools, runtime))
    })
    .await?;
    let observation = match runtime
        .execute_v2(
            obligation,
            operation,
            NativeVerifierCommandV2 {
                verifier_id: config.verifier_id.clone(),
                command,
            },
            cancel,
        )
        .await?
    {
        DispatchOutcome::Existing { sequence } => {
            return Ok(BytecodeRunOutcome::Existing { sequence });
        }
        DispatchOutcome::Finished(value) => value,
    };
    let stdout = store.read_private(&observation.stdout.operation, now()?)?;
    let metadata = put(
        &store,
        &format!("{operation}.professional-observation"),
        &serde_json::to_vec(&observation).map_err(|e| e.to_string())?,
        retain,
    )?;
    let mut diagnostics = Vec::new();
    let after_config = config.clone();
    match blocking(move || bytecode_paths::tools(&after_config)).await {
        Ok(after) if after == tools => {}
        Ok(_) => diagnostics.push("tool manifest drift".into()),
        Err(e) => diagnostics.push(e),
    }
    if let Some(e) = &observation.recheck_error {
        diagnostics.push(e.clone());
    }
    let report = BytecodeReport {
        schema_version: 1,
        mode: "registered".into(),
        trust: "local_unverified".into(),
        approval: None,
        status: "incomplete".into(),
        subject_digest: captured.identity_digest.clone(),
        config_digest,
        policy_digest: policy_digest(&policy)?,
        context_digest: canonical_digest(config)?,
        candidate_kind: kind(&captured.selection),
        selection_scope_digest: selection_scope_digest(&captured.selection)?,
        retain_until_ms: retain,
        previous_report: None,
        stop_reason: observation.stop_reason.clone(),
        execution_status: observation.execution_status,
        exit_code: observation.exit_code,
        started_at: observation.started_at.clone(),
        finished_at: observation.finished_at.clone(),
        input_drifted: observation.input_drifted,
        output_truncated: observation.output_truncated,
        diagnostics,
        result: None,
        stdout: BytecodeArtifactRef {
            operation: observation.stdout.operation.clone(),
            digest: observation.stdout.digest.clone(),
            size: observation.stdout.size,
        },
        stderr: BytecodeArtifactRef {
            operation: observation.stderr.operation.clone(),
            digest: observation.stderr.digest.clone(),
            size: observation.stderr.size,
        },
        execution_observation: Some(metadata),
    };
    Ok(BytecodeRunOutcome::Finished(Box::new(persist(
        &store, operation, retain, report, config, &policy, captured, &stdout, &tools,
    )?)))
}
/// 在真实旧原件核验后重编译修复候选；配置或规则范围变化需新检查，不静默沿用旧报告。
#[allow(clippy::too_many_arguments)]
pub fn verify_local(
    previous: &BytecodeReceipt,
    project: &Path,
    scratch: &Path,
    config: &BytecodeConfig,
    policy: &BytecodePolicy,
    selection: &Selection,
    store: &EvidenceStore,
    operation: &str,
    retain: u64,
    cancel: Cancellation,
) -> Result<BytecodeReceipt, String> {
    previous.verify(store, now()?)?;
    let request = BytecodePipelineRequest::new(config, policy)?;
    let digest = NativeVerifierCommandV2::configuration_digest(&request.command(config, project)?)?;
    if previous.report.mode != "local"
        || previous.report.config_digest != digest
        || previous.report.policy_digest != policy_digest(policy)?
        || previous.report.context_digest != canonical_digest(config)?
        || previous.report.candidate_kind != kind(selection)
        || previous.report.selection_scope_digest != selection_scope_digest(selection)?
    {
        return Err("previous report rule/configuration scope drift".into());
    }
    check_local_bound(
        project,
        scratch,
        config,
        policy,
        selection,
        store,
        operation,
        retain,
        cancel,
        Some(previous.artifact.clone()),
    )
}

#[cfg(test)]
#[path = "bytecode_assessment_tests.rs"]
mod tests;
