use archguard::CargoWorkspaceAnalyzer;
use guardengine::{Decision, GuardAnalyzer, evaluate, load_contract_yaml};
use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(decision) => match decision {
            Decision::Allow => ExitCode::SUCCESS,
            Decision::Block => ExitCode::from(2),
            Decision::RequireApproval => ExitCode::from(3),
        },
        Err(error) => {
            eprintln!("archguard: {error}");
            ExitCode::from(4)
        }
    }
}

fn run() -> Result<Decision, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] != "check" {
        return Err("usage: archguard check --project <path> --contract <contract.yaml> [--report <report.json>] [--facts <facts.json>]".into());
    }
    let project = argument(&args, "--project").ok_or("--project required")?;
    let contract = argument(&args, "--contract").ok_or("--contract required")?;
    let parsed = load_contract_yaml(&fs::read(contract)?)?;
    let facts = CargoWorkspaceAnalyzer.analyze(Path::new(project), project)?;
    if let Some(path) = argument(&args, "--facts") {
        fs::write(path, format!("{}\n", serde_json::to_string_pretty(&facts)?))?;
    }
    let report = evaluate(&parsed, &facts)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&report)?);
    if let Some(path) = argument(&args, "--report") {
        fs::write(path, output)?;
    } else {
        print!("{output}");
    }
    Ok(report.decision)
}

fn argument<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].as_str())
}
