use guardengine::Decision;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

/// 解析项目模型命令；默认只读，基线更新仅在显式 --apply 时执行。
pub(crate) fn run(args: &[String]) -> Result<(Value, Decision), String> {
    let family = args.first().map(String::as_str).unwrap_or("");
    let action = args
        .get(1)
        .map(String::as_str)
        .ok_or("model action required")?;
    let allowed: &[&str] = match (family, action) {
        ("model", "show") => &["--project", "--entity"],
        ("model", "validate") => &["--project", "--model"],
        ("change", "impact") => &["--project", "--change"],
        ("change", "check" | "verify") => &["--project", "--change", "--analyzer", "--report"],
        ("baseline", "init") => &["--project", "--model", "--analyzer", "--apply"],
        ("baseline", "update") => &["--project", "--change", "--analyzer", "--report", "--apply"],
        _ => return Err("unknown model/change/baseline action".into()),
    };
    let mut options = BTreeMap::new();
    let mut index = 2;
    let mut apply = false;
    while index < args.len() {
        let key = args[index].as_str();
        if !allowed.contains(&key) || options.contains_key(key) {
            return Err("unknown or duplicate model option".into());
        }
        if key == "--apply" {
            apply = true;
            options.insert(key, "");
            index += 1;
            continue;
        }
        let value = args
            .get(index + 1)
            .filter(|s| !s.is_empty() && !s.starts_with("--"))
            .ok_or("model option requires value")?;
        options.insert(key, value.as_str());
        index += 2;
    }
    let required = |key: &str| {
        options
            .get(key)
            .copied()
            .ok_or_else(|| format!("{key} required"))
    };
    let project = Path::new(required("--project")?);
    if !project.is_absolute() || !project.is_dir() {
        return Err("model workflow requires an absolute project directory".into());
    }
    let root = project.canonicalize().map_err(|e| e.to_string())?;
    let analyzer = options.get("--analyzer").copied().unwrap_or("auto");
    match (family, action) {
        ("model", "show") => Ok((
            crate::model_service::show(&root, options.get("--entity").copied())?,
            Decision::Allow,
        )),
        ("model", "validate") => Ok((
            crate::model_service::validate(&root, options.get("--model").map(Path::new))?,
            Decision::Allow,
        )),
        ("change", "impact") => Ok((
            crate::model_service::impact(&root, required("--change")?)?,
            Decision::Allow,
        )),
        ("change", "check" | "verify") => {
            let (report, decision) =
                crate::model_service::check_change(&root, required("--change")?, analyzer)?;
            if action == "verify" {
                crate::model_service::verify(&root, Path::new(required("--report")?), &report)?;
                Ok((json!({"status":"verified","report":report}), decision))
            } else {
                if let Some(file) = options.get("--report") {
                    let path = root.join(file);
                    // 导出与复验采用相同项目根，提前拒绝无法复验的外部报告路径。
                    crate::input_snapshot::safe_file(
                        &root,
                        path.parent().ok_or("report parent required")?,
                    )?;
                    crate::report_output::write_new(&[(
                        path,
                        format!(
                            "{}\n",
                            serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?
                        ),
                    )])?;
                }
                Ok((report, decision))
            }
        }
        ("baseline", "init") => crate::model_service::initialize(
            &root,
            Path::new(required("--model")?),
            analyzer,
            apply,
        ),
        ("baseline", "update") => crate::model_service::update(
            &root,
            required("--change")?,
            Path::new(required("--report")?),
            analyzer,
            apply,
        ),
        _ => unreachable!(),
    }
}
