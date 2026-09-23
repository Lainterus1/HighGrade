use crate::{Report, Result, inspect::link_target, install, paths};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub const MAX_ENTRIES: usize = 50_000;
pub const MAX_READ_FILES: usize = 128;
pub const MAX_READ_BYTES: u64 = 64 * 1024;
pub const MAX_ITEMS: usize = 12;
pub const MAX_EXCLUSIONS: usize = 32;
pub const MAX_PATH_BYTES: usize = 200;
pub const MAX_DEPTH: usize = 64;
pub const MAX_OUTPUT_BYTES: usize = 64 * 1024;

pub const REQUIRED: [(&str, Option<&str>); 7] = [
    ("README.md", Some("purpose-navigation")),
    ("AGENTS.md", Some("agent-rules")),
    ("docs/ARCHITECTURE.md", Some("current-architecture")),
    ("docs/ENGINEERING.md", Some("engineering-rules")),
    ("docs/DEVELOPMENT.md", Some("commands-procedures")),
    (".highgrade/project/INSTRUCTIONS.md", None),
    (".highgrade/project/documents.json", None),
];

const LINKS: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (0, 4), (1, 5), (5, 6)];

fn is_reparse(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    let reparse = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let reparse = false;
    meta.file_type().is_symlink() || reparse
}

fn is_secret(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || matches!(
            name.as_str(),
            "secrets" | "credentials" | "id_rsa" | "id_ed25519"
        )
        || name.starts_with("secrets.")
        || name.starts_with("credentials.")
        || [".pem", ".key", ".pfx", ".p12"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

fn generated(_parent: &str, name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "node_modules"
            | ".pnpm-store"
            | ".output"
            | ".tanstack"
            | "target"
            | ".venv"
            | "venv"
            | ".tox"
            | ".pytest_cache"
            | ".mypy_cache"
            | ".ruff_cache"
            | "__pycache__"
            | ".next"
            | ".nuxt"
            | ".turbo"
            | "dist"
            | "build"
            | "coverage"
    )
}

fn skipped_path(rel: &str, project_exclusions: &BTreeSet<String>) -> bool {
    let mut parent = String::new();
    for name in rel.split('/') {
        let child = if parent.is_empty() {
            name.to_owned()
        } else {
            format!("{parent}/{name}")
        };
        if name.eq_ignore_ascii_case(".git")
            || is_secret(name)
            || install::contains_exclusion(project_exclusions, &child)
            || (parent.eq_ignore_ascii_case(".highgrade") && name.eq_ignore_ascii_case("releases"))
            || generated(&parent, name)
        {
            return true;
        }
        parent = child;
    }
    false
}

// Check each component by its actual spelling, including on case-insensitive filesystems.
pub fn file_state(root: &Path, rel: &str) -> &'static str {
    let mut cursor = root.to_path_buf();
    for part in rel.split('/') {
        let entries = match fs::read_dir(&cursor) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return "absent",
            Err(_) => return "unknown",
        };
        let mut found = false;
        for (index, entry) in entries.enumerate() {
            if index >= MAX_ENTRIES {
                return "unknown";
            }
            let Ok(entry) = entry else { return "unknown" };
            if entry.file_name() == part {
                found = true;
                break;
            }
        }
        if !found {
            return "absent";
        }
        cursor.push(part);
        match fs::symlink_metadata(&cursor) {
            Ok(meta) if is_reparse(&meta) => return "unknown",
            Ok(_) => {}
            Err(_) => return "unknown",
        }
    }
    match fs::symlink_metadata(cursor) {
        Ok(meta) if meta.is_file() => "present",
        Ok(_) => "invalid",
        Err(_) => "unknown",
    }
}

fn name_slot(rel: &str) -> Option<usize> {
    let name = rel.rsplit('/').next()?.to_ascii_lowercase();
    match name.as_str() {
        "readme.md" | "overview.md" | "index.md" => Some(0),
        "agents.md" | "claude.md" | "copilot-instructions.md" => Some(1),
        "architecture.md" | "system-design.md" | "system-architecture.md" => Some(2),
        "engineering.md" | "coding-standards.md" | "conventions.md" => Some(3),
        "development.md" | "contributing.md" | "getting-started.md" | "setup.md" => Some(4),
        "instructions.md" | "project-instructions.md" => Some(5),
        "documents.json" if rel.to_ascii_lowercase().contains("highgrade/") => Some(6),
        _ => None,
    }
}

fn label_slot(label: &str) -> Option<usize> {
    let label = label.to_lowercase();
    if label.contains("архитектур")
        || label.contains("architecture")
        || label.contains("устройство")
    {
        Some(2)
    } else if label.contains("инженер")
        || label.contains("engineering")
        || label.contains("соглашен")
    {
        Some(3)
    } else if label.contains("разработ")
        || label.contains("development")
        || label.contains("команд")
    {
        Some(4)
    } else if label.contains("агент") || label.contains("agent") {
        Some(1)
    } else if label.contains("инструкц") || label.contains("instruction") {
        Some(5)
    } else if label.contains("реестр") || label.contains("registry") {
        Some(6)
    } else {
        None
    }
}

fn markdown_links(doc: &str, content: &str) -> Vec<(String, Option<usize>)> {
    let mut out = vec![];
    let mut current: Option<(String, String)> = None;
    for event in Parser::new(content) {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                current = Some((dest_url.into_string(), String::new()));
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, label)) = current.as_mut() {
                    if label.len() < 120 {
                        label.push_str(&text);
                    }
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some((dest, label)) = current.take() {
                    if let Ok((target, _)) = link_target(doc, &dest) {
                        if paths::relative(&target).is_ok() {
                            out.push((target, label_slot(&label)));
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn navigation(
    root: &Path,
    report: &mut Report,
    project_exclusions: Option<&BTreeSet<String>>,
) -> (Vec<Value>, usize) {
    let mut sources = BTreeMap::new();
    let mut read_files = 0;
    for source in [0, 1, 5] {
        let path = REQUIRED[source].0;
        let excluded = project_exclusions.is_some_and(|set| skipped_path(path, set));
        let links = if excluded || file_state(root, path) != "present" {
            None
        } else {
            read_files += 1;
            paths::safe(root, path)
                .and_then(|p| paths::read_limited(&p, MAX_READ_BYTES))
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .map(|text| markdown_links(path, &text))
        };
        if excluded {
            report.finding(
                "unknown",
                "BootstrapLinkSourceExcluded",
                path,
                "Источник ссылок исключён из предварительного обхода.",
            );
        } else if links.is_none() && file_state(root, path) == "present" {
            report.finding(
                "unknown",
                "BootstrapLinkSourceUnreadable",
                path,
                "Источник ссылок недоступен или превышает лимит.",
            );
        }
        sources.insert(source, links);
    }
    let mut rows = vec![];
    for (from, to) in LINKS {
        let source = REQUIRED[from].0;
        let target = REQUIRED[to].0;
        let status = match &sources[&from] {
            None => "unknown",
            Some(links) if links.iter().any(|(path, _)| path == target) => {
                if project_exclusions.is_some_and(|set| skipped_path(target, set)) {
                    "unknown"
                } else if file_state(root, target) == "present" {
                    "present"
                } else {
                    "invalid"
                }
            }
            Some(links)
                if links.iter().any(|(path, slot)| {
                    *slot == Some(to)
                        && path != target
                        && !project_exclusions.is_some_and(|set| skipped_path(path, set))
                }) =>
            {
                "noncanonical"
            }
            Some(_) => "missing",
        };
        if matches!(status, "missing" | "invalid" | "noncanonical") {
            report.finding("failed", "RequiredLinkMissingOrInvalid", source, target);
        }
        rows.push(json!({"from":source,"to":target,"status":status}));
    }
    (rows, read_files)
}

pub fn check_connected(root: &Path, roles: &BTreeMap<String, String>, report: &mut Report) {
    for (path, role) in REQUIRED {
        match file_state(root, path) {
            "present" => {}
            "absent" | "invalid" => report.finding(
                "failed",
                "CanonicalPathMissing",
                path,
                "Обязательный путь отсутствует или не является файлом.",
            ),
            _ => report.finding(
                "unknown",
                "CanonicalPathUnknown",
                path,
                "Обязательный путь не удалось проверить.",
            ),
        }
        if let Some(role) = role {
            if roles.get(role).map(String::as_str) != Some(path) {
                report.finding("failed", "CanonicalRolePathMismatch", path, role);
            }
        }
    }
    let (required_links, _) = navigation(root, report, None);
    report
        .measurements
        .push(json!({"required_links":required_links}));
}

struct Scan {
    root: PathBuf,
    report: Report,
    candidate_scan_performed: bool,
    visited: usize,
    files: usize,
    directories: usize,
    read_files: usize,
    project_exclusions: BTreeSet<String>,
    needs_candidate: [bool; 7],
    candidates: [Vec<Value>; 7],
    candidate_paths: [BTreeSet<String>; 7],
    exclusions: Vec<Value>,
    exclusion_counts: BTreeMap<&'static str, usize>,
    unknown_counts: BTreeMap<&'static str, usize>,
}

impl Scan {
    fn new(root: PathBuf) -> Self {
        let needs_candidate =
            std::array::from_fn(|index| file_state(&root, REQUIRED[index].0) != "present");
        Self {
            root,
            report: Report::new("inspect"),
            candidate_scan_performed: false,
            visited: 0,
            files: 0,
            directories: 0,
            read_files: 0,
            project_exclusions: BTreeSet::new(),
            needs_candidate,
            candidates: std::array::from_fn(|_| vec![]),
            candidate_paths: std::array::from_fn(|_| BTreeSet::new()),
            exclusions: vec![],
            exclusion_counts: BTreeMap::new(),
            unknown_counts: BTreeMap::new(),
        }
    }

    fn unknown(&mut self, code: &'static str, location: &str) {
        let count = self.unknown_counts.entry(code).or_default();
        *count += 1;
        if *count <= MAX_ITEMS {
            self.report.finding(
                "unknown",
                code,
                if location.len() <= MAX_PATH_BYTES {
                    location
                } else {
                    "bootstrap"
                },
                "Область не проверена полностью.",
            );
        }
    }

    fn exclude(&mut self, path: &str, reason: &'static str) {
        *self.exclusion_counts.entry(reason).or_default() += 1;
        if path.len() > MAX_PATH_BYTES {
            self.unknown("BootstrapPathLimit", "bootstrap");
        } else if self.exclusions.len() < MAX_EXCLUSIONS {
            self.exclusions.push(json!({"path":path,"reason":reason}));
        } else {
            self.unknown("BootstrapExclusionLimit", "bootstrap");
        }
    }

    fn candidate(&mut self, slot: usize, path: &str, evidence: &'static str) {
        if !self.needs_candidate[slot] || path == REQUIRED[slot].0 || path.len() > MAX_PATH_BYTES {
            if path.len() > MAX_PATH_BYTES {
                self.unknown("BootstrapPathLimit", "bootstrap");
            }
            return;
        }
        if !self.candidate_paths[slot].insert(path.to_owned()) {
            return;
        }
        if self.candidates[slot].len() < MAX_ITEMS {
            self.candidates[slot].push(json!({"path":path,"evidence":evidence}));
        } else {
            self.unknown("BootstrapCandidateLimit", "bootstrap");
        }
    }

    fn read_known(&mut self, rel: &str) -> Option<String> {
        if self.read_files >= MAX_READ_FILES {
            self.unknown("BootstrapReadLimit", rel);
            return None;
        }
        self.read_files += 1;
        let result = paths::safe(&self.root, rel)
            .and_then(|path| paths::read_limited(&path, MAX_READ_BYTES))
            .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()));
        match result {
            Ok(text) => Some(text),
            Err(_) => {
                self.unknown("BootstrapInputUnreadable", rel);
                None
            }
        }
    }

    fn load_exclusions(&mut self) {
        let rel = install::EXCLUSION_CONFIG;
        match fs::symlink_metadata(self.root.join(rel)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => {
                self.unknown("BootstrapExclusionRegistryUnreadable", rel);
                return;
            }
            Ok(_) => {}
        }
        let Some(text) = self.read_known(rel) else {
            return;
        };
        match install::parse_exclusions(text.as_bytes()) {
            Ok(exclusions) => self.project_exclusions = exclusions,
            Err(_) => self.unknown("BootstrapExclusionRegistryInvalid", rel),
        }
    }

    fn walk(&mut self, dir: &Path, rel: &str, depth: usize) {
        if depth > MAX_DEPTH {
            self.unknown("BootstrapDepthLimit", rel);
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => {
                self.unknown("BootstrapDirectoryUnreadable", rel);
                return;
            }
        };
        let mut sorted = vec![];
        for entry in entries {
            match entry {
                Ok(entry) => sorted.push(entry),
                Err(_) => self.unknown("BootstrapEntryUnreadable", rel),
            }
            if sorted.len() > MAX_ENTRIES {
                self.unknown("BootstrapEntriesLimit", rel);
                break;
            }
        }
        sorted.sort_by_key(|entry| entry.file_name());
        for entry in sorted {
            if self.visited >= MAX_ENTRIES {
                self.unknown("BootstrapEntriesLimit", rel);
                break;
            }
            self.visited += 1;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                self.unknown("BootstrapPathEncoding", rel);
                continue;
            };
            let child = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if name.eq_ignore_ascii_case(".git") {
                self.exclude(&child, "vcs-metadata");
                continue;
            }
            if is_secret(&name) {
                self.exclude(&child, "closed-name");
                continue;
            }
            if install::contains_exclusion(&self.project_exclusions, &child) {
                self.exclude(&child, "project-exclusion");
                continue;
            }
            if rel.eq_ignore_ascii_case(".highgrade") && name.eq_ignore_ascii_case("releases") {
                self.exclude(&child, "installed-release");
                continue;
            }
            let meta = match fs::symlink_metadata(entry.path()) {
                Ok(meta) => meta,
                Err(_) => {
                    self.unknown("BootstrapMetadataUnreadable", &child);
                    continue;
                }
            };
            if is_reparse(&meta) {
                self.exclude(&child, "link-or-reparse");
                self.unknown("BootstrapLinkSkipped", &child);
                continue;
            }
            if meta.is_dir() {
                if generated(rel, &name) {
                    self.exclude(&child, "generated-or-cache");
                    continue;
                }
                self.directories += 1;
                self.walk(&entry.path(), &child, depth + 1);
            } else if meta.is_file() {
                self.files += 1;
                if let Some(slot) = name_slot(&child) {
                    self.candidate(slot, &child, "filename");
                }
            } else {
                self.exclude(&child, "special-file");
                self.unknown("BootstrapSpecialFile", &child);
            }
        }
    }

    fn linked_candidates(&mut self) {
        for source in [0, 1, 5] {
            let rel = REQUIRED[source].0;
            if skipped_path(rel, &self.project_exclusions)
                || file_state(&self.root, rel) != "present"
            {
                continue;
            }
            let Some(text) = self.read_known(rel) else {
                continue;
            };
            for (target, label) in markdown_links(rel, &text) {
                if skipped_path(&target, &self.project_exclusions) {
                    continue;
                }
                if let Some(slot) = label.or_else(|| name_slot(&target)) {
                    if paths::safe(&self.root, &target).is_ok()
                        && fs::symlink_metadata(self.root.join(&target))
                            .is_ok_and(|m| m.is_file() && !is_reparse(&m))
                    {
                        self.candidate(slot, &target, "markdown-link");
                    }
                }
            }
        }
    }

    fn finish(mut self) -> Report {
        if self.candidate_scan_performed {
            self.linked_candidates();
        }
        let mut slots = vec![];
        for (index, (path, role)) in REQUIRED.iter().enumerate() {
            let state = file_state(&self.root, path);
            match state {
                "absent" | "invalid" => self.report.finding(
                    "failed",
                    "CanonicalPathMissing",
                    path,
                    "Обязательный путь отсутствует или не является файлом.",
                ),
                "unknown" => self.report.finding(
                    "unknown",
                    "CanonicalPathUnknown",
                    path,
                    "Обязательный путь не удалось проверить.",
                ),
                _ => {}
            }
            let candidate_status = if state == "present" {
                "not_needed"
            } else if !self.candidates[index].is_empty() {
                "candidate_found"
            } else {
                "not_found_in_scan"
            };
            slots.push(json!({"expected_path":path,"role":role,"path_state":state,"candidate_status":candidate_status,"candidates":self.candidates[index]}));
        }
        let (links, link_reads) =
            navigation(&self.root, &mut self.report, Some(&self.project_exclusions));
        self.read_files += link_reads;
        self.report.measurements.push(json!({
            "mode":"bootstrap",
            "schema_version":1,
            "candidate_scan":if self.candidate_scan_performed {"performed"} else {"not_needed"},
            "slots":slots,
            "required_links":links,
            "visited_entries":self.visited,
            "files_seen":self.files,
            "directories_seen":self.directories,
            "read_files":self.read_files,
            "exclusions":self.exclusions,
            "exclusion_counts":self.exclusion_counts,
            "unknown_counts":self.unknown_counts,
            "limits":{"entries":MAX_ENTRIES,"read_files":MAX_READ_FILES,"read_bytes_per_file":MAX_READ_BYTES,"candidates_per_slot":MAX_ITEMS,"exclusion_paths":MAX_EXCLUSIONS,"path_bytes":MAX_PATH_BYTES,"depth":MAX_DEPTH,"output_bytes":MAX_OUTPUT_BYTES}
        }));
        self.report.limitations.push("Структурный поиск не доказывает отсутствие содержания, полноту аудита или смысловую правильность документов. Кандидатов проверяет агент.".into());
        if !self.candidate_scan_performed {
            self.report.limitations.push("Все обязательные пути найдены: дерево не обходилось для поиска альтернатив; это не полный аудит проекта.".into());
        }
        if !self.project_exclusions.is_empty() {
            self.report.limitations.push("Проектные исключения применены как пути; одобрение аудита и актуальность receipt не проверялись.".into());
        }
        if serde_json::to_vec_pretty(&self.report).is_ok_and(|bytes| bytes.len() > MAX_OUTPUT_BYTES)
        {
            if let Some(value) = self.report.measurements.get_mut(0) {
                if let Some(slots) = value["slots"].as_array_mut() {
                    for slot in slots {
                        slot["candidates"] = json!([]);
                    }
                }
                value["exclusions"] = json!([]);
                value["output_truncated"] = json!(true);
            }
            self.report.finding(
                "unknown",
                "BootstrapOutputLimit",
                "bootstrap",
                "Списки кандидатов и исключений усечены.",
            );
        }
        self.report
    }
}

pub fn bootstrap(root: &Path) -> Result<Report> {
    if !root.is_absolute() {
        return Err("RootInvalid: absolute path required".into());
    }
    let root = paths::root(root)?;
    fs::read_dir(&root).map_err(|_| "RootInvalid: directory unreadable")?;
    let mut scan = Scan::new(root.clone());
    scan.load_exclusions();
    if scan.needs_candidate.iter().any(|needed| *needed) {
        scan.candidate_scan_performed = true;
        scan.walk(&root, "", 0);
    }
    Ok(scan.finish())
}
