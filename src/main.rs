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
    let mut seen = std::collections::HashSet::new();
    for pair in args[2..].chunks(2) {
        if pair.len() != 2
            || !matches!(
                pair[0].as_str(),
                "--project" | "--contract" | "--facts" | "--report" | "--profile"
            )
            || pair[1].starts_with("--")
            || !seen.insert(&pair[0])
        {
            return Err("unknown, duplicate or missing option value".into());
        }
    }
    let project = argument(&args, "--project").ok_or("--project required")?;
    let contract = argument(&args, "--contract").ok_or("--contract required")?;
    let destinations: Vec<_> = [argument(&args, "--facts"), argument(&args, "--report")]
        .into_iter()
        .flatten()
        .map(Path::new)
        .collect();
    // Validate all destinations before invalidation, including input aliases.
    let resolved: Vec<_> = destinations
        .iter()
        .map(|p| archguard::integration::cli::resolved_destination(p))
        .collect::<Result<_, _>>()?;
    let protected: Vec<_> = [
        Path::new(contract).to_path_buf(),
        Path::new(project).join("Cargo.toml"),
    ]
    .into_iter()
    .filter_map(|p| p.canonicalize().ok())
    .collect();
    if resolved.iter().any(|p| protected.contains(p))
        || (resolved.len() == 2 && resolved[0] == resolved[1])
    {
        return Err("output aliases an input or another output".into());
    }
    for path in &destinations {
        archguard::integration::cli::invalidate_output(path)?;
    }
    let parsed = load_contract_yaml(&fs::read(contract)?)?;
    let facts = match argument(&args, "--profile").unwrap_or("legacy") {
        "legacy" => CargoWorkspaceAnalyzer.analyze(Path::new(project), project)?,
        "cargo-declarations-v1" => {
            let profile = archguard::analysis::profile::FrozenAnalysisProfile::freeze(&parsed, []);
            archguard::analysis::analyze(Path::new(project), project, &profile)?
        }
        _ => return Err("unsupported analysis profile".into()),
    };
    if let Some(path) = argument(&args, "--facts") {
        archguard::integration::cli::publish_output(
            Path::new(path),
            format!("{}\n", serde_json::to_string_pretty(&facts)?).as_bytes(),
        )?;
    }
    let report = evaluate(&parsed, &facts)?;
    let output = format!("{}\n", serde_json::to_string_pretty(&report)?);
    if let Some(path) = argument(&args, "--report") {
        archguard::integration::cli::publish_output(Path::new(path), output.as_bytes())?;
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
