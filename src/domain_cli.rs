use guardengine::Decision;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// 执行专业命令，返回 JSON 输出和决策；未知参数拒绝执行。
pub fn run(args: &[String]) -> Result<(Value, Decision), String> {
    let command = args.first().map(String::as_str).ok_or("command required")?;
    if command == "bytecode" {
        return crate::bytecode_cli::run(&args[1..]);
    }
    if ["model", "change", "baseline"].contains(&command) {
        return crate::model_cli::run(args);
    }
    let offset = if command == "contract" {
        if args.get(1).map(String::as_str) != Some("validate") {
            return Err("expected contract validate".into());
        }
        2
    } else {
        1
    };
    let allowed: &[&str] = match command {
        "check" => &[
            "--project",
            "--contract",
            "--analyzer",
            "--report",
            "--facts",
        ],
        "explain" => &["--project", "--contract", "--analyzer", "--rule"],
        "verify" => &["--project", "--contract", "--analyzer", "--report"],
        "contract" => &["--project", "--contract"],
        _ => return Err("expected check, explain, verify or contract validate".into()),
    };
    let mut options = BTreeMap::new();
    for pair in args[offset..].chunks(2) {
        if pair.len() != 2
            || !allowed.contains(&pair[0].as_str())
            || pair[1].starts_with("--")
            || options.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err("unknown, duplicate or incomplete option".into());
        }
    }
    let project = Path::new(options.get("--project").copied().unwrap_or("."));
    let contract = options.get("--contract").map(Path::new);
    if command == "contract" {
        let default = project.join(".archguard/contract.yaml");
        let parsed = crate::check_service::contract(contract.unwrap_or(&default))?;
        return Ok((json!({"status":"valid","contract":parsed}), Decision::Allow));
    }
    let (parsed, facts, report) = crate::check_service::check(
        project,
        contract,
        options.get("--analyzer").copied().unwrap_or("auto"),
    )?;
    let decision = report.decision.clone();
    match command {
        "explain" => Ok((
            crate::check_service::explain(
                &parsed,
                &report,
                options.get("--rule").ok_or("--rule required")?,
            )?,
            decision,
        )),
        "verify" => crate::check_service::verify(
            Path::new(options.get("--report").ok_or("--report required")?),
            &report,
        ),
        _ => {
            let mut outputs = Vec::new();
            for (flag, value) in [("--report", json!(report)), ("--facts", json!(facts))] {
                if let Some(path) = options.get(flag) {
                    outputs.push((
                        PathBuf::from(path),
                        format!(
                            "{}\n",
                            serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
                        ),
                    ));
                }
            }
            crate::report_output::write_new(&outputs)?;
            Ok((json!(report), decision))
        }
    }
}
