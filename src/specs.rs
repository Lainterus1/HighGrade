//! Native specifications. Project-owned catalog with legacy store compatibility; reports remain external.
use crate::{Report, Result, hash, paths};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
mod checks;
mod delivery;
pub use delivery::diagnose as diagnose_delivery;
mod presentation;
mod progress;
mod relations;
mod storage;
pub use presentation::pretty_value;
pub use storage::{CATALOG, catalog_path};

pub const STORE: &str = ".highgrade/specs/store.json";
pub fn diagnose(
    root: &Path,
    action: &str,
    id: Option<&str>,
    check: Option<&str>,
) -> Result<Report> {
    let root = paths::root(root)?;
    let mut report = Report::new("doctor");
    match action {
        "spec-read" => {
            let (_, sha) = load(&root)?;
            report
                .measurements
                .push(json!({"action":action,"store_sha256":sha,"executed":false}));
        }
        "spec-run" => {
            let (store, _) = load(&root)?;
            let id = id.ok_or("Usage: doctor --action spec-run requires --id")?;
            let c = store.changes.get(id).ok_or("ChangeMissing")?;
            let selected = check.unwrap_or("all");
            let snapshot = checks::capture_inputs(&root, &store, id, selected)?;
            report
                .measurements
                .push(json!({"action":action,"snapshot":snapshot,"executed":false}));
            for b in c
                .checks
                .iter()
                .filter(|b| selected == "all" || b.id == selected)
            {
                let r = &store.runners[b.runner.as_ref().unwrap()];
                let found = if r.program.contains('/') {
                    paths::safe(&root, &r.program)?.is_file()
                } else {
                    std::env::var_os("PATH")
                        .into_iter()
                        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
                        .filter(|p| p.is_absolute())
                        .any(|p| {
                            p.join(&r.program).is_file()
                                || (cfg!(windows) && p.join(format!("{}.exe", r.program)).is_file())
                        })
                };
                report.measurements.push(
                    json!({"check":b.id,"program":r.program,"available":found,"executed":false}),
                );
                if !found {
                    report.finding(
                        "failed",
                        "RunnerUnavailable",
                        &b.id,
                        &format!("Install or configure runner executable {}", r.program),
                    );
                }
            }
        }
        _ => return Err("Usage: doctor --action spec-read|spec-run".into()),
    }
    Ok(report)
}
pub fn input_schemas() -> Value {
    json!({"change":schemars::schema_for!(Change),"metadata_batch":schemars::schema_for!(Vec<MetadataInput>),"evidence_input":schemars::schema_for!(EvidenceInput),"evidence_batch":schemars::schema_for!(Vec<BatchEvidence>),"report_import":schemars::schema_for!(checks::ImportReport),"decision_input":schemars::schema_for!(progress::DecisionInput),"runner":schemars::schema_for!(checks::Runner)})
}
const LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub given: String,
    pub when: String,
    pub then: String,
    pub verification: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    pub title: String,
    pub statement: String,
    pub scenarios: Vec<Scenario>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Delta {
    Add { requirement: Requirement },
    Modify { requirement: Requirement },
    Remove { id: String, reason: String },
}
impl Delta {
    fn id(&self) -> &str {
        match self {
            Self::Add { requirement } | Self::Modify { requirement } => &requirement.id,
            Self::Remove { id, .. } => id,
        }
    }
    fn requirement(&self) -> Option<&Requirement> {
        match self {
            Self::Add { requirement } | Self::Modify { requirement } => Some(requirement),
            _ => None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub done: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    Skipped,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub command: String,
    pub captured_at: String,
    pub method: String,
    pub scenario: String,
    pub outcome: Outcome,
    pub observation: String,
    pub scenario_sha256: String,
    pub files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_paths: Option<BTreeSet<String>>,
    pub report: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Go,
    NoGo,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Review {
    pub verdict: Verdict,
    pub reviewer: String,
    pub conclusion: String,
    pub change_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub id: String,
    pub goal: String,
    pub rationale: String,
    pub scope: String,
    pub questions: Vec<String>,
    pub tasks: Vec<Task>,
    pub operations: Vec<Delta>,
    pub baseline: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scoped_baseline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_tags: Option<BTreeSet<String>>,
    pub imports: BTreeMap<String, Origin>,
    pub evidence: BTreeMap<String, Evidence>,
    pub review: Option<Review>,
    pub archived: bool,
    pub abandoned_reason: Option<String>,
    pub title: String,
    pub created_at: Option<u64>,
    pub acceptance: Vec<progress::Decision>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<ResultSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<relations::Link>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_to: Vec<relations::Link>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub tags: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<checks::Check>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub shared_inputs: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<checks::Run>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResultSnapshot {
    pub recorded_at: u64,
    pub evidence: BTreeMap<String, Evidence>,
    pub review: Option<Review>,
}
fn result_mut<'a>(s: &'a mut Store, key: &str) -> Result<&'a mut Change> {
    if s.schema_version != 3 {
        return change_mut(s, key);
    }
    let c = s.changes.get_mut(key).ok_or("ChangeMissing")?;
    if c.abandoned_reason.is_some() {
        return Err("AbandonedChange".into());
    }
    c.history.push(ResultSnapshot {
        recorded_at: progress::now()?,
        evidence: c.evidence.clone(),
        review: c.review.clone(),
    });
    Ok(c)
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Origin {
    pub original_text: String,
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Store {
    pub schema_version: u32,
    pub next_number: u64,
    pub requirements: BTreeMap<String, Requirement>,
    pub changes: BTreeMap<String, Change>,
    pub retired_ids: BTreeSet<String>,
    pub origins: BTreeMap<String, Origin>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tags: BTreeMap<String, relations::Tag>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub runners: BTreeMap<String, checks::Runner>,
}
impl Default for Store {
    fn default() -> Self {
        Self {
            schema_version: 2,
            next_number: 1,
            requirements: BTreeMap::new(),
            changes: BTreeMap::new(),
            retired_ids: BTreeSet::new(),
            origins: BTreeMap::new(),
            tags: BTreeMap::new(),
            runners: BTreeMap::new(),
        }
    }
}
fn digest<T: Serialize>(v: &T) -> String {
    hash(&serde_json::to_vec(v).expect("serializable model"))
}
fn id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
fn check_id(s: &str, field: &str) -> Result<()> {
    if id(s) {
        Ok(())
    } else {
        Err(format!("InvalidId: {field}"))
    }
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8], field: &str) -> Result<T> {
    serde_json::from_slice(bytes).map_err(|e| format!("InvalidFormat: {field}: {e}"))
}
pub fn load(root: &Path) -> Result<(Store, String)> {
    let parent = paths::safe(root, ".highgrade/specs")?;
    if !parent.exists() && !storage::exists(root)? {
        return Ok((
            Store {
                schema_version: 3,
                ..Store::default()
            },
            "absent".into(),
        ));
    }
    let _guard = lock(root)?;
    load_unlocked(root)
}
fn load_unlocked(root: &Path) -> Result<(Store, String)> {
    if storage::exists(root)? {
        return storage::load(root);
    }
    let p = paths::safe(root, STORE)?;
    if !p.exists() {
        return Ok((
            Store {
                schema_version: 3,
                ..Store::default()
            },
            "absent".into(),
        ));
    }
    let bytes = paths::read_limited(&p, LIMIT)?;
    let s = progress::decode_store(&bytes)?;
    integrity(&s)?;
    Ok((s, hash(&bytes)))
}
fn requirement_ids(r: &Requirement, ids: &mut BTreeSet<String>) -> Result<()> {
    if !crate::trace::valid_id(&r.id) {
        return Err("InvalidId: /requirement/id".into());
    }
    check_id(&r.id, "/requirement/id")?;
    if !ids.insert(r.id.clone()) {
        return Err(format!("DuplicateId: {}", r.id));
    }
    for sc in &r.scenarios {
        if !crate::trace::valid_id(&sc.id) {
            return Err("InvalidId: /scenarios/id".into());
        }
        check_id(&sc.id, "/scenarios/id")?;
        if !ids.insert(sc.id.clone()) {
            return Err(format!("DuplicateId: {}", sc.id));
        }
    }
    Ok(())
}
fn integrity(s: &Store) -> Result<()> {
    relations::validate(s)?;
    checks::validate(s)?;
    let mut names = BTreeSet::new();
    for key in s.changes.keys() {
        if !names.insert(key.to_ascii_lowercase()) {
            return Err("CaseInsensitiveIdConflict: changes".into());
        }
    }

    if s.next_number == 0
        || s.changes
            .keys()
            .filter_map(|id| progress::number(id))
            .any(|n| n >= s.next_number)
    {
        return Err("InvalidCounter: next_number must exceed every assigned HG number".into());
    }
    let mut ids = s.retired_ids.clone();
    for (key, r) in &s.requirements {
        if key != &r.id {
            return Err(format!("IdMismatch: /requirements/{key}"));
        }
        requirement_ids(r, &mut ids)?;
    }
    for (key, c) in &s.changes {
        if key != &c.id {
            return Err(format!("IdMismatch: /changes/{key}"));
        }
        check_id(key, "/changes/id")?;
        let mut local = BTreeSet::new();
        for t in &c.tasks {
            check_id(&t.id, "/tasks/id")?;
            if !local.insert(t.id.clone()) {
                return Err("DuplicateId: /tasks".into());
            }
        }
        let mut delta_ids = BTreeSet::new();
        for d in &c.operations {
            if !delta_ids.insert(d.id()) {
                return Err(format!("DuplicateOperation: {}", d.id()));
            }
            if let Some(r) = d.requirement() {
                requirement_ids(r, &mut local)?;
            } else {
                check_id(d.id(), "/operations/id")?;
            }
        }
    }
    Ok(())
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn lock(root: &Path) -> Result<Lock> {
    // create_new rejects any existing entry, including symlinks. Inspecting the
    // lock's metadata first races with Windows delete-pending lock release.
    let p = paths::safe(root, ".highgrade/specs")?.join("write.lock");
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&p)
        .map_err(|e| {
            format!(
                "StoreBusy: {}; inspect owner before recovering abandoned lock: {e}",
                p.display()
            )
        })?;
    let guard = Lock(p);
    writeln!(f, "pid={}", std::process::id()).map_err(|e| e.to_string())?;
    Ok(guard)
}
fn write(root: &Path, s: &Store) -> Result<()> {
    integrity(s)?;
    if s.schema_version == 3 {
        return storage::write(root, s);
    }
    let bytes = serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("StoreTooLarge: split project scope before extending store".into());
    }
    let p = paths::safe(root, STORE)?;
    if p.exists() {
        crate::package::replace_active(&p, &bytes)
    } else {
        let temp = paths::safe(root, ".highgrade/specs/initial.part")?;
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| format!("StoreTempConflict: {e}"))?;
        f.write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        fs::rename(temp, p).map_err(|e| e.to_string())
    }
}
fn extend_baseline(c: &mut Change, requirements: &BTreeMap<String, Requirement>) -> Result<()> {
    if c.scoped_baseline != Some(true) {
        return Ok(());
    }
    for d in &c.operations {
        if matches!(d, Delta::Modify { .. } | Delta::Remove { .. })
            && !c.baseline.contains_key(d.id())
        {
            let r = requirements
                .get(d.id())
                .ok_or("RequirementMissing: baseline")?;
            c.baseline.insert(d.id().into(), digest(r));
        }
    }
    Ok(())
}
fn preserve_tag_revision(old: &Change, new: &mut Change) {
    if old.tags != new.tags && new.revision_tags.is_none() {
        new.revision_tags = Some(old.tags.clone());
    }
}
fn revision(c: &Change) -> String {
    progress::revision(c)
}
fn change_mut<'a>(s: &'a mut Store, id: &str) -> Result<&'a mut Change> {
    let c = s
        .changes
        .get_mut(id)
        .ok_or_else(|| format!("ChangeMissing: /changes/{id}"))?;
    if c.archived {
        return Err(format!("ArchivedChange: /changes/{id}"));
    }
    Ok(c)
}

/// Uniform CLI envelope; payload is the selective editable artifact, never a second authority.
pub fn command(root: &Path, op: &str, options: &BTreeMap<String, String>) -> Result<Report> {
    let root = paths::root(root)?;
    let get = |k: &str| {
        options
            .get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("Usage: required {k}"))
    };
    let mut report = Report::new(op);
    report.limitations.push("Structure, recorded observations and fingerprints do not certify semantic correctness, report authenticity, user acceptance or publication. Review declared input coverage.".into());
    if op == "spec-schema" {
        let mut schema =
            serde_json::to_value(schemars::schema_for!(Store)).map_err(|e| e.to_string())?;
        schema["properties"]["schema_version"]["const"] = json!(2);
        report.measurements.push(
            json!({"directory_format":storage::schema(),"store":schema,"change":schemars::schema_for!(Change),"evidence_input":schemars::schema_for!(EvidenceInput),"decision_input":schemars::schema_for!(progress::DecisionInput),"inputs":input_schemas()}),
        );
        return Ok(report);
    }
    if op == "spec-init" {
        let _guard = lock(&root)?;
        storage::init(
            &root,
            options
                .get("--directory")
                .map(String::as_str)
                .unwrap_or("specs"),
        )?;
        report.measurements.push(
            json!({"catalog_path":catalog_path(&root)?,"directory":storage::directory(&root)?,"pointer_path":"specs-location.json","store_sha256":load_unlocked(&root)?.1}),
        );
        return Ok(report);
    }
    if op == "spec-recover" {
        let _guard = lock(&root)?;
        storage::recover(&root, get("--expected")?)?;
        report
            .measurements
            .push(json!({"store_sha256":load_unlocked(&root)?.1}));
        return Ok(report);
    }
    let mutation = (op == "spec-metadata" && options.get("--apply").is_some_and(|v| v == "true"))
        || matches!(
            op,
            "spec-new"
                | "spec-save"
                | "spec-edit"
                | "spec-evidence"
                | "spec-evidence-batch"
                | "spec-run-import"
                | "spec-review"
                | "spec-integrate"
                | "spec-transfer"
                | "spec-import"
                | "spec-abandon"
                | "spec-migrate"
                | "spec-decide"
                | "spec-tag-set"
                | "spec-tag-remove"
                | "spec-tag-merge"
                | "spec-runner-set"
                | "spec-run"
        );
    let _guard = if mutation
        || paths::safe(&root, ".highgrade/specs")?.exists()
        || storage::exists(&root)?
    {
        Some(lock(&root)?)
    } else {
        None
    };
    let (mut store, sha) = if _guard.is_some() {
        load_unlocked(&root)?
    } else {
        (
            Store {
                schema_version: 3,
                ..Store::default()
            },
            "absent".into(),
        )
    };
    if mutation && options.contains_key("--expected-local") {
        if op != "spec-edit" || options.contains_key("--expected") {
            return Err("Usage: spec-edit accepts either --expected or --expected-local".into());
        }
        if get("--expected-local")? != local_revision(&store, get("--id")?)? {
            return Err("LocalConflict: reread selected change and dependencies".into());
        }
    } else if mutation && get("--expected")? != sha {
        return Err(format!("StoreConflict: /; current_sha256={sha}"));
    }
    if mutation && store.schema_version == 1 && op != "spec-migrate" {
        return Err("MigrationRequired: use spec-migrate with the observed store hash".into());
    }
    let allocated = if op == "spec-new" && !options.contains_key("--id") {
        Some(format!("HG-{:04}", store.next_number))
    } else {
        None
    };
    let change_id = options
        .get("--id")
        .map(String::as_str)
        .or(allocated.as_deref());
    if matches!(op, "spec-diff" | "spec-check" | "spec-validate") {
        get("--id")?;
    }
    if change_id.is_some() && options.contains_key("--requirement") {
        return Err("Usage: choose --id or --requirement".into());
    }
    if options
        .get("--view")
        .is_some_and(|v| !matches!(v.as_str(), "full" | "summary" | "editable" | "requirements"))
    {
        return Err("Usage: --view full|summary|editable|requirements".into());
    }
    if options.contains_key("--tag")
        && op == "spec-read"
        && (options.get("--view").map(String::as_str) != Some("requirements")
            || change_id.is_some()
            || options.contains_key("--requirement"))
    {
        return Err("Usage: --tag requires spec-read --view requirements without an id".into());
    }
    match op {
        "spec-list" => {
            report
                .measurements
                .push(relations::list(&root, &store, options)?);
            match catalog(&store) {
                Ok(c) => report.measurements.push(json!({"trace_sha256":digest(&c)})),
                Err(e) => report.finding("unknown", "ActiveSpecConflict", "/changes", &e),
            }
        }
        "spec-stats" => report
            .measurements
            .push(checks::statistics(&root, &store, get("--id")?)?),
        "spec-run-inputs" => report.measurements.push(
            json!({"snapshot":checks::capture_inputs(&root,&store,get("--id")?,get("--check")?)?}),
        ),
        "spec-run-import" => checks::import_report(
            &root,
            &mut store,
            &paths::safe(&root, get("--input")?)?,
            &mut report,
        )?,
        "spec-runner-set" => {
            let runner = decode(
                &paths::read_limited(&paths::safe(&root, get("--input")?)?, LIMIT)?,
                "runner",
            )?;
            store.runners.insert(get("--id")?.into(), runner);
        }
        "spec-run" => checks::execute(
            &root,
            &mut store,
            get("--id")?,
            get("--check")?,
            &mut report,
        )?,
        "spec-tags" => report.measurements.push(json!({"tags":store.tags})),
        "spec-tag-set" | "spec-tag-remove" | "spec-tag-merge" => {
            relations::mutation(&mut store, op, options)?
        }
        "spec-new" => {
            let key = change_id.expect("allocated id");
            check_id(key, "/changes/id")?;
            if store.changes.contains_key(key) {
                return Err("ChangeExists: /changes/id".into());
            }
            let req = Requirement {
                id: format!("{key}-R1"),
                title: String::new(),
                statement: String::new(),
                scenarios: vec![Scenario {
                    id: format!("{key}-S1"),
                    given: String::new(),
                    when: String::new(),
                    then: String::new(),
                    verification: String::new(),
                }],
            };
            let c = Change {
                id: key.into(),
                title: options.get("--title").cloned().unwrap_or_default(),
                created_at: Some(progress::now()?),
                acceptance: vec![],
                history: vec![],
                depends_on: vec![],
                related_to: vec![],
                tags: BTreeSet::new(),
                checks: vec![],
                shared_inputs: BTreeSet::new(),
                runs: vec![],
                goal: String::new(),
                rationale: String::new(),
                scope: String::new(),
                questions: vec![],
                tasks: vec![Task {
                    id: format!("{key}-T1"),
                    description: String::new(),
                    done: false,
                }],
                operations: vec![Delta::Add { requirement: req }],
                baseline: BTreeMap::new(),
                scoped_baseline: Some(true),
                revision_tags: None,
                imports: BTreeMap::new(),
                evidence: BTreeMap::new(),
                review: None,
                archived: false,
                abandoned_reason: None,
            };
            store.changes.insert(key.into(), c);
            progress::advance(&mut store, key)?;
        }
        "spec-edit" => {
            let patch: Value = decode(
                &paths::read_limited(&paths::safe(&root, get("--input")?)?, LIMIT)?,
                "/edit",
            )?;
            let fields = patch.as_object().ok_or("InvalidEdit: expected object")?;
            let requirements = store.requirements.clone();
            let c = change_mut(&mut store, get("--id")?)?;
            let previous = c.clone();
            let mut value = serde_json::to_value(&*c).unwrap();
            for (key, field) in fields {
                if !EDITABLE.contains(&key.as_str()) {
                    return Err(format!("ProtectedField: {key}"));
                }
                value[key] = field.clone();
            }
            *c = decode(&serde_json::to_vec(&value).unwrap(), "/change")?;
            preserve_tag_revision(&previous, c);
            if options.contains_key("--expected-local")
                && c.scoped_baseline == Some(true)
                && c.operations.iter().any(|d| {
                    matches!(d, Delta::Modify { .. } | Delta::Remove { .. })
                        && !c.baseline.contains_key(d.id())
                })
            {
                return Err(
                    "BaselineCaptureRequiresStoreCAS: reread requirements and use --expected"
                        .into(),
                );
            }
            extend_baseline(c, &requirements)?;
            if options.get("--validate").is_some_and(|v| v == "true") {
                readiness(
                    &root,
                    &store,
                    &store.changes[get("--id")?],
                    &mut report,
                    false,
                );
                if report.exit_code() != 0 {
                    return Ok(report);
                }
            }
        }
        "spec-save" => {
            let input = paths::safe(&root, get("--input")?)?;
            let mut incoming: Change = decode(&paths::read_limited(&input, LIMIT)?, "/change")?;
            let requirements = store.requirements.clone();
            let c = change_mut(&mut store, get("--id")?)?;
            if incoming.id != c.id
                || incoming.baseline != c.baseline
                || incoming.scoped_baseline != c.scoped_baseline
                || incoming.revision_tags != c.revision_tags
                || incoming.imports != c.imports
                || incoming.evidence != c.evidence
                || incoming.review != c.review
                || incoming.archived != c.archived
                || incoming.abandoned_reason != c.abandoned_reason
                || incoming.created_at != c.created_at
                || incoming.acceptance != c.acceptance
                || incoming.history != c.history
                || incoming.runs != c.runs
            {
                return Err(
                    "ProtectedField: /change; preserve id, baseline, evidence, review, archived"
                        .into(),
                );
            }
            preserve_tag_revision(c, &mut incoming);
            extend_baseline(&mut incoming, &requirements)?;
            *c = incoming;
        }
        "spec-read" | "spec-diff" | "spec-check" | "spec-validate" => {
            if op == "spec-read"
                && options.get("--view").is_some_and(|v| v == "requirements")
                && change_id.is_none()
                && !options.contains_key("--requirement")
            {
                let tag = options.get("--tag");
                if tag.is_some_and(|t| !store.tags.contains_key(t)) {
                    return Err("UnknownTag".into());
                }
                let selected: BTreeSet<_> = store
                    .changes
                    .values()
                    .filter(|c| tag.is_none_or(|t| c.tags.contains(t)))
                    .flat_map(|c| c.operations.iter().map(Delta::id))
                    .collect();
                let requirements: Vec<_> = store
                    .requirements
                    .values()
                    .filter(|r| tag.is_none() || selected.contains(r.id.as_str()))
                    .collect();
                report.measurements.push(json!({"requirements":requirements,"selection":"current requirements; tag coverage follows explicit change tags"}));
            } else if let Some(key) = change_id {
                let c = store.changes.get(key).ok_or("ChangeMissing: /changes/id")?;
                if op == "spec-check" || op == "spec-validate" {
                    readiness(&root, &store, c, &mut report, op == "spec-check");
                    if op == "spec-check" && options.get("--brief").is_some_and(|v| v == "true") {
                        let summary = brief(&root, &store, c, &report);
                        report.measurements.push(summary);
                    }
                }
                if op == "spec-diff" {
                    report.measurements.push(json!({"operations":c.operations,"current":c.operations.iter().map(|d| (d.id(), store.requirements.get(d.id()))).collect::<BTreeMap<_,_>>()}));
                }
            } else if let Some(key) = options.get("--requirement") {
                report.measurements.push(json!({"requirement":store.requirements.get(key).ok_or("RequirementMissing: /requirements/id")?}));
            } else {
                return Err("Usage: --id or --requirement required".into());
            }
        }
        "spec-metadata" => {
            if get("--expected")? != sha {
                return Err("StoreConflict: metadata preview/apply".into());
            }
            let entries: Vec<MetadataInput> = decode(
                &paths::read_limited(&paths::safe(&root, get("--input")?)?, LIMIT)?,
                "metadata",
            )?;
            if entries.is_empty() {
                return Err("EmptyMetadataBatch".into());
            }
            let mut seen = BTreeSet::new();
            let mut changes = Vec::new();
            for item in entries {
                if !seen.insert(item.id.clone()) {
                    return Err("DuplicateMetadataId".into());
                }
                if item.tags.iter().any(|t| !store.tags.contains_key(t)) {
                    return Err("UnknownTag".into());
                }
                let c = store.changes.get_mut(&item.id).ok_or("ChangeMissing")?;
                if c.tags != item.tags {
                    changes.push(json!({"id":item.id,"before":c.tags,"after":item.tags}));
                    if c.revision_tags.is_none() {
                        c.revision_tags = Some(c.tags.clone());
                    }
                    c.tags = item.tags;
                }
            }
            report
                .measurements
                .push(json!({"changes":changes,"applied":mutation,"atomic":true}));
        }
        "spec-evidence" => record_evidence(
            &root,
            result_mut(&mut store, get("--id")?)?,
            &paths::safe(&root, get("--input")?)?,
        )?,
        "spec-evidence-batch" => {
            let entries: Vec<BatchEvidence> = decode(
                &paths::read_limited(&paths::safe(&root, get("--input")?)?, LIMIT)?,
                "/batch",
            )?;
            if entries.is_empty() {
                return Err("EmptyEvidenceBatch".into());
            }
            let mut changed = BTreeSet::new();
            let mut seen = BTreeSet::new();
            for entry in entries {
                if !seen.insert((entry.id.clone(), entry.evidence.scenario.clone())) {
                    return Err("DuplicateBatchScenario".into());
                }
                let original = store.changes.get(&entry.id).ok_or("ChangeMissing")?;
                if original.abandoned_reason.is_some() {
                    return Err("AbandonedChange".into());
                }
                let mut candidate = original.clone();
                apply_evidence(&root, &mut candidate, entry.evidence)?;
                if candidate.evidence != original.evidence {
                    if changed.insert(entry.id.clone()) {
                        result_mut(&mut store, &entry.id)?;
                    }
                    store.changes.get_mut(&entry.id).unwrap().evidence = candidate.evidence;
                }
            }
            report
                .measurements
                .push(json!({"updated":changed,"atomic":true}));
        }
        "spec-review" => {
            let c = result_mut(&mut store, get("--id")?)?;
            let reviewer = get("--reviewer")?;
            let conclusion = get("--conclusion")?;
            if reviewer.trim().is_empty() || conclusion.trim().is_empty() {
                return Err("ReviewIncomplete: /review".into());
            }
            let verdict = match get("--verdict")? {
                "go" => Verdict::Go,
                "no_go" => Verdict::NoGo,
                _ => return Err("InvalidVerdict: /review/verdict; go or no_go".into()),
            };
            c.review = Some(Review {
                verdict,
                reviewer: reviewer.into(),
                conclusion: conclusion.into(),
                change_sha256: revision(c),
            });
        }
        "spec-integrate" => {
            let c = change_mut(&mut store, get("--id")?)?.clone();
            readiness(&root, &store, &c, &mut report, true);
            if report.exit_code() != 0 {
                return Ok(report);
            }
            for d in &c.operations {
                match d {
                    Delta::Add { requirement } | Delta::Modify { requirement } => {
                        if let Some(old) = store
                            .requirements
                            .insert(requirement.id.clone(), requirement.clone())
                        {
                            for sc in old.scenarios {
                                if !requirement.scenarios.iter().any(|s| s.id == sc.id) {
                                    store.retired_ids.insert(sc.id);
                                }
                            }
                        }
                    }
                    Delta::Remove { id, .. } => {
                        if let Some(old) = store.requirements.remove(id) {
                            store.retired_ids.insert(id.clone());
                            store
                                .retired_ids
                                .extend(old.scenarios.into_iter().map(|s| s.id));
                        }
                    }
                }
            }
            store.origins.extend(c.imports.clone());
            store.changes.get_mut(&c.id).unwrap().archived = true;
        }
        "spec-transfer" => {
            let c = change_mut(&mut store, get("--id")?)?.clone();
            readiness(&root, &store, &c, &mut report, false);
            let imported_adds = !c.imports.is_empty()
                && c.imports.len() == c.operations.len()
                && c.operations.iter().all(|d| matches!(d, Delta::Add { .. }));
            if !imported_adds {
                report.finding(
                    "failed",
                    "TransferRequiresImports",
                    "/operations",
                    "Only imported add requirements can be transferred.",
                );
            }
            if c.tasks.iter().any(|t| !t.done) {
                report.finding(
                    "failed",
                    "TransferTasksIncomplete",
                    "/tasks",
                    "Complete the contract transfer tasks before transfer.",
                );
            }
            if !c.review.as_ref().is_some_and(|r| {
                r.verdict == Verdict::Go && progress::matches_revision(&c, &r.change_sha256)
            }) {
                report.finding(
                    "failed",
                    "TransferReviewMissingOrStale",
                    "/review",
                    "A current independent equivalence review is required.",
                );
            }
            for link in &c.depends_on {
                if !store
                    .changes
                    .get(&link.id)
                    .is_some_and(|d| d.archived && d.abandoned_reason.is_none())
                {
                    report.finding(
                        "failed",
                        "DependencyNotIntegrated",
                        "/depends_on",
                        "A dependency is not integrated.",
                    );
                }
            }
            if report.exit_code() != 0 {
                return Ok(report);
            }
            for d in &c.operations {
                if let Delta::Add { requirement } = d {
                    store
                        .requirements
                        .insert(requirement.id.clone(), requirement.clone());
                }
            }
            store.origins.extend(c.imports.clone());
            store.changes.get_mut(&c.id).unwrap().archived = true;
            report.finding(
                "warning",
                "BehaviorUnconfirmed",
                &c.id,
                "The contract was transferred; scenario behavior is not certified.",
            );
            report.measurements.push(json!({
                "transferred_requirements":c.imports.keys().collect::<Vec<_>>(),
                "behavior_verified":false,
                "human_accepted":false
            }));
        }
        "spec-abandon" => {
            let reason = get("--reason")?;
            if reason.trim().is_empty() {
                return Err("ReasonMissing: /abandoned_reason".into());
            }
            let c = change_mut(&mut store, get("--id")?)?;
            c.archived = true;
            c.abandoned_reason = Some(reason.into());
        }
        "spec-import" => import(
            &root,
            &mut store,
            get("--id")?,
            get("--input")?,
            get("--source")?,
        )?,
        "spec-migrate" => {
            match options.get("--to").map(String::as_str) {
                Some("compact") => {
                    storage::compact(&root, &store)?;
                    report.measurements.push(json!({"store_sha256":load_unlocked(&root)?.1,"directory_schema_version":4}));
                    return Ok(report);
                }
                Some("directory") => storage::migrate(&root, &mut store, &sha)?,
                None if store.schema_version != 3 => {
                    progress::migrate(&root, &mut store, &sha, &mut report)?
                }
                None => {}
                _ => return Err("Usage: --to directory|compact".into()),
            }
        }
        "spec-decide" => {
            progress::decide(&root, &mut store, &paths::safe(&root, get("--input")?)?)?
        }
        _ => return Err("Usage: unknown spec operation".into()),
    }
    if mutation {
        if load_unlocked(&root)?.1 != sha {
            return Err("StoreConflict: inputs changed during operation".into());
        }
        write(&root, &store)?;
    }
    let current_sha = if mutation {
        load_unlocked(&root)?.1
    } else {
        sha
    };
    report
        .measurements
        .push(json!({"store_sha256":current_sha,"schema_version":store.schema_version}));
    if let Some(key) = change_id {
        if let Some(c) = store.changes.get(key) {
            if op == "spec-read" {
                report
                    .measurements
                    .push(json!({"local_sha256":local_revision(&store,key)?}));
            }
            if matches!(op, "spec-edit" | "spec-integrate")
                && options.get("--brief").is_some_and(|v| v == "true")
            {
                let mut state = Report::new("spec-check");
                readiness(&root, &store, c, &mut state, true);
                report.measurements.push(brief(&root, &store, c, &state));
            } else if op == "spec-read" && options.get("--view").is_some_and(|v| v != "full") {
                let view = options["--view"].as_str();
                let value = serde_json::to_value(c).unwrap();
                let selected = match view {
                    "editable" => Value::Object(
                        EDITABLE
                            .iter()
                            .filter_map(|key| value.get(*key).map(|v| ((*key).into(), v.clone())))
                            .collect(),
                    ),
                    "summary" => {
                        json!({"id":c.id,"title":c.title,"tags":c.tags,"goal":c.goal,"rationale":c.rationale,"scope":c.scope,"tasks":c.tasks,"questions":c.questions,"links":relations::metadata(&store,c),"contract_sha256":progress::contract_revision(c)})
                    }
                    "requirements" => json!({"id":c.id,"operations":c.operations}),
                    _ => return Err("Usage: --view full|summary|editable|requirements".into()),
                };
                report
                    .measurements
                    .push(json!({"view":view,"value":selected}));
            } else if !matches!(op, "spec-stats" | "spec-list" | "spec-run-inputs")
                && !(op == "spec-check" && options.get("--brief").is_some_and(|v| v == "true"))
            {
                report
                    .measurements
                    .push(json!({"change":c,"links":relations::metadata(&store,c),"change_sha256":revision(c),"inputs_sha256":progress::inputs_hash(&root,c).ok()}));
            }
        }
    }
    Ok(report)
}
fn local_revision(s: &Store, id: &str) -> Result<String> {
    s.changes.get(id).ok_or("ChangeMissing")?;
    let mut pending = vec![id.to_string()];
    let mut scope = BTreeMap::new();
    while let Some(id) = pending.pop() {
        if scope.contains_key(&id) {
            continue;
        }
        let change = s.changes.get(&id).ok_or("DependencyMissing")?;
        pending.extend(change.depends_on.iter().map(|l| l.id.clone()));
        scope.insert(id, change);
    }
    let requirements: BTreeMap<_, _> = scope
        .values()
        .flat_map(|c| c.operations.iter())
        .map(|d| (d.id(), s.requirements.get(d.id())))
        .collect();
    let runners: BTreeMap<_, _> = scope
        .values()
        .flat_map(|c| c.checks.iter())
        .filter_map(|b| b.runner.as_ref())
        .map(|id| (id, s.runners.get(id)))
        .collect();
    Ok(digest(&(scope, requirements, runners, &s.retired_ids)))
}

const EDITABLE: &[&str] = &[
    "title",
    "goal",
    "rationale",
    "scope",
    "questions",
    "tasks",
    "operations",
    "depends_on",
    "related_to",
    "tags",
    "checks",
    "shared_inputs",
];

fn readiness(root: &Path, store: &Store, c: &Change, report: &mut Report, complete: bool) {
    let mut missing = |location: &str, code: &str| {
        report.finding(
            "failed",
            code,
            location,
            "Change is not ready for integration.",
        )
    };
    if complete {
        for binding in &c.checks {
            if !checks::fresh(root, store, c, binding) {
                missing("/checks", "CheckMissingOrStale");
            }
        }
        for link in &c.depends_on {
            if !store
                .changes
                .get(&link.id)
                .is_some_and(|d| d.archived && d.abandoned_reason.is_none())
            {
                missing("/depends_on", "DependencyNotIntegrated");
            }
        }
    }
    if c.abandoned_reason.is_some() {
        missing("/archived", "AlreadyIntegrated");
        return;
    }
    if c.created_at.is_some() && c.title.trim().is_empty() {
        missing("/title", "DraftIncomplete");
    }
    for (field, value) in [
        ("goal", &c.goal),
        ("rationale", &c.rationale),
        ("scope", &c.scope),
    ] {
        if value.trim().is_empty() {
            missing(&format!("/{field}"), "DraftIncomplete");
        }
    }
    if !c.questions.is_empty() {
        missing("/questions", "OpenQuestions");
    }
    if c.tasks.is_empty()
        || c.tasks
            .iter()
            .any(|t| (complete && !t.done) || t.description.trim().is_empty())
    {
        missing("/tasks", "TasksIncomplete");
    }
    if c.operations.is_empty() {
        missing("/operations", "EmptyChange");
    }
    if let Err(e) = valid_imports(c) {
        missing("/imports", &e);
    }
    for origin in c.imports.values() {
        if !fingerprint(root, &origin.path).is_ok_and(|sha| sha == origin.sha256) {
            missing("/imports", "ImportSourceDrift");
        }
    }
    let mut candidate = store.requirements.clone();
    for (i, d) in c.operations.iter().enumerate() {
        let loc = format!("/operations/{i}");
        let old = store.requirements.get(d.id());
        match d {
            _ if c.archived => {}
            Delta::Add { .. } => {
                if old.is_some()
                    || store.retired_ids.contains(d.id())
                    || store.origins.contains_key(d.id())
                {
                    missing(&loc, "IdAlreadyOwned");
                }
            }
            Delta::Modify { .. } | Delta::Remove { .. } => {
                if old.is_none() || old.map(digest).as_ref() != c.baseline.get(d.id()) {
                    missing(&loc, "BaseDrift");
                }
            }
        }
        if let Delta::Remove { id, reason } = d {
            if reason.trim().is_empty() {
                missing(&loc, "RemovalReasonMissing");
            }
            candidate.remove(id);
        }
        if let Some(r) = d.requirement() {
            candidate.insert(r.id.clone(), r.clone());
            if r.title.trim().is_empty() || r.statement.trim().is_empty() || r.scenarios.is_empty()
            {
                missing(&loc, "RequirementIncomplete");
            }
            for (j, sc) in r.scenarios.iter().enumerate() {
                let sloc = format!("{loc}/requirement/scenarios/{j}");
                if [&sc.given, &sc.when, &sc.then, &sc.verification]
                    .iter()
                    .any(|s| s.trim().is_empty())
                {
                    missing(&sloc, "ScenarioIncomplete");
                }
                let good = c.evidence.get(&sc.id).is_some_and(|e| {
                    e.scenario == sc.id
                        && e.outcome == Outcome::Passed
                        && e.scenario_sha256 == scenario_revision(r, sc)
                        && !e.files.is_empty()
                        && e.files.iter().all(|(rel, expected)| {
                            fingerprint(root, rel).is_ok_and(|v| &v == expected)
                        })
                });
                if complete && !good {
                    missing(&sloc, "EvidenceMissingOrStale");
                }
            }
        }
    }
    if !c.archived {
        let mut ids = store.retired_ids.clone();
        for r in candidate.values() {
            if requirement_ids(r, &mut ids).is_err() {
                missing("/operations", "IdConflict");
            }
        }
    }
    if complete
        && !c.review.as_ref().is_some_and(|r| {
            r.verdict == Verdict::Go && progress::matches_revision(c, &r.change_sha256)
        })
    {
        missing("/review", "ReviewMissingOrStale");
    }
}
fn evidence_reason(
    root: &Path,
    r: &Requirement,
    sc: &Scenario,
    e: Option<&Evidence>,
) -> Option<Value> {
    let Some(e) = e else {
        return Some(
            json!({"kind":"evidence","id":sc.id,"reason":"missing_evidence","next":"record_evidence"}),
        );
    };
    if e.outcome != Outcome::Passed {
        return Some(
            json!({"kind":"evidence","id":sc.id,"reason":"outcome_not_passed","next":"repeat_scenario"}),
        );
    }
    if e.scenario_sha256 != scenario_revision(r, sc) {
        return Some(
            json!({"kind":"evidence","id":sc.id,"reason":"scenario_changed","next":"repeat_scenario"}),
        );
    }
    if e.files.is_empty() {
        return Some(
            json!({"kind":"evidence","id":sc.id,"reason":"inputs_missing","next":"record_evidence"}),
        );
    }
    for (path, expected) in &e.files {
        let reason = match fingerprint(root, path) {
            Ok(actual) if actual == *expected => continue,
            Ok(_) => "input_changed",
            Err(_) => "input_unavailable",
        };
        return Some(
            json!({"kind":"evidence","id":sc.id,"reason":reason,"path":path,"next":"repeat_scenario"}),
        );
    }
    None
}
fn brief(root: &Path, store: &Store, c: &Change, report: &Report) -> Value {
    let mut blockers = Vec::new();
    for binding in &c.checks {
        if let Some(issue) = checks::stale_reason(root, store, c, binding) {
            blockers.push(issue);
        }
    }
    for requirement in c.operations.iter().filter_map(Delta::requirement) {
        for scenario in &requirement.scenarios {
            if let Some(issue) =
                evidence_reason(root, requirement, scenario, c.evidence.get(&scenario.id))
            {
                blockers.push(issue);
            }
        }
    }
    let current_revision = revision(c);
    let review = match c.review.as_ref() {
        None => "missing",
        Some(r) if r.verdict != Verdict::Go => "no_go",
        Some(r) if !progress::matches_revision(c, &r.change_sha256) => "stale",
        Some(_) => "go",
    };
    if review != "go" {
        blockers.push(json!({"kind":"review","reason":review,"next":if review == "no_go" { "resolve_review" } else { "repeat_review" }}));
    }
    let technical_ready = report.exit_code() == 0;
    let human = progress::human_state(root, c, technical_ready);
    let verified_revision = if human == "accepted" {
        c.acceptance.last().map(|d| d.verified_revision.as_str())
    } else {
        None
    };
    json!({
        "brief": {
            "id": c.id,
            "technical": if technical_ready { "ready" } else { "in_progress" },
            "human": human,
            "change_sha256": current_revision,
            "contract_sha256": progress::contract_revision(c),
            "inputs_sha256": progress::inputs_hash(root, c).ok(),
            "review": review,
            "verified_revision": verified_revision,
            "blockers": blockers,
        }
    })
}
fn fingerprint(root: &Path, rel: &str) -> Result<String> {
    Ok(hash(&paths::read_limited(
        &paths::safe(root, rel)?,
        64 * 1024 * 1024,
    )?))
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EvidenceInput {
    command: String,
    captured_at: String,
    method: String,
    scenario: String,
    outcome: Outcome,
    observation: String,
    inputs: Vec<String>,
    report: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MetadataInput {
    id: String,
    tags: BTreeSet<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct BatchEvidence {
    id: String,
    evidence: EvidenceInput,
}
fn record_evidence(root: &Path, c: &mut Change, input: &Path) -> Result<()> {
    let v: EvidenceInput = decode(&paths::read_limited(input, LIMIT)?, "/evidence")?;
    apply_evidence(root, c, v)
}
fn apply_evidence(root: &Path, c: &mut Change, v: EvidenceInput) -> Result<()> {
    let r = c
        .operations
        .iter()
        .filter_map(Delta::requirement)
        .find(|r| r.scenarios.iter().any(|s| s.id == v.scenario))
        .ok_or("ScenarioMissing: /evidence/scenario")?;
    if v.observation.trim().is_empty()
        || v.inputs.is_empty()
        || v.command.trim().is_empty()
        || v.captured_at.trim().is_empty()
        || !matches!(v.method.as_str(), "manual" | "native_report")
    {
        return Err("EvidenceIncomplete: /evidence/observation or inputs".into());
    }
    let mut files = BTreeMap::new();
    for rel in v.inputs.iter().chain(std::iter::once(&v.report)) {
        if rel.starts_with(".highgrade/specs/") {
            return Err("EvidenceInputInvalid: store cannot be its own evidence".into());
        }
        files.insert(rel.clone(), fingerprint(root, rel)?);
    }
    c.evidence.insert(
        v.scenario.clone(),
        Evidence {
            command: v.command,
            captured_at: v.captured_at,
            method: v.method,
            scenario: v.scenario.clone(),
            outcome: v.outcome,
            observation: v.observation,
            scenario_sha256: scenario_revision(
                r,
                r.scenarios.iter().find(|s| s.id == v.scenario).unwrap(),
            ),
            files,
            input_paths: Some(v.inputs.into_iter().collect()),
            report: v.report,
        },
    );
    Ok(())
}
fn scenario_revision(r: &Requirement, s: &Scenario) -> String {
    digest(&(r.id.as_str(), r.title.as_str(), r.statement.as_str(), s))
}

/// Effective contracts for trace. Conflicting active edits require explicit resolution.
pub(crate) fn catalog(store: &Store) -> Result<BTreeMap<String, Requirement>> {
    let mut current = store.requirements.clone();
    let mut touched = BTreeSet::new();
    for c in store.changes.values().filter(|c| !c.archived) {
        valid_imports(c)?;
        for delta in &c.operations {
            if !touched.insert(delta.id()) {
                return Err(format!("ActiveSpecConflict: {}", delta.id()));
            }
            if let Some(r) = delta.requirement() {
                current.insert(r.id.clone(), r.clone());
            } else {
                current.remove(delta.id());
            }
        }
    }
    Ok(current)
}
pub fn catalog_hash(store: &Store) -> Result<String> {
    Ok(digest(&catalog(store)?))
}
pub fn trace_hash(root: &Path) -> Result<String> {
    catalog_hash(&load(root)?.0)
}
fn import(root: &Path, store: &mut Store, key: &str, input: &str, source: &str) -> Result<()> {
    check_id(key, "/changes/id")?;
    if store.changes.contains_key(key) {
        return Err("ChangeExists: /changes/id".into());
    }
    let r: Requirement = decode(
        &paths::read_limited(&paths::safe(root, input)?, LIMIT)?,
        "/requirement",
    )?;
    let text = String::from_utf8(paths::read_limited(&paths::safe(root, source)?, LIMIT)?)
        .map_err(|e| e.to_string())?;
    let marker = format!("### Requirement: [{}]", r.id);
    let lines: Vec<_> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.starts_with(&marker))
        .ok_or("LegacyIdMissing: /requirement/id")?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| l.starts_with("### Requirement:"))
        .map(|(i, _)| i)
        .unwrap_or(lines.len());
    let original = lines[start..end].join("\n");
    let legacy_ids: BTreeSet<_> = lines[start..end]
        .iter()
        .filter_map(|l| {
            l.strip_prefix("#### Scenario: [")
                .and_then(|v| v.split_once(']').map(|p| p.0.to_owned()))
        })
        .collect();
    let supplied: BTreeSet<_> = r.scenarios.iter().map(|s| s.id.clone()).collect();
    if supplied.is_empty() || supplied != legacy_ids {
        return Err("ImportScenarioMismatch: /scenarios; preserve every legacy scenario ID".into());
    }
    if store.requirements.contains_key(&r.id) || store.retired_ids.contains(&r.id) {
        return Err("IdAlreadyOwned: /requirement/id".into());
    }
    let mut imports = BTreeMap::new();
    imports.insert(
        r.id.clone(),
        Origin {
            path: source.into(),
            sha256: hash(text.as_bytes()),
            original_text: original,
        },
    );
    let c = Change {
        id: key.into(),
        title: format!("Transfer {}", r.id),
        created_at: Some(progress::now()?),
        acceptance: vec![],
        history: vec![],
        depends_on: vec![],
        related_to: vec![],
        tags: BTreeSet::new(),
        checks: vec![],
        shared_inputs: BTreeSet::new(),
        runs: vec![],
        goal: format!("Transfer {} without changing its contract", r.id),
        rationale:
            "Explicit selected legacy transfer; compare original_text during independent review"
                .into(),
        scope: source.into(),
        questions: vec![],
        tasks: vec![Task {
            id: format!("{key}-T1"),
            description: "Verify semantic equivalence, scenarios and current evidence".into(),
            done: false,
        }],
        operations: vec![Delta::Add { requirement: r }],
        baseline: BTreeMap::new(),
        scoped_baseline: Some(true),
        revision_tags: None,
        imports,
        evidence: BTreeMap::new(),
        review: None,
        archived: false,
        abandoned_reason: None,
    };
    store.changes.insert(key.into(), c);
    progress::advance(store, key)?;
    Ok(())
}

fn valid_imports(c: &Change) -> Result<()> {
    for (id, origin) in &c.imports {
        let r = c
            .operations
            .iter()
            .find_map(|d| match d {
                Delta::Add { requirement } if &requirement.id == id => Some(requirement),
                _ => None,
            })
            .ok_or("ImportOperationMismatch")?;
        let original_ids: BTreeSet<_> = origin
            .original_text
            .lines()
            .filter_map(|l| {
                l.strip_prefix("#### Scenario: [")
                    .and_then(|s| s.split_once(']').map(|p| p.0))
            })
            .collect();
        let final_ids: BTreeSet<_> = r.scenarios.iter().map(|s| s.id.as_str()).collect();
        if original_ids.is_empty() || original_ids != final_ids {
            return Err("ImportScenarioMismatch".into());
        }
    }
    Ok(())
}
