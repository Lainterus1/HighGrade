use crate::{Report, Result, hash, paths, read_json};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '-' || *c == '_')
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect()
}
fn anchors(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut heading = None;
    for event in Parser::new(text) {
        match event {
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::Text(s) | Event::Code(s) => {
                if let Some(h) = heading.as_mut() {
                    h.push_str(&s)
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(h) = heading.take() {
                    let base = slug(&h);
                    let count = counts.entry(base.clone()).or_default();
                    let id = if *count == 0 {
                        base
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    out.insert(id);
                }
            }
            _ => {}
        }
    }
    out
}
fn decode(s: &str) -> Result<String> {
    let mut b = vec![];
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("MalformedPercentEscape".into());
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|e| e.to_string())?;
            b.push(u8::from_str_radix(hex, 16).map_err(|e| e.to_string())?);
            i += 3;
        } else {
            b.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(b).map_err(|e| e.to_string())
}
pub(crate) fn link_target(doc: &str, dest: &str) -> Result<(String, Option<String>)> {
    let (path, anchor) = dest
        .split_once('#')
        .map_or((dest, None), |(p, a)| (p, Some(a)));
    let path = decode(path)?;
    if path.starts_with('/') || path.contains(['\\', ':', '?']) {
        return Err("UnsupportedOrAbsoluteLink".into());
    }
    let mut parts: Vec<&str> = doc.split('/').collect();
    parts.pop();
    if path.is_empty() {
        return Ok((doc.into(), anchor.map(decode).transpose()?));
    }
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("LinkOutsideRoot".into());
                }
            }
            _ => parts.push(part),
        }
    }
    Ok((parts.join("/"), anchor.map(decode).transpose()?))
}
fn links(root: &Path, doc: &str, text: &str, r: &mut Report) {
    let unresolved = std::cell::RefCell::new(vec![]);
    let callback = |link: pulldown_cmark::BrokenLink<'_>| {
        unresolved.borrow_mut().push(link.reference.to_string());
        None
    };
    let parser = Parser::new_with_broken_link_callback(
        text,
        Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS,
        Some(callback),
    );
    for event in parser {
        if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) = event {
            let dest = dest_url.as_ref();
            if dest.starts_with("https://")
                || dest.starts_with("http://")
                || dest.starts_with("mailto:")
            {
                r.finding("warning", "ExternalLinkUnchecked", doc, dest);
                continue;
            }
            let check = (|| -> Result<()> {
                let (target, anchor) = link_target(doc, dest)?;
                let p = paths::safe(root, &target)?;
                if !p.exists() {
                    return Err(format!("LinkMissing: {dest}"));
                }
                if let Some(a) = anchor.filter(|a| !a.is_empty()) {
                    if p.extension().and_then(|s| s.to_str()) != Some("md") {
                        return Err(format!("AnchorUnsupported: {dest}"));
                    }
                    let bytes = paths::read_limited(&p, 8 * 1024 * 1024)?;
                    let target_text = String::from_utf8(bytes).map_err(|e| e.to_string())?;
                    if !anchors(&target_text).contains(&a) {
                        return Err(format!("AnchorMissing: {dest}"));
                    }
                }
                Ok(())
            })();
            if let Err(e) = check {
                r.finding("failed", "LinkInvalid", doc, &e);
            }
        }
    }
    for reference in unresolved.into_inner() {
        r.finding("failed", "UnresolvedReference", doc, &reference);
    }
}
fn budget(value: Option<&Value>, size: u64, where_: &str, r: &mut Report) {
    let Some(v) = value.filter(|v| !v.is_null()) else {
        r.finding(
            "unknown",
            "BudgetNotAgreed",
            where_,
            "Бюджет не согласован.",
        );
        return;
    };
    let valid = (|| -> Result<u64> {
        if v["unit"] != "bytes" || v["agreed"] != true {
            return Err("Нужен согласованный бюджет в bytes".into());
        }
        let n = |k: &str| v[k].as_u64().ok_or_else(|| format!("Missing budget.{k}"));
        let (min, base, current, max) = (n("min")?, n("baseline")?, n("current")?, n("ceiling")?);
        if min == 0 || !(min <= base && base <= max && min <= current && current <= max) {
            return Err("Диапазон бюджета нарушен".into());
        }
        if v["review_trigger"]
            .as_str()
            .is_none_or(|s| s.trim().is_empty())
        {
            return Err("Не задано условие пересмотра".into());
        }
        let history = v["history"].as_array().ok_or("Нет истории бюджета")?;
        let mut previous = base;
        for item in history {
            if item["from"].as_u64() != Some(previous)
                || item["reason"].as_str().is_none_or(|s| s.trim().is_empty())
            {
                return Err("Разрыв истории бюджета".into());
            }
            previous = item["to"].as_u64().ok_or("Некорректный budget.to")?;
            if previous < min || previous > max {
                return Err("Историческое значение вне диапазона".into());
            }
        }
        if previous != current {
            return Err("Текущий бюджет не соответствует истории".into());
        }
        Ok(current)
    })();
    match valid {
        Ok(limit) if size > limit => r.finding(
            "warning",
            "BudgetExceeded",
            where_,
            &format!("{size} bytes > {limit}; требуется пересмотр, не автоматическое повышение"),
        ),
        Ok(_) => {}
        Err(e) => r.finding("failed", "BudgetInvalid", where_, &e),
    }
}
fn within(path: &str, area: &str) -> bool {
    path == area || path.starts_with(&format!("{area}/"))
}
fn uncovered(
    root: &Path,
    rel: &str,
    areas: &BTreeSet<String>,
    exclusions: &BTreeSet<String>,
    r: &mut Report,
) -> Result<()> {
    if exclusions.contains(rel) {
        return Ok(());
    }
    if areas.iter().any(|a| within(rel, a)) {
        return Ok(());
    }
    if !areas.iter().any(|a| within(a, rel)) {
        r.finding(
            "unknown",
            "AreaUnclassified",
            rel,
            "Область не назначена источнику документации.",
        );
        return Ok(());
    }
    for item in fs::read_dir(paths::safe(root, rel)?).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let child = format!(
            "{rel}/{}",
            item.file_name().to_str().ok_or("InvalidUnicodePath")?
        );
        if exclusions.contains(&child) {
            continue;
        }
        let path = paths::safe(root, &child)?;
        if path.is_dir() {
            uncovered(root, &child, areas, exclusions, r)?;
        } else if !areas.iter().any(|a| within(&child, a)) {
            r.finding(
                "unknown",
                "AreaUnclassified",
                &child,
                "Файл вне объявленных областей.",
            );
        }
    }
    Ok(())
}
pub fn inspect(root: &Path, registry: Option<&str>, scope: Option<&str>) -> Result<Report> {
    let root = paths::root(root)?;
    let (exclusions, _) = crate::install::exclusions(&root)?;
    if let Some(scope) = scope {
        paths::safe(&root, scope)?;
    }
    let reg = if let Some(reg) = registry {
        reg.to_owned()
    } else {
        let candidates = [
            ".highgrade/project/documents.json",
            ".mycodex/project/documents.json",
        ];
        let mut found = vec![];
        for candidate in candidates {
            if paths::safe(&root, candidate)?.exists() {
                found.push(candidate);
            }
        }
        if found.len() != 1 {
            return Err("RegistryMissingOrAmbiguous: задайте --registry REL".into());
        }
        found[0].to_owned()
    };
    let config = read_json(&paths::safe(&root, &reg)?)?;
    if config.get("schema_version").is_some_and(|v| v != 1) {
        return Err("SchemaUnsupported".into());
    }
    let docs = config["documents"]
        .as_array()
        .ok_or("ConfigInvalid: documents должен быть массивом")?;
    let mut r = Report::new("inspect");
    let mut ids = BTreeSet::new();
    let mut roles = BTreeSet::new();
    let mut role_paths = BTreeMap::<String, String>::new();
    let mut known_paths = BTreeSet::new();
    let mut rows: Vec<&Value> = docs.iter().collect();
    if let Some(extra) = config.get("additional_sources") {
        for row in extra
            .as_array()
            .ok_or("ConfigInvalid: additional_sources")?
        {
            if row["active"] != false {
                rows.push(row);
            }
        }
    }
    let mut route_bytes = 0u64;
    let mut route_ids = vec![];
    let mut classified = BTreeSet::new();
    let mut scope_covered = false;
    for (index, doc) in rows.iter().enumerate() {
        let path = doc["path"].as_str().ok_or("ConfigInvalid: document.path")?;
        let role = doc["role"].as_str().ok_or("ConfigInvalid: document.role")?;
        if index < docs.len() {
            let id = doc["id"].as_str().ok_or("ConfigInvalid: document.id")?;
            if !ids.insert(id) {
                r.finding("failed", "DuplicateId", path, id);
            }
            if !roles.insert(role) {
                r.finding("failed", "DuplicateRole", path, role);
            }
            role_paths.insert(role.to_owned(), path.to_owned());
        }
        if !known_paths.insert(path.to_lowercase()) {
            r.finding("failed", "DuplicatePath", path, "Путь назначен повторно.");
        }
        paths::relative(path)?;
        classified.insert(path.to_owned());
        let scopes: Vec<&str> = match doc.get("scope") {
            Some(v) => v
                .as_array()
                .ok_or("ConfigInvalid: scope")?
                .iter()
                .map(|s| s.as_str().ok_or("ConfigInvalid: scope item"))
                .collect::<std::result::Result<_, _>>()?,
            None => vec![],
        };
        for s in &scopes {
            paths::relative(s.trim_end_matches('/'))?;
            classified.insert(s.trim_end_matches('/').to_owned());
        }
        scope_covered |= scope == Some(path)
            || scopes
                .iter()
                .any(|s| scope.is_some_and(|p| within(p, s.trim_end_matches('/'))));
        let selected = scope.is_none()
            || doc["loading"] == "entry"
            || scopes.iter().any(|s| {
                scope.is_some_and(|p| {
                    within(p, s.trim_end_matches('/')) || within(s.trim_end_matches('/'), p)
                })
            })
            || scope == Some(path);
        if !selected {
            continue;
        }
        match paths::safe(&root, path).and_then(|p| paths::read_limited(&p, 8 * 1024 * 1024)) {
            Ok(data) => match std::str::from_utf8(&data) {
                Ok(text) => {
                    let size = data.len() as u64;
                    r.measurements.push(json!({"path":path,"role":role,"bytes":size,"lines":text.lines().count(),"sha256":hash(&data),"token_estimate":text.chars().count().div_ceil(4),"token_method":"unicode_chars/4 heuristic; not model context size"}));
                    if doc["loading"] != "history" {
                        route_bytes += size;
                        route_ids.push(path);
                    }
                    if index < docs.len() {
                        budget(doc.get("budget"), size, path, &mut r);
                    }
                    if path.ends_with(".md") {
                        links(&root, path, text, &mut r);
                    }
                }
                Err(_) => r.finding("failed", "InvalidUtf8", path, "Документ не UTF-8"),
            },
            Err(e) => r.finding("failed", "DocumentUnreadable", path, &e),
        }
    }
    for role in [
        "purpose-navigation",
        "agent-rules",
        "current-architecture",
        "engineering-rules",
        "commands-procedures",
    ] {
        if !roles.contains(role) {
            r.finding("failed", "RequiredRoleMissing", &reg, role);
        }
    }
    crate::bootstrap::check_connected(&root, &role_paths, &mut r);
    if scope.is_some() && !scope_covered {
        r.finding(
            "unknown",
            "ScopeUnclassified",
            scope.unwrap(),
            "Нет явного источника, покрывающего всю запрошенную область.",
        );
    }
    for item in fs::read_dir(&root).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let name = item.file_name().to_string_lossy().into_owned();
        if ![
            ".git",
            ".highgrade",
            ".mycodex",
            ".agents",
            "target",
            "node_modules",
            ".venv",
        ]
        .contains(&name.as_str())
            && !exclusions.contains(&name)
            && item.file_type().map_err(|e| e.to_string())?.is_dir()
        {
            uncovered(&root, &name, &classified, &exclusions, &mut r)?;
        }
    }
    budget(
        config.get("route_budget"),
        route_bytes,
        "context-route",
        &mut r,
    );
    r.measurements.push(json!({"route":route_ids,"route_bytes":route_bytes,"scope":scope.unwrap_or("all-registered"),"registry":reg}));
    r.limitations.push("Структурная проверка, не смысловой аудит. Сеть не использована. История бюджета проверяется внутри записи; изменение согласованной базы требует отдельного сравнения/решения автора.".into());
    r.limitations.push("Якоря: стандартные Markdown-заголовки, Unicode lowercase; HTML/custom anchors и отдельные диалекты могут требовать ручной проверки. --scope использует явную карту, не граф зависимостей кода.".into());
    Ok(r)
}
