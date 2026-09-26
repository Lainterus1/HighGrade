mod support;
use highgrade::{hash, issues};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use support::TestDir;
fn call(root: &Path, op: &str, args: &[(&str, &str)]) -> highgrade::Result<highgrade::Report> {
    issues::command(
        root,
        op,
        &args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )
}
fn sha(root: &Path) -> String {
    call(root, "issue-list", &[])
        .unwrap()
        .measurements
        .iter()
        .find_map(|m| m["store_sha256"].as_str())
        .unwrap()
        .to_string()
}
fn put(root: &Path, file: &str, v: Value) {
    fs::write(root.join(file), serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}
fn content() -> Value {
    json!({"title":"Docker unavailable","tags":["docker","environment"],"status":"open","symptom":"Cannot connect to engine","applicability":{"environment":"Windows","conditions":["Container check"]},"cause":{"status":"unknown","description":""},"resolution":{"description":"Start configured service","prevention":""},"related_specs":["HG-0041"]})
}
fn new(root: &Path) {
    put(root, "input.json", content());
    call(
        root,
        "issue-new",
        &[("--expected", &sha(root)), ("--input", "input.json")],
    )
    .unwrap();
}
fn read(root: &Path) -> Value {
    call(
        root,
        "issue-read",
        &[("--id", "ISS-0001"), ("--view", "full")],
    )
    .unwrap()
    .measurements[0]["issue"]
        .clone()
}
fn edit(root: &Path, v: Value) -> highgrade::Result<highgrade::Report> {
    put(root, "patch.json", v);
    call(
        root,
        "issue-edit",
        &[
            ("--id", "ISS-0001"),
            ("--expected", &sha(root)),
            ("--input", "patch.json"),
        ],
    )
}
fn record(root: &Path, key: &str, outcome: &str) -> highgrade::Result<highgrade::Report> {
    put(
        root,
        "obs.json",
        json!({"occurrence_key":key,"outcome":outcome,"action":"Repeat blocked action","observed":"Observed engine state","evidence":["proof.txt"]}),
    );
    call(
        root,
        "issue-record",
        &[
            ("--id", "ISS-0001"),
            ("--expected", &sha(root)),
            ("--input", "obs.json"),
        ],
    )
}
// highgrade: HG-0041-S1
#[test]
fn issue_catalog_search_and_read_never_execute_recipes() {
    let root = TestDir::new("hg-issues-");
    assert_eq!(sha(&root), "absent");
    assert!(!root.join("issues").exists());
    assert!(!root.join(".highgrade").exists());
    new(&root);
    assert_eq!(
        call(
            &root,
            "issue-list",
            &[
                ("--query", "ENGINE"),
                ("--tag", "docker"),
                ("--status", "open")
            ]
        )
        .unwrap()
        .measurements[0]["issues"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        call(&root, "issue-list", &[("--tag", "absent")])
            .unwrap()
            .measurements[0]["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    edit(
        &root,
        json!({"resolution":{"description":"touch SHOULD_NOT_EXIST","prevention":""}}),
    )
    .unwrap();
    let before = sha(&root);
    call(&root, "issue-validate", &[("--id", "ISS-0001")]).unwrap();
    read(&root);
    assert_eq!(sha(&root), before);
    assert!(!root.join("SHOULD_NOT_EXIST").exists());
    fs::remove_dir_all(root.join(".highgrade")).unwrap();
    assert_eq!(read(&root)["id"], "ISS-0001");
    let out = Command::new(env!("CARGO_BIN_EXE_highgrade"))
        .args(["issue-record", "--help"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(v["measurements"][0]["input_schema"]["properties"]["occurrence_key"].is_object());
    assert_eq!(
        v["measurements"][0]["schemas_command"],
        "highgrade issue-schema --root PATH"
    );
    assert!(
        v["measurements"][0]["example"]
            .as_str()
            .unwrap()
            .contains("ISS-0001")
    );
}
// highgrade: HG-0041-S2, HG-0041-S3
#[test]
fn issue_evidence_rejects_case_variants_of_owned_paths() {
    let root = TestDir::new("hg-issues-");
    new(&root);
    let before = sha(&root);
    for path in [
        "issues/entries/ISS-0001.json",
        "Issues/entries/ISS-0001.json",
        ".HIGHGRADE/issues/write.lock",
    ] {
        put(
            &root,
            "obs.json",
            json!({"occurrence_key":"self-reference","outcome":"passed","action":"repeat","observed":"result","evidence":[path]}),
        );
        let error = call(
            &root,
            "issue-record",
            &[
                ("--id", "ISS-0001"),
                ("--expected", &before),
                ("--input", "obs.json"),
            ],
        )
        .unwrap_err();
        assert!(error.contains("IssueEvidenceSelfReference"), "{error}");
        assert_eq!(sha(&root), before);
    }
}
// highgrade: HG-0041-S2
#[test]
fn invalid_edits_and_concurrent_writers_leave_whole_catalog() {
    let root = TestDir::new("hg-issues-");
    new(&root);
    let before = sha(&root);
    for patch in [
        json!({"id":"ISS-9999"}),
        json!({"title":""}),
        json!({"tags":["bad/tag"]}),
        json!({"cause":{"status":"confirmed","description":""}}),
        json!({"status":"resolved"}),
    ] {
        assert!(edit(&root, patch).is_err());
        assert_eq!(sha(&root), before);
    }
    put(&root, "input.json", content());
    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args([
                "issue-new",
                "--root",
                root.to_str().unwrap(),
                "--expected",
                &before,
                "--input",
                "input.json",
            ])
            .output()
            .unwrap()
    };
    let r = root.clone();
    let old = before.clone();
    let thread = std::thread::spawn(move || {
        Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args([
                "issue-new",
                "--root",
                r.to_str().unwrap(),
                "--expected",
                &old,
                "--input",
                "input.json",
            ])
            .output()
            .unwrap()
    });
    let a = spawn();
    let b = thread.join().unwrap();
    assert_ne!(a.status.success(), b.status.success());
    assert_eq!(
        call(&root, "issue-list", &[]).unwrap().measurements[0]["total"],
        2
    );
}
// highgrade: HG-0041-S3, HG-0041-S5
#[test]
fn occurrence_receipts_are_idempotent_and_verification_is_material() {
    let root = TestDir::new("hg-issues-");
    new(&root);
    fs::write(root.join("proof.txt"), "observed").unwrap();
    record(&root, "attempt-1", "passed").unwrap();
    let before = sha(&root);
    record(&root, "attempt-1", "passed").unwrap();
    assert_eq!(sha(&root), before);
    assert!(
        record(&root, "attempt-1", "failed")
            .unwrap_err()
            .contains("OccurrenceConflict")
    );
    assert_eq!(sha(&root), before);
    edit(&root, json!({"status":"mitigated"})).unwrap();
    assert_eq!(read(&root)["content"]["status"], "mitigated");
    assert!(edit(&root, json!({"status":"resolved"})).is_err());
    edit(
        &root,
        json!({"resolution":{"description":"Updated recipe","prevention":""}}),
    )
    .unwrap();
    assert_eq!(read(&root)["content"]["status"], "stale");
    record(&root, "attempt-1", "passed").unwrap();
    assert_eq!(read(&root)["last_verified_at"], Value::Null);
    edit(
        &root,
        json!({"cause":{"status":"confirmed","description":"Stopped service"}}),
    )
    .unwrap();
    record(&root, "attempt-2", "passed").unwrap();
    edit(&root, json!({"status":"resolved"})).unwrap();
    edit(&root, json!({"tags":["docker"]})).unwrap();
    assert_eq!(read(&root)["content"]["status"], "resolved");
    fs::write(root.join("proof.txt"), "changed").unwrap();
    assert!(
        call(&root, "issue-read", &[("--id", "ISS-0001")])
            .unwrap()
            .findings
            .iter()
            .any(|f| f["code"] == "IssueEvidenceStale")
    );
    assert!(edit(&root, json!({"status":"resolved"})).is_err());
    record(&root, "attempt-3", "failed").unwrap();
    assert_eq!(read(&root)["content"]["status"], "stale");
    assert_eq!(read(&root)["occurrences"], 3);
    edit(&root, json!({"status":"open"})).unwrap();
    assert_eq!(read(&root)["content"]["status"], "open");
}
// highgrade: HG-0041-S4
#[test]
fn interrupted_issue_transaction_requires_explicit_safe_recovery() {
    let root = TestDir::new("hg-issues-");
    new(&root);
    let cat = "issues/catalog.json";
    let entry = "issues/entries/ISS-0001.json";
    let a = fs::read(root.join(cat)).unwrap();
    let b = fs::read(root.join(entry)).unwrap();
    let mut after: Value = serde_json::from_slice(&b).unwrap();
    after["content"]["title"] = json!("Updated");
    let tx = json!({cat:{"before":hash(&a),"after":String::from_utf8(a.clone()).unwrap()},entry:{"before":hash(&b),"after":serde_json::to_string_pretty(&after).unwrap()}});
    put(&root, "issues/transaction.json", tx.clone());
    let expected = hash(&fs::read(root.join("issues/transaction.json")).unwrap());
    assert!(
        call(&root, "issue-list", &[])
            .unwrap_err()
            .contains("RecoveryRequired")
    );
    put(&root, entry, json!({"foreign":"edit"}));
    assert!(
        call(&root, "issue-recover", &[("--expected", &expected)])
            .unwrap_err()
            .contains("RecoveryConflict")
    );
    assert!(root.join("issues/transaction.json").exists());
    fs::write(root.join(entry), b).unwrap();
    call(&root, "issue-recover", &[("--expected", &expected)]).unwrap();
    assert_eq!(read(&root)["content"]["title"], "Updated");
    // Fully applied but journal cleanup interrupted: recovery is also idempotent.
    put(&root, "issues/transaction.json", tx);
    call(&root, "issue-recover", &[("--expected", &expected)]).unwrap();
    assert!(!root.join("issues/transaction.json").exists());
}
// highgrade: HG-0041-S2, HG-0041-S4
#[test]
fn foreign_catalog_versions_and_unsafe_recovery_are_rejected() {
    let root = TestDir::new("hg-issues-");
    fs::create_dir(root.join("issues")).unwrap();
    fs::write(root.join("issues/mine.txt"), "mine").unwrap();
    assert!(call(&root, "issue-list", &[]).is_err());
    assert_eq!(
        fs::read_to_string(root.join("issues/mine.txt")).unwrap(),
        "mine"
    );
    fs::remove_file(root.join("issues/mine.txt")).unwrap();
    new(&root);
    let old = fs::read(root.join("issues/catalog.json")).unwrap();
    put(
        &root,
        "issues/catalog.json",
        json!({"schema_version":2,"next_number":2}),
    );
    assert!(
        call(&root, "issue-list", &[])
            .unwrap_err()
            .contains("VersionUnsupported")
    );
    fs::write(root.join("issues/catalog.json"), old).unwrap();
    let tx = json!({"issues/catalog.json":{"before":null,"after":"{}"},"../outside":{"before":null,"after":"bad"}});
    put(&root, "issues/transaction.json", tx);
    let b = fs::read(root.join("issues/transaction.json")).unwrap();
    assert!(call(&root, "issue-recover", &[("--expected", &hash(&b))]).is_err());
    assert!(!root.join("outside").exists());
}
