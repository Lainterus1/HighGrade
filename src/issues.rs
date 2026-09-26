//! Project-owned known problems. Recipes are data and never executed.
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
const CATALOG: &str = "issues/catalog.json";
const JOURNAL: &str = "issues/transaction.json";
const LIMIT: u64 = 8 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: u32,
    next_number: u64,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Open,
    Mitigated,
    Resolved,
    Stale,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Applicability {
    environment: String,
    conditions: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum CauseStatus {
    Unknown,
    Confirmed,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Cause {
    status: CauseStatus,
    description: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Resolution {
    description: String,
    prevention: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Content {
    title: String,
    tags: BTreeSet<String>,
    status: Status,
    symptom: String,
    applicability: Applicability,
    cause: Cause,
    resolution: Resolution,
    related_specs: BTreeSet<String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Passed,
    Failed,
    Unknown,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Observation {
    occurrence_key: String,
    outcome: Outcome,
    action: String,
    observed: String,
    evidence: BTreeSet<String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Verification {
    observation: Observation,
    recorded_at: u64,
    content_sha256: String,
    evidence_sha256: BTreeMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Issue {
    id: String,
    content: Content,
    occurrences: u64,
    created_at: u64,
    updated_at: u64,
    last_verified_at: Option<u64>,
    verification: Option<Verification>,
    // Idempotency receipts, not an unbounded copy of every log.
    recorded_cases: BTreeMap<String, String>,
}
fn decode<T: serde::de::DeserializeOwned>(b: &[u8]) -> Result<T> {
    serde_json::from_slice(b).map_err(|e| format!("IssueFormatInvalid: {e}"))
}
fn digest<T: Serialize>(v: &T) -> String {
    hash(&serde_json::to_vec(v).unwrap())
}
fn now() -> Result<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|x| x.as_secs())
        .map_err(|e| e.to_string())
}
fn material(c: &Content) -> String {
    digest(
        &json!({"symptom":c.symptom,"applicability":c.applicability,"cause":c.cause,"resolution":c.resolution}),
    )
}
fn number(id: &str) -> Result<u64> {
    let n = id
        .strip_prefix("ISS-")
        .and_then(|x| x.parse::<u64>().ok())
        .filter(|n| *n > 0)
        .ok_or("IssueIdInvalid")?;
    if id != format!("ISS-{n:04}") {
        return Err("IssueIdInvalid".into());
    }
    Ok(n)
}
fn text(s: &str) -> bool {
    !s.trim().is_empty()
}
fn valid_content(c: &Content) -> Result<()> {
    if !text(&c.title)
        || !text(&c.symptom)
        || !text(&c.applicability.environment)
        || c.applicability.conditions.iter().any(|x| !text(x))
    {
        return Err("IssueContentIncomplete".into());
    }
    if c.tags.iter().any(|t| {
        t.is_empty()
            || t.len() > 64
            || !t
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    }) {
        return Err("IssueTagInvalid".into());
    }
    if c.related_specs.iter().any(|id| !crate::trace::valid_id(id)) {
        return Err("IssueSpecIdInvalid".into());
    }
    if c.cause.status == CauseStatus::Confirmed && !text(&c.cause.description) {
        return Err("IssueCauseIncomplete".into());
    }
    Ok(())
}
fn valid_issue(i: &Issue) -> Result<()> {
    number(&i.id)?;
    valid_content(&i.content)?;
    if i.occurrences != i.recorded_cases.len() as u64 || i.updated_at < i.created_at {
        return Err("IssueBookkeepingInvalid".into());
    }
    if let Some(v) = &i.verification {
        valid_observation(&v.observation)?;
        if !i.recorded_cases.contains_key(&v.observation.occurrence_key) {
            return Err("IssueVerificationInvalid".into());
        }
    }
    if matches!(i.content.status, Status::Mitigated | Status::Resolved) {
        let v = i.verification.as_ref().ok_or("IssueVerificationRequired")?;
        if v.observation.outcome != Outcome::Passed
            || v.content_sha256 != material(&i.content)
            || i.last_verified_at != Some(v.recorded_at)
            || !text(&i.content.resolution.description)
        {
            return Err("IssueVerificationStale".into());
        }
        if i.content.status == Status::Resolved && i.content.cause.status != CauseStatus::Confirmed
        {
            return Err("IssueConfirmedCauseRequired".into());
        }
    }
    Ok(())
}
fn valid_observation(v: &Observation) -> Result<()> {
    if v.occurrence_key.is_empty()
        || v.occurrence_key.len() > 128
        || !text(&v.action)
        || !text(&v.observed)
    {
        return Err("IssueObservationIncomplete".into());
    }
    for p in &v.evidence {
        paths::relative(p)?;
        let normalized = p.to_ascii_lowercase();
        if normalized == "issues"
            || normalized.starts_with("issues/")
            || normalized == ".highgrade"
            || normalized.starts_with(".highgrade/")
        {
            return Err("IssueEvidenceSelfReference".into());
        }
    }
    Ok(())
}
fn bytes(root: &Path, rel: &str) -> Result<Option<Vec<u8>>> {
    let p = paths::safe(root, rel)?;
    if p.exists() {
        Ok(Some(paths::read_limited(&p, LIMIT)?))
    } else {
        Ok(None)
    }
}
struct Lock(PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn lock(root: &Path) -> Result<Lock> {
    let dir = paths::safe(root, ".highgrade/issues")?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let p = dir.join("write.lock");
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&p)
        .map_err(|e| format!("IssueBusy: inspect lock owner before recovery: {e}"))?;
    let guard = Lock(p);
    write!(f, "{}", std::process::id()).map_err(|e| e.to_string())?;
    Ok(guard)
}
fn put(root: &Path, rel: &str, b: &[u8]) -> Result<()> {
    if b.len() as u64 > LIMIT {
        return Err("IssueDocumentTooLarge".into());
    }
    let p = paths::safe(root, rel)?;
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = paths::safe(
        root,
        &format!(".highgrade/issues/{}.part", hash(rel.as_bytes())),
    )?;
    fs::create_dir_all(tmp.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp)
        .map_err(|e| e.to_string())?;
    f.write_all(b)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    drop(f);
    paths::rename_atomic(&tmp, &p)
}
fn snapshot(root: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    let dir = paths::safe(root, "issues")?;
    if !dir.exists() {
        return Ok(files);
    }
    for e in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e
            .file_name()
            .into_string()
            .map_err(|_| "IssueForeignFile")?;
        if !matches!(
            name.as_str(),
            "catalog.json" | "entries" | "transaction.json"
        ) {
            return Err(format!("IssueForeignFile: {name}"));
        }
        paths::safe(root, &format!("issues/{name}"))?;
    }
    if let Some(b) = bytes(root, CATALOG)? {
        files.insert(CATALOG.into(), b);
    }
    let entries = paths::safe(root, "issues/entries")?;
    if entries.exists() {
        for e in fs::read_dir(entries).map_err(|e| e.to_string())? {
            let name = e
                .map_err(|e| e.to_string())?
                .file_name()
                .into_string()
                .map_err(|_| "IssueForeignFile")?;
            number(name.strip_suffix(".json").ok_or("IssueForeignFile")?)?;
            let rel = format!("issues/entries/{name}");
            files.insert(rel.clone(), bytes(root, &rel)?.ok_or("IssueFileMissing")?);
        }
    }
    Ok(files)
}
fn parse(files: &BTreeMap<String, Vec<u8>>) -> Result<(Catalog, BTreeMap<String, Issue>)> {
    let Some(header) = files.get(CATALOG) else {
        if !files.is_empty() {
            return Err("IssueForeignCatalog".into());
        }
        return Ok((
            Catalog {
                schema_version: 1,
                next_number: 1,
            },
            BTreeMap::new(),
        ));
    };
    let c: Catalog = decode(header)?;
    if c.schema_version != 1 {
        return Err("IssueVersionUnsupported".into());
    }
    if c.next_number == 0 {
        return Err("IssueCounterInvalid".into());
    }
    let mut items = BTreeMap::new();
    for (p, b) in files.iter().filter(|(p, _)| p.as_str() != CATALOG) {
        let i: Issue = decode(b)?;
        valid_issue(&i)?;
        if p != &format!("issues/entries/{}.json", i.id) || number(&i.id)? >= c.next_number {
            return Err("IssueCatalogInvalid".into());
        }
        items.insert(i.id.clone(), i);
    }
    Ok((c, items))
}
fn revision(files: &BTreeMap<String, Vec<u8>>) -> String {
    if files.is_empty() {
        "absent".into()
    } else {
        digest(
            &files
                .iter()
                .map(|(p, b)| (p, hash(b)))
                .collect::<BTreeMap<_, _>>(),
        )
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Replacement {
    before: Option<String>,
    after: String,
}
fn recover(root: &Path, expected: &str) -> Result<()> {
    let b = bytes(root, JOURNAL)?.ok_or("IssueJournalMissing")?;
    if hash(&b) != expected {
        return Err("IssueJournalConflict".into());
    }
    let tx: BTreeMap<String, Replacement> = decode(&b)?;
    if tx.len() != 2 || !tx.contains_key(CATALOG) {
        return Err("IssueJournalInvalid".into());
    }
    let mut files = snapshot(root)?;
    for (p, r) in &tx {
        if p != CATALOG {
            let id = p
                .strip_prefix("issues/entries/")
                .and_then(|x| x.strip_suffix(".json"))
                .ok_or("IssueJournalInvalid")?;
            number(id)?;
        }
        let current = files.get(p).map(|b| hash(b));
        let after = hash(r.after.as_bytes());
        if current != r.before && current.as_deref() != Some(&after) {
            return Err(format!("IssueRecoveryConflict: {p}"));
        }
        files.insert(p.clone(), r.after.as_bytes().to_vec());
    }
    parse(&files)?;
    for (p, r) in tx {
        put(root, &p, r.after.as_bytes())?;
    }
    fs::remove_file(paths::safe(root, JOURNAL)?).map_err(|e| e.to_string())
}
fn save(
    root: &Path,
    old: &BTreeMap<String, Vec<u8>>,
    catalog: &Catalog,
    issue: &Issue,
) -> Result<()> {
    let mut tx = BTreeMap::new();
    for (p, v) in [
        (CATALOG.to_string(), json!(catalog)),
        (format!("issues/entries/{}.json", issue.id), json!(issue)),
    ] {
        tx.insert(
            p.clone(),
            Replacement {
                before: old.get(&p).map(|b| hash(b)),
                after: crate::specs::pretty_value(&v).unwrap(),
            },
        );
    }
    let b = serde_json::to_vec_pretty(&tx).unwrap();
    put(root, JOURNAL, &b)?;
    recover(root, &hash(&b))
}
fn current_evidence(root: &Path, v: &Verification) -> bool {
    v.evidence_sha256.iter().all(|(p, h)| {
        bytes(root, p)
            .ok()
            .flatten()
            .is_some_and(|b| hash(&b) == *h)
    })
}
pub fn schemas() -> Value {
    let mut patch = serde_json::to_value(schemars::schema_for!(Content)).unwrap();
    patch.as_object_mut().unwrap().remove("required");
    json!({"patch":patch,"catalog":schemars::schema_for!(Catalog),"issue":schemars::schema_for!(Issue),"content":schemars::schema_for!(Content),"observation":schemars::schema_for!(Observation)})
}
pub fn command(root: &Path, op: &str, o: &BTreeMap<String, String>) -> Result<Report> {
    let get = |k: &str| {
        o.get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("Usage: required {k}"))
    };
    let mut report = Report::new(op);
    if op == "issue-schema" {
        report.measurements.push(schemas());
        return Ok(report);
    }
    if o.get("--view")
        .is_some_and(|v| !matches!(v.as_str(), "full" | "summary"))
    {
        return Err("Usage: --view summary|full".into());
    }
    let root = paths::root(root)?;
    let write = matches!(
        op,
        "issue-new" | "issue-edit" | "issue-record" | "issue-recover"
    );
    let _guard = if write || paths::safe(&root, "issues")?.exists() {
        Some(lock(&root)?)
    } else {
        None
    };
    if op == "issue-recover" {
        recover(&root, get("--expected")?)?;
        report.measurements.push(json!({"recovered":true}));
        return Ok(report);
    }
    if let Some(b) = bytes(&root, JOURNAL)? {
        return Err(format!(
            "IssueRecoveryRequired: issue-recover --expected {}",
            hash(&b)
        ));
    }
    let files = snapshot(&root)?;
    let sha = revision(&files);
    let (mut catalog, mut issues) = parse(&files)?;
    if write && get("--expected")? != sha {
        return Err(format!("IssueConflict: current_sha256={sha}"));
    }
    let mut changed = false;
    let mut selected = None;
    match op {
        "issue-list" => {
            if let Some(status) = o.get("--status") {
                let _: Status =
                    serde_json::from_value(json!(status)).map_err(|_| "IssueStatusInvalid")?;
            }
            let query = o.get("--query").map(|s| s.to_lowercase());
            let rows:Vec<_>=issues.values().filter(|i|{
   o.get("--tag").is_none_or(|tag|i.content.tags.contains(tag)) && o.get("--status").is_none_or(|s|json!(i.content.status)==json!(s)) && query.as_ref().is_none_or(|q|serde_json::to_string(&i.content).unwrap().to_lowercase().contains(q))
  }).map(|i|json!({"id":i.id,"title":i.content.title,"tags":i.content.tags,"status":i.content.status,"symptom":i.content.symptom,"occurrences":i.occurrences,"verification_current":i.verification.as_ref().is_some_and(|v|v.observation.outcome==Outcome::Passed&&v.content_sha256==material(&i.content)&&current_evidence(&root,v))})).collect();
            report
                .measurements
                .push(json!({"issues":rows,"total":issues.len()}));
        }
        "issue-new" => {
            let content: Content = decode(&paths::read_limited(
                &paths::safe(&root, get("--input")?)?,
                LIMIT,
            )?)?;
            if content.status != Status::Open {
                return Err("IssueNewMustBeOpen".into());
            }
            let id = format!("ISS-{:04}", catalog.next_number);
            catalog.next_number = catalog
                .next_number
                .checked_add(1)
                .ok_or("IssueCounterExhausted")?;
            let time = now()?;
            let issue = Issue {
                id: id.clone(),
                content,
                occurrences: 0,
                created_at: time,
                updated_at: time,
                last_verified_at: None,
                verification: None,
                recorded_cases: BTreeMap::new(),
            };
            valid_issue(&issue)?;
            issues.insert(id.clone(), issue);
            selected = Some(id);
            changed = true;
        }
        "issue-read" | "issue-validate" | "issue-edit" | "issue-record" => {
            let id = get("--id")?;
            number(id)?;
            let issue = issues.get_mut(id).ok_or("IssueMissing")?;
            selected = Some(id.to_string());
            if op == "issue-edit" {
                let patch: Value = decode(&paths::read_limited(
                    &paths::safe(&root, get("--input")?)?,
                    LIMIT,
                )?)?;
                let fields = patch.as_object().ok_or("IssuePatchInvalid")?;
                let mut value = json!(issue.content);
                let old = json!(issue);
                for (k, v) in fields {
                    if !value.as_object().unwrap().contains_key(k) {
                        return Err(format!("IssueProtectedField: {k}"));
                    }
                    value[k] = v.clone();
                }
                let content: Content =
                    serde_json::from_value(value).map_err(|e| format!("IssuePatchInvalid: {e}"))?;
                let material_changed = material(&content) != material(&issue.content);
                let was_verified =
                    matches!(issue.content.status, Status::Mitigated | Status::Resolved);
                issue.content = content;
                if material_changed {
                    issue.last_verified_at = None;
                    if was_verified
                        || matches!(issue.content.status, Status::Mitigated | Status::Resolved)
                    {
                        issue.content.status = Status::Stale;
                    }
                }
                valid_issue(issue)?;
                if matches!(issue.content.status, Status::Mitigated | Status::Resolved)
                    && !issue
                        .verification
                        .as_ref()
                        .is_some_and(|v| current_evidence(&root, v))
                {
                    return Err("IssueEvidenceStale".into());
                }
                changed = json!(issue) != old;
            }
            if op == "issue-record" {
                let obs: Observation = decode(&paths::read_limited(
                    &paths::safe(&root, get("--input")?)?,
                    LIMIT,
                )?)?;
                valid_observation(&obs)?;
                let evidence = obs
                    .evidence
                    .iter()
                    .map(|p| {
                        Ok((
                            p.clone(),
                            hash(&bytes(&root, p)?.ok_or("IssueEvidenceMissing")?),
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>>>()?;
                let receipt = digest(&json!({"observation":obs,"evidence":evidence}));
                if let Some(old) = issue.recorded_cases.get(&obs.occurrence_key) {
                    if old != &receipt {
                        return Err("IssueOccurrenceConflict".into());
                    }
                } else {
                    let time = now()?;
                    issue
                        .recorded_cases
                        .insert(obs.occurrence_key.clone(), receipt);
                    issue.occurrences = issue.recorded_cases.len() as u64;
                    issue.last_verified_at = if obs.outcome == Outcome::Passed {
                        Some(time)
                    } else {
                        None
                    };
                    if obs.outcome != Outcome::Passed
                        && matches!(issue.content.status, Status::Mitigated | Status::Resolved)
                    {
                        issue.content.status = Status::Stale;
                    }
                    issue.verification = Some(Verification {
                        observation: obs,
                        recorded_at: time,
                        content_sha256: material(&issue.content),
                        evidence_sha256: evidence,
                    });
                    changed = true;
                }
            }
        }
        _ => return Err("Usage: unknown issue command".into()),
    }
    if let Some(id) = &selected {
        let i = issues.get_mut(id).unwrap();
        valid_issue(i)?;
        if changed {
            i.updated_at = now()?;
            save(&root, &files, &catalog, i)?;
        }
        if i.verification
            .as_ref()
            .is_some_and(|v| !current_evidence(&root, v))
        {
            report.finding(
                "warning",
                "IssueEvidenceStale",
                id,
                "Recorded evidence changed or is unavailable; verify before reuse",
            );
        }
        let view = o.get("--view").map(String::as_str).unwrap_or("summary");
        let value = match view {
            "full" => json!(i),
            "summary" => {
                json!({"id":i.id,"content":i.content,"occurrences":i.occurrences,"last_verified_at":i.last_verified_at,"verification":i.verification})
            }
            _ => return Err("Usage: --view summary|full".into()),
        };
        report
            .measurements
            .push(json!({"issue":value,"changed":changed}));
    }
    report
        .measurements
        .push(json!({"store_sha256":if changed{revision(&snapshot(&root)?)}else{sha}}));
    report.limitations.push("Schema validation does not prove root cause, applicability or authenticity. Stored recipes are never executed.".into());
    Ok(report)
}
