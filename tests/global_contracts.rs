use highgrade::{global, hash};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn temp() -> PathBuf {
    loop {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "hg-global-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&p) {
            Ok(()) => return p,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("could not create isolated test directory: {error}"),
        }
    }
}
fn source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kit")
}
#[test]
fn kit_manifest_uses_stable_lf_bytes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let attributes = fs::read_to_string(root.join(".gitattributes")).unwrap();
    assert!(attributes.lines().any(|line| line == "kit/** text eol=lf"));
    let manifest: Value =
        serde_json::from_slice(&fs::read(source().join("manifest.json")).unwrap()).unwrap();
    for (rel, expected) in manifest["files"].as_object().unwrap() {
        let data = fs::read(source().join(rel)).unwrap();
        assert!(
            !data.windows(2).any(|bytes| bytes == b"\r\n"),
            "{rel} must use LF"
        );
        assert_eq!(hash(&data), expected.as_str().unwrap(), "{rel}");
    }
}
fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_highgrade"))
}
fn write(p: &Path, bytes: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for item in fs::read_dir(from).unwrap() {
        let item = item.unwrap();
        let dest = to.join(item.file_name());
        if item.file_type().unwrap().is_dir() {
            copy_tree(&item.path(), &dest);
        } else {
            fs::copy(item.path(), dest).unwrap();
        }
    }
}
fn candidate() -> PathBuf {
    let dst = temp();
    copy_tree(&source(), &dst);
    let manifest_path = dst.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["release"] = json!("v0-2-12");
    let rules = dst.join("rules.md");
    write(&rules, b"# New shared rules\n");
    manifest["files"]["rules.md"] = json!(hash(&fs::read(&rules).unwrap()));
    write(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    dst
}
// highgrade: HG-PD-S01
#[test]
fn installed_runtime_references_survive_removal_of_source_copy() {
    let copied = temp();
    copy_tree(&source(), &copied);
    let profile = temp();
    let manifest: Value =
        serde_json::from_slice(&fs::read(copied.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        global::install(&profile, &copied, &exe()).unwrap().status,
        "passed"
    );
    fs::remove_dir_all(&copied).unwrap();
    let release = profile
        .join(".highgrade/global/releases")
        .join(manifest["release"].as_str().unwrap());
    for (rel, expected) in manifest["files"].as_object().unwrap() {
        let installed = if let Some(skill) = rel.strip_prefix("skills/") {
            profile.join(".agents/skills").join(skill)
        } else {
            release.join(rel)
        };
        let data = fs::read(&installed).unwrap();
        assert_eq!(hash(&data), expected.as_str().unwrap(), "{rel}");
        // Templates intentionally describe links in the future target project.
        if rel.starts_with("templates/") || rel.starts_with("skills/") {
            continue;
        }
        let text = std::str::from_utf8(&data).unwrap();
        for event in pulldown_cmark::Parser::new(text) {
            if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link { dest_url, .. }) = event
            {
                let link = dest_url.split('#').next().unwrap();
                if link.is_empty() || link.starts_with("https://") {
                    continue;
                }
                let target = installed
                    .parent()
                    .unwrap()
                    .join(link)
                    .canonicalize()
                    .unwrap();
                assert!(
                    target.starts_with(release.canonicalize().unwrap()),
                    "external dependency: {rel}: {link}"
                );
            }
        }
    }
    assert_eq!(global::status(&profile).unwrap().status, "passed");
}

// highgrade: HG-PD-S05
#[test]
fn shipped_registry_example_is_accepted_without_inventing_budgets() {
    let profile = temp();
    assert_eq!(
        global::install(&profile, &source(), &exe()).unwrap().status,
        "passed"
    );
    let manifest: Value =
        serde_json::from_slice(&fs::read(source().join("manifest.json")).unwrap()).unwrap();
    let guide = fs::read_to_string(
        profile
            .join(".highgrade/global/releases")
            .join(manifest["release"].as_str().unwrap())
            .join("references/cli.md"),
    )
    .unwrap();
    let registry = guide
        .split("```json\n")
        .nth(1)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    let project = temp();
    write(
        &project.join(".highgrade/project/documents.json"),
        registry.as_bytes(),
    );
    write(&project.join("README.md"), b"# Fixture\n[Agent](AGENTS.md) [Architecture](docs/ARCHITECTURE.md) [Engineering](docs/ENGINEERING.md) [Development](docs/DEVELOPMENT.md)\n");
    write(
        &project.join("AGENTS.md"),
        b"[Project](.highgrade/project/INSTRUCTIONS.md)\n",
    );
    write(
        &project.join(".highgrade/project/INSTRUCTIONS.md"),
        b"---\nhighgrade_project_schema: 1\n---\n[Registry](documents.json)\n",
    );
    for name in ["ARCHITECTURE", "ENGINEERING", "DEVELOPMENT"] {
        write(&project.join(format!("docs/{name}.md")), b"# Fixture\n");
    }
    let output = Command::new(exe())
        .args(["inspect", "--root", project.to_str().unwrap()])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["operation"], "inspect");
    let findings = report["findings"].as_array().unwrap();
    assert!(
        !findings.iter().any(|f| f["status"] == "failed"),
        "{report}"
    );
    assert!(
        findings.iter().any(|f| f["code"] == "BudgetNotAgreed"),
        "{report}"
    );
    assert_ne!(report["status"], "passed");
}

#[test]
fn global_install_does_not_touch_project_and_checks_adapter() {
    let profile = temp();
    let project = temp();
    write(&project.join("README.md"), b"project-owned");
    let report = global::install(&profile, &source(), &exe()).unwrap();
    assert_eq!(report.status, "passed");
    for name in [
        "init", "task", "spec", "work", "clear", "update", "approve", "push",
    ] {
        assert!(
            profile
                .join(format!(".agents/skills/highgrade-{name}/SKILL.md"))
                .is_file()
        );
        assert!(
            !project
                .join(format!(".agents/skills/highgrade-{name}/SKILL.md"))
                .exists()
        );
    }
    assert!(
        !profile
            .join(".agents/skills/highgrade-deploy/SKILL.md")
            .exists()
    );
    assert_eq!(
        fs::read(project.join("README.md")).unwrap(),
        b"project-owned"
    );
    let adapter = fs::read(source().join("templates/INSTRUCTIONS.md")).unwrap();
    write(
        &project.join(".highgrade/project/INSTRUCTIONS.md"),
        &adapter,
    );
    let doctor = highgrade::doctor(&project).unwrap();
    assert!(
        doctor
            .measurements
            .iter()
            .any(|m| m["project_instruction"] == "compatible")
    );
    assert!(!doctor.findings.iter().any(|f| f["code"] == "NotConnected"));
    assert_eq!(
        global::install(&profile, &source(), &exe()).unwrap().status,
        "passed"
    );
}

// highgrade: HG-TR-S01
#[test]
fn doctor_keeps_unavailable_openspec_version_unknown() {
    let project = temp();
    let output = Command::new(env!("CARGO_BIN_EXE_highgrade"))
        .args(["doctor", "--root", project.to_str().unwrap()])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let openspec = report["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["tool"] == "openspec")
        .unwrap();
    assert_eq!(openspec["found_in_path"], false);
    assert_eq!(openspec["executed"], false);
    assert_eq!(openspec["version"], Value::Null);
    assert!(report["findings"].as_array().unwrap().iter().any(|entry| {
        entry["code"] == "DependencyMissing"
            && entry["location"] == "openspec"
            && entry["status"] == "unknown"
    }));
}
#[test]
fn global_update_preview_switch_and_cleanup_keep_unrelated_files() {
    let profile = temp();
    write(&profile.join("personal.txt"), b"keep");
    global::install(&profile, &source(), &exe()).unwrap();
    let next = candidate();
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None).unwrap();
    assert_eq!(preview.status, "unknown");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-11"
    );
    let preview_hash = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    assert!(
        global::update(&profile, Some(&next), Some(&exe()), true, Some("wrong"),)
            .unwrap_err()
            .contains("GlobalCandidateChanged")
    );
    let rules_path = next.join("rules.md");
    let manifest_path = next.join("manifest.json");
    let old_rules = fs::read(&rules_path).unwrap();
    let old_manifest = fs::read(&manifest_path).unwrap();
    write(&rules_path, b"# Changed after preview\n");
    let mut changed: Value = serde_json::from_slice(&old_manifest).unwrap();
    changed["files"]["rules.md"] = json!(hash(&fs::read(&rules_path).unwrap()));
    write(
        &manifest_path,
        &serde_json::to_vec_pretty(&changed).unwrap(),
    );
    assert!(
        global::update(
            &profile,
            Some(&next),
            Some(&exe()),
            true,
            Some(preview_hash),
        )
        .unwrap_err()
        .contains("GlobalCandidateChanged")
    );
    write(&rules_path, &old_rules);
    write(&manifest_path, &old_manifest);
    let applied = global::update(
        &profile,
        Some(&next),
        Some(&exe()),
        true,
        Some(preview_hash),
    )
    .unwrap();
    assert_eq!(applied.status, "passed");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-12"
    );
    assert!(!profile.join(".highgrade/global/releases/v0-2-11").exists());
    assert_eq!(fs::read(profile.join("personal.txt")).unwrap(), b"keep");
}
// highgrade: HG-RW-S12
#[test]
fn update_preserves_foreign_file_in_old_release() {
    let profile = temp();
    global::install(&profile, &source(), &exe()).unwrap();
    let foreign = profile.join(".highgrade/global/releases/v0-2-11/personal.txt");
    write(&foreign, b"keep");
    let next = candidate();
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    let applied =
        global::update(&profile, Some(&next), Some(&exe()), true, Some(fingerprint)).unwrap();
    assert_eq!(applied.status, "warning");
    assert!(
        applied
            .findings
            .iter()
            .any(|finding| finding["code"] == "OldReleaseCleanupPending")
    );
    assert_eq!(fs::read(&foreign).unwrap(), b"keep");
    let old_release = profile.join(".highgrade/global/releases/v0-2-11");
    assert!(old_release.join("journal.json").exists());
    assert!(!old_release.join("rules.md").exists());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-12"
    );
    fs::remove_file(&foreign).unwrap();
    let following = candidate();
    let manifest_path = following.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["release"] = json!("v0-2-13");
    write(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    let preview = global::update(&profile, Some(&following), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    let applied = global::update(
        &profile,
        Some(&following),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert_eq!(applied.status, "passed", "{:?}", applied.findings);
    assert!(!old_release.exists());
}
#[test]
fn global_update_rejects_manual_rollback_option() {
    let output = Command::new(exe())
        .args([
            "global-update",
            "--profile",
            "unused",
            "--rollback",
            "v0-2-8",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "failed");
    assert!(
        report["findings"][0]["message"]
            .as_str()
            .unwrap()
            .contains("неизвестный параметр")
    );
}
#[test]
fn legacy_deploy_migrates_to_approve_push_and_cleans_old_release() {
    let profile = legacy_deploy_profile();
    let legacy = profile.join(".agents/skills/deploy/SKILL.md");
    let approve = profile.join(".agents/skills/highgrade-approve/SKILL.md");
    let push = profile.join(".agents/skills/highgrade-push/SKILL.md");
    let original = fs::read(&legacy).unwrap();
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    #[cfg(windows)]
    {
        let temp_active = profile.join(format!(
            ".highgrade/global/active.{}.part",
            std::process::id()
        ));
        write(&temp_active, b"block pointer replacement");
        assert!(
            global::update(
                &profile,
                Some(&source()),
                Some(&exe()),
                true,
                Some(fingerprint),
            )
            .is_err()
        );
        assert_eq!(fs::read(&legacy).unwrap(), original);
        assert!(!approve.exists());
        assert!(!push.exists());
        assert_eq!(
            global::status(&profile).unwrap().measurements[0]["release"],
            "v0-2-5"
        );
        fs::remove_file(temp_active).unwrap();
    }
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert!(approve.is_file());
    assert!(push.is_file());
    assert!(!legacy.exists());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-11"
    );
    assert!(!profile.join(".highgrade/global/releases/v0-2-5").exists());
    assert_ne!(fs::read(&approve).unwrap(), original);
}
// highgrade: HG-RW-S07
#[test]
fn v026_deploy_migrates_to_two_routes_and_removes_old_release() {
    let profile = deploy_profile();
    let old = profile.join(".agents/skills/highgrade-deploy/SKILL.md");
    let approve = profile.join(".agents/skills/highgrade-approve/SKILL.md");
    let push = profile.join(".agents/skills/highgrade-push/SKILL.md");
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert!(approve.is_file());
    assert!(push.is_file());
    assert!(!old.exists());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-11"
    );
    assert!(!profile.join(".highgrade/global/releases/v0-2-6").exists());
}
#[cfg(windows)]
#[test]
fn locked_v026_router_is_inactive_after_switch() {
    use std::os::windows::fs::OpenOptionsExt;
    let profile = deploy_profile();
    let old = profile.join(".agents/skills/highgrade-deploy/SKILL.md");
    let handle = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&old)
        .unwrap();
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    let applied = global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert_eq!(applied.status, "warning");
    assert!(old.is_file());
    assert!(
        profile
            .join(".agents/skills/highgrade-approve/SKILL.md")
            .is_file()
    );
    assert!(
        profile
            .join(".agents/skills/highgrade-push/SKILL.md")
            .is_file()
    );
    let status = global::status(&profile).unwrap();
    assert_eq!(status.measurements[0]["release"], "v0-2-11");
    assert!(
        status
            .findings
            .iter()
            .any(|f| f["code"] == "InactiveRouterRemains")
    );
    drop(handle);
}
fn six_skill_profile() -> PathBuf {
    let profile = temp();
    let release = "v0-2-1";
    let mut checksums = BTreeMap::new();
    for (rel, _) in
        serde_json::from_slice::<Value>(&fs::read(source().join("manifest.json")).unwrap()).unwrap()
            ["files"]
            .as_object()
            .unwrap()
    {
        if [
            "procedures/approve.md",
            "procedures/push.md",
            "skills/highgrade-approve/SKILL.md",
            "skills/highgrade-push/SKILL.md",
        ]
        .contains(&rel.as_str())
        {
            continue;
        }
        let dest = if let Some(name) = rel
            .strip_prefix("skills/")
            .and_then(|path| path.strip_suffix("/SKILL.md"))
        {
            format!(".agents/skills/{name}/SKILL.md")
        } else {
            format!(".highgrade/global/releases/{release}/{rel}")
        };
        let data = fs::read(source().join(rel)).unwrap();
        write(&profile.join(&dest), &data);
        checksums.insert(dest, hash(&data));
    }
    let exe_rel = format!(
        ".highgrade/global/releases/{release}/highgrade{}",
        std::env::consts::EXE_SUFFIX
    );
    let exe_data = fs::read(exe()).unwrap();
    write(&profile.join(&exe_rel), &exe_data);
    checksums.insert(exe_rel, hash(&exe_data));
    let journal = serde_json::to_vec_pretty(&json!({
        "schema_version": 3,
        "release": release,
        "manifest_sha256": hash(&fs::read(source().join("manifest.json")).unwrap()),
        "files": checksums
    }))
    .unwrap();
    write(
        &profile.join(format!(".highgrade/global/releases/{release}/journal.json")),
        &journal,
    );
    write(
        &profile.join(".highgrade/global/active.json"),
        &serde_json::to_vec_pretty(&json!({
            "schema_version": 3,
            "status": "connected",
            "release": release,
            "journal_sha256": hash(&journal)
        }))
        .unwrap(),
    );
    profile
}
fn legacy_deploy_profile() -> PathBuf {
    old_deploy_profile("v0-2-5", "deploy")
}
fn deploy_profile() -> PathBuf {
    old_deploy_profile("v0-2-6", "highgrade-deploy")
}
fn old_deploy_profile(release: &str, skill_name: &str) -> PathBuf {
    let profile = temp();
    let mut checksums = BTreeMap::new();
    let manifest_bytes = fs::read(source().join("manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for (rel, _) in manifest["files"].as_object().unwrap() {
        if [
            "procedures/approve.md",
            "procedures/push.md",
            "skills/highgrade-approve/SKILL.md",
            "skills/highgrade-push/SKILL.md",
        ]
        .contains(&rel.as_str())
        {
            continue;
        }
        let dest = if let Some(name) = rel
            .strip_prefix("skills/")
            .and_then(|path| path.strip_suffix("/SKILL.md"))
        {
            format!(".agents/skills/{name}/SKILL.md")
        } else {
            format!(".highgrade/global/releases/{release}/{rel}")
        };
        let data = fs::read(source().join(rel)).unwrap();
        write(&profile.join(&dest), &data);
        checksums.insert(dest, hash(&data));
    }
    let procedure = b"# Legacy deploy procedure\n";
    let procedure_rel = format!(".highgrade/global/releases/{release}/procedures/deploy.md");
    write(&profile.join(&procedure_rel), procedure);
    checksums.insert(procedure_rel, hash(procedure));
    let legacy = format!(
        "---\nname: {skill_name}\ndescription: legacy release route\n---\n\n# Legacy deploy\n"
    );
    for rel in [
        format!(".agents/skills/{skill_name}/SKILL.md"),
        format!(".highgrade/global/releases/{release}/skills/{skill_name}/SKILL.md"),
    ] {
        write(&profile.join(&rel), legacy.as_bytes());
        checksums.insert(rel, hash(legacy.as_bytes()));
    }
    let exe_rel = format!(
        ".highgrade/global/releases/{release}/highgrade{}",
        std::env::consts::EXE_SUFFIX
    );
    let exe_data = fs::read(exe()).unwrap();
    write(&profile.join(&exe_rel), &exe_data);
    checksums.insert(exe_rel, hash(&exe_data));
    let journal = serde_json::to_vec_pretty(&json!({
        "schema_version": 3,
        "release": release,
        "manifest_sha256": hash(&manifest_bytes),
        "files": checksums
    }))
    .unwrap();
    write(
        &profile.join(format!(".highgrade/global/releases/{release}/journal.json")),
        &journal,
    );
    write(
        &profile.join(".highgrade/global/active.json"),
        &serde_json::to_vec_pretty(&json!({
            "schema_version": 3,
            "status": "connected",
            "release": release,
            "journal_sha256": hash(&journal)
        }))
        .unwrap(),
    );
    profile
}
fn six_skill_profile_with_crlf_routers() -> PathBuf {
    let profile = six_skill_profile();
    let release = "v0-2-1";
    let journal_path = profile.join(format!(".highgrade/global/releases/{release}/journal.json"));
    let mut journal: Value = serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
    for name in [
        "highgrade-init",
        "highgrade-task",
        "highgrade-spec",
        "highgrade-work",
        "highgrade-clear",
        "highgrade-update",
    ] {
        let rel = format!(".agents/skills/{name}/SKILL.md");
        let path = profile.join(&rel);
        let lf = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        let crlf = lf.replace("\r\n", "\n").replace('\n', "\r\n").into_bytes();
        write(&path, &crlf);
        journal["files"][&rel] = json!(hash(&crlf));
    }
    let journal_bytes = serde_json::to_vec_pretty(&journal).unwrap();
    write(&journal_path, &journal_bytes);
    let active_path = profile.join(".highgrade/global/active.json");
    let mut active: Value = serde_json::from_slice(&fs::read(&active_path).unwrap()).unwrap();
    active["journal_sha256"] = json!(hash(&journal_bytes));
    write(&active_path, &serde_json::to_vec_pretty(&active).unwrap());
    profile
}

#[test]
fn six_skill_release_preserves_owned_crlf_routers() {
    let profile = six_skill_profile_with_crlf_routers();
    let init = profile.join(".agents/skills/highgrade-init/SKILL.md");
    let old_bytes = fs::read(&init).unwrap();
    assert!(old_bytes.windows(2).any(|bytes| bytes == b"\r\n"));
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert_eq!(fs::read(&init).unwrap(), old_bytes);
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-11"
    );
    assert!(!profile.join(".highgrade/global/releases/v0-2-1").exists());
}

#[test]
fn six_skill_release_updates_to_approve_push_and_cleans_old_release() {
    let profile = six_skill_profile();

    let approve = profile.join(".agents/skills/highgrade-approve/SKILL.md");
    let push = profile.join(".agents/skills/highgrade-push/SKILL.md");
    write(&approve, b"foreign skill");
    assert!(
        global::update(&profile, Some(&source()), Some(&exe()), false, None)
            .unwrap_err()
            .contains("GlobalRouterIncompatible: highgrade-approve")
    );
    assert_eq!(fs::read(&approve).unwrap(), b"foreign skill");
    fs::remove_file(&approve).unwrap();

    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-11"
    );
    assert!(approve.is_file());
    assert!(push.is_file());
    assert!(!profile.join(".highgrade/global/releases/v0-2-1").exists());
}
#[test]
fn failed_stage_does_not_leave_new_routers() {
    let profile = six_skill_profile();
    let blocked = profile.join(".highgrade/global/releases/v0-2-11/rules.md");
    write(&blocked, b"foreign content");
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    assert!(
        global::update(
            &profile,
            Some(&source()),
            Some(&exe()),
            true,
            Some(fingerprint),
        )
        .is_err()
    );
    assert!(
        !profile
            .join(".agents/skills/highgrade-approve/SKILL.md")
            .exists()
    );
    assert!(
        !profile
            .join(".agents/skills/highgrade-push/SKILL.md")
            .exists()
    );
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
    fs::remove_file(blocked).unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-11"
    );
}
#[test]
fn failed_stage_preserves_foreign_candidate_journal() {
    let profile = six_skill_profile();
    let foreign = profile.join(".highgrade/global/releases/v0-2-11/journal.json");
    write(&foreign, b"foreign journal");
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    let error = global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
    )
    .unwrap_err();
    assert!(error.contains("GlobalCandidateJournalChanged"));
    assert_eq!(fs::read(&foreign).unwrap(), b"foreign journal");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
}
#[cfg(windows)]
#[test]
fn failed_forward_pointer_change_allows_a_different_candidate() {
    let profile = six_skill_profile();
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    let temp_active = profile.join(format!(
        ".highgrade/global/active.{}.part",
        std::process::id()
    ));
    write(&temp_active, b"block pointer replacement");
    assert!(
        global::update(
            &profile,
            Some(&source()),
            Some(&exe()),
            true,
            Some(fingerprint),
        )
        .is_err()
    );
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
    assert!(
        !profile
            .join(".agents/skills/highgrade-approve/SKILL.md")
            .exists()
    );
    assert!(
        !profile
            .join(".agents/skills/highgrade-push/SKILL.md")
            .exists()
    );
    fs::remove_file(temp_active).unwrap();
    let different = candidate();
    assert!(global::update(&profile, Some(&different), Some(&exe()), false, None).is_ok());
}
#[test]
fn global_install_rejects_hash_and_foreign_router_without_activation() {
    let profile = temp();
    write(
        &profile.join(".agents/skills/highgrade-work/SKILL.md"),
        b"foreign owner",
    );
    assert!(
        global::install(&profile, &source(), &exe())
            .unwrap_err()
            .contains("GlobalSkillConflict")
    );
    assert!(!profile.join(".highgrade/global/active.json").exists());
    assert_eq!(
        fs::read(profile.join(".agents/skills/highgrade-work/SKILL.md")).unwrap(),
        b"foreign owner"
    );
    let profile = temp();
    let bad = candidate();
    write(&bad.join("rules.md"), b"changed after manifest");
    assert!(
        global::install(&profile, &bad, &exe())
            .unwrap_err()
            .contains("GlobalManifestHashMismatch")
    );
    assert!(!profile.join(".highgrade/global/active.json").exists());
}
// highgrade: HG-PD-S10
#[test]
fn supported_adapter_does_not_claim_semantic_readiness() {
    let project = temp();
    write(
        &project.join(".highgrade/project/INSTRUCTIONS.md"),
        b"---\nhighgrade_project_schema: 1\n---\n# TODO: commands and permissions unknown\n",
    );
    let report = highgrade::doctor(&project).unwrap();
    let instruction = report
        .measurements
        .iter()
        .find(|m| m["project_instruction"] == "compatible")
        .unwrap();
    assert_eq!(instruction["compatibility_scope"], "schema-only");
    assert_eq!(instruction["semantic_compatibility"], "not_assessed");
    assert_eq!(instruction["schema_version"], 1);
}

// highgrade: HG-PD-S12
#[test]
fn global_update_preserves_project_adaptation_in_profile() {
    let profile = temp();
    let adapter = profile.join("projects/customer/.highgrade/project/INSTRUCTIONS.md");
    let bytes = b"---\nhighgrade_project_schema: 1\n---\nAccepted exception: no activation. Keep project permissions.\n";
    write(&adapter, bytes);
    assert_eq!(
        global::install(&profile, &source(), &exe()).unwrap().status,
        "passed"
    );
    let next = candidate();
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    let updated =
        global::update(&profile, Some(&next), Some(&exe()), true, Some(fingerprint)).unwrap();
    assert_eq!(updated.status, "passed");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-12"
    );
    assert_eq!(fs::read(adapter).unwrap(), bytes);
}

// highgrade: HG-PD-S11
#[test]
fn unsupported_project_adapter_is_a_failure() {
    let project = temp();
    let adapter = project.join(".highgrade/project/INSTRUCTIONS.md");
    for bytes in [
        b"# no schema\n".as_slice(),
        b"---\nhighgrade_project_schema: 99\n---\nKeep this text\n".as_slice(),
    ] {
        write(&adapter, bytes);
        let doctor = highgrade::doctor(&project).unwrap();
        assert!(
            doctor
                .findings
                .iter()
                .any(|f| f["code"] == "ProjectInstructionSchemaUnsupported")
        );
        assert!(
            !doctor
                .measurements
                .iter()
                .any(|m| m["project_instruction"] == "compatible")
        );
        assert_eq!(fs::read(&adapter).unwrap(), bytes);
    }
}
