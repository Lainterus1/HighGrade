mod support;
use highgrade::specs;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use support::TestDir;

const RESULTS: &str = "specs/changes/HG-0001/results.json";
const CATALOG: &str = "specs/catalog.json";
const TABLE: &str = "input_manifests";
const REFERENCE: &str = "input_manifest_sha256";

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
fn read(root: &Path, path: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
}
fn put(root: &Path, path: &str, value: &Value) {
    fs::write(root.join(path), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn migrate(root: &Path, format: &str) {
    call(
        root,
        "spec-migrate",
        &[("--to", format), ("--expected", &sha(root))],
    )
    .unwrap();
}
fn semantic_report(mut report: highgrade::Report) -> Value {
    report
        .measurements
        .retain(|value| value.get("store_sha256").is_none());
    serde_json::to_value(report).unwrap()
}
fn fingerprints(root: &Path) -> Value {
    call(root, "spec-read", &[("--id", "HG-0001")])
        .unwrap()
        .measurements
        .into_iter()
        .find(|v| v.get("change_sha256").is_some())
        .unwrap()
}
fn observe(root: &Path, scenario: usize, observation: &str) {
    let inputs: Vec<_> = (0..40)
        .map(|n| format!("input-{n:02}-{}.txt", "x".repeat(80)))
        .collect();
    put(
        root,
        "observation.json",
        &json!({"command":"manual fixture","captured_at":"2026-10-01","method":"manual","scenario":format!("HG-0001-S{scenario}"),"outcome":"passed","observation":observation,"inputs":inputs,"report":"report.txt"}),
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
fn fixture() -> TestDir {
    let root = TestDir::new("hg-shared-inputs-");
    call(
        &root,
        "spec-new",
        &[
            ("--title", "Shared input preservation"),
            ("--expected", &sha(&root)),
        ],
    )
    .unwrap();
    let mut change =
        serde_json::to_value(&specs::load(&root).unwrap().0.changes["HG-0001"]).unwrap();
    for field in ["goal", "rationale", "scope"] {
        change[field] = json!("Verify lossless evidence storage");
    }
    change["tasks"][0]["done"] = json!(true);
    change["tasks"][0]["description"] = json!("Verify storage");
    let requirement = &mut change["operations"][0]["requirement"];
    requirement["title"] = json!("Storage");
    requirement["statement"] = json!("Preserve all observations");
    requirement["scenarios"] = json!((1..=3).map(|n| json!({"id":format!("HG-0001-S{n}"),"given":"Stored evidence","when":"Migrating","then":"Preserved","verification":"Contract test"})).collect::<Vec<_>>());
    put(&root, "edit.json", &change);
    call(
        &root,
        "spec-save",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(&root)),
            ("--input", "edit.json"),
        ],
    )
    .unwrap();
    for n in 0..40 {
        fs::write(
            root.join(format!("input-{n:02}-{}.txt", "x".repeat(80))),
            format!("input {n}"),
        )
        .unwrap();
    }
    fs::write(root.join("report.txt"), "Observed fixture").unwrap();
    for n in 1..=3 {
        observe(&root, n, "First observation");
    }
    call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(&root)),
            ("--reviewer", "Fixture reviewer"),
            ("--verdict", "go"),
            ("--conclusion", "Full evidence reviewed"),
        ],
    )
    .unwrap();
    assert_eq!(
        call(&root, "spec-check", &[("--id", "HG-0001")])
            .unwrap()
            .status,
        "passed"
    );
    let hashes = fingerprints(&root);
    put(
        &root,
        "decision.json",
        &json!({"decisions":[{"id":"HG-0001","decision":"accepted","decided_by":"Fixture user","change_sha256":hashes["change_sha256"],"inputs_sha256":hashes["inputs_sha256"],"verified_revision":"fixture","comment":"Test acceptance"}]}),
    );
    call(
        &root,
        "spec-decide",
        &[("--expected", &sha(&root)), ("--input", "decision.json")],
    )
    .unwrap();
    // A repeat retains the reviewed evidence and archives a non-null review.
    observe(&root, 1, "First observation");
    assert!(
        specs::load(&root).unwrap().0.changes["HG-0001"]
            .history
            .iter()
            .any(|snapshot| snapshot.review.is_some())
    );
    root
}

// highgrade: HG-0064-S1
#[test]
fn shared_inputs_migrate_v3_and_v4_losslessly_and_idempotently() {
    for version in [3, 4] {
        let root = fixture();
        // Preserve legacy absence separately from an explicitly empty input set.
        let mut data = read(&root, RESULTS);
        data["history"][1]["evidence"]["HG-0001-S1"]
            .as_object_mut()
            .unwrap()
            .remove("input_paths");
        data["history"][2]["evidence"]["HG-0001-S1"]["input_paths"] = json!([]);
        data["history"][3]["evidence"]["HG-0001-S1"]["input_paths"] = Value::Null;
        put(&root, RESULTS, &data);
        if version == 4 {
            migrate(&root, "compact");
        }
        let before = specs::load(&root).unwrap().0;
        let hashes = fingerprints(&root);
        let old_size = fs::metadata(root.join(RESULTS)).unwrap().len();
        let old_objects = (version == 4).then(|| {
            read(&root, RESULTS)["history"]["objects"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<Vec<_>>()
        });
        migrate(&root, "shared-inputs");
        assert_eq!(read(&root, CATALOG)["schema_version"], 5);
        assert_eq!(before, specs::load(&root).unwrap().0);
        assert_eq!(hashes, fingerprints(&root));
        let shared = read(&root, RESULTS);
        assert_eq!(shared[TABLE].as_object().unwrap().len(), 3);
        assert!(fs::metadata(root.join(RESULTS)).unwrap().len() < old_size);
        if let Some(keys) = old_objects {
            assert_eq!(
                keys,
                shared["history"]["objects"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
            );
        }
        let bytes = fs::read(root.join(RESULTS)).unwrap();
        let before_sha = sha(&root);
        migrate(&root, "shared-inputs");
        assert_eq!(before_sha, sha(&root));
        assert_eq!(bytes, fs::read(root.join(RESULTS)).unwrap());
        let err = call(
            &root,
            "spec-migrate",
            &[("--to", "compact"), ("--expected", &sha(&root))],
        )
        .unwrap_err();
        assert!(err.contains("CatalogDowngradeUnsupported"), "{err}");
        assert_eq!(before_sha, sha(&root));
    }
}

// highgrade: HG-0064-S2
#[test]
fn shared_inputs_append_reuses_manifests_with_bounded_growth() {
    let root = fixture();
    migrate(&root, "shared-inputs");
    let initial = specs::load(&root).unwrap().0;
    let start_size = fs::metadata(root.join(RESULTS)).unwrap().len();
    let manifest_size = serde_json::to_vec(&read(&root, RESULTS)[TABLE])
        .unwrap()
        .len() as u64;
    for n in 0..12 {
        let prior = specs::load(&root).unwrap().0.changes["HG-0001"].clone();
        observe(&root, 1, &format!("Unique observation {n}"));
        let after = specs::load(&root).unwrap().0.changes["HG-0001"].clone();
        assert_eq!(&after.history[..prior.history.len()], prior.history);
        assert_eq!(after.history.last().unwrap().evidence, prior.evidence);
        assert_eq!(after.history.last().unwrap().review, prior.review);
        assert_eq!(after.acceptance, initial.changes["HG-0001"].acceptance);
        assert_eq!(read(&root, RESULTS)[TABLE].as_object().unwrap().len(), 1);
        assert_eq!(read(&root, CATALOG)["schema_version"], 5);
    }
    let growth = fs::metadata(root.join(RESULTS)).unwrap().len() - start_size;
    assert!(
        growth < 12 * manifest_size / 2,
        "growth {growth}, one input set {manifest_size}"
    );
    fs::write(
        root.join(format!("input-00-{}.txt", "x".repeat(80))),
        "changed",
    )
    .unwrap();
    observe(&root, 1, "Changed input");
    assert_eq!(read(&root, RESULTS)[TABLE].as_object().unwrap().len(), 2);
    call(
        &root,
        "spec-new",
        &[("--title", "Empty sibling"), ("--expected", &sha(&root))],
    )
    .unwrap();
    assert_eq!(read(&root, CATALOG)["schema_version"], 5);
    assert!(!root.join("specs/changes/HG-0002/results.json").exists());
}

// highgrade: HG-0064-S3
#[test]
fn shared_inputs_fail_closed_on_corrupt_missing_or_ambiguous_data() {
    let root = fixture();
    migrate(&root, "shared-inputs");
    let valid = read(&root, RESULTS);
    let key = valid[TABLE]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let history_key = valid["history"]["objects"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, v)| v.get(REFERENCE).is_some())
        .unwrap()
        .0
        .clone();
    let mut cases = Vec::new();
    let mut bad = valid.clone();
    bad[TABLE][&key]["files"]["report.txt"] = json!("corrupt");
    cases.push((bad, "InputManifestHashMismatch"));
    let mut bad = valid.clone();
    bad[TABLE].as_object_mut().unwrap().remove(&key);
    cases.push((bad, "InputManifestMissing"));
    let mut bad = valid.clone();
    bad.as_object_mut().unwrap().remove(TABLE);
    cases.push((bad, "InputManifestsMissing"));
    let mut bad = valid.clone();
    bad[TABLE] = json!([]);
    cases.push((bad, "InvalidInputManifests"));
    let mut bad = valid.clone();
    bad[TABLE][&key]["extra"] = json!(true);
    cases.push((bad, "InvalidInputManifest"));
    let mut bad = valid.clone();
    bad["evidence"]["HG-0001-S1"][REFERENCE] = json!("unknown");
    cases.push((bad, "InputManifestMissing"));
    let mut bad = valid.clone();
    bad["evidence"]["HG-0001-S1"][REFERENCE] = json!([]);
    cases.push((bad, "InvalidEvidenceInputReference"));
    let mut bad = valid.clone();
    bad["evidence"]["HG-0001-S1"]
        .as_object_mut()
        .unwrap()
        .remove(REFERENCE);
    cases.push((bad, "EvidenceInputReferenceMissing"));
    let mut bad = valid.clone();
    bad["evidence"]["HG-0001-S1"]["files"] = json!({});
    cases.push((bad, "InlineEvidenceInputsUnsupported"));
    let mut bad = valid.clone();
    bad["history"]["objects"][&history_key]["input_paths"] = json!([]);
    cases.push((bad, "InlineEvidenceInputsUnsupported"));
    let mut bad = valid.clone();
    bad["history"]["objects"][&history_key][REFERENCE] = json!("unknown");
    cases.push((bad, "InputManifestMissing"));
    let mut bad = valid.clone();
    bad[TABLE][&key].as_object_mut().unwrap().remove("files");
    cases.push((bad, "InvalidInputManifest"));
    let mut bad = valid.clone();
    bad["history"]["objects"][&history_key]["observation"] = json!("changed");
    cases.push((bad, "HistoryObjectHashMismatch"));
    let mut bad = valid.clone();
    bad[TABLE]["unused"] = json!({"files":{}});
    cases.push((bad, "InputManifestHashMismatch"));
    for (bad, error) in cases {
        put(&root, RESULTS, &bad);
        let before = fs::read(root.join(RESULTS)).unwrap();
        let actual = specs::load(&root).unwrap_err();
        assert!(actual.contains(error), "expected {error}, got {actual}");
        assert_eq!(before, fs::read(root.join(RESULTS)).unwrap());
        assert!(!root.join("specs/transaction.json").exists());
    }
    put(&root, RESULTS, &valid);
    assert!(specs::load(&root).is_ok());
}

// highgrade: HG-0033-S2, HG-0064-S3
#[test]
fn shared_inputs_migration_recovery_validates_complete_prospective_snapshot() {
    let root = fixture();
    migrate(&root, "compact");
    let before = specs::load(&root).unwrap().0;
    let old_catalog = fs::read(root.join(CATALOG)).unwrap();
    let old_results = fs::read(root.join(RESULTS)).unwrap();
    migrate(&root, "shared-inputs");
    let new_catalog = fs::read(root.join(CATALOG)).unwrap();
    let new_results = fs::read(root.join(RESULTS)).unwrap();
    let journal = json!({
        CATALOG:{"before":highgrade::hash(&old_catalog),"after":String::from_utf8(new_catalog.clone()).unwrap()},
        RESULTS:{"before":highgrade::hash(&old_results),"after":String::from_utf8(new_results.clone()).unwrap()}
    });
    // Both possible partial orders and fully applied recovery are supported.
    for (catalog, results) in [
        (&old_catalog, &old_results),
        (&new_catalog, &old_results),
        (&old_catalog, &new_results),
        (&new_catalog, &new_results),
    ] {
        fs::write(root.join(CATALOG), catalog).unwrap();
        fs::write(root.join(RESULTS), results).unwrap();
        put(&root, "specs/transaction.json", &journal);
        assert!(
            specs::load(&root)
                .unwrap_err()
                .contains("CatalogRecoveryRequired")
        );
        let expected = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
        call(&root, "spec-recover", &[("--expected", &expected)]).unwrap();
        assert_eq!(before, specs::load(&root).unwrap().0);
        assert_eq!(new_results, fs::read(root.join(RESULTS)).unwrap());
        assert_eq!(new_catalog, fs::read(root.join(CATALOG)).unwrap());
    }
    fs::write(root.join(CATALOG), &old_catalog).unwrap();
    fs::write(root.join(RESULTS), &old_results).unwrap();
    let mut corrupt = serde_json::from_slice::<Value>(&new_results).unwrap();
    corrupt[TABLE] = json!({});
    let mut bad_journal = journal;
    bad_journal[RESULTS]["after"] = json!(serde_json::to_string_pretty(&corrupt).unwrap());
    put(&root, "specs/transaction.json", &bad_journal);
    let expected = highgrade::hash(&fs::read(root.join("specs/transaction.json")).unwrap());
    let err = call(&root, "spec-recover", &[("--expected", &expected)]).unwrap_err();
    assert!(err.contains("InputManifestMissing"), "{err}");
    assert_eq!(old_results, fs::read(root.join(RESULTS)).unwrap());
    assert_eq!(old_catalog, fs::read(root.join(CATALOG)).unwrap());
    assert!(root.join("specs/transaction.json").exists());
}

// highgrade: HG-0064-S4
#[test]
fn shared_inputs_preserve_fingerprints_and_stale_detection() {
    let root = fixture();
    let before = call(&root, "spec-check", &[("--id", "HG-0001")]).unwrap();
    let hashes = fingerprints(&root);
    let path = root.join(format!("input-00-{}.txt", "x".repeat(80)));
    let original = fs::read(&path).unwrap();
    fs::write(&path, "changed").unwrap();
    let stale = call(&root, "spec-check", &[("--id", "HG-0001")]).unwrap();
    assert_eq!(stale.status, "failed");
    fs::write(&path, &original).unwrap();
    migrate(&root, "shared-inputs");
    assert_eq!(hashes, fingerprints(&root));
    assert_eq!(
        semantic_report(before),
        semantic_report(call(&root, "spec-check", &[("--id", "HG-0001")]).unwrap())
    );
    fs::write(&path, "changed").unwrap();
    assert_eq!(
        semantic_report(stale),
        semantic_report(call(&root, "spec-check", &[("--id", "HG-0001")]).unwrap())
    );
}

// highgrade: HG-0064-S1
#[test]
fn shared_inputs_schema_exposes_exact_versioned_storage_shape() {
    let root = fixture();
    let report = call(&root, "spec-schema", &[]).unwrap();
    let format = &report.measurements[0]["directory_format"];
    assert_eq!(format["versions"], json!([3, 4, 5]));
    assert_eq!(
        format["catalog"]["properties"]["schema_version"]["enum"],
        json!([3, 4, 5])
    );
    let schema = &format["shared_input_results"];
    assert_eq!(schema["additionalProperties"], false);
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!(TABLE))
    );
    let evidence = &schema["$defs"]["Evidence"];
    assert!(
        evidence["required"]
            .as_array()
            .unwrap()
            .contains(&json!(REFERENCE))
    );
    assert!(evidence["properties"].get("files").is_none());
    assert!(evidence["properties"].get("input_paths").is_none());
    assert_eq!(
        schema["$defs"]["InputManifest"]["additionalProperties"],
        false
    );
    assert!(schema["$defs"]["HistoryEntry"].is_object());
    assert!(
        schema["properties"]["history"]["properties"]["objects"]["additionalProperties"]["anyOf"]
            .is_array()
    );
}

// highgrade: HG-0033-S2
#[test]
fn shared_inputs_limits_fail_before_any_catalog_write() {
    let root = fixture();
    migrate(&root, "shared-inputs");
    let before = sha(&root);
    let results = fs::read(root.join(RESULTS)).unwrap();
    let catalog = fs::read(root.join(CATALOG)).unwrap();
    let oversized = "\\".repeat(3 * 1024 * 1024);
    let error = call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-0001"),
            ("--expected", &before),
            ("--reviewer", "Fixture reviewer"),
            ("--verdict", "go"),
            ("--conclusion", &oversized),
        ],
    )
    .unwrap_err();
    assert!(error.contains("TransactionTooLarge"), "{error}");
    assert_eq!(before, sha(&root));
    assert_eq!(results, fs::read(root.join(RESULTS)).unwrap());
    assert_eq!(catalog, fs::read(root.join(CATALOG)).unwrap());
    assert!(!root.join("specs/transaction.json").exists());
}

// highgrade: HG-0064-S1, HG-0064-S2
#[test]
fn shared_inputs_support_empty_catalog_and_review_only_results() {
    let root = TestDir::new("hg-shared-empty-");
    call(
        &root,
        "spec-new",
        &[("--title", "Empty"), ("--expected", &sha(&root))],
    )
    .unwrap();
    migrate(&root, "shared-inputs");
    assert!(!root.join(RESULTS).exists());
    call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(&root)),
            ("--reviewer", "Fixture reviewer"),
            ("--verdict", "no_go"),
            ("--conclusion", "Needs implementation"),
        ],
    )
    .unwrap();
    assert_eq!(read(&root, RESULTS)[TABLE], json!({}));
    let before = specs::load(&root).unwrap().0;
    migrate(&root, "shared-inputs");
    assert_eq!(before, specs::load(&root).unwrap().0);
}
