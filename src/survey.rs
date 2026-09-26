//! Project-owned survey, before any specs/registry exist. No recipe execution.
use crate::{Report, Result, discovery, hash, paths};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};
pub const FILE: &str = ".highgrade/project/survey.json";
const LOCK: &str = ".highgrade/project/survey.lock";
const TEMP: &str = ".highgrade/project/survey.next";
const LIMIT: u64 = 8 * 1024 * 1024;
const SECTIONS: [&str; 8] = [
    "components",
    "knowledge",
    "processes",
    "skills",
    "observations",
    "findings",
    "adaptation",
    "routes",
];
const DECISIONS: [&str; 7] = [
    "purpose",
    "scope",
    "preservation",
    "changes",
    "local_rules",
    "environment",
    "mandate",
];
#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum State {
    Confirmed,
    Proposed,
    Unknown,
    NotApplicable,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Survey,
    Planned,
    Adapting,
    Completed,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Item {
    id: String,
    state: State,
    description: String,
    basis: Vec<String>,
    sources: BTreeSet<String>,
    blocks: BTreeSet<String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Content {
    goal: String,
    scope: Vec<String>,
    stage: Stage,
    next: String,
    decisions: Vec<Item>,
    sections: BTreeMap<String, Vec<Item>>,
    source_paths: BTreeSet<String>,
    areas: BTreeSet<String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    files: BTreeMap<String, Option<String>>,
    areas: BTreeMap<String, discovery::Structure>,
    exclusions_sha256: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Review {
    reviewer: String,
    verdict: Verdict,
    conclusion: String,
    content_sha256: String,
    snapshot_sha256: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Go,
    NoGo,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Revision {
    number: u64,
    content: Content,
    snapshot: Option<Snapshot>,
    review: Option<Review>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Document {
    schema_version: u32,
    current: Revision,
    history: Vec<Revision>,
}
fn digest<T: Serialize>(v: &T) -> String {
    hash(&serde_json::to_vec(v).unwrap())
}
fn material(c: &Content) -> String {
    let mut v = serde_json::to_value(c).unwrap();
    v.as_object_mut().unwrap().remove("stage");
    v.as_object_mut().unwrap().remove("next");
    digest(&v)
}
fn nonempty(s: &str) -> Result<()> {
    if s.trim().is_empty() {
        Err("SurveyEmptyText".into())
    } else {
        Ok(())
    }
}
fn validate(c: &Content) -> Result<()> {
    nonempty(&c.goal)?;
    if c.scope.is_empty() || c.scope.len() > 128 || c.source_paths.len() > 256 || c.areas.len() > 32
    {
        return Err("SurveyScopeInvalid".into());
    }
    for p in c.scope.iter().chain(c.areas.iter()) {
        if p != "." {
            paths::relative(p)?;
        }
    }
    for p in &c.source_paths {
        paths::relative(p)?;
    }
    for p in c.source_paths.iter().chain(c.areas.iter()) {
        if !c
            .scope
            .iter()
            .any(|scope| scope == "." || p == scope || p.starts_with(&format!("{scope}/")))
        {
            return Err("SurveyOutsideScope".into());
        }
    }
    if c.stage != Stage::Completed {
        nonempty(&c.next)?;
    } else if !c.next.is_empty() {
        return Err("SurveyCompletedHasNext".into());
    }
    let keys: BTreeSet<_> = c.sections.keys().map(String::as_str).collect();
    if keys != SECTIONS.into_iter().collect() {
        return Err("SurveySectionsInvalid".into());
    }
    let decision_ids: BTreeSet<_> = c.decisions.iter().map(|i| i.id.as_str()).collect();
    if decision_ids != DECISIONS.into_iter().collect() || c.decisions.len() != 7 {
        return Err("SurveyDecisionsInvalid".into());
    }
    let mut ids = BTreeSet::new();
    for i in c.decisions.iter().chain(c.sections.values().flatten()) {
        nonempty(&i.id)?;
        nonempty(&i.description)?;
        if !ids.insert(&i.id) {
            return Err("SurveyDuplicateId".into());
        }
        if matches!(i.state, State::Confirmed | State::NotApplicable) && i.basis.is_empty() {
            return Err("SurveyBasisRequired".into());
        }
        for b in i.basis.iter().chain(i.blocks.iter()) {
            nonempty(b)?;
        }
        if !i.sources.is_subset(&c.source_paths) {
            return Err("SurveyUnknownSource".into());
        }
    }
    Ok(())
}
fn load(root: &Path) -> Result<(Document, String)> {
    let bytes = paths::read_limited(&paths::safe(root, FILE)?, LIMIT)?;
    let d: Document =
        serde_json::from_slice(&bytes).map_err(|e| format!("SurveyFormatInvalid: {e}"))?;
    if d.schema_version != 1 {
        return Err("SurveySchemaUnsupported".into());
    }
    validate(&d.current.content)?;
    for (index, r) in d.history.iter().enumerate() {
        validate(&r.content)?;
        if r.number != index as u64 + 1 || r.content.stage != Stage::Completed {
            return Err("SurveyHistoryInvalid".into());
        }
    }
    if d.current.number != d.history.len() as u64 + 1 {
        return Err("SurveyRevisionInvalid".into());
    }
    Ok((d, hash(&bytes)))
}
fn capture(root: &Path, c: &Content) -> Result<Snapshot> {
    let (exclusions, exclusions_sha256) = crate::install::exclusions(root)?;
    let mut files = BTreeMap::new();
    for p in &c.source_paths {
        if discovery::excluded(p, &exclusions) {
            return Err("SurveySourceExcluded".into());
        }
        // Unsafe paths are rejected, ordinary missing/unreadable files stay unknown.
        paths::safe(root, p)?;
        files.insert(p.clone(), discovery::source_hash(root, p).ok());
    }
    let mut areas = BTreeMap::new();
    for p in &c.areas {
        areas.insert(p.clone(), discovery::structure(root, p)?);
    }
    Ok(Snapshot {
        files,
        areas,
        exclusions_sha256,
    })
}
fn assess(root: &Path, r: &Revision) -> Result<Value> {
    let c = &r.content;
    let mut blockers: Vec<Value> = vec![];
    for d in &c.decisions {
        if !matches!(d.state, State::Confirmed | State::NotApplicable) {
            blockers
                .push(json!({"id":d.id,"reason":"decision_unresolved","actions":["adaptation"]}));
        }
    }
    for (section, items) in &c.sections {
        if items.is_empty() {
            blockers
                .push(json!({"id":section,"reason":"section_unexamined","actions":["adaptation"]}));
        }
        for item in items {
            if !item.blocks.is_empty() && matches!(item.state, State::Unknown | State::Proposed) {
                blockers.push(json!({"id":item.id,"reason":"unresolved","actions":item.blocks}));
            }
        }
    }
    let now = capture(root, c)?;
    let mut changed = Vec::new();
    let mut unknown = Vec::new();
    for (p, h) in &now.files {
        if h.is_none() {
            unknown.push(p.clone());
        }
        if r.snapshot.as_ref().and_then(|s| s.files.get(p)) != Some(h) {
            changed.push(p.clone());
        }
    }
    for (p, s) in &now.areas {
        if !s.complete {
            unknown.push(p.clone());
        }
        if r.snapshot.as_ref().and_then(|s| s.areas.get(p)) != Some(s) {
            changed.push(p.clone());
        }
    }
    let snapshot_current = r.snapshot.as_ref().is_some_and(|s| {
        s.files.keys().eq(now.files.keys())
            && s.areas.keys().eq(now.areas.keys())
            && s.exclusions_sha256 == now.exclusions_sha256
            && changed.is_empty()
            && unknown.is_empty()
    });
    let review_current = r.review.as_ref().is_some_and(|v| {
        v.content_sha256 == material(c) && v.snapshot_sha256 == digest(&r.snapshot)
    });
    let review_go = review_current && r.review.as_ref().is_some_and(|v| v.verdict == Verdict::Go);
    let adaptation_blocked = blockers.iter().any(|b| {
        b["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a == "adaptation")
    });
    Ok(
        json!({"structure":"valid","snapshot_current":snapshot_current,"changed_sources":changed,"unknown_sources":unknown,
        "blockers":blockers,"review_record_current":review_current,"review_go":review_go,"semantic_correctness":"not_certified",
        "adaptation_ready":snapshot_current && review_go && !adaptation_blocked,
        "completion_ready":snapshot_current && review_go && blockers.is_empty(),"historically_completed":c.stage==Stage::Completed,
        "user_acceptance":"not_recorded_by_this_tool"}),
    )
}
struct Guard(std::path::PathBuf);
impl Drop for Guard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn store(root: &Path, expected: Option<&str>, d: &Document) -> Result<()> {
    let dest = paths::safe(root, FILE)?;
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    let lock = paths::safe(root, LOCK)?;
    let _handle = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|_| "SurveyLocked: preserve and inspect existing lock")?;
    drop(_handle);
    let _guard = Guard(lock);
    match expected {
        Some(h) => {
            if hash(&paths::read_limited(&dest, LIMIT)?) != h {
                return Err("SurveyConflict".into());
            }
        }
        None => {
            if dest.exists() {
                return Err("SurveyExists".into());
            }
        }
    }
    let bytes = serde_json::to_vec_pretty(d).unwrap();
    if bytes.len() as u64 > LIMIT {
        return Err("SurveyTooLarge".into());
    }
    let temp = paths::safe(root, TEMP)?;
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|_| "SurveyTemporaryExists: preserve and inspect")?;
    let _temporary = Guard(temp.clone());
    f.write_all(&bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    drop(f);
    paths::rename_atomic(&temp, &dest)?;
    Ok(())
}
pub fn schemas() -> Value {
    let content = serde_json::to_value(schemars::schema_for!(Content)).unwrap();
    let mut patch = content.clone();
    patch.as_object_mut().unwrap().remove("required");
    json!({"document":schemars::schema_for!(Document),"content":content,"patch":patch,"template":draft()})
}
fn draft() -> Content {
    Content {
        goal: "Определить цель подключения".into(),
        scope: vec![".".into()],
        stage: Stage::Survey,
        next: "Уточнить неизвестные ключевые решения".into(),
        decisions: DECISIONS
            .into_iter()
            .map(|id| Item {
                id: id.into(),
                state: State::Unknown,
                description: format!("Не определено: {id}"),
                basis: vec![],
                sources: BTreeSet::new(),
                blocks: ["adaptation".into()].into(),
            })
            .collect(),
        sections: SECTIONS.into_iter().map(|id| (id.into(), vec![])).collect(),
        source_paths: BTreeSet::new(),
        areas: BTreeSet::new(),
    }
}
pub fn command(root: &Path, op: &str, opts: &BTreeMap<String, String>) -> Result<Report> {
    let root = paths::root(root)?;
    let get = |k: &str| {
        opts.get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("Required: {k}"))
    };
    let mut report = Report::new(op);
    report.limitations.push("Recorded statements are data, not proof of semantic correctness, user acceptance or permission. No project commands are executed.".into());
    if op == "survey-schema" {
        report.measurements.push(schemas());
        return Ok(report);
    }
    if op == "survey-new" {
        let d = Document {
            schema_version: 1,
            current: Revision {
                number: 1,
                content: draft(),
                snapshot: None,
                review: None,
            },
            history: vec![],
        };
        store(&root, None, &d)?;
    } else {
        let (mut d, sha) = load(&root)?;
        if op == "survey-read" {
            let view = opts.get("--view").map(String::as_str).unwrap_or("summary");
            let value = match view {
                "full" => serde_json::to_value(&d).unwrap(),
                "editable" => serde_json::to_value(&d.current.content).unwrap(),
                "summary" => {
                    json!({"revision":d.current.number,"goal":d.current.content.goal,"stage":d.current.content.stage,"next":d.current.content.next,"history_count":d.history.len()})
                }
                "markdown" => {
                    let c = &d.current.content;
                    let mut text = format!(
                        "# {}\n\nЭтап: {:?}\n\nСледующее действие: {}\n",
                        c.goal,
                        serde_json::to_value(&c.stage).unwrap(),
                        c.next
                    );
                    for (name, items) in std::iter::once(("decisions", &c.decisions))
                        .chain(c.sections.iter().map(|(k, v)| (k.as_str(), v)))
                    {
                        text.push_str(&format!("\n## {name}\n"));
                        for i in items {
                            text.push_str(&format!("\n- {} [{}]: {}\n  Основание: {}\n  Источники: {}\n  Блокирует: {}\n",i.id,serde_json::to_value(&i.state).unwrap(),i.description,i.basis.join("; "),i.sources.iter().cloned().collect::<Vec<_>>().join(", "),i.blocks.iter().cloned().collect::<Vec<_>>().join(", ")));
                        }
                    }
                    json!(text)
                }
                key if SECTIONS.contains(&key) => {
                    serde_json::to_value(&d.current.content.sections[key]).unwrap()
                }
                _ => return Err("SurveyViewInvalid".into()),
            };
            report
                .measurements
                .push(json!({"value":value,"survey_sha256":sha}));
            return Ok(report);
        }
        if op == "survey-check" {
            let status = assess(&root, &d.current)?;
            if status["completion_ready"] != true {
                report.finding(
                    "unknown",
                    "SurveyIncomplete",
                    FILE,
                    "See separate decisions, source freshness and review record.",
                );
            }
            report.measurements.push(status);
            report.measurements.push(json!({"survey_sha256":sha}));
            return Ok(report);
        }
        let expected = get("--expected")?;
        if expected != sha {
            return Err("SurveyConflict".into());
        }
        if op == "survey-reopen" {
            if d.current.content.stage != Stage::Completed {
                return Err("SurveyNotCompleted".into());
            }
            d.history.push(d.current.clone());
            d.current.number += 1;
            d.current.content.stage = Stage::Survey;
            d.current.content.next = "Проверить изменения источников и прежние решения".into();
            d.current.review = None;
            d.current.snapshot = None;
        } else {
            if d.current.content.stage == Stage::Completed {
                return Err("SurveyCompleted: reopen first".into());
            }
            match op {
                "survey-edit" => {
                    let bytes = paths::read_limited(&paths::safe(&root, get("--input")?)?, LIMIT)?;
                    let patch: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
                    let patch = patch.as_object().ok_or("SurveyPatchInvalid")?;
                    let mut content = serde_json::to_value(&d.current.content).unwrap();
                    for (k, v) in patch {
                        if !content.as_object().unwrap().contains_key(k) {
                            return Err(format!("SurveyUnknownField: {k}"));
                        }
                        content[k] = v.clone();
                    }
                    d.current.content = serde_json::from_value(content)
                        .map_err(|e| format!("SurveyFormatInvalid: {e}"))?;
                    validate(&d.current.content)?;
                    if d.current.content.stage != Stage::Survey {
                        let a = assess(&root, &d.current)?;
                        let key = if d.current.content.stage == Stage::Completed {
                            "completion_ready"
                        } else {
                            "adaptation_ready"
                        };
                        if a[key] != true {
                            return Err("SurveyTransitionBlocked".into());
                        }
                    }
                }
                "survey-snapshot" => d.current.snapshot = Some(capture(&root, &d.current.content)?),
                "survey-review" => {
                    let verdict = match get("--verdict")? {
                        "go" => Verdict::Go,
                        "no_go" => Verdict::NoGo,
                        _ => return Err("SurveyVerdictInvalid".into()),
                    };
                    let reviewer = get("--reviewer")?;
                    let conclusion = get("--conclusion")?;
                    nonempty(reviewer)?;
                    nonempty(conclusion)?;
                    d.current.review = Some(Review {
                        verdict,
                        reviewer: reviewer.into(),
                        conclusion: conclusion.into(),
                        content_sha256: material(&d.current.content),
                        snapshot_sha256: digest(&d.current.snapshot),
                    });
                }
                _ => return Err("SurveyOperationInvalid".into()),
            }
        }
        store(&root, Some(expected), &d)?;
    }
    let (d, sha) = load(&root)?;
    report
        .measurements
        .push(json!({"survey_sha256":sha,"revision":d.current.number,"content":d.current.content}));
    Ok(report)
}
