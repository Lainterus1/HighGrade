//! Portable project launchers resolve the existing global active pointer each time.
use crate::{Report, Result, install, paths};
use serde_json::json;
use std::{fs, path::Path};
pub const PROTOCOL: u32 = 1;
fn files() -> [(&'static str, Vec<u8>); 2] {
    [
        ("HighGrade UI.sh", include_bytes!("launcher/ui.sh").to_vec()),
        (
            "HighGrade UI.cmd",
            include_str!("launcher/ui.cmd")
                .replace('\n', "\r\n")
                .into_bytes(),
        ),
    ]
}
pub fn prepare(root: &Path, apply: bool) -> Result<Report> {
    let root = paths::root(root)?;
    let files = files();
    let mut report = Report::new("ui-launcher");
    // Preflight every destination before creating any missing file.
    for (name, bytes) in &files {
        let path = paths::safe(&root, name)?;
        if path.exists() && paths::read_limited(&path, 1024 * 1024)? != *bytes {
            return Err(format!("UiLauncherConflict: {name}; user file preserved"));
        }
    }
    for (name, bytes) in files {
        let path = paths::safe(&root, name)?;
        let missing = !path.exists();
        if apply {
            install::put_once(&root, name, &bytes)?;
            #[cfg(unix)]
            if name.ends_with(".sh") {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = fs::metadata(&path)
                    .map_err(|e| e.to_string())?
                    .permissions();
                permissions.set_mode(permissions.mode() | 0o100);
                fs::set_permissions(&path, permissions).map_err(|e| e.to_string())?;
            }
        }
        report.measurements.push(
            json!({"path":name,"missing_before":missing,"applied":apply,"protocol":PROTOCOL}),
        );
    }
    Ok(report)
}
pub fn diagnose(root: &Path, report: &mut Report) -> Result<()> {
    let files = files();
    if !paths::safe(root, ".highgrade/project/INSTRUCTIONS.md")?.exists()
        && !files
            .iter()
            .any(|(name, _)| root.join(name).symlink_metadata().is_ok())
    {
        return Ok(());
    }
    for (name, bytes) in files {
        let path = match paths::safe(root, name) {
            Ok(p) => p,
            Err(e) => {
                report.finding("warning", "UiLauncherUnsafe", name, &e);
                continue;
            }
        };
        if !path.exists() {
            report.finding(
                "warning",
                "UiLauncherMissing",
                name,
                "Run ui-launcher --root PROJECT --apply true to create missing launchers.",
            );
        } else if paths::read_limited(&path, 1024 * 1024)? != bytes {
            report.finding(
                "warning",
                "UiLauncherModified",
                name,
                "File differs; preserve it and review before replacement.",
            );
        } else if name.ends_with(".sh") && install::verify_executable(&path).is_err() {
            report.finding(
                "warning",
                "UiLauncherNotExecutable",
                name,
                "Run ui-launcher --root PROJECT --apply true to restore its executable permission.",
            );
        }
    }
    Ok(())
}
