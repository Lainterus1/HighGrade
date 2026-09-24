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
    c["title"] = json!("Sum");
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

fn automatic(root: &Path, title: &str) -> String {
    let r = call(
        root,
        "spec-new",
        &[("--title", title), ("--expected", &sha(root))],
    )
    .unwrap();
    r.measurements.iter().find_map(|m| m.get("change")).unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
fn item(root: &Path, id: &str, decision: &str) -> Value {
    let r = call(root, "spec-read", &[("--id", id)]).unwrap();
    let m = r
        .measurements
        .iter()
        .find(|m| m.get("change").is_some())
        .unwrap();
    json!({"id":id,"decision":decision,"decided_by":"simulated human in isolated test","change_sha256":m["change_sha256"],"inputs_sha256":m["inputs_sha256"],"verified_revision":"fixture-build-1","comment":if decision=="needs_changes" {"Needs clearer behavior"} else {""}})
}
fn decide(root: &Path, items: Vec<Value>) -> highgrade::Result<highgrade::Report> {
    write(root, "decision.json", &json!({"decisions":items}));
    call(
        root,
        "spec-decide",
        &[("--expected", &sha(root)), ("--input", "decision.json")],
    )
}
fn listing(root: &Path) -> Value {
    call(root, "spec-list", &[]).unwrap().measurements[0].clone()
}

#[test]
fn automatic_numbers_survive_abandon_failure_and_competing_cli_writers() {
    let root = root();
    let first = automatic(&root, "First");
    assert_eq!(first, "HG-0001");
    let original = change(&root, &first);
    assert!(original["created_at"].as_u64().unwrap() > 0);
    assert_eq!(original["title"], "First");
    call(
        &root,
        "spec-abandon",
        &[
            ("--id", &first),
            ("--expected", &sha(&root)),
            ("--reason", "Cancelled"),
        ],
    )
    .unwrap();
    let expected = sha(&root);
    let launch = || {
        Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args([
                "spec-new",
                "--root",
                root.to_str().unwrap(),
                "--title",
                "Concurrent",
                "--expected",
                &expected,
            ])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut a = launch();
    let mut b = launch();
    assert_eq!(
        usize::from(a.wait().unwrap().success()) + usize::from(b.wait().unwrap().success()),
        1
    );
    assert!(
        specs::load(&root)
            .unwrap()
            .0
            .changes
            .contains_key("HG-0002")
    );
    let before = fs::read(root.join(specs::STORE)).unwrap();
    let expected = sha(&root);
    let temp = if cfg!(windows) {
        root.join(specs::STORE)
            .with_extension(format!("{}.part", std::process::id()))
    } else {
        root.join(specs::STORE).with_extension("next")
    };
    fs::create_dir(&temp).unwrap();
    assert!(call(&root, "spec-new", &[("--expected", &expected)]).is_err());
    assert_eq!(before, fs::read(root.join(specs::STORE)).unwrap());
    fs::remove_dir(temp).unwrap();
    assert_eq!(automatic(&root, "Next"), "HG-0003");
    let mut c = change(&root, "HG-0003");
    c["title"] = json!("Renamed");
    save(&root, "HG-0003", &c);
    assert_eq!(change(&root, "HG-0003")["id"], "HG-0003");
    c["created_at"] = json!(0);
    write(&root, "edit.json", &c);
    assert!(
        call(
            &root,
            "spec-save",
            &[
                ("--id", "HG-0003"),
                ("--expected", &sha(&root)),
                ("--input", "edit.json")
            ]
        )
        .unwrap_err()
        .contains("ProtectedField")
    );
}

#[test]
fn human_history_is_atomic_protected_and_bound_to_spec_and_implementation() {
    let root = root();
    complete(&root, "HG-A");
    complete(&root, "HG-B");
    let initial = listing(&root);
    assert_eq!(initial["summary"]["ready"], 2);
    assert_eq!(initial["summary"]["pending"], 2);
    let a = item(&root, "HG-A", "accepted");
    let mut b = item(&root, "HG-B", "needs_changes");
    let before = fs::read(root.join(specs::STORE)).unwrap();
    b["comment"] = json!("");
    assert!(
        decide(&root, vec![a.clone(), b])
            .unwrap_err()
            .contains("DecisionIncomplete")
    );
    assert_eq!(before, fs::read(root.join(specs::STORE)).unwrap());
    assert!(
        decide(&root, vec![a.clone(), a.clone()])
            .unwrap_err()
            .contains("DuplicateDecision")
    );
    let b = item(&root, "HG-B", "needs_changes");
    decide(&root, vec![a.clone(), b]).unwrap();
    assert_eq!(listing(&root)["summary"]["accepted"], 1);
    assert_eq!(listing(&root)["summary"]["needs_changes"], 1);
    assert_eq!(check(&root, "HG-A").status, "passed");
    assert_eq!(integrate(&root, "HG-A").status, "passed");
    assert_eq!(listing(&root)["summary"]["accepted"], 1);
    let mut edit = change(&root, "HG-B");
    edit["acceptance"] = json!([]);
    write(&root, "edit.json", &edit);
    assert!(
        call(
            &root,
            "spec-save",
            &[
                ("--id", "HG-B"),
                ("--expected", &sha(&root)),
                ("--input", "edit.json")
            ]
        )
        .unwrap_err()
        .contains("ProtectedField")
    );
    let stale = item(&root, "HG-B", "accepted");
    let mut edit = change(&root, "HG-B");
    edit["goal"] = json!("Revised goal");
    save(&root, "HG-B", &edit);
    assert_eq!(listing(&root)["summary"]["stale"], 1);
    assert!(
        decide(&root, vec![stale])
            .unwrap_err()
            .contains("DecisionStale")
    );
    review(&root, "HG-B");
    decide(&root, vec![item(&root, "HG-B", "accepted")]).unwrap();
    let c = change(&root, "HG-B");
    assert_eq!(c["acceptance"].as_array().unwrap().len(), 2);
    assert!(c["acceptance"][1]["decided_at"].as_u64().unwrap() > 0);
    fs::write(root.join("logic.txt"), "different implementation").unwrap();
    assert_eq!(listing(&root)["summary"]["accepted"], 0);
    assert_eq!(listing(&root)["summary"]["stale"], 2);
    assert!(
        decide(&root, vec![a])
            .unwrap_err()
            .contains("DecisionStale")
    );
    let current = item(&root, "HG-B", "accepted");
    assert!(
        decide(&root, vec![current])
            .unwrap_err()
            .contains("DecisionNotReady")
    );
}

#[test]
fn json_progress_rejects_missing_skipped_unknown_and_negative_review() {
    let root = root();
    let draft = automatic(&root, "Draft");
    complete(&root, "HG-A");
    for outcome in ["skipped", "unknown", "failed"] {
        evidence(&root, "HG-A", outcome);
        review(&root, "HG-A");
        assert_eq!(listing(&root)["summary"]["ready"], 0);
        assert!(decide(&root, vec![item(&root, "HG-A", "accepted")]).is_err());
    }
    evidence(&root, "HG-A", "passed");
    review(&root, "HG-A");
    assert_eq!(listing(&root)["summary"]["ready"], 1);
    call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-A"),
            ("--expected", &sha(&root)),
            ("--verdict", "no_go"),
            ("--reviewer", "reviewer"),
            ("--conclusion", "Issue found"),
        ],
    )
    .unwrap();
    assert_eq!(listing(&root)["summary"]["ready"], 0);
    review(&root, "HG-A");
    assert_eq!(integrate(&root, "HG-A").status, "passed");
    decide(&root, vec![item(&root, "HG-A", "accepted")]).unwrap();
    fs::remove_file(root.join("result.txt")).unwrap();
    let l = listing(&root);
    assert_eq!(l["summary"]["ready"], 0);
    assert_eq!(l["summary"]["stale"], 1);
    call(
        &root,
        "spec-abandon",
        &[
            ("--id", &draft),
            ("--expected", &sha(&root)),
            ("--reason", "Not needed"),
        ],
    )
    .unwrap();
    let l = listing(&root);
    assert_eq!(l["summary"]["abandoned"], 1);
    assert_eq!(l["summary"]["total"], 2);
}

#[test]
fn isolated_three_spec_lifecycle_keeps_exact_decisions_and_pending_work() {
    let root = root();
    let mut ids = vec![];
    for title in ["First", "Second", "Third"] {
        let id = automatic(&root, title);
        fill(&root, &id);
        // Exercise the actual executable, recording its real machine report as evidence.
        let output = Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .arg("--version")
            .output()
            .unwrap();
        assert!(output.status.success());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["status"], "passed");
        fs::write(root.join("result.txt"), &output.stdout).unwrap();
        fs::write(root.join("logic.txt"), "CLI version fixture").unwrap();
        let mut c = change(&root, &id);
        let sc = &mut c["operations"][0]["requirement"]["scenarios"][0];
        sc["given"] = json!("Built highgrade CLI");
        sc["when"] = json!("Run --version");
        sc["then"] = json!("Exit 0 and JSON status passed");
        sc["verification"] = json!("Spawn real executable and parse stdout");
        save(&root, &id, &c);
        write(
            &root,
            "evidence.json",
            &json!({"command":"highgrade --version","captured_at":"2026-09-24T00:00:00Z","method":"native_report","scenario":format!("{id}-S1"),"outcome":"passed","observation":"Real CLI exited 0 with status passed","inputs":["logic.txt"],"report":"result.txt"}),
        );
        call(
            &root,
            "spec-evidence",
            &[
                ("--id", &id),
                ("--expected", &sha(&root)),
                ("--input", "evidence.json"),
            ],
        )
        .unwrap();
        review(&root, &id);
        if title != "Third" {
            assert_eq!(integrate(&root, &id).status, "passed");
        }
        ids.push(id);
    }
    assert_eq!(listing(&root)["summary"]["pending"], 3);
    decide(
        &root,
        vec![
            item(&root, &ids[0], "accepted"),
            item(&root, &ids[1], "accepted"),
            item(&root, &ids[2], "needs_changes"),
        ],
    )
    .unwrap();
    assert_eq!(listing(&root)["summary"]["accepted"], 2);
    // Revise the returned active specification and re-review before the next decision.
    let mut edited = change(&root, &ids[2]);
    edited["title"] = json!("Third: clarified version check");
    save(&root, &ids[2], &edited);
    assert_eq!(listing(&root)["summary"]["stale"], 1);
    review(&root, &ids[2]);
    decide(&root, vec![item(&root, &ids[2], "accepted")]).unwrap();
    assert_eq!(listing(&root)["summary"]["accepted"], 3);
    assert_eq!(integrate(&root, &ids[2]).status, "passed");
    assert_eq!(
        change(&root, &ids[2])["acceptance"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn v1_migration_preserves_exact_backup_ids_evidence_and_review_digest() {
    let root = root();
    fs::create_dir_all(root.join(".highgrade/specs")).unwrap();
    let original = include_bytes!("fixtures/native-v1/store.json");
    fs::write(root.join(specs::STORE), original).unwrap();
    fs::write(
        root.join("logic.txt"),
        include_bytes!("fixtures/native-v1/logic.txt"),
    )
    .unwrap();
    fs::write(
        root.join("result.txt"),
        include_bytes!("fixtures/native-v1/result.txt"),
    )
    .unwrap();
    let before = specs::load(&root).unwrap();
    assert_eq!(check(&root, "HG-0042").status, "passed");
    assert_eq!(listing(&root)["summary"]["pending"], 1);
    assert_eq!(
        original.as_slice(),
        fs::read(root.join(specs::STORE)).unwrap()
    );
    assert!(
        call(&root, "spec-new", &[("--expected", &before.1)])
            .unwrap_err()
            .contains("MigrationRequired")
    );
    let backup = root.join(format!(".highgrade/specs/backups/v1-{}.json", before.1));
    fs::create_dir_all(backup.parent().unwrap()).unwrap();
    fs::write(&backup, "unrelated backup").unwrap();
    assert!(
        call(&root, "spec-migrate", &[("--expected", &before.1)])
            .unwrap_err()
            .contains("BackupConflict")
    );
    assert_eq!(
        original.as_slice(),
        fs::read(root.join(specs::STORE)).unwrap()
    );
    fs::remove_file(&backup).unwrap();
    // Migration writes the backup first; a failed atomic replacement leaves v1 intact.
    let temp = if cfg!(windows) {
        root.join(specs::STORE)
            .with_extension(format!("{}.part", std::process::id()))
    } else {
        root.join(specs::STORE).with_extension("next")
    };
    fs::create_dir(&temp).unwrap();
    assert!(call(&root, "spec-migrate", &[("--expected", &before.1)]).is_err());
    assert_eq!(
        original.as_slice(),
        fs::read(root.join(specs::STORE)).unwrap()
    );
    assert_eq!(original.as_slice(), fs::read(&backup).unwrap());
    fs::remove_dir(temp).unwrap();
    call(&root, "spec-migrate", &[("--expected", &before.1)]).unwrap();
    let after = specs::load(&root).unwrap();
    assert_eq!(after.0.schema_version, 2);
    assert_eq!(after.0.next_number, 43);
    assert_eq!(after.0.changes, before.0.changes);
    assert_eq!(check(&root, "HG-0042").status, "passed");
    assert!(after.0.changes["HG-0042"].created_at.is_none());
    assert!(after.0.changes["HG-0042"].acceptance.is_empty());
    assert!(
        call(&root, "spec-migrate", &[("--expected", &before.1)])
            .unwrap_err()
            .contains("StoreConflict")
    );
    call(&root, "spec-migrate", &[("--expected", &after.1)]).unwrap();
    assert_eq!(sha(&root), after.1);
    assert_eq!(automatic(&root, "New"), "HG-0043");
    // v2 fields must be explicit, while unknown fields in v1 stay unsupported.
    let mut malformed: Value = serde_json::from_slice(original).unwrap();
    malformed["changes"]["HG-0042"]["title"] = json!("injected");
    write(&root, specs::STORE, &malformed);
    assert!(specs::load(&root).unwrap_err().contains("InvalidFormat"));
    let mut malformed = serde_json::to_value(after.0).unwrap();
    malformed.as_object_mut().unwrap().remove("next_number");
    write(&root, specs::STORE, &malformed);
    assert!(specs::load(&root).is_err());
}

#[test]
fn returned_integrated_spec_is_corrected_by_a_new_change_without_rewriting_history() {
    let root = root();
    complete(&root, "HG-A");
    assert_eq!(integrate(&root, "HG-A").status, "passed");
    decide(&root, vec![item(&root, "HG-A", "needs_changes")]).unwrap();
    let historical = change(&root, "HG-A");
    let correction = automatic(&root, "Correction of HG-A");
    fill(&root, &correction);
    let mut c = change(&root, &correction);
    let mut req =
        serde_json::to_value(&specs::load(&root).unwrap().0.requirements["HG-A-R1"]).unwrap();
    req["statement"] = json!("Return a checked sum including zero inputs");
    c["operations"] = json!([{"action":"modify","requirement":req}]);
    save(&root, &correction, &c);
    fs::write(root.join("logic.txt"), "corrected calculator").unwrap();
    fs::write(root.join("result.txt"), "corrected fixture: 0+3=3").unwrap();
    write(
        &root,
        "evidence.json",
        &json!({"command":"isolated correction fixture","captured_at":"2026-09-24T00:00:00Z","method":"manual","scenario":"HG-A-S1","outcome":"passed","observation":"Synthetic evidence used only to exercise store lifecycle","inputs":["logic.txt"],"report":"result.txt"}),
    );
    call(
        &root,
        "spec-evidence",
        &[
            ("--id", &correction),
            ("--expected", &sha(&root)),
            ("--input", "evidence.json"),
        ],
    )
    .unwrap();
    review(&root, &correction);
    assert_eq!(integrate(&root, &correction).status, "passed");
    decide(&root, vec![item(&root, &correction, "accepted")]).unwrap();
    assert_eq!(change(&root, "HG-A"), historical);
    assert_eq!(listing(&root)["summary"]["accepted"], 1);
    assert_eq!(listing(&root)["summary"]["stale"], 1);
    assert_eq!(
        change(&root, "HG-A")["acceptance"][0]["decision"],
        "needs_changes"
    );
}
