use guardengine::{
    Decision, GuardReport, evaluate, load_contract_yaml, load_facts_json, verify_report,
};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(decision) => match decision {
            Decision::Allow => ExitCode::SUCCESS,
            Decision::Block => ExitCode::from(2),
            Decision::RequireApproval => ExitCode::from(3),
        },
        Err(error) => {
            eprintln!("guardengine: {error}");
            ExitCode::from(4)
        }
    }
}

fn run() -> Result<Decision, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 6 {
        return Err("usage: guardengine <evaluate|verify> --contract <contract.yaml> --facts <facts.json> [--report <report.json>]".into());
    }
    let contract_path = argument(&args, "--contract").ok_or("--contract is required")?;
    let facts_path = argument(&args, "--facts").ok_or("--facts is required")?;
    let contract = load_contract_yaml(&fs::read(contract_path)?)?;
    let facts = load_facts_json(&fs::read(facts_path)?)?;
    match args[1].as_str() {
        "evaluate" => {
            let report = evaluate(&contract, &facts)?;
            let json = serde_json::to_string_pretty(&report)?;
            if let Some(path) = argument(&args, "--report") {
                fs::write(path, format!("{json}\n"))?;
            } else {
                println!("{json}");
            }
            Ok(report.decision)
        }
        "verify" => {
            let path =
                argument(&args, "--report").ok_or("--report is required for verification")?;
            let report: GuardReport = serde_json::from_slice(&fs::read(path)?)?;
            if !verify_report(&report, &contract, &facts)? {
                return Err(
                    "report does not match recomputed evidence; verification FAILED".into(),
                );
            }
            eprintln!(
                "report matches contract and facts; decision={:?}; NOT a signed attestation",
                report.decision
            );
            Ok(report.decision)
        }
        _ => Err("expected evaluate or verify".into()),
    }
}

fn argument<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}
