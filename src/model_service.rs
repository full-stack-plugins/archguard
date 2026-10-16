use crate::{
    architecture_model::{ArchitectureModel, identity},
    baseline_record::BaselineRecord,
    baseline_store::{self, digest, read},
    change_inspection::ChangeInspection,
    model_change::ModelChange,
};
use guardengine::{Completeness, Decision, GuardContract, GuardFacts};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

fn baseline(root: &Path) -> Result<(String, BaselineRecord), String> {
    baseline_store::load(root)?
        .ok_or_else(|| "project architecture baseline is not configured".into())
}
fn inspect(root: &Path, id: &str) -> Result<ChangeInspection, String> {
    if !identity(id) {
        return Err("change ID must be a safe stable identifier".into());
    }
    let (base_digest, baseline) = baseline(root)?;
    let change: ModelChange = read(root, &root.join(format!(".archguard/changes/{id}.json")))?;
    if change.id != id || change.base_digest != base_digest {
        return Err("change ID differs from filename or baseline is stale; review delta against current baseline".into());
    }
    let specification = change.requirement.inspect(root)?;
    let (candidate, seeds) = change.apply(&baseline.model)?;
    Ok(ChangeInspection {
        base_digest,
        baseline,
        change,
        candidate,
        seeds,
        specification,
    })
}
fn checked(
    root: &Path,
    model: &ArchitectureModel,
    analyzer: &str,
    expected_contract: Option<&str>,
) -> Result<(Value, Decision), String> {
    model.validate()?;
    let (contract, facts, report) = crate::check_service::check_unbound(root, None, analyzer)?;
    if contract.api_version != guardengine::API_VERSION_V2 {
        return Err("project model workflow requires a reviewed v1alpha2 contract".into());
    }
    if expected_contract.is_some_and(|expected| expected != report.contract_digest) {
        return Err(
            "contract changed from model baseline; explicit reviewed contract migration required"
                .into(),
        );
    }
    let findings = model.compare(&facts);
    let java = crate::java_source::check(root, model, analyzer);
    let java_pass = java.as_ref().is_none_or(|value| {
        value["complete"] == true && value["findings"].as_array().is_some_and(Vec::is_empty)
    });
    let decision = if findings.is_empty() && java_pass {
        report.decision.clone()
    } else {
        Decision::Block
    };
    let mut result = json!({"schema_version":1,"kind":"ArchitectureModelCheck","version":env!("CARGO_PKG_VERSION"),"decision":decision,"project":root,"model_digest":digest(model)?,"model_findings":findings,"architecture_report":report,"coverage":{"build":facts.coverage,"model":"declared identities, ownership and relations; exact internal module/dependency conformance","excluded":["compiler type inference and domain business invariants","runtime calls","approval authority"],"check_scope":"full project"},"approval_verified":false,"signed":false});
    if let Some(java) = java {
        result["java_analysis"] = java;
    }
    Ok((result, decision))
}

/// 获取当前基线摘要和版本供智能体发现，不执行架构判定。
pub fn context(root: &Path) -> Result<Value, String> {
    match baseline_store::load(root)? {
        Some((hash, record)) => Ok(
            json!({"status":"configured","digest":hash,"revision":record.revision,"project":record.model.project,"elements":record.model.elements.len(),"relations":record.model.relations.len(),"java_source_configured":record.model.java.is_some(),"evaluated":false}),
        ),
        None => Ok(json!({"status":"unconfigured","evaluated":false})),
    }
}
/// 返回完整目标模型或指定元素的一跳邻域；视图不是第二套可写模型。
pub fn show(root: &Path, entity: Option<&str>) -> Result<Value, String> {
    let (hash, record) = baseline(root)?;
    let mut model = record.model;
    if let Some(id) = entity {
        if !model.elements.iter().any(|e| e.id == id) {
            return Err("unknown model entity".into());
        }
        let mut selected = BTreeSet::from([id.to_owned()]);
        for relation in &model.relations {
            if relation.from == id {
                selected.insert(relation.to.clone());
            }
            if relation.to == id {
                selected.insert(relation.from.clone());
            }
        }
        for element in &model.elements {
            for parent in [&element.module, &element.context].into_iter().flatten() {
                if element.id == id {
                    selected.insert(parent.clone());
                }
                if parent == id {
                    selected.insert(element.id.clone());
                }
            }
        }
        model.elements.retain(|e| selected.contains(&e.id));
        model
            .relations
            .retain(|r| selected.contains(&r.from) && selected.contains(&r.to));
    }
    Ok(
        json!({"digest":hash,"revision":record.revision,"scope":if entity.is_some(){"one-hop read-only view"}else{"whole project"},"model":model}),
    )
}
/// 验证输入模型结构或当前基线结构；不证明代码已实现模型。
pub fn validate(root: &Path, file: Option<&Path>) -> Result<Value, String> {
    let model: ArchitectureModel = match file {
        Some(path) => read(root, &root.join(path))?,
        None => baseline(root)?.1.model,
    };
    model.validate()?;
    Ok(json!({"status":"valid","model_digest":digest(&model)?,"implementation_checked":false}))
}
/// 分析需求增量的声明关系影响，返回路径；不使用影响子集缩小实际检查范围。
pub fn impact(root: &Path, id: &str) -> Result<Value, String> {
    let data = inspect(root, id)?;
    let mut result =
        crate::model_impact::analyze(&data.baseline.model, &data.candidate, &data.seeds);
    result["base_digest"] = json!(data.base_digest);
    result["change_digest"] = json!(digest(&data.change)?);
    result["specification"] = data.specification;
    Ok(result)
}
/// 检查候选模型及整个实际构建图，返回绑定所有当前输入的报告及决策。
pub fn check_change(root: &Path, id: &str, analyzer: &str) -> Result<(Value, Decision), String> {
    let data = inspect(root, id)?;
    let (mut report, decision) = checked(
        root,
        &data.candidate,
        analyzer,
        Some(&data.baseline.contract_digest),
    )?;
    report["base_digest"] = json!(data.base_digest);
    report["change_id"] = json!(id);
    report["change_digest"] = json!(digest(&data.change)?);
    report["specification"] = data.specification;
    report["impact"] =
        crate::model_impact::analyze(&data.baseline.model, &data.candidate, &data.seeds);
    report["remediation"] = json!(
        "按 model_findings、java_analysis.findings 和 architecture_report 的文件、行号及依赖链修复源码/声明；不得为通过检查擅自放宽模型或契约。修改后执行业务测试并重新 change check、change verify，审阅后显式 baseline update --apply。"
    );
    // 前后重读控制输入，发现分析期间变化即拒绝，避免拼接不同版本。
    let after = inspect(root, id)?;
    if after.base_digest != data.base_digest
        || digest(&after.change)? != report["change_digest"]
        || after.specification != report["specification"]
    {
        return Err("model/change/native source changed during analysis".into());
    }
    Ok((report, decision))
}
/// 对照重新计算的报告验证已有文件；不相信其自报摘要或决策。
pub fn verify(root: &Path, file: &Path, current: &Value) -> Result<(), String> {
    let saved: Value = read(root, &root.join(file))?;
    if &saved != current {
        return Err("model report is stale or modified; recompute current inputs".into());
    }
    Ok(())
}
/// 将当前项目基线纳入普通架构检查，模型不一致使整体覆盖无法证明并阻断。
pub(crate) fn bind_current(
    root: &Path,
    contract: &GuardContract,
    facts: &mut GuardFacts,
) -> Result<(), String> {
    if let Some((hash, record)) = baseline_store::load(root)? {
        if digest(contract)? != record.contract_digest {
            return Err("contract differs from active model baseline".into());
        }
        let findings = record.model.compare(facts);
        let analyzer = if facts.analyzer.id == "archguard.maven.reactor" {
            "maven"
        } else {
            "cargo"
        };
        let java = crate::java_source::check(root, &record.model, analyzer);
        facts.subject.snapshot_digest =
            digest(&json!({"build":facts.subject.snapshot_digest,"baseline":hash}))?;
        if !findings.is_empty() {
            facts.completeness = Completeness::Partial;
            facts.diagnostics.push(format!(
                "active project model conformance not established: {findings:?}"
            ));
        }
        if let Some(java) = java {
            facts.subject.snapshot_digest =
                digest(&json!({"build_and_model":facts.subject.snapshot_digest,"java":java}))?;
            if java["complete"] != true || !java["findings"].as_array().is_some_and(Vec::is_empty) {
                facts.completeness = Completeness::Partial;
                facts
                    .diagnostics
                    .push(format!("Java source conformance not established: {java}"));
            }
        }
    }
    Ok(())
}

fn prepare_init(
    root: &Path,
    file: &Path,
    analyzer: &str,
) -> Result<(BaselineRecord, Value, Decision), String> {
    if baseline_store::load(root)?.is_some() {
        return Err("baseline already exists; use a reviewed requirement delta".into());
    }
    let model: ArchitectureModel = read(root, &root.join(file))?;
    let (report, decision) = checked(root, &model, analyzer, None)?;
    let record = BaselineRecord {
        schema_version: 1,
        revision: 1,
        parent: None,
        model,
        contract_digest: report["architecture_report"]["contractDigest"]
            .as_str()
            .unwrap()
            .into(),
        change_id: None,
        change_digest: None,
        report_digest: digest(&report)?,
    };
    Ok((record, report, decision))
}
fn prepare_update(
    root: &Path,
    id: &str,
    file: &Path,
    analyzer: &str,
) -> Result<(BaselineRecord, Value, Decision), String> {
    let (report, decision) = check_change(root, id, analyzer)?;
    verify(root, file, &report)?;
    let data = inspect(root, id)?;
    if report["base_digest"] != data.base_digest
        || report["change_digest"] != digest(&data.change)?
        || report["specification"] != data.specification
    {
        return Err("update inputs changed".into());
    }
    let record = BaselineRecord {
        schema_version: 1,
        revision: data
            .baseline
            .revision
            .checked_add(1)
            .ok_or("revision overflow")?,
        parent: Some(data.base_digest),
        model: data.candidate,
        contract_digest: data.baseline.contract_digest,
        change_id: Some(id.into()),
        change_digest: Some(digest(&data.change)?),
        report_digest: digest(&report)?,
    };
    Ok((record, report, decision))
}
/// 预览或显式初始化基线；只有完整 ALLOW 且显式 apply 才写入，现有版本不可覆盖。
pub fn initialize(
    root: &Path,
    file: &Path,
    analyzer: &str,
    apply: bool,
) -> Result<(Value, Decision), String> {
    let prepared = prepare_init(root, file, analyzer)?;
    publish(root, prepared, apply, || prepare_init(root, file, analyzer))
}
/// 预览或显式推进需求基线；在写锁内重新检查全部输入并拒绝旧证据。
pub fn update(
    root: &Path,
    id: &str,
    file: &Path,
    analyzer: &str,
    apply: bool,
) -> Result<(Value, Decision), String> {
    let prepared = prepare_update(root, id, file, analyzer)?;
    publish(root, prepared, apply, || {
        prepare_update(root, id, file, analyzer)
    })
}
fn publish(
    root: &Path,
    prepared: (BaselineRecord, Value, Decision),
    apply: bool,
    recheck: impl FnOnce() -> Result<(BaselineRecord, Value, Decision), String>,
) -> Result<(Value, Decision), String> {
    let (record, report, decision) = prepared;
    let hash = digest(&record)?;
    if apply && decision == Decision::Allow {
        crate::input_snapshot::safe_file(root, &root.join(".archguard"))?;
        let _lock = crate::baseline_lock::BaselineLock::acquire(root)?;
        let (fresh, fresh_report, fresh_decision) = recheck()?;
        if digest(&fresh)? != hash || fresh_report != report || fresh_decision != Decision::Allow {
            return Err("baseline candidate changed while acquiring lock".into());
        }
        baseline_store::publish(root, record.parent.as_deref(), &record)?;
    }
    Ok((
        json!({"applied":apply && decision==Decision::Allow,"digest":hash,"revision":record.revision,"parent":record.parent,"decision":decision,"check":report,"approval_verified":false}),
        decision,
    ))
}
