use highgrade::{Report, Result};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

fn run() -> Result<Report> {
    let mut args = std::env::args().skip(1);
    let op = args.next().unwrap_or_default();
    if op == "--version" {
        if args.next().is_some() {
            return Err("Usage: --version не принимает параметры".into());
        }
        let mut r = Report::new("version");
        r.measurements
            .push(json!({"version":env!("CARGO_PKG_VERSION")}));
        return Ok(r);
    }
    let mut options = BTreeMap::new();
    let input: Vec<String> = args.collect();
    let mut index = 0;
    while index < input.len() {
        let key = &input[index];
        if !key.starts_with("--") {
            return Err("Usage: ожидается параметр --name".into());
        }
        if key == "--bootstrap" {
            if options.insert(key.clone(), "true".into()).is_some() {
                return Err("Usage: повторный параметр".into());
            }
            index += 1;
        } else {
            let Some(value) = input.get(index + 1) else {
                return Err("Usage: отсутствует значение параметра".into());
            };
            if value.starts_with("--") || options.insert(key.clone(), value.clone()).is_some() {
                return Err("Usage: неверная пара или повторный параметр".into());
            }
            index += 2;
        }
    }
    let allowed = match op.as_str() {
        "doctor" => vec!["--root"],
        "inspect" => vec!["--root", "--registry", "--scope", "--bootstrap"],
        "global-install" => vec!["--profile", "--source", "--candidate-exe"],
        "global-update" => vec![
            "--profile",
            "--source",
            "--candidate-exe",
            "--apply",
            "--candidate-sha256",
            "--rollback",
        ],
        "global-status" => vec!["--profile"],
        "legacy-install" => vec!["--root", "--source", "--audit"],
        "legacy-update" => vec![
            "--root",
            "--source",
            "--candidate-exe",
            "--decision",
            "--apply",
            "--rollback",
        ],
        "inventory" => vec!["--root"],
        "trace" => vec!["--root", "--record"],
        _ => {
            return Err(
                "Usage: highgrade doctor|inspect|inventory|trace --root PATH; --version".into(),
            );
        }
    };
    if options.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("Usage: неизвестный параметр".into());
    }
    if let Some(value) = options.get("--apply") {
        if value != "true" {
            return Err("Usage: --apply принимает только true".into());
        }
    }
    if options.contains_key("--bootstrap")
        && (options.contains_key("--registry") || options.contains_key("--scope"))
    {
        return Err("Usage: --bootstrap нельзя сочетать с --registry или --scope".into());
    }
    let get = |key: &str| {
        options
            .get(key)
            .map(String::as_str)
            .ok_or_else(|| format!("Usage: required {key}"))
    };
    let root = if op.starts_with("global-") {
        Path::new(".")
    } else {
        Path::new(get("--root")?)
    };
    match op.as_str() {
        "global-status" => highgrade::global::status(Path::new(get("--profile")?)),
        "global-install" => highgrade::global::install(
            Path::new(get("--profile")?),
            Path::new(get("--source")?),
            Path::new(get("--candidate-exe")?),
        ),
        "global-update" => highgrade::global::update(
            Path::new(get("--profile")?),
            options.get("--source").map(Path::new),
            options.get("--candidate-exe").map(Path::new),
            options.get("--apply").is_some_and(|v| v == "true"),
            options.get("--candidate-sha256").map(String::as_str),
            options.get("--rollback").map(String::as_str),
        ),
        "doctor" => highgrade::doctor(root),
        "inspect" if options.contains_key("--bootstrap") => highgrade::bootstrap::bootstrap(root),
        "inspect" => highgrade::inspect::inspect(
            root,
            options.get("--registry").map(String::as_str),
            options.get("--scope").map(String::as_str),
        ),
        "legacy-install" => {
            let source = Path::new(get("--source")?);
            let manifest = highgrade::read_json(&source.join("manifest.json"))?;
            if manifest["schema_version"] == 2 {
                highgrade::package::install(root, source, get("--audit")?)
            } else {
                highgrade::install::install(root, source, get("--audit")?, None)
            }
        }
        "legacy-update" => highgrade::package::update(
            root,
            options.get("--source").map(Path::new),
            options.get("--candidate-exe").map(Path::new),
            options.get("--decision").map(String::as_str),
            options.get("--apply").is_some_and(|v| v == "true"),
            options.get("--rollback").map(String::as_str),
        ),
        "inventory" => {
            let root = highgrade::paths::root(root)?;
            let files = highgrade::install::inventory(&root, ".highgrade/audit-receipt.json")?;
            let (exclusions, exclusions_sha256) = highgrade::install::exclusions(&root)?;
            let mut r = Report::new("inventory");
            r.measurements
                .push(json!({"files":files,"count":files.len(),"exclusions":exclusions,"exclusions_sha256":exclusions_sha256}));
            r.limitations.push("Список файлов и хешей не заменяет смысловой аудит; receipt создаётся после завершённого аудита и решения автора.".into());
            Ok(r)
        }
        "trace" => highgrade::trace::trace(root, get("--record")?),
        _ => unreachable!(),
    }
}
fn main() {
    match run() {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            std::process::exit(report.exit_code());
        }
        Err(error) => {
            let mut report = Report::new("error");
            report.finding("failed", "OperationFailed", "arguments-or-input", &error);
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
            std::process::exit(report.exit_code());
        }
    }
}
