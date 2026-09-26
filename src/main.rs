use highgrade::{Report, Result};
use serde_json::json;
use std::{collections::BTreeMap, path::Path};

fn allowed_options(op: &str) -> Result<Vec<&'static str>> {
    Ok(match op {
        "issue-list" => vec!["--root", "--query", "--tag", "--status"],
        "issue-read" => vec!["--root", "--id", "--view"],
        "issue-validate" => vec!["--root", "--id"],
        "issue-schema" => vec!["--root"],
        "issue-new" => vec!["--root", "--expected", "--input"],
        "issue-edit" | "issue-record" => vec!["--root", "--id", "--expected", "--input"],
        "issue-recover" => vec!["--root", "--expected"],
        "doctor" => vec!["--root", "--action", "--id", "--check"],
        "inspect" => vec!["--root", "--registry", "--scope", "--bootstrap"],
        "global-install" => vec!["--profile", "--source", "--candidate-exe"],
        "global-update" => vec![
            "--profile",
            "--source",
            "--candidate-exe",
            "--apply",
            "--candidate-sha256",
        ],
        "global-status" | "global-recover" => vec!["--profile"],
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
        "spec-schema" | "spec-tags" => vec!["--root"],
        "spec-list" => vec!["--root", "--id", "--tag", "--technical", "--human"],
        "spec-tag-set" => vec!["--root", "--expected", "--id", "--title", "--description"],
        "spec-runner-set" => vec!["--root", "--expected", "--id", "--input"],
        "spec-run" => vec!["--root", "--expected", "--id", "--check"],
        "spec-run-inputs" => vec!["--root", "--id", "--check"],
        "spec-run-import" => vec!["--root", "--expected", "--input"],
        "spec-stats" => vec!["--root", "--id"],
        "spec-tag-remove" => vec!["--root", "--expected", "--id"],
        "spec-tag-merge" => vec!["--root", "--expected", "--from", "--into"],
        "spec-new" => vec!["--root", "--id", "--title", "--expected"],
        "spec-integrate" => vec!["--root", "--id", "--expected", "--brief"],
        "spec-transfer" => vec!["--root", "--id", "--expected"],
        "spec-migrate" => vec!["--root", "--expected", "--to"],
        "spec-init" => vec!["--root", "--directory"],
        "spec-recover" => vec!["--root", "--expected"],
        "spec-decide" => vec!["--root", "--expected", "--input"],
        "spec-read" => vec!["--root", "--id", "--requirement", "--view", "--tag"],
        "spec-edit" => vec![
            "--root",
            "--id",
            "--expected",
            "--expected-local",
            "--input",
            "--validate",
            "--brief",
        ],
        "spec-diff" | "spec-validate" => vec!["--root", "--id"],
        "spec-check" => vec!["--root", "--id", "--brief"],
        "spec-save" | "spec-evidence" => vec!["--root", "--id", "--expected", "--input"],
        "spec-evidence-batch" => vec!["--root", "--expected", "--input"],
        "spec-metadata" => vec!["--root", "--expected", "--input", "--apply"],
        "spec-review" => vec![
            "--root",
            "--id",
            "--expected",
            "--reviewer",
            "--conclusion",
            "--verdict",
        ],
        "spec-abandon" => vec!["--root", "--id", "--expected", "--reason"],
        "spec-import" => vec!["--root", "--expected", "--input", "--source", "--id"],
        _ => {
            return Err(
                "Usage: highgrade doctor|inspect|inventory|trace --root PATH; --version".into(),
            );
        }
    })
}

fn command_help(op: &str) -> Result<Report> {
    let options = allowed_options(op)?;
    let mut required = if op.starts_with("global-") {
        vec!["--profile"]
    } else {
        vec!["--root"]
    };
    if options.contains(&"--expected") && op != "spec-edit" {
        required.push("--expected");
    }
    if options.contains(&"--input") {
        required.push("--input");
    }
    if options.contains(&"--id") && !matches!(op, "spec-new" | "spec-read" | "spec-list" | "doctor")
    {
        required.push("--id");
    }
    if options.contains(&"--check") && op != "doctor" {
        required.push("--check");
    }
    match op {
        "global-install" | "global-update" => required.extend(["--source", "--candidate-exe"]),
        "spec-review" => required.extend(["--reviewer", "--conclusion", "--verdict"]),
        "spec-abandon" => required.push("--reason"),
        "spec-tag-set" => required.extend(["--title", "--description"]),
        "spec-tag-merge" => required.extend(["--from", "--into"]),
        "trace" => required.push("--record"),
        "legacy-install" => required.extend(["--source", "--audit"]),
        "spec-import" => required.push("--source"),
        _ => {}
    }
    let mut example_options = required.clone();
    match op {
        "spec-read" => example_options.push("--id"),
        "spec-new" => example_options.push("--title"),
        "spec-edit" => example_options.push("--expected"),
        _ => {}
    }
    let example = format!(
        "highgrade {op} {}",
        example_options
            .iter()
            .map(|k| format!(
                "{k} {}",
                match *k {
                    "--root" => "PATH",
                    "--profile" => "PROFILE",
                    "--expected" => "HASH",
                    "--id" if op.starts_with("issue-") => "ISS-0001",
                    "--id" => "HG-ID",
                    "--input" => "input.json",
                    "--check" => "all",
                    "--verdict" => "go",
                    _ => "VALUE",
                }
            ))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let mut r = Report::new("help");
    let schemas = if op.starts_with("issue-") {
        highgrade::issues::schemas()
    } else {
        highgrade::specs::input_schemas()
    };
    let key = match op {
        "spec-save" => "change",
        "issue-new" => "content",
        "issue-edit" => "patch",
        "issue-record" => "observation",
        "spec-metadata" => "metadata_batch",
        "spec-evidence" => "evidence_input",
        "spec-evidence-batch" => "evidence_batch",
        "spec-run-import" => "report_import",
        "spec-decide" => "decision_input",
        "spec-runner-set" => "runner",
        _ => "",
    };
    r.measurements.push(json!({"command":op,"options":options,"required":required,"example":example,"conditions":match op {"spec-read"=>"--id or --requirement; --view requirements may use --tag","spec-new"=>"--title required when allocating an ID","spec-edit"=>"Exactly one of --expected or --expected-local from spec-read; first modify/remove baseline capture requires --expected","doctor"=>"Optional --action spec-read|spec-run; spec-run requires --id, optional --check (default all)","spec-metadata"=>"Preview by default; --apply true commits the entire validated batch with --expected CAS", "global-update"=>"--apply true requires --candidate-sha256 from preview",_=>"See parameter values and JSON schemas"},"compatibility":op.starts_with("legacy-"),"input_schema":schemas.get(key),"schemas_command":if op.starts_with("issue-") {"highgrade issue-schema --root PATH"} else {"highgrade spec-schema --root PATH"}}));
    Ok(r)
}

fn run() -> Result<Report> {
    let mut args = std::env::args().skip(1);
    let op = args.next().unwrap_or_default();
    if op == "--help" {
        if args.next().is_some() {
            return Err("Usage: --help takes no options".into());
        }
        let mut r = Report::new("help");
        r.measurements.push(json!({
            "project_commands":"doctor inspect inventory trace spec-list spec-new spec-read spec-edit spec-save spec-diff spec-validate spec-evidence spec-review spec-check spec-integrate spec-import spec-transfer spec-abandon spec-schema spec-migrate spec-decide spec-init spec-recover spec-tags spec-tag-set spec-tag-remove spec-tag-merge spec-runner-set spec-run spec-run-inputs spec-run-import spec-evidence-batch spec-stats spec-metadata",
            "issue_commands":"issue-list issue-read issue-new issue-edit issue-record issue-validate issue-schema issue-recover",
            "installation_commands":"global-install global-update global-status global-recover",
            "compatibility_commands":"legacy-install legacy-update",
            "project_root":"--root PATH",
            "spec_start":"spec-list --root PATH; spec-new --root PATH --title TITLE --expected HASH",
            "spec_formats":"spec-schema --root PATH",
            "reference":"Installed release references/cli.md; --version"
        }));
        return Ok(r);
    }
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
    if input == ["--help"] {
        return command_help(&op);
    }
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
    let allowed = allowed_options(&op)?;
    if options.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err("Usage: неизвестный параметр".into());
    }
    if let Some(value) = options.get("--apply") {
        if value != "true" {
            return Err("Usage: --apply принимает только true".into());
        }
    }
    if let Some(value) = options.get("--brief") {
        if value != "true" {
            return Err("Usage: --brief принимает только true".into());
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
        _ if op.starts_with("issue-") => highgrade::issues::command(root, &op, &options),
        _ if op.starts_with("spec-") => highgrade::specs::command(root, &op, &options),
        "global-status" => highgrade::global::status(Path::new(get("--profile")?)),
        "global-install" => highgrade::global::install(
            Path::new(get("--profile")?),
            Path::new(get("--source")?),
            Path::new(get("--candidate-exe")?),
        ),
        "global-recover" => highgrade::global::recover(Path::new(get("--profile")?)),
        "global-update" => highgrade::global::update(
            Path::new(get("--profile")?),
            options.get("--source").map(Path::new),
            options.get("--candidate-exe").map(Path::new),
            options.get("--apply").is_some_and(|v| v == "true"),
            options.get("--candidate-sha256").map(String::as_str),
        ),
        "doctor" if options.contains_key("--action") => highgrade::specs::diagnose(
            root,
            get("--action")?,
            options.get("--id").map(String::as_str),
            options.get("--check").map(String::as_str),
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
            r.limitations.push("Список файлов и хешей не заменяет смысловой аудит; audit receipt относится только к прежнему legacy-install.".into());
            Ok(r)
        }
        "trace" => highgrade::trace::trace(root, get("--record")?),
        _ => unreachable!(),
    }
}
fn main() {
    match run() {
        Ok(report) => {
            println!(
                "{}",
                highgrade::specs::pretty_value(&serde_json::to_value(&report).unwrap()).unwrap()
            );
            std::process::exit(report.exit_code());
        }
        Err(error) => {
            let mut report = Report::new("error");
            report.finding("failed", "OperationFailed", "arguments-or-input", &error);
            report.measurements.push(json!({"next":"highgrade <command> --help", "input_schema":"highgrade spec-schema --root PATH"}));
            println!(
                "{}",
                highgrade::specs::pretty_value(&serde_json::to_value(&report).unwrap()).unwrap()
            );
            std::process::exit(report.exit_code());
        }
    }
}
