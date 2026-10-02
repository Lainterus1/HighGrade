//! Compact reading projection; never replaces stored evidence or authenticates a run.
use super::*;
fn short(text: &str) -> Value {
    json!({"text":text.chars().take(240).collect::<String>(),"truncated":text.chars().count()>240})
}
pub(super) fn view(root: &Path, c: &Change) -> Value {
    let cards: Vec<Value> = c
        .evidence
        .values()
        .map(|e| {
            let scenario = c
                .operations
                .iter()
                .filter_map(Delta::requirement)
                .find_map(|r| {
                    r.scenarios
                        .iter()
                        .find(|s| s.id == e.scenario)
                        .map(|s| (r, s))
                });
            let mut issues = Vec::new();
            if e.report.is_empty() || !e.files.contains_key(&e.report) {
                issues.push(json!({"reason":"report_missing"}));
            }
            if !scenario.is_some_and(|(r, s)| scenario_revision(r, s) == e.scenario_sha256) {
                issues.push(json!({"reason":"scenario_changed_or_unavailable"}));
            }
            if e.files.is_empty() {
                issues.push(json!({"reason":"inputs_missing"}));
            }
            for (path, expected) in &e.files {
                match fingerprint(root, path) {
                    Ok(actual) if actual == *expected => (),
                    Ok(_) => issues.push(json!({"reason":"input_changed","path":path})),
                    Err(_) => issues.push(json!({"reason":"input_unavailable","path":path})),
                }
            }
            json!({"scenario":e.scenario,"method":e.method,"command":short(&e.command),
            "environment":"see_source_report","captured_at":e.captured_at,"outcome":e.outcome,"observation":short(&e.observation),
            "inputs_sha256":digest(&e.files),"input_count":e.files.len(),
            "source":{"path":e.report,"sha256":e.files.get(&e.report)},
            "integrity":if issues.is_empty(){"current"}else{"stale"},
            "issues":issues.iter().take(3).collect::<Vec<_>>(),"issue_count":issues.len()})
        })
        .collect();
    json!({"id":c.id,"cards":cards,"detail":"spec-read --id ID --view full",
        "limitations":"Recorded observations, not proof of authenticity or acceptance. Environment and limits remain in the source report; integrity is independent of outcome."})
}
