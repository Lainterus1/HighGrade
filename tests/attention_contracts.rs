mod support;
use highgrade::specs;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use support::TestDir;
fn call(p: &Path, op: &str, args: &[(&str, &str)]) -> highgrade::Result<highgrade::Report> {
    specs::command(
        p,
        op,
        &args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
    )
}
fn sha(p: &Path) -> String {
    specs::load(p).unwrap().1
}
fn put(p: &Path, v: Value) {
    fs::write(p.join("input.json"), serde_json::to_vec(&v).unwrap()).unwrap();
}
fn edit(p: &Path, id: &str, v: Value) {
    put(p, v);
    call(
        p,
        "spec-edit",
        &[
            ("--id", id),
            ("--expected", &sha(p)),
            ("--input", "input.json"),
        ],
    )
    .unwrap();
}
fn setup() -> TestDir {
    let p = TestDir::new("hg-attention-");
    for i in 1..=3 {
        let id = format!("HG-{i:04}");
        call(
            &p,
            "spec-new",
            &[("--title", "Реальная спека"), ("--expected", &sha(&p))],
        )
        .unwrap();
        edit(
            &p,
            &id,
            json!({"goal":"Результат","rationale":"Причина","scope":"Область",
            "tasks":[{"id":format!("{id}-T1"),"description":"Работа","done":false}],
            "operations":[{"action":"add","requirement":{"id":format!("{id}-R1"),"title":"Требование","statement":"Условие",
                "scenarios":[{"id":format!("{id}-S1"),"given":"Дано","when":"Действие","then":"Результат","verification":"Наблюдение"}]}}]}),
        );
    }
    p
}
fn ui(p: &Path) -> Value {
    call(p, "spec-ui", &[]).unwrap().measurements[0].clone()
}
fn row(p: &Path, id: &str) -> Value {
    ui(p)["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap()
        .clone()
}
fn mutate(p: &Path, id: &str, v: Value) -> highgrade::Result<highgrade::Report> {
    put(p, v);
    call(
        p,
        "spec-attention",
        &[
            ("--id", id),
            ("--expected", &sha(p)),
            ("--input", "input.json"),
        ],
    )
}
fn request(p: &Path, id: &str, key: &str, kind: &str) {
    mutate(p,id,json!({"action":"request","request_id":key,"kind":kind,"reason":key,"content_sha256":row(p,id)["content_sha256"]})).unwrap();
}
fn response(
    p: &Path,
    id: &str,
    key: &str,
    decision: &str,
    comment: &str,
) -> highgrade::Result<highgrade::Report> {
    mutate(
        p,
        id,
        json!({"action":"respond","request_id":key,"decision":decision,"author":"Автор","comment":comment,"content_sha256":row(p,id)["content_sha256"]}),
    )
}
// highgrade: HG-0052-S1, HG-0052-S2
#[test]
fn projection_preserves_bytes_excludes_legacy_and_reports_corruption() {
    let p = setup();
    // Fixture mutation deliberately models a valid previous structural profile.
    let file = p.join("specs/changes/HG-0003/spec.json");
    let mut v: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    v.as_object_mut().unwrap().remove("scoped_baseline");
    fs::write(&file, serde_json::to_vec(&v).unwrap()).unwrap();
    let before = sha(&p);
    let view = ui(&p);
    assert_eq!(view["excluded_count"], 1);
    assert_eq!(view["changes"].as_array().unwrap().len(), 2);
    assert_eq!(row(&p, "HG-0001")["requirements_agreement"], "pending");
    assert_eq!(sha(&p), before);
    fs::write(file, b"{invalid").unwrap();
    assert!(call(&p, "spec-ui", &[]).is_err());
}
// highgrade: HG-0052-S3, HG-0052-S4, HG-0052-S5, HG-0052-S6
#[test]
fn explicit_handoffs_do_not_grant_execution_and_responses_close_only_selected_request() {
    let p = setup();
    let id = "HG-0001";
    assert_eq!(ui(&p)["counts"]["needs_decision"], 0);
    request(&p, id, "approval", "requirements");
    let before = sha(&p);
    request(&p, id, "approval", "requirements");
    assert_eq!(sha(&p), before);
    response(&p, id, "approval", "accepted", "").unwrap();
    assert_eq!(row(&p, id)["requirements_agreement"], "accepted");
    let c = &specs::load(&p).unwrap().0.changes[id];
    assert!(!c.tasks[0].done && c.acceptance.is_empty() && c.evidence.is_empty() && !c.archived);
    assert!(mutate(&p,id,json!({"action":"request","request_id":"result","reason":"готово","kind":"result","content_sha256":row(&p,id)["content_sha256"]})).unwrap_err().contains("DecisionNotReady"));
    request(&p, id, "q1", "question");
    request(&p, id, "q2", "question");
    let before = sha(&p);
    assert!(response(&p, id, "q1", "answer", "").is_err());
    assert_eq!(sha(&p), before);
    response(&p, id, "q1", "answer", "Ответ").unwrap();
    assert_eq!(row(&p, id)["requests"].as_array().unwrap().len(), 1);
    assert_eq!(row(&p, id)["primary_action"]["id"], "q2");
    response(&p, id, "q2", "answer", "Другой ответ").unwrap();
    request(&p, id, "reject", "requirements");
    let before = sha(&p);
    assert!(response(&p, id, "reject", "needs_changes", "").is_err());
    assert_eq!(sha(&p), before);
    response(&p, id, "reject", "needs_changes", "Уточнить условие").unwrap();
    assert_eq!(row(&p, id)["category"], "in_work");
    assert_eq!(row(&p, id)["history"].as_array().unwrap().len(), 4);
}
// highgrade: HG-0052-S7, HG-0052-S8, HG-0052-S9
#[test]
fn content_snapshots_metadata_and_stale_writes_are_distinct() {
    let p = setup();
    let id = "HG-0001";
    request(&p, id, "approval", "requirements");
    response(&p, id, "approval", "accepted", "").unwrap();
    call(
        &p,
        "spec-tag-set",
        &[
            ("--id", "ui"),
            ("--title", "UI"),
            ("--description", "UI"),
            ("--expected", &sha(&p)),
        ],
    )
    .unwrap();
    edit(&p, id, json!({"tags":["ui"]}));
    assert_eq!(row(&p, id)["requirements_agreement"], "accepted");
    let old = row(&p, id)["content_sha256"].clone();
    let old_store = sha(&p);
    edit(&p, id, json!({"goal":"Новая цель"}));
    let before = sha(&p);
    put(
        &p,
        json!({"action":"respond","request_id":"approval","content_sha256":old,"decision":"accepted","author":"Автор","comment":""}),
    );
    assert!(
        call(
            &p,
            "spec-attention",
            &[
                ("--id", id),
                ("--input", "input.json"),
                ("--expected", &old_store)
            ]
        )
        .unwrap_err()
        .contains("StoreConflict")
    );
    assert!(
        call(
            &p,
            "spec-attention",
            &[
                ("--id", id),
                ("--input", "input.json"),
                ("--expected", &before)
            ]
        )
        .unwrap_err()
        .contains("DecisionStale")
    );
    assert_eq!(sha(&p), before);
    let r = row(&p, id);
    assert_eq!(r["requirements_agreement"], "stale");
    assert_eq!(r["category"], "in_work");
    assert_eq!(r["changes_since_agreement"][0]["field"], "goal");
    request(&p, id, "new approval", "requirements");
    assert_eq!(row(&p, id)["category"], "needs_decision");
}
// highgrade: HG-0052-S10
#[test]
fn queue_priority_categories_and_counts_are_deterministic() {
    let p = setup();
    request(&p, "HG-0001", "a", "requirements");
    request(&p, "HG-0002", "b", "question");
    request(&p, "HG-0002", "c", "requirements");
    call(
        &p,
        "spec-abandon",
        &[
            ("--id", "HG-0003"),
            ("--expected", &sha(&p)),
            ("--reason", "Отмена"),
        ],
    )
    .unwrap();
    let a = ui(&p);
    assert_eq!(a, ui(&p));
    assert_eq!(a["changes"][0]["id"], "HG-0002");
    assert_eq!(a["counts"]["needs_decision"], 2);
    assert_eq!(a["counts"]["cancelled"], 1);
    assert_eq!(a["counts"]["completed"], 0);
    assert_eq!(row(&p, "HG-0002")["primary_action"]["kind"], "question");
}

fn evidence(p: &Path, id: &str, time: &str) {
    put(
        p,
        json!({"command":"isolated fixture observation","captured_at":time,"method":"manual","scenario":format!("{id}-S1"),"outcome":"passed","observation":"fixture satisfied","inputs":["logic.txt"],"report":"report.txt"}),
    );
    call(
        p,
        "spec-evidence",
        &[
            ("--id", id),
            ("--expected", &sha(p)),
            ("--input", "input.json"),
        ],
    )
    .unwrap();
}
// highgrade: HG-0052-S1, HG-0052-S4, HG-0052-S6, HG-0052-S8, HG-0052-S9, HG-0052-S10
#[test]
fn result_decisions_reuse_evidence_gates_and_equivalent_reruns_keep_agreement() {
    let p = setup();
    let id = "HG-0001";
    request(&p, id, "content", "requirements");
    response(&p, id, "content", "accepted", "").unwrap();
    edit(
        &p,
        id,
        json!({"tasks":[{"id":"HG-0001-T1","description":"Работа","done":true}]}),
    );
    fs::write(p.join("logic.txt"), "v1").unwrap();
    fs::write(p.join("report.txt"), "Observed fixture").unwrap();
    evidence(&p, id, "2026-09-27T00:00:00Z");
    call(
        &p,
        "spec-review",
        &[
            ("--id", id),
            ("--expected", &sha(&p)),
            ("--reviewer", "fixture reviewer"),
            ("--verdict", "go"),
            ("--conclusion", "Fixture contract satisfied"),
        ],
    )
    .unwrap();
    assert_eq!(
        call(&p, "spec-check", &[("--id", id)]).unwrap().status,
        "passed"
    );
    request(&p, id, "result", "result");
    evidence(&p, id, "2026-09-27T01:00:00Z");
    assert_eq!(row(&p, id)["requirements_agreement"], "accepted");
    let respond = json!({"action":"respond","request_id":"result","content_sha256":row(&p,id)["content_sha256"],"decision":"accepted","author":"Автор","comment":"","verified_revision":"fixture-v1"});
    let snapshot = call(&p, "spec-read", &[("--id", id)]).unwrap();
    let version = snapshot
        .measurements
        .iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap();
    put(
        &p,
        json!({"decisions":[{"id":id,"decision":"accepted","decided_by":"Автор","comment":"","verified_revision":"fixture-v1","change_sha256":version["change_sha256"],"inputs_sha256":version["inputs_sha256"]}]}),
    );
    call(
        &p,
        "spec-decide",
        &[("--expected", &sha(&p)), ("--input", "input.json")],
    )
    .unwrap();
    assert_eq!(row(&p, id)["category"], "completed");
    let before = sha(&p);
    let view = ui(&p);
    assert_eq!(sha(&p), before);
    assert_eq!(view["changes"].as_array().unwrap().len(), 3);
    request(&p, id, "return", "result");
    response(&p, id, "return", "needs_changes", "").unwrap_err();
    mutate(&p,id,json!({"action":"respond","request_id":"return","content_sha256":row(&p,id)["content_sha256"],"decision":"needs_changes","author":"Автор","comment":"Исправить","verified_revision":"fixture-v1"})).unwrap();
    assert_eq!(row(&p, id)["category"], "in_work");
    request(&p, id, "stale", "result");
    fs::write(p.join("logic.txt"), "v2").unwrap();
    let before = sha(&p);
    let mut stale = respond;
    stale["request_id"] = json!("stale");
    assert!(mutate(&p, id, stale).unwrap_err().contains("DecisionStale"));
    assert_eq!(sha(&p), before);
    fs::remove_file(p.join("logic.txt")).unwrap();
    assert_eq!(row(&p, id)["category"], "unknown");
    request(&p, id, "missing input question", "question");
    assert_eq!(row(&p, id)["category"], "needs_decision");
    assert!(row(&p, id)["diagnostic"].is_string());
}

// highgrade: HG-0052-S6, HG-0052-S7
#[test]
fn answers_in_reverse_request_order_use_recorded_sequence() {
    let p = setup();
    let id = "HG-0001";
    request(&p, id, "first", "requirements");
    request(&p, id, "second", "requirements");
    response(&p, id, "second", "accepted", "").unwrap();
    response(&p, id, "first", "needs_changes", "Поправить").unwrap();
    assert_eq!(row(&p, id)["requirements_agreement"], "needs_changes");
    let r = row(&p, id);
    assert_eq!(r["history"][0]["response"]["sequence"], 2);
    assert_eq!(r["history"][1]["response"]["sequence"], 1);
}
