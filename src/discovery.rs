//! Bounded structural discovery. The legacy inventory contract is unchanged.
use crate::{Result, bootstrap, hash, install, paths};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

pub const MAX_ENTRIES: usize = 5000;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Structure {
    pub entries: BTreeMap<String, String>,
    pub omitted: BTreeMap<String, String>,
    pub complete: bool,
    pub exclusions_sha256: Option<String>,
}
pub fn excluded(rel: &str, exclusions: &std::collections::BTreeSet<String>) -> bool {
    let mut parent = String::new();
    for name in rel.split('/') {
        let child = if parent.is_empty() {
            name.into()
        } else {
            format!("{parent}/{name}")
        };
        if install::contains_exclusion(exclusions, &child)
            || name.eq_ignore_ascii_case(".git")
            || bootstrap::is_secret(name)
            || bootstrap::generated(&parent, name)
            || (parent.eq_ignore_ascii_case(".highgrade") && !name.eq_ignore_ascii_case("project"))
            || (parent.eq_ignore_ascii_case(".highgrade/project")
                && name.to_ascii_lowercase().starts_with("survey"))
        {
            return true;
        }
        parent = child;
    }
    false
}
fn omit(out: &mut Structure, path: &str, reason: &str, incomplete: bool) {
    if out.omitted.len() < 128 {
        out.omitted.insert(path.into(), reason.into());
    }
    if incomplete {
        out.complete = false;
    }
}
pub fn structure(root: &Path, scope: &str) -> Result<Structure> {
    let root = paths::root(root)?;
    let (exclusions, exclusions_sha256) = install::exclusions(&root)?;
    if scope != "." {
        paths::relative(scope)?;
    }
    let mut out = Structure {
        entries: BTreeMap::new(),
        omitted: BTreeMap::new(),
        complete: true,
        exclusions_sha256,
    };
    if scope != "." && excluded(scope, &exclusions) {
        return Err("SurveyScopeExcluded".into());
    }
    let start = if scope == "." {
        root.clone()
    } else {
        paths::safe(&root, scope)?
    };
    if !start.is_dir() {
        return Err("SurveyScopeInvalid: directory required".into());
    }
    let mut pending = vec![(
        if scope == "." {
            String::new()
        } else {
            scope.into()
        },
        0usize,
    )];
    let mut visited = 0usize;
    while let Some((rel, depth)) = pending.pop() {
        if depth > 64 {
            omit(&mut out, &rel, "depth_limit", true);
            continue;
        }
        let dir = if rel.is_empty() {
            root.clone()
        } else {
            match paths::safe(&root, &rel) {
                Ok(p) => p,
                Err(_) => {
                    omit(&mut out, &rel, "unsafe_or_unreadable", true);
                    continue;
                }
            }
        };
        let items = match fs::read_dir(dir) {
            Ok(x) => x,
            Err(_) => {
                omit(&mut out, &rel, "unreadable", true);
                continue;
            }
        };
        for item in items {
            visited += 1;
            if visited > MAX_ENTRIES {
                omit(&mut out, scope, "entry_limit", true);
                return Ok(out);
            }
            let item = match item {
                Ok(x) => x,
                Err(_) => {
                    omit(&mut out, &rel, "unreadable_entry", true);
                    continue;
                }
            };
            let name = match item.file_name().into_string() {
                Ok(x) => x,
                Err(_) => {
                    omit(&mut out, &rel, "non_unicode_entry", true);
                    continue;
                }
            };
            let child = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if excluded(&child, &exclusions) {
                omit(&mut out, &child, "excluded", false);
                continue;
            }
            let safe = match paths::safe(&root, &child) {
                Ok(p) => p,
                Err(_) => {
                    omit(&mut out, &child, "unsafe_or_unreadable", true);
                    continue;
                }
            };
            match fs::metadata(&safe) {
                Ok(m) if m.is_dir() => {
                    out.entries.insert(child.clone(), "directory".into());
                    pending.push((child, depth + 1));
                }
                Ok(m) if m.is_file() => {
                    out.entries.insert(child, "file".into());
                }
                _ => omit(&mut out, &child, "unsupported_or_unreadable", true),
            }
        }
    }
    Ok(out)
}
pub fn source_hash(root: &Path, rel: &str) -> Result<String> {
    let (exclusions, _) = install::exclusions(root)?;
    paths::relative(rel)?;
    if excluded(rel, &exclusions) {
        return Err("SurveySourceExcluded".into());
    }
    Ok(hash(&paths::read_limited(
        &paths::safe(root, rel)?,
        32 * 1024 * 1024,
    )?))
}
