use highgrade::specs;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "hg-catalog-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&p).unwrap();
    p
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
    assert_eq!(
        list.measurements[0]["changes"][0]["links"]["depends_on"][0]["id"],
        "HG-0003"
    );
    assert!(
        list.measurements[0]["changes"][0]
            .get("operations")
            .is_none()
    );
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
if mode == 'timeout': time.sleep(4)
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
        &json!({"program":"python","args":["runner.py","{file}","{selector}","{report}"],"cwd":".","format":"junit","timeout_seconds":1}),
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
        ("normal","import unittest\nclass Cases(unittest.TestCase):\n    def test_sum(self): self.fail('real assertion failure')\n".into(),"failed"),
        ("normal","import unittest\nclass Cases(unittest.TestCase):\n    @unittest.skip('explicit skip')\n    def test_sum(self): pass\n".into(),"skipped"),
        ("empty",String::from_utf8(good.clone()).unwrap(),"unknown"),
        ("timeout",String::from_utf8(good.clone()).unwrap(),"failed"),
        ("drift",String::from_utf8(good.clone()).unwrap(),"unknown"),
    ] {
        fs::write(root.join("mode.txt"),mode).unwrap(); fs::write(root.join("test_math.py"),code).unwrap();
        let report=call(&root,"spec-run",&[("--id","HG-0001"),("--check","all"),("--expected",&sha(&root))]).unwrap();
        assert_eq!(report.status=="passed",expected=="passed","{mode}: {report:?}");
        let state=specs::load(&root).unwrap().0;
        let run=state.changes["HG-0001"].runs.last().unwrap();
        assert_eq!(serde_json::to_value(&run.outcome).unwrap(),expected,"{mode}");
        assert!(root.join(&run.report).exists());
    }
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0001"].runs.len(),
        6
    );
    fs::remove_dir_all(root).unwrap();
}

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
