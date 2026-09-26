mod support;
use highgrade::{hash, trace::trace};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use support::TestDir;
fn root() -> TestDir {
    TestDir::new("hg-trace-")
}
fn put(root: &Path, rel: &str, data: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}
fn save(root: &Path, rel: &str, value: &Value) {
    put(root, rel, &serde_json::to_vec_pretty(value).unwrap());
}
fn spec() -> &'static [u8] {
    b"## Purpose\nA verifiable addition.\n\n### Requirement: [HG-MATH-REQ] Add integers\nThe system SHALL add integers.\n\n#### Scenario: [HG-MATH-001] Adds two values\n- WHEN two values are added\n- THEN the sum is returned\n"
}
fn rust_list(root: &Path) -> Value {
    json!({"rust-suites":{"math":{"kind":"test","cwd":root.to_string_lossy(),"binary-name":"math","testcases":{"adds":{"filter-match":{"status":"matches"},"ignored":false}}}}})
}

fn spec_call(
    root: &Path,
    operation: &str,
    args: &[(&str, &str)],
) -> highgrade::Result<highgrade::Report> {
    highgrade::specs::command(
        root,
        operation,
        &args
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>(),
    )
}

// highgrade: HG-0008-S3
#[test]
fn reviewed_transfer_trace_selects_one_owner_and_preserves_neighbor() {
    use highgrade::specs::{Requirement, STORE, Scenario, Store};
    let root = root();
    let legacy_path = "openspec/specs/math/spec.md";
    let legacy = [spec(), b"\n### Requirement: [HG-OTHER-REQ] Unrelated rule\nThe other rule remains.\n\n#### Scenario: [HG-OTHER-001] Other case\n- WHEN the other feature runs\n- THEN its result remains available\n"].concat();
    let source = "tests/math.rs";
    let list = "evidence/list.json";
    let report = "evidence/junit.xml";
    put(
        &root,
        source,
        b"// highgrade: HG-MATH-001\n#[test]\nfn adds() { assert_eq!(1+1,2); }\n",
    );
    save(&root, list, &rust_list(&root));
    put(&root, report, br#"<testsuites><testsuite name="math"><testcase classname="math" name="adds"/></testsuite></testsuites>"#);
    let mut run = base(&root, "rust-nextest", source, report, Some(list));
    put(&root, legacy_path, &legacy);
    run["source_hashes"][legacy_path] = json!(hash(&legacy));
    save(
        &root,
        STORE,
        &serde_json::to_value(Store::default()).unwrap(),
    );
    let selected = Requirement {
        id: "HG-MATH-REQ".into(),
        title: "Add".into(),
        statement: "Return sum".into(),
        scenarios: vec![Scenario {
            id: "HG-MATH-001".into(),
            given: "1,1".into(),
            when: "Add".into(),
            then: "2".into(),
            verification: "math::adds".into(),
        }],
    };
    save(
        &root,
        "selected.json",
        &serde_json::to_value(selected).unwrap(),
    );
    let expected = highgrade::specs::load(&root).unwrap().1;
    spec_call(
        &root,
        "spec-import",
        &[
            ("--id", "HG-IMPORT"),
            ("--expected", &expected),
            ("--input", "selected.json"),
            ("--source", legacy_path),
        ],
    )
    .unwrap();
    let mut change =
        serde_json::to_value(&highgrade::specs::load(&root).unwrap().0.changes["HG-IMPORT"])
            .unwrap();
    change["tasks"][0]["description"] = json!("Compare selected requirement and source");
    change["tasks"][0]["done"] = json!(true);
    save(&root, "edit.json", &change);
    let expected = highgrade::specs::load(&root).unwrap().1;
    spec_call(
        &root,
        "spec-save",
        &[
            ("--id", "HG-IMPORT"),
            ("--expected", &expected),
            ("--input", "edit.json"),
        ],
    )
    .unwrap();
    let expected = highgrade::specs::load(&root).unwrap().1;
    spec_call(
        &root,
        "spec-review",
        &[
            ("--id", "HG-IMPORT"),
            ("--expected", &expected),
            ("--reviewer", "reviewer"),
            ("--verdict", "go"),
            ("--conclusion", "Equivalent selected contract"),
        ],
    )
    .unwrap();
    let expected = highgrade::specs::load(&root).unwrap().1;
    spec_call(
        &root,
        "spec-transfer",
        &[("--id", "HG-IMPORT"), ("--expected", &expected)],
    )
    .unwrap();
    assert_eq!(fs::read(root.join(legacy_path)).unwrap(), legacy);
    let store = highgrade::specs::load(&root).unwrap().0;
    assert!(store.requirements.contains_key("HG-MATH-REQ"));
    assert!(store.origins.contains_key("HG-MATH-REQ"));
    run["spec_files"].as_array_mut().unwrap().push(json!(STORE));
    run["source_hashes"][STORE] = json!(highgrade::specs::trace_hash(&root).unwrap());
    save(&root, "run.json", &run);
    let traced = trace(&root, "run.json").unwrap();
    let rows: Vec<_> = traced
        .measurements
        .iter()
        .filter_map(|row| row["scenario_id"].as_str())
        .collect();
    assert_eq!(
        rows.iter().filter(|id| **id == "HG-MATH-001").count(),
        1,
        "{traced:?}"
    );
    assert_eq!(
        rows.iter().filter(|id| **id == "HG-OTHER-001").count(),
        1,
        "{traced:?}"
    );
    assert!(
        traced
            .measurements
            .iter()
            .any(|row| row["scenario_id"] == "HG-MATH-001" && row["status"] == "passed"),
        "{traced:?}"
    );
    assert!(
        !traced
            .findings
            .iter()
            .any(|row| row["code"] == "DuplicateScenarioId" || row["status"] == "failed"),
        "{traced:?}"
    );
}

#[test]
fn native_owner_replaces_only_imported_markdown_and_ignores_evidence_metadata() {
    use highgrade::specs::{Origin, Requirement, STORE, Scenario, Store};
    let root = root();
    let src = "tests/math.rs";
    let list = "evidence/list.json";
    let report = "evidence/junit.xml";
    put(
        &root,
        src,
        b"// highgrade: HG-MATH-001\n#[test]\nfn adds() { assert_eq!(1+1,2); }\n",
    );
    save(&root, list, &rust_list(&root));
    put(&root, report, br#"<testsuites><testsuite name="math"><testcase classname="math" name="adds"/></testsuite></testsuites>"#);
    let mut run = base(&root, "rust-nextest", src, report, Some(list));
    let mut store = Store::default();
    store.requirements.insert(
        "HG-MATH-REQ".into(),
        Requirement {
            id: "HG-MATH-REQ".into(),
            title: "Add".into(),
            statement: "Return sum".into(),
            scenarios: vec![Scenario {
                id: "HG-MATH-001".into(),
                given: "1,1".into(),
                when: "Add".into(),
                then: "2".into(),
                verification: "math::adds".into(),
            }],
        },
    );
    store.origins.insert(
        "HG-MATH-REQ".into(),
        Origin {
            path: "openspec/specs/math/spec.md".into(),
            sha256: hash(spec()),
            original_text: String::from_utf8(spec().to_vec()).unwrap(),
        },
    );
    save(
        &root,
        "selected.json",
        &serde_json::to_value(&store.requirements["HG-MATH-REQ"]).unwrap(),
    );
    save(
        &root,
        STORE,
        &serde_json::to_value(Store::default()).unwrap(),
    );
    let expected = highgrade::specs::load(&root).unwrap().1;
    let args = [
        ("--id", "HG-IMPORT"),
        ("--expected", &expected),
        ("--input", "selected.json"),
        ("--source", "openspec/specs/math/spec.md"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    highgrade::specs::command(&root, "spec-import", &args).unwrap();
    run["spec_files"].as_array_mut().unwrap().push(json!(STORE));
    run["source_hashes"][STORE] = json!(highgrade::specs::trace_hash(&root).unwrap());
    save(&root, "run.json", &run);
    let r = trace(&root, "run.json").unwrap();
    assert_eq!(r.status, "passed", "{r:?}");
    // The completed owner has the same effective contract as the imported draft.
    save(&root, STORE, &serde_json::to_value(&store).unwrap());
    assert_eq!(trace(&root, "run.json").unwrap().status, "passed");
    // Historical provenance metadata does not change the effective contract fingerprint.
    store
        .origins
        .get_mut("HG-MATH-REQ")
        .unwrap()
        .original_text
        .push_str("\n");
    save(&root, STORE, &serde_json::to_value(&store).unwrap());
    assert_eq!(trace(&root, "run.json").unwrap().status, "passed");
    store.requirements.get_mut("HG-MATH-REQ").unwrap().statement = "Changed expectation".into();
    save(&root, STORE, &serde_json::to_value(&store).unwrap());
    assert_ne!(trace(&root, "run.json").unwrap().status, "passed");
}
fn base(root: &Path, tool: &str, source: &str, report: &str, inventory: Option<&str>) -> Value {
    let spec_path = "openspec/specs/math/spec.md";
    put(root, spec_path, spec());
    let mut hashes = serde_json::Map::new();
    for rel in [spec_path, source].into_iter().chain(inventory) {
        hashes.insert(rel.into(), json!(hash(&fs::read(root.join(rel)).unwrap())));
    }
    json!({"schema_version":1,"tool":tool,"command":"native test command","scope":"math","captured_at":"2026-09-23T00:00:00Z","capture_root":root.to_string_lossy(),"exit_code":0,"report":report,"report_sha256":hash(&fs::read(root.join(report)).unwrap()),"spec_files":[spec_path],"test_sources":[source],"source_hashes":hashes,"inventory":inventory})
}
#[test]
fn rust_requires_real_listed_passing_result() {
    let root = root();
    let src = "tests/math.rs";
    let list = "evidence/list.json";
    let report = "evidence/junit.xml";
    put(
        &root,
        src,
        b"// highgrade: HG-MATH-001\n#[test]\nfn adds() { assert_eq!(1+1,2); }\n",
    );
    save(&root, list, &rust_list(&root));
    put(&root,report,br#"<testsuites><testsuite name="math"><testcase classname="math" name="adds"/></testsuite></testsuites>"#);
    let run = base(&root, "rust-nextest", src, report, Some(list));
    save(&root, "run.json", &run);
    let result = trace(&root, "run.json").unwrap();
    assert_eq!(result.status, "passed", "{:?}", result.findings);
    let mut changed = run.clone();
    changed["report_sha256"] = json!("wrong");
    save(&root, "run.json", &changed);
    assert_eq!(trace(&root, "run.json").unwrap().status, "failed");
    save(&root, "run.json", &run);
    put(&root, src, b"// changed");
    let result = trace(&root, "run.json").unwrap();
    assert!(result.findings.iter().any(|f| f["code"] == "SourceChanged"));
    assert_ne!(result.status, "passed");
}
// highgrade: HG-0024-S1, HG-0024-S2, HG-0024-S3
#[test]
fn automatic_trace_keeps_scope_and_outcomes_distinct() {
    let root = root();
    let src = "tests/math.rs";
    let list = "evidence/list.json";
    let report = "evidence/junit.xml";
    put(
        &root,
        src,
        b"// highgrade: HG-MATH-001\n#[test]\nfn adds() {}\n#[test]\nfn helper() {}\n",
    );
    let mut inventory = rust_list(&root);
    inventory["rust-suites"]["math"]["testcases"]["helper"] =
        json!({"filter-match":{"status":"matches"},"ignored":false});
    save(&root, list, &inventory);
    let xml = |outcome: &str| {
        format!(
            "<testsuites><testsuite name=\"math\"><testcase classname=\"math\" name=\"adds\">{outcome}</testcase><testcase classname=\"math\" name=\"helper\"/></testsuite></testsuites>"
        )
    };
    put(&root, report, xml("").as_bytes());
    let manual = b"## Purpose\nAddition.\n### Requirement: [HG-MATH-REQ] Add\n#### Scenario: [HG-MATH-001] Automatic\n- WHEN called\n- THEN result\n#### Scenario: [HG-MATH-002] Manual\n- WHEN observed\n- THEN reviewed\n";
    let mut run = base(&root, "rust-nextest", src, report, Some(list));
    put(&root, "openspec/specs/math/spec.md", manual);
    run["source_hashes"]["openspec/specs/math/spec.md"] = json!(hash(manual));
    save(&root, "run.json", &run);
    let traced = trace(&root, "run.json").unwrap();
    assert_eq!(traced.status, "passed", "{traced:?}");
    let counts = traced
        .measurements
        .iter()
        .find(|m| m.get("test_cases_reported").is_some())
        .unwrap();
    assert_eq!(counts["test_cases_reported"], 2);
    assert_eq!(counts["test_cases_executed"], 2);
    assert_eq!(counts["linked_test_cases"], 1);
    assert_eq!(counts["automatic_passed"], 1);
    assert_eq!(counts["outside_automatic_trace"], 1);
    assert!(
        traced
            .measurements
            .iter()
            .any(|m| m["scenario_id"] == "HG-MATH-002" && m["status"] == "outside_automatic_trace")
    );
    assert!(
        !traced
            .findings
            .iter()
            .any(|f| f["code"] == "ScenarioUnconfirmed")
    );
    for (outcome, expected) in [("<failure/>", "failed"), ("<skipped/>", "skipped")] {
        put(&root, report, xml(outcome).as_bytes());
        run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
        save(&root, "run.json", &run);
        let traced = trace(&root, "run.json").unwrap();
        let counts = traced
            .measurements
            .iter()
            .find(|m| m.get("test_cases_reported").is_some())
            .unwrap();
        assert_eq!(
            counts["test_cases_executed"],
            if expected == "skipped" { 1 } else { 2 }
        );
        assert_eq!(
            counts["test_cases_skipped"],
            if expected == "skipped" { 1 } else { 0 }
        );
        assert!(
            traced
                .measurements
                .iter()
                .any(|m| m["scenario_id"] == "HG-MATH-001" && m["status"] == expected)
        );
        assert!(
            traced
                .findings
                .iter()
                .any(|f| f["code"] == "ScenarioUnconfirmed")
        );
    }
    put(&root, report, b"<testsuites><testsuite name=\"math\"><testcase classname=\"math\" name=\"helper\"/></testsuite></testsuites>");
    run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
    save(&root, "run.json", &run);
    let traced = trace(&root, "run.json").unwrap();
    assert!(
        traced
            .measurements
            .iter()
            .any(|m| m["scenario_id"] == "HG-MATH-001" && m["status"] == "unknown")
    );
    let counts = traced
        .measurements
        .iter()
        .find(|m| m.get("test_cases_reported").is_some())
        .unwrap();
    assert_eq!(counts["linked_test_cases"], 0);
    assert_eq!(counts["test_cases_missing"], 1);
    assert_eq!(counts["test_cases_unknown"], 0);
    assert_eq!(counts["automatic_scenarios"], 1);
    assert_eq!(counts["outside_automatic_trace"], 1);
    put(&root, report, xml("").as_bytes());
    run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
    let mut bad_inventory = inventory.clone();
    bad_inventory["rust-suites"]["math"]["binary-name"] = json!("other");
    save(&root, list, &bad_inventory);
    run["source_hashes"][list] = json!(hash(&fs::read(root.join(list)).unwrap()));
    save(&root, "run.json", &run);
    let traced = trace(&root, "run.json").unwrap();
    assert!(
        traced
            .measurements
            .iter()
            .any(|m| m["scenario_id"] == "HG-MATH-001" && m["status"] == "unknown")
    );
    save(&root, list, &inventory);
    run["source_hashes"][list] = json!(hash(&fs::read(root.join(list)).unwrap()));
    put(&root, report, xml("").as_bytes());
    run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
    run["source_hashes"][src] = json!("stale");
    save(&root, "run.json", &run);
    let traced = trace(&root, "run.json").unwrap();
    assert!(
        traced
            .measurements
            .iter()
            .any(|m| m["scenario_id"] == "HG-MATH-001" && m["status"] == "stale")
    );
}
#[test]
fn active_modified_requirement_replaces_base_for_trace() {
    let root = root();
    let src = "tests/math.rs";
    let list = "evidence/list.json";
    let report = "evidence/junit.xml";
    let delta = "openspec/changes/change-math/specs/math/spec.md";
    put(
        &root,
        src,
        b"// highgrade: HG-MATH-002\n#[test]\nfn adds() {}\n",
    );
    save(&root, list, &rust_list(&root));
    put(&root, report, br#"<testsuites><testsuite name="math"><testcase classname="math" name="adds"/></testsuite></testsuites>"#);
    put(&root, delta, b"## MODIFIED Requirements\n\n### Requirement: [HG-MATH-REQ] Add integers\nThe system SHALL add integers.\n\n#### Scenario: [HG-MATH-002] Adds two values\n- WHEN two values are added\n- THEN the sum is returned\n");
    let mut run = base(&root, "rust-nextest", src, report, Some(list));
    run["spec_files"].as_array_mut().unwrap().push(json!(delta));
    run["source_hashes"][delta] = json!(hash(&fs::read(root.join(delta)).unwrap()));
    save(&root, "run.json", &run);
    let result = trace(&root, "run.json").unwrap();
    assert_eq!(result.status, "passed", "{:?}", result.findings);
    assert!(
        result
            .measurements
            .iter()
            .any(|m| m["scenario_id"] == "HG-MATH-002" && m["status"] == "passed")
    );
    assert!(
        !result
            .measurements
            .iter()
            .any(|m| m["scenario_id"] == "HG-MATH-001")
    );
}
#[test]
fn rust_skip_failure_unknown_and_empty_are_unconfirmed() {
    for (element, expected) in [
        ("<skipped/>", "skipped"),
        ("<failure/>", "failed"),
        ("<flakyFailure/>", "failed"),
        ("", "passed"),
    ] {
        let root = root();
        let src = "tests/math.rs";
        let list = "list.json";
        let report = "junit.xml";
        put(
            &root,
            src,
            b"// highgrade: HG-MATH-001\n#[test]\nfn adds() {}\n",
        );
        save(&root, list, &rust_list(&root));
        put(&root,report,format!("<testsuites><testsuite><testcase classname=\"math\" name=\"adds\">{element}</testcase></testsuite></testsuites>").as_bytes());
        let run = base(&root, "rust-nextest", src, report, Some(list));
        save(&root, "run.json", &run);
        let result = trace(&root, "run.json").unwrap();
        assert_eq!(result.measurements[0]["status"], expected);
        if expected != "passed" {
            assert_ne!(result.status, "passed");
        }
    }
    let root = root();
    let src = "tests/math.rs";
    let list = "list.json";
    let report = "junit.xml";
    put(
        &root,
        src,
        b"// highgrade: HG-MATH-001\n#[test]\nfn adds() {}\n",
    );
    save(&root, list, &json!({"rust-suites":{}}));
    put(&root, report, b"<testsuites/>");
    let run = base(&root, "rust-nextest", src, report, Some(list));
    save(&root, "run.json", &run);
    assert_eq!(trace(&root, "run.json").unwrap().status, "failed");
}
#[test]
fn playwright_uses_annotation_and_result_not_just_link() {
    let root = root();
    let src = "tests/math.spec.ts";
    let report = "evidence/pw.json";
    put(&root,src,b"test('adds', { annotation: { type: 'highgrade', description: 'HG-MATH-001' } }, async () => {});");
    let item = |status: &str, results: Value| json!({"config":{"rootDir":root.to_string_lossy()},"suites":[{"specs":[{"id":"spec-1","file":"tests/math.spec.ts","tests":[{"projectName":"chromium","expectedStatus":"passed","status":status,"annotations":[{"type":"highgrade","description":"HG-MATH-001"}],"results":results}]}]}]});
    save(
        &root,
        report,
        &item("expected", json!([{"status":"passed"}])),
    );
    let mut run = base(&root, "playwright", src, report, None);
    save(&root, "run.json", &run);
    let traced = trace(&root, "run.json").unwrap();
    assert_eq!(traced.status, "passed");
    let counts = traced
        .measurements
        .iter()
        .find(|m| m.get("test_cases_reported").is_some())
        .unwrap();
    assert_eq!(counts["test_cases_reported"], 1);
    assert!(counts["test_cases_missing"].is_null());
    save(
        &root,
        report,
        &json!({"config":{"rootDir":root.to_string_lossy()},"suites":[{"specs":[{"id":"spec-1","file":"other.spec.ts","tests":[{"projectName":"chromium","expectedStatus":"passed","status":"expected","annotations":[{"type":"highgrade","description":"HG-MATH-001"}],"results":[{"status":"passed"}]}]}]}]}),
    );
    run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
    save(&root, "run.json", &run);
    assert!(
        trace(&root, "run.json")
            .unwrap()
            .findings
            .iter()
            .any(|f| f["code"] == "PlaywrightSourceUnpinned")
    );
    save(
        &root,
        report,
        &item("skipped", json!([{"status":"skipped"}])),
    );
    run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
    save(&root, "run.json", &run);
    assert_eq!(
        trace(&root, "run.json").unwrap().measurements[0]["status"],
        "skipped"
    );
    save(&root, report, &item("expected", json!([])));
    run["report_sha256"] = json!(hash(&fs::read(root.join(report)).unwrap()));
    save(&root, "run.json", &run);
    assert_ne!(trace(&root, "run.json").unwrap().status, "passed");
    put(&root, "openspec/specs/math/spec.md", b"### Requirement: [HG-MATH-REQ] Add\n#### Scenario: [HG-MATH-001] One\n#### Scenario: Missing ID\n");
    let mut missing = run.clone();
    missing["source_hashes"]["openspec/specs/math/spec.md"] = json!(hash(
        &fs::read(root.join("openspec/specs/math/spec.md")).unwrap()
    ));
    save(&root, "run.json", &missing);
    assert!(
        trace(&root, "run.json")
            .unwrap()
            .findings
            .iter()
            .any(|f| f["code"] == "SpecIdMissing")
    );
}
#[test]
fn duplicate_and_unknown_scenario_ids_do_not_pass() {
    let root = root();
    let src = "tests/math.spec.ts";
    let report = "pw.json";
    put(&root, src, b"test fixture");
    save(
        &root,
        report,
        &json!({"config":{"rootDir":root.to_string_lossy()},"suites":[{"specs":[{"id":"spec-1","file":"tests/math.spec.ts","tests":[{"projectName":"chromium","expectedStatus":"passed","status":"expected","annotations":[{"type":"highgrade","description":"HG-UNKNOWN-001"}],"results":[{"status":"passed"}]}]}]}]}),
    );
    let run = base(&root, "playwright", src, report, None);
    save(&root, "run.json", &run);
    assert_eq!(trace(&root, "run.json").unwrap().status, "failed");
    put(&root, "openspec/specs/other/spec.md", spec());
    let mut run = run;
    run["spec_files"] = json!([
        "openspec/specs/math/spec.md",
        "openspec/specs/other/spec.md"
    ]);
    run["source_hashes"]["openspec/specs/other/spec.md"] = json!(hash(spec()));
    save(&root, "run.json", &run);
    assert!(
        trace(&root, "run.json")
            .unwrap()
            .findings
            .iter()
            .any(|f| f["code"] == "DuplicateScenarioId")
    );
}
#[test]
fn playwright_resolves_source_relative_to_report_root() {
    let root = root();
    let src = "web/tests/ui.spec.ts";
    let report = "web/report.json";
    put(&root, src, b"native test source");
    save(
        &root,
        report,
        &json!({"config":{"rootDir":root.join("web").to_string_lossy()},"suites":[{"specs":[{"id":"ui","file":"tests/ui.spec.ts","tests":[{"projectName":"chrome","expectedStatus":"passed","status":"expected","annotations":[{"type":"highgrade","description":"HG-MATH-001"}],"results":[{"status":"passed"}]}]}]}]}),
    );
    let run = base(&root, "playwright", src, report, None);
    save(&root, "run.json", &run);
    assert_eq!(trace(&root, "run.json").unwrap().status, "passed");
}
#[test]
fn native_reports_remain_readable_after_checkout_moves() {
    for tool in ["rust-nextest", "playwright"] {
        let original = root();
        let moved = root();
        let spec_path = "openspec/specs/math/spec.md";
        let source = if tool == "rust-nextest" {
            "tests/math.rs"
        } else {
            "web/ui.spec.ts"
        };
        let report = if tool == "rust-nextest" {
            "junit.xml"
        } else {
            "web/report.json"
        };
        put(
            &original,
            source,
            if tool == "rust-nextest" {
                b"// highgrade: HG-MATH-001\n#[test]\nfn adds() {}\n"
            } else {
                b"native test"
            },
        );
        if tool == "rust-nextest" {
            save(&original, "list.json", &rust_list(&original));
            put(
                &original,
                report,
                br#"<testsuites><testcase classname="math" name="adds"/></testsuites>"#,
            );
        } else {
            save(
                &original,
                report,
                &json!({"config":{"rootDir":original.join("web").to_string_lossy()},"suites":[{"specs":[{"id":"ui","file":"ui.spec.ts","tests":[{"projectName":"chrome","expectedStatus":"passed","status":"expected","annotations":[{"type":"highgrade","description":"HG-MATH-001"}],"results":[{"status":"passed"}]}]}]}]}),
            );
        }
        let run = base(
            &original,
            tool,
            source,
            report,
            if tool == "rust-nextest" {
                Some("list.json")
            } else {
                None
            },
        );
        save(&original, "run.json", &run);
        for rel in
            [spec_path, source, report, "run.json"]
                .into_iter()
                .chain(if tool == "rust-nextest" {
                    Some("list.json")
                } else {
                    None
                })
        {
            put(&moved, rel, &fs::read(original.join(rel)).unwrap());
        }
        assert_eq!(
            trace(&moved, "run.json").unwrap().status,
            "passed",
            "{tool}"
        );
    }
}
