use crate::{Report, Result, hash, paths, read_json};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
};

const JOURNAL: &str = ".highgrade/install.json";
const COMPLETE: &str = ".highgrade/installed.json";
const ENTRY: &str = ".agents/skills/highgrade-init/SKILL.md";
pub const EXCLUSION_CONFIG: &str = ".highgrade/project/inventory-exclusions.json";

pub fn exclusions(root: &Path) -> Result<(BTreeSet<String>, Option<String>)> {
    let path = paths::safe(root, EXCLUSION_CONFIG)?;
    if !path.exists() {
        return Ok((BTreeSet::new(), None));
    }
    let bytes = paths::read_limited(&path, 8 * 1024 * 1024)?;
    let config: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if config["schema_version"] != 1 {
        return Err("InventoryExclusionsInvalid: schema_version".into());
    }
    let entries = config["entries"]
        .as_array()
        .ok_or("InventoryExclusionsInvalid: entries")?;
    if entries.len() > 128 {
        return Err("InventoryExclusionsInvalid: too many entries".into());
    }
    let mut excluded = BTreeSet::new();
    for entry in entries {
        let rel = entry["path"]
            .as_str()
            .ok_or("InventoryExclusionsInvalid: path")?;
        let reason = entry["reason"]
            .as_str()
            .ok_or("InventoryExclusionsInvalid: reason")?;
        paths::relative(rel)?;
        if rel == ".highgrade"
            || rel.starts_with(".highgrade/")
            || reason.trim().is_empty()
            || !excluded.insert(rel.to_owned())
        {
            return Err(format!("InventoryExclusionsInvalid: {rel}"));
        }
    }
    Ok((excluded, Some(hash(&bytes))))
}

pub fn verify_audit_exclusions(root: &Path, audit: &Value) -> Result<()> {
    let (_, config_hash) = exclusions(root)?;
    if let Some(expected) = config_hash {
        if audit["exclusions_sha256"].as_str() != Some(expected.as_str()) {
            return Err("AuditExclusionsStale".into());
        }
    } else if !audit["exclusions_sha256"].is_null() {
        return Err("AuditExclusionsStale".into());
    }
    Ok(())
}

/// Inventory is structural evidence, not an automated semantic audit.
pub fn inventory(root: &Path, audit_rel: &str) -> Result<BTreeMap<String, String>> {
    fn visit(
        root: &Path,
        rel: &str,
        audit: &str,
        exclusions: &BTreeSet<String>,
        out: &mut BTreeMap<String, String>,
    ) -> Result<()> {
        let dir = if rel.is_empty() {
            root.to_path_buf()
        } else {
            paths::safe(root, rel)?
        };
        for item in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let name = item
                .file_name()
                .into_string()
                .map_err(|_| "InvalidUnicodePath")?;
            let child = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if exclusions.contains(&child)
                || child == audit
                || [
                    "highgrade-init",
                    "highgrade-work",
                    "highgrade-refresh",
                    "highgrade-update",
                ]
                .iter()
                .any(|s| child == format!(".agents/skills/{s}"))
                || (rel.is_empty() && [".git", ".highgrade"].contains(&name.as_str()))
                || (["target", "node_modules", ".venv", ".git"].contains(&name.as_str())
                    && item.file_type().map_err(|e| e.to_string())?.is_dir())
            {
                continue;
            }
            let path = paths::safe(root, &child)?;
            if path.is_dir() {
                visit(root, &child, audit, exclusions, out)?;
            } else {
                if out.len() >= 10000 {
                    return Err("InventoryTooLarge: limit=10000 files".into());
                }
                out.insert(child, hash(&paths::read_limited(&path, 32 * 1024 * 1024)?));
            }
        }
        Ok(())
    }
    let (exclusions, _) = exclusions(root)?;
    let mut out = BTreeMap::new();
    visit(root, "", audit_rel, &exclusions, &mut out)?;
    Ok(out)
}

pub(crate) fn put_once(root: &Path, rel: &str, data: &[u8]) -> Result<()> {
    let dest = paths::safe(root, rel)?;
    if dest.exists() {
        if paths::read_limited(&dest, 128 * 1024 * 1024)? == data {
            return Ok(());
        }
        return Err(format!("DestinationConflict: {rel}"));
    }
    fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    paths::safe(root, rel)?;
    let temp_rel = format!("{rel}.part");
    let temp = paths::safe(root, &temp_rel)?;
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&temp) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let previous = paths::read_limited(&temp, 128 * 1024 * 1024)?;
            if previous != data {
                return Err(format!(
                    "PartialWriteConflict: {temp_rel}; сохранён для ручного разбора"
                ));
            }
            OpenOptions::new()
                .write(true)
                .open(&temp)
                .map_err(|e| e.to_string())?
        }
        Err(e) => return Err(e.to_string()),
    };
    file.write_all(data).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    // A hard-link is an atomic no-clobber publication, unlike Unix rename.
    paths::safe(root, rel)?;
    fs::hard_link(&temp, &dest).map_err(|e| format!("PublishFailed: {rel}: {e}"))?;
    fs::remove_file(&temp).map_err(|e| format!("TempCleanupFailed: {e}"))?;
    Ok(())
}

pub fn verify(root: &Path) -> Result<()> {
    let journal_bytes = paths::read_limited(&paths::safe(root, JOURNAL)?, 8 * 1024 * 1024)?;
    let journal: serde_json::Value =
        serde_json::from_slice(&journal_bytes).map_err(|e| e.to_string())?;
    let complete = read_json(&paths::safe(root, COMPLETE)?)?;
    if complete["journal_sha256"] != hash(&journal_bytes)
        || complete["status"] != "prototype-connected"
    {
        return Err("InstallationMarkerInvalid".into());
    }
    let entries = journal["files"].as_object().ok_or("JournalInvalid")?;
    if entries.is_empty() {
        return Err("JournalEmpty".into());
    }
    for (rel, expected) in entries {
        if expected.as_str()
            != Some(&hash(&paths::read_limited(
                &paths::safe(root, rel)?,
                128 * 1024 * 1024,
            )?))
        {
            return Err(format!("InstalledFileChanged: {rel}"));
        }
    }
    Ok(())
}

/// stop_after is a controlled crash boundary for integration tests; no CLI option exposes it.
pub fn install(
    root: &Path,
    source: &Path,
    audit_rel: &str,
    stop_after: Option<usize>,
) -> Result<Report> {
    let root = paths::root(root)?;
    let source = paths::root(source)?;
    let audit = read_json(&paths::safe(&root, audit_rel)?)?;
    if audit["schema_version"] != 1
        || audit["status"] != "approved"
        || audit["scope"] != "full-project"
        || audit["evidence"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err(
            "AuditRequired: требуется явное подтверждение полного аудита и согласования".into(),
        );
    }
    verify_audit_exclusions(&root, &audit)?;
    let expected: BTreeMap<String, String> =
        serde_json::from_value(audit["files"].clone()).map_err(|_| "AuditInvalid: files")?;
    if inventory(&root, audit_rel)? != expected {
        return Err("AuditStale: состав или состояние проекта изменились".into());
    }
    let manifest = read_json(&paths::safe(&source, "manifest.json")?)?;
    if manifest["schema_version"] != 1 {
        return Err("ManifestVersionUnsupported".into());
    }
    let release = manifest["release"]
        .as_str()
        .ok_or("ManifestInvalid: release")?;
    if release.is_empty()
        || release.len() > 64
        || !release
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("ReleaseIdInvalid".into());
    }
    let source_files = manifest["files"]
        .as_object()
        .ok_or("ManifestInvalid: files")?;
    if source_files.len() != 2
        || !source_files.contains_key("SKILL.md")
        || !source_files.contains_key("rules.md")
    {
        return Err("ManifestFilesInvalid".into());
    }
    let mut contents = BTreeMap::new();
    for name in ["SKILL.md", "rules.md"] {
        let data = paths::read_limited(&paths::safe(&source, name)?, 1024 * 1024)?;
        if source_files[name].as_str() != Some(&hash(&data)) {
            return Err(format!("ManifestHashMismatch: {name}"));
        }
        let text = String::from_utf8(data).map_err(|_| "BundleNotUtf8")?;
        contents.insert(
            if name == "SKILL.md" {
                ENTRY.to_owned()
            } else {
                format!(".highgrade/releases/{release}/rules.md")
            },
            text.replace("{{release}}", release).into_bytes(),
        );
    }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    contents.insert(
        format!(
            ".highgrade/releases/{release}/highgrade{}",
            std::env::consts::EXE_SUFFIX
        ),
        paths::read_limited(&executable, 128 * 1024 * 1024)?,
    );
    let files: BTreeMap<String, String> =
        contents.iter().map(|(p, b)| (p.clone(), hash(b))).collect();
    let journal=serde_json::to_vec_pretty(&json!({"schema_version":1,"release":release,"files":files,"audit_sha256":hash(&serde_json::to_vec(&audit).unwrap())})).unwrap();
    let journal_path = paths::safe(&root, JOURNAL)?;
    if journal_path.exists() {
        if paths::read_limited(&journal_path, 8 * 1024 * 1024)? != journal {
            return Err("DifferentInstallation: update относится к P5; изменённый кандидат не продолжает старую операцию".into());
        }
    } else {
        for rel in contents.keys().chain([COMPLETE.to_owned()].iter()) {
            if paths::safe(&root, rel)?.exists() {
                return Err(format!("DestinationConflict: {rel}"));
            }
        }
    }
    let lock_path = paths::safe(&root, ".highgrade/install.lock")?;
    fs::create_dir_all(lock_path.parent().unwrap()).map_err(|e| e.to_string())?;
    paths::safe(&root, ".highgrade/install.lock")?;
    let lock = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|e| e.to_string())?;
    lock.try_lock()
        .map_err(|e| format!("InstallationBusy: {e}"))?;
    // Revalidate under our writer lock; external editors remain outside this protocol.
    if inventory(&root, audit_rel)? != expected {
        return Err("AuditStale: concurrent change".into());
    }
    put_once(&root, JOURNAL, &journal)?;
    if stop_after == Some(0) {
        return Err("TestInterruption".into());
    }
    for (index, (rel, data)) in contents.iter().enumerate() {
        put_once(&root, rel, data)?;
        if stop_after == Some(index + 1) {
            return Err("TestInterruption".into());
        }
    }
    if inventory(&root, audit_rel)? != expected {
        return Err("AuditStale: change during installation; connection remains incomplete".into());
    }
    let marker=serde_json::to_vec_pretty(&json!({"status":"prototype-connected","workflow_ready":false,"release":release,"journal_sha256":hash(&journal)})).unwrap();
    put_once(&root, COMPLETE, &marker)?;
    verify(&root)?;
    let mut r = Report::new("install");
    r.measurements
        .push(json!({"release":release,"connection":"prototype-connected","workflow_ready":false}));
    r.limitations.push("Подтверждение аудита проверено структурно и по снимку, не семантически. Подключён прототип P1; полная процедура init — P3, приёмка адаптации не выполнена.".into());
    Ok(r)
}
