use highgrade::{
    hash,
    inspect::inspect,
    install::{install, inventory, verify},
    paths,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn task_checkboxes_and_inline_code_are_not_references() {
    let root = dir();
    registry(&root);
    write(
        &root,
        "README.md",
        b"- [x] Completed\n- [ ] Pending\n\n`cwds=[root]`\n",
    );
    let report = inspect(&root, Some("registry.json"), None).unwrap();
    assert!(!has(&report, "UnresolvedReference"));
}

#[test]
fn cli_errors_keep_the_report_contract() {
    for args in [
        vec!["doctor", "--bad", "x"],
        vec!["--version", "extra"],
        vec!["inspect", "--root", "missing-highgrade-fixture"],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"], "failed");
        for key in ["operation", "findings", "measurements", "limitations"] {
            assert!(report.get(key).is_some());
        }
    }
}

#[test]
fn incomplete_temp_is_preserved_and_not_activated() {
    let (root, source) = fixture();
    install(&root, &source, "audit.json", Some(0)).unwrap_err();
    let partial = ".agents/skills/highgrade-init/SKILL.md.part";
    write(&root, partial, b"incomplete");
    assert!(
        install(&root, &source, "audit.json", None)
            .unwrap_err()
            .contains("PartialWriteConflict")
    );
    assert_eq!(fs::read(root.join(partial)).unwrap(), b"incomplete");
    assert!(!root.join(".highgrade/installed.json").exists());
}

#[test]
fn complete_temp_can_resume_publication() {
    let (root, source) = fixture();
    install(&root, &source, "audit.json", Some(0)).unwrap_err();
    let data = fs::read_to_string(source.join("SKILL.md"))
        .unwrap()
        .replace("{{release}}", "p0p2-prototype");
    write(
        &root,
        ".agents/skills/highgrade-init/SKILL.md.part",
        data.as_bytes(),
    );
    install(&root, &source, "audit.json", None).unwrap();
    verify(&paths::root(&root).unwrap()).unwrap();
}
fn dir() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "highgrade-test-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}
fn write(root: &Path, path: &str, data: &[u8]) {
    let p = root.join(path);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}
fn save(root: &Path, path: &str, value: &Value) {
    write(
        root,
        path,
        serde_json::to_string_pretty(value).unwrap().as_bytes(),
    );
}
fn b(current: u64) -> Value {
    json!({"unit":"bytes","agreed":true,"min":1,"baseline":current,"current":current,"ceiling":current+1000,"history":[],"review_trigger":"after architecture change"})
}
fn registry(root: &Path) -> Value {
    let mut docs = vec![];
    for (id, role, file) in [
        ("readme", "purpose-navigation", "README.md"),
        ("agents", "agent-rules", "AGENTS.md"),
        (
            "architecture",
            "current-architecture",
            "docs/ARCHITECTURE.md",
        ),
        ("engineering", "engineering-rules", "docs/ENGINEERING.md"),
        ("development", "commands-procedures", "docs/DEVELOPMENT.md"),
    ] {
        write(root, file, "# Заголовок\r\n\r\nТекст\r\n".as_bytes());
        docs.push(json!({"id":id,"role":role,"path":file,"budget":b(1000),"scope":["src"],"loading":"task"}));
    }
    let value = json!({"schema_version":1,"documents":docs,"route_budget":b(10000)});
    save(root, "registry.json", &value);
    value
}
fn has(r: &highgrade::Report, code: &str) -> bool {
    r.findings.iter().any(|f| f["code"] == code)
}
fn fixture() -> (PathBuf, PathBuf) {
    let root = dir();
    registry(&root);
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("p0p2-bundle");
    let files = inventory(&paths::root(&root).unwrap(), "audit.json").unwrap();
    save(
        &root,
        "audit.json",
        &json!({"schema_version":1,"status":"approved","scope":"full-project","evidence":"synthetic fixture only; not a real project audit","files":files}),
    );
    (root, source)
}

#[test]
fn measures_utf8_crlf_and_does_not_modify_inputs() {
    let root = dir();
    registry(&root);
    let before = inventory(&paths::root(&root).unwrap(), "unused.json").unwrap();
    let r = inspect(&root, Some("registry.json"), None).unwrap();
    assert_eq!(r.status, "passed");
    assert_eq!(
        r.measurements[0]["bytes"],
        fs::read(root.join("README.md")).unwrap().len()
    );
    assert_eq!(r.measurements[0]["lines"], 3);
    assert_eq!(
        before,
        inventory(&paths::root(&root).unwrap(), "unused.json").unwrap()
    );
}
#[test]
fn missing_budget_is_unknown_not_success() {
    let root = dir();
    let mut reg = registry(&root);
    reg["documents"][0]["budget"] = Value::Null;
    save(&root, "registry.json", &reg);
    let r = inspect(&root, Some("registry.json"), None).unwrap();
    assert!(has(&r, "BudgetNotAgreed"));
    assert_eq!(r.exit_code(), 2);
}
#[test]
fn exceeding_agreed_budget_warns_without_raising_it() {
    let root = dir();
    let mut reg = registry(&root);
    reg["documents"][0]["budget"] = b(1);
    save(&root, "registry.json", &reg);
    let before = fs::read(root.join("registry.json")).unwrap();
    let r = inspect(&root, Some("registry.json"), None).unwrap();
    assert!(has(&r, "BudgetExceeded"));
    assert_eq!(r.exit_code(), 0);
    assert_eq!(before, fs::read(root.join("registry.json")).unwrap());
}
#[test]
fn budget_history_cannot_skip_or_exceed_ceiling() {
    let root = dir();
    let mut reg = registry(&root);
    reg["documents"][0]["budget"]["current"] = json!(1200);
    save(&root, "registry.json", &reg);
    assert!(has(
        &inspect(&root, Some("registry.json"), None).unwrap(),
        "BudgetInvalid"
    ));
    reg["documents"][0]["budget"]["history"] =
        json!([{"from":1000,"to":2100,"reason":"grow"},{"from":2100,"to":1200,"reason":"shrink"}]);
    save(&root, "registry.json", &reg);
    assert!(has(
        &inspect(&root, Some("registry.json"), None).unwrap(),
        "BudgetInvalid"
    ));
}
#[test]
fn valid_budget_history_passes() {
    let root = dir();
    let mut reg = registry(&root);
    reg["documents"][0]["budget"]["current"] = json!(1200);
    reg["documents"][0]["budget"]["history"] = json!([{"from":1000,"to":1200,"reason":"new area"}]);
    save(&root, "registry.json", &reg);
    assert!(!has(
        &inspect(&root, Some("registry.json"), None).unwrap(),
        "BudgetInvalid"
    ));
}
#[test]
fn links_support_reference_unicode_parent_and_code_exclusion() {
    let root = dir();
    registry(&root);
    write(
        &root,
        "README.md",
        b"[doc][ref]\n\n[ref]: docs/ARCHITECTURE.md\n\n```md\n[not real](missing.md)\n```\n",
    );
    write(
        &root,
        "docs/ARCHITECTURE.md",
        "# Тема\n\n[назад](../README.md)\n[якорь](#тема)\n".as_bytes(),
    );
    let r = inspect(&root, Some("registry.json"), None).unwrap();
    assert!(!has(&r, "LinkInvalid"));
}
#[test]
fn broken_links_and_anchors_are_failures() {
    let root = dir();
    registry(&root);
    write(
        &root,
        "README.md",
        b"[x](absent.md)\n[x](docs/ARCHITECTURE.md#absent)\n",
    );
    assert!(has(
        &inspect(&root, Some("registry.json"), None).unwrap(),
        "LinkInvalid"
    ));
}
#[test]
fn unresolved_reference_is_not_silent() {
    let root = dir();
    registry(&root);
    write(&root, "README.md", b"[x][not-defined]\n");
    assert!(has(
        &inspect(&root, Some("registry.json"), None).unwrap(),
        "UnresolvedReference"
    ));
}
#[test]
fn traversal_and_ads_are_rejected() {
    let root = dir();
    for path in [
        "../outside",
        "D:/secret",
        "file:stream",
        "docs/../file",
        "a\\b",
        "CON",
        "x./y",
    ] {
        assert!(
            paths::safe(&paths::root(&root).unwrap(), path).is_err(),
            "{path}"
        );
    }
}
#[test]
fn missing_roles_duplicates_and_files_are_reported() {
    let root = dir();
    let mut reg = registry(&root);
    reg["documents"][1] = reg["documents"][0].clone();
    reg["documents"][2]["path"] = json!("missing.md");
    save(&root, "registry.json", &reg);
    let r = inspect(&root, Some("registry.json"), None).unwrap();
    for code in [
        "DuplicateId",
        "DuplicateRole",
        "RequiredRoleMissing",
        "DocumentUnreadable",
    ] {
        assert!(has(&r, code), "{code}");
    }
}
#[test]
fn new_area_is_unknown() {
    let root = dir();
    registry(&root);
    fs::create_dir(root.join("new-service")).unwrap();
    assert!(has(
        &inspect(&root, Some("registry.json"), None).unwrap(),
        "AreaUnclassified"
    ));
}
#[test]
fn scoped_inspection_selects_sources_and_reports_unmapped_scope() {
    let root = dir();
    registry(&root);
    let r = inspect(&root, Some("registry.json"), Some("src/module")).unwrap();
    assert!(!has(&r, "ScopeUnclassified"));
    let r = inspect(&root, Some("registry.json"), Some("other")).unwrap();
    assert!(has(&r, "ScopeUnclassified"));
}
#[test]
fn malformed_registry_and_multiple_defaults_do_not_fallback() {
    let root = dir();
    registry(&root);
    write(&root, "bad.json", b"{");
    assert!(inspect(&root, Some("bad.json"), None).is_err());
    save(&root, ".highgrade/project/documents.json", &json!({}));
    save(&root, ".mycodex/project/documents.json", &json!({}));
    assert!(inspect(&root, None, None).is_err());
}
#[test]
fn parent_scope_reads_child_documents() {
    let root = dir();
    let mut reg = registry(&root);
    for doc in reg["documents"].as_array_mut().unwrap() {
        doc["scope"] = json!(["src/auth"]);
    }
    reg["documents"][0]["loading"] = json!("entry");
    write(&root, "docs/ARCHITECTURE.md", b"[broken](missing.md)");
    save(&root, "registry.json", &reg);
    let r = inspect(&root, Some("registry.json"), Some("src")).unwrap();
    assert!(has(&r, "LinkInvalid"));
}
#[test]
fn entry_does_not_hide_unmapped_sibling() {
    let root = dir();
    let mut reg = registry(&root);
    for doc in reg["documents"].as_array_mut().unwrap() {
        doc["scope"] = json!(["src/auth"]);
    }
    reg["documents"][0]["loading"] = json!("entry");
    save(&root, "registry.json", &reg);
    write(&root, "src/auth/a.rs", b"");
    write(&root, "src/payments/b.rs", b"");
    let r = inspect(&root, Some("registry.json"), Some("src/payments")).unwrap();
    assert!(has(&r, "ScopeUnclassified"));
    assert!(
        r.findings
            .iter()
            .any(|f| f["code"] == "AreaUnclassified" && f["location"] == "src/payments")
    );
}
#[test]
fn colocated_document_does_not_cover_its_parent() {
    let root = dir();
    let mut reg = registry(&root);
    for doc in reg["documents"].as_array_mut().unwrap() {
        doc["scope"] = json!(["src/auth"]);
    }
    reg["documents"][2]["path"] = json!("src/ARCHITECTURE.md");
    write(&root, "src/ARCHITECTURE.md", b"# Architecture");
    write(&root, "src/payments/b.rs", b"");
    save(&root, "registry.json", &reg);
    let r = inspect(&root, Some("registry.json"), None).unwrap();
    assert!(
        r.findings
            .iter()
            .any(|f| f["code"] == "AreaUnclassified" && f["location"] == "src/payments")
    );
}
#[test]
fn install_requires_audit_before_writes() {
    let (root, source) = fixture();
    save(&root, "audit.json", &json!({"status":"pending"}));
    assert!(install(&root, &source, "audit.json", None).is_err());
    assert!(!root.join(".highgrade").exists());
}
#[test]
fn install_rejects_stale_audit() {
    let (root, source) = fixture();
    write(&root, "new.md", b"new");
    assert!(
        install(&root, &source, "audit.json", None)
            .unwrap_err()
            .contains("AuditStale")
    );
    assert!(!root.join(".highgrade").exists());
}
#[test]
fn install_preserves_agents_and_repeats_without_duplicates() {
    let (root, source) = fixture();
    let before = fs::read(root.join("AGENTS.md")).unwrap();
    install(&root, &source, "audit.json", None).unwrap();
    install(&root, &source, "audit.json", None).unwrap();
    assert_eq!(before, fs::read(root.join("AGENTS.md")).unwrap());
    verify(&paths::root(&root).unwrap()).unwrap();
}
#[test]
fn interruption_at_each_publication_boundary_resumes() {
    for step in 0..=3 {
        let (root, source) = fixture();
        assert!(
            install(&root, &source, "audit.json", Some(step))
                .unwrap_err()
                .contains("TestInterruption")
        );
        assert!(!root.join(".highgrade/installed.json").exists());
        install(&root, &source, "audit.json", None).unwrap();
        verify(&paths::root(&root).unwrap()).unwrap();
    }
}
#[test]
fn recovery_does_not_overwrite_foreign_edit() {
    let (root, source) = fixture();
    install(&root, &source, "audit.json", Some(1)).unwrap_err();
    let path = ".agents/skills/highgrade-init/SKILL.md";
    write(&root, path, b"owner change");
    assert!(install(&root, &source, "audit.json", None).is_err());
    assert_eq!(fs::read(root.join(path)).unwrap(), b"owner change");
    assert!(!root.join(".highgrade/installed.json").exists());
}
#[test]
fn conflicting_skill_is_not_adopted_even_if_equal() {
    let (root, source) = fixture();
    write(&root, ".agents/skills/highgrade-init/SKILL.md", b"owner");
    assert!(
        install(&root, &source, "audit.json", None)
            .unwrap_err()
            .contains("DestinationConflict")
    );
}
#[test]
fn modified_installed_file_is_detected() {
    let (root, source) = fixture();
    install(&root, &source, "audit.json", None).unwrap();
    write(
        &root,
        ".highgrade/releases/p0p2-prototype/rules.md",
        b"changed",
    );
    assert!(verify(&paths::root(&root).unwrap()).is_err());
}
#[test]
fn package_hash_failure_writes_nothing() {
    let (root, source) = fixture();
    let bad = dir();
    for name in ["manifest.json", "rules.md", "SKILL.md"] {
        write(&bad, name, &fs::read(source.join(name)).unwrap());
    }
    write(&bad, "rules.md", b"changed");
    assert!(
        install(&root, &bad, "audit.json", None)
            .unwrap_err()
            .contains("ManifestHashMismatch")
    );
    assert!(!root.join(".highgrade").exists());
}
#[test]
fn source_inventory_is_case_sensitive_and_stable() {
    let root = dir();
    write(&root, "data.txt", b"hello");
    let inv = inventory(&paths::root(&root).unwrap(), "audit.json").unwrap();
    assert_eq!(inv["data.txt"], hash(b"hello"));
}
#[cfg(windows)]
#[test]
fn junction_is_not_followed() {
    let root = dir();
    let outside = dir();
    let link = root.join("linked");
    let output = std::process::Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(paths::safe(&paths::root(&root).unwrap(), "linked/file").is_err());
    assert!(paths::root(&link).is_err());
}
