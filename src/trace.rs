use crate::{Report, Result, hash, paths, read_json};
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

fn relocated_root(native: &str, capture: &Path, current: &Path) -> Result<PathBuf> {
    let native = Path::new(native);
    if !capture.is_absolute() || !native.is_absolute() {
        return Err("NativeRootInvalid".into());
    }
    let relative = native
        .strip_prefix(capture)
        .map_err(|_| "NativeRootOutsideCapture")?;
    if !relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("NativeRootInvalid".into());
    }
    let relocated = current.join(relative);
    if !relocated.is_dir() {
        return Err("NativeRootMissing".into());
    }
    Ok(relocated)
}

fn ids(value: &Value, key: &str) -> Result<Vec<String>> {
    let rows = value[key]
        .as_array()
        .ok_or_else(|| format!("RunInvalid: {key}"))?;
    rows.iter()
        .map(|v| {
            let s = v
                .as_str()
                .ok_or_else(|| format!("RunInvalid: {key} item"))?;
            paths::relative(s)?;
            Ok(s.into())
        })
        .collect()
}
pub(crate) fn valid_id(s: &str) -> bool {
    if !(6..=80).contains(&s.len()) {
        return false;
    }
    let mut parts = s.split('-');
    let Some(prefix) = parts.next() else {
        return false;
    };
    (2..=32).contains(&prefix.len())
        && prefix.bytes().all(|b| b.is_ascii_uppercase())
        && parts.clone().next().is_some()
        && parts.all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        })
}

#[cfg(test)]
mod id_tests {
    use super::valid_id;

    #[test]
    fn accepts_project_ids_and_rejects_malformed_ids() {
        for id in ["HG-MATH-001", "SNAF-011-R1", "SNAF-011-S1"] {
            assert!(valid_id(id), "{id}");
        }
        for id in ["HG-", "SNAF--S1", "SNAF-s1", "1SNAF-011-S1", "SNAF_011-S1"] {
            assert!(!valid_id(id), "{id}");
        }
    }
}
fn spec_ids(
    text: &str,
    source: &str,
    r: &mut Report,
    requirements: &mut BTreeSet<String>,
    scenarios: &mut BTreeMap<String, String>,
    replaced: Option<&BTreeSet<String>>,
) {
    let mut heading: Option<(HeadingLevel, String)> = None;
    let mut skip_requirement = false;
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => heading = Some((level, String::new())),
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, value)) = heading.as_mut() {
                    value.push_str(&t)
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, title)) = heading.take() {
                    let prefix = match level {
                        HeadingLevel::H3 => "Requirement: [",
                        HeadingLevel::H4 => "Scenario: [",
                        _ => continue,
                    };
                    if !title.starts_with(prefix) {
                        if (level == HeadingLevel::H3 && title.starts_with("Requirement:"))
                            || (level == HeadingLevel::H4 && title.starts_with("Scenario:"))
                        {
                            r.finding("failed", "SpecIdMissing", source, &title);
                        }
                        continue;
                    }
                    let Some((id, rest)) = title[prefix.len()..].split_once(']') else {
                        r.finding("failed", "SpecIdInvalid", source, &title);
                        continue;
                    };
                    if !valid_id(id) || rest.trim().is_empty() {
                        r.finding("failed", "SpecIdInvalid", source, &title);
                        continue;
                    }
                    if level == HeadingLevel::H3 {
                        skip_requirement = replaced.is_some_and(|ids| ids.contains(id));
                        if skip_requirement {
                            continue;
                        }
                        if !requirements.insert(id.into()) {
                            r.finding("failed", "DuplicateRequirementId", source, id);
                        }
                    } else if !skip_requirement
                        && scenarios.insert(id.into(), source.into()).is_some()
                    {
                        r.finding("failed", "DuplicateScenarioId", source, id);
                    }
                }
            }
            _ => {}
        }
    }
}
fn rust_links(
    source: &str,
    text: &str,
    r: &mut Report,
    out: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let lines: Vec<_> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(raw) = line.trim().strip_prefix("// highgrade:") else {
            continue;
        };
        let mut attr = false;
        let mut function = None;
        for next in lines.iter().skip(i + 1).take(6).map(|s| s.trim()) {
            if next == "#[test]" {
                attr = true;
                continue;
            }
            if next.starts_with("#[") {
                continue;
            }
            if attr {
                if let Some(after) = next
                    .strip_prefix("fn ")
                    .or_else(|| next.strip_prefix("pub fn "))
                {
                    let name = after
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect::<String>();
                    if !name.is_empty() {
                        function = Some(name);
                    }
                }
            }
            break;
        }
        let Some(name) = function else {
            r.finding("failed", "RustMarkerUnbound", source, raw.trim());
            continue;
        };
        let entry = out.entry(name).or_default();
        for item in raw.split(',') {
            let id = item.trim();
            if !valid_id(id) {
                r.finding("failed", "MarkerIdInvalid", source, id);
            } else {
                entry.insert(id.into());
            }
        }
    }
}
fn load(root: &Path, rel: &str, max: u64) -> Result<Vec<u8>> {
    paths::read_limited(&paths::safe(root, rel)?, max)
}
fn rust_results(
    root: &Path,
    capture: &Path,
    run: &Value,
    sources: &[String],
    r: &mut Report,
) -> Result<(BTreeMap<String, String>, BTreeMap<String, String>)> {
    let inventory = run["inventory"].as_str().ok_or("RunInvalid: inventory")?;
    let list = read_json(&paths::safe(root, inventory)?)?;
    let suites = list["rust-suites"].as_object().ok_or("RustListInvalid")?;
    let mut known = BTreeSet::new();
    let mut source_bins: BTreeMap<String, String> = BTreeMap::new();
    for (binary, details) in suites {
        if details["kind"] == "test" {
            if let (Some(cwd), Some(name)) =
                (details["cwd"].as_str(), details["binary-name"].as_str())
            {
                if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                {
                    let expected = relocated_root(cwd, capture, root)?
                        .join("tests")
                        .join(format!("{name}.rs"));
                    if let Ok(expected) = expected.canonicalize() {
                        for rel in sources {
                            if paths::safe(root, rel)?.canonicalize().ok().as_ref()
                                == Some(&expected)
                            {
                                if source_bins.insert(rel.clone(), binary.clone()).is_some() {
                                    r.finding("failed", "RustSourceAmbiguous", rel, binary);
                                }
                            }
                        }
                    }
                }
            }
        }
        let tests = details["testcases"]
            .as_object()
            .ok_or("RustListInvalid: testcases")?;
        for (name, item) in tests {
            if item["filter-match"]["status"] == "matches" {
                known.insert(format!("{binary}::{name}"));
            }
        }
    }
    if known.is_empty() {
        r.finding("failed", "EmptySuite", inventory, "Не обнаружено тестов.");
    }
    let path = run["report"].as_str().ok_or("RunInvalid: report")?;
    let data = load(root, path, 32 * 1024 * 1024)?;
    let text = std::str::from_utf8(&data).map_err(|e| e.to_string())?;
    let xml = roxmltree::Document::parse(text).map_err(|e| format!("JUnitInvalid: {e}"))?;
    let mut actual = BTreeMap::new();
    for test in xml.descendants().filter(|n| n.has_tag_name("testcase")) {
        let name = test
            .attribute("name")
            .ok_or("JUnitInvalid: testcase.name")?;
        let class = test
            .attribute("classname")
            .ok_or("JUnitInvalid: testcase.classname")?;
        let key = format!("{class}::{name}");
        let outcome = if test.children().any(|n| {
            [
                "failure",
                "error",
                "rerunFailure",
                "flakyFailure",
                "rerunError",
                "flakyError",
            ]
            .contains(&n.tag_name().name())
        }) {
            "failed"
        } else if test.children().any(|n| n.has_tag_name("skipped")) {
            "skipped"
        } else {
            "passed"
        };
        if actual.insert(key.clone(), outcome.into()).is_some() {
            r.finding("failed", "DuplicateTestResult", path, &key);
        }
        if !known.contains(&key) {
            r.finding("failed", "TestNotInInventory", path, &key);
        }
    }
    for key in known {
        if !actual.contains_key(&key) {
            r.finding("unknown", "TestNotExecuted", path, &key);
        }
    }
    Ok((actual, source_bins))
}
fn pw_walk(
    node: &Value,
    results: &mut BTreeMap<String, (String, BTreeSet<String>)>,
    r: &mut Report,
    where_: &str,
    root: &Path,
    report_root: &Path,
    sources: &BTreeSet<String>,
) {
    if let Some(specs) = node["specs"].as_array() {
        for spec in specs {
            let Some(spec_id) = spec["id"].as_str() else {
                r.finding("failed", "PlaywrightIdMissing", where_, "spec.id");
                continue;
            };
            let file = spec["file"].as_str().unwrap_or("");
            let normalized = paths::relative(file)
                .ok()
                .and_then(|_| report_root.join(file).canonicalize().ok())
                .and_then(|p| {
                    p.strip_prefix(root)
                        .ok()
                        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
                });
            if normalized
                .as_ref()
                .is_none_or(|path| !sources.contains(path))
            {
                r.finding("failed", "PlaywrightSourceUnpinned", where_, file);
            }
            let Some(tests) = spec["tests"].as_array() else {
                r.finding("failed", "PlaywrightInvalid", where_, spec_id);
                continue;
            };
            for (index, test) in tests.iter().enumerate() {
                let project = test["projectName"].as_str().unwrap_or("default");
                let key = format!("{spec_id}::{project}::{index}");
                let mut linked = BTreeSet::new();
                if let Some(annotations) = test["annotations"].as_array() {
                    for annotation in annotations {
                        if annotation["type"] == "highgrade" {
                            if let Some(raw) = annotation["description"].as_str() {
                                for id in raw.split(',').map(str::trim) {
                                    if valid_id(id) {
                                        linked.insert(id.into());
                                    } else {
                                        r.finding("failed", "MarkerIdInvalid", where_, id);
                                    }
                                }
                            } else {
                                r.finding("failed", "MarkerIdInvalid", where_, &key);
                            }
                        }
                    }
                }
                let runs = test["results"].as_array();
                let outcome = if runs.is_none_or(|a| a.is_empty()) {
                    "unknown"
                } else if runs.unwrap().iter().any(|v| {
                    v["status"] == "failed"
                        || v["status"] == "timedOut"
                        || v["status"] == "interrupted"
                }) {
                    "failed"
                } else if runs.unwrap().iter().any(|v| v["status"] == "skipped")
                    || test["status"] == "skipped"
                {
                    "skipped"
                } else if runs.unwrap().iter().all(|v| v["status"] == "passed")
                    && test["status"] == "expected"
                    && test["expectedStatus"] == "passed"
                {
                    "passed"
                } else {
                    "unknown"
                };
                if results
                    .insert(key.clone(), (outcome.into(), linked))
                    .is_some()
                {
                    r.finding("failed", "DuplicateTestResult", where_, &key);
                }
            }
        }
    }
    if let Some(children) = node["suites"].as_array() {
        for child in children {
            pw_walk(child, results, r, where_, root, report_root, sources);
        }
    }
}
pub fn trace(root: &Path, record_rel: &str) -> Result<Report> {
    let root = paths::root(root)?;
    let run = read_json(&paths::safe(&root, record_rel)?)?;
    if run["schema_version"] != 1 {
        return Err("RunVersionUnsupported".into());
    }
    let tool = run["tool"].as_str().ok_or("RunInvalid: tool")?;
    if !["rust-nextest", "playwright"].contains(&tool) {
        return Err("RunToolUnsupported".into());
    }
    for key in ["command", "captured_at", "scope"] {
        if run[key].as_str().is_none_or(|s| s.trim().is_empty()) {
            return Err(format!("RunInvalid: {key}"));
        }
    }
    let capture = Path::new(
        run["capture_root"]
            .as_str()
            .ok_or("RunInvalid: capture_root")?,
    );
    if !capture.is_absolute() {
        return Err("RunInvalid: capture_root absolute path required".into());
    }
    let report_path = run["report"].as_str().ok_or("RunInvalid: report")?;
    let report_bytes = load(&root, report_path, 32 * 1024 * 1024)?;
    let mut r = Report::new("trace");
    if run["report_sha256"].as_str() != Some(&hash(&report_bytes)) {
        r.finding(
            "failed",
            "ReportHashMismatch",
            report_path,
            "Исходный отчёт изменён.",
        );
    }
    let source_hashes = run["source_hashes"]
        .as_object()
        .ok_or("RunInvalid: source_hashes")?;
    if source_hashes.is_empty() {
        return Err("RunInvalid: source_hashes empty".into());
    }
    let (native, native_sha) = crate::specs::load(&root)?;
    let mut stale = false;
    for (rel, expected) in source_hashes {
        let current_hash =
            if rel == crate::specs::STORE || rel == &crate::specs::catalog_path(&root)? {
                crate::specs::catalog_hash(&native)?
            } else {
                hash(&load(&root, rel, 32 * 1024 * 1024)?)
            };
        if expected.as_str() != Some(&current_hash) {
            stale = true;
            r.finding(
                "unknown",
                "SourceChanged",
                rel,
                "Отчёт относится к прежнему состоянию.",
            );
        }
    }
    let specs = ids(&run, "spec_files")?;
    let sources = ids(&run, "test_sources")?;
    if specs.is_empty() || sources.is_empty() {
        return Err("RunInvalid: spec_files/test_sources empty".into());
    }
    for rel in specs.iter().chain(&sources) {
        if !source_hashes.contains_key(rel) {
            r.finding(
                "unknown",
                "SourceUnpinned",
                rel,
                "Состояние источника не зафиксировано.",
            );
        }
    }
    if tool == "rust-nextest" {
        let inventory = run["inventory"].as_str().ok_or("RunInvalid: inventory")?;
        if !source_hashes.contains_key(inventory) {
            r.finding(
                "unknown",
                "SourceUnpinned",
                inventory,
                "Инвентарь тестов не зафиксирован.",
            );
        }
    }
    let mut requirements = BTreeSet::new();
    let mut scenarios = BTreeMap::new();
    let mut replaced: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for rel in &specs {
        let parts: Vec<_> = rel.split('/').collect();
        if parts.len() != 6
            || parts[0] != "openspec"
            || parts[1] != "changes"
            || parts[3] != "specs"
            || parts[5] != "spec.md"
        {
            continue;
        }
        let base = format!("openspec/specs/{}/spec.md", parts[4]);
        if !specs.contains(&base) {
            continue;
        }
        let bytes = load(&root, rel, 8 * 1024 * 1024)?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let mut modified = false;
        for line in text.lines() {
            if line.starts_with("## ") {
                modified = line == "## MODIFIED Requirements";
            } else if modified {
                if let Some(id) = line
                    .strip_prefix("### Requirement: [")
                    .and_then(|s| s.split_once(']').map(|v| v.0))
                {
                    replaced.entry(base.clone()).or_default().insert(id.into());
                }
            }
        }
    }
    if native_sha != "absent" {
        for (id, origin) in native.origins.iter().chain(
            native
                .changes
                .values()
                .filter(|c| !c.archived)
                .flat_map(|c| c.imports.iter()),
        ) {
            replaced
                .entry(origin.path.clone())
                .or_default()
                .insert(id.clone());
        }
        if (!native.origins.is_empty()
            || native
                .changes
                .values()
                .any(|c| !c.archived && !c.imports.is_empty()))
            && !specs.iter().any(|p| {
                p == crate::specs::STORE
                    || p == &crate::specs::catalog_path(&root).unwrap_or_default()
            })
        {
            r.finding(
                "unknown",
                "NativeSpecsUnpinned",
                crate::specs::STORE,
                "Include the current owner in spec_files and source_hashes.",
            );
        }
    }
    for rel in &specs {
        if rel == crate::specs::STORE || rel == &crate::specs::catalog_path(&root)? {
            for req in crate::specs::catalog(&native)?.values() {
                if !requirements.insert(req.id.clone()) {
                    r.finding("failed", "DuplicateId", rel, &req.id);
                }
                for sc in &req.scenarios {
                    if scenarios.insert(sc.id.clone(), rel.clone()).is_some() {
                        r.finding("failed", "DuplicateId", rel, &sc.id);
                    }
                }
            }
            continue;
        }
        let bytes = load(&root, rel, 8 * 1024 * 1024)?;
        let text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        spec_ids(
            &text,
            rel,
            &mut r,
            &mut requirements,
            &mut scenarios,
            replaced.get(rel),
        );
    }
    if scenarios.is_empty() {
        r.finding(
            "failed",
            "EmptySpec",
            record_rel,
            "Нет сценариев с устойчивыми ID.",
        );
    }
    let mut links: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    if tool == "rust-nextest" {
        let (actual, source_bins) = rust_results(&root, capture, &run, &sources, &mut r)?;
        let mut named = BTreeMap::new();
        for rel in &sources {
            let Some(binary) = source_bins.get(rel) else {
                r.finding("unknown", "RustSourceUnverified", rel, "Поддерживаются обычные integration tests tests/<binary-name>.rs из nextest list.");
                continue;
            };
            let data = load(&root, rel, 8 * 1024 * 1024)?;
            let mut local = BTreeMap::new();
            rust_links(
                rel,
                &String::from_utf8(data).map_err(|e| e.to_string())?,
                &mut r,
                &mut local,
            );
            for (name, ids) in local {
                named
                    .entry(format!("{binary}::{name}"))
                    .or_insert_with(BTreeSet::new)
                    .extend(ids);
            }
        }
        for (name, ids) in named {
            let Some(status) = actual.get(&name) else {
                r.finding(
                    "unknown",
                    "RustTestMissing",
                    &name,
                    "Нет результата для теста из указанного binary.",
                );
                continue;
            };
            for id in ids {
                links
                    .entry(id)
                    .or_default()
                    .push((name.clone(), status.clone()));
            }
        }
    } else {
        let report: Value = serde_json::from_slice(&report_bytes)
            .map_err(|e| format!("PlaywrightReportInvalid: {e}"))?;
        let report_root = report["config"]["rootDir"]
            .as_str()
            .ok_or("PlaywrightRootMissing")?;
        let report_root = relocated_root(report_root, capture, &root)?;
        let mut actual = BTreeMap::new();
        let source_set: BTreeSet<_> = sources.iter().map(|s| s.replace('\\', "/")).collect();
        pw_walk(
            &report,
            &mut actual,
            &mut r,
            report_path,
            &root,
            &report_root,
            &source_set,
        );
        if report["errors"]
            .as_array()
            .is_some_and(|errors| !errors.is_empty())
        {
            r.finding(
                "failed",
                "PlaywrightRunErrors",
                report_path,
                "Глобальные ошибки запуска.",
            );
        }
        if actual.is_empty() {
            r.finding("failed", "EmptySuite", report_path, "Нет тестов.");
        }
        for (test, (status, ids)) in actual {
            for id in ids {
                links
                    .entry(id)
                    .or_default()
                    .push((test.clone(), status.clone()));
            }
        }
    }
    for id in links.keys() {
        if !scenarios.contains_key(id) {
            r.finding("failed", "UnknownScenarioId", record_rel, id);
        }
    }
    for (id, path) in &scenarios {
        let linked = links.get(id).cloned().unwrap_or_default();
        let status = if linked.is_empty() {
            "unlinked"
        } else if linked.iter().any(|(_, s)| s == "failed") {
            "failed"
        } else if linked.iter().any(|(_, s)| s == "unknown") {
            "unknown"
        } else if linked.iter().any(|(_, s)| s == "skipped") {
            "skipped"
        } else if stale {
            "stale"
        } else {
            "passed"
        };
        if status != "passed" {
            r.finding(
                if status == "failed" {
                    "failed"
                } else {
                    "unknown"
                },
                "ScenarioUnconfirmed",
                path,
                &format!("{id}: {status}"),
            );
        }
        r.measurements
            .push(json!({"scenario_id":id,"source":path,"status":status,"tests":linked}));
    }
    if run["exit_code"].as_i64() != Some(0) {
        r.finding(
            "failed",
            "NativeRunFailed",
            record_rel,
            "Штатная команда завершилась с ошибкой или код неизвестен.",
        );
    }
    r.measurements.push(json!({"tool":tool,"report":report_path,"report_sha256":hash(&report_bytes),"command":run["command"],"captured_at":run["captured_at"],"scope":run["scope"]}));
    r.limitations.push("Сопоставление структурное; качество теста и полноту выбора существенных зависимостей проверяет агент. CLI не запускает тесты и не создаёт доказательства.".into());
    Ok(r)
}
