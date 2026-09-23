use highgrade::{hash, install::inventory, package, paths};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn temp() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "hg-package-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    p
}
fn source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bundle")
}
#[test]
fn bundled_skill_frontmatter_uses_safe_scalars() {
    for name in [
        "highgrade-init",
        "highgrade-work",
        "highgrade-refresh",
        "highgrade-update",
    ] {
        let bytes = fs::read(source().join(format!("skills/{name}/SKILL.md"))).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert!(
            lines.len() >= 4 && lines[0] == "---" && lines[3] == "---",
            "{name}"
        );
        assert_eq!(lines[1], format!("name: {name}"));
        let description = lines[2].strip_prefix("description: ").unwrap();
        assert!(!description.is_empty(), "{name}");
        assert!(
            description.starts_with('"')
                || description.starts_with('\'')
                || !description.contains(": "),
            "{name}: unquoted YAML description contains a colon"
        );
    }
}
fn source_release() -> String {
    let manifest: Value =
        serde_json::from_slice(&fs::read(source().join("manifest.json")).unwrap()).unwrap();
    manifest["release"].as_str().unwrap().to_owned()
}
fn next_release() -> String {
    let current = source_release();
    let (stem, patch) = current.rsplit_once('-').unwrap();
    format!("{stem}-{}", patch.parse::<u32>().unwrap() + 1)
}
fn write(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}
fn fixture() -> PathBuf {
    let root = temp();
    write(&root.join("AGENTS.md"), b"foreign project rules");
    write(&root.join("README.md"), b"project");
    let files = inventory(&paths::root(&root).unwrap(), "audit.json").unwrap();
    write(&root.join("audit.json"),serde_json::to_vec(&json!({"schema_version":1,"status":"approved","scope":"full-project","evidence":"synthetic fixture","files":files})).unwrap().as_slice());
    root
}
#[test]
fn full_install_keeps_project_files_and_repeats() {
    let root = fixture();
    let before = fs::read(root.join("AGENTS.md")).unwrap();
    package::install(&root, &source(), "audit.json").unwrap();
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        source_release()
    );
    for name in [
        "highgrade-init",
        "highgrade-work",
        "highgrade-refresh",
        "highgrade-update",
    ] {
        assert!(
            root.join(format!(".agents/skills/{name}/SKILL.md"))
                .exists()
        );
    }
    assert_eq!(before, fs::read(root.join("AGENTS.md")).unwrap());
    package::install(&root, &source(), "audit.json").unwrap();
}
#[test]
fn full_install_rejects_pending_audit_and_foreign_skill() {
    let root = fixture();
    write(&root.join("audit.json"), br#"{"status":"pending"}"#);
    assert!(package::install(&root, &source(), "audit.json").is_err());
    assert!(!root.join(".highgrade").exists());
    let root = fixture();
    write(
        &root.join(".agents/skills/highgrade-work/SKILL.md"),
        b"owner",
    );
    assert!(package::install(&root, &source(), "audit.json").is_err());
    assert_eq!(
        fs::read(root.join(".agents/skills/highgrade-work/SKILL.md")).unwrap(),
        b"owner"
    );
}
fn copy_tree(from: &Path, to: &Path) {
    for item in fs::read_dir(from).unwrap() {
        let item = item.unwrap();
        let name = item.file_name();
        let dest = to.join(name);
        if item.file_type().unwrap().is_dir() {
            fs::create_dir_all(&dest).unwrap();
            copy_tree(&item.path(), &dest)
        } else {
            write(&dest, &fs::read(item.path()).unwrap())
        }
    }
}
fn next_source() -> PathBuf {
    let dst = temp();
    copy_tree(&source(), &dst);
    let manifest_path = dst.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["release"] = json!(next_release());
    let rules = dst.join("rules.md");
    write(&rules, b"# New release\n");
    manifest["files"]["rules.md"] = json!(hash(&fs::read(&rules).unwrap()));
    write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap().as_slice(),
    );
    dst
}
fn candidate_exe(source: &Path) -> PathBuf {
    let path = source.join("candidate.exe");
    write(&path, &fs::read(env!("CARGO_BIN_EXE_highgrade")).unwrap());
    path
}
#[test]
fn project_exclusions_require_an_explicit_reason_and_fresh_audit() {
    let root = fixture();
    write(
        &root.join("local-cache/generated.txt"),
        b"not product source",
    );
    let config = root.join(".highgrade/project/inventory-exclusions.json");
    write(&config, &serde_json::to_vec(&json!({"schema_version":1,"entries":[{"path":"local-cache","reason":"Generated local tool state; auditor classified it separately"}]})).unwrap());
    let files = inventory(&paths::root(&root).unwrap(), "audit.json").unwrap();
    assert!(!files.contains_key("local-cache/generated.txt"));
    let receipt = root.join("audit.json");
    write(&receipt, &serde_json::to_vec(&json!({"schema_version":1,"status":"approved","scope":"full-project","evidence":"synthetic fixture","files":files})).unwrap());
    assert!(
        package::install(&root, &source(), "audit.json")
            .unwrap_err()
            .contains("AuditExclusionsStale")
    );
    let config_hash = hash(&fs::read(&config).unwrap());
    write(&receipt, &serde_json::to_vec(&json!({"schema_version":1,"status":"approved","scope":"full-project","evidence":"synthetic fixture","files":files,"exclusions_sha256":config_hash})).unwrap());
    write(&config, &serde_json::to_vec(&json!({"schema_version":1,"entries":[{"path":"local-cache","reason":"reason changed after audit"}]})).unwrap());
    assert!(
        package::install(&root, &source(), "audit.json")
            .unwrap_err()
            .contains("AuditExclusionsStale")
    );
    write(&config, &serde_json::to_vec(&json!({"schema_version":1,"entries":[{"path":"local-cache","reason":"Generated local tool state; auditor classified it separately"}]})).unwrap());
    package::install(&root, &source(), "audit.json").unwrap();
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        source_release()
    );
}
#[test]
fn update_rejects_manifest_cli_version_mismatch() {
    let root = fixture();
    package::install(&root, &source(), "audit.json").unwrap();
    let candidate = next_source();
    let manifest_path = candidate.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["cli_version"] = json!("0.1.1");
    write(&manifest_path, &serde_json::to_vec(&manifest).unwrap());
    let selected_exe = candidate_exe(&candidate);
    assert!(
        package::update(
            &root,
            Some(&candidate),
            Some(&selected_exe),
            None,
            false,
            None
        )
        .unwrap_err()
        .contains("CandidateExecutableVersionMismatch")
    );
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        source_release()
    );
}
#[test]
fn update_requires_fresh_explicit_decision_and_preserves_adaptation() {
    let root = fixture();
    package::install(&root, &source(), "audit.json").unwrap();
    write(&root.join(".highgrade/project/custom.txt"), b"keep me");
    let candidate = next_source();
    let selected_exe = candidate_exe(&candidate);
    let check = package::update(
        &root,
        Some(&candidate),
        Some(&selected_exe),
        None,
        false,
        None,
    )
    .unwrap();
    assert_eq!(check.status, "unknown");
    let current = package::active(&paths::root(&root).unwrap())
        .unwrap()
        .unwrap();
    let manifest_hash = hash(&fs::read(candidate.join("manifest.json")).unwrap());
    let decision = root.join(".highgrade/project/update-decision.json");
    let adaptation_hash = check.measurements[0]["adaptation_sha256"].as_str().unwrap();
    let project_hash = check.measurements[0]["project_sha256"].as_str().unwrap();
    let exe_hash = check.measurements[0]["candidate_exe_sha256"]
        .as_str()
        .unwrap();
    write(&decision,serde_json::to_vec(&json!({"status":"approved","from":current.0,"to":next_release(),"active_journal_sha256":current.1,"candidate_manifest_sha256":"wrong","candidate_exe_sha256":exe_hash,"adaptation_sha256":adaptation_hash,"project_sha256":project_hash,"evidence":"synthetic review"})).unwrap().as_slice());
    assert!(
        package::update(
            &root,
            Some(&candidate),
            Some(&selected_exe),
            Some(".highgrade/project/update-decision.json"),
            true,
            None
        )
        .is_err()
    );
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        source_release()
    );
    write(&decision,serde_json::to_vec(&json!({"status":"approved","from":current.0,"to":next_release(),"active_journal_sha256":current.1,"candidate_manifest_sha256":manifest_hash,"candidate_exe_sha256":exe_hash,"adaptation_sha256":adaptation_hash,"project_sha256":project_hash,"evidence":"synthetic review"})).unwrap().as_slice());
    let chosen_bytes = fs::read(&selected_exe).unwrap();
    write(&selected_exe, b"other binary after review");
    assert!(
        package::update(
            &root,
            Some(&candidate),
            Some(&selected_exe),
            Some(".highgrade/project/update-decision.json"),
            true,
            None
        )
        .is_err()
    );
    write(&selected_exe, &chosen_bytes);
    write(&root.join("AGENTS.md"), b"changed after review");
    assert!(
        package::update(
            &root,
            Some(&candidate),
            Some(&selected_exe),
            Some(".highgrade/project/update-decision.json"),
            true,
            None
        )
        .unwrap_err()
        .contains("DecisionInvalidOrStale")
    );
    write(&root.join("AGENTS.md"), b"foreign project rules");
    write(
        &root.join(".highgrade/project/custom.txt"),
        b"changed after review",
    );
    assert!(
        package::update(
            &root,
            Some(&candidate),
            Some(&selected_exe),
            Some(".highgrade/project/update-decision.json"),
            true,
            None
        )
        .unwrap_err()
        .contains("DecisionInvalidOrStale")
    );
    write(&root.join(".highgrade/project/custom.txt"), b"keep me");
    package::update(
        &root,
        Some(&candidate),
        Some(&selected_exe),
        Some(".highgrade/project/update-decision.json"),
        true,
        None,
    )
    .unwrap();
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        next_release()
    );
    assert_eq!(
        fs::read(root.join(".highgrade/project/custom.txt")).unwrap(),
        b"keep me"
    );
    assert_eq!(
        fs::read(root.join(format!(
            ".highgrade/releases/{}/highgrade.exe",
            next_release()
        )))
        .unwrap(),
        fs::read(&selected_exe).unwrap()
    );
    package::update(
        &root,
        None,
        None,
        None,
        false,
        Some(source_release().as_str()),
    )
    .unwrap();
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        source_release()
    );
}
#[test]
fn candidate_conflict_never_changes_active_release() {
    let root = fixture();
    package::install(&root, &source(), "audit.json").unwrap();
    let candidate = next_source();
    let selected_exe = candidate_exe(&candidate);
    write(
        &candidate.join("skills/highgrade-work/SKILL.md"),
        b"different router",
    );
    let mut m: Value =
        serde_json::from_slice(&fs::read(candidate.join("manifest.json")).unwrap()).unwrap();
    m["files"]["skills/highgrade-work/SKILL.md"] = json!(hash(b"different router"));
    write(
        &candidate.join("manifest.json"),
        serde_json::to_vec(&m).unwrap().as_slice(),
    );
    assert!(
        package::update(
            &root,
            Some(&candidate),
            Some(&selected_exe),
            None,
            false,
            None
        )
        .unwrap_err()
        .contains("SkillRouterIncompatible")
    );
    assert_eq!(
        package::active(&paths::root(&root).unwrap())
            .unwrap()
            .unwrap()
            .0,
        source_release()
    );
}
