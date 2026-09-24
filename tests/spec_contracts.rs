use highgrade::specs::{self, Store};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    loop {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "hg-spec-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        match fs::create_dir(&p) {
            Ok(()) => return p,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => panic!("isolated test directory: {e}"),
        }
    }
}

fn call(root: &Path, op: &str, args: &[(&str, &str)]) -> highgrade::Result<highgrade::Report> {
    specs::command(
        root,
        op,
        &args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
    )
}
fn sha(root: &Path) -> String {
    specs::load(root).unwrap().1
}
fn write(root: &Path, rel: &str, value: &Value) {
    fs::write(root.join(rel), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn new(root: &Path, id: &str) {
    call(
        root,
        "spec-new",
        &[("--id", id), ("--expected", &sha(root))],
    )
    .unwrap();
}
fn change(root: &Path, id: &str) -> Value {
    serde_json::to_value(&specs::load(root).unwrap().0.changes[id]).unwrap()
}
fn save(root: &Path, id: &str, c: &Value) {
    write(root, "edit.json", c);
    call(
        root,
        "spec-save",
        &[
            ("--id", id),
            ("--expected", &sha(root)),
            ("--input", "edit.json"),
        ],
    )
    .unwrap();
}
fn fill(root: &Path, id: &str) {
    let mut c = change(root, id);
    c["goal"] = json!("Add numbers");
    c["rationale"] = json!("A caller needs a correct sum");
    c["scope"] = json!("calculator");
    c["tasks"][0]["description"] = json!("Implement and verify sum");
    c["tasks"][0]["done"] = json!(true);
    let r = &mut c["operations"][0]["requirement"];
    r["title"] = json!("Sum");
    r["statement"] = json!("Return the sum of two inputs");
    r["scenarios"][0]["given"] = json!("2 and 3");
    r["scenarios"][0]["when"] = json!("add is called");
    r["scenarios"][0]["then"] = json!("5 is returned");
    r["scenarios"][0]["verification"] = json!("Run calculator test and inspect output");
    save(root, id, &c);
}
fn evidence(root: &Path, id: &str, outcome: &str) {
    write(
        root,
        "evidence.json",
        &json!({"command":"calculator test","captured_at":"2026-09-24T00:00:00Z","method":"native_report","scenario":format!("{id}-S1"),"outcome":outcome,"observation":"The calculator test returned 5","inputs":["logic.txt"],"report":"result.txt"}),
    );
    call(
        root,
        "spec-evidence",
        &[
            ("--id", id),
            ("--expected", &sha(root)),
            ("--input", "evidence.json"),
        ],
    )
    .unwrap();
}
fn review(root: &Path, id: &str) {
    call(
        root,
        "spec-review",
        &[
            ("--id", id),
            ("--expected", &sha(root)),
            ("--reviewer", "independent reviewer"),
            ("--verdict", "go"),
            (
                "--conclusion",
                "GO: rule, real test, input coverage and limits reviewed",
            ),
        ],
    )
    .unwrap();
}
fn check(root: &Path, id: &str) -> highgrade::Report {
    call(root, "spec-check", &[("--id", id)]).unwrap()
}
fn integrate(root: &Path, id: &str) -> highgrade::Report {
    call(
        root,
        "spec-integrate",
        &[("--id", id), ("--expected", &sha(root))],
    )
    .unwrap()
}
fn complete(root: &Path, id: &str) {
    new(root, id);
    fill(root, id);
    fs::write(root.join("logic.txt"), "sum").unwrap();
    fs::write(root.join("result.txt"), "2+3=5").unwrap();
    evidence(root, id, "passed");
    review(root, id);
}

// highgrade: HG-SS-S01
#[test]
fn drafts_are_readable_but_unknown_versions_and_fields_never_write() {
    let root = root();
    new(&root, "HG-A");
    assert_eq!(check(&root, "HG-A").status, "failed");
    let original = fs::read(root.join(specs::STORE)).unwrap();
    let mut c = change(&root, "HG-A");
    c["unexpected"] = json!(1);
    write(&root, "edit.json", &c);
    assert!(
        call(
            &root,
            "spec-save",
            &[
                ("--id", "HG-A"),
                ("--expected", &sha(&root)),
                ("--input", "edit.json")
            ]
        )
        .unwrap_err()
        .contains("unknown field")
    );
    assert_eq!(original, fs::read(root.join(specs::STORE)).unwrap());
    let mut s: Value = serde_json::from_slice(&original).unwrap();
    s["schema_version"] = json!(99);
    write(&root, specs::STORE, &s);
    assert!(
        specs::load(&root)
            .unwrap_err()
            .contains("UnsupportedVersion")
    );
    let schema = call(&root, "spec-schema", &[]).unwrap();
    assert_eq!(
        schema.measurements[0]["change"]["additionalProperties"],
        false
    );
}

// highgrade: HG-SS-S02
#[test]
fn competing_processes_and_failed_replace_preserve_whole_store() {
    let root = root();
    new(&root, "HG-A");
    let expected = sha(&root);
    let launch = |id: &str| {
        Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args([
                "spec-new",
                "--root",
                root.to_str().unwrap(),
                "--id",
                id,
                "--expected",
                &expected,
            ])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut a = launch("HG-B");
    let mut b = launch("HG-C");
    let successes =
        usize::from(a.wait().unwrap().success()) + usize::from(b.wait().unwrap().success());
    assert_eq!(successes, 1);
    assert_eq!(specs::load(&root).unwrap().0.changes.len(), 2);
    let before = fs::read(root.join(specs::STORE)).unwrap();
    // An abandoned writer lock must block mutation without guessing its liveness.
    let lock_expected = sha(&root);
    fs::write(root.join(".highgrade/specs/write.lock"), "pid=dead").unwrap();
    assert!(
        call(
            &root,
            "spec-new",
            &[("--id", "HG-D"), ("--expected", &lock_expected)]
        )
        .unwrap_err()
        .contains("StoreBusy")
    );
    assert_eq!(before, fs::read(root.join(specs::STORE)).unwrap());
    fs::remove_file(root.join(".highgrade/specs/write.lock")).unwrap();
    // Collision with a prepared temp file simulates a write that cannot publish.
    let temp = if cfg!(windows) {
        root.join(specs::STORE)
            .with_extension(format!("{}.part", std::process::id()))
    } else {
        root.join(specs::STORE).with_extension("next")
    };
    fs::create_dir(&temp).unwrap();
    assert!(
        call(
            &root,
            "spec-new",
            &[("--id", "HG-D"), ("--expected", &sha(&root))]
        )
        .is_err()
    );
    assert_eq!(before, fs::read(root.join(specs::STORE)).unwrap());
}

// highgrade: HG-SS-S03
#[test]
fn integration_rejects_stale_base_and_requires_removal_reason() {
    let root = root();
    complete(&root, "HG-A");
    assert_eq!(integrate(&root, "HG-A").status, "passed");
    new(&root, "HG-MOD");
    fill(&root, "HG-MOD");
    let mut modified = change(&root, "HG-MOD");
    let mut requirement =
        serde_json::to_value(&specs::load(&root).unwrap().0.requirements["HG-A-R1"]).unwrap();
    requirement["title"] = json!("Sum with stable identifiers");
    modified["operations"] = json!([{"action":"modify", "requirement":requirement}]);
    save(&root, "HG-MOD", &modified);
    write(
        &root,
        "evidence.json",
        &json!({"command":"calculator test","captured_at":"2026-09-24T00:00:00Z","method":"native_report","scenario":"HG-A-S1","outcome":"passed","observation":"sum still returns 5","inputs":["logic.txt"],"report":"result.txt"}),
    );
    call(
        &root,
        "spec-evidence",
        &[
            ("--id", "HG-MOD"),
            ("--expected", &sha(&root)),
            ("--input", "evidence.json"),
        ],
    )
    .unwrap();
    review(&root, "HG-MOD");
    assert_eq!(integrate(&root, "HG-MOD").status, "passed");
    assert_eq!(
        specs::load(&root).unwrap().0.requirements["HG-A-R1"].title,
        "Sum with stable identifiers"
    );
    new(&root, "HG-B");
    new(&root, "HG-C");
    for key in ["HG-B", "HG-C"] {
        fill(&root, key);
        let mut c = change(&root, key);
        c["operations"] =
            json!([{"action":"remove","id":"HG-A-R1","reason":"Superseded by caller contract"}]);
        save(&root, key, &c);
        review(&root, key);
    }
    let mut c = change(&root, "HG-B");
    c["operations"][0]["reason"] = json!("");
    save(&root, "HG-B", &c);
    review(&root, "HG-B");
    assert_eq!(integrate(&root, "HG-B").status, "failed");
    c = change(&root, "HG-B");
    c["operations"][0]["reason"] = json!("No longer required");
    save(&root, "HG-B", &c);
    review(&root, "HG-B");
    assert_eq!(integrate(&root, "HG-B").status, "passed");
    assert!(
        check(&root, "HG-C")
            .findings
            .iter()
            .any(|f| f["code"] == "BaseDrift")
    );
    let store = specs::load(&root).unwrap().0;
    assert!(store.requirements.is_empty());
    assert!(store.changes["HG-B"].archived);
    assert!(store.retired_ids.contains("HG-A-S1"));
}

// highgrade: HG-SS-S04
#[test]
fn evidence_and_review_go_stale_without_losing_unaffected_observations() {
    let root = root();
    complete(&root, "HG-A");
    assert_eq!(check(&root, "HG-A").status, "passed");
    fs::write(root.join("unrelated.txt"), "unrelated change").unwrap();
    assert_eq!(check(&root, "HG-A").status, "passed");
    for file in ["logic.txt", "result.txt"] {
        let old = fs::read(root.join(file)).unwrap();
        fs::write(root.join(file), "changed").unwrap();
        assert_eq!(check(&root, "HG-A").status, "failed");
        fs::write(root.join(file), old).unwrap();
    }
    evidence(&root, "HG-A", "skipped");
    review(&root, "HG-A");
    assert_eq!(check(&root, "HG-A").status, "failed");
    evidence(&root, "HG-A", "passed");
    review(&root, "HG-A");
    let mut c = change(&root, "HG-A");
    let kept = c["evidence"].clone();
    c["operations"][0]["requirement"]["scenarios"][0]["then"] = json!("6 is returned");
    save(&root, "HG-A", &c);
    assert_eq!(change(&root, "HG-A")["evidence"], kept);
    assert_eq!(check(&root, "HG-A").status, "failed");
}

// highgrade: HG-SS-S06
#[test]
fn selected_import_retains_source_and_leaves_other_legacy_requirements() {
    let root = root();
    complete(&root, "HG-A");
    let r = change(&root, "HG-A")["operations"][0]["requirement"].clone();
    let legacy = "### Requirement: [HG-A-R1] Sum\nOriginal rule\n#### Scenario: [HG-A-S1] Addition\n- GIVEN 2 and 3\n- WHEN added\n- THEN 5\n### Requirement: [HG-OTHER-R1] Other\nKeep this rule\n";
    fs::write(root.join("legacy.md"), legacy).unwrap();
    write(&root, "requirement.json", &r);
    call(
        &root,
        "spec-import",
        &[
            ("--expected", &sha(&root)),
            ("--input", "requirement.json"),
            ("--source", "legacy.md"),
            ("--id", "HG-IMPORT"),
        ],
    )
    .unwrap();
    let s: Store = specs::load(&root).unwrap().0;
    assert_eq!(s.requirements.len(), 0);
    assert!(
        s.changes["HG-IMPORT"].imports["HG-A-R1"]
            .original_text
            .contains("Original rule")
    );
    assert_eq!(
        s.changes["HG-IMPORT"].imports["HG-A-R1"].sha256,
        highgrade::hash(legacy.as_bytes())
    );
    assert_eq!(fs::read_to_string(root.join("legacy.md")).unwrap(), legacy);
    let mut draft = change(&root, "HG-IMPORT");
    draft["operations"][0]["requirement"]["id"] = json!("HG-OTHER-R1");
    save(&root, "HG-IMPORT", &draft);
    assert!(
        check(&root, "HG-IMPORT")
            .findings
            .iter()
            .any(|f| f["code"] == "ImportOperationMismatch")
    );
    draft = change(&root, "HG-IMPORT");
    draft["operations"][0]["requirement"]["id"] = json!("HG-A-R1");
    draft["operations"][0]["requirement"]["scenarios"][0]["id"] = json!("HG-OTHER-S1");
    save(&root, "HG-IMPORT", &draft);
    assert!(
        check(&root, "HG-IMPORT")
            .findings
            .iter()
            .any(|f| f["code"] == "ImportScenarioMismatch")
    );
}

#[test]
fn negative_review_and_sibling_scenario_regressions() {
    let root = root();
    complete(&root, "HG-A");
    call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-A"),
            ("--expected", &sha(&root)),
            ("--verdict", "no_go"),
            ("--reviewer", "reviewer"),
            ("--conclusion", "Wrong behavior"),
        ],
    )
    .unwrap();
    assert_eq!(integrate(&root, "HG-A").status, "failed");
    let mut c = change(&root, "HG-A");
    let mut second = c["operations"][0]["requirement"]["scenarios"][0].clone();
    second["id"] = json!("HG-A-S2");
    c["operations"][0]["requirement"]["scenarios"]
        .as_array_mut()
        .unwrap()
        .push(second);
    save(&root, "HG-A", &c);
    // S1's evidence predates adding S2 but its own contract has not changed.
    let report = check(&root, "HG-A");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f["location"] == "/operations/0/requirement/scenarios/1")
    );
    assert!(
        !report
            .findings
            .iter()
            .any(|f| f["location"] == "/operations/0/requirement/scenarios/0")
    );
    c = change(&root, "HG-A");
    c["operations"][0]["requirement"]["statement"] = json!("Changed common rule");
    save(&root, "HG-A", &c);
    assert!(
        check(&root, "HG-A")
            .findings
            .iter()
            .any(|f| f["location"] == "/operations/0/requirement/scenarios/0")
    );
}

#[test]
fn read_payload_and_cas_always_describe_the_same_snapshot() {
    let root = root();
    new(&root, "HG-A");
    let writer_root = root.clone();
    let writer = std::thread::spawn(move || {
        for i in 0..40 {
            loop {
                let Ok((mut snapshot, expected)) = specs::load(&writer_root) else {
                    std::thread::yield_now();
                    continue;
                };
                let c = snapshot.changes.get_mut("HG-A").unwrap();
                c.goal = format!("Goal {i}");
                write(&writer_root, "edit.json", &serde_json::to_value(c).unwrap());
                match call(
                    &writer_root,
                    "spec-save",
                    &[
                        ("--id", "HG-A"),
                        ("--expected", &expected),
                        ("--input", "edit.json"),
                    ],
                ) {
                    Ok(_) => break,
                    Err(e) if e.contains("StoreBusy") => std::thread::yield_now(),
                    Err(e) => panic!("{e}"),
                }
            }
        }
    });
    for _ in 0..80 {
        let report = loop {
            match call(&root, "spec-read", &[("--id", "HG-A")]) {
                Ok(r) => break r,
                Err(e) if e.contains("StoreBusy") => std::thread::yield_now(),
                Err(e) => panic!("{e}"),
            }
        };
        let value = report
            .measurements
            .iter()
            .find_map(|m| m.get("change"))
            .unwrap();
        let c = serde_json::from_value(value.clone()).unwrap();
        let mut snapshot = Store::default();
        snapshot.changes.insert("HG-A".into(), c);
        let expected = highgrade::hash(&serde_json::to_vec_pretty(&snapshot).unwrap());
        assert!(
            report
                .measurements
                .iter()
                .any(|m| m["store_sha256"] == expected)
        );
    }
    writer.join().unwrap();
}

#[test]
fn native_doctor_does_not_require_openspec_or_execute_external_tools() {
    let root = root();
    fs::create_dir_all(root.join(".highgrade/project")).unwrap();
    fs::write(
        root.join(".highgrade/project/INSTRUCTIONS.md"),
        "---\nhighgrade_project_schema: 1\nhighgrade_spec_format: native-v1\n---\n",
    )
    .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_highgrade"))
        .args(["doctor", "--root", root.to_str().unwrap()])
        .env("PATH", "")
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        report["status"], "passed",
        "Native CLI has no runtime dependency on Cargo, Node, OpenSpec or codex executables"
    );
    assert!(
        !report["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["location"] == "openspec")
    );
    assert!(
        report["measurements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["project_instruction"] == "compatible")
    );
}
