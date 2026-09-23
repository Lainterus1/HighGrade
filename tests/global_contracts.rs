use highgrade::{global, hash};
use serde_json::{Value, json};
use std::{
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
    manifest["release"] = json!("v0-2-2");
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
        "v0-2-1"
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
        "v0-2-2"
    );
    let reverted = global::update(&profile, None, None, false, None, Some("v0-2-1")).unwrap();
    assert_eq!(reverted.status, "passed");
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "v0-2-1"
    );
    assert_eq!(fs::read(profile.join("personal.txt")).unwrap(), b"keep");
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
