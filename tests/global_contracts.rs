mod support;
use highgrade::{global, hash};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::TestDir;

fn temp() -> TestDir {
    TestDir::new("hg-global-")
}
fn current_release() -> String {
    let manifest: Value =
        serde_json::from_slice(&fs::read(source().join("manifest.json")).unwrap()).unwrap();
    manifest["release"].as_str().unwrap().to_owned()
}
fn release_after(offset: u32) -> String {
    let current = current_release();
    let (stem, patch) = current.rsplit_once('-').unwrap();
    format!("{stem}-{}", patch.parse::<u32>().unwrap() + offset)
}
fn next_release() -> String {
    release_after(1)
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

fn change_candidate_skill(next: &Path) -> Vec<u8> {
    let rel = "skills/highgrade-work/SKILL.md";
    let mut bytes = fs::read(next.join(rel)).unwrap();
    bytes.extend_from_slice(b"\nCandidate instruction.\n");
    write(&next.join(rel), &bytes);
    let path = next.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    manifest["files"][rel] = json!(hash(&bytes));
    write(&path, &serde_json::to_vec_pretty(&manifest).unwrap());
    bytes
}

// highgrade: HG-0027-S1
#[test]
fn update_replaces_owned_skill_and_preserves_user_edit() {
    let profile = temp();
    global::install(&profile, &source(), &exe()).unwrap();
    let next = candidate();
    let expected = change_candidate_skill(&next);
    let skill = profile.join(".agents/skills/highgrade-work/SKILL.md");
    let before = fs::read(&skill).unwrap();
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None).unwrap();
    assert_eq!(fs::read(&skill).unwrap(), before);
    write(&skill, b"user edit");
    assert!(
        global::update(
            &profile,
            Some(&next),
            Some(&exe()),
            true,
            preview.measurements[0]["candidate_sha256"].as_str()
        )
        .is_err()
    );
    assert_eq!(fs::read(&skill).unwrap(), b"user edit");
    write(&skill, &before);
    global::update(
        &profile,
        Some(&next),
        Some(&exe()),
        true,
        preview.measurements[0]["candidate_sha256"].as_str(),
    )
    .unwrap();
    assert_eq!(fs::read(&skill).unwrap(), expected);
    assert_eq!(global::status(&profile).unwrap().status, "passed");
    assert!(
        !profile
            .join(".highgrade/global/skill-transaction.json")
            .exists()
    );
}

// highgrade: HG-0027-S2
#[test]
fn interrupted_skill_update_recovers_without_overwriting_user_edit() {
    let profile = temp();
    global::install(&profile, &source(), &exe()).unwrap();
    let rel = ".agents/skills/highgrade-work/SKILL.md";
    let skill = profile.join(rel);
    let before = fs::read(&skill).unwrap();
    let pointer: Value =
        serde_json::from_slice(&fs::read(profile.join(".highgrade/global/active.json")).unwrap())
            .unwrap();
    let after = b"candidate bytes";
    let tx = json!({"old_active":pointer,"candidate":next_release(),"before":{rel:before},"after":{rel:hash(after)}});
    let tx_path = profile.join(".highgrade/global/skill-transaction.json");
    write(&tx_path, &serde_json::to_vec_pretty(&tx).unwrap());
    write(&skill, after);
    assert!(
        global::status(&profile)
            .unwrap_err()
            .contains("GlobalSkillRecoveryRequired")
    );
    write(&skill, b"new user edit");
    assert!(
        global::recover(&profile)
            .unwrap_err()
            .contains("GlobalRouterChanged")
    );
    assert_eq!(fs::read(&skill).unwrap(), b"new user edit");
    assert!(tx_path.exists());
    write(&skill, after);
    assert_eq!(global::recover(&profile).unwrap().status, "passed");
    assert_eq!(fs::read(&skill).unwrap(), before);
    assert!(!tx_path.exists());
}

// highgrade: HG-0027-S2
#[test]
fn failed_candidate_stage_restores_changed_skill() {
    let profile = temp();
    global::install(&profile, &source(), &exe()).unwrap();
    let next = candidate();
    change_candidate_skill(&next);
    let skill = profile.join(".agents/skills/highgrade-work/SKILL.md");
    let before = fs::read(&skill).unwrap();
    let blocked = profile
        .join(".highgrade/global/releases")
        .join(next_release())
        .join("rules.md");
    write(&blocked, b"foreign content");
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None).unwrap();
    assert!(
        global::update(
            &profile,
            Some(&next),
            Some(&exe()),
            true,
            preview.measurements[0]["candidate_sha256"].as_str()
        )
        .is_err()
    );
    assert_eq!(fs::read(&skill).unwrap(), before);
    assert_eq!(fs::read(&blocked).unwrap(), b"foreign content");
    assert_eq!(global::status(&profile).unwrap().status, "passed");
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
fn candidate() -> TestDir {
    let dst = temp();
    copy_tree(&source(), &dst);
    let manifest_path = dst.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["release"] = json!(next_release());
    let rules = dst.join("rules.md");
    write(&rules, b"# New shared rules\n");
    manifest["files"]["rules.md"] = json!(hash(&fs::read(&rules).unwrap()));
    write(
        &manifest_path,
        &serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    dst
}
// highgrade: HG-0010-S1
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

// highgrade: HG-0020-S9, HG-0027-S3
#[test]
fn planner_router_is_portable() {
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

    let router = profile.join(".agents/skills/highgrade-planner/SKILL.md");
    let procedure = profile
        .join(".highgrade/global/releases")
        .join(manifest["release"].as_str().unwrap())
        .join("procedures/planner.md");
    assert_eq!(
        hash(&fs::read(&router).unwrap()),
        manifest["files"]["skills/highgrade-planner/SKILL.md"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        hash(&fs::read(&procedure).unwrap()),
        manifest["files"]["procedures/planner.md"].as_str().unwrap()
    );
    assert!(
        fs::read_to_string(&router)
            .unwrap()
            .contains("procedures/planner.md")
    );
    assert!(
        fs::read_to_string(&router)
            .unwrap()
            .contains("## План и продолжение")
    );
    assert_eq!(global::status(&profile).unwrap().status, "passed");
}

#[test]
fn planner_update_accepts_prior_eight_skill_release() {
    let profile = prior_skill_profile();
    assert_eq!(global::status(&profile).unwrap().status, "passed");
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let fingerprint = preview.measurements[0]["candidate_sha256"]
        .as_str()
        .unwrap();
    assert_eq!(
        global::update(
            &profile,
            Some(&source()),
            Some(&exe()),
            true,
            Some(fingerprint),
        )
        .unwrap()
        .status,
        "passed"
    );
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        current_release()
    );
    assert!(
        profile
            .join(".agents/skills/highgrade-planner/SKILL.md")
            .is_file()
    );
    assert!(
        profile
            .join(".highgrade/global/releases")
            .join(current_release())
            .join("procedures/planner.md")
            .is_file()
    );
    assert!(!profile.join(".highgrade/global/releases/v0-2-16").exists());
}

// highgrade: HG-0010-S4
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
            .join("references/tools/diagnostics.md"),
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

// highgrade: HG-0015-S1
#[test]
fn doctor_reports_unavailable_tool_without_claiming_version() {
    let project = temp();
    let output = Command::new(env!("CARGO_BIN_EXE_highgrade"))
        .args(["doctor", "--root", project.to_str().unwrap()])
        .env("PATH", "")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let tool = report["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["tool"] == "cargo")
        .unwrap();
    assert_eq!(tool["found_in_path"], false);
    assert_eq!(tool["executed"], false);
    assert_eq!(tool["version"], Value::Null);
    assert!(report["findings"].as_array().unwrap().iter().any(|entry| {
        entry["code"] == "DependencyMissing"
            && entry["location"] == "cargo"
            && entry["status"] == "unknown"
    }));
}
// highgrade: HG-0011-S9, HG-0036-S1
#[test]
fn global_update_preview_switch_and_cleanup_keep_unrelated_files() {
    let profile = temp();
    write(&profile.join("personal.txt"), b"keep");
    global::install(&profile, &source(), &exe()).unwrap();
    let next = candidate();
    let preview = global::update(&profile, Some(&next), Some(&exe()), false, None).unwrap();
    assert_eq!(preview.status, "unknown");
    assert!(
        preview
            .measurements
            .iter()
            .any(|m| m["validation"] == "passed"
                && m["decision"] == "required"
                && m["applied"] == false)
    );
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        current_release()
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
        next_release()
    );
    assert!(
        !profile
            .join(".highgrade/global/releases")
            .join(current_release())
            .exists()
    );
    assert_eq!(fs::read(profile.join("personal.txt")).unwrap(), b"keep");
}
// highgrade: HG-0011-S10
#[test]
fn update_preserves_foreign_file_in_old_release() {
    let profile = temp();
    global::install(&profile, &source(), &exe()).unwrap();
    let foreign = profile
        .join(".highgrade/global/releases")
        .join(current_release())
        .join("personal.txt");
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
    let old_release = profile
        .join(".highgrade/global/releases")
        .join(current_release());
    assert!(old_release.join("journal.json").exists());
    assert!(!old_release.join("rules.md").exists());
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        next_release()
    );
    fs::remove_file(&foreign).unwrap();
    let following = candidate();
    let manifest_path = following.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["release"] = json!(release_after(2));
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
        current_release()
    );
    assert!(!profile.join(".highgrade/global/releases/v0-2-5").exists());
    assert_ne!(fs::read(&approve).unwrap(), original);
}
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
        current_release()
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
    assert_eq!(status.measurements[0]["release"], current_release());
    assert!(
        status
            .findings
            .iter()
            .any(|f| f["code"] == "InactiveRouterRemains")
    );
    drop(handle);
}
fn prior_skill_profile() -> TestDir {
    profile_without_tools(false)
}
fn profile_without_tools(planner: bool) -> TestDir {
    profile_before_fixer(planner, false)
}
fn profile_before_fixer(planner: bool, tools: bool) -> TestDir {
    let profile = temp();
    let release = if tools {
        "v0-2-24"
    } else if planner {
        "v0-2-23"
    } else {
        "v0-2-16"
    };
    let manifest: Value =
        serde_json::from_slice(&fs::read(source().join("manifest.json")).unwrap()).unwrap();
    let mut checksums = BTreeMap::new();
    for (rel, _) in manifest["files"].as_object().unwrap() {
        if (!tools && rel.starts_with("references/tools/"))
            || rel == "procedures/fix.md"
            || rel == "references/tools/issues.md"
            || is_survey_material(rel)
        {
            continue;
        }
        if !planner
            && ["procedures/planner.md", "skills/highgrade-planner/SKILL.md"]
                .contains(&rel.as_str())
        {
            continue;
        }
        let data = fs::read(source().join(rel)).unwrap();
        let dest = if let Some(name) = rel
            .strip_prefix("skills/")
            .and_then(|path| path.strip_suffix("/SKILL.md"))
        {
            format!(".agents/skills/{name}/SKILL.md")
        } else {
            format!(".highgrade/global/releases/{release}/{rel}")
        };
        write(&profile.join(&dest), &data);
        checksums.insert(dest, hash(&data));
    }
    for rel in [
        "skills/highgrade-approve/SKILL.md",
        "skills/highgrade-push/SKILL.md",
    ] {
        let data = fs::read(source().join(rel)).unwrap();
        let dest = format!(".highgrade/global/releases/{release}/{rel}");
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
        "manifest_sha256": "previous-release-fixture",
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

fn six_skill_profile() -> TestDir {
    let profile = temp();
    let release = "v0-2-1";
    let mut checksums = BTreeMap::new();
    for (rel, _) in
        serde_json::from_slice::<Value>(&fs::read(source().join("manifest.json")).unwrap()).unwrap()
            ["files"]
            .as_object()
            .unwrap()
    {
        if rel.starts_with("references/tools/")
            || rel == "procedures/fix.md"
            || is_survey_material(rel)
        {
            continue;
        }
        if [
            "procedures/approve.md",
            "procedures/push.md",
            "skills/highgrade-approve/SKILL.md",
            "skills/highgrade-push/SKILL.md",
            "procedures/planner.md",
            "skills/highgrade-planner/SKILL.md",
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
fn legacy_deploy_profile() -> TestDir {
    old_deploy_profile("v0-2-5", "deploy")
}
fn deploy_profile() -> TestDir {
    old_deploy_profile("v0-2-6", "highgrade-deploy")
}
fn old_deploy_profile(release: &str, skill_name: &str) -> TestDir {
    let profile = temp();
    let mut checksums = BTreeMap::new();
    let manifest_bytes = fs::read(source().join("manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    for (rel, _) in manifest["files"].as_object().unwrap() {
        if rel.starts_with("references/tools/")
            || rel == "procedures/fix.md"
            || is_survey_material(rel)
        {
            continue;
        }
        if [
            "procedures/approve.md",
            "procedures/push.md",
            "skills/highgrade-approve/SKILL.md",
            "skills/highgrade-push/SKILL.md",
            "procedures/planner.md",
            "skills/highgrade-planner/SKILL.md",
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
fn six_skill_profile_with_crlf_routers() -> TestDir {
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
        current_release()
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
        current_release()
    );
    assert!(approve.is_file());
    assert!(push.is_file());
    assert!(!profile.join(".highgrade/global/releases/v0-2-1").exists());
}
#[test]
fn failed_stage_does_not_leave_new_routers() {
    let profile = six_skill_profile();
    let blocked = profile
        .join(".highgrade/global/releases")
        .join(current_release())
        .join("rules.md");
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
    assert!(
        !profile
            .join(".agents/skills/highgrade-planner/SKILL.md")
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
        current_release()
    );
}
#[test]
fn failed_stage_preserves_foreign_candidate_journal() {
    let profile = six_skill_profile();
    let foreign = profile
        .join(".highgrade/global/releases")
        .join(current_release())
        .join("journal.json");
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
    assert!(
        !profile
            .join(".agents/skills/highgrade-planner/SKILL.md")
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
// highgrade: HG-0010-S10
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

// highgrade: HG-0010-S12
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
        next_release()
    );
    assert_eq!(fs::read(adapter).unwrap(), bytes);
}

// highgrade: HG-0010-S11
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

// highgrade: HG-0040-S2, HG-0042-S2
#[test]
fn shipped_tool_references_resolve_without_repository_files() {
    use pulldown_cmark::{Event, Parser, Tag};
    let manifest: Value =
        serde_json::from_slice(&fs::read(source().join("manifest.json")).unwrap()).unwrap();
    let files = manifest["files"].as_object().unwrap();
    let package = temp();
    // Recreate just the shipped files, with no development checkout available.
    for rel in files.keys() {
        let dest = package.join(rel);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(source().join(rel), dest).unwrap();
    }
    for name in [
        "specifications",
        "verification",
        "installation",
        "diagnostics",
    ] {
        assert!(
            package
                .join(format!("references/tools/{name}.md"))
                .is_file()
        );
    }
    let package_root = fs::canonicalize(&package).unwrap();
    for rel in files.keys().filter(|p| {
        p.starts_with("procedures/") || p.starts_with("references/") || *p == "rules.md"
    }) {
        let doc = fs::read_to_string(package.join(rel)).unwrap();
        for event in Parser::new(&doc) {
            if let Event::Start(Tag::Link { dest_url, .. }) = event {
                if dest_url.contains("://") {
                    continue;
                }
                let (file, anchor) = dest_url.split_once('#').unwrap_or((&dest_url, ""));
                let target = if file.is_empty() {
                    package.join(rel)
                } else {
                    package.join(rel).parent().unwrap().join(file)
                };
                let resolved = fs::canonicalize(&target)
                    .unwrap_or_else(|e| panic!("{rel} -> {dest_url}: {e}"));
                assert!(
                    resolved.starts_with(&package_root),
                    "{rel} -> {dest_url} leaves release"
                );
                if !anchor.is_empty() {
                    let target_text = fs::read_to_string(&target).unwrap();
                    let headings: Vec<_> = target_text
                        .lines()
                        .filter(|l| l.starts_with('#'))
                        .map(|l| {
                            l.trim_start_matches('#')
                                .trim()
                                .to_lowercase()
                                .chars()
                                .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-')
                                .map(|c| if c.is_whitespace() { '-' } else { c })
                                .collect::<String>()
                        })
                        .collect();
                    assert!(
                        headings.iter().any(|h| h == anchor),
                        "{rel} -> {dest_url}: missing heading"
                    );
                }
            }
        }
    }
}

// highgrade: HG-0040-S2
#[test]
fn nine_skill_release_upgrades_to_thematic_tool_references() {
    let profile = profile_without_tools(true);
    assert_eq!(global::status(&profile).unwrap().status, "passed");
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let report = global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        preview.measurements[0]["candidate_sha256"].as_str(),
    )
    .unwrap();
    assert_eq!(report.status, "passed");
    assert_eq!(global::status(&profile).unwrap().status, "passed");
    for name in [
        "specifications",
        "verification",
        "installation",
        "diagnostics",
    ] {
        let rel = format!("references/tools/{name}.md");
        assert_eq!(
            fs::read(profile.join(format!(
                ".highgrade/global/releases/{}/{}",
                current_release(),
                rel
            )))
            .unwrap(),
            fs::read(source().join(rel)).unwrap()
        );
    }
}

// highgrade: HG-0042-S2
#[test]
fn thematic_tools_release_upgrades_to_fixer_materials() {
    let profile = profile_before_fixer(true, true);
    assert_eq!(global::status(&profile).unwrap().status, "passed");
    let preview = global::update(&profile, Some(&source()), Some(&exe()), false, None).unwrap();
    let applied = global::update(
        &profile,
        Some(&source()),
        Some(&exe()),
        true,
        preview.measurements[0]["candidate_sha256"].as_str(),
    )
    .unwrap();
    assert_eq!(applied.status, "passed");
    assert_eq!(global::status(&profile).unwrap().status, "passed");
    for rel in ["procedures/fix.md", "references/tools/issues.md"] {
        assert_eq!(
            fs::read(profile.join(format!(
                ".highgrade/global/releases/{}/{rel}",
                current_release()
            )))
            .unwrap(),
            fs::read(source().join(rel)).unwrap()
        );
    }
}

fn is_survey_material(rel: &str) -> bool {
    [
        "references/tools/survey.md",
        "references/survey-schema.json",
        "templates/survey.json",
    ]
    .contains(&rel)
}
