use crate::{Report, Result, hash, install, package, paths};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    path::Path,
};

const ACTIVE: &str = ".highgrade/global/active.json";
const LOCK: &str = ".highgrade/global/install.lock";
const APPROVE_SKILL: &str = "skills/highgrade-approve/SKILL.md";
const PUSH_SKILL: &str = "skills/highgrade-push/SKILL.md";
const DEPLOY_SKILL: &str = "skills/highgrade-deploy/SKILL.md";
const LEGACY_DEPLOY_SKILL: &str = "skills/deploy/SKILL.md";
const TRANSITION_SKILLS: [(&str, &str); 4] = [
    ("deploy", LEGACY_DEPLOY_SKILL),
    ("highgrade-deploy", DEPLOY_SKILL),
    ("highgrade-approve", APPROVE_SKILL),
    ("highgrade-push", PUSH_SKILL),
];
const SKILLS: [&str; 8] = [
    "highgrade-init",
    "highgrade-task",
    "highgrade-spec",
    "highgrade-work",
    "highgrade-clear",
    "highgrade-update",
    "highgrade-approve",
    "highgrade-push",
];
const DEPLOY_SKILLS: [&str; 7] = [
    "highgrade-init",
    "highgrade-task",
    "highgrade-spec",
    "highgrade-work",
    "highgrade-clear",
    "highgrade-update",
    "highgrade-deploy",
];
const LEGACY_SKILLS: [&str; 7] = [
    "highgrade-init",
    "highgrade-task",
    "highgrade-spec",
    "highgrade-work",
    "highgrade-clear",
    "highgrade-update",
    "deploy",
];
const PREVIOUS_SKILLS: [&str; 6] = [
    "highgrade-init",
    "highgrade-task",
    "highgrade-spec",
    "highgrade-work",
    "highgrade-clear",
    "highgrade-update",
];
const MATERIALS: [&str; 18] = [
    "rules.md",
    "procedures/init.md",
    "procedures/task.md",
    "procedures/spec.md",
    "procedures/work.md",
    "procedures/clear.md",
    "procedures/update.md",
    "procedures/approve.md",
    "procedures/push.md",
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
const DEPLOY_MATERIALS: [&str; 17] = [
    "rules.md",
    "procedures/init.md",
    "procedures/task.md",
    "procedures/spec.md",
    "procedures/work.md",
    "procedures/clear.md",
    "procedures/update.md",
    "procedures/deploy.md",
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
const PREVIOUS_MATERIALS: [&str; 16] = [
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
fn owned_routers(journal: &Value) -> Vec<(String, &'static str)> {
    TRANSITION_SKILLS
        .iter()
        .filter_map(|(name, backup)| {
            let rel = router(name);
            journal["files"][rel.as_str()]
                .is_string()
                .then_some((rel, *backup))
        })
        .collect()
}
fn remove_new_routers(profile: &Path, old_journal: &Value, candidate: &Candidate) -> Result<()> {
    for name in ["highgrade-approve", "highgrade-push"] {
        let rel = router(name);
        if !old_journal["files"][rel.as_str()].is_string() {
            let expected = candidate
                .files
                .get(&rel)
                .ok_or("GlobalCandidateRouterMissing")?;
            retire_router(profile, &rel, &hash(expected))
                .map_err(|e| format!("GlobalNewRouterCleanupFailed: {e}"))?;
        }
    }
    Ok(())
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
fn journal_bytes(
    release: &str,
    manifest_hash: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>> {
    let checksums: BTreeMap<String, String> = files
        .iter()
        .map(|(path, data)| (path.clone(), hash(data)))
        .collect();
    serde_json::to_vec_pretty(&json!({"schema_version":3,"release":release,"manifest_sha256":manifest_hash,"files":checksums}))
        .map_err(|e| e.to_string())
}
fn newline_equivalent(a: &[u8], b: &[u8]) -> bool {
    match (std::str::from_utf8(a), std::str::from_utf8(b)) {
        (Ok(a), Ok(b)) => a.replace("\r\n", "\n") == b.replace("\r\n", "\n"),
        _ => false,
    }
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
    for rel in [APPROVE_SKILL, PUSH_SKILL] {
        files.insert(
            release_file(release, rel),
            paths::read_limited(&paths::safe(&source, rel)?, 8 * 1024 * 1024)?,
        );
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
    let journal = journal_bytes(release, &manifest_hash, &files)?;
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
    let actual: BTreeSet<&str> = files.keys().map(String::as_str).collect();
    let expected = |skills: &[&str], materials: &[&str]| -> BTreeSet<String> {
        materials
            .iter()
            .map(|rel| release_file(release, rel))
            .chain(skills.iter().map(|name| router(name)))
            .chain(std::iter::once(release_file(
                release,
                &format!("highgrade{}", std::env::consts::EXE_SUFFIX),
            )))
            .collect()
    };
    let mut current = expected(&SKILLS, &MATERIALS);
    for rel in [APPROVE_SKILL, PUSH_SKILL] {
        current.insert(release_file(release, rel));
    }
    let mut deploy = expected(&DEPLOY_SKILLS, &DEPLOY_MATERIALS);
    deploy.insert(release_file(release, DEPLOY_SKILL));
    let mut legacy = expected(&LEGACY_SKILLS, &DEPLOY_MATERIALS);
    legacy.insert(release_file(release, LEGACY_DEPLOY_SKILL));
    let previous = expected(&PREVIOUS_SKILLS, &PREVIOUS_MATERIALS);
    if actual != current.iter().map(String::as_str).collect()
        && actual != deploy.iter().map(String::as_str).collect()
        && actual != legacy.iter().map(String::as_str).collect()
        && actual != previous.iter().map(String::as_str).collect()
    {
        return Err("GlobalJournalInvalid: files".into());
    }
    for (rel, expected) in files {
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
fn retire_router(profile: &Path, rel: &str, expected: &str) -> Result<()> {
    let path = paths::safe(profile, rel)?;
    if !path.exists() {
        return Ok(());
    }
    if hash(&paths::read_limited(&path, 8 * 1024 * 1024)?) != expected {
        return Err(format!("GlobalRouterChanged: {rel}"));
    }
    fs::remove_file(path).map_err(|e| format!("GlobalRouterCleanupFailed: {rel}: {e}"))
}
fn stage(profile: &Path, c: &Candidate) -> Result<()> {
    install::put_once(profile, &journal_file(&c.release), &c.journal)?;
    let new_routers = [router("highgrade-approve"), router("highgrade-push")];
    for (rel, data) in &c.files {
        if !new_routers.contains(rel) {
            install::put_once(profile, rel, data)?;
        }
    }
    let mut created = Vec::new();
    let checked = (|| {
        for rel in &new_routers {
            let existed = paths::safe(profile, rel)?.exists();
            install::put_once(profile, rel, &c.files[rel])?;
            if !existed {
                created.push(rel.clone());
            }
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
        package::verify_doctor(&installed_exe, &probe_root)
    })();
    if checked.is_err() {
        for rel in created {
            retire_router(profile, &rel, &hash(&c.files[&rel]))
                .map_err(|e| format!("GlobalStageRouterCleanupFailed: {e}"))?;
        }
    }
    checked
}

fn retire_release(profile: &Path, release: &str) -> Result<()> {
    let prefix = format!(".highgrade/global/releases/{release}/");
    let journal_path = paths::safe(profile, &journal_file(release))?;
    let journal_bytes = paths::read_limited(&journal_path, 8 * 1024 * 1024)?;
    let journal: Value = serde_json::from_slice(&journal_bytes)
        .map_err(|e| format!("GlobalRetireJournalInvalid: {e}"))?;
    if journal["schema_version"] != 3 || journal["release"] != release {
        return Err("GlobalRetireJournalInvalid".into());
    }
    let files = journal["files"]
        .as_object()
        .ok_or("GlobalRetireJournalInvalid: files")?;
    let allowed: BTreeSet<String> = MATERIALS
        .iter()
        .chain(DEPLOY_MATERIALS.iter())
        .chain(PREVIOUS_MATERIALS.iter())
        .map(|s| (*s).to_owned())
        .chain(
            SKILLS
                .iter()
                .chain(DEPLOY_SKILLS.iter())
                .chain(LEGACY_SKILLS.iter())
                .chain(PREVIOUS_SKILLS.iter())
                .map(|name| format!("skills/{name}/SKILL.md")),
        )
        .chain(std::iter::once(format!(
            "highgrade{}",
            std::env::consts::EXE_SUFFIX
        )))
        .collect();
    let mut owned = Vec::new();
    for (rel, expected) in files {
        if let Some(suffix) = rel.strip_prefix(&prefix) {
            if !allowed.contains(suffix)
                || !expected
                    .as_str()
                    .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            {
                return Err(format!("GlobalRetireJournalInvalid: {rel}"));
            }
            owned.push((rel.as_str(), expected.as_str().unwrap()));
        } else if !SKILLS
            .iter()
            .chain(DEPLOY_SKILLS.iter())
            .chain(LEGACY_SKILLS.iter())
            .chain(PREVIOUS_SKILLS.iter())
            .any(|name| router(name) == *rel)
        {
            return Err(format!("GlobalRetireJournalInvalid: {rel}"));
        }
    }
    // Journal paths are constrained to known release files. Remove only files
    // whose current bytes still match the journal, leaving foreign edits alone.
    let mut directories = BTreeSet::new();
    let release_dir = paths::safe(profile, prefix.trim_end_matches('/'))?;
    let mut issues = Vec::new();
    for (rel, expected) in &owned {
        let path = match paths::safe(profile, rel) {
            Ok(path) => path,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        if path.exists() {
            match paths::read_limited(&path, 128 * 1024 * 1024) {
                Ok(bytes) if hash(&bytes) == *expected => {}
                Ok(_) => {
                    issues.push(format!("GlobalRetireFileChanged: {rel}"));
                    continue;
                }
                Err(error) => {
                    issues.push(error);
                    continue;
                }
            }
            if let Err(error) = fs::remove_file(&path) {
                issues.push(format!("GlobalRetireFileFailed: {rel}: {error}"));
                continue;
            }
        }
        let mut parent = path.parent();
        while let Some(dir) = parent {
            if !dir.starts_with(&release_dir) {
                break;
            }
            directories.insert(dir.to_path_buf());
            parent = dir.parent();
        }
    }
    directories.insert(release_dir.clone());
    if !issues.is_empty() {
        return Err(issues.join("; "));
    }
    let mut directories: Vec<_> = directories.into_iter().collect();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for dir in directories {
        if dir == release_dir {
            continue;
        }
        match fs::remove_dir(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) if e.kind() == std::io::ErrorKind::DirectoryNotEmpty => {}
            Err(e) => {
                return Err(format!(
                    "GlobalRetireDirectoryFailed: {}: {e}",
                    dir.display()
                ));
            }
        }
    }
    let remaining = fs::read_dir(&release_dir).map_err(|e| {
        format!(
            "GlobalRetireDirectoryFailed: {}: {e}",
            release_dir.display()
        )
    })?;
    for entry in remaining {
        let entry = entry.map_err(|e| format!("GlobalRetireDirectoryFailed: {e}"))?;
        if entry.path() != journal_path {
            return Err(format!(
                "GlobalRetireForeignContent: {}",
                entry.path().display()
            ));
        }
    }
    fs::remove_file(&journal_path).map_err(|e| format!("GlobalRetireJournalFailed: {e}"))?;
    if let Err(error) = fs::remove_dir(&release_dir) {
        fs::write(&journal_path, &journal_bytes).map_err(|restore| {
            format!(
                "GlobalRetireDirectoryFailed: {error}; GlobalRetireJournalRestoreFailed: {restore}"
            )
        })?;
        return Err(format!("GlobalRetireDirectoryFailed: {error}"));
    }
    Ok(())
}

fn retire_inactive_releases(profile: &Path, active_release: &str, report: &mut Report) {
    let base = match paths::safe(profile, ".highgrade/global/releases") {
        Ok(path) => path,
        Err(error) => {
            report.finding(
                "warning",
                "OldReleaseCleanupPending",
                ".highgrade/global/releases",
                &error,
            );
            return;
        }
    };
    let entries = match fs::read_dir(&base) {
        Ok(entries) => entries,
        Err(error) => {
            report.finding(
                "warning",
                "OldReleaseCleanupPending",
                ".highgrade/global/releases",
                &error.to_string(),
            );
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.finding(
                    "warning",
                    "OldReleaseCleanupPending",
                    ".highgrade/global/releases",
                    &error.to_string(),
                );
                continue;
            }
        };
        let release = entry.file_name().to_string_lossy().into_owned();
        if release == active_release || !valid_release(&release) {
            continue;
        }
        if let Err(error) = retire_release(profile, &release) {
            report.finding("warning", "OldReleaseCleanupPending", &release, &error);
        }
    }
}
fn retire_failed_candidate(profile: &Path, candidate: &Candidate) -> Result<()> {
    let journal = paths::safe(profile, &journal_file(&candidate.release))?;
    if journal.exists() {
        if paths::read_limited(&journal, 8 * 1024 * 1024)? != candidate.journal {
            return Err("GlobalCandidateJournalChanged: cleanup skipped".into());
        }
        retire_release(profile, &candidate.release)
    } else {
        Ok(())
    }
}
pub fn status(profile: &Path) -> Result<Report> {
    let profile = paths::root(profile)?;
    let mut r = Report::new("global-status");
    match active(&profile)? {
        Some((release, _)) => {
            let journal: Value = serde_json::from_slice(&paths::read_limited(
                &paths::safe(&profile, &journal_file(&release))?,
                8 * 1024 * 1024,
            )?)
            .map_err(|e| e.to_string())?;
            for name in [
                "deploy",
                "highgrade-deploy",
                "highgrade-approve",
                "highgrade-push",
            ] {
                let rel = router(name);
                if !journal["files"][rel.as_str()].is_string()
                    && paths::safe(&profile, &rel)?.exists()
                {
                    r.finding(
                        "warning",
                        "InactiveRouterRemains",
                        &rel,
                        "Этот путь не принадлежит активной поставке; проверьте происхождение перед любым удалением.",
                    );
                }
            }
            r.measurements
                .push(json!({"profile":profile,"release":release,"connected":true}));
        }
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
        for s in SKILLS.into_iter().chain(["deploy", "highgrade-deploy"]) {
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
) -> Result<Report> {
    let profile = paths::root(profile)?;
    let (old, old_hash) = active(&profile)?.ok_or("GlobalNotInstalled")?;
    let mut c = candidate(
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
    let old_journal: Value = serde_json::from_slice(&paths::read_limited(
        &paths::safe(&profile, &journal_file(&old))?,
        8 * 1024 * 1024,
    )?)
    .map_err(|e| e.to_string())?;
    let mut adapted = false;
    for name in PREVIOUS_SKILLS {
        let rel = router(name);
        if old_journal["files"][rel.as_str()].is_string() {
            let installed = paths::read_limited(&paths::safe(&profile, &rel)?, 8 * 1024 * 1024)?;
            let candidate_router = c
                .files
                .get_mut(&rel)
                .ok_or("GlobalCandidateRouterMissing")?;
            if *candidate_router != installed && newline_equivalent(candidate_router, &installed) {
                *candidate_router = installed;
                adapted = true;
            }
        }
    }
    if adapted {
        c.journal = journal_bytes(&c.release, &c.manifest_hash, &c.files)?;
    }
    let old_routers = owned_routers(&old_journal);
    let candidate_journal = paths::safe(&profile, &journal_file(&c.release))?;
    let resuming = candidate_journal.exists()
        && paths::read_limited(&candidate_journal, 8 * 1024 * 1024)? == c.journal;
    for s in SKILLS {
        let rel = router(s);
        let target = paths::safe(&profile, &rel)?;
        if target.exists()
            && (!old_journal["files"][rel.as_str()].is_string() && !resuming
                || c.files.get(&rel) != Some(&paths::read_limited(&target, 8 * 1024 * 1024)?))
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
    if let Err(error) = stage(&profile, &c) {
        retire_failed_candidate(&profile, &c)
            .map_err(|cleanup| format!("GlobalStageFailed: {error}; {cleanup}"))?;
        return Err(error);
    }
    let active_path = paths::safe(&profile, ACTIVE)?;
    if let Err(error) = package::replace_active(&active_path, &active_bytes(&c.release, &c.journal))
    {
        let routers = remove_new_routers(&profile, &old_journal, &c);
        let release = retire_failed_candidate(&profile, &c);
        if let Err(cleanup) = routers.and(release) {
            return Err(format!("GlobalActivationCleanupFailed: {error}; {cleanup}"));
        }
        return Err(error);
    }
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
        let old_journal_bytes = verify_release(&profile, &old)?;
        package::replace_active(&active_path, &active_bytes(&old, &old_journal_bytes))?;
        let routers = remove_new_routers(&profile, &old_journal, &c);
        let release = retire_failed_candidate(&profile, &c);
        if let Err(cleanup) = routers.and(release) {
            return Err(format!("GlobalPostcheckCleanupFailed: {e}; {cleanup}"));
        }
        return Err(format!("GlobalPostcheckFailedRestoredPrevious: {e}"));
    }
    for (rel, _) in old_routers {
        if !c.files.contains_key(&rel) {
            let expected = old_journal["files"][rel.as_str()].as_str().unwrap();
            if let Err(error) = retire_router(&profile, &rel, expected) {
                r.finding("warning", "LegacyRouterCleanupPending", &rel, &error);
            }
        }
    }
    retire_inactive_releases(&profile, &c.release, &mut r);
    r.measurements.push(json!({"active_release":c.release}));
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
