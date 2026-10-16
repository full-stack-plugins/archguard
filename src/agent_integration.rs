use guard_install::{DomainTool, Options, Tool};
use serde_json::{Value, json};
use std::path::Path;

const DOMAIN_TOOLS: &[DomainTool] = &[
    DomainTool {
        name: "archguard_model_show",
        description: "Read the single project model or an entity neighborhood; no baseline changes.",
        schema: model_show_schema,
        call: model_show,
    },
    DomainTool {
        name: "archguard_model_validate",
        description: "Validate project model identities, ownership and relations; does not certify domain semantics.",
        schema: model_validate_schema,
        call: model_validate,
    },
    DomainTool {
        name: "archguard_change_impact",
        description: "Analyze a requirement delta against the current baseline and native specification; return conservative impact paths.",
        schema: change_impact_schema,
        call: change_impact,
    },
    DomainTool {
        name: "archguard_change_check",
        description: "Check a requirement candidate against the full Maven/Cargo graph and configured Java package/layer/domain ownership policy; read only, no compiler or runtime semantics.",
        schema: change_check_schema,
        call: change_check,
    },
    DomainTool {
        name: "archguard_change_verify",
        description: "Recompute a requirement check and verify an existing report; never updates the baseline.",
        schema: change_verify_schema,
        call: change_verify,
    },
    DomainTool {
        name: "archguard_check",
        description: "Check current Maven reactor or Cargo workspace against its contract. Returns ALLOW/BLOCK/REQUIRE_APPROVAL within declared coverage; does not export files.",
        schema: check_schema,
        call: check,
    },
    DomainTool {
        name: "archguard_explain",
        description: "Recompute and explain a rule with source evidence and remediation guidance.",
        schema: explain_schema,
        call: explain,
    },
    DomainTool {
        name: "archguard_verify",
        description: "Reanalyze current source and contract, then compare an existing unsigned report. Never trusts saved facts.",
        schema: verify_schema,
        call: verify,
    },
];

/// 返回工具接入能力和共享领域入口。
pub(crate) fn tool() -> Tool {
    let mut tool = Tool::new(
        "archguard",
        env!("CARGO_PKG_VERSION"),
        "修改模块依赖和领域边界前读取 .archguard/contract.yaml、已有 spec-binding.json 和 context 中的 domain.model；用 spec context 选择唯一原生规格。已有项目基线时先 model show；每个需求在 .archguard/changes/<id>.json 维护引用当前基线及原生规格的增量，用 change impact 分析影响，change check 检查整个项目，修复后重新 check 并 change verify --report。仅在明确授权后通过 CLI baseline update --apply 更新已复验且 ALLOW 的基线；MCP 只读。未配置基线时保留 contract validate、check、explain、verify 流程。插件已加载时可用 archguard-domain-modeling、archguard-grill-with-docs、archguard-codebase-design、archguard-improve-codebase-architecture、archguard-research；缺少插件时不假定技能已安装。支持 Maven/Cargo 声明依赖图、显式 Maven profile 和声明领域模型结构；模型配置 java 后检查源码包依赖、分层与 java_type 实际归属，按 java_analysis.findings 修复，不覆盖 Java 方法调用、领域实现语义或完整有效模型。缺失模块、未知谓词和不完整模型不能作为通过；不得自行放宽契约或批准 REVIEW。",
        &[
            "contract validate",
            "check",
            "explain",
            "verify",
            "model show",
            "model validate",
            "change impact",
            "change check",
            "change verify",
            "baseline init",
            "baseline update",
        ],
    );
    tool.context = Some(context);
    tool.mcp_tools = DOMAIN_TOOLS;
    tool
}
fn context(options: &Options) -> Result<Value, String> {
    let binding = archguard::spec_binding::context(&options.project)?;
    let model = match archguard::model_service::context(&options.project) {
        Ok(value) => value,
        Err(message) => json!({"status":"invalid","message":message,"evaluated":false}),
    };
    let path = options.project.join(".archguard/contract.yaml");
    if !path.exists() {
        return Ok(
            json!({"status":"unconfigured","contract":path,"spec_binding":binding,"model":model,"reason":"No project architecture contract; do not infer compliance."}),
        );
    }
    match archguard::check_service::contract(&path) {
        Ok(contract) => Ok(
            json!({"status":"configured","contract":path,"spec_binding":binding,"model":model,"contract_digest":guardengine::digest_json(&contract).map_err(|e|e.to_string())?,"metadata":contract.metadata,"rules":contract.spec.rules,"evaluated":false}),
        ),
        Err(error) => {
            Ok(json!({"status":"invalid","contract":path,"diagnostic":error,"evaluated":false}))
        }
    }
}
fn schema(extra: Option<&str>) -> Value {
    let mut value = json!({"type":"object","properties":{"project":{"type":"string"},"contract":{"type":"string"},"analyzer":{"type":"string","enum":["auto","maven","cargo"]}},"required":["project"],"additionalProperties":false});
    if let Some(key) = extra {
        value["properties"][key] = json!({"type":"string"});
        value["required"].as_array_mut().unwrap().push(json!(key));
    }
    value
}
fn check_schema() -> Value {
    schema(None)
}
fn explain_schema() -> Value {
    schema(Some("rule"))
}
fn verify_schema() -> Value {
    schema(Some("report"))
}
fn invoke(args: &Value, command: &str, extra: Option<&str>) -> Result<Value, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    let mut cli = vec![command.to_owned()];
    if !object.contains_key("project") {
        return Err("absolute project required".into());
    }
    if extra.is_some_and(|key| !object.contains_key(key)) {
        return Err("required argument missing".into());
    }
    for (key, value) in object {
        if !["project", "contract", "analyzer"].contains(&key.as_str())
            && Some(key.as_str()) != extra
        {
            return Err("unknown argument; MCP does not accept output paths".into());
        }
        let value = value
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or("arguments must be nonempty strings")?;
        if ["project", "contract", "report"].contains(&key.as_str())
            && !Path::new(value).is_absolute()
        {
            return Err(format!("{key} must be absolute"));
        }
        cli.extend([format!("--{key}"), value.to_owned()]);
    }
    archguard::domain_cli::run(&cli).map(|(value, _)| value)
}
fn check(args: &Value) -> Result<Value, String> {
    invoke(args, "check", None)
}
fn explain(args: &Value) -> Result<Value, String> {
    invoke(args, "explain", Some("rule"))
}
fn verify(args: &Value) -> Result<Value, String> {
    invoke(args, "verify", Some("report"))
}

fn model_schema(optional: &[&str], required: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    for key in optional.iter().chain(required) {
        properties.insert((*key).into(), json!({"type":"string"}));
    }
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}
fn model_show_schema() -> Value {
    model_schema(&["entity"], &["project"])
}
fn model_validate_schema() -> Value {
    model_schema(&["model"], &["project"])
}
fn change_impact_schema() -> Value {
    model_schema(&[], &["project", "change"])
}
fn change_check_schema() -> Value {
    model_schema(&["analyzer"], &["project", "change"])
}
fn change_verify_schema() -> Value {
    model_schema(&["analyzer"], &["project", "change", "report"])
}
fn model_invoke(args: &Value, family: &str, action: &str, schema: Value) -> Result<Value, String> {
    let object = args.as_object().ok_or("arguments must be an object")?;
    for required in schema["required"].as_array().unwrap() {
        if !object.contains_key(required.as_str().unwrap()) {
            return Err("required argument missing".into());
        }
    }
    let mut cli = vec![family.to_owned(), action.to_owned()];
    for (key, value) in object {
        if schema["properties"].get(key).is_none() {
            return Err("unknown argument; MCP model operations are read only".into());
        }
        let value = value
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("nonempty string arguments required")?;
        if ["project", "model", "report"].contains(&key.as_str()) && !Path::new(value).is_absolute()
        {
            return Err(format!("{key} must be absolute"));
        }
        cli.extend([format!("--{key}"), value.into()]);
    }
    archguard::domain_cli::run(&cli).map(|(value, _)| value)
}
fn model_show(args: &Value) -> Result<Value, String> {
    model_invoke(args, "model", "show", model_show_schema())
}
fn model_validate(args: &Value) -> Result<Value, String> {
    model_invoke(args, "model", "validate", model_validate_schema())
}
fn change_impact(args: &Value) -> Result<Value, String> {
    model_invoke(args, "change", "impact", change_impact_schema())
}
fn change_check(args: &Value) -> Result<Value, String> {
    model_invoke(args, "change", "check", change_check_schema())
}
fn change_verify(args: &Value) -> Result<Value, String> {
    model_invoke(args, "change", "verify", change_verify_schema())
}
