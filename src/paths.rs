use crate::Result;
use std::{
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

fn reject_reparse(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) => {
            #[cfg(windows)]
            let reparse = {
                use std::os::windows::fs::MetadataExt;
                m.file_attributes() & 0x400 != 0
            };
            #[cfg(not(windows))]
            let reparse = false;
            if m.file_type().is_symlink() || reparse {
                return Err(format!(
                    "UnsafePath: {}: symlink/reparse point",
                    path.display()
                ));
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("PathUnreadable: {}: {e}", path.display())),
    }
    Ok(())
}
pub fn root(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
    let mut cursor = PathBuf::new();
    for part in absolute.components() {
        cursor.push(part);
        if matches!(part, Component::Prefix(_)) {
            continue;
        }
        reject_reparse(&cursor)?;
    }
    let result = absolute
        .canonicalize()
        .map_err(|e| format!("RootInvalid: {e}"))?;
    if !result.is_dir() {
        return Err("RootInvalid: not a directory".into());
    }
    Ok(result)
}
pub fn relative(rel: &str) -> Result<()> {
    if rel.is_empty() || rel.contains(['\\', ':', '\0']) || Path::new(rel).is_absolute() {
        return Err(format!("UnsafePath: {rel}"));
    }
    for part in rel.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.contains(['<', '>', '|', '?', '*'])
            || [
                "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
                "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
                "LPT9",
            ]
            .contains(&stem.as_str())
            || !Path::new(part)
                .components()
                .all(|c| matches!(c, Component::Normal(_)))
        {
            return Err(format!("UnsafePath: {rel}"));
        }
    }
    Ok(())
}
pub fn safe(root: &Path, rel: &str) -> Result<PathBuf> {
    relative(rel)?;
    let mut cursor = root.to_path_buf();
    reject_reparse(&cursor)?;
    for part in rel.split('/') {
        cursor.push(part);
        reject_reparse(&cursor)?;
    }
    Ok(cursor)
}
pub fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>> {
    let f = File::open(path).map_err(|e| format!("InputUnreadable: {}: {e}", path.display()))?;
    if !f.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("InputInvalid: not a regular file".into());
    }
    let mut data = vec![];
    f.take(max + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > max {
        return Err(format!("InputTooLarge: {}: limit={max}", path.display()));
    }
    Ok(data)
}
