pub mod bootstrap;
pub mod global;
pub mod inspect;
pub mod install;
pub mod package;
pub mod paths;
pub mod specs;
pub mod trace;

use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

pub type Result<T> = std::result::Result<T, String>;

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn read_json(path: &Path) -> Result<Value> {
    let data = paths::read_limited(path, 8 * 1024 * 1024)?;
    serde_json::from_slice(&data).map_err(|e| format!("ConfigInvalid: {}: {e}", path.display()))
}

#[derive(Serialize, Default, Debug)]
pub struct Report {
    pub operation: String,
    pub status: String,
    pub findings: Vec<Value>,
    pub measurements: Vec<Value>,
    pub limitations: Vec<String>,
}
impl Report {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.into(),
            status: "passed".into(),
            ..Self::default()
        }
    }
    pub fn finding(&mut self, status: &str, code: &str, location: &str, message: &str) {
        self.findings
            .push(json!({"status":status,"code":code,"location":location,"message":message}));
        let rank = |s: &str| match s {
            "failed" => 3,
            "unknown" => 2,
            "warning" => 1,
            _ => 0,
        };
        if rank(status) > rank(&self.status) {
            self.status = status.into();
        }
    }
    pub fn exit_code(&self) -> i32 {
        match self.status.as_str() {
            "failed" => 1,
            "unknown" => 2,
            _ => 0,
        }
    }
}

pub fn doctor(root: &Path) -> Result<Report> {
    let root = paths::root(root)?;
    let mut r = Report::new("doctor");
    r.measurements.push(
        json!({"highgrade_version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS}),
    );
    let native_specs = paths::safe(&root, &specs::catalog_path(&root)?)?.exists()
        || paths::safe(&root, specs::STORE)?.exists()
        || paths::safe(&root, ".highgrade/project/INSTRUCTIONS.md")
            .ok()
            .and_then(|p| paths::read_limited(&p, 1024 * 1024).ok())
            .is_some_and(|b| {
                String::from_utf8_lossy(&b)
                    .lines()
                    .any(|l| l.trim() == "highgrade_spec_format: native-v1")
            });
    if native_specs {
        match specs::load(&root) {
            Ok((store, sha)) => r.measurements.push(json!({"spec_format":"native-v1","store_schema_version":store.schema_version,"store_sha256":sha,"semantic_readiness":"not_assessed"})),
            Err(e) => r.finding("failed", "NativeSpecsInvalid", specs::STORE, &e),
        }
    }
    for name in ["cargo", "codex", "node", "openspec"] {
        let found = std::env::var_os("PATH")
            .into_iter()
            .flat_map(|s| std::env::split_paths(&s).collect::<Vec<_>>())
            .filter(|p| p.is_absolute())
            .any(|dir| {
                let extensions: &[&str] = if cfg!(windows) {
                    &["exe", "cmd", "bat"]
                } else {
                    &[""]
                };
                extensions.iter().any(|ext| {
                    dir.join(if ext.is_empty() {
                        name.to_owned()
                    } else {
                        format!("{name}.{ext}")
                    })
                    .is_file()
                })
            });
        r.measurements
            .push(json!({"tool":name,"found_in_path":found,"version":null,"executed":false,"required_by_legacy_probe":!native_specs}));
        if !found && !native_specs {
            r.finding(
                "unknown",
                "DependencyMissing",
                name,
                "Не найден в PATH; наличие установки и версия не проверены.",
            );
        }
    }
    if global::project_check(&root, &mut r)? {
        return Ok(r);
    }
    let active = paths::safe(&root, ".highgrade/active.json")?;
    if active.exists() {
        match package::active(&root) {
            Ok(Some((release, _))) => r.measurements.push(json!({"connection":"connected","release":release,"workflow_ready":"requires adaptation acceptance"})),
            Ok(None) => r.finding("failed","InstallationInvalid",".highgrade","Активный указатель исчез."),
            Err(e) => r.finding("failed","InstallationInvalid",".highgrade",&e),
        }
        return Ok(r);
    }
    let journal = paths::safe(&root, ".highgrade/install.json")?;
    let complete = paths::safe(&root, ".highgrade/installed.json")?;
    if journal.exists() {
        match install::verify(&root) {
            Ok(()) => r
                .measurements
                .push(json!({"connection":"prototype-connected","workflow_ready":false})),
            Err(e) => r.finding("failed", "InstallationInvalid", ".highgrade", &e),
        }
    } else if complete.exists() {
        r.finding(
            "failed",
            "InstallationInvalid",
            ".highgrade",
            "Маркер без журнала.",
        );
    } else {
        r.finding(
            "unknown",
            "NotConnected",
            ".highgrade",
            "Основа не подключена; отдельные диагностические команды доступны.",
        );
    }
    r.limitations.push("Обнаружение команды в PATH не подтверждает её версию или работоспособность; применимые проверки выполняются отдельно.".into());
    Ok(r)
}
