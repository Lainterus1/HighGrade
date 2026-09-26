//! Project-owned files. A durable intent journal prevents mixed snapshots from
//! being reported as successful after an interrupted multi-file transaction.
use super::*;

pub const CATALOG: &str = "specs/catalog.json";
const JOURNAL: &str = "specs/transaction.json";
const README: &str = "# Спецификации проекта\n\nДанные принадлежат проекту и сохраняются при удалении High Grade.\n`catalog.json` — версия и счётчик; `requirements/` — действующие требования;\n`changes/<ID>/spec.json` — изменение, `results.json` — проверки и решения.\nЗаписывайте через инструмент с проверкой ожидаемого хеша. При наличии\n`transaction.json` чтение заблокировано до явного `spec-recover`.\n";

const LOCATION: &str = "specs-location.json";
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Location {
    directory: String,
}
pub fn directory(root: &Path) -> Result<String> {
    let p = paths::safe(root, LOCATION)?;
    let dir = if p.exists() {
        decode::<Location>(&paths::read_limited(&p, 4096)?, LOCATION)?.directory
    } else {
        "specs".into()
    };
    paths::relative(&dir)?;
    if dir.split('/').any(|p| p.starts_with('.')) {
        return Err("InvalidSpecsDirectory: project-owned visible directory required".into());
    }
    Ok(dir)
}
pub fn catalog_path(root: &Path) -> Result<String> {
    Ok(format!("{}/catalog.json", directory(root)?))
}
fn safe(root: &Path, rel: &str) -> Result<PathBuf> {
    let real = if rel == "specs" {
        directory(root)?
    } else if let Some(tail) = rel.strip_prefix("specs/") {
        format!("{}/{tail}", directory(root)?)
    } else {
        rel.into()
    };
    paths::safe(root, &real)
}
pub fn init(root: &Path, dir: &str) -> Result<()> {
    paths::relative(dir)?;
    if dir.split('/').any(|p| p.starts_with('.')) {
        return Err("InvalidSpecsDirectory".into());
    }
    if exists(root)? || paths::safe(root, STORE)?.exists() || paths::safe(root, LOCATION)?.exists()
    {
        return Err("CatalogAlreadyConfigured: migrate existing data explicitly".into());
    }
    let p = paths::safe(root, dir)?;
    if p.exists()
        && fs::read_dir(&p)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err("ForeignCatalog".into());
    }
    if dir != "specs" {
        put(
            root,
            LOCATION,
            &serde_json::to_vec_pretty(&Location {
                directory: dir.into(),
            })
            .unwrap(),
        )?;
    }
    write(
        root,
        &Store {
            schema_version: 3,
            ..Store::default()
        },
    )
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Header {
    schema_version: u32,
    next_number: u64,
    retired_ids: BTreeSet<String>,
    origins: BTreeMap<String, Origin>,
    legacy_sha256: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Entry {
    before: Option<String>,
    after: Option<String>,
}
type Journal = BTreeMap<String, Entry>;

pub fn exists(root: &Path) -> Result<bool> {
    Ok(safe(root, CATALOG)?.exists() || safe(root, JOURNAL)?.exists())
}
fn bytes(root: &Path, rel: &str) -> Result<Option<Vec<u8>>> {
    let p = safe(root, rel)?;
    if p.exists() {
        Ok(Some(paths::read_limited(&p, LIMIT)?))
    } else {
        Ok(None)
    }
}
fn put(root: &Path, rel: &str, bytes: &[u8]) -> Result<()> {
    let p = safe(root, rel)?;
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    // One reserved staging file per target, outside the document directories.
    // A retry reconstructs partial staging bytes from the durable journal.
    let temp = paths::safe(
        root,
        &format!(".highgrade/specs/staging/{}.part", hash(rel.as_bytes())),
    )?;
    fs::create_dir_all(temp.parent().unwrap()).map_err(|e| e.to_string())?;
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    drop(f);
    rename_atomic(&temp, &p)
}
#[cfg(not(windows))]
fn rename_atomic(from: &Path, to: &Path) -> Result<()> {
    fs::rename(from, to).map_err(|e| e.to_string())
}
#[cfg(windows)]
fn rename_atomic(from: &Path, to: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }
    let wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let (a, b) = (wide(from), wide(to));
    // MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH; same project volume.
    if unsafe { MoveFileExW(a.as_ptr(), b.as_ptr(), 1 | 8) } == 0 {
        return Err(format!(
            "CatalogReplaceFailed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}
fn entries(root: &Path, rel: &str) -> Result<Vec<String>> {
    let p = safe(root, rel)?;
    if !p.exists() {
        return Ok(vec![]);
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(p).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "InvalidCatalogName")?;
        safe(root, &format!("{rel}/{name}"))?;
        names.push(name);
    }
    names.sort();
    Ok(names)
}
fn snapshot(root: &Path, partial: bool) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    if let Some(b) = bytes(root, CATALOG)? {
        files.insert(CATALOG.into(), b);
    }
    if let Some(b) = bytes(root, "specs/runners.json")? {
        files.insert("specs/runners.json".into(), b);
    }
    if let Some(b) = bytes(root, "specs/tags.json")? {
        files.insert("specs/tags.json".into(), b);
    }
    for name in entries(root, "specs/requirements")? {
        let id = name
            .strip_suffix(".json")
            .ok_or("UnexpectedRequirementFile")?;
        check_id(id, "requirement filename")?;
        let rel = format!("specs/requirements/{name}");
        files.insert(rel.clone(), bytes(root, &rel)?.ok_or("RequirementMissing")?);
    }
    for id in entries(root, "specs/changes")? {
        check_id(&id, "change directory")?;
        for name in entries(root, &format!("specs/changes/{id}"))? {
            if !matches!(name.as_str(), "spec.json" | "results.json" | "evidence") {
                return Err(format!("UnexpectedChangeFile: {id}/{name}"));
            }
        }
        for name in ["spec.json", "results.json"] {
            let rel = format!("specs/changes/{id}/{name}");
            if let Some(b) = bytes(root, &rel)? {
                files.insert(rel, b);
            } else if name == "spec.json" && !partial {
                return Err(format!("ChangeMissing: {id}"));
            }
        }
    }
    Ok(files)
}
const RESULTS: &[&str] = &["evidence", "review", "acceptance", "history", "runs"];
pub fn load(root: &Path) -> Result<(Store, String)> {
    if let Some(b) = bytes(root, JOURNAL)? {
        return Err(format!(
            "CatalogRecoveryRequired: spec-recover --expected {}",
            hash(&b)
        ));
    }
    let files = snapshot(root, false)?;
    decode_snapshot(root, &files)
}
fn decode_snapshot(root: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(Store, String)> {
    let h: Header = decode(files.get(CATALOG).ok_or("CatalogMissing")?, CATALOG)?;
    if !matches!(h.schema_version, 3 | 4) {
        return Err("UnsupportedVersion: catalog".into());
    }
    if let Some(legacy) = bytes(root, STORE)? {
        if h.legacy_sha256.as_deref() != Some(hash(&legacy).as_str()) {
            return Err("LegacyStoreConflict: legacy store changed after migration".into());
        }
    }
    let mut s = Store {
        schema_version: 3,
        next_number: h.next_number,
        retired_ids: h.retired_ids,
        origins: h.origins,
        ..Store::default()
    };
    if let Some(b) = files.get("specs/runners.json") {
        s.runners = decode(b, "runners")?;
    }
    if let Some(b) = files.get("specs/tags.json") {
        s.tags = decode(b, "tags")?;
    }
    for (rel, b) in files {
        if rel.starts_with("specs/requirements/") {
            let r: Requirement = decode(b, rel)?;
            if rel != &format!("specs/requirements/{}.json", r.id) {
                return Err("IdMismatch: requirement filename".into());
            }
            s.requirements.insert(r.id.clone(), r);
        } else if rel.ends_with("/spec.json") {
            let mut v: Value = decode(b, rel)?;
            if RESULTS.iter().any(|f| v.get(*f).is_some()) {
                return Err("ProtectedField: results in spec.json".into());
            }
            let result_path = rel.replace("/spec.json", "/results.json");
            let mut results = if let Some(b) = files.get(&result_path) {
                decode::<Value>(b, &result_path)?
            } else {
                json!({"evidence":{},"review":null,"acceptance":[],"history":[],"runs":[]})
            };
            let obj = results.as_object_mut().ok_or("InvalidResults")?;
            if h.schema_version == 4 && files.contains_key(&result_path) {
                let history = obj.get_mut("history").ok_or("MissingResultsField")?;
                *history = expand_history(history)?;
            }
            if obj.keys().any(|k| !RESULTS.contains(&k.as_str())) {
                return Err("UnknownResultsField".into());
            }
            for f in RESULTS {
                v[*f] = obj.remove(*f).ok_or("MissingResultsField")?;
            }
            let c: Change = decode(&serde_json::to_vec(&v).unwrap(), rel)?;
            if rel != &format!("specs/changes/{}/spec.json", c.id) {
                return Err("IdMismatch: change filename".into());
            }
            s.changes.insert(c.id.clone(), c);
        }
    }
    integrity(&s)?;
    let hashes: BTreeMap<_, _> = files.iter().map(|(p, b)| (p, hash(b))).collect();
    Ok((s, digest(&(directory(root)?, hashes))))
}
fn allowed(rel: &str) -> bool {
    if rel == CATALOG || rel == "specs/tags.json" || rel == "specs/runners.json" {
        return true;
    }
    if let Some(tail) = rel.strip_prefix("specs/requirements/") {
        return tail.strip_suffix(".json").is_some_and(id);
    }
    if let Some((key, file)) = rel
        .strip_prefix("specs/changes/")
        .and_then(|s| s.split_once('/'))
    {
        return id(key) && matches!(file, "spec.json" | "results.json");
    }
    false
}
pub fn recover(root: &Path, expected: &str) -> Result<()> {
    let b = bytes(root, JOURNAL)?.ok_or("NoCatalogTransaction")?;
    if hash(&b) != expected {
        return Err("StoreConflict: transaction".into());
    }
    let journal: Journal = decode(&b, JOURNAL)?;
    if journal.is_empty() {
        return Err("EmptyCatalogTransaction".into());
    }
    // Validate every target before changing any. Never execute arbitrary paths
    // from a repository-supplied journal, even with the caller's observed hash.
    for (rel, entry) in &journal {
        if entry.after.is_none()
            && (rel == CATALOG
                || rel == "specs/tags.json"
                || rel == "specs/runners.json"
                || rel.ends_with("/spec.json"))
        {
            return Err("InvalidCatalogDeletion: required document".into());
        }
        if !allowed(rel) {
            return Err("UnsafeTransactionTarget".into());
        }
        let current = bytes(root, rel)?.map(|b| hash(&b));
        let after = entry.after.as_ref().map(|s| hash(s.as_bytes()));
        if current != entry.before && current != after {
            return Err(format!("RecoveryConflict: {rel}"));
        }
        if entry.after.as_ref().is_some_and(|s| s.len() as u64 > LIMIT) {
            return Err("CatalogFileTooLarge".into());
        }
    }
    let mut prospective = snapshot(root, true)?;
    for (rel, entry) in &journal {
        if let Some(s) = &entry.after {
            prospective.insert(rel.clone(), s.as_bytes().to_vec());
        } else {
            prospective.remove(rel);
        }
    }
    decode_snapshot(root, &prospective)?;
    for (rel, entry) in journal {
        match entry.after {
            Some(s) => {
                if bytes(root, &rel)?.as_deref() != Some(s.as_bytes()) {
                    put(root, &rel, s.as_bytes())?;
                }
            }
            None => {
                if safe(root, &rel)?.exists() {
                    fs::remove_file(safe(root, &rel)?).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    // A malformed edited journal must not produce a successful readable snapshot.
    let journal_path = safe(root, JOURNAL)?;
    fs::remove_file(journal_path).map_err(|e| e.to_string())?;
    load(root)?;
    Ok(())
}
pub fn write(root: &Path, s: &Store) -> Result<()> {
    write_format(root, s, None)
}

pub fn compact(root: &Path, s: &Store) -> Result<()> {
    if !exists(root)? {
        return Err("MigrationRequired: use --to directory first".into());
    }
    write_format(root, s, Some(4))
}

fn write_format(root: &Path, s: &Store, format: Option<u32>) -> Result<()> {
    if bytes(root, JOURNAL)?.is_some() {
        return Err("CatalogRecoveryRequired".into());
    }
    if bytes(root, CATALOG)?.is_none() {
        let dir = safe(root, "specs")?;
        if dir.exists()
            && fs::read_dir(&dir)
                .map_err(|e| e.to_string())?
                .next()
                .is_some()
        {
            return Err("ForeignCatalog: choose an empty project specification directory".into());
        }
    }
    let old = snapshot(root, false)?;
    let legacy_sha256 = if let Some(h) = old.get(CATALOG) {
        decode::<Header>(h, CATALOG)?.legacy_sha256
    } else {
        bytes(root, STORE)?.map(|b| hash(&b))
    };
    let current_format = old
        .get(CATALOG)
        .map(|b| decode::<Header>(b, CATALOG))
        .transpose()?
        .map_or(3, |h| h.schema_version);
    let h = Header {
        schema_version: format.unwrap_or(current_format),
        next_number: s.next_number,
        retired_ids: s.retired_ids.clone(),
        origins: s.origins.clone(),
        legacy_sha256,
    };
    let mut new = BTreeMap::new();
    new.insert(CATALOG.into(), serde_json::to_vec_pretty(&h).unwrap());
    new.insert(
        "specs/tags.json".into(),
        serde_json::to_vec_pretty(&s.tags).unwrap(),
    );
    new.insert(
        "specs/runners.json".into(),
        serde_json::to_vec_pretty(&s.runners).unwrap(),
    );
    for r in s.requirements.values() {
        new.insert(
            format!("specs/requirements/{}.json", r.id),
            serde_json::to_vec_pretty(r).unwrap(),
        );
    }
    for c in s.changes.values() {
        let mut v = serde_json::to_value(c).unwrap();
        for field in ["depends_on", "related_to", "tags", "checks"] {
            if v.get(field).is_none() {
                v[field] = json!([]);
            }
        }
        let mut results = serde_json::Map::new();
        for f in RESULTS {
            results.insert(
                (*f).into(),
                v.as_object_mut()
                    .unwrap()
                    .remove(*f)
                    .unwrap_or_else(|| json!([])),
            );
        }
        if h.schema_version == 4 {
            results.insert("history".into(), compact_history(&results["history"])?);
        }
        new.insert(
            format!("specs/changes/{}/spec.json", c.id),
            pretty_value(&v).unwrap().into_bytes(),
        );
        if !c.evidence.is_empty()
            || c.review.is_some()
            || !c.acceptance.is_empty()
            || !c.history.is_empty()
            || !c.runs.is_empty()
        {
            new.insert(
                format!("specs/changes/{}/results.json", c.id),
                serde_json::to_vec_pretty(&results).unwrap(),
            );
        }
    }
    let mut journal = Journal::new();
    for rel in old.keys().chain(new.keys()).collect::<BTreeSet<_>>() {
        if old.get(rel) == new.get(rel) {
            continue;
        }
        if new.get(rel).is_some_and(|b| b.len() as u64 > LIMIT) {
            return Err("CatalogFileTooLarge".into());
        }
        journal.insert(
            rel.clone(),
            Entry {
                before: old.get(rel).map(|b| hash(b)),
                after: new.get(rel).map(|b| String::from_utf8(b.clone()).unwrap()),
            },
        );
    }
    if journal.is_empty() {
        return Ok(());
    }
    let b = serde_json::to_vec_pretty(&journal).unwrap();
    if b.len() as u64 > LIMIT {
        return Err("TransactionTooLarge: split this operation".into());
    }
    put(root, JOURNAL, &b)?;
    recover(root, &hash(&b))?;
    if bytes(root, "specs/README.md")?.is_none() {
        put(root, "specs/README.md", README.as_bytes())?;
    }
    Ok(())
}

pub fn migrate(root: &Path, s: &mut Store, sha: &str) -> Result<()> {
    if s.schema_version == 3 {
        return Ok(());
    }
    // Preserve exact legacy bytes outside service configuration.
    let legacy = bytes(root, STORE)?.ok_or("LegacyStoreMissing")?;
    if hash(&legacy) != sha {
        return Err("StoreConflict: migration".into());
    }
    s.schema_version = 3;
    // Backup before switching authority. A sibling file avoids adopting a
    // partially populated destination if the first transaction is interrupted.
    let backup = format!("specs-backup-{sha}.json");
    if let Some(b) = bytes(root, &backup)? {
        if b != legacy {
            return Err("BackupConflict".into());
        }
    } else {
        put(root, &backup, &legacy)?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CompactHistory {
    objects: BTreeMap<String, Value>,
    entries: Vec<HistoryEntry>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HistoryEntry {
    recorded_at: u64,
    evidence: BTreeMap<String, String>,
    review: String,
}
fn compact_history(value: &Value) -> Result<Value> {
    let snapshots: Vec<ResultSnapshot> =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    let mut result = CompactHistory {
        objects: BTreeMap::new(),
        entries: Vec::new(),
    };
    for snapshot in snapshots {
        let mut intern = |v: Value| {
            let key = digest(&v);
            result.objects.entry(key.clone()).or_insert(v);
            key
        };
        let evidence = snapshot
            .evidence
            .into_iter()
            .map(|(id, e)| (id, intern(json!(e))))
            .collect();
        let review = intern(json!(snapshot.review));
        result.entries.push(HistoryEntry {
            recorded_at: snapshot.recorded_at,
            evidence,
            review,
        });
    }
    Ok(json!(result))
}
fn expand_history(value: &Value) -> Result<Value> {
    let compact: CompactHistory =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    for (key, v) in &compact.objects {
        if digest(v) != *key {
            return Err("HistoryObjectHashMismatch".into());
        }
    }
    let mut history = Vec::new();
    for entry in compact.entries {
        let mut evidence = serde_json::Map::new();
        for (id, key) in entry.evidence {
            evidence.insert(
                id,
                compact
                    .objects
                    .get(&key)
                    .ok_or("HistoryObjectMissing")?
                    .clone(),
            );
        }
        let review = compact
            .objects
            .get(&entry.review)
            .ok_or("HistoryObjectMissing")?;
        history.push(json!({"recorded_at":entry.recorded_at,"evidence":evidence,"review":review}));
    }
    Ok(json!(history))
}

pub fn schema() -> Value {
    let mut spec = serde_json::to_value(schemars::schema_for!(Change)).unwrap();
    let mut results =
        json!({"type":"object","additionalProperties":false,"properties":{},"required":RESULTS});
    results["$defs"] = spec["$defs"].clone();
    for field in RESULTS {
        if let Some(property) = spec["properties"].as_object_mut().unwrap().remove(*field) {
            results["properties"][*field] = property;
        }
    }
    spec["required"]
        .as_array_mut()
        .unwrap()
        .retain(|v| !RESULTS.contains(&v.as_str().unwrap()));
    json!({"catalog":schemars::schema_for!(Header),"spec":spec,"results":results,"compact_history":schemars::schema_for!(CompactHistory),"versions":[3,4],"tags":schemars::schema_for!(BTreeMap<String,relations::Tag>),"runners":schemars::schema_for!(BTreeMap<String,checks::Runner>)})
}
