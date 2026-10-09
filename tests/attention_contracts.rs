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
fn catalog_bytes(p: &Path) -> BTreeMap<String, Vec<u8>> {
    fn collect(p: &Path, rel: &str, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(p.join(rel)).unwrap() {
            let entry = entry.unwrap();
            let path = format!("{rel}/{}", entry.file_name().to_str().unwrap());
            if entry.file_type().unwrap().is_dir() {
                collect(p, &path, files);
            } else {
                files.insert(path, fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    collect(p, "specs", &mut files);
    files
}
fn rejects_without_writes(p: &Path, id: &str, value: Value, expected: &str) {
    let before = catalog_bytes(p);
    let error = mutate(p, id, value)
        .err()
        .expect("mutation must be rejected");
    assert!(error.contains(expected), "{error}");
    assert_eq!(catalog_bytes(p), before);
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
    let before = catalog_bytes(&p);
    response(&p, id, "approval", "accepted", "").unwrap();
    assert_eq!(catalog_bytes(&p), before);
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
fn integrated_fixture(p: &Path, number: u32) -> String {
    let id = format!("HG-{number:04}");
    call(
        p,
        "spec-new",
        &[("--title", "История"), ("--expected", &sha(p))],
    )
    .unwrap();
    edit(
        p,
        &id,
        json!({"goal":"Результат","rationale":"Причина","scope":"Область",
        "tasks":[{"id":format!("{id}-T1"),"description":"Работа","done":true}],
        "operations":[{"action":"add","requirement":{"id":format!("{id}-R1"),"title":"Требование","statement":"Условие",
            "scenarios":[{"id":format!("{id}-S1"),"given":"Дано","when":"Действие","then":"Результат","verification":"Наблюдение"}]}}]}),
    );
    let source = format!("{id}-logic.txt");
    let report = format!("{id}-report.txt");
    fs::write(p.join(&source), "v1").unwrap();
    fs::write(p.join(&report), "Fixture observation").unwrap();
    put(
        p,
        json!({"command":"isolated fixture observation","captured_at":"2026-09-29T00:00:00Z","method":"manual",
        "scenario":format!("{id}-S1"),"outcome":"passed","observation":"fixture satisfied","inputs":[source],"report":report}),
    );
    assert_eq!(
        call(
            p,
            "spec-evidence",
            &[
                ("--id", &id),
                ("--expected", &sha(p)),
                ("--input", "input.json")
            ]
        )
        .unwrap()
        .status,
        "passed"
    );
    assert_eq!(
        call(
            p,
            "spec-review",
            &[
                ("--id", &id),
                ("--expected", &sha(p)),
                ("--reviewer", "fixture reviewer"),
                ("--verdict", "go"),
                ("--conclusion", "Fixture contract satisfied")
            ]
        )
        .unwrap()
        .status,
        "passed"
    );
    assert_eq!(
        call(
            p,
            "spec-integrate",
            &[("--id", &id), ("--expected", &sha(p))]
        )
        .unwrap()
        .status,
        "passed"
    );
    id
}
fn accept_fixture(p: &Path, id: &str) {
    request(p, id, "result", "result");
    let r = row(p, id);
    assert_eq!(mutate(p,id,json!({"action":"respond","request_id":"result","content_sha256":r["content_sha256"],
        "decision":"accepted","author":"Fixture user","comment":"","verified_revision":"fixture-build-v1"})).unwrap().status,"passed");
}
// highgrade: HG-0059-S1
#[test]
fn integrated_history_is_not_current_work_or_current_acceptance() {
    let p = setup();
    request(&p, "HG-0002", "question", "question");
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
    let stale = integrated_fixture(&p, 4);
    accept_fixture(&p, &stale);
    fs::write(p.join(format!("{stale}-logic.txt")), "v2").unwrap();
    let current = integrated_fixture(&p, 5);
    accept_fixture(&p, &current);
    let missing = integrated_fixture(&p, 6);
    fs::remove_file(p.join(format!("{missing}-logic.txt"))).unwrap();
    let before = sha(&p);
    let projection = ui(&p);
    assert_eq!(projection, ui(&p));
    assert_eq!(sha(&p), before);
    for (id, category, integrated) in [
        ("HG-0001", "in_work", false),
        ("HG-0002", "needs_decision", false),
        ("HG-0003", "cancelled", false),
        ("HG-0004", "historical", true),
        ("HG-0005", "completed", true),
        ("HG-0006", "unknown", true),
    ] {
        let r = row(&p, id);
        assert_eq!(r["category"], category, "{id}");
        assert_eq!(r["integrated"], integrated, "{id}");
        assert_eq!(projection["counts"][category], 1, "{category}");
    }
    assert_eq!(row(&p, &stale)["technical_ready"], false);
    assert_eq!(row(&p, &stale)["human"], "stale");
    request(&p, &stale, "late question", "question");
    assert_eq!(row(&p, &stale)["category"], "needs_decision");
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

// highgrade: HG-0052-S4, HG-0052-S6, HG-0052-S9, HG-0061-S5
#[test]
fn repeated_requests_recheck_readiness_and_preserve_unrelated_questions() {
    let p = setup();
    let id = "HG-0001";
    let request = json!({"action":"request","request_id":"approval","kind":"requirements",
        "reason":"Проверить требования","content_sha256":row(&p,id)["content_sha256"]});
    mutate(&p, id, request.clone()).unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, id, request.clone()).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    for (field, value) in [("reason", "Другое основание"), ("kind", "question")] {
        let mut different = request.clone();
        different[field] = json!(value);
        rejects_without_writes(&p, id, different, "AttentionRequestConflict");
    }
    edit(&p, id, json!({"questions":["Уточнить условия"]}));
    assert_eq!(row(&p, id)["content_sha256"], request["content_sha256"]);
    rejects_without_writes(&p, id, request, "RequirementsNotPrepared");
    let question = json!({"action":"request","request_id":"question","kind":"question",
        "reason":"Уточнить условия","content_sha256":row(&p,id)["content_sha256"]});
    mutate(&p, id, question.clone()).unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, id, question).unwrap();
    assert_eq!(catalog_bytes(&p), before);
}

// highgrade: HG-0052-S4, HG-0052-S6, HG-0052-S8, HG-0052-S9, HG-0061-S5
#[test]
fn repeated_result_requests_recheck_inputs_revision_and_readiness() {
    let p = TestDir::new("hg-attention-repeat-request-");
    let id = integrated_fixture(&p, 1);
    let request = json!({"action":"request","request_id":"result","kind":"result",
        "reason":"Принять результат","content_sha256":row(&p,&id)["content_sha256"]});
    mutate(&p, &id, request.clone()).unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, &id, request.clone()).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    let report = p.join(format!("{id}-report.txt"));
    fs::write(&report, "Changed report, unchanged implementation").unwrap();
    rejects_without_writes(&p, &id, request.clone(), "DecisionNotReady");
    fs::write(&report, "Fixture observation").unwrap();
    let source = p.join(format!("{id}-logic.txt"));
    fs::write(&source, "v2").unwrap();
    rejects_without_writes(&p, &id, request.clone(), "DecisionStale");
    fs::remove_file(&source).unwrap();
    let before = catalog_bytes(&p);
    assert!(mutate(&p, &id, request.clone()).is_err());
    assert_eq!(catalog_bytes(&p), before);
    fs::write(&source, "v1").unwrap();
    put(
        &p,
        json!({"command":"isolated fixture observation","captured_at":"2026-09-29T01:00:00Z",
        "method":"manual","scenario":format!("{id}-S1"),"outcome":"passed","observation":"fixture satisfied",
        "inputs":[format!("{id}-logic.txt")],"report":format!("{id}-report.txt")}),
    );
    call(
        &p,
        "spec-evidence",
        &[
            ("--id", &id),
            ("--expected", &sha(&p)),
            ("--input", "input.json"),
        ],
    )
    .unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, &id, request.clone()).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    put(
        &p,
        json!({"command":"isolated fixture observation","captured_at":"2026-09-29T02:00:00Z",
        "method":"manual","scenario":format!("{id}-S1"),"outcome":"passed","observation":"a materially different observation",
        "inputs":[format!("{id}-logic.txt")],"report":format!("{id}-report.txt")}),
    );
    call(
        &p,
        "spec-evidence",
        &[
            ("--id", &id),
            ("--expected", &sha(&p)),
            ("--input", "input.json"),
        ],
    )
    .unwrap();
    assert_eq!(row(&p, &id)["content_sha256"], request["content_sha256"]);
    rejects_without_writes(&p, &id, request, "DecisionStale");
}

fn result_response(p: &Path, id: &str, key: &str, revision: &str) -> Value {
    json!({"action":"respond","request_id":key,"content_sha256":row(p,id)["content_sha256"],
        "decision":"accepted","author":"Автор","comment":"","verified_revision":revision})
}
fn decide_result(p: &Path, id: &str, revision: &str) {
    let loaded = call(p, "spec-read", &[("--id", id)]).unwrap();
    let snapshot = loaded
        .measurements
        .iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap();
    put(
        p,
        json!({"decisions":[{"id":id,"decision":"accepted","decided_by":"Автор","comment":"",
        "verified_revision":revision,"change_sha256":snapshot["change_sha256"],"inputs_sha256":snapshot["inputs_sha256"]}]}),
    );
    call(
        p,
        "spec-decide",
        &[("--expected", &sha(p)), ("--input", "input.json")],
    )
    .unwrap();
}

// highgrade: HG-0070-S1, HG-0070-S3
#[test]
fn result_status_without_build_id_keeps_snapshot_history_and_replay_gates() {
    let p = setup();
    let id = integrated_fixture(&p, 4);
    request(&p, &id, "result", "result");
    let value = json!({"action":"respond","request_id":"result",
        "content_sha256":row(&p,&id)["content_sha256"],"decision":"accepted",
        "author":"Локальный интерфейс","comment":""});
    mutate(&p, &id, value.clone()).unwrap();
    let store = specs::load(&p).unwrap().0;
    let decision = store.changes[&id].acceptance.last().unwrap();
    assert!(decision.verified_revision.is_empty());
    assert!(!decision.change_sha256.is_empty() && !decision.inputs_sha256.is_empty());
    assert_eq!(row(&p, &id)["category"], "completed");
    let before = catalog_bytes(&p);
    mutate(&p, &id, value.clone()).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    let mut different = value.clone();
    different["verified_revision"] = json!("unrelated-build");
    rejects_without_writes(&p, &id, different, "AttentionAlreadyAnswered");
    request(&p, &id, "return", "result");
    response(&p, &id, "return", "needs_changes", "").unwrap();
    assert_eq!(row(&p, &id)["human"], "needs_changes");
    assert_eq!(specs::load(&p).unwrap().0.changes[&id].acceptance.len(), 2);
    fs::write(p.join(format!("{id}-logic.txt")), "changed input").unwrap();
    rejects_without_writes(&p, &id, value, "DecisionStale");
}

// highgrade: HG-0070-S2
#[test]
fn saved_result_build_id_is_optional_and_reused_without_user_input() {
    let p = setup();
    let id = integrated_fixture(&p, 4);
    let request = json!({"action":"request","request_id":"known","kind":"result",
        "reason":"Уже проверенный результат","content_sha256":row(&p,&id)["content_sha256"],
        "verified_revision":"fixture-build-known"});
    mutate(&p, &id, request.clone()).unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, &id, request.clone()).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    let mut changed = request;
    changed["verified_revision"] = json!("other-build");
    rejects_without_writes(&p, &id, changed, "AttentionRequestConflict");
    response(&p, &id, "known", "accepted", "").unwrap();
    let store = specs::load(&p).unwrap().0;
    assert_eq!(
        store.changes[&id]
            .acceptance
            .last()
            .unwrap()
            .verified_revision,
        "fixture-build-known"
    );
    let before = catalog_bytes(&p);
    response(&p, &id, "known", "accepted", "").unwrap();
    assert_eq!(catalog_bytes(&p), before);
}

fn direct_decision(p: &Path, id: &str) -> Value {
    let loaded = call(p, "spec-read", &[("--id", id)]).unwrap();
    let snapshot = loaded
        .measurements
        .iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap();
    json!({"id":id,"decision":"accepted","decided_by":"Автор",
        "change_sha256":snapshot["change_sha256"],"inputs_sha256":snapshot["inputs_sha256"]})
}

fn known_result(p: &Path, id: &str, key: &str, version: &str) {
    mutate(
        p,
        id,
        json!({"action":"request","request_id":key,"kind":"result",
        "reason":"Готовый результат","content_sha256":row(p,id)["content_sha256"],
        "verified_revision":version}),
    )
    .unwrap();
}

// highgrade: HG-0070-S2, HG-0070-S3
#[test]
fn unknown_result_reuses_unique_saved_metadata_and_closes_the_same_snapshot() {
    let p = setup();
    let id = integrated_fixture(&p, 4);
    request(&p, &id, "unknown", "result");
    known_result(&p, &id, "known", "saved-v1");
    response(&p, &id, "unknown", "accepted", "").unwrap();
    let store = specs::load(&p).unwrap().0;
    let c = &store.changes[&id];
    assert_eq!(c.acceptance.last().unwrap().verified_revision, "saved-v1");
    assert!(
        c.attention
            .as_ref()
            .unwrap()
            .requests
            .iter()
            .all(|r| r.response.is_some())
    );
    assert_eq!(row(&p, &id)["category"], "completed");
    let before = catalog_bytes(&p);
    response(&p, &id, "unknown", "accepted", "").unwrap();
    response(&p, &id, "known", "accepted", "").unwrap();
    assert_eq!(catalog_bytes(&p), before);
    let mut different = result_response(&p, &id, "unknown", "other-build");
    different["comment"] = json!("different");
    rejects_without_writes(&p, &id, different, "AttentionAlreadyAnswered");
}

// highgrade: HG-0070-S2
#[test]
fn direct_decision_reuses_the_saved_build_and_closes_the_result_request() {
    let p = setup();
    let id = integrated_fixture(&p, 4);
    known_result(&p, &id, "known", "saved-v1");
    put(&p, json!({"decisions":[direct_decision(&p,&id)]}));
    call(
        &p,
        "spec-decide",
        &[("--expected", &sha(&p)), ("--input", "input.json")],
    )
    .unwrap();
    let store = specs::load(&p).unwrap().0;
    let c = &store.changes[&id];
    assert_eq!(c.acceptance.last().unwrap().verified_revision, "saved-v1");
    assert_eq!(
        c.attention.as_ref().unwrap().requests[0]
            .response
            .as_ref()
            .unwrap()
            .verified_revision
            .as_deref(),
        Some("saved-v1")
    );
    assert_eq!(row(&p, &id)["category"], "completed");
    known_result(&p, &id, "next", "saved-v2");
    let mut decision = direct_decision(&p, &id);
    decision["verified_revision"] = json!("different-build");
    put(&p, json!({"decisions":[decision]}));
    let before = catalog_bytes(&p);
    let error = call(
        &p,
        "spec-decide",
        &[("--expected", &sha(&p)), ("--input", "input.json")],
    )
    .unwrap_err();
    assert!(error.contains("DecisionRevisionConflict"), "{error}");
    assert_eq!(catalog_bytes(&p), before);
}

// highgrade: HG-0070-S2, HG-0070-S3
#[test]
fn ambiguous_saved_build_rejects_the_whole_direct_decision_batch() {
    let p = setup();
    let first = integrated_fixture(&p, 4);
    let second = integrated_fixture(&p, 5);
    known_result(&p, &first, "first", "saved-v1");
    known_result(&p, &second, "second", "saved-v2");
    known_result(&p, &second, "alternative", "saved-v3");
    put(
        &p,
        json!({"decisions":[direct_decision(&p,&first),direct_decision(&p,&second)]}),
    );
    let before = catalog_bytes(&p);
    let error = call(
        &p,
        "spec-decide",
        &[("--expected", &sha(&p)), ("--input", "input.json")],
    )
    .unwrap_err();
    assert!(error.contains("DecisionRevisionConflict"), "{error}");
    assert_eq!(catalog_bytes(&p), before);
    for id in [&first, &second] {
        assert!(specs::load(&p).unwrap().0.changes[id].acceptance.is_empty());
        assert_eq!(row(&p, id)["category"], "needs_decision");
    }
}

// highgrade: HG-0052-S4, HG-0052-S6, HG-0052-S9, HG-0061-S5
#[test]
fn repeated_result_responses_preserve_exact_revision_and_recheck_gates() {
    let p = TestDir::new("hg-attention-repeat-response-");
    let id = integrated_fixture(&p, 1);
    request(&p, &id, "result", "result");
    let response = result_response(&p, &id, "result", "fixture-v1");
    mutate(&p, &id, response.clone()).unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, &id, response.clone()).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    for revision in ["", " ", "fixture-v2"] {
        let mut different = response.clone();
        different["verified_revision"] = json!(revision);
        rejects_without_writes(&p, &id, different, "AttentionAlreadyAnswered");
    }
    for (field, value) in [
        ("author", "Другой автор"),
        ("comment", "Другой комментарий"),
    ] {
        let mut different = response.clone();
        different[field] = json!(value);
        rejects_without_writes(&p, &id, different, "AttentionAlreadyAnswered");
    }
    let mut different = response.clone();
    different["decision"] = json!("needs_changes");
    different["comment"] = json!("Доработать");
    rejects_without_writes(&p, &id, different, "AttentionAlreadyAnswered");
    let report = p.join(format!("{id}-report.txt"));
    fs::write(&report, "Changed report, unchanged implementation").unwrap();
    rejects_without_writes(&p, &id, response.clone(), "DecisionNotReady");
    fs::write(&report, "Fixture observation").unwrap();
    let source = p.join(format!("{id}-logic.txt"));
    fs::write(&source, "v2").unwrap();
    rejects_without_writes(&p, &id, response.clone(), "DecisionStale");
    fs::remove_file(&source).unwrap();
    let before = catalog_bytes(&p);
    assert!(mutate(&p, &id, response.clone()).is_err());
    assert_eq!(catalog_bytes(&p), before);
    fs::write(&source, "v1").unwrap();
    let before = catalog_bytes(&p);
    mutate(&p, &id, response).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    assert_eq!(specs::load(&p).unwrap().0.changes[&id].acceptance.len(), 1);
}

// highgrade: HG-0052-S6, HG-0052-S9, HG-0061-S5
#[test]
fn result_responses_keep_their_revision_when_later_decisions_share_snapshot() {
    let p = TestDir::new("hg-attention-response-history-");
    let id = integrated_fixture(&p, 1);
    request(&p, &id, "first", "result");
    request(&p, &id, "same-decision", "result");
    decide_result(&p, &id, "fixture-v1");
    request(&p, &id, "later", "result");
    decide_result(&p, &id, "fixture-v2");
    let before = catalog_bytes(&p);
    for key in ["first", "same-decision"] {
        mutate(&p, &id, result_response(&p, &id, key, "fixture-v1")).unwrap();
        assert_eq!(catalog_bytes(&p), before);
        rejects_without_writes(
            &p,
            &id,
            result_response(&p, &id, key, "fixture-v2"),
            "AttentionAlreadyAnswered",
        );
    }
    mutate(&p, &id, result_response(&p, &id, "later", "fixture-v2")).unwrap();
    assert_eq!(catalog_bytes(&p), before);
    rejects_without_writes(
        &p,
        &id,
        result_response(&p, &id, "later", "fixture-v1"),
        "AttentionAlreadyAnswered",
    );
    assert_eq!(specs::load(&p).unwrap().0.changes[&id].acceptance.len(), 2);
}

// highgrade: HG-0052-S6, HG-0052-S9, HG-0061-S6
#[test]
fn legacy_result_responses_require_unambiguous_acceptance_revision() {
    let p = TestDir::new("hg-attention-legacy-response-");
    let id = integrated_fixture(&p, 1);
    request(&p, &id, "result", "result");
    let response = result_response(&p, &id, "result", "fixture-v1");
    mutate(&p, &id, response.clone()).unwrap();
    // Model the historical response format, which did not store the verified build.
    let spec = p.join(format!("specs/changes/{id}/spec.json"));
    let mut value: Value = serde_json::from_slice(&fs::read(&spec).unwrap()).unwrap();
    value["attention"]["requests"][0]["response"]
        .as_object_mut()
        .unwrap()
        .remove("verified_revision");
    let legacy_response = value["attention"]["requests"][0]["response"].clone();
    // A no-op must preserve even non-canonical historical formatting.
    fs::write(&spec, serde_json::to_vec(&value).unwrap()).unwrap();
    let before = catalog_bytes(&p);
    let before_sha = sha(&p);
    let repeated = mutate(&p, &id, response.clone()).unwrap();
    assert_eq!(repeated.status, "passed");
    assert_eq!(repeated.measurements[0]["store_sha256"], before_sha);
    assert_eq!(catalog_bytes(&p), before);
    rejects_without_writes(
        &p,
        &id,
        result_response(&p, &id, "result", "fixture-v2"),
        "AttentionAlreadyAnswered",
    );
    // Two otherwise identical decisions in the same second cannot identify which
    // verified build closed a legacy response. Neither candidate is a safe repeat.
    let results = p.join(format!("specs/changes/{id}/results.json"));
    let mut value: Value = serde_json::from_slice(&fs::read(&results).unwrap()).unwrap();
    let mut ambiguous = value["acceptance"][0].clone();
    ambiguous["verified_revision"] = json!("fixture-v2");
    value["acceptance"].as_array_mut().unwrap().push(ambiguous);
    fs::write(&results, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    rejects_without_writes(&p, &id, response, "AttentionAlreadyAnswered");
    rejects_without_writes(
        &p,
        &id,
        result_response(&p, &id, "result", "fixture-v2"),
        "AttentionAlreadyAnswered",
    );
    request(&p, &id, "new-result", "result");
    mutate(
        &p,
        &id,
        result_response(&p, &id, "new-result", "fixture-v3"),
    )
    .unwrap();
    let value: Value = serde_json::from_slice(&fs::read(&spec).unwrap()).unwrap();
    assert_eq!(
        value["attention"]["requests"][0]["response"],
        legacy_response
    );
    assert_eq!(
        value["attention"]["requests"][1]["response"]["verified_revision"],
        "fixture-v3"
    );
}
