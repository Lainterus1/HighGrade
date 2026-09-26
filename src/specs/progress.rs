//! Numbering, explicit format upgrade and version-bound human decisions.
use super::*;
use serde::ser::SerializeStruct;

pub(super) fn now() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_secs())
        .map_err(|e| format!("ClockInvalid: {e}"))
}
pub(super) fn number(id: &str) -> Option<u64> {
    let tail = id.strip_prefix("HG-")?;
    if tail.is_empty() || !tail.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    tail.parse().ok()
}
pub(super) fn advance(s: &mut Store, id: &str) -> Result<()> {
    if let Some(n) = number(id) {
        s.next_number = s
            .next_number
            .max(n.checked_add(1).ok_or("CounterExhausted")?);
    }
    Ok(())
}

pub(super) fn decode_store(bytes: &[u8]) -> Result<Store> {
    let mut v: Value = decode(bytes, STORE)?;
    match v["schema_version"].as_u64() {
        Some(2) => decode(bytes, STORE),
        Some(1) => {
            if v.get("next_number").is_some() {
                return Err("InvalidFormat: v1 next_number".into());
            }
            v["next_number"] = json!(1);
            let changes = v["changes"]
                .as_object_mut()
                .ok_or("InvalidFormat: changes")?;
            let mut next = 1;
            for (id, c) in changes {
                let c = c.as_object_mut().ok_or("InvalidFormat: change")?;
                for (field, value) in [
                    ("title", json!("")),
                    ("created_at", Value::Null),
                    ("acceptance", json!([])),
                ] {
                    if c.insert(field.into(), value).is_some() {
                        return Err(format!("InvalidFormat: v1 {field}"));
                    }
                }
                if let Some(n) = number(id) {
                    next = next.max(n.checked_add(1).ok_or("CounterExhausted")?);
                }
            }
            v["next_number"] = json!(next);
            decode(&serde_json::to_vec(&v).unwrap(), STORE)
        }
        _ => Err("UnsupportedVersion: /schema_version; supported: 1, 2".into()),
    }
}
pub(super) fn migrate(root: &Path, s: &mut Store, sha: &str, report: &mut Report) -> Result<()> {
    if s.schema_version == 2 {
        return Ok(());
    }
    let bytes = paths::read_limited(&paths::safe(root, STORE)?, LIMIT)?;
    if hash(&bytes) != sha {
        return Err("StoreConflict: migration source".into());
    }
    let rel = format!(".highgrade/specs/backups/v1-{sha}.json");
    let p = paths::safe(root, &rel)?;
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    match OpenOptions::new().write(true).create_new(true).open(&p) {
        Ok(mut f) => f
            .write_all(&bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if paths::read_limited(&p, LIMIT)? != bytes {
                return Err("BackupConflict: migration backup differs".into());
            }
        }
        Err(e) => return Err(format!("BackupFailed: {e}")),
    }
    s.schema_version = 2;
    report
        .measurements
        .push(json!({"backup":rel,"source_sha256":sha}));
    Ok(())
}

// Keep the exact v1 serialization order for old reviews. Bookkeeping and human
// decisions never alter the technical revision; a title edit does.
pub(super) fn legacy_revision(c: &Change) -> String {
    struct Revision<'a>(&'a Change);
    impl Serialize for Revision<'_> {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            let c = self.0;
            let mut s =
                serializer.serialize_struct("Change", 13 + usize::from(!c.title.is_empty()))?;
            s.serialize_field("id", &c.id)?;
            s.serialize_field("goal", &c.goal)?;
            s.serialize_field("rationale", &c.rationale)?;
            s.serialize_field("scope", &c.scope)?;
            s.serialize_field("questions", &c.questions)?;
            s.serialize_field("tasks", &c.tasks)?;
            s.serialize_field("operations", &c.operations)?;
            s.serialize_field("baseline", &c.baseline)?;
            s.serialize_field("imports", &c.imports)?;
            s.serialize_field("evidence", &c.evidence)?;
            s.serialize_field("review", &Option::<Review>::None)?;
            s.serialize_field("archived", &false)?;
            s.serialize_field("abandoned_reason", &c.abandoned_reason)?;
            if !c.title.is_empty() {
                s.serialize_field("title", &c.title)?;
            }
            if !c.depends_on.is_empty() {
                s.serialize_field("depends_on", &c.depends_on)?;
            }
            if !c.related_to.is_empty() {
                s.serialize_field("related_to", &c.related_to)?;
            }
            if !c.checks.is_empty() {
                s.serialize_field("checks", &c.checks)?;
            }
            if !c.runs.is_empty() {
                s.serialize_field("runs", &c.runs)?;
            }
            s.end()
        }
    }
    digest(&Revision(c))
}

pub(super) fn contract_revision(c: &Change) -> String {
    let mut value = serde_json::to_value(c).expect("serializable change");
    let object = value.as_object_mut().unwrap();
    for field in [
        "evidence",
        "runs",
        "review",
        "acceptance",
        "history",
        "archived",
        "created_at",
        "title",
        "rationale",
        "related_to",
    ] {
        object.remove(field);
    }
    for task in object.get_mut("tasks").unwrap().as_array_mut().unwrap() {
        task.as_object_mut().unwrap().remove("done");
    }
    digest(&value)
}

pub(super) fn revision(c: &Change) -> String {
    let evidence: BTreeMap<_, _> = c.evidence.iter().map(|(id, e)| {
        let files: BTreeMap<_, _> = e.files.iter().filter(|(p, _)| e.input_paths.as_ref().is_none_or(|inputs| inputs.contains(*p))).collect();
        (id, json!({"scenario":e.scenario_sha256,"method":e.method,"outcome":e.outcome,"observation":e.observation,"command":e.command,"files":files}))
    }).collect();
    let mut latest = BTreeMap::new();
    for run in &c.runs {
        let files: BTreeMap<_, _> = run
            .files
            .iter()
            .filter(|(p, _)| {
                run.inputs_after
                    .as_ref()
                    .is_none_or(|inputs| inputs.contains_key(*p))
            })
            .collect();
        latest.insert(&run.check, json!({"definition":run.check_sha256,"outcome":run.outcome,"files":files,"inputs_after":run.inputs_after}));
    }
    digest(
        &json!({"revision_schema":2,"contract":contract_revision(c),"evidence":evidence,"latest_checks":latest}),
    )
}

pub(super) fn matches_revision(c: &Change, expected: &str) -> bool {
    expected == revision(c) || expected == legacy_revision(c)
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HumanVerdict {
    Accepted,
    NeedsChanges,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub decision: HumanVerdict,
    pub decided_by: String,
    /// UTC seconds since Unix epoch, assigned by the tool.
    pub decided_at: u64,
    pub change_sha256: String,
    pub inputs_sha256: String,
    pub verified_revision: String,
    pub comment: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct DecisionInput {
    decisions: Vec<DecisionItem>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct DecisionItem {
    id: String,
    decision: HumanVerdict,
    decided_by: String,
    change_sha256: String,
    inputs_sha256: String,
    verified_revision: String,
    comment: String,
}
pub(super) fn inputs_hash(root: &Path, c: &Change) -> Result<String> {
    inputs_hash_with_reports(root, c, false)
}
fn inputs_hash_with_reports(root: &Path, c: &Change, include_reports: bool) -> Result<String> {
    let mut files = BTreeMap::new();
    for e in c.evidence.values() {
        for rel in e.files.keys() {
            if !include_reports
                && e.input_paths
                    .as_ref()
                    .is_some_and(|inputs| !inputs.contains(rel))
            {
                continue;
            }
            files.insert(rel, fingerprint(root, rel)?);
        }
    }
    Ok(digest(&files))
}
fn ready(root: &Path, s: &Store, c: &Change) -> bool {
    let mut report = Report::new("spec-check");
    readiness(root, s, c, &mut report, true);
    report.exit_code() == 0
}
pub(super) fn human_state(root: &Path, c: &Change, technical_ready: bool) -> &'static str {
    match c.acceptance.last() {
        None => "pending",
        Some(d)
            if !matches_revision(c, &d.change_sha256)
                || !inputs_hash_with_reports(root, c, d.change_sha256 == legacy_revision(c))
                    .is_ok_and(|h| h == d.inputs_sha256) =>
        {
            "stale"
        }
        Some(d) if d.decision == HumanVerdict::NeedsChanges => "needs_changes",
        Some(_) if technical_ready => "accepted",
        Some(_) => "stale",
    }
}
pub(super) fn decide(root: &Path, s: &mut Store, input: &Path) -> Result<()> {
    let v: DecisionInput = decode(&paths::read_limited(input, LIMIT)?, "/decisions")?;
    if v.decisions.is_empty() {
        return Err("DecisionIncomplete: empty batch".into());
    }
    let mut seen = BTreeSet::new();
    let at = now()?;
    // Validate every item before changing any item; the outer store write is atomic.
    for item in &v.decisions {
        if !seen.insert(&item.id) {
            return Err("DuplicateDecision: id".into());
        }
        let c = s.changes.get(&item.id).ok_or("ChangeMissing: decision")?;
        if c.abandoned_reason.is_some() {
            return Err("AbandonedChange: decision".into());
        }
        if item.decided_by.trim().is_empty()
            || item.verified_revision.trim().is_empty()
            || (item.decision == HumanVerdict::NeedsChanges && item.comment.trim().is_empty())
        {
            return Err(
                "DecisionIncomplete: author, verified revision or rejection comment".into(),
            );
        }
        if item.change_sha256 != revision(c) || item.inputs_sha256 != inputs_hash(root, c)? {
            return Err("DecisionStale: reviewed specification or inputs changed".into());
        }
        if item.decision == HumanVerdict::Accepted && !ready(root, s, c) {
            return Err("DecisionNotReady: technical checks must pass".into());
        }
    }
    for item in v.decisions {
        s.changes
            .get_mut(&item.id)
            .unwrap()
            .acceptance
            .push(Decision {
                decision: item.decision,
                decided_by: item.decided_by,
                decided_at: at,
                change_sha256: item.change_sha256,
                inputs_sha256: item.inputs_sha256,
                verified_revision: item.verified_revision,
                comment: item.comment,
            });
    }
    Ok(())
}
pub(super) fn list(root: &Path, s: &Store) -> Value {
    let mut counts = BTreeMap::from([
        ("total", s.changes.len()),
        ("ready", 0),
        ("in_progress", 0),
        ("abandoned", 0),
        ("accepted", 0),
        ("needs_changes", 0),
        ("pending", 0),
        ("stale", 0),
    ]);
    let mut changes = Vec::new();
    for c in s.changes.values() {
        let technical = if c.abandoned_reason.is_some() {
            "abandoned"
        } else if ready(root, s, c) {
            "ready"
        } else {
            "in_progress"
        };
        let human = human_state(root, c, technical == "ready");
        *counts.get_mut(technical).unwrap() += 1;
        *counts.get_mut(human).unwrap() += 1;
        changes.push(json!({"id":c.id,"title":c.title,"goal":c.goal,"created_at":c.created_at,"archived":c.archived,"technical":technical,"human":human,"change_sha256":revision(c)}));
    }
    json!({"requirements":s.requirements.keys().collect::<Vec<_>>(),"changes":changes,"summary":counts,"origins":s.origins.iter().map(|(id,o)|(id,&o.path)).collect::<BTreeMap<_,_>>()})
}
