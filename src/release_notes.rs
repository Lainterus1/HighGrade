//! Offline user release notes, authenticated by the Delivery manifest.
use crate::Result;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const PATH: &str = "references/release-notes.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    schema_version: u8,
    releases: Vec<Entry>,
    #[serde(default)]
    unreleased: Option<Changes>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Changes {
    changes: Vec<String>,
    breaking: Vec<String>,
    actions: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    release: String,
    previous: String,
    changes: Vec<String>,
    breaking: Vec<String>,
    actions: Vec<String>,
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn version(s: &str) -> Option<Vec<u64>> {
    s.strip_prefix('v')?
        .split('-')
        .map(|part| part.parse().ok())
        .collect()
}
fn texts(values: &[String]) -> bool {
    values
        .iter()
        .all(|s| !s.trim().is_empty() && s.len() <= 4096)
}
fn append_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

pub fn summary(bytes: &[u8], from: &str, to: &str) -> Result<Value> {
    let history: History =
        serde_json::from_slice(bytes).map_err(|e| format!("ReleaseNotesInvalid: {e}"))?;
    if history.schema_version != 1 || history.releases.len() > 10000 {
        return Err("ReleaseNotesInvalid: schema or entry limit".into());
    }
    if let Some(pending) = &history.unreleased
        && (!texts(&pending.changes) || !texts(&pending.breaking) || !texts(&pending.actions))
    {
        return Err("ReleaseNotesInvalid: unreleased text".into());
    }
    let mut entries = BTreeMap::new();
    for entry in &history.releases {
        if !id(&entry.release)
            || !id(&entry.previous)
            || entry.release == entry.previous
            || !texts(&entry.changes)
            || !texts(&entry.breaking)
            || !texts(&entry.actions)
            || entries.insert(entry.release.as_str(), entry).is_some()
        {
            return Err("ReleaseNotesInvalid: entry or duplicate release".into());
        }
    }
    // A repeated node is invalid even outside the requested transition.
    let mut checked = BTreeSet::new();
    for start in entries.keys() {
        let mut path = BTreeSet::new();
        let mut cursor = *start;
        while !checked.contains(cursor) {
            let Some(entry) = entries.get(cursor) else {
                break;
            };
            if !path.insert(cursor) {
                return Err("ReleaseNotesInvalid: history cycle".into());
            }
            cursor = &entry.previous;
        }
        checked.extend(path);
    }
    let reverse = version(from).zip(version(to)).is_some_and(|(a, b)| a > b);
    let mut cursor = to;
    let mut selected = Vec::new();
    if !reverse {
        while cursor != from {
            let Some(entry) = entries.get(cursor) else {
                break;
            };
            selected.push(*entry);
            cursor = &entry.previous;
        }
    }
    let coverage = if from == to {
        "complete"
    } else if reverse || selected.is_empty() {
        "unavailable"
    } else if cursor == from {
        "complete"
    } else {
        "partial"
    };
    let mut changes = Vec::new();
    let mut breaking = Vec::new();
    let mut actions = Vec::new();
    // Prefer newest changes in the compact view. Required information is never truncated.
    for entry in &selected {
        append_unique(&mut changes, &entry.changes);
        append_unique(&mut breaking, &entry.breaking);
        append_unique(&mut actions, &entry.actions);
    }
    let total = changes.len();
    changes.truncate(5);
    Ok(json!({"from":from,"to":to,"coverage":coverage,
        "releases":selected.iter().rev().map(|e| &e.release).collect::<Vec<_>>(),
        "changes":changes,"total_changes":total,"breaking":breaking,"actions":actions,
        "no_user_changes":if coverage == "unavailable" { None } else { Some(total == 0) },
        "missing_release":if coverage == "partial" { Some(cursor) } else { None },
        "reason":if reverse { Some("reverse_transition") } else if coverage == "unavailable" { Some("history_unavailable") } else { None }
    }))
}
