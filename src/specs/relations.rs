//! Flat vocabulary and explicit links; neither feature inherits specification text.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub id: String,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Tag {
    pub title: String,
    pub description: String,
}

pub fn validate(s: &Store) -> Result<()> {
    if s.schema_version < 3
        && (!s.tags.is_empty()
            || s.changes.values().any(|c| {
                !c.tags.is_empty()
                    || !c.depends_on.is_empty()
                    || !c.related_to.is_empty()
                    || !c.history.is_empty()
            }))
    {
        return Err("MigrationRequired: catalog metadata".into());
    }
    for (id, tag) in &s.tags {
        check_id(id, "tag")?;
        if tag.title.trim().is_empty() || tag.description.trim().is_empty() {
            return Err("TagIncomplete".into());
        }
    }
    for c in s.changes.values() {
        for tag in &c.tags {
            if !s.tags.contains_key(tag) {
                return Err(format!("UnknownTag: {tag}"));
            }
        }
        for links in [&c.depends_on, &c.related_to] {
            let mut seen = BTreeSet::new();
            for link in links {
                if link.id == c.id || !s.changes.contains_key(&link.id) {
                    return Err(format!("InvalidLink: {} -> {}", c.id, link.id));
                }
                if link.reason.trim().is_empty() || !seen.insert(&link.id) {
                    return Err("LinkReasonOrUniquenessMissing".into());
                }
            }
        }
    }
    // Kahn traversal avoids stack overflow on large imported dependency graphs.
    let mut degrees: BTreeMap<&str, usize> = s
        .changes
        .values()
        .map(|c| (c.id.as_str(), c.depends_on.len()))
        .collect();
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for c in s.changes.values() {
        for link in &c.depends_on {
            children.entry(&link.id).or_default().push(&c.id);
        }
    }
    let mut pending: Vec<_> = degrees
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut count = 0;
    while let Some(id) = pending.pop() {
        count += 1;
        for child in children.get(id).into_iter().flatten() {
            let degree = degrees.get_mut(child).unwrap();
            *degree -= 1;
            if *degree == 0 {
                pending.push(child);
            }
        }
    }
    if count != s.changes.len() {
        return Err("DependencyCycle".into());
    }
    Ok(())
}
pub fn mutation(s: &mut Store, op: &str, o: &BTreeMap<String, String>) -> Result<()> {
    if s.schema_version != 3 {
        return Err("MigrationRequired: tags require directory catalog".into());
    }
    let get = |k: &str| {
        o.get(k)
            .map(String::as_str)
            .ok_or_else(|| format!("Usage: required {k}"))
    };
    match op {
        "spec-tag-set" => {
            let id = get("--id")?;
            check_id(id, "tag")?;
            s.tags.insert(
                id.into(),
                Tag {
                    title: get("--title")?.into(),
                    description: get("--description")?.into(),
                },
            );
        }
        "spec-tag-remove" => {
            let id = get("--id")?;
            if s.changes.values().any(|c| c.tags.contains(id)) {
                return Err("TagInUse: merge or remove references first".into());
            }
            if s.tags.remove(id).is_none() {
                return Err("UnknownTag".into());
            }
        }
        "spec-tag-merge" => {
            let from = get("--from")?;
            let into = get("--into")?;
            if from == into || !s.tags.contains_key(from) || !s.tags.contains_key(into) {
                return Err("InvalidTagMerge".into());
            }
            for c in s.changes.values_mut() {
                if c.tags.remove(from) {
                    c.tags.insert(into.into());
                }
            }
            s.tags.remove(from);
        }
        _ => unreachable!(),
    }
    validate(s)
}
pub fn metadata(s: &Store, c: &Change) -> Value {
    let resolve = |links: &Vec<Link>| {
        links.iter().map(|l|json!({"id":l.id,"reason":l.reason,"title":s.changes[&l.id].title,"integrated":s.changes[&l.id].archived && s.changes[&l.id].abandoned_reason.is_none()})).collect::<Vec<_>>()
    };
    let touched: BTreeSet<_> = c.operations.iter().map(Delta::id).collect();
    let overlaps = s
        .changes
        .values()
        .filter(|other| {
            other.id != c.id && other.operations.iter().any(|d| touched.contains(d.id()))
        })
        .map(|other| json!({"id":other.id,"title":other.title}))
        .collect::<Vec<_>>();
    json!({"depends_on":resolve(&c.depends_on),"related_to":resolve(&c.related_to),"same_requirements":overlaps})
}
pub fn list(root: &Path, s: &Store, o: &BTreeMap<String, String>) -> Result<Value> {
    if let Some(tag) = o.get("--tag") {
        if !s.tags.contains_key(tag) {
            return Err("UnknownTag".into());
        }
    }
    for (flag, valid) in [
        ("--technical", &["ready", "in_progress", "abandoned"][..]),
        (
            "--human",
            &["accepted", "needs_changes", "pending", "stale"][..],
        ),
    ] {
        if o.get(flag).is_some_and(|v| !valid.contains(&v.as_str())) {
            return Err(format!("InvalidFilter: {flag}"));
        }
    }
    if o.get("--id").is_some_and(|id| !s.changes.contains_key(id)) {
        return Err("ChangeMissing".into());
    }
    let selected: Vec<_> = s
        .changes
        .values()
        .filter(|c| {
            o.get("--id").is_none_or(|id| *id == c.id)
                && o.get("--tag").is_none_or(|tag| c.tags.contains(tag))
        })
        .collect();
    let mut v = progress::list(root, s, &selected);
    v["summary_scope"] = json!("id/tag selection before technical/human filters");
    let changes = v["changes"].as_array_mut().unwrap();
    changes.retain(|row| {
        let c = &s.changes[row["id"].as_str().unwrap()];
        o.get("--tag").is_none_or(|t| c.tags.contains(t))
            && o.get("--technical").is_none_or(|t| row["technical"] == *t)
            && o.get("--human").is_none_or(|t| row["human"] == *t)
    });
    for row in changes.iter_mut() {
        let c = &s.changes[row["id"].as_str().unwrap()];
        row["tags"] = json!(c.tags);
        row["links"] = metadata(s, c);
    }
    let selected = changes.len();
    v["selection"] =
        json!({"selected":selected,"total":s.changes.len(),"filters":o,"context_complete":false});
    Ok(v)
}
