use highgrade::{global, hash};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn temp() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "hg-global-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    p
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
    manifest["release"] = json!("v0-2-7");
    let rules = dst.join("rules.md");
    write(&rules, b"# New shared rules\n");
    manifest["files"]["rules.md"] = json!(hash(&fs::read(&rules).unwrap()));
    write(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    dst
}
#[test]
fn global_install_does_not_touch_project_and_checks_adapter() {
    let profile = temp();
    let project = temp();
    write(&project.join("README.md"), b"project-owned");
    let report = global::install(&profile, &source(), &exe()).unwrap();
    assert_eq!(report.status, "passed");
    for name in ["init", "task", "spec", "work", "clear", "update"] {
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
        profile
            .join(".agents/skills/highgrade-deploy/SKILL.md")
            .is_file()
    );
    assert!(
        !project
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
#[test]
fn global_update_preview_switch_and_rollback_keep_unrelated_files() {
    let profile = temp();
    write(&profile.join("personal.txt"), b"keep");
    global::install(&profile, &source(), &exe()).unwrap();
    let next = candidate();
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None, None).unwrap();
    assert_eq!(preview.status, "unknown");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    let preview_hash = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    assert!(
        global::update(
            &profile,
            Some(&next),
            Some(&exe()),
            true,
            Some("wrong"),
            None
        )
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
            None
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
        None,
    )
    .unwrap();
    assert_eq!(applied.status, "passed");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-7"
    );
    let reverted = global::update(&profile, None, None, false, None, Some("v0-2-6")).unwrap();
    assert_eq!(reverted.status, "passed");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    assert_eq!(fs::read(profile.join("personal.txt")).unwrap(), b"keep");
}
#[test]
fn legacy_deploy_renames_and_rolls_back() {
    let profile = legacy_deploy_profile();
    let legacy = profile.join(".agents/skills/deploy/SKILL.md");
    let new_router = profile.join(".agents/skills/highgrade-deploy/SKILL.md");
    let original = fs::read(&legacy).unwrap();
    let preview =
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None).unwrap();
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
                None,
            )
            .is_err()
        );
        assert_eq!(fs::read(&legacy).unwrap(), original);
        assert!(!new_router.exists());
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
        None,
    )
    .unwrap();
    assert!(new_router.is_file());
    assert!(!legacy.exists());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );

    global::update(&profile, None, None, false, None, Some("v0-2-5")).unwrap();
    assert_eq!(fs::read(&legacy).unwrap(), original);
    assert!(!new_router.exists());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-5"
    );

    write(
        &new_router,
        &fs::read(source().join("skills/highgrade-deploy/SKILL.md")).unwrap(),
    );
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-5"
    );
    assert_eq!(global::status(&profile).unwrap().status, "warning");
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
        None,
    )
    .unwrap();
    assert!(!legacy.exists());
    write(&legacy, &original);
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    assert_eq!(global::status(&profile).unwrap().status, "warning");
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
        if rel == "procedures/deploy.md" || rel == "skills/highgrade-deploy/SKILL.md" {
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
    let profile = temp();
    let release = "v0-2-5";
    let mut checksums = BTreeMap::new();
    let manifest_bytes = fs::read(source().join("manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for (rel, _) in manifest["files"].as_object().unwrap() {
        if rel == "skills/highgrade-deploy/SKILL.md" {
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
    let legacy = b"---\nname: deploy\ndescription: legacy release route\n---\n\n# Legacy deploy\n";
    for rel in [
        ".agents/skills/deploy/SKILL.md".to_string(),
        format!(".highgrade/global/releases/{release}/skills/deploy/SKILL.md"),
    ] {
        write(&profile.join(&rel), legacy);
        checksums.insert(rel, hash(legacy));
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
    let preview =
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
        None,
    )
    .unwrap();
    assert_eq!(fs::read(&init).unwrap(), old_bytes);
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    global::update(&profile, None, None, false, None, Some("v0-2-1")).unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
}

#[test]
fn six_skill_release_updates_to_deploy_and_can_roll_back() {
    let profile = six_skill_profile();
    let release = "v0-2-1";

    let deploy = profile.join(".agents/skills/highgrade-deploy/SKILL.md");
    write(&deploy, b"foreign skill");
    assert!(
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None)
            .unwrap_err()
            .contains("GlobalRouterIncompatible: highgrade-deploy")
    );
    assert_eq!(fs::read(&deploy).unwrap(), b"foreign skill");
    fs::remove_file(&deploy).unwrap();

    let preview =
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
        None,
    )
    .unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    assert!(deploy.is_file());
    global::update(&profile, None, None, false, None, Some(release)).unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        release
    );
    assert!(!deploy.exists());
    global::update(&profile, None, None, false, None, Some("v0-2-6")).unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    assert!(deploy.is_file());
}
#[test]
fn failed_stage_does_not_leave_a_deploy_router() {
    let profile = six_skill_profile();
    let blocked = profile.join(".highgrade/global/releases/v0-2-6/rules.md");
    write(&blocked, b"foreign content");
    let preview =
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None).unwrap();
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
            None
        )
        .is_err()
    );
    assert!(
        !profile
            .join(".agents/skills/highgrade-deploy/SKILL.md")
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
        None,
    )
    .unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
}
#[cfg(windows)]
#[test]
fn failed_forward_pointer_change_allows_a_different_candidate() {
    let profile = six_skill_profile();
    let preview =
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None).unwrap();
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
            None
        )
        .is_err()
    );
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
    assert!(
        !profile
            .join(".agents/skills/highgrade-deploy/SKILL.md")
            .exists()
    );
    fs::remove_file(temp_active).unwrap();
    let different = candidate();
    assert!(global::update(&profile, Some(&different), Some(&exe()), false, None, None).is_ok());
}
#[cfg(windows)]
#[test]
fn router_cleanup_warning_and_pointer_failure_preserve_active_release() {
    let profile = six_skill_profile();
    let preview =
        global::update(&profile, Some(&source()), Some(&exe()), false, None, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        Some(fingerprint),
        None,
    )
    .unwrap();
    let deploy = profile.join(".agents/skills/highgrade-deploy/SKILL.md");
    use std::os::windows::fs::OpenOptionsExt;
    let handle = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&deploy)
        .unwrap();
    let rolled_back = global::update(&profile, None, None, false, None, Some("v0-2-1")).unwrap();
    assert_eq!(rolled_back.status, "warning");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
    assert!(deploy.exists());
    drop(handle);
    fs::remove_file(&deploy).unwrap();
    let temp_active = profile.join(format!(
        ".highgrade/global/active.{}.part",
        std::process::id()
    ));
    write(&temp_active, b"block pointer replacement");
    assert!(global::update(&profile, None, None, false, None, Some("v0-2-6")).is_err());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
    assert!(!deploy.exists());
    fs::remove_file(temp_active).unwrap();
    global::update(&profile, None, None, false, None, Some("v0-2-6")).unwrap();
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-6"
    );
    assert!(deploy.is_file());
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
#[test]
fn unsupported_project_adapter_is_a_failure() {
    let project = temp();
    write(
        &project.join(".highgrade/project/INSTRUCTIONS.md"),
        b"# no schema\n",
    );
    let doctor = highgrade::doctor(&project).unwrap();
    assert!(
        doctor
            .findings
            .iter()
            .any(|f| f["code"] == "ProjectInstructionSchemaUnsupported")
    );
}
