use crate::{Report, Result, hash, install, paths, read_json};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const ACTIVE: &str = ".highgrade/active.json";
const LOCK: &str = ".highgrade/install.lock";
const SKILLS: [&str; 4] = [
    "highgrade-init",
    "highgrade-work",
    "highgrade-refresh",
    "highgrade-update",
];
const MATERIALS: [&str; 13] = [
    "rules.md",
    "procedures/init.md",
    "procedures/work.md",
    "procedures/refresh.md",
    "procedures/update.md",
    "procedures/archive.md",
    "references/audit.md",
    "references/cli.md",
    "templates/README.md",
    "templates/AGENTS.md",
    "templates/ARCHITECTURE.md",
    "templates/ENGINEERING.md",
    "templates/DEVELOPMENT.md",
];

pub struct Candidate {
    pub release: String,
    pub cli_version: String,
    pub manifest_hash: String,
    pub executable_hash: String,
    files: BTreeMap<String, Vec<u8>>,
    journal: Vec<u8>,
}
fn skill_target(name: &str) -> String {
    format!(".agents/skills/{name}/SKILL.md")
}
fn release_file(release: &str, rel: &str) -> String {
    format!(".highgrade/releases/{release}/{rel}")
}
fn journal_path(release: &str) -> String {
    release_file(release, "journal.json")
}

pub fn candidate(source: &Path, executable: &Path) -> Result<Candidate> {
    let source = paths::root(source)?;
    let manifest_bytes =
        paths::read_limited(&paths::safe(&source, "manifest.json")?, 8 * 1024 * 1024)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes).map_err(|e| e.to_string())?;
    if manifest["schema_version"] != 2 {
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
    let cli_version = manifest["cli_version"]
        .as_str()
        .ok_or("ManifestInvalid: cli_version")?;
    if cli_version.is_empty()
        || cli_version.len() > 32
        || !cli_version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'+')
    {
        return Err("ManifestInvalid: cli_version".into());
    }
    let hashes = manifest["files"]
        .as_object()
        .ok_or("ManifestInvalid: files")?;
    if hashes.len() != SKILLS.len() + MATERIALS.len() {
        return Err("ManifestFilesInvalid".into());
    }
    let mut files = BTreeMap::new();
    for rel in MATERIALS
        .iter()
        .map(|s| s.to_string())
        .chain(SKILLS.iter().map(|s| format!("skills/{s}/SKILL.md")))
    {
        let data = paths::read_limited(&paths::safe(&source, &rel)?, 8 * 1024 * 1024)?;
        if hashes.get(&rel).and_then(Value::as_str) != Some(&hash(&data)) {
            return Err(format!("ManifestHashMismatch: {rel}"));
        }
        let target = if let Some(name) = rel
            .strip_prefix("skills/")
            .and_then(|s| s.strip_suffix("/SKILL.md"))
        {
            skill_target(name)
        } else {
            release_file(release, &rel)
        };
        files.insert(target, data);
    }
    let executable_bytes = paths::read_limited(executable, 128 * 1024 * 1024)?;
    let executable_hash = hash(&executable_bytes);
    files.insert(
        release_file(
            release,
            &format!("highgrade{}", std::env::consts::EXE_SUFFIX),
        ),
        executable_bytes,
    );
    let map: BTreeMap<String, String> = files.iter().map(|(p, v)| (p.clone(), hash(v))).collect();
    let journal=serde_json::to_vec_pretty(&json!({"schema_version":2,"release":release,"manifest_sha256":hash(&manifest_bytes),"files":map})).map_err(|e|e.to_string())?;
    Ok(Candidate {
        release: release.into(),
        cli_version: cli_version.into(),
        manifest_hash: hash(&manifest_bytes),
        executable_hash,
        files,
        journal,
    })
}
fn release_verified(root: &Path, release: &str) -> Result<Vec<u8>> {
    let bytes = paths::read_limited(&paths::safe(root, &journal_path(release))?, 8 * 1024 * 1024)?;
    let journal: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if journal["schema_version"] != 2 || journal["release"] != release {
        return Err("ReleaseJournalInvalid".into());
    }
    let files = journal["files"]
        .as_object()
        .ok_or("ReleaseJournalInvalid: files")?;
    if files.len() != SKILLS.len() + MATERIALS.len() + 1 {
        return Err("ReleaseJournalInvalid: count".into());
    }
    for (rel, h) in files {
        let expected = h.as_str().ok_or("ReleaseJournalInvalid: hash")?;
        if !rel.starts_with(&format!(".highgrade/releases/{release}/"))
            && !SKILLS.iter().any(|s| rel == &skill_target(s))
        {
            return Err("ReleaseJournalInvalid: path".into());
        }
        if hash(&paths::read_limited(
            &paths::safe(root, rel)?,
            128 * 1024 * 1024,
        )?) != expected
        {
            return Err(format!("InstalledFileChanged: {rel}"));
        }
    }
    Ok(bytes)
}
pub fn active(root: &Path) -> Result<Option<(String, String)>> {
    let p = paths::safe(root, ACTIVE)?;
    if !p.exists() {
        return Ok(None);
    }
    let value = read_json(&p)?;
    let release = value["release"].as_str().ok_or("ActiveInvalid: release")?;
    if release.is_empty()
        || release.len() > 64
        || !release
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("ActiveInvalid: release".into());
    }
    let expected = value["journal_sha256"]
        .as_str()
        .ok_or("ActiveInvalid: hash")?;
    let bytes = release_verified(root, release)?;
    if hash(&bytes) != expected {
        return Err("ActiveInvalid: journal changed".into());
    }
    Ok(Some((release.into(), expected.into())))
}
fn active_bytes(release: &str, journal: &[u8]) -> Vec<u8> {
    serde_json::to_vec_pretty(&json!({"schema_version":2,"status":"connected","release":release,"journal_sha256":hash(journal),"workflow_ready":false})).unwrap()
}
fn binary_report(executable: &Path, args: &[&str]) -> Result<Value> {
    if !executable.is_absolute() {
        return Err("CandidateExecutableMustBeAbsolute".into());
    }
    let mut child = Command::new(executable)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("CandidateExecutableInvalid: {e}"))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("CandidateCommandTimedOut".into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    let code = output.status.code();
    if (args.first() == Some(&"--version") && code != Some(0))
        || (args.first() == Some(&"doctor") && ![Some(0), Some(2)].contains(&code))
    {
        let excerpt = String::from_utf8_lossy(&output.stdout)
            .chars()
            .take(400)
            .collect::<String>();
        return Err(format!(
            "CandidateCommandFailed: exit={code:?}; report={excerpt}"
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("CandidateReportInvalid: {e}"))
}
pub(crate) fn verify_binary(executable: &Path, expected_version: &str) -> Result<()> {
    let report = binary_report(executable, &["--version"])?;
    if report["operation"] != "version"
        || report["status"] != "passed"
        || report["measurements"][0]["version"].as_str() != Some(expected_version)
    {
        return Err(format!(
            "CandidateExecutableVersionMismatch: expected {expected_version}"
        ));
    }
    Ok(())
}
pub(crate) fn verify_doctor(executable: &Path, root: &Path) -> Result<()> {
    let root = root.to_str().ok_or("ProjectPathInvalidUnicode")?;
    let report = binary_report(executable, &["doctor", "--root", root])?;
    if report["operation"] != "doctor"
        || !["passed", "warning", "unknown"]
            .iter()
            .any(|v| report["status"] == *v)
    {
        return Err("CandidateDoctorFailed".into());
    }
    Ok(())
}
fn adaptation_hash(root: &Path, decision_rel: Option<&str>) -> Result<String> {
    let base = ".highgrade/project";
    let mut stack = vec![base.to_string()];
    let mut files = BTreeMap::new();
    while let Some(dir) = stack.pop() {
        let path = paths::safe(root, &dir)?;
        if !path.exists() {
            continue;
        }
        for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "AdaptationPathInvalid")?;
            let rel = format!("{dir}/{name}");
            let safe = paths::safe(root, &rel)?;
            if safe.is_dir() {
                stack.push(rel);
            } else if safe.is_file() {
                if decision_rel != Some(rel.as_str()) {
                    if files.len() >= 10000 {
                        return Err("AdaptationTooManyFiles".into());
                    }
                    files.insert(rel, hash(&paths::read_limited(&safe, 32 * 1024 * 1024)?));
                }
            } else {
                return Err(format!("AdaptationEntryInvalid: {rel}"));
            }
        }
    }
    Ok(hash(
        &serde_json::to_vec(&files).map_err(|e| e.to_string())?,
    ))
}
fn lock(root: &Path) -> Result<File> {
    let p = paths::safe(root, LOCK)?;
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    paths::safe(root, LOCK)?;
    let f = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(p)
        .map_err(|e| e.to_string())?;
    f.try_lock().map_err(|e| format!("InstallationBusy: {e}"))?;
    Ok(f)
}
fn stage(root: &Path, c: &Candidate) -> Result<()> {
    install::put_once(root, &journal_path(&c.release), &c.journal)?;
    for (rel, data) in &c.files {
        install::put_once(root, rel, data)?;
    }
    release_verified(root, &c.release)?;
    Ok(())
}
pub fn install(root: &Path, source: &Path, audit_rel: &str) -> Result<Report> {
    let root = paths::root(root)?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let source_manifest = read_json(&source.join("manifest.json"))?;
    if source_manifest["cli_version"] != env!("CARGO_PKG_VERSION") {
        return Err("InstallExecutableVersionMismatch".into());
    }
    if paths::safe(&root, ACTIVE)?.exists() {
        let c = candidate(source, &executable)?;
        if active(&root)?.as_ref() != Some(&(c.release.clone(), hash(&c.journal))) {
            return Err("AlreadyConnected: use update".into());
        }
        let mut r = Report::new("install");
        r.measurements
            .push(json!({"release":c.release,"connection":"connected","already_installed":true}));
        return Ok(r);
    }
    if paths::safe(&root, ".highgrade/installed.json")?.exists() {
        return Err("PrototypeInstalled: migration required".into());
    }
    let audit = read_json(&paths::safe(&root, audit_rel)?)?;
    if audit["schema_version"] != 1
        || audit["status"] != "approved"
        || audit["scope"] != "full-project"
        || audit["evidence"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err("AuditRequired".into());
    }
    install::verify_audit_exclusions(&root, &audit)?;
    let expected: BTreeMap<String, String> =
        serde_json::from_value(audit["files"].clone()).map_err(|_| "AuditInvalid: files")?;
    if install::inventory(&root, audit_rel)? != expected {
        return Err("AuditStale".into());
    }
    let c = candidate(source, &executable)?;
    let journal = paths::safe(&root, &journal_path(&c.release))?;
    let resuming = journal.exists() && paths::read_limited(&journal, 8 * 1024 * 1024)? == c.journal;
    if journal.exists() && !resuming {
        return Err("DifferentInstallation".into());
    }
    if !resuming {
        // A pre-existing skill belongs to the project, even if its bytes match.
        for s in SKILLS {
            if paths::safe(&root, &skill_target(s))?.exists() {
                return Err(format!("DestinationConflict: {s}"));
            }
        }
        for rel in c.files.keys() {
            if paths::safe(&root, rel)?.exists() {
                return Err(format!("DestinationConflict: {rel}"));
            }
        }
    }
    let _guard = lock(&root)?;
    if install::inventory(&root, audit_rel)? != expected {
        return Err("AuditStale: concurrent change".into());
    }
    stage(&root, &c)?;
    if install::inventory(&root, audit_rel)? != expected {
        return Err("AuditStale: change during install".into());
    }
    install::put_once(&root, ACTIVE, &active_bytes(&c.release, &c.journal))?;
    active(&root)?;
    let mut r = Report::new("install");
    r.measurements
        .push(json!({"release":c.release,"connection":"connected","workflow_ready":false}));
    r.limitations.push("Receipt подтверждён только структурно; адаптация и её пользовательская приёмка выполняются отдельно.".into());
    Ok(r)
}
fn preflight(root: &Path, c: &Candidate, old: &str) -> Result<()> {
    if old == c.release {
        return Err("SameRelease: no update".into());
    }
    for s in SKILLS {
        let rel = skill_target(s);
        let current = paths::read_limited(&paths::safe(root, &rel)?, 8 * 1024 * 1024)?;
        if c.files.get(&rel) != Some(&current) {
            return Err(format!("SkillRouterIncompatible: {s}"));
        }
    }
    let journal = paths::safe(root, &journal_path(&c.release))?;
    if journal.exists() {
        if paths::read_limited(&journal, 8 * 1024 * 1024)? != c.journal {
            return Err("DifferentReleaseAlreadyStaged".into());
        }
    }
    for (rel, data) in &c.files {
        if rel.starts_with(&format!(".highgrade/releases/{}/", c.release)) {
            let p = paths::safe(root, rel)?;
            if p.exists() && paths::read_limited(&p, 128 * 1024 * 1024)? != *data {
                return Err(format!("DestinationConflict: {rel}"));
            }
        }
    }
    Ok(())
}
#[cfg(windows)]
pub(crate) fn replace_active(dest: &Path, new_data: &[u8]) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced: *const u16,
            replacement: *const u16,
            backup: *const u16,
            flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }
    let temp = dest.with_extension(format!("{}.part", std::process::id()));
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("ActivationTempConflict: {e}"))?;
    f.write_all(new_data).map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())?;
    drop(f);
    let to_wide = |p: &Path| {
        p.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let (a, b) = (to_wide(dest), to_wide(&temp));
    let ok = unsafe {
        ReplaceFileW(
            a.as_ptr(),
            b.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(format!(
            "ActivationFailed: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}
#[cfg(not(windows))]
pub(crate) fn replace_active(dest: &Path, new_data: &[u8]) -> Result<()> {
    let temp = dest.with_extension("next");
    fs::write(&temp, new_data).map_err(|e| e.to_string())?;
    fs::rename(temp, dest).map_err(|e| e.to_string())
}
pub fn update(
    root: &Path,
    source: Option<&Path>,
    candidate_exe: Option<&Path>,
    decision_rel: Option<&str>,
    apply: bool,
    rollback: Option<&str>,
) -> Result<Report> {
    let probe_root = std::path::absolute(root).map_err(|e| e.to_string())?;
    let root = paths::root(root)?;
    let (old, old_hash) = active(&root)?.ok_or("NotConnected: v2 release required")?;
    if let Some(release) = rollback {
        if source.is_some() || candidate_exe.is_some() || decision_rel.is_some() || apply {
            return Err("Usage: rollback accepts only release".into());
        }
        if release == old {
            return Err("SameRelease".into());
        }
        let bytes = release_verified(&root, release)?;
        let _guard = lock(&root)?;
        if active(&root)?.as_ref() != Some(&(old.clone(), old_hash.clone())) {
            return Err("ActiveChanged".into());
        }
        replace_active(&paths::safe(&root, ACTIVE)?, &active_bytes(release, &bytes))?;
        active(&root)?;
        let mut r = Report::new("update");
        r.measurements
            .push(json!({"from":old,"to":release,"rolled_back":true}));
        return Ok(r);
    }
    let c = candidate(
        source.ok_or("Usage: source required")?,
        candidate_exe.ok_or("Usage: --candidate-exe required")?,
    )?;
    preflight(&root, &c, &old)?;
    let selected_exe = candidate_exe.ok_or("Usage: --candidate-exe required")?;
    let mut r = Report::new("update");
    let excluded_decision = decision_rel.unwrap_or(".highgrade/project/update-decision.json");
    let project_hash = adaptation_hash(&root, Some(excluded_decision))?;
    let content_hash = hash(
        &serde_json::to_vec(&install::inventory(&root, excluded_decision)?)
            .map_err(|e| e.to_string())?,
    );
    r.measurements.push(
        json!({"from":old,"candidate":c.release,"manifest_sha256":c.manifest_hash,"candidate_exe_sha256":c.executable_hash,"adaptation_sha256":project_hash,"project_sha256":content_hash,"apply":apply}),
    );
    if !apply {
        verify_binary(selected_exe, &c.cli_version)?;
        verify_doctor(selected_exe, &probe_root)?;
        r.finding(
            "unknown",
            "SemanticDecisionRequired",
            "update",
            "Проверьте смысл правил и проектной адаптации до применения.",
        );
        return Ok(r);
    }
    let rel = decision_rel.ok_or("DecisionRequired: --decision REL")?;
    let decision = read_json(&paths::safe(&root, rel)?)?;
    let decision_hash = hash(&paths::read_limited(
        &paths::safe(&root, rel)?,
        8 * 1024 * 1024,
    )?);
    if decision["status"] != "approved"
        || decision["from"] != old
        || decision["to"] != c.release
        || decision["active_journal_sha256"] != old_hash
        || decision["candidate_manifest_sha256"] != c.manifest_hash
        || decision["candidate_exe_sha256"] != c.executable_hash
        || decision["adaptation_sha256"] != project_hash
        || decision["project_sha256"] != content_hash
        || decision["evidence"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
    {
        return Err("DecisionInvalidOrStale".into());
    }
    verify_binary(selected_exe, &c.cli_version)?;
    verify_doctor(selected_exe, &probe_root)?;
    let _guard = lock(&root)?;
    if hash(&paths::read_limited(
        &paths::safe(&root, rel)?,
        8 * 1024 * 1024,
    )?) != decision_hash
    {
        return Err("DecisionChanged".into());
    }
    if active(&root)?.as_ref() != Some(&(old.clone(), old_hash)) {
        return Err("ActiveChanged".into());
    }
    preflight(&root, &c, &old)?;
    if adaptation_hash(&root, decision_rel)? != project_hash {
        return Err("AdaptationChanged".into());
    }
    if hash(&serde_json::to_vec(&install::inventory(&root, rel)?).map_err(|e| e.to_string())?)
        != content_hash
    {
        return Err("ProjectChanged".into());
    }
    stage(&root, &c)?;
    if hash(&paths::read_limited(
        &paths::safe(&root, rel)?,
        8 * 1024 * 1024,
    )?) != decision_hash
    {
        return Err("DecisionChanged".into());
    }
    if adaptation_hash(&root, decision_rel)? != project_hash {
        return Err("AdaptationChanged".into());
    }
    if hash(&serde_json::to_vec(&install::inventory(&root, rel)?).map_err(|e| e.to_string())?)
        != content_hash
    {
        return Err("ProjectChanged".into());
    }
    // The only activation write replaces one pointer; previous release remains immutable.
    replace_active(
        &paths::safe(&root, ACTIVE)?,
        &active_bytes(&c.release, &c.journal),
    )?;
    let installed_exe = paths::safe(
        &root,
        &release_file(
            &c.release,
            &format!("highgrade{}", std::env::consts::EXE_SUFFIX),
        ),
    )?;
    if let Err(e) = active(&root).and_then(|_| verify_doctor(&installed_exe, &probe_root)) {
        let old_bytes = release_verified(&root, &old)?;
        replace_active(
            &paths::safe(&root, ACTIVE)?,
            &active_bytes(&old, &old_bytes),
        )?;
        return Err(format!("PostcheckFailedRollback: {e}"));
    }
    r.measurements
        .push(json!({"active_release":c.release,"previous_release_retained":old}));
    r.limitations.push("Структурная совместимость проверена CLI; смысловая проверка подтверждена внешним решением, не вычислена программой.".into());
    Ok(r)
}
