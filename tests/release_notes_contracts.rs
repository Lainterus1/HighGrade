use highgrade::release_notes;
use serde_json::{Value, json};

fn notes(entries: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema_version":1,"releases":entries})).unwrap()
}
fn entry(release: &str, previous: &str, changes: Value) -> Value {
    json!({"release":release,"previous":previous,"changes":changes,"breaking":[],"actions":[]})
}
// highgrade: HG-0074-S2, HG-0074-S4
#[test]
fn summary_covers_skipped_releases_and_keeps_all_actions() {
    let mut latest = entry("v0-3-3", "v0-3-2", json!(["shared", "new", "six", "seven"]));
    latest["actions"] = json!(["1", "2", "3", "4", "5", "6"]);
    latest["breaking"] = json!(["format"]);
    let data = notes(json!([
        entry(
            "v0-3-2",
            "v0-3-1",
            json!(["shared", "two", "three", "four"])
        ),
        latest
    ]));
    let s = release_notes::summary(&data, "v0-3-1", "v0-3-3").unwrap();
    assert_eq!(s["coverage"], "complete");
    assert_eq!(s["releases"], json!(["v0-3-2", "v0-3-3"]));
    assert_eq!(s["changes"].as_array().unwrap().len(), 5);
    assert_eq!(s["total_changes"], 7);
    assert_eq!(s["actions"].as_array().unwrap().len(), 6);
    assert_eq!(s["breaking"], json!(["format"]));
}
// highgrade: HG-0074-S3, HG-0074-S4
#[test]
fn summary_states_gaps_unknown_reverse_and_empty_changes() {
    let data = notes(json!([entry("v0-3-3", "v0-3-2", json!([]))]));
    let partial = release_notes::summary(&data, "v0-3-1", "v0-3-3").unwrap();
    assert_eq!(partial["coverage"], "partial");
    assert_eq!(partial["missing_release"], "v0-3-2");
    assert_eq!(partial["no_user_changes"], true);
    assert_eq!(
        release_notes::summary(&data, "v0-3-3", "v0-3-1").unwrap()["coverage"],
        "unavailable"
    );
    assert_eq!(
        release_notes::summary(&data, "v0-3-3", "v0-3-3").unwrap()["coverage"],
        "complete"
    );
}
#[test]
fn malformed_history_is_rejected_instead_of_invented() {
    let mut x = entry("v0-3-3", "v0-3-2", json!(["new"]));
    x["previous"] = json!("v0-3-3");
    assert!(release_notes::summary(&notes(json!([x])), "v0-3-1", "v0-3-3").is_err());
    let x = entry("v0-3-3", "v0-3-2", json!([]));
    assert!(release_notes::summary(&notes(json!([x.clone(), x])), "v0-3-1", "v0-3-3").is_err());
    assert!(
        release_notes::summary(br#"{"schema_version":2,"releases":[]}"#, "v0-3-1", "v0-3-3")
            .is_err()
    );
}

// highgrade: HG-0074-S3
#[test]
fn absent_history_and_cycles_outside_transition_are_explicit() {
    let empty = notes(json!([]));
    let unavailable = release_notes::summary(&empty, "v0-3-1", "v0-3-3").unwrap();
    assert_eq!(unavailable["coverage"], "unavailable");
    assert_eq!(unavailable["reason"], "history_unavailable");
    assert!(unavailable["no_user_changes"].is_null());
    assert_eq!(unavailable["changes"], json!([]));
    let cyclic = notes(json!([
        entry("v0-3-3", "v0-3-2", json!(["new"])),
        entry("other-a", "other-b", json!([])),
        entry("other-b", "other-a", json!([]))
    ]));
    assert!(
        release_notes::summary(&cyclic, "v0-3-2", "v0-3-3")
            .unwrap_err()
            .contains("history cycle")
    );
}

// highgrade: HG-0074-S4
#[test]
fn empty_complete_transition_retains_all_required_information() {
    let mut record = entry("v0-3-3", "v0-3-2", json!([]));
    record["breaking"] = json!(["b1", "b2", "b3", "b4", "b5", "b6"]);
    record["actions"] = json!(["a1", "a2", "a3", "a4", "a5", "a6"]);
    let summary = release_notes::summary(&notes(json!([record])), "v0-3-2", "v0-3-3").unwrap();
    assert_eq!(summary["coverage"], "complete");
    assert_eq!(summary["no_user_changes"], true);
    assert_eq!(summary["changes"], json!([]));
    assert_eq!(summary["total_changes"], 0);
    assert_eq!(summary["breaking"].as_array().unwrap().len(), 6);
    assert_eq!(summary["actions"].as_array().unwrap().len(), 6);
}
