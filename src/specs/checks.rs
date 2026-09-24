//! Explicit execution of configured project runners; no shell and no execution
//! while reading specifications. Reports prove observations, not authenticity.
use super::*;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub id: String,
    pub scenario_ids: BTreeSet<String>,
    pub runner: Option<String>,
    pub file: String,
    pub selector: String,
    pub preparation: String,
    pub action: String,
    pub observation: String,
    pub inputs: BTreeSet<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Runner {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub format: String,
    pub timeout_seconds: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub check: String,
    pub started_at: u64,
    pub command: Vec<String>,
    pub cwd: String,
    pub inputs_after: Option<BTreeMap<String, String>>,
    pub check_sha256: String,
    pub outcome: Outcome,
    pub observation: String,
    pub files: BTreeMap<String, String>,
    pub report: String,
}
pub fn validate(s: &Store) -> Result<()> {
    if s.schema_version < 3
        && (!s.runners.is_empty()
            || s.changes
                .values()
                .any(|c| !c.checks.is_empty() || !c.runs.is_empty()))
    {
        return Err("MigrationRequired: structured checks".into());
    }
    for (id, r) in &s.runners {
        check_id(id, "runner")?;
        if r.program.trim().is_empty()
            || !(1..=3600).contains(&r.timeout_seconds)
            || !matches!(r.format.as_str(), "junit" | "cargo-test")
            || !r.args.iter().any(|a| a.contains("{selector}"))
            || (r.format == "junit" && !r.args.iter().any(|a| a.contains("{report}")))
        {
            return Err(
                "InvalidRunner: program, selector/report arguments, format or timeout".into(),
            );
        }
        if r.cwd != "." {
            paths::relative(&r.cwd)?;
        }
        // A project-relative executable is resolved within root. Bare tool names
        // are resolved by PATH; executable configuration is reviewed project code.
        if r.program.contains(['/', '\\', ':']) {
            paths::relative(&r.program)?;
        }
    }
    for c in s.changes.values() {
        let scenarios: BTreeSet<_> = c
            .operations
            .iter()
            .filter_map(Delta::requirement)
            .flat_map(|r| r.scenarios.iter().map(|s| s.id.as_str()))
            .collect();
        let mut seen = BTreeSet::new();
        for b in &c.checks {
            check_id(&b.id, "check")?;
            if !seen.insert(&b.id)
                || b.scenario_ids.is_empty()
                || b.scenario_ids
                    .iter()
                    .any(|id| !scenarios.contains(id.as_str()))
            {
                return Err("InvalidCheckScenarios".into());
            }
            if [&b.preparation, &b.action, &b.observation]
                .iter()
                .any(|s| s.trim().is_empty())
            {
                return Err("CheckProcedureIncomplete".into());
            }
            if let Some(r) = &b.runner {
                if !s.runners.contains_key(r) || b.selector.trim().is_empty() || b.inputs.is_empty()
                {
                    return Err("InvalidCheckRunnerOrInputs".into());
                }
                paths::relative(&b.file)?;
            } else if !b.file.is_empty() {
                paths::relative(&b.file)?;
            }
            for rel in &b.inputs {
                paths::relative(rel)?;
            }
        }
    }
    Ok(())
}
fn revision(s: &Store, b: &Check) -> String {
    digest(&(b, b.runner.as_ref().and_then(|r| s.runners.get(r))))
}
pub fn fresh(root: &Path, s: &Store, c: &Change, b: &Check) -> bool {
    if b.runner.is_none() {
        return true;
    } // manual observation uses spec-evidence
    c.runs
        .iter()
        .rev()
        .find(|r| r.check == b.id)
        .is_some_and(|r| {
            r.outcome == Outcome::Passed
                && r.check_sha256 == revision(s, b)
                && !r.files.is_empty()
                && r.files
                    .iter()
                    .all(|(p, h)| fingerprint(root, p).is_ok_and(|now| now == *h))
        })
}
fn parse(format: &str, text: &str, selector: &str) -> Outcome {
    if format == "cargo-test" {
        let selected = text
            .lines()
            .any(|l| l.trim() == format!("test {selector} ... ok"));
        let summary = text
            .lines()
            .any(|l| l.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored;"));
        return if selected && summary {
            Outcome::Passed
        } else {
            Outcome::Unknown
        };
    }
    let Ok(doc) = roxmltree::Document::parse(text) else {
        return Outcome::Unknown;
    };
    let selected: Vec<_> = doc
        .descendants()
        .filter(|n| {
            n.has_tag_name("testcase")
                && (n.attribute("name") == Some(selector)
                    || format!(
                        "{}::{}",
                        n.attribute("classname").unwrap_or(""),
                        n.attribute("name").unwrap_or("")
                    ) == selector)
        })
        .collect();
    if selected.len() != 1 {
        return Outcome::Unknown;
    }
    let node = selected[0];
    if node
        .children()
        .any(|n| n.has_tag_name("failure") || n.has_tag_name("error"))
    {
        Outcome::Failed
    } else if node.children().any(|n| n.has_tag_name("skipped")) {
        Outcome::Skipped
    } else {
        Outcome::Passed
    }
}
fn inputs(root: &Path, b: &Check, r: &Runner) -> Result<BTreeMap<String, String>> {
    let mut rels = b.inputs.clone();
    rels.insert(b.file.clone());
    if r.program.contains('/') {
        rels.insert(r.program.clone());
    }
    rels.insert(format!("{}/runners.json", storage::directory(root)?));
    rels.into_iter()
        .map(|p| Ok((p.clone(), fingerprint(root, &p)?)))
        .collect()
}
pub fn execute(
    root: &Path,
    s: &mut Store,
    id: &str,
    selected: &str,
    report: &mut Report,
) -> Result<()> {
    validate(s)?;
    let c = s.changes.get(id).ok_or("ChangeMissing")?;
    if c.abandoned_reason.is_some() {
        return Err("AbandonedChange".into());
    }
    let bindings: Vec<_> = c
        .checks
        .iter()
        .filter(|b| selected == "all" || b.id == selected)
        .cloned()
        .collect();
    if bindings.is_empty() {
        return Err("EmptyCheckSelection".into());
    }
    let mut observed = Vec::new();
    for b in bindings {
        let Some(runner_id) = &b.runner else {
            report.finding(
                "unknown",
                "ManualCheckRequired",
                &b.id,
                &format!("{}; {}; {}", b.preparation, b.action, b.observation),
            );
            continue;
        };
        let r = &s.runners[runner_id];
        let result = (|| -> Result<Run> {
            let before = inputs(root, &b, r)?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let rel = format!(
                "{}/changes/{id}/evidence/{}-{stamp}",
                storage::directory(root)?,
                b.id
            );
            let xml = format!("{rel}.xml");
            let log = format!("{rel}.log");
            let xml_path = paths::safe(root, &xml)?;
            let log_path = paths::safe(root, &log)?;
            fs::create_dir_all(log_path.parent().unwrap()).map_err(|e| e.to_string())?;
            let output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&log_path)
                .map_err(|e| e.to_string())?;
            let cwd = if r.cwd == "." {
                root.to_path_buf()
            } else {
                paths::safe(root, &r.cwd)?
            };
            let file = paths::safe(root, &b.file)?;
            let target = Path::new(&b.file)
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("InvalidTestFile")?;
            let args: Vec<_> = r
                .args
                .iter()
                .map(|a| {
                    a.replace("{file}", &file.to_string_lossy())
                        .replace("{target}", target)
                        .replace("{selector}", &b.selector)
                        .replace("{report}", &xml_path.to_string_lossy())
                })
                .collect();
            let program = if r.program.contains('/') {
                paths::safe(root, &r.program)?
                    .to_string_lossy()
                    .into_owned()
            } else {
                r.program.clone()
            };
            let started_at = progress::now()?;
            let started = Instant::now();
            let mut timed_out = false;
            let process = Command::new(&program)
                .args(&args)
                .current_dir(&cwd)
                .stdin(Stdio::null())
                .stdout(output.try_clone().map_err(|e| e.to_string())?)
                .stderr(output)
                .spawn();
            let (successful, note) = match process {
                Err(e) => (false, format!("RunnerStartFailed: {e}")),
                Ok(mut child) => {
                    let status = loop {
                        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                            break Some(status);
                        }
                        if started.elapsed() >= Duration::from_secs(r.timeout_seconds) {
                            timed_out = true;
                            #[cfg(windows)]
                            {
                                let _ = Command::new("taskkill.exe")
                                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .status();
                            }
                            let _ = child.kill();
                            let _ = child.wait();
                            break None;
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    };
                    (
                        status.is_some_and(|s| s.success()),
                        if timed_out {
                            "RunnerTimeout".into()
                        } else {
                            format!("Process status: {status:?}")
                        },
                    )
                }
            };
            let source = if r.format == "junit" { &xml } else { &log };
            let mut outcome = if successful {
                paths::read_limited(&paths::safe(root, source)?, 64 * 1024 * 1024)
                    .ok()
                    .and_then(|b| String::from_utf8(b).ok())
                    .map(|text| parse(&r.format, &text, &b.selector))
                    .unwrap_or(Outcome::Unknown)
            } else {
                Outcome::Failed
            };
            let after = inputs(root, &b, r);
            let mut note = note;
            if !after.as_ref().is_ok_and(|a| *a == before) {
                outcome = Outcome::Unknown;
                note.push_str("; InputsChangedDuringRun");
            }
            let mut files = before;
            files.insert(log.clone(), fingerprint(root, &log)?);
            if xml_path.exists() {
                files.insert(xml.clone(), fingerprint(root, &xml)?);
            }
            let report_path = if xml_path.exists() { xml } else { log };
            Ok(Run {
                check: b.id.clone(),
                started_at,
                command: std::iter::once(program.clone()).chain(args).collect(),
                cwd: cwd.to_string_lossy().into_owned(),
                inputs_after: after.ok(),
                check_sha256: revision(s, &b),
                outcome,
                observation: note,
                files,
                report: report_path,
            })
        })();
        let run=result.unwrap_or_else(|e| Run {check:b.id.clone(),started_at:progress::now().unwrap_or(0),check_sha256:revision(s,&b),command:std::iter::once(r.program.clone()).chain(r.args.clone()).collect(),cwd:r.cwd.clone(),inputs_after:None,outcome:Outcome::Unknown,observation:format!("CheckNotCompleted: {e}; command contains configured placeholders if execution did not start"),files:BTreeMap::new(),report:String::new()});
        observed.push((b.clone(), run));
    }
    // Retain every result, including failures. All bindings for a scenario must
    // be fresh and passing; a later successful sibling cannot hide a failure.
    let c = result_mut(s, id)?;
    for (_, run) in &observed {
        c.runs.push(run.clone());
    }
    let scenarios: BTreeSet<_> = observed
        .iter()
        .flat_map(|(b, _)| b.scenario_ids.iter().cloned())
        .collect();
    for scenario in scenarios {
        let c = &s.changes[id];
        let bindings: Vec<_> = c
            .checks
            .iter()
            .filter(|b| b.scenario_ids.contains(&scenario))
            .collect();
        let passed = bindings
            .iter()
            .all(|b| b.runner.is_some() && fresh(root, s, c, b));
        let mut files = BTreeMap::new();
        for b in &bindings {
            if let Some(run) = c.runs.iter().rev().find(|r| r.check == b.id) {
                files.extend(run.files.clone());
            }
        }
        let r = c
            .operations
            .iter()
            .filter_map(Delta::requirement)
            .find(|r| r.scenarios.iter().any(|sc| sc.id == scenario))
            .unwrap();
        let sc = r.scenarios.iter().find(|sc| sc.id == scenario).unwrap();
        let newest = &observed
            .iter()
            .rev()
            .find(|(b, _)| b.scenario_ids.contains(&scenario))
            .unwrap()
            .1;
        let e=Evidence{command:format!("spec-run --id {id} --check {selected}"),captured_at:newest.started_at.to_string(),method:"native_report".into(),scenario:scenario.clone(),outcome:if passed {Outcome::Passed} else {Outcome::Unknown},observation:"Результаты всех привязанных проверок сохранены в runs; подготовка и полнота входов требуют ревью".into(),scenario_sha256:scenario_revision(r,sc),files,report:newest.report.clone()};
        s.changes.get_mut(id).unwrap().evidence.insert(scenario, e);
    }
    for (b, run) in observed {
        if run.outcome != Outcome::Passed {
            report.finding("failed", "CheckNotPassed", &b.id, &run.observation);
        }
        report.measurements.push(json!({"check":b.id,"run":run}));
    }
    Ok(())
}
