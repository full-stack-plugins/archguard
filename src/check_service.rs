use crate::{CargoWorkspaceAnalyzer, MavenWorkspaceAnalyzer, input_snapshot::read_bounded};
use guardengine::{
    Decision, GuardAnalyzer, GuardContract, GuardFacts, GuardReport, evaluate, load_contract_yaml,
};
use serde_json::{Value, json};
use std::path::Path;

/// 从指定文件读取并校验契约，返回结构化契约；不执行分析。
pub fn contract(path: &Path) -> Result<GuardContract, String> {
    load_contract_yaml(&read_bounded(path)?).map_err(|e| e.to_string())
}

/// 检查当前项目，返回契约、事实和报告；自动选择唯一构建系统。
pub fn check(
    project: &Path,
    contract_path: Option<&Path>,
    analyzer: &str,
) -> Result<(GuardContract, GuardFacts, GuardReport), String> {
    let (parsed, mut facts, _) = check_unbound(project, contract_path, analyzer)?;
    let root = project.canonicalize().map_err(|e| e.to_string())?;
    crate::model_service::bind_current(&root, &parsed, &mut facts)?;
    let report = evaluate(&parsed, &facts).map_err(|e| e.to_string())?;
    Ok((parsed, facts, report))
}

/// 仅检查实际构建与既有契约，供候选模型检查使用；避免混入旧基线的模型约束。
pub(crate) fn check_unbound(
    project: &Path,
    contract_path: Option<&Path>,
    analyzer: &str,
) -> Result<(GuardContract, GuardFacts, GuardReport), String> {
    let root = project.canonicalize().map_err(|e| e.to_string())?;
    let default_contract = root.join(".archguard/contract.yaml");
    let parsed = contract(contract_path.unwrap_or(&default_contract))?;
    let selected = match analyzer {
        "auto" => match (
            root.join("pom.xml").is_file(),
            root.join("Cargo.toml").is_file(),
        ) {
            (true, false) => "maven",
            (false, true) => "cargo",
            _ => return Err(
                "auto requires exactly one of pom.xml or Cargo.toml; select --analyzer explicitly"
                    .into(),
            ),
        },
        "maven" | "cargo" => analyzer,
        _ => return Err("analyzer must be auto, maven or cargo".into()),
    };
    let identity = root.to_string_lossy();
    let mut facts = if selected == "maven" {
        MavenWorkspaceAnalyzer.analyze(&root, &identity)
    } else {
        CargoWorkspaceAnalyzer.analyze(&root, &identity)
    }
    .map_err(|e| e.to_string())?;
    crate::spec_binding::bind(&root, &parsed, &mut facts)?;
    let report = evaluate(&parsed, &facts).map_err(|e| e.to_string())?;
    Ok((parsed, facts, report))
}

/// 解释指定规则的最新判定；修复建议不改变契约或批准状态。
pub fn explain(
    contract: &GuardContract,
    report: &GuardReport,
    rule: &str,
) -> Result<Value, String> {
    let evaluation = report
        .evaluations
        .iter()
        .find(|item| item.rule_id == rule)
        .ok_or("unknown rule ID")?;
    let definition = contract
        .spec
        .rules
        .iter()
        .find(|item| item.id == rule)
        .ok_or("unknown rule ID")?;
    Ok(
        json!({"rule":definition,"evaluation":evaluation,"decision":report.decision,"subject":report.subject,
        "remediation":"根据 matchedFacts 的源文件及依赖链移除、反转或抽取违规依赖；必需关系缺失时补齐声明。INDETERMINATE 时先修复模型或覆盖范围。完成后重新 check，并用 verify 校验新报告。不得自动放宽契约或将 REVIEW 当作已批准。"}),
    )
}

/// 将报告与当前输入重新计算的结果比对，返回验证结果及门禁决策。
pub fn verify(path: &Path, current: &GuardReport) -> Result<(Value, Decision), String> {
    let saved: GuardReport =
        serde_json::from_slice(&read_bounded(path)?).map_err(|e| e.to_string())?;
    if &saved != current {
        return Err("report is stale or modified: current sources, contract, analyzer and engine must match".into());
    }
    Ok((
        json!({"status":"verified","report":current}),
        current.decision.clone(),
    ))
}
