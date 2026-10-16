mod agent_integration;
use guardengine::Decision;
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Some(code) = guard_install::dispatch(&agent_integration::tool(), &args) {
        return code;
    }
    if args
        .first()
        .is_some_and(|a| matches!(a.as_str(), "--help" | "-h" | "help"))
    {
        println!(
            "Project model: model show [--entity ID] | model validate [--model FILE]\nRequirement: change impact|check|verify --change ID [--analyzer auto|maven|cargo]; check exports --report NEW_FILE, verify reads --report FILE.\nBaseline: baseline init --model FILE | baseline update --change ID --report FILE; preview by default, --apply explicitly writes. All model commands require --project ABSOLUTE_PATH. Files must belong to this project."
        );
        println!(
            "archguard check|explain|verify --project PATH [--contract FILE] [--analyzer auto|maven|cargo]\ncheck: [--report NEW_FILE] [--facts NEW_FILE]; explain: --rule ID; verify: --report EXISTING_FILE\narchguard contract validate [--project PATH] [--contract FILE]\nDefault contract: .archguard/contract.yaml. Exit: ALLOW=0 BLOCK=2 REQUIRE_APPROVAL=3 error=4.\nAgent integration: install, uninstall, context, instructions render|sync, doctor --integration, serve --mcp. Run install --help."
        );
        return ExitCode::SUCCESS;
    }
    if args.first().is_some_and(|a| a == "--version") {
        println!("archguard {}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    match archguard::domain_cli::run(&args) {
        Ok((value, decision)) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
            ExitCode::from(match decision {
                Decision::Allow => 0,
                Decision::Block => 2,
                Decision::RequireApproval => 3,
            })
        }
        Err(error) => {
            println!("{}", serde_json::json!({"status":"error","message":error}));
            ExitCode::from(4)
        }
    }
}
