mod support;
use highgrade::specs;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use support::TestDir;

fn root() -> TestDir {
    TestDir::new("hg-catalog-")
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
fn create(root: &Path) {
    call(
        root,
        "spec-new",
        &[("--title", "Проверка"), ("--expected", &sha(root))],
    )
    .unwrap();
}
fn json(root: &Path, rel: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(rel)).unwrap()).unwrap()
}
fn put(root: &Path, rel: &str, v: &Value) {
    fs::write(root.join(rel), serde_json::to_vec_pretty(v).unwrap()).unwrap();
}

// highgrade: HG-0002-S1
#[test]
fn project_catalog_survives_service_removal_and_preserves_siblings() {
    let root = root();
    create(&root);
    let first = fs::read(root.join("specs/changes/HG-0001/spec.json")).unwrap();
    let modified = fs::metadata(root.join("specs/changes/HG-0001/spec.json"))
        .unwrap()
        .modified()
        .unwrap();
    create(&root);
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0001/spec.json")).unwrap(),
        first
    );
    assert_eq!(
        fs::metadata(root.join("specs/changes/HG-0001/spec.json"))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    let sibling_path = root.join("specs/changes/HG-0002/spec.json");
    let sibling = fs::read(&sibling_path).unwrap();
    let sibling_modified = fs::metadata(&sibling_path).unwrap().modified().unwrap();
    edit(&root, "HG-0001", "goal", json!("Изменённая цель")).unwrap();
    assert_eq!(fs::read(&sibling_path).unwrap(), sibling);
    assert_eq!(
        fs::metadata(&sibling_path).unwrap().modified().unwrap(),
        sibling_modified
    );
    assert_eq!(
        json(&root, "specs/changes/HG-0001/spec.json")["goal"],
        "Изменённая цель"
    );
    assert!(!root.join("specs/changes/HG-0001/results.json").exists());
    assert!(!root.join(specs::STORE).exists());
    assert!(json(&root, "specs/catalog.json").get("changes").is_none());
    fs::remove_dir_all(root.join(".highgrade")).unwrap();
    assert_eq!(specs::load(&root).unwrap().0.changes.len(), 2);
    create(&root);
    assert!(root.join("specs/changes/HG-0003/spec.json").exists());
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn foreign_directory_and_unknown_fields_are_not_adopted() {
    let root = root();
    fs::create_dir(root.join("specs")).unwrap();
    fs::write(root.join("specs/mine.md"), "Проект").unwrap();
    assert!(
        call(&root, "spec-new", &[("--expected", "absent")])
            .unwrap_err()
            .contains("ForeignCatalog")
    );
    assert_eq!(
        fs::read_to_string(root.join("specs/mine.md")).unwrap(),
        "Проект"
    );
    assert!(!root.join("specs/catalog.json").exists());
    fs::remove_dir_all(root).unwrap();
}
// highgrade: HG-0002-S2
#[test]
fn explicit_migration_preserves_all_data_and_detects_legacy_writer() {
    let root = root();
    fs::create_dir_all(root.join(".highgrade/specs")).unwrap();
    let legacy = include_bytes!("fixtures/native-v1/store.json");
    fs::write(root.join(specs::STORE), legacy).unwrap();
    let before = specs::load(&root).unwrap();
    call(
        &root,
        "spec-migrate",
        &[("--expected", &before.1), ("--to", "directory")],
    )
    .unwrap();
    let after = specs::load(&root).unwrap().0;
    assert_eq!(before.0.changes, after.changes);
    assert_eq!(before.0.requirements, after.requirements);
    assert_eq!(before.0.origins, after.origins);
    assert_eq!(
        fs::read(root.join(format!("specs-backup-{}.json", before.1))).unwrap(),
        legacy
    );
    let mut old: Value = serde_json::from_slice(legacy).unwrap();
    old["next_number"] = json!(100);
    put(&root, specs::STORE, &old);
    assert!(
        specs::load(&root)
            .unwrap_err()
            .contains("LegacyStoreConflict")
    );
    fs::remove_dir_all(root).unwrap();
}
// highgrade: HG-0002-S2
#[test]
fn interrupted_transaction_refuses_reads_and_recovers_only_observed_targets() {
    let root = root();
    create(&root);
    let original = fs::read(root.join("specs/changes/HG-0001/spec.json")).unwrap();
    let mut changed: Value = serde_json::from_slice(&original).unwrap();
    changed["title"] = json!("Новое название");
    let desired = serde_json::to_string_pretty(&changed).unwrap();
    let journal = json!({"specs/changes/HG-0001/spec.json":{"before":highgrade::hash(&original),"after":desired}});
    put(&root, "specs/transaction.json", &journal);
    assert!(
        specs::load(&root)
            .unwrap_err()
            .contains("CatalogRecoveryRequired")
    );
    let transaction_sha = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    assert!(call(&root, "spec-recover", &[("--expected", "wrong")]).is_err());
    call(&root, "spec-recover", &[("--expected", &transaction_sha)]).unwrap();
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0001"].title,
        "Новое название"
    );
    // A partially applied journal is idempotently rolled forward.
    put(&root, "specs/transaction.json", &journal);
    call(&root, "spec-recover", &[("--expected", &transaction_sha)]).unwrap();
    let bad = json!({"../outside":{"before":null,"after":"delete nothing"}});
    put(&root, "specs/transaction.json", &bad);
    let h = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    assert!(
        call(&root, "spec-recover", &[("--expected", &h)])
            .unwrap_err()
            .contains("UnsafeTransactionTarget")
    );
    assert!(root.join("specs/transaction.json").exists());
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0002-S2
#[test]
fn malformed_recovery_never_unblocks_catalog() {
    let root = root();
    create(&root);
    let p = "specs/changes/HG-0001/spec.json";
    let original = fs::read(root.join(p)).unwrap();
    put(
        &root,
        "specs/transaction.json",
        &json!({p:{"before":highgrade::hash(&original),"after":"{}"}}),
    );
    let h = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    assert!(call(&root, "spec-recover", &[("--expected", &h)]).is_err());
    assert_eq!(fs::read(root.join(p)).unwrap(), original);
    assert!(root.join("specs/transaction.json").exists());
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0002-S1
#[test]
fn foreign_structured_files_are_not_deleted() {
    let root = root();
    fs::create_dir_all(root.join("specs/requirements")).unwrap();
    fs::write(
        root.join("specs/requirements/OTHER-R1.json"),
        "чужие данные",
    )
    .unwrap();
    assert!(
        call(&root, "spec-new", &[("--expected", "absent")])
            .unwrap_err()
            .contains("ForeignCatalog")
    );
    assert_eq!(
        fs::read_to_string(root.join("specs/requirements/OTHER-R1.json")).unwrap(),
        "чужие данные"
    );
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0002-S2
#[test]
fn recovery_reconstructs_partial_staging_and_missing_first_document() {
    let root = root();
    create(&root);
    let p = "specs/changes/HG-0001/spec.json";
    let document = fs::read(root.join(p)).unwrap();
    put(
        &root,
        "specs/transaction.json",
        &json!({p:{"before":null,"after":String::from_utf8(document.clone()).unwrap()}}),
    );
    fs::remove_file(root.join(p)).unwrap();
    let stage = root.join(format!(
        ".highgrade/specs/staging/{}.part",
        highgrade::hash(p.as_bytes())
    ));
    fs::write(&stage, b"partial").unwrap();
    let h = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    call(&root, "spec-recover", &[("--expected", &h)]).unwrap();
    assert_eq!(fs::read(root.join(p)).unwrap(), document);
    assert!(!stage.exists());
    fs::remove_dir_all(root).unwrap();
}

fn save_change(root: &Path, c: &Value) {
    put(root, "edit.json", c);
    call(
        root,
        "spec-save",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(root)),
            ("--input", "edit.json"),
        ],
    )
    .unwrap();
}
fn observe(root: &Path) {
    put(
        root,
        "observation.json",
        &json!({"command":"Ручная проверка","captured_at":"2026-09-25","method":"manual","scenario":"HG-0001-S1","outcome":"passed","observation":"Результат проверен","inputs":["logic.txt"],"report":"report.txt"}),
    );
    call(
        root,
        "spec-evidence",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(root)),
            ("--input", "observation.json"),
        ],
    )
    .unwrap();
}
fn review(root: &Path) {
    call(
        root,
        "spec-review",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(root)),
            ("--reviewer", "Независимый проверяющий"),
            ("--verdict", "go"),
            ("--conclusion", "Сценарий и покрытие проверены"),
        ],
    )
    .unwrap();
}
// highgrade: HG-0033-S1, HG-0033-S2
#[test]
fn compact_history_preserves_observations_and_rejects_corruption() {
    let root = root();
    create(&root);
    fs::write(root.join("logic.txt"), "logic").unwrap();
    fs::write(root.join("report.txt"), "report").unwrap();
    for _ in 0..20 {
        observe(&root);
    }
    let before = specs::load(&root).unwrap().0;
    let path = "specs/changes/HG-0001/results.json";
    let old_size = fs::metadata(root.join(path)).unwrap().len();
    call(
        &root,
        "spec-migrate",
        &[("--to", "compact"), ("--expected", &sha(&root))],
    )
    .unwrap();
    assert_eq!(json(&root, "specs/catalog.json")["schema_version"], 4);
    assert_eq!(before.changes, specs::load(&root).unwrap().0.changes);
    assert!(fs::metadata(root.join(path)).unwrap().len() < old_size);
    observe(&root);
    let after = specs::load(&root).unwrap().0;
    assert_eq!(after.changes["HG-0001"].history.len(), 21);
    let mut data = json(&root, path);
    let objects = data["history"]["objects"].as_object_mut().unwrap();
    let key = objects.keys().next().unwrap().clone();
    objects.insert(key, json!("corrupt"));
    put(&root, path, &data);
    assert!(
        specs::load(&root)
            .unwrap_err()
            .contains("HistoryObjectHashMismatch")
    );
}
// highgrade: HG-0033-S1, HG-0033-S2
#[test]
fn compact_catalog_accepts_empty_changes_and_new_changes() {
    let root = root();
    create(&root);
    call(
        &root,
        "spec-migrate",
        &[("--to", "compact"), ("--expected", &sha(&root))],
    )
    .unwrap();
    create(&root);
    assert_eq!(specs::load(&root).unwrap().0.changes.len(), 2);
    assert!(!root.join("specs/changes/HG-0002/results.json").exists());
}

// highgrade: HG-0033-S2
#[test]
fn compact_transaction_limit_and_recovery_preserve_catalog() {
    let root = root();
    create(&root);
    call(
        &root,
        "spec-migrate",
        &[("--to", "compact"), ("--expected", &sha(&root))],
    )
    .unwrap();
    let before = sha(&root);
    let oversized = "\\".repeat(3 * 1024 * 1024);
    let err = call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-0001"),
            ("--expected", &before),
            ("--reviewer", "reviewer"),
            ("--verdict", "go"),
            ("--conclusion", &oversized),
        ],
    )
    .unwrap_err();
    assert!(err.contains("TransactionTooLarge"), "{err}");
    assert_eq!(before, sha(&root));
    assert!(!root.join("specs/transaction.json").exists());
    let path = "specs/changes/HG-0001/spec.json";
    let original = fs::read(root.join(path)).unwrap();
    let mut desired: Value = serde_json::from_slice(&original).unwrap();
    desired["title"] = json!("Recovered");
    put(
        &root,
        "specs/transaction.json",
        &json!({path:{"before":highgrade::hash(&original),"after":serde_json::to_string_pretty(&desired).unwrap()}}),
    );
    assert!(specs::load(&root).is_err());
    let expected = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    call(&root, "spec-recover", &[("--expected", &expected)]).unwrap();
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0001"].title,
        "Recovered"
    );
}
// highgrade: HG-0002-S3
#[test]
fn integrated_recheck_retains_history_and_invalidates_old_acceptance() {
    let root = root();
    create(&root);
    let mut c = serde_json::to_value(&specs::load(&root).unwrap().0.changes["HG-0001"]).unwrap();
    for field in ["goal", "rationale", "scope"] {
        c[field] = json!("Проверка суммы");
    }
    c["tasks"][0]["done"] = json!(true);
    c["tasks"][0]["description"] = json!("Проверить сумму");
    let r = &mut c["operations"][0]["requirement"];
    r["title"] = json!("Сумма");
    r["statement"] = json!("Вернуть сумму");
    for field in ["given", "when", "then", "verification"] {
        r["scenarios"][0][field] = json!("2 + 3 = 5");
    }
    save_change(&root, &c);
    fs::write(root.join("logic.txt"), "2+3").unwrap();
    fs::write(root.join("report.txt"), "5").unwrap();
    observe(&root);
    review(&root);
    assert_eq!(
        call(&root, "spec-check", &[("--id", "HG-0001")])
            .unwrap()
            .status,
        "passed"
    );
    let read = call(&root, "spec-read", &[("--id", "HG-0001")]).unwrap();
    let hashes = read
        .measurements
        .iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap();
    put(
        &root,
        "decision.json",
        &json!({"decisions":[{"id":"HG-0001","decision":"accepted","decided_by":"Тестовая симуляция пользователя","change_sha256":hashes["change_sha256"],"inputs_sha256":hashes["inputs_sha256"],"verified_revision":"fixture","comment":"Изолированный тест"}]}),
    );
    call(
        &root,
        "spec-decide",
        &[("--expected", &sha(&root)), ("--input", "decision.json")],
    )
    .unwrap();
    assert_eq!(
        call(
            &root,
            "spec-integrate",
            &[("--id", "HG-0001"), ("--expected", &sha(&root))]
        )
        .unwrap()
        .status,
        "passed"
    );
    let requirement = fs::read(root.join("specs/requirements/HG-0001-R1.json")).unwrap();
    let spec = fs::read(root.join("specs/changes/HG-0001/spec.json")).unwrap();
    fs::write(root.join("logic.txt"), "2 + 3").unwrap();
    assert_eq!(
        call(&root, "spec-check", &[("--id", "HG-0001")])
            .unwrap()
            .status,
        "failed"
    );
    observe(&root);
    review(&root);
    assert_eq!(
        call(&root, "spec-check", &[("--id", "HG-0001")])
            .unwrap()
            .status,
        "passed"
    );
    let listed = call(&root, "spec-list", &[]).unwrap();
    assert_eq!(listed.measurements[0]["changes"][0]["human"], "stale");
    assert_eq!(
        fs::read(root.join("specs/requirements/HG-0001-R1.json")).unwrap(),
        requirement
    );
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0001/spec.json")).unwrap(),
        spec
    );
    let results = json(&root, "specs/changes/HG-0001/results.json");
    assert_eq!(results["acceptance"].as_array().unwrap().len(), 1);
    assert_eq!(results["history"].as_array().unwrap().len(), 4);
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0002-S2
#[test]
fn two_catalog_writers_cannot_allocate_the_same_number() {
    let root = root();
    create(&root);
    let expected = sha(&root);
    let spawn = || {
        std::process::Command::new(env!("CARGO_BIN_EXE_highgrade"))
            .args([
                "spec-new",
                "--root",
                root.to_str().unwrap(),
                "--title",
                "Конкурентная запись",
                "--expected",
                &expected,
            ])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let a = spawn();
    let b = spawn();
    let a = a.wait_with_output().unwrap();
    let b = b.wait_with_output().unwrap();
    assert_ne!(a.status.success(), b.status.success());
    assert_eq!(specs::load(&root).unwrap().0.changes.len(), 2);
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0002-S1
#[test]
fn selected_directory_is_project_owned_and_self_contained() {
    let root = root();
    call(
        &root,
        "spec-init",
        &[("--directory", "docs/specifications")],
    )
    .unwrap();
    create(&root);
    assert_eq!(
        specs::catalog_path(&root).unwrap(),
        "docs/specifications/catalog.json"
    );
    assert!(
        root.join("docs/specifications/changes/HG-0001/spec.json")
            .exists()
    );
    assert!(!root.join("specs").exists());
    fs::remove_dir_all(root.join(".highgrade")).unwrap();
    assert_eq!(specs::load(&root).unwrap().0.changes.len(), 1);
    let original = fs::read(root.join("specs-location.json")).unwrap();
    put(
        &root,
        "specs-location.json",
        &json!({"directory":"../outside"}),
    );
    assert!(specs::load(&root).unwrap_err().contains("UnsafePath"));
    fs::write(root.join("specs-location.json"), original).unwrap();
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0002-S2
#[test]
fn recovery_cannot_delete_a_required_specification() {
    let root = root();
    create(&root);
    let p = "specs/changes/HG-0001/spec.json";
    let original = fs::read(root.join(p)).unwrap();
    put(
        &root,
        "specs/transaction.json",
        &json!({p:{"before":highgrade::hash(&original),"after":null}}),
    );
    let h = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    assert!(
        call(&root, "spec-recover", &[("--expected", &h)])
            .unwrap_err()
            .contains("InvalidCatalogDeletion")
    );
    assert_eq!(fs::read(root.join(p)).unwrap(), original);
    assert!(root.join("specs/transaction.json").exists());
    fs::remove_dir_all(root).unwrap();
}

fn edit(root: &Path, id: &str, field: &str, value: Value) -> highgrade::Result<highgrade::Report> {
    let mut c = serde_json::to_value(&specs::load(root)?.0.changes[id]).unwrap();
    c[field] = value;
    put(root, "edit.json", &c);
    call(
        root,
        "spec-save",
        &[
            ("--id", id),
            ("--expected", &sha(root)),
            ("--input", "edit.json"),
        ],
    )
}
fn tag(root: &Path, id: &str) {
    call(
        root,
        "spec-tag-set",
        &[
            ("--id", id),
            ("--title", "Раздел"),
            ("--description", "Область проекта"),
            ("--expected", &sha(root)),
        ],
    )
    .unwrap();
}
// highgrade: HG-0003-S1
#[test]
fn explicit_links_reject_missing_cycles_and_do_not_inherit_content() {
    let root = root();
    create(&root);
    create(&root);
    let before = sha(&root);
    assert!(
        edit(
            &root,
            "HG-0001",
            "depends_on",
            json!([{"id":"HG-9999","reason":"Нужен результат"}])
        )
        .unwrap_err()
        .contains("InvalidLink")
    );
    assert_eq!(sha(&root), before);
    edit(
        &root,
        "HG-0002",
        "depends_on",
        json!([{"id":"HG-0001","reason":"Нужен результат"}]),
    )
    .unwrap();
    let before = sha(&root);
    assert!(
        edit(
            &root,
            "HG-0001",
            "depends_on",
            json!([{"id":"HG-0002","reason":"Цикл"}])
        )
        .unwrap_err()
        .contains("DependencyCycle")
    );
    assert_eq!(sha(&root), before);
    edit(
        &root,
        "HG-0001",
        "related_to",
        json!([{"id":"HG-0002","reason":"Общая тема"}]),
    )
    .unwrap();
    let checked = call(&root, "spec-check", &[("--id", "HG-0002")]).unwrap();
    assert!(
        serde_json::to_string(&checked)
            .unwrap()
            .contains("DependencyNotIntegrated")
    );
    let checked = call(&root, "spec-check", &[("--id", "HG-0001")]).unwrap();
    assert!(
        !serde_json::to_string(&checked)
            .unwrap()
            .contains("DependencyNotIntegrated")
    );
    let c = &specs::load(&root).unwrap().0.changes["HG-0002"];
    assert_eq!(c.goal, "");
    assert_eq!(c.operations.len(), 1);
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0003-S2, HG-0003-S3
#[test]
fn tags_merge_atomically_and_filters_keep_dependencies_discoverable() {
    let root = root();
    create(&root);
    create(&root);
    create(&root);
    tag(&root, "api");
    tag(&root, "backend");
    tag(&root, "ui");
    assert!(
        edit(&root, "HG-0001", "tags", json!(["unknown"]))
            .unwrap_err()
            .contains("UnknownTag")
    );
    edit(&root, "HG-0001", "tags", json!(["api", "backend"])).unwrap();
    edit(&root, "HG-0002", "tags", json!(["backend"])).unwrap();
    edit(&root, "HG-0003", "tags", json!(["ui"])).unwrap();
    edit(
        &root,
        "HG-0001",
        "depends_on",
        json!([{"id":"HG-0003","reason":"Интерфейс"}]),
    )
    .unwrap();
    assert!(
        call(
            &root,
            "spec-tag-remove",
            &[("--id", "backend"), ("--expected", &sha(&root))]
        )
        .unwrap_err()
        .contains("TagInUse")
    );
    let unrelated = fs::read(root.join("specs/changes/HG-0003/spec.json")).unwrap();
    call(
        &root,
        "spec-tag-merge",
        &[
            ("--from", "backend"),
            ("--into", "api"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0003/spec.json")).unwrap(),
        unrelated
    );
    let store = specs::load(&root).unwrap().0;
    assert!(!store.tags.contains_key("backend"));
    assert_eq!(store.changes["HG-0001"].tags.len(), 1);
    let list = call(
        &root,
        "spec-list",
        &[("--tag", "api"), ("--human", "pending")],
    )
    .unwrap();
    assert_eq!(list.measurements[0]["selection"]["selected"], 2);
    assert_eq!(list.measurements[0]["selection"]["total"], 3);
    let ids = list.measurements[0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["HG-0001", "HG-0002"]);
    assert_eq!(
        list.measurements[0]["changes"][0]["links"]["depends_on"][0]["id"],
        "HG-0003"
    );
    assert!(
        list.measurements[0]["changes"][0]
            .get("operations")
            .is_none()
    );
    let before_invalid_filter = sha(&root);
    assert!(
        call(&root, "spec-list", &[("--tag", "unknown")])
            .unwrap_err()
            .contains("UnknownTag")
    );
    assert!(
        call(&root, "spec-list", &[("--human", "unknown")])
            .unwrap_err()
            .contains("InvalidFilter: --human")
    );
    assert_eq!(sha(&root), before_invalid_filter);
    fs::remove_dir_all(root).unwrap();
}

fn runner_fixture(root: &Path) {
    create(root);
    fs::write(root.join("test_math.py"),"import unittest\nclass Cases(unittest.TestCase):\n    def test_sum(self): self.assertEqual(2+3,5)\n").unwrap();
    // A real unittest run emits the existing JUnit interchange format. The
    // product neither supplies a test language nor infers success from exit 0.
    fs::write(root.join("runner.py"),r#"import importlib.util, sys, unittest, xml.etree.ElementTree as ET, pathlib, time
source, selector, report = sys.argv[1:]
mode = pathlib.Path('mode.txt').read_text()
if mode == 'timeout': time.sleep(60)
if mode == 'slow-start': time.sleep(2)
spec=importlib.util.spec_from_file_location('test_math',source)
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
suite=unittest.defaultTestLoader.loadTestsFromName(selector.replace('::','.'),module)
result=unittest.TestResult(); suite.run(result)
root=ET.Element('testsuite')
if mode != 'empty':
    case=ET.SubElement(root,'testcase',classname=selector.split('::')[0],name=selector.split('::')[1])
    if result.failures or result.errors: ET.SubElement(case,'failure')
    if result.skipped: ET.SubElement(case,'skipped')
ET.ElementTree(root).write(report)
if mode == 'drift': pathlib.Path(source).write_text(pathlib.Path(source).read_text()+'\n# changed during run\n')
sys.exit(0 if result.wasSuccessful() else 1)
"#).unwrap();
    fs::write(root.join("mode.txt"), "normal").unwrap();
    put(
        root,
        "runner.json",
        &json!({"program":"python","args":["runner.py","{file}","{selector}","{report}"],"cwd":".","format":"junit","timeout_seconds":30}),
    );
    call(
        root,
        "spec-runner-set",
        &[
            ("--id", "unit"),
            ("--input", "runner.json"),
            ("--expected", &sha(root)),
        ],
    )
    .unwrap();
    edit(root,"HG-0001","checks",json!([{"id":"sum","scenario_ids":["HG-0001-S1"],"runner":"unit","file":"test_math.py","selector":"Cases::test_sum","preparation":"Создать 2 и 3","action":"Сложить","observation":"Результат 5","inputs":["test_math.py","runner.py","mode.txt"]}])).unwrap();
}

// highgrade: HG-0034-S1, HG-0034-S2
#[test]
fn native_report_import_reuses_suite_and_rejects_stale_or_missing_results() {
    let root = root();
    runner_fixture(&root);
    let store = specs::load(&root).unwrap().0;
    let mut checks = serde_json::to_value(&store.changes["HG-0001"].checks).unwrap();
    let mut second = checks[0].clone();
    second["id"] = json!("other");
    second["selector"] = json!("Cases::test_other");
    checks.as_array_mut().unwrap().push(second);
    edit(&root, "HG-0001", "checks", checks).unwrap();
    let captured = call(
        &root,
        "spec-run-inputs",
        &[("--id", "HG-0001"), ("--check", "all")],
    )
    .unwrap();
    let snapshot = captured
        .measurements
        .iter()
        .find_map(|m| m.get("snapshot"))
        .unwrap();
    fs::write(root.join("suite.xml"),"<testsuite><testcase classname='Cases' name='test_sum'/><testcase classname='Cases' name='test_other'/></testsuite>").unwrap();
    put(
        &root,
        "import.json",
        &json!({"snapshot":snapshot,"report":"suite.xml","command":["native-test"]}),
    );
    let import = || {
        call(
            &root,
            "spec-run-import",
            &[("--expected", &sha(&root)), ("--input", "import.json")],
        )
    };
    import().unwrap();
    let first = sha(&root);
    import().unwrap();
    assert_eq!(first, sha(&root));
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0001"].runs.len(),
        2
    );
    for body in ["<skipped/>", "<failure/>", "<error/>"] {
        fs::write(root.join("suite.xml"),format!("<testsuite><testcase classname='Cases' name='test_sum'>{body}</testcase></testsuite>")).unwrap();
        assert!(import().unwrap_err().contains("ImportedCheckNotPassed"));
        assert_eq!(first, sha(&root));
    }
    fs::write(root.join("suite.xml"), "<testsuite/>").unwrap();
    assert!(import().unwrap_err().contains("ImportedCheckNotPassed"));
    fs::write(root.join("mode.txt"), "changed").unwrap();
    assert!(import().unwrap_err().contains("ImportedInputsStale"));
    assert_eq!(first, sha(&root));
}

// highgrade: HG-0034-S3
#[test]
fn evidence_batch_is_atomic_and_exact_repeat_is_noop() {
    let root = root();
    create(&root);
    fs::write(root.join("logic.txt"), "logic").unwrap();
    fs::write(root.join("report.txt"), "report").unwrap();
    let evidence = json!({"command":"observe","captured_at":"2026-09-26","method":"manual","scenario":"HG-0001-S1","outcome":"passed","observation":"observed","inputs":["logic.txt"],"report":"report.txt"});
    put(
        &root,
        "batch.json",
        &json!([{"id":"HG-0001","evidence":evidence}]),
    );
    let batch = || {
        call(
            &root,
            "spec-evidence-batch",
            &[("--expected", &sha(&root)), ("--input", "batch.json")],
        )
    };
    batch().unwrap();
    let first = sha(&root);
    batch().unwrap();
    assert_eq!(first, sha(&root));
    let mut changed = evidence.clone();
    changed["observation"] = json!("changed");
    put(
        &root,
        "batch.json",
        &json!([{"id":"HG-0001","evidence":changed},{"id":"missing","evidence":evidence}]),
    );
    assert!(batch().is_err());
    assert_eq!(first, sha(&root));
}

// highgrade: HG-0037-S1, HG-0037-S2
#[test]
fn selected_list_and_local_edit_preserve_independent_work() {
    let root = root();
    create(&root);
    create(&root);
    let token = |id: &str| {
        call(&root, "spec-read", &[("--id", id), ("--view", "summary")])
            .unwrap()
            .measurements
            .into_iter()
            .find_map(|m| {
                m.get("local_sha256")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap()
    };
    let first = token("HG-0001");
    edit(&root, "HG-0002", "title", json!("Independent")).unwrap();
    put(&root, "patch.json", &json!({"title":"Selected"}));
    call(
        &root,
        "spec-edit",
        &[
            ("--id", "HG-0001"),
            ("--expected-local", &first),
            ("--input", "patch.json"),
        ],
    )
    .unwrap();
    assert!(
        call(
            &root,
            "spec-edit",
            &[
                ("--id", "HG-0001"),
                ("--expected-local", &first),
                ("--input", "patch.json")
            ]
        )
        .unwrap_err()
        .contains("LocalConflict")
    );
    let report = call(&root, "spec-list", &[("--id", "HG-0001")]).unwrap();
    let selected = report
        .measurements
        .iter()
        .find(|m| m.get("selection").is_some())
        .unwrap();
    assert_eq!(selected["changes"].as_array().unwrap().len(), 1);
    assert_eq!(selected["summary"]["total"], 1);
    assert_eq!(selected["selection"]["total"], 2);
    assert!(
        !report
            .measurements
            .iter()
            .any(|m| m.get("change").is_some())
    );
}

// highgrade: HG-0037-S2
#[test]
fn local_edit_rejects_dependency_runner_drift() {
    let root = root();
    runner_fixture(&root);
    create(&root);
    edit(
        &root,
        "HG-0002",
        "depends_on",
        json!([{"id":"HG-0001","reason":"Uses checked contract"}]),
    )
    .unwrap();
    let r = call(
        &root,
        "spec-read",
        &[("--id", "HG-0002"), ("--view", "summary")],
    )
    .unwrap();
    let token = r
        .measurements
        .iter()
        .find_map(|m| m.get("local_sha256").and_then(Value::as_str))
        .unwrap();
    let mut runner = json(&root, "runner.json");
    runner["timeout_seconds"] = json!(31);
    put(&root, "runner.json", &runner);
    call(
        &root,
        "spec-runner-set",
        &[
            ("--id", "unit"),
            ("--expected", &sha(&root)),
            ("--input", "runner.json"),
        ],
    )
    .unwrap();
    put(&root, "patch.json", &json!({"title":"stale"}));
    assert!(
        call(
            &root,
            "spec-edit",
            &[
                ("--id", "HG-0002"),
                ("--expected-local", token),
                ("--input", "patch.json")
            ]
        )
        .unwrap_err()
        .contains("LocalConflict")
    );
}

// highgrade: HG-0036-S3
#[test]
fn action_diagnostics_check_runner_without_execution() {
    let root = root();
    runner_fixture(&root);
    let before = sha(&root);
    assert_eq!(
        specs::diagnose(&root, "spec-read", None, None)
            .unwrap()
            .status,
        "passed"
    );
    let r = specs::diagnose(&root, "spec-run", Some("HG-0001"), Some("all")).unwrap();
    assert_eq!(r.status, "passed");
    assert_eq!(before, sha(&root));
    let mut runner = json(&root, "runner.json");
    runner["program"] = json!("missing-highgrade-test-tool");
    put(&root, "runner.json", &runner);
    call(
        &root,
        "spec-runner-set",
        &[
            ("--id", "unit"),
            ("--expected", &sha(&root)),
            ("--input", "runner.json"),
        ],
    )
    .unwrap();
    let r = specs::diagnose(&root, "spec-run", Some("HG-0001"), Some("all")).unwrap();
    assert_eq!(r.status, "failed");
    assert!(r.findings.iter().any(|f| f["code"] == "RunnerUnavailable"));
}

// highgrade: HG-0029-S1
#[test]
fn equivalent_native_runs_keep_material_revision_and_append_history() {
    let root = root();
    runner_fixture(&root);
    let run = || {
        call(
            &root,
            "spec-run",
            &[
                ("--id", "HG-0001"),
                ("--check", "all"),
                ("--expected", &sha(&root)),
            ],
        )
        .unwrap()
    };
    let revision = || {
        let read = call(&root, "spec-read", &[("--id", "HG-0001")]).unwrap();
        read.measurements
            .iter()
            .find_map(|m| m.get("change_sha256").cloned())
            .unwrap()
    };
    assert_eq!(run().status, "passed");
    let first = revision();
    assert_eq!(run().status, "passed");
    assert_eq!(revision(), first);
    let results = json(&root, "specs/changes/HG-0001/results.json");
    let runs = results["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_ne!(runs[0]["report"], runs[1]["report"]);
    assert!(!results["history"].as_array().unwrap().is_empty());
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0030-S1
#[test]
fn compact_views_and_atomic_edit_protect_results_and_cas() {
    let root = root();
    create(&root);
    let initial = sha(&root);
    for view in ["summary", "editable", "requirements"] {
        let result = call(&root, "spec-read", &[("--id", "HG-0001"), ("--view", view)]).unwrap();
        let text = serde_json::to_string(&result).unwrap();
        assert!(!text.contains("\"history\""));
        assert!(!text.contains("\"baseline\""));
    }
    assert_eq!(sha(&root), initial);
    put(&root, "patch.json", &json!({"title":"Updated"}));
    call(
        &root,
        "spec-edit",
        &[
            ("--id", "HG-0001"),
            ("--input", "patch.json"),
            ("--expected", &initial),
        ],
    )
    .unwrap();
    assert!(
        call(
            &root,
            "spec-edit",
            &[
                ("--id", "HG-0001"),
                ("--input", "patch.json"),
                ("--expected", &initial)
            ]
        )
        .unwrap_err()
        .contains("StoreConflict")
    );
    let current = sha(&root);
    let invalid = call(
        &root,
        "spec-edit",
        &[
            ("--id", "HG-0001"),
            ("--input", "patch.json"),
            ("--expected", &current),
            ("--validate", "true"),
        ],
    )
    .unwrap();
    assert_eq!(invalid.status, "failed");
    assert_eq!(sha(&root), current);
    put(&root, "patch.json", &json!({"acceptance":[]}));
    assert!(
        call(
            &root,
            "spec-edit",
            &[
                ("--id", "HG-0001"),
                ("--input", "patch.json"),
                ("--expected", &current)
            ]
        )
        .unwrap_err()
        .contains("ProtectedField")
    );
    assert_eq!(sha(&root), current);
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0030-S2
#[test]
fn shared_inputs_track_content_and_membership() {
    let root = root();
    runner_fixture(&root);
    fs::write(root.join("shared.txt"), "configuration").unwrap();
    edit(&root, "HG-0001", "shared_inputs", json!(["shared.txt"])).unwrap();
    call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "all"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    let issues = |root: &Path| {
        brief_data(&brief(root))["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["kind"] == "check")
    };
    assert!(!issues(&root));
    fs::write(root.join("shared.txt"), "changed").unwrap();
    assert!(issues(&root));
    fs::write(root.join("shared.txt"), "configuration").unwrap();
    assert!(!issues(&root));
    edit(&root, "HG-0001", "shared_inputs", json!([])).unwrap();
    assert!(issues(&root));
    fs::remove_dir_all(root).unwrap();
}
// highgrade: HG-0004-S2
#[test]
fn explicit_runner_records_real_pass_fail_skip_empty_timeout_and_drift() {
    let root = root();
    runner_fixture(&root);
    let before = sha(&root);
    call(&root, "spec-read", &[("--id", "HG-0001")]).unwrap();
    call(&root, "spec-list", &[]).unwrap();
    assert_eq!(sha(&root), before);
    assert!(!root.join("specs/changes/HG-0001/evidence").exists());
    assert!(
        call(
            &root,
            "spec-run",
            &[
                ("--id", "HG-0001"),
                ("--check", "missing"),
                ("--expected", &sha(&root))
            ]
        )
        .unwrap_err()
        .contains("EmptyCheckSelection")
    );
    let good = fs::read(root.join("test_math.py")).unwrap();
    for (mode,code,expected) in [
        ("normal",String::from_utf8(good.clone()).unwrap(),"passed"),
        ("slow-start",String::from_utf8(good.clone()).unwrap(),"passed"),
        ("normal","import unittest\nclass Cases(unittest.TestCase):\n    def test_sum(self): self.fail('real assertion failure')\n".into(),"failed"),
        ("normal","import unittest\nclass Cases(unittest.TestCase):\n    @unittest.skip('explicit skip')\n    def test_sum(self): pass\n".into(),"skipped"),
        ("empty",String::from_utf8(good.clone()).unwrap(),"unknown"),
        ("timeout",String::from_utf8(good.clone()).unwrap(),"failed"),
        ("drift",String::from_utf8(good.clone()).unwrap(),"unknown"),
    ] {
        let mut runner = json(&root, "runner.json");
        runner["timeout_seconds"] = json!(if mode == "timeout" { 1 } else { 30 });
        put(&root, "runner.json", &runner);
        call(&root, "spec-runner-set", &[("--id", "unit"), ("--input", "runner.json"), ("--expected", &sha(&root))]).unwrap();
        fs::write(root.join("mode.txt"),mode).unwrap(); fs::write(root.join("test_math.py"),code).unwrap();
        let report=call(&root,"spec-run",&[("--id","HG-0001"),("--check","all"),("--expected",&sha(&root))]).unwrap();
        assert_eq!(report.status=="passed",expected=="passed","{mode}: {report:?}");
        let state=specs::load(&root).unwrap().0;
        let run=state.changes["HG-0001"].runs.last().unwrap();
        assert_eq!(serde_json::to_value(&run.outcome).unwrap(),expected,"{mode}");
        assert!(root.join(&run.report).exists());
        if mode == "timeout" { assert!(run.observation.contains("RunnerTimeout")); }
    }
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0001"].runs.len(),
        7
    );
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0023-S1, HG-0023-S2
#[test]
fn run_statistics_track_attempts_without_execution() {
    let root = root();
    runner_fixture(&root);
    let empty = call(&root, "spec-stats", &[("--id", "HG-0001")]).unwrap();
    assert_eq!(empty.measurements[0]["checks"][0]["attempts"], 0);
    assert!(empty.measurements[0]["checks"][0]["latest_duration_ms"].is_null());

    let run = |root: &Path| {
        call(
            root,
            "spec-run",
            &[
                ("--id", "HG-0001"),
                ("--check", "sum"),
                ("--expected", &sha(root)),
            ],
        )
        .unwrap()
    };
    assert_eq!(run(&root).status, "passed");
    let results_path = root.join("specs/changes/HG-0001/results.json");
    let mut legacy: Value = serde_json::from_slice(&fs::read(&results_path).unwrap()).unwrap();
    legacy["runs"][0]
        .as_object_mut()
        .unwrap()
        .remove("duration_ms");
    fs::write(&results_path, serde_json::to_vec_pretty(&legacy).unwrap()).unwrap();

    fs::write(root.join("test_math.py"), "import unittest\nclass Cases(unittest.TestCase):\n    def test_sum(self): self.fail('failed assertion')\n").unwrap();
    assert_eq!(run(&root).status, "failed");
    let changed_inputs = call(&root, "spec-stats", &[("--id", "HG-0001")]).unwrap();
    let row = &changed_inputs.measurements[0]["checks"][0];
    assert_eq!(row["current_revision_attempts"], 2);
    assert_eq!(row["comparable_attempts"], 1);
    assert_eq!(row["comparable_measured_attempts"], 1);
    fs::write(root.join("mode.txt"), "timeout").unwrap();
    let mut runner = json(&root, "runner.json");
    runner["timeout_seconds"] = json!(1);
    put(&root, "runner.json", &runner);
    call(
        &root,
        "spec-runner-set",
        &[
            ("--id", "unit"),
            ("--input", "runner.json"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(run(&root).status, "failed");
    let before = fs::read(&results_path).unwrap();
    let stats = call(&root, "spec-stats", &[("--id", "HG-0001")]).unwrap();
    assert_eq!(stats.measurements.len(), 2);
    let row = &stats.measurements[0]["checks"][0];
    assert_eq!(row["attempts"], 3);
    assert_eq!(row["measured_attempts"], 2);
    assert_eq!(row["unmeasured_attempts"], 1);
    assert_eq!(row["current_revision_attempts"], 1);
    assert_eq!(row["comparable_attempts"], 1);
    assert_eq!(row["comparable_measured_attempts"], 1);
    assert_eq!(row["latest_current_revision"], true);
    assert_eq!(row["latest_comparable"], true);
    assert_eq!(row["latest_outcome"], "failed");
    assert!(row["latest_duration_ms"].as_u64().unwrap() >= 1000);
    assert!(row["median_duration_ms"].as_u64().unwrap() >= 1000);
    assert_eq!(fs::read(&results_path).unwrap(), before);
    assert_eq!(
        call(&root, "spec-stats", &[("--id", "HG-0001")])
            .unwrap()
            .measurements,
        stats.measurements
    );
    let mut runner = json(&root, "runner.json");
    runner["program"] = json!("highgrade-missing-runner-for-test");
    put(&root, "runner.json", &runner);
    call(
        &root,
        "spec-runner-set",
        &[
            ("--id", "unit"),
            ("--input", "runner.json"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(run(&root).status, "failed");
    let missing = call(&root, "spec-stats", &[("--id", "HG-0001")]).unwrap();
    let row = &missing.measurements[0]["checks"][0];
    assert_eq!(row["attempts"], 4);
    assert_eq!(row["measured_attempts"], 2);
    assert_eq!(row["unmeasured_attempts"], 2);
    assert_eq!(row["current_revision_attempts"], 1);
    assert_eq!(row["comparable_attempts"], 1);
    assert_eq!(row["comparable_measured_attempts"], 0);
    assert!(row["median_duration_ms"].is_null());
    assert_eq!(row["latest_current_revision"], true);
    assert_eq!(row["latest_outcome"], "unknown");
    assert!(row["latest_duration_ms"].is_null());
    assert!(
        specs::load(&root).unwrap().0.changes["HG-0001"]
            .runs
            .last()
            .unwrap()
            .observation
            .contains("RunnerStartFailed")
    );
    assert!(call(&root, "spec-stats", &[("--id", "missing")]).is_err());
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0004-S1
#[test]
fn multiple_checks_cannot_hide_failed_sibling_and_manual_is_explicit() {
    let root = root();
    runner_fixture(&root);
    let mut checks =
        serde_json::to_value(&specs::load(&root).unwrap().0.changes["HG-0001"].checks).unwrap();
    let mut second = checks[0].clone();
    second["id"] = json!("second");
    second["selector"] = json!("Cases::missing");
    checks.as_array_mut().unwrap().push(second);
    edit(&root, "HG-0001", "checks", checks).unwrap();
    let r = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "all"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(r.status, "failed");
    let r = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "sum"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(r.status, "passed");
    assert_ne!(
        serde_json::to_value(
            &specs::load(&root).unwrap().0.changes["HG-0001"].evidence["HG-0001-S1"].outcome
        )
        .unwrap(),
        "passed"
    );
    let mut checks =
        serde_json::to_value(&specs::load(&root).unwrap().0.changes["HG-0001"].checks).unwrap();
    checks[1]["runner"] = Value::Null;
    edit(&root, "HG-0001", "checks", checks).unwrap();
    let r = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "second"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(r.status, "unknown");
    fs::remove_dir_all(root).unwrap();
}

fn brief_fixture(root: &Path, two_scenarios: bool) {
    create(root);
    let mut c = serde_json::to_value(&specs::load(root).unwrap().0.changes["HG-0001"]).unwrap();
    c["goal"] = json!("Проверить точную актуальность");
    c["rationale"] = json!("Повторять только затронутое");
    c["scope"] = json!("Два входа");
    c["tasks"][0]["description"] = json!("Проверить сценарии");
    c["tasks"][0]["done"] = json!(true);
    let r = &mut c["operations"][0]["requirement"];
    r["title"] = json!("Актуальность");
    r["statement"] = json!("Сохранять действительные наблюдения");
    for field in ["given", "when", "then", "verification"] {
        r["scenarios"][0][field] = json!("Вход A даёт результат A");
    }
    if two_scenarios {
        r["scenarios"].as_array_mut().unwrap().push(json!({
            "id":"HG-0001-S2","given":"Вход B","when":"Проверка B",
            "then":"Результат B","verification":"Наблюдать B"
        }));
    }
    save_change(root, &c);
    fs::write(root.join("logic-a.txt"), "A").unwrap();
    fs::write(root.join("logic-b.txt"), "B").unwrap();
    fs::write(root.join("report.txt"), "A and B passed").unwrap();
    for (scenario, input) in [("HG-0001-S1", "logic-a.txt"), ("HG-0001-S2", "logic-b.txt")]
        .into_iter()
        .take(if two_scenarios { 2 } else { 1 })
    {
        put(
            root,
            "observation.json",
            &json!({
                "command":"fixture check","captured_at":"2026-09-26T00:00:00Z",
                "method":"native_report","scenario":scenario,"outcome":"passed",
                "observation":"Observed expected result","inputs":[input],"report":"report.txt"
            }),
        );
        call(
            root,
            "spec-evidence",
            &[
                ("--id", "HG-0001"),
                ("--expected", &sha(root)),
                ("--input", "observation.json"),
            ],
        )
        .unwrap();
    }
    review(root);
}
fn brief(root: &Path) -> highgrade::Report {
    call(
        root,
        "spec-check",
        &[("--id", "HG-0001"), ("--brief", "true")],
    )
    .unwrap()
}
fn brief_data(report: &highgrade::Report) -> &Value {
    report
        .measurements
        .iter()
        .find_map(|v| v.get("brief"))
        .unwrap()
}

// highgrade: HG-0022-S1, HG-0022-S2
#[test]
fn spec_check_brief_reports_affected_inputs() {
    let root = root();
    brief_fixture(&root, true);
    assert_eq!(brief(&root).status, "passed");
    let before = fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap();
    fs::write(root.join("logic-a.txt"), "changed").unwrap();
    let report = brief(&root);
    assert_eq!(report.status, "failed");
    let blockers = brief_data(&report)["blockers"].as_array().unwrap();
    assert!(blockers.iter().any(|b| b["kind"] == "evidence"
        && b["id"] == "HG-0001-S1"
        && b["reason"] == "input_changed"
        && b["path"] == "logic-a.txt"));
    assert!(!blockers.iter().any(|b| b["id"] == "HG-0001-S2"));
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap(),
        before
    );
    fs::write(root.join("logic-a.txt"), "A").unwrap();
    fs::write(root.join("status.txt"), "v1").unwrap();
    fs::write(root.join("status.txt"), "v2").unwrap();
    assert_eq!(brief(&root).status, "passed");
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0022-S3
#[test]
fn spec_check_brief_reports_stale_run() {
    let root = root();
    runner_fixture(&root);
    let run = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "sum"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(run.status, "passed");
    fs::write(root.join("test_math.py"), "changed after run").unwrap();
    let report = brief(&root);
    let blockers = brief_data(&report)["blockers"].as_array().unwrap();
    assert!(blockers.iter().any(|b| b["kind"] == "check"
        && b["id"] == "sum"
        && b["reason"] == "input_changed"
        && b["path"] == "test_math.py"));
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0022-S4
#[test]
fn spec_check_brief_returns_stable_handoff() {
    let root = root();
    brief_fixture(&root, false);
    let read = call(&root, "spec-read", &[("--id", "HG-0001")]).unwrap();
    let exact = read
        .measurements
        .iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap();
    put(
        &root,
        "decision.json",
        &json!({"decisions":[{
            "id":"HG-0001","decision":"accepted","decided_by":"simulated user",
            "change_sha256":exact["change_sha256"],"inputs_sha256":exact["inputs_sha256"],
            "verified_revision":"fixture-sha","comment":""
        }]}),
    );
    call(
        &root,
        "spec-decide",
        &[("--expected", &sha(&root)), ("--input", "decision.json")],
    )
    .unwrap();
    let before = fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap();
    let first = brief(&root);
    let second = brief(&root);
    assert_eq!(first.status, "passed");
    assert_eq!(brief_data(&first), brief_data(&second));
    assert_eq!(brief_data(&first)["technical"], "ready");
    assert_eq!(brief_data(&first)["human"], "accepted");
    assert_eq!(brief_data(&first)["review"], "go");
    assert_eq!(brief_data(&first)["verified_revision"], "fixture-sha");
    assert_eq!(brief_data(&first)["change_sha256"], exact["change_sha256"]);
    assert_eq!(brief_data(&first)["inputs_sha256"], exact["inputs_sha256"]);
    assert!(first.measurements.iter().all(|v| v.get("change").is_none()));
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap(),
        before
    );
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0004-S2
#[test]
fn batch_failure_keeps_prior_results_and_each_scenario_owns_its_report() {
    let root = root();
    runner_fixture(&root);
    let state = specs::load(&root).unwrap().0;
    let mut c = serde_json::to_value(&state.changes["HG-0001"]).unwrap();
    let mut scenario = c["operations"][0]["requirement"]["scenarios"][0].clone();
    scenario["id"] = json!("HG-0001-S2");
    c["operations"][0]["requirement"]["scenarios"]
        .as_array_mut()
        .unwrap()
        .push(scenario);
    let mut second = c["checks"][0].clone();
    second["id"] = json!("second");
    second["scenario_ids"] = json!(["HG-0001-S2"]);
    c["checks"].as_array_mut().unwrap().push(second);
    save_change(&root, &c);
    call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "all"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    let state = specs::load(&root).unwrap().0;
    let c = &state.changes["HG-0001"];
    assert_ne!(
        c.evidence["HG-0001-S1"].report,
        c.evidence["HG-0001-S2"].report
    );
    for e in c.evidence.values() {
        assert!(e.files.contains_key(&e.report));
    }
    let mut bindings = serde_json::to_value(&c.checks).unwrap();
    bindings[1]["inputs"] = json!(["missing.txt"]);
    edit(&root, "HG-0001", "checks", bindings).unwrap();
    let r = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "all"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(r.status, "failed");
    let state = specs::load(&root).unwrap().0;
    let c = &state.changes["HG-0001"];
    assert_eq!(c.runs.len(), 4);
    assert_eq!(serde_json::to_value(&c.runs[2].outcome).unwrap(), "passed");
    assert_eq!(serde_json::to_value(&c.runs[3].outcome).unwrap(), "unknown");
    assert_eq!(c.runs[2].command[0], "python");
    assert!(
        c.runs[2]
            .inputs_after
            .as_ref()
            .unwrap()
            .contains_key("test_math.py")
    );
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0004-S1, HG-0004-S2
#[test]
fn cargo_runner_requires_a_real_exact_test() {
    let root = root();
    create(&root);
    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("tests")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub fn sum(a:u32,b:u32)->u32 {a+b}\n",
    )
    .unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='catalog_pilot'\nversion='0.1.0'\nedition='2024'\n",
    )
    .unwrap();
    fs::write(
        root.join("tests/math.rs"),
        "#[test]\nfn sum() {assert_eq!(catalog_pilot::sum(2,3),5);}\n",
    )
    .unwrap();
    put(
        &root,
        "runner.json",
        &json!({"program":"cargo","args":["test","--offline","--test","{target}","{selector}","--","--exact"],"cwd":".","format":"cargo-test","timeout_seconds":30}),
    );
    call(
        &root,
        "spec-runner-set",
        &[
            ("--id", "cargo"),
            ("--input", "runner.json"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    edit(&root,"HG-0001","checks",json!([{"id":"sum","scenario_ids":["HG-0001-S1"],"runner":"cargo","file":"tests/math.rs","selector":"sum","preparation":"Два числа","action":"Сложение","observation":"5","inputs":["src/lib.rs","Cargo.toml"]}])).unwrap();
    let r = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "sum"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(r.status, "passed", "{r:?}");
    let mut bindings =
        serde_json::to_value(&specs::load(&root).unwrap().0.changes["HG-0001"].checks).unwrap();
    bindings[0]["selector"] = json!("missing");
    edit(&root, "HG-0001", "checks", bindings).unwrap();
    let r = call(
        &root,
        "spec-run",
        &[
            ("--id", "HG-0001"),
            ("--check", "sum"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    assert_eq!(r.status, "failed");
    fs::remove_dir_all(root).unwrap();
}

// highgrade: HG-0038-S1, HG-0038-S2
#[test]
fn metadata_batch_preserves_integrated_history_and_rejects_invalid_batches() {
    let root = root();
    brief_fixture(&root, false);
    let read = call(&root, "spec-read", &[("--id", "HG-0001")]).unwrap();
    let hashes = read
        .measurements
        .iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap();
    put(
        &root,
        "decision.json",
        &json!({"decisions":[{"id":"HG-0001","decision":"accepted","decided_by":"test fixture","change_sha256":hashes["change_sha256"],"inputs_sha256":hashes["inputs_sha256"],"verified_revision":"fixture","comment":"test only"}]}),
    );
    call(
        &root,
        "spec-decide",
        &[("--expected", &sha(&root)), ("--input", "decision.json")],
    )
    .unwrap();
    assert_eq!(
        call(
            &root,
            "spec-integrate",
            &[("--id", "HG-0001"), ("--expected", &sha(&root))]
        )
        .unwrap()
        .status,
        "passed"
    );
    call(
        &root,
        "spec-tag-set",
        &[
            ("--id", "specifications"),
            ("--title", "Specs"),
            ("--description", "Contract tools"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    let results = fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap();
    let requirement = fs::read(root.join("specs/requirements/HG-0001-R1.json")).unwrap();
    let original = sha(&root);
    let valid = json!([{"id":"HG-0001","tags":["specifications"]}]);
    put(&root, "metadata.json", &valid);
    let run = |expected: &str, apply: &str| {
        call(
            &root,
            "spec-metadata",
            &[
                ("--expected", expected),
                ("--input", "metadata.json"),
                ("--apply", apply),
            ],
        )
    };
    run(&original, "false").unwrap();
    assert_eq!(sha(&root), original);
    for invalid in [
        json!([{"id":"HG-0001","tags":["specifications"]},{"id":"missing","tags":[]}]),
        json!([{"id":"HG-0001","tags":["unknown"]}]),
        json!([{"id":"HG-0001","tags":[]},{"id":"HG-0001","tags":[]}]),
    ] {
        put(&root, "metadata.json", &invalid);
        assert!(run(&original, "true").is_err());
        assert_eq!(sha(&root), original);
    }
    put(&root, "metadata.json", &valid);
    assert!(run("stale", "true").is_err());
    run(&original, "true").unwrap();
    assert_eq!(
        call(&root, "spec-list", &[]).unwrap().measurements[0]["changes"][0]["human"],
        "accepted"
    );
    let after = sha(&root);
    run(&after, "true").unwrap();
    assert_eq!(sha(&root), after);
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap(),
        results
    );
    assert_eq!(
        fs::read(root.join("specs/requirements/HG-0001-R1.json")).unwrap(),
        requirement
    );
    fs::write(root.join("logic-a.txt"), "material drift").unwrap();
    put(&root, "metadata.json", &json!([{"id":"HG-0001","tags":[]}]));
    run(&after, "true").unwrap();
    assert_eq!(
        call(&root, "spec-list", &[]).unwrap().measurements[0]["changes"][0]["human"],
        "stale"
    );
    assert_eq!(
        fs::read(root.join("specs/changes/HG-0001/results.json")).unwrap(),
        results
    );
}

// highgrade: HG-0039-S1, HG-0039-S2
#[test]
fn scoped_baseline_captures_only_selected_requirements_and_never_refreshes() {
    let root = root();
    brief_fixture(&root, false);
    call(
        &root,
        "spec-integrate",
        &[("--id", "HG-0001"), ("--expected", &sha(&root))],
    )
    .unwrap();
    let original = json(&root, "specs/requirements/HG-0001-R1.json");
    create(&root);
    let c = &specs::load(&root).unwrap().0.changes["HG-0002"];
    assert!(c.baseline.is_empty());
    let operations = json!([{"action":"modify","requirement":original}]);
    edit(&root, "HG-0002", "operations", operations.clone()).unwrap();
    let captured = specs::load(&root).unwrap().0.changes["HG-0002"]
        .baseline
        .clone();
    assert_eq!(captured.len(), 1);
    let mut sibling = original.clone();
    sibling["id"] = json!("HG-OTHER-R1");
    sibling["scenarios"][0]["id"] = json!("HG-OTHER-S1");
    put(&root, "specs/requirements/HG-OTHER-R1.json", &sibling);
    assert!(
        !call(&root, "spec-validate", &[("--id", "HG-0002")])
            .unwrap()
            .findings
            .iter()
            .any(|f| f["code"] == "BaseDrift")
    );
    let mut drift = original.clone();
    drift["statement"] = json!("Changed by another accepted change");
    put(&root, "specs/requirements/HG-0001-R1.json", &drift);
    edit(&root, "HG-0002", "operations", json!([])).unwrap();
    edit(&root, "HG-0002", "operations", operations).unwrap();
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0002"].baseline,
        captured
    );
    assert!(
        call(&root, "spec-validate", &[("--id", "HG-0002")])
            .unwrap()
            .findings
            .iter()
            .any(|f| f["code"] == "BaseDrift")
    );
    create(&root);
    edit(
        &root,
        "HG-0003",
        "operations",
        json!([{"action":"remove","id":"HG-0001-R1","reason":"Retired"}]),
    )
    .unwrap();
    assert_ne!(
        specs::load(&root).unwrap().0.changes["HG-0003"].baseline,
        captured
    );
    // An existing broad baseline remains broad and is not silently modernized.
    let path = "specs/changes/HG-0003/spec.json";
    let mut old = json(&root, path);
    old.as_object_mut().unwrap().remove("scoped_baseline");
    old["baseline"]["HG-OTHER-R1"] = json!("historic-hash");
    put(&root, path, &old);
    edit(&root, "HG-0003", "rationale", json!("Editorial change")).unwrap();
    assert_eq!(json(&root, path)["baseline"], old["baseline"]);
}

// highgrade: HG-0040-S1
#[test]
fn readable_presentation_preserves_values_and_puts_service_fields_last() {
    let root = root();
    create(&root);
    let c = &specs::load(&root).unwrap().0.changes["HG-0001"];
    let value = serde_json::to_value(c).unwrap();
    let readable = specs::pretty_value(&value).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&readable).unwrap(), value);
    let text = fs::read_to_string(root.join("specs/changes/HG-0001/spec.json")).unwrap();
    assert!(text.find("\"title\"").unwrap() < text.find("\"baseline\"").unwrap());
    assert!(text.find("\"given\"").unwrap() < text.find("\"when\"").unwrap());
    assert!(text.find("\"when\"").unwrap() < text.find("\"then\"").unwrap());
    assert!(text.find("\"then\"").unwrap() < text.find("\"verification\"").unwrap());
}

// highgrade: HG-0039-S2
#[test]
fn first_baseline_capture_rejects_local_cas_after_target_changed() {
    let root = root();
    brief_fixture(&root, false);
    call(
        &root,
        "spec-integrate",
        &[("--id", "HG-0001"), ("--expected", &sha(&root))],
    )
    .unwrap();
    create(&root);
    let read = call(&root, "spec-read", &[("--id", "HG-0002")]).unwrap();
    let local = read
        .measurements
        .iter()
        .find_map(|v| v["local_sha256"].as_str())
        .unwrap();
    let whole = sha(&root);
    let old = json(&root, "specs/requirements/HG-0001-R1.json");
    let mut current = old.clone();
    current["statement"] = json!("Concurrent update");
    put(&root, "specs/requirements/HG-0001-R1.json", &current);
    put(
        &root,
        "patch.json",
        &json!({"operations":[{"action":"modify","requirement":old}]}),
    );
    let unchanged = sha(&root);
    let run = |key: &str, hash: &str| {
        call(
            &root,
            "spec-edit",
            &[("--id", "HG-0002"), (key, hash), ("--input", "patch.json")],
        )
    };
    assert!(
        run("--expected-local", local)
            .unwrap_err()
            .contains("BaselineCaptureRequiresStoreCAS")
    );
    assert!(
        run("--expected", &whole)
            .unwrap_err()
            .contains("StoreConflict")
    );
    assert_eq!(sha(&root), unchanged);
    assert!(
        specs::load(&root).unwrap().0.changes["HG-0002"]
            .baseline
            .is_empty()
    );
}
