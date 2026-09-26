use highgrade::bootstrap::{MAX_ITEMS, MAX_OUTPUT_BYTES, MAX_READ_BYTES, bootstrap};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

// highgrade: HG-0036-S2
#[test]
fn bootstrap_distinguishes_adaptation_from_technical_failure() {
    let fixture = Fixture::new();
    fixture.write("guide.md", "# Alternative project documentation\n");
    let report = bootstrap(fixture.root()).unwrap();
    let state = report
        .measurements
        .iter()
        .find(|m| m.get("adaptation").is_some())
        .unwrap();
    assert_eq!(state["adaptation"], "required");
    assert_eq!(state["validation"], "passed");
    assert_eq!(state["applied"], false);
}

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "highgrade-bootstrap-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn root(&self) -> &Path {
        &self.0
    }
    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn canonical(&self) {
        self.write("README.md", "# Project\n[Agents](AGENTS.md) [Architecture](docs/ARCHITECTURE.md) [Engineering](docs/ENGINEERING.md) [Development](docs/DEVELOPMENT.md)\n");
        self.write(
            "AGENTS.md",
            "[Project instruction](.highgrade/project/INSTRUCTIONS.md)\n",
        );
        self.write("docs/ARCHITECTURE.md", "# Architecture\n");
        self.write("docs/ENGINEERING.md", "# Engineering\n");
        self.write("docs/DEVELOPMENT.md", "# Development\n");
        self.write(
            ".highgrade/project/INSTRUCTIONS.md",
            "[Registry](documents.json)\n",
        );
        self.registry("docs/ARCHITECTURE.md");
    }
    fn registry(&self, architecture: &str) {
        let pairs = [
            ("readme", "README.md", "purpose-navigation"),
            ("agents", "AGENTS.md", "agent-rules"),
            ("architecture", architecture, "current-architecture"),
            ("engineering", "docs/ENGINEERING.md", "engineering-rules"),
            ("development", "docs/DEVELOPMENT.md", "commands-procedures"),
        ];
        let documents: Vec<_> = pairs
            .iter()
            .map(|(id, path, role)| json!({"id":id,"path":path,"role":role,"loading":"entry","budget":{"unit":"bytes","agreed":true,"min":1,"baseline":10000,"current":10000,"ceiling":20000,"history":[],"review_trigger":"after the fixture run"}}))
            .collect();
        self.write(
            ".highgrade/project/documents.json",
            &json!({"schema_version":1,"documents":documents,"route_budget":{"unit":"bytes","agreed":true,"min":1,"baseline":50000,"current":50000,"ceiling":100000,"history":[],"review_trigger":"after the fixture run"}}).to_string(),
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        assert!(self.0.starts_with(std::env::temp_dir()));
        assert!(
            self.0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("highgrade-bootstrap-")
        );
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn row<'a>(report: &'a highgrade::Report, path: &str) -> &'a Value {
    report.measurements[0]["slots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["expected_path"] == path)
        .unwrap()
}
fn link_status(report: &highgrade::Report, from: &str, to: &str) -> String {
    report.measurements[0]["required_links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["from"] == from && link["to"] == to)
        .unwrap()["status"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn finding(report: &highgrade::Report, code: &str) -> bool {
    report.findings.iter().any(|row| row["code"] == code)
}

// highgrade: HG-0009-S4
#[test]
fn bootstrap_reports_all_slots_before_registry_exists() {
    let fixture = Fixture::new();
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(report.status, "failed");
    assert_eq!(report.measurements[0]["slots"].as_array().unwrap().len(), 7);
    assert_eq!(report.measurements[0]["candidate_scan"], "performed");
    assert_eq!(row(&report, "README.md")["path_state"], "absent");
    assert_eq!(
        row(&report, ".highgrade/project/documents.json")["candidate_status"],
        "not_found_in_scan"
    );
    assert!(finding(&report, "CanonicalPathMissing"));
    assert!(!finding(&report, "RegistryMissingOrAmbiguous"));
}

// highgrade: HG-0009-S16, HG-0009-S19
#[test]
fn canonical_project_has_separate_path_and_link_evidence() {
    let fixture = Fixture::new();
    fixture.canonical();
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(report.status, "passed");
    assert_eq!(report.measurements[0]["candidate_scan"], "not_needed");
    assert_eq!(report.measurements[0]["visited_entries"], 0);
    assert_eq!(
        row(&report, "docs/ARCHITECTURE.md")["path_state"],
        "present"
    );
    assert_eq!(
        report.measurements[0]["required_links"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    for (from, to) in [
        ("README.md", "AGENTS.md"),
        ("README.md", "docs/ARCHITECTURE.md"),
        ("README.md", "docs/ENGINEERING.md"),
        ("README.md", "docs/DEVELOPMENT.md"),
        ("AGENTS.md", ".highgrade/project/INSTRUCTIONS.md"),
        (
            ".highgrade/project/INSTRUCTIONS.md",
            ".highgrade/project/documents.json",
        ),
    ] {
        assert_eq!(link_status(&report, from, to), "present", "{from} -> {to}");
    }
    let connected = highgrade::inspect::inspect(fixture.root(), None, None).unwrap();
    assert_eq!(connected.status, "passed");
    assert!(connected.findings.is_empty());
    assert!(
        connected
            .limitations
            .iter()
            .any(|item| item.contains("Структурная проверка, не смысловой аудит"))
    );
}

#[test]
fn complete_project_still_checks_exclusion_input_without_walking_tree() {
    let fixture = Fixture::new();
    fixture.canonical();
    fixture.write(
        ".highgrade/project/inventory-exclusions.json",
        r#"{"schema_version":1,"entries":[{"path":"../outside","reason":"bad"}]}"#,
    );
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(report.measurements[0]["candidate_scan"], "not_needed");
    assert_eq!(report.measurements[0]["visited_entries"], 0);
    assert!(finding(&report, "BootstrapExclusionRegistryInvalid"));
}

// highgrade: HG-0009-S7
#[test]
fn complete_project_still_reports_missing_navigation_without_candidate_walk() {
    let fixture = Fixture::new();
    fixture.canonical();
    fixture.write(
        "README.md",
        "[Agents](AGENTS.md) [Engineering](docs/ENGINEERING.md) [Development](docs/DEVELOPMENT.md)\n",
    );
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(report.measurements[0]["candidate_scan"], "not_needed");
    assert_eq!(report.measurements[0]["visited_entries"], 0);
    assert_eq!(
        link_status(&report, "README.md", "docs/ARCHITECTURE.md"),
        "missing"
    );
    assert_eq!(report.status, "failed");
}

#[test]
fn bundled_templates_match_the_canonical_bootstrap_contract() {
    let fixture = Fixture::new();
    for (target, source) in [
        ("README.md", include_str!("../kit/templates/README.md")),
        ("AGENTS.md", include_str!("../kit/templates/AGENTS.md")),
        (
            "docs/ARCHITECTURE.md",
            include_str!("../kit/templates/ARCHITECTURE.md"),
        ),
        (
            "docs/ENGINEERING.md",
            include_str!("../kit/templates/ENGINEERING.md"),
        ),
        (
            "docs/DEVELOPMENT.md",
            include_str!("../kit/templates/DEVELOPMENT.md"),
        ),
        (
            ".highgrade/project/INSTRUCTIONS.md",
            include_str!("../kit/templates/INSTRUCTIONS.md"),
        ),
    ] {
        fixture.write(target, source);
    }
    fixture.registry("docs/ARCHITECTURE.md");
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(report.status, "passed");
    assert_eq!(report.measurements[0]["candidate_scan"], "not_needed");
    let connected = highgrade::inspect::inspect(fixture.root(), None, None).unwrap();
    assert!(!finding(&connected, "CanonicalPathMissing"));
    assert!(!finding(&connected, "CanonicalRolePathMismatch"));
    assert!(!finding(&connected, "RequiredLinkMissingOrInvalid"));
}

// highgrade: HG-0009-S5, HG-0009-S11
#[test]
fn alternative_name_is_candidate_but_never_accepted_as_role() {
    let fixture = Fixture::new();
    fixture.write("README.md", "# Demo\n[Архитектура](docs/system.md)\n");
    fixture.write("docs/system.md", "# Secret SECRET_SENTINEL\n");
    fixture.write(".env", "TOKEN=SECRET_SENTINEL");
    let report = bootstrap(fixture.root()).unwrap();
    let arch = row(&report, "docs/ARCHITECTURE.md");
    assert_eq!(arch["path_state"], "absent");
    assert_eq!(arch["candidate_status"], "candidate_found");
    assert!(
        arch["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["path"] == "docs/system.md" && c["evidence"] == "markdown-link")
    );
    assert_eq!(
        link_status(&report, "README.md", "docs/ARCHITECTURE.md"),
        "noncanonical"
    );
    assert!(
        report.measurements[0]["exclusions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["path"] == ".env")
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("SECRET_SENTINEL")
    );
}

// highgrade: HG-0009-S8
#[test]
fn migrated_architecture_with_old_readme_link_is_reported() {
    let fixture = Fixture::new();
    fixture.canonical();
    fixture.write("docs/old-architecture.md", "# Old path\n");
    fixture.write("README.md", "# Project\n[Agents](AGENTS.md) [Architecture](docs/old-architecture.md) [Engineering](docs/ENGINEERING.md) [Development](docs/DEVELOPMENT.md)\n");
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(
        row(&report, "docs/ARCHITECTURE.md")["path_state"],
        "present"
    );
    assert_eq!(report.measurements[0]["candidate_scan"], "not_needed");
    assert_eq!(
        link_status(&report, "README.md", "docs/ARCHITECTURE.md"),
        "noncanonical"
    );
    assert_eq!(report.status, "failed");
}

// highgrade: HG-0009-S15
#[test]
fn missing_link_is_distinct_from_missing_file_and_registry_path_mismatch() {
    let fixture = Fixture::new();
    fixture.canonical();
    fixture.write("README.md", "# Demo\n[Agents](AGENTS.md) [Architecture](docs/ARCHITECTURE.md) [Development](docs/DEVELOPMENT.md)\n");
    fixture.write("docs/OLD.md", "# Old\n");
    fixture.registry("docs/OLD.md");
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(row(&report, "docs/ENGINEERING.md")["path_state"], "present");
    assert_eq!(
        link_status(&report, "README.md", "docs/ENGINEERING.md"),
        "missing"
    );
    let connected = highgrade::inspect::inspect(fixture.root(), None, None).unwrap();
    assert!(!finding(&connected, "CanonicalRolePathMismatch"));
    assert!(finding(&connected, "RequiredLinkMissingOrInvalid"));
}

// highgrade: HG-0028-S1
#[test]
fn connected_project_uses_registered_existing_path() {
    let fixture = Fixture::new();
    fixture.canonical();
    fs::rename(
        fixture.root().join("docs/ARCHITECTURE.md"),
        fixture.root().join("docs/system.md"),
    )
    .unwrap();
    fixture.registry("docs/system.md");
    fixture.write("README.md", "[Agents](AGENTS.md) [Architecture](docs/system.md) [Engineering](docs/ENGINEERING.md) [Development](docs/DEVELOPMENT.md)\n");
    let report = highgrade::inspect::inspect(fixture.root(), None, None).unwrap();
    assert_eq!(report.status, "passed", "{:?}", report.findings);
    fixture.write("README.md", "[Agents](AGENTS.md) [Architecture](docs/ARCHITECTURE.md) [Engineering](docs/ENGINEERING.md) [Development](docs/DEVELOPMENT.md)\n");
    let invalid = highgrade::inspect::inspect(fixture.root(), None, None).unwrap();
    assert!(finding(&invalid, "RequiredLinkMissingOrInvalid"));
}

// highgrade: HG-0028-S2
#[test]
fn combined_roles_are_counted_once_and_invalid_assignments_fail() {
    let fixture = Fixture::new();
    fixture.canonical();
    let path = fixture.root().join(".highgrade/project/documents.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let documents = registry["documents"].as_array_mut().unwrap();
    documents.truncate(1);
    documents[0].as_object_mut().unwrap().remove("role");
    documents[0]["roles"] = json!([
        "purpose-navigation",
        "agent-rules",
        "current-architecture",
        "engineering-rules",
        "commands-procedures"
    ]);
    documents[0]["scope"] = json!(["docs"]);
    fixture.write(
        "README.md",
        "[Instructions](.highgrade/project/INSTRUCTIONS.md)\n",
    );
    fs::write(&path, registry.to_string()).unwrap();
    let report = highgrade::inspect::inspect(fixture.root(), None, None).unwrap();
    assert_eq!(report.status, "passed", "{:?}", report.findings);
    assert_eq!(
        report
            .measurements
            .iter()
            .filter(|m| m["path"] == "README.md")
            .count(),
        1
    );
    registry["documents"][0]["roles"]
        .as_array_mut()
        .unwrap()
        .push(json!("agent-rules"));
    fs::write(&path, registry.to_string()).unwrap();
    assert!(finding(
        &highgrade::inspect::inspect(fixture.root(), None, None).unwrap(),
        "DuplicateRole"
    ));
    registry["documents"][0]["roles"] = json!(["purpose-navigation"]);
    fs::write(&path, registry.to_string()).unwrap();
    assert!(finding(
        &highgrade::inspect::inspect(fixture.root(), None, None).unwrap(),
        "RequiredRoleMissing"
    ));
    registry["documents"][0]["path"] = json!("../outside.md");
    fs::write(&path, registry.to_string()).unwrap();
    assert!(highgrade::inspect::inspect(fixture.root(), None, None).is_err());
}

// highgrade: HG-0009-S10
#[test]
fn exclusions_are_safe_and_bad_registry_does_not_hide_candidates() {
    let fixture = Fixture::new();
    fixture.write(".highgrade/project/inventory-exclusions.json", r#"{"schema_version":1,"entries":[{"path":"artifacts","reason":"PRIVATE_REASON_SENTINEL"}]}"#);
    fixture.write("artifacts/ARCHITECTURE.md", "# Generated\n");
    let report = bootstrap(fixture.root()).unwrap();
    assert_eq!(row(&report, "docs/ARCHITECTURE.md")["path_state"], "absent");
    assert_eq!(
        row(&report, "docs/ARCHITECTURE.md")["candidate_status"],
        "not_found_in_scan"
    );
    assert!(
        report.measurements[0]["exclusions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["path"] == "artifacts")
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("PRIVATE_REASON_SENTINEL")
    );
    fixture.write(".highgrade/project/inventory-exclusions.json", r#"{"schema_version":1,"entries":[{"path":"../outside","reason":"PRIVATE_REASON_SENTINEL"}]}"#);
    let bad = bootstrap(fixture.root()).unwrap();
    assert!(finding(&bad, "BootstrapExclusionRegistryInvalid"));
    assert!(bad.findings.iter().any(|entry| {
        entry["code"] == "BootstrapExclusionRegistryInvalid" && entry["status"] == "unknown"
    }));
    assert!(
        bad.measurements[0]["exclusions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["path"] != "artifacts")
    );
    let architecture = row(&bad, "docs/ARCHITECTURE.md");
    assert_eq!(architecture["path_state"], "absent");
    assert_eq!(architecture["candidate_status"], "candidate_found");
    assert!(
        architecture["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|candidate| candidate["path"] == "artifacts/ARCHITECTURE.md")
    );
}

#[test]
fn bootstrap_does_not_read_excluded_sources_or_offer_excluded_link_targets() {
    let fixture = Fixture::new();
    fixture.write("README.md", "[Architecture](private/system.md)\n");
    fixture.write("private/system.md", "# Private\n");
    fixture.write(
        ".highgrade/project/inventory-exclusions.json",
        r#"{"schema_version":1,"entries":[{"path":"private","reason":"private material"},{"path":"README.md","reason":"private material"}]}"#,
    );
    let report = bootstrap(fixture.root()).unwrap();
    assert!(finding(&report, "BootstrapLinkSourceExcluded"));
    assert_eq!(
        link_status(&report, "README.md", "docs/ARCHITECTURE.md"),
        "unknown"
    );
    assert!(
        row(&report, "docs/ARCHITECTURE.md")["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    fixture.write(
        ".highgrade/project/inventory-exclusions.json",
        r#"{"schema_version":1,"entries":[{"path":"private","reason":"private material"}]}"#,
    );
    let report = bootstrap(fixture.root()).unwrap();
    assert!(
        row(&report, "docs/ARCHITECTURE.md")["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        link_status(&report, "README.md", "docs/ARCHITECTURE.md"),
        "missing"
    );
}

#[test]
fn bootstrap_skips_generated_skill_outputs() {
    let fixture = Fixture::new();
    fixture.write(
        ".agents/skills/example/target/ARCHITECTURE.md",
        "# Generated\n",
    );
    fixture.write(
        ".codex/skills/example/build/ARCHITECTURE.md",
        "# Generated\n",
    );
    let report = bootstrap(fixture.root()).unwrap();
    assert!(
        row(&report, "docs/ARCHITECTURE.md")["candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        report.measurements[0]["exclusion_counts"]["generated-or-cache"],
        2
    );
}

// highgrade: HG-0009-S20
#[test]
fn bootstrap_reports_multiple_unvisited_locations_with_a_count() {
    let fixture = Fixture::new();
    for prefix in ["first", "second"] {
        let deep = format!(
            "{prefix}/{}",
            vec!["a"; highgrade::bootstrap::MAX_DEPTH + 1].join("/")
        );
        fixture.write(&format!("{deep}/ARCHITECTURE.md"), "# Hidden by depth\n");
    }
    let report = bootstrap(fixture.root()).unwrap();
    let locations: Vec<_> = report
        .findings
        .iter()
        .filter(|item| item["code"] == "BootstrapDepthLimit")
        .map(|item| item["location"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(locations.len(), 2);
    assert_ne!(locations[0], locations[1]);
    assert_eq!(
        report.measurements[0]["unknown_counts"]["BootstrapDepthLimit"],
        2
    );
}

#[test]
fn candidate_and_input_limits_keep_partial_evidence() {
    let fixture = Fixture::new();
    for index in 0..MAX_ITEMS + 2 {
        fixture.write(&format!("area-{index:02}/ARCHITECTURE.md"), "# Candidate\n");
    }
    fixture.write(
        ".highgrade/project/inventory-exclusions.json",
        &"x".repeat(MAX_READ_BYTES as usize + 1),
    );
    let report = bootstrap(fixture.root()).unwrap();
    assert!(finding(&report, "BootstrapCandidateLimit"));
    assert!(finding(&report, "BootstrapInputUnreadable"));
    assert_eq!(
        row(&report, "docs/ARCHITECTURE.md")["candidates"]
            .as_array()
            .unwrap()
            .len(),
        MAX_ITEMS
    );
    assert!(serde_json::to_vec_pretty(&report).unwrap().len() <= MAX_OUTPUT_BYTES);
}

#[test]
fn cli_accepts_bootstrap_without_registry_and_rejects_map_and_conflicts() {
    let fixture = Fixture::new();
    let binary = env!("CARGO_BIN_EXE_highgrade");
    let root = fixture.root().to_str().unwrap();
    let result = Command::new(binary)
        .args(["inspect", "--bootstrap", "--root", root])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1)); // structural gaps, not CLI failure
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["operation"], "inspect");
    assert_eq!(report["measurements"][0]["mode"], "bootstrap");
    let old = Command::new(binary)
        .args(["map", "--root", root])
        .output()
        .unwrap();
    let old_report: Value = serde_json::from_slice(&old.stdout).unwrap();
    assert_eq!(old_report["status"], "failed");
    let conflict = Command::new(binary)
        .args(["inspect", "--bootstrap", "--root", root, "--scope", "docs"])
        .output()
        .unwrap();
    let conflict_report: Value = serde_json::from_slice(&conflict.stdout).unwrap();
    assert_eq!(conflict_report["status"], "failed");
    assert!(bootstrap(Path::new("relative-project")).is_err());
}

#[cfg(windows)]
#[test]
fn bootstrap_skips_junction_and_matches_exclusion_case() {
    let fixture = Fixture::new();
    let outside = Fixture::new();
    outside.write("ARCHITECTURE.md", "# OUTSIDE_SENTINEL");
    fixture.write(
        ".highgrade/project/inventory-exclusions.json",
        r#"{"schema_version":1,"entries":[{"path":"artifacts","reason":"local"}]}"#,
    );
    fixture.write("Artifacts/ARCHITECTURE.md", "# Generated");
    let link = fixture.root().join("elsewhere");
    let quote = |path: &Path| path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",
        quote(&link),
        quote(outside.root())
    );
    let created = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
    let report = bootstrap(fixture.root()).unwrap();
    fs::remove_dir(&link).unwrap();
    assert!(finding(&report, "BootstrapLinkSkipped"));
    assert!(
        report.measurements[0]["exclusions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["path"] == "Artifacts")
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("OUTSIDE_SENTINEL")
    );
}
