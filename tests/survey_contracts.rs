mod support;
use highgrade::{discovery, survey};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use support::TestDir;
fn call(root: &Path, op: &str, args: &[(&str, &str)]) -> highgrade::Result<highgrade::Report> {
    survey::command(
        root,
        op,
        &args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}
fn read(root: &Path, view: &str) -> Value {
    call(root, "survey-read", &[("--view", view)])
        .unwrap()
        .measurements[0]
        .clone()
}
fn sha(root: &Path) -> String {
    read(root, "summary")["survey_sha256"]
        .as_str()
        .unwrap()
        .into()
}
fn edit(root: &Path, patch: Value) -> highgrade::Result<highgrade::Report> {
    fs::write(root.join("patch.json"), serde_json::to_vec(&patch).unwrap()).unwrap();
    call(
        root,
        "survey-edit",
        &[("--expected", &sha(root)), ("--input", "patch.json")],
    )
}
fn ready(root: &Path) {
    call(root, "survey-new", &[]).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.txt"), "source").unwrap();
    let mut c = read(root, "editable")["value"].clone();
    c["goal"] = json!("Connect fixture");
    for d in c["decisions"].as_array_mut().unwrap() {
        d["state"] = json!("confirmed");
        d["basis"] = json!(["Explicit test fixture decision"]);
        d["blocks"] = json!([]);
    }
    for (name, section) in c["sections"].as_object_mut().unwrap() {
        *section = json!([{"id":format!("{name}-1"),"state":"not_applicable","description":"Synthetic fixture does not have this subsystem","basis":["Fixture setup"],"sources":[],"blocks":[]}]);
    }
    c["source_paths"] = json!(["src/main.txt"]);
    c["areas"] = json!(["src"]);
    edit(root, c).unwrap();
    call(root, "survey-snapshot", &[("--expected", &sha(root))]).unwrap();
    call(
        root,
        "survey-review",
        &[
            ("--expected", &sha(root)),
            ("--verdict", "go"),
            ("--reviewer", "independent fixture reviewer"),
            ("--conclusion", "Reviewed fixture contract"),
        ],
    )
    .unwrap();
}
fn status(root: &Path) -> Value {
    call(root, "survey-check", &[]).unwrap().measurements[0].clone()
}
// highgrade: HG-0045-S1, HG-0046-S4
#[test]
fn survey_starts_without_specs_and_reading_never_executes_or_writes() {
    let root = TestDir::new("hg-survey-");
    call(&root, "survey-new", &[]).unwrap();
    assert!(!root.join("specs").exists());
    assert!(!root.join(".highgrade/project/documents.json").exists());
    assert!(call(&root, "survey-new", &[]).is_err());
    edit(
        &root,
        json!({"goal":"touch NEVER_EXECUTE","next":"continue"}),
    )
    .unwrap();
    let before = fs::read(root.join(survey::FILE)).unwrap();
    for view in ["summary", "full", "editable", "markdown", "components"] {
        read(&root, view);
    }
    assert_eq!(status(&root)["structure"], "valid");
    assert_eq!(status(&root)["completion_ready"], false);
    assert_eq!(before, fs::read(root.join(survey::FILE)).unwrap());
    assert!(!root.join("NEVER_EXECUTE").exists());
    assert!(call(&root, "survey-schema", &[]).unwrap().measurements[0]["content"].is_object());
    let cli = Command::new(env!("CARGO_BIN_EXE_highgrade"))
        .args([
            "survey-read",
            "--root",
            root.to_str().unwrap(),
            "--view",
            "markdown",
        ])
        .output()
        .unwrap();
    assert!(cli.status.success());
}
// highgrade: HG-0045-S2
#[test]
fn unresolved_decision_and_dependent_findings_do_not_become_ready() {
    let root = TestDir::new("hg-survey-");
    ready(&root);
    assert_eq!(status(&root)["completion_ready"], true);
    let mut c = read(&root, "editable")["value"].clone();
    c["sections"]["findings"] = json!([{"id":"unknown-deploy","state":"unknown","description":"Deployment unexamined","basis":[],"sources":[],"blocks":["deploy"]}]);
    edit(&root, c).unwrap();
    call(
        &root,
        "survey-review",
        &[
            ("--expected", &sha(&root)),
            ("--verdict", "go"),
            ("--reviewer", "reviewer"),
            ("--conclusion", "Only local adaptation selected"),
        ],
    )
    .unwrap();
    let s = status(&root);
    assert_eq!(s["adaptation_ready"], true);
    assert_eq!(s["completion_ready"], false);
    assert_eq!(s["semantic_correctness"], "not_certified");
    let before = fs::read(root.join(survey::FILE)).unwrap();
    assert!(edit(&root, json!({"stage":"completed","next":""})).is_err());
    assert_eq!(before, fs::read(root.join(survey::FILE)).unwrap());
    let mut c = read(&root, "editable")["value"].clone();
    c["decisions"][0]["state"] = json!("unknown");
    edit(&root, c).unwrap();
    assert_eq!(status(&root)["adaptation_ready"], false);
}
// highgrade: HG-0045-S3, HG-0046-S2
#[test]
fn survey_tracks_only_selected_sources_and_preserves_completed_history() {
    let root = TestDir::new("hg-survey-");
    ready(&root);
    edit(&root, json!({"next":"different resume step"})).unwrap();
    assert_eq!(status(&root)["completion_ready"], true);
    fs::write(root.join("unrelated.txt"), "unrelated").unwrap();
    assert_eq!(status(&root)["completion_ready"], true);
    fs::write(root.join("src/new.txt"), "new component").unwrap();
    assert_eq!(status(&root)["snapshot_current"], false);
    fs::remove_file(root.join("src/new.txt")).unwrap();
    edit(&root, json!({"stage":"completed","next":""})).unwrap();
    let previous = read(&root, "full")["value"]["current"].clone();
    fs::write(root.join("src/main.txt"), "changed").unwrap();
    let s = status(&root);
    assert_eq!(s["snapshot_current"], false);
    assert_eq!(s["historically_completed"], true);
    call(&root, "survey-reopen", &[("--expected", &sha(&root))]).unwrap();
    let full = read(&root, "full")["value"].clone();
    assert_eq!(full["history"][0], previous);
    assert_eq!(
        full["current"]["content"]["decisions"],
        previous["content"]["decisions"]
    );
    assert!(full["current"]["review"].is_null());
}
// highgrade: HG-0046-S3
#[test]
fn survey_cas_invalid_inputs_and_lock_leave_document_unchanged() {
    let root = TestDir::new("hg-survey-");
    ready(&root);
    let stale = sha(&root);
    edit(&root, json!({"next":"next revision"})).unwrap();
    let bytes = fs::read(root.join(survey::FILE)).unwrap();
    assert!(
        call(
            &root,
            "survey-edit",
            &[("--expected", &stale), ("--input", "patch.json")]
        )
        .is_err()
    );
    for patch in [
        json!({"unknown":true}),
        json!({"schema_version":99}),
        json!({"source_paths":["../outside"]}),
        json!({"source_paths":[survey::FILE]}),
        json!({"scope":["src"],"source_paths":["outside.txt"]}),
    ] {
        let result = edit(&root, patch);
        // Own survey input is rejected during capture rather than patch validation.
        if result.is_ok() {
            assert!(call(&root, "survey-snapshot", &[("--expected", &sha(&root))]).is_err());
            fs::write(root.join(survey::FILE), &bytes).unwrap();
        }
        assert_eq!(bytes, fs::read(root.join(survey::FILE)).unwrap());
    }
    fs::write(root.join(".highgrade/project/survey.lock"), "foreign lock").unwrap();
    assert!(edit(&root, json!({"next":"cannot write"})).is_err());
    assert_eq!(bytes, fs::read(root.join(survey::FILE)).unwrap());
    fs::remove_file(root.join(".highgrade/project/survey.lock")).unwrap();
    let mut bad: Value = serde_json::from_slice(&bytes).unwrap();
    bad["schema_version"] = json!(99);
    fs::write(root.join(survey::FILE), serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(call(&root, "survey-check", &[]).is_err());
}
// highgrade: HG-0046-S1
#[test]
fn structural_inventory_keeps_configuration_and_never_hashes_large_or_secret_files() {
    let root = TestDir::new("hg-survey-");
    fs::create_dir_all(root.join(".highgrade/project")).unwrap();
    fs::write(root.join(".highgrade/project/INSTRUCTIONS.md"), "config").unwrap();
    fs::write(root.join(".env"), "PRIVATE_SENTINEL").unwrap();
    fs::create_dir(root.join("target")).unwrap();
    fs::write(root.join("target/cache"), "cache").unwrap();
    fs::File::create(root.join("large.bin"))
        .unwrap()
        .set_len(33 * 1024 * 1024)
        .unwrap();
    let s = discovery::structure(&root, ".").unwrap();
    assert!(s.complete);
    assert!(s.entries.contains_key(".highgrade/project/INSTRUCTIONS.md"));
    assert!(s.entries.contains_key("large.bin"));
    assert!(!s.entries.contains_key(".env"));
    assert!(
        !serde_json::to_string(&s)
            .unwrap()
            .contains("PRIVATE_SENTINEL")
    );
    assert!(highgrade::install::inventory(&root, "audit.json").is_err()); // legacy still refuses an oversized file
    assert!(discovery::source_hash(&root, ".env").is_err());
    let result = Command::new(env!("CARGO_BIN_EXE_highgrade"))
        .args([
            "inventory",
            "--root",
            root.to_str().unwrap(),
            "--mode",
            "structure",
        ])
        .output()
        .unwrap();
    assert!(result.status.success());
}
// highgrade: HG-0046-S1, HG-0046-S2
#[test]
fn discovery_limits_and_missing_sources_are_explicitly_incomplete() {
    let root = TestDir::new("hg-survey-");
    ready(&root);
    fs::remove_file(root.join("src/main.txt")).unwrap();
    assert_eq!(status(&root)["unknown_sources"], json!(["src/main.txt"]));
    for n in 0..=discovery::MAX_ENTRIES {
        fs::write(root.join("src").join(format!("{n}.txt")), "").unwrap();
    }
    let s = discovery::structure(&root, "src").unwrap();
    assert!(!s.complete);
    assert!(s.omitted.values().any(|v| v == "entry_limit"));
    assert!(discovery::structure(&root, "../outside").is_err());
}
#[cfg(windows)]
// highgrade: HG-0046-S1, HG-0046-S3
#[test]
fn survey_refuses_junctions_and_preserves_outside_sentinel() {
    let root = TestDir::new("hg-survey-");
    let outside = TestDir::new("hg-survey-outside-");
    fs::write(outside.join("sentinel"), "outside").unwrap();
    let quote = |p: &Path| p.to_string_lossy().replace('\'', "''");
    let script = format!(
        "New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",
        quote(&root.join("link")),
        quote(&outside)
    );
    assert!(
        Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .unwrap()
            .status
            .success()
    );
    let s = discovery::structure(&root, ".").unwrap();
    assert!(!s.complete);
    assert!(!s.entries.contains_key("link/sentinel"));
    assert!(discovery::source_hash(&root, "link/sentinel").is_err());
    ready(&root);
    edit(&root, json!({"areas":["."]})).unwrap();
    call(&root, "survey-snapshot", &[("--expected", &sha(&root))]).unwrap();
    call(
        &root,
        "survey-review",
        &[
            ("--expected", &sha(&root)),
            ("--verdict", "go"),
            ("--reviewer", "reviewer"),
            ("--conclusion", "Partial snapshot examined"),
        ],
    )
    .unwrap();
    assert_eq!(status(&root)["snapshot_current"], false);
    fs::remove_dir(root.join("link")).unwrap();
    assert_eq!(status(&root)["snapshot_current"], false);
    assert_eq!(status(&root)["completion_ready"], false);
    assert_eq!(
        fs::read_to_string(outside.join("sentinel")).unwrap(),
        "outside"
    );
}

#[test]
fn survey_help_exposes_its_own_schema_and_required_arguments() {
    for op in ["survey-edit", "survey-review"] {
        let out = Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args([op, "--help"])
            .output()
            .unwrap();
        assert!(out.status.success());
        let r: Value = serde_json::from_slice(&out.stdout).unwrap();
        let help = &r["measurements"][0];
        assert_eq!(
            help["schemas_command"],
            "highgrade survey-schema --root PATH"
        );
        if op == "survey-edit" {
            assert!(help["input_schema"].is_object());
        } else {
            assert!(help["example"].as_str().unwrap().contains("--reviewer"));
            assert!(help["example"].as_str().unwrap().contains("--conclusion"));
        }
    }
}

// highgrade: HG-0045-S2
#[test]
fn negative_review_is_current_but_does_not_allow_transition() {
    let root = TestDir::new("hg-survey-");
    ready(&root);
    call(
        &root,
        "survey-review",
        &[
            ("--expected", &sha(&root)),
            ("--reviewer", "reviewer"),
            ("--conclusion", "Missing semantic work"),
            ("--verdict", "no_go"),
        ],
    )
    .unwrap();
    let s = status(&root);
    assert_eq!(s["review_record_current"], true);
    assert_eq!(s["review_go"], false);
    assert_eq!(s["adaptation_ready"], false);
    assert!(edit(&root, json!({"stage":"planned"})).is_err());
}

// highgrade: HG-0047-S3
#[test]
fn survey_assets_and_schema_work_from_installed_release() {
    let profile = TestDir::new("hg-survey-profile-");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("kit");
    let manifest: Value =
        serde_json::from_slice(&fs::read(source.join("manifest.json")).unwrap()).unwrap();
    highgrade::global::install(
        &profile,
        &source,
        Path::new(env!("CARGO_BIN_EXE_highgrade")),
    )
    .unwrap();
    let release = profile
        .join(".highgrade/global/releases")
        .join(manifest["release"].as_str().unwrap());
    let output = Command::new(release.join("highgrade.exe"))
        .args(["survey-schema", "--root", profile.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    let mut actual = actual["measurements"][0].clone();
    let template = actual.as_object_mut().unwrap().remove("template").unwrap();
    let shipped: Value =
        serde_json::from_slice(&fs::read(release.join("references/survey-schema.json")).unwrap())
            .unwrap();
    assert_eq!(actual, shipped);
    let shipped: Value =
        serde_json::from_slice(&fs::read(release.join("templates/survey.json")).unwrap()).unwrap();
    assert_eq!(template, shipped);
    assert!(release.join("references/tools/survey.md").exists());
    assert!(release.join("procedures/init.md").exists());
}
