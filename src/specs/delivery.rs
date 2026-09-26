//! Read-only audit of the selected evidence delivery, independent of readiness.
use super::*;
use std::process::Command;

pub fn diagnose(root: &Path, id: &str, bundle: Option<&str>) -> Result<Report> {
    let root = paths::root(root)?;
    let (store, _) = load(&root)?;
    let change = store.changes.get(id).ok_or("ChangeMissing")?;
    let mut r = Report::new("doctor");
    let selected = bundle
        .map(|p| -> Result<BTreeSet<String>> {
            let values: BTreeSet<String> =
                decode(&paths::read_limited(&paths::safe(&root, p)?, LIMIT)?, p)?;
            for path in &values {
                paths::safe(&root, path)?;
            }
            Ok(values)
        })
        .transpose()?;
    let git = |args: &[&str]| {
        Command::new("git")
            .args(["--no-pager", "--no-replace-objects"])
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args(if args.first() == Some(&"check-ignore") {
                vec![]
            } else {
                vec!["--literal-pathspecs"]
            })
            .args(args)
            .current_dir(&root)
            .output()
    };
    let git_available = git(&["rev-parse", "--is-inside-work-tree"])
        .is_ok_and(|o| o.status.success() && o.stdout.starts_with(b"true"));
    let mut files: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for scenario in change
        .operations
        .iter()
        .filter_map(Delta::requirement)
        .flat_map(|q| &q.scenarios)
    {
        if let Some(e) = change.evidence.get(&scenario.id) {
            for (path, hash) in &e.files {
                files.entry(path.clone()).or_default().insert(hash.clone());
            }
            if !e.files.contains_key(&e.report) {
                r.finding(
                    "unknown",
                    "DeliveryReportUnpinned",
                    &e.report,
                    "Обязательный отчёт не имеет записанного хеша.",
                );
            }
        } else {
            r.finding(
                "unknown",
                "DeliveryEvidenceMissing",
                &scenario.id,
                "Нет текущего обязательного доказательства.",
            );
        }
    }
    for check in &change.checks {
        if check.runner.is_some() {
            if let Some(run) = change.runs.iter().rev().find(|run| run.check == check.id) {
                for (path, hash) in &run.files {
                    files.entry(path.clone()).or_default().insert(hash.clone());
                }
            } else {
                r.finding(
                    "unknown",
                    "DeliveryRunMissing",
                    &check.id,
                    "Нет отчёта обязательной проверки.",
                );
            }
        }
    }
    if files.is_empty() {
        r.finding(
            "unknown",
            "DeliveryEmpty",
            id,
            "Нет закреплённых файлов доказательств.",
        );
    }
    let mut entries = Vec::new();
    for (path, hashes) in files {
        let bytes =
            paths::safe(&root, &path).and_then(|p| paths::read_limited(&p, 64 * 1024 * 1024));
        let current = bytes
            .as_ref()
            .is_ok_and(|b| hashes.len() == 1 && hashes.contains(&hash(b)));
        if !current {
            r.finding(
                "failed",
                "DeliveryInputChangedOrMissing",
                &path,
                "Файл отсутствует, недоступен либо отличается от обязательного доказательства.",
            );
        }
        let composition = if let Some(selected) = &selected {
            if selected.contains(&path) {
                "selected"
            } else {
                "not_selected"
            }
        } else if git_available {
            let index = git(&["ls-files", "--stage", "-z", "--", &path]);
            let records = index.as_ref().ok().filter(|o| o.status.success()).map(|o| {
                o.stdout
                    .split(|b| *b == 0)
                    .filter(|v| !v.is_empty())
                    .collect::<Vec<_>>()
            });
            if let Some(records) = records.filter(|v| !v.is_empty()) {
                let header = std::str::from_utf8(
                    records[0].split(|b| *b == b'\t').next().unwrap_or_default(),
                )
                .unwrap_or("");
                let parts = header.split_whitespace().collect::<Vec<_>>();
                let regular = records.len() == 1
                    && parts.len() == 3
                    && matches!(parts[0], "100644" | "100755")
                    && parts[2] == "0"
                    && matches!(parts[1].len(), 40 | 64)
                    && parts[1].bytes().all(|c| c.is_ascii_hexdigit());
                let exact = regular
                    && git(&["cat-file", "-s", parts[1]]).is_ok_and(|o| {
                        o.status.success()
                            && std::str::from_utf8(&o.stdout)
                                .ok()
                                .and_then(|s| s.trim().parse::<u64>().ok())
                                .is_some_and(|n| n <= 64 * 1024 * 1024)
                    })
                    && git(&["cat-file", "blob", parts[1]]).is_ok_and(|o| {
                        o.status.success() && hashes.len() == 1 && hashes.contains(&hash(&o.stdout))
                    });
                if exact { "selected" } else { "index_differs" }
            } else if git(&["check-ignore", "--quiet", "--", &path])
                .is_ok_and(|o| o.status.success())
            {
                "ignored"
            } else {
                "untracked"
            }
        } else {
            "unknown"
        };
        if composition != "selected" {
            r.finding("unknown", "DeliveryNotSelected", &path, composition);
        }
        entries.push(json!({"path":path,"current":current,"composition":composition}));
    }
    r.measurements.push(json!({"action":"spec-delivery","id":id,"composition":if selected.is_some(){"bundle"}else if git_available{"git_index"}else{"unknown"},"files":entries,"delivery_ready":r.status=="passed","technical_readiness":"use_spec_check","human_acceptance":"unchanged"}));
    r.limitations.push("Проверены локальные файлы и выбранный состав; внешняя доступность, срок хранения, отправка и приёмка не подтверждаются. Bundle — JSON-массив относительных путей, не загрузка. Все закреплённые входы проверяются консервативно; воспроизводимость не угадывается. Байты blob в Git index должны совпасть с закреплёнными хешами; фильтры не запускаются. Различие нормализации остаётся явным index_differs.".into());
    Ok(r)
}
