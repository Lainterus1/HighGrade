//! Regression for causal evidence scopes; no new normative scenario is added.
mod support;
use highgrade::specs;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use support::TestDir;
fn root() -> TestDir {
    TestDir::new("hg-evidence-scopes-")
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
        &[("--title", "Scope fixture"), ("--expected", &sha(root))],
    )
    .unwrap();
}
fn put(root: &Path, rel: &str, value: &Value) {
    fs::write(root.join(rel), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn edit(root: &Path, id: &str, field: &str, value: Value) -> highgrade::Result<highgrade::Report> {
    put(root, "edit.json", &json!({field:value}));
    call(
        root,
        "spec-edit",
        &[
            ("--id", id),
            ("--input", "edit.json"),
            ("--expected", &sha(root)),
        ],
    )
}
fn brief_data(report: &highgrade::Report) -> &Value {
    report
        .measurements
        .iter()
        .find_map(|m| m.get("brief"))
        .unwrap()
}
fn runner_fixture(root: &Path) {
    create(root);
    fs::write(root.join("mode.txt"), "normal").unwrap();
    fs::write(
        root.join("runner.py"),
        r#"import importlib.util, sys, unittest, xml.etree.ElementTree as ET, pathlib
source, selector, report = sys.argv[1:]
assert pathlib.Path('mode.txt').read_text() == 'normal'
spec=importlib.util.spec_from_file_location('subject',source)
module=importlib.util.module_from_spec(spec); spec.loader.exec_module(module)
suite=unittest.defaultTestLoader.loadTestsFromName(selector.replace('::','.'),module)
result=unittest.TestResult(); suite.run(result)
root=ET.Element('testsuite')
case=ET.SubElement(root,'testcase',classname=selector.split('::')[0],name=selector.split('::')[1])
if result.failures or result.errors: ET.SubElement(case,'failure')
if result.skipped: ET.SubElement(case,'skipped')
ET.ElementTree(root).write(report)
sys.exit(0 if result.wasSuccessful() else 1)
"#,
    )
    .unwrap();
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
}

#[test]
fn causal_runner_scopes_preserve_siblings_but_invalidate_real_dependencies() {
    // Executable support for HG-0043-R2. This exercises evidence invalidation,
    // not the separate normative review or an agent's procedure execution.
    let root = root();
    runner_fixture(&root);
    create(&root);
    fs::write(root.join("shared.py"), "EXPECTED = 5\n").unwrap();
    for (id, name) in [("HG-0001", "maintenance"), ("HG-0002", "interface")] {
        fs::write(root.join(format!("{name}.txt")), "5").unwrap();
        fs::write(root.join(format!("test_{name}.py")), format!(
            "import unittest, pathlib\nfrom shared import EXPECTED\nclass Cases(unittest.TestCase):\n    def test_sum(self): self.assertEqual(int(pathlib.Path('{name}.txt').read_text()), EXPECTED)\n"
        )).unwrap();
        edit(
            &root,
            id,
            "shared_inputs",
            json!(["shared.py", "runner.py", "mode.txt"]),
        )
        .unwrap();
        edit(
            &root,
            id,
            "checks",
            json!([{
                "id":"sum", "scenario_ids":[format!("{id}-S1")], "runner":"unit",
                "file":format!("test_{name}.py"), "selector":"Cases::test_sum",
                "preparation":"Create independent fixture and shared expected value",
                "action":"Compare fixture with shared value", "observation":"Value is 5",
                "inputs":[format!("{name}.txt")]
            }]),
        )
        .unwrap();
        call(
            &root,
            "spec-run",
            &[
                ("--id", id),
                ("--check", "all"),
                ("--expected", &sha(&root)),
            ],
        )
        .unwrap();
    }
    let has_stale_check = |id: &str| {
        let report = call(&root, "spec-check", &[("--id", id), ("--brief", "true")]).unwrap();
        brief_data(&report)["blockers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["kind"] == "check")
    };
    assert!(!has_stale_check("HG-0001"));
    assert!(!has_stale_check("HG-0002"));
    let history = specs::load(&root).unwrap().0;
    for unrelated in ["interface-document.md", "delivery-manifest.json"] {
        fs::write(root.join(unrelated), "changed unrelated input").unwrap();
        assert!(!has_stale_check("HG-0001"));
        assert!(!has_stale_check("HG-0002"));
    }
    // Each implementation fixture and its test only invalidate their own run.
    for (file, affected, unaffected) in [
        ("maintenance.txt", "HG-0001", "HG-0002"),
        ("test_maintenance.py", "HG-0001", "HG-0002"),
        ("interface.txt", "HG-0002", "HG-0001"),
    ] {
        let bytes = fs::read(root.join(file)).unwrap();
        fs::write(root.join(file), "changed").unwrap();
        assert!(has_stale_check(affected), "{file}");
        assert!(!has_stale_check(unaffected), "{file}");
        fs::write(root.join(file), bytes).unwrap();
    }
    // Shared implementation, runner and config remain causal dependencies.
    for file in ["shared.py", "runner.py", "mode.txt", "specs/runners.json"] {
        let bytes = fs::read(root.join(file)).unwrap();
        let changed = if file.ends_with(".json") {
            String::from_utf8(bytes.clone())
                .unwrap()
                .replace("30", "31")
        } else {
            "changed".to_string()
        };
        fs::write(root.join(file), changed).unwrap();
        assert!(has_stale_check("HG-0001"), "{file}");
        assert!(has_stale_check("HG-0002"), "{file}");
        fs::write(root.join(file), bytes).unwrap();
    }
    assert_eq!(specs::load(&root).unwrap().0.changes, history.changes);
    edit(
        &root,
        "HG-0001",
        "shared_inputs",
        json!(["runner.py", "mode.txt"]),
    )
    .unwrap();
    assert!(has_stale_check("HG-0001"));
    assert!(!has_stale_check("HG-0002"));
    assert_eq!(
        specs::load(&root).unwrap().0.changes["HG-0001"].runs,
        history.changes["HG-0001"].runs
    );
}
