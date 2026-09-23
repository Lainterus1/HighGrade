use crate::{Report, Result, hash, install, package, paths};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    path::Path,
};

const ACTIVE: &str = ".highgrade/global/active.json";
const LOCK: &str = ".highgrade/global/install.lock";
const SKILLS: [&str; 6] = [
    "highgrade-init",
    "highgrade-task",
    "highgrade-spec",
    "highgrade-work",
    "highgrade-clear",
    "highgrade-update",
];
const MATERIALS: [&str; 16] = [
    "rules.md",
    "procedures/init.md",
    "procedures/task.md",
    "procedures/spec.md",
    "procedures/work.md",
    "procedures/clear.md",
    "procedures/update.md",
    "procedures/archive.md",
    "references/audit.md",
    "references/cli.md",
    "templates/README.md",
    "templates/AGENTS.md",
    "templates/ARCHITECTURE.md",
    "templates/ENGINEERING.md",
    "templates/DEVELOPMENT.md",
    "templates/INSTRUCTIONS.md",
];

struct Candidate {
    release: String,
    cli_version: String,
    files: BTreeMap<String, Vec<u8>>,
    journal: Vec<u8>,
    manifest_hash: String,
    candidate_sha256: String,
}
fn router(name: &str) -> String {
    format!(".agents/skills/{name}/SKILL.md")
}
fn release_file(release: &str, rel: &str) -> String {
    format!(".highgrade/global/releases/{release}/{rel}")
}
fn journal_file(release: &str) -> String {
    release_file(release, "journal.json")
}
fn valid_release(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn candidate(source: &Path, executable: &Path) -> Result<Candidate> {
    let source = paths::root(source)?;
    let manifest_bytes =
        paths::read_limited(&paths::safe(&source, "manifest.json")?, 8 * 1024 * 1024)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 3 {
        return Err("GlobalManifestVersionUnsupported".into());
    }
    let release = manifest["release"]
        .as_str()
        .ok_or("GlobalManifestInvalid: release")?;
    if !valid_release(release) {
        return Err("GlobalReleaseIdInvalid".into());
    }
    let version = manifest["cli_version"]
        .as_str()
        .ok_or("GlobalManifestInvalid: cli_version")?;
    if version.is_empty() || version.len() > 32 {
        return Err("GlobalManifestInvalid: cli_version".into());
    }
    package::verify_binary(executable, version)?;
    let hashes = manifest["files"]
        .as_object()
        .ok_or("GlobalManifestInvalid: files")?;
    if hashes.len() != SKILLS.len() + MATERIALS.len() {
        return Err("GlobalManifestFilesInvalid".into());
    }
    let mut files = BTreeMap::new();
    for rel in MATERIALS
        .iter()
        .map(|s| s.to_string())
        .chain(SKILLS.iter().map(|s| format!("skills/{s}/SKILL.md")))
    {
        let data = paths::read_limited(&paths::safe(&source, &rel)?, 8 * 1024 * 1024)?;
        if hashes.get(&rel).and_then(Value::as_str) != Some(&hash(&data)) {
            return Err(format!("GlobalManifestHashMismatch: {rel}"));
        }
        let target = if let Some(name) = rel
            .strip_prefix("skills/")
            .and_then(|s| s.strip_suffix("/SKILL.md"))
        {
            router(name)
        } else {
            release_file(release, &rel)
        };
        files.insert(target, data);
    }
    let executable_bytes = paths::read_limited(executable, 128 * 1024 * 1024)?;
    let manifest_hash = hash(&manifest_bytes);
    let candidate_sha256 =
        hash(format!("{}:{}", manifest_hash, hash(&executable_bytes)).as_bytes());
    files.insert(
        release_file(
            release,
            &format!("highgrade{}", std::env::consts::EXE_SUFFIX),
        ),
        executable_bytes,
    );
    let checksums: BTreeMap<String, String> = files
        .iter()
        .map(|(p, data)| (p.clone(), hash(data)))
        .collect();
    let journal = serde_json::to_vec_pretty(&json!({"schema_version":3,"release":release,"manifest_sha256":hash(&manifest_bytes),"files":checksums})).map_err(|e| e.to_string())?;
    Ok(Candidate {
        release: release.into(),
        cli_version: version.into(),
        files,
        journal,
        manifest_hash,
        candidate_sha256,
    })
}
fn verify_release(profile: &Path, release: &str) -> Result<Vec<u8>> {
    let bytes = paths::read_limited(
        &paths::safe(profile, &journal_file(release))?,
        8 * 1024 * 1024,
    )?;
    let journal: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if journal["schema_version"] != 3 || journal["release"] != release {
        return Err("GlobalJournalInvalid".into());
    }
    let files = journal["files"]
        .as_object()
        .ok_or("GlobalJournalInvalid: files")?;
    if files.len() != SKILLS.len() + MATERIALS.len() + 1 {
        return Err("GlobalJournalInvalid: count".into());
    }
    for (rel, expected) in files {
        if !rel.starts_with(&format!(".highgrade/global/releases/{release}/"))
            && !SKILLS.iter().any(|s| rel == &router(s))
        {
            return Err("GlobalJournalInvalid: path".into());
        }
        let expected = expected.as_str().ok_or("GlobalJournalInvalid: hash")?;
        if hash(&paths::read_limited(
            &paths::safe(profile, rel)?,
            128 * 1024 * 1024,
        )?) != expected
        {
            return Err(format!("GlobalInstalledFileChanged: {rel}"));
        }
    }
    Ok(bytes)
}
fn active(profile: &Path) -> Result<Option<(String, String)>> {
    let p = paths::safe(profile, ACTIVE)?;
    if !p.exists() {
        return Ok(None);
    }
    let value = crate::read_json(&p)?;
    let release = value["release"]
        .as_str()
        .ok_or("GlobalActiveInvalid: release")?;
    if value["schema_version"] != 3 || !valid_release(release) {
        return Err("GlobalActiveInvalid".into());
    }
    let expected = value["journal_sha256"]
        .as_str()
        .ok_or("GlobalActiveInvalid: hash")?;
    let journal = verify_release(profile, release)?;
    if hash(&journal) != expected {
        return Err("GlobalActiveInvalid: journal changed".into());
    }
    Ok(Some((release.into(), expected.into())))
}
fn active_bytes(release: &str, journal: &[u8]) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({"schema_version":3,"status":"connected","release":release,"journal_sha256":hash(journal)})).unwrap()
}
fn lock(profile: &Path) -> Result<File> {
    let p = paths::safe(profile, LOCK)?;
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let p = paths::safe(profile, LOCK)?;
    let f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(p)
        .map_err(|e| e.to_string())?;
    f.try_lock()
        .map_err(|e| format!("GlobalInstallationBusy: {e}"))?;
    Ok(f)
}
fn stage(profile: &Path, c: &Candidate) -> Result<()> {
    install::put_once(profile, &journal_file(&c.release), &c.journal)?;
    for (rel, data) in &c.files {
        install::put_once(profile, rel, data)?;
    }
    verify_release(profile, &c.release)?;
    let installed_exe = paths::safe(
        profile,
        &release_file(
            &c.release,
            &format!("highgrade{}", std::env::consts::EXE_SUFFIX),
        ),
    )?;
    package::verify_binary(&installed_exe, &c.cli_version)?;
    let probe_root = paths::safe(
        profile,
        &format!(".highgrade/global/releases/{}", c.release),
    )?;
    package::verify_doctor(&installed_exe, &probe_root)?;
    Ok(())
}
pub fn status(profile: &Path) -> Result<Report> {
    let profile = paths::root(profile)?;
    let mut r = Report::new("global-status");
    match active(&profile)? {
        Some((release, _)) => r
            .measurements
            .push(json!({"profile":profile,"release":release,"connected":true})),
        None => r.finding(
            "unknown",
            "GlobalNotInstalled",
            ACTIVE,
            "Общая основа High Grade не установлена.",
        ),
    }
    Ok(r)
}
pub fn install(profile: &Path, source: &Path, executable: &Path) -> Result<Report> {
    let profile = paths::root(profile)?;
    let c = candidate(source, executable)?;
    package::verify_doctor(executable, source)?;
    if let Some((release, hash_old)) = active(&profile)? {
        if release == c.release && hash_old == hash(&c.journal) {
            return status(&profile);
        }
        return Err("GlobalAlreadyInstalled: use global-update".into());
    }
    let existing_journal = paths::safe(&profile, &journal_file(&c.release))?;
    let resuming = existing_journal.exists()
        && paths::read_limited(&existing_journal, 8 * 1024 * 1024)? == c.journal;
    if !resuming {
        for s in SKILLS {
            if paths::safe(&profile, &router(s))?.exists() {
                return Err(format!("GlobalSkillConflict: {s}"));
            }
        }
        for rel in c.files.keys() {
            if paths::safe(&profile, rel)?.exists() {
                return Err(format!("GlobalDestinationConflict: {rel}"));
            }
        }
    }
    let _guard = lock(&profile)?;
    if active(&profile)?.is_some() {
        return Err("GlobalActiveChanged".into());
    }
    stage(&profile, &c)?;
    install::put_once(&profile, ACTIVE, &active_bytes(&c.release, &c.journal))?;
    let mut r = status(&profile)?;
    r.operation = "global-install".into();
    r.measurements
        .push(json!({"manifest_sha256":c.manifest_hash}));
    Ok(r)
}
pub fn update(
    profile: &Path,
    source: Option<&Path>,
    executable: Option<&Path>,
    apply: bool,
    expected_sha256: Option<&str>,
    rollback: Option<&str>,
) -> Result<Report> {
    let profile = paths::root(profile)?;
    let (old, old_hash) = active(&profile)?.ok_or("GlobalNotInstalled")?;
    if let Some(release) = rollback {
        if source.is_some()
            || executable.is_some()
            || apply
            || expected_sha256.is_some()
            || !valid_release(release)
        {
            return Err("Usage: rollback accepts only release".into());
        }
        let journal = verify_release(&profile, release)?;
        let _guard = lock(&profile)?;
        if active(&profile)? != Some((old.clone(), old_hash.clone())) {
            return Err("GlobalActiveChanged".into());
        }
        package::replace_active(
            &paths::safe(&profile, ACTIVE)?,
            &active_bytes(release, &journal),
        )?;
        let mut r = status(&profile)?;
        r.operation = "global-update".into();
        r.measurements
            .push(json!({"from":old,"to":release,"rolled_back":true}));
        return Ok(r);
    }
    let c = candidate(
        source.ok_or("Usage: --source required")?,
        executable.ok_or("Usage: --candidate-exe required")?,
    )?;
    if old == c.release {
        return Err("GlobalSameRelease".into());
    }
    package::verify_doctor(executable.unwrap(), source.unwrap())?;
    if apply && expected_sha256 != Some(c.candidate_sha256.as_str()) {
        return Err("GlobalCandidateChanged: pass candidate_sha256 from preview".into());
    }
    if !apply && expected_sha256.is_some() {
        return Err("Usage: --candidate-sha256 requires --apply true".into());
    }
    for s in SKILLS {
        let rel = router(s);
        if c.files.get(&rel)
            != Some(&paths::read_limited(
                &paths::safe(&profile, &rel)?,
                8 * 1024 * 1024,
            )?)
        {
            return Err(format!("GlobalRouterIncompatible: {s}"));
        }
    }
    let mut r = Report::new("global-update");
    r.measurements.push(
        json!({"from":old,"candidate":c.release,"manifest_sha256":c.manifest_hash,"candidate_sha256":c.candidate_sha256,"apply":apply}),
    );
    if !apply {
        r.finding(
            "unknown",
            "SemanticReviewRequired",
            "global-update",
            "Сравните смысл общих правил перед применением.",
        );
        return Ok(r);
    }
    let _guard = lock(&profile)?;
    if active(&profile)? != Some((old.clone(), old_hash.clone())) {
        return Err("GlobalActiveChanged".into());
    }
    stage(&profile, &c)?;
    package::replace_active(
        &paths::safe(&profile, ACTIVE)?,
        &active_bytes(&c.release, &c.journal),
    )?;
    let installed_exe = paths::safe(
        &profile,
        &release_file(
            &c.release,
            &format!("highgrade{}", std::env::consts::EXE_SUFFIX),
        ),
    )?;
    let probe_root = paths::safe(
        &profile,
        &format!(".highgrade/global/releases/{}", c.release),
    )?;
    if let Err(e) = active(&profile)
        .and_then(|_| package::verify_binary(&installed_exe, &c.cli_version))
        .and_then(|_| package::verify_doctor(&installed_exe, &probe_root))
    {
        let old_journal = verify_release(&profile, &old)?;
        package::replace_active(
            &paths::safe(&profile, ACTIVE)?,
            &active_bytes(&old, &old_journal),
        )?;
        return Err(format!("GlobalPostcheckFailedRollback: {e}"));
    }
    r.measurements
        .push(json!({"active_release":c.release,"previous_release_retained":old}));
    Ok(r)
}

pub fn project_check(root: &Path, r: &mut Report) -> Result<bool> {
    let p = paths::safe(root, ".highgrade/project/INSTRUCTIONS.md")?;
    if !p.exists() {
        return Ok(false);
    }
    let data = paths::read_limited(&p, 1024 * 1024)?;
    let text = std::str::from_utf8(&data).map_err(|_| "ProjectInstructionInvalidUtf8")?;
    if !text.starts_with("---\nhighgrade_project_schema: 1\n---\n")
        && !text.starts_with("---\r\nhighgrade_project_schema: 1\r\n---\r\n")
    {
        r.finding(
            "failed",
            "ProjectInstructionSchemaUnsupported",
            ".highgrade/project/INSTRUCTIONS.md",
            "Нужна актуализация через highgrade-clear.",
        );
    } else {
        r.measurements
            .push(json!({"project_instruction":"compatible","schema_version":1}));
    }
    Ok(true)
}
