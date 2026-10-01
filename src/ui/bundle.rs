//! Validate the complete embedded bundle before opening a listener or any project.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub const API_VERSION: &str = "2";
type Asset<'a> = (&'a str, &'a str, &'a [u8]);
#[derive(Debug, Deserialize)]
pub struct Manifest {
    schema_version: u32,
    pub api_version: String,
    pub cli_version: String,
    pub source_sha: Option<String>,
    pub files: BTreeMap<String, String>,
}
pub fn validate(assets: &[Asset<'_>], version: &str) -> Result<Manifest, String> {
    let fail = || {
        "UiBundleInvalid: комплект UI отсутствует, повреждён или несовместим; пересоберите npm ci && npm run build в ui, затем CLI, либо переустановите полный выпуск".to_string()
    };
    let bytes = assets
        .iter()
        .find(|a| a.0 == "/highgrade-ui.json")
        .ok_or_else(fail)?
        .2;
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| fail())?;
    if manifest.schema_version != 1
        || manifest.api_version != version
        || manifest.cli_version != env!("CARGO_PKG_VERSION")
        || manifest.files.len() + 1 != assets.len()
        || [
            "index.html",
            "fonts/Manrope.ttf",
            "brand/mark.svg",
            "brand/wordmark.svg",
            "licenses/Manrope-OFL.txt",
            "licenses/Lucide-LICENSE.txt",
            "licenses/packages.json",
        ]
        .iter()
        .any(|file| !manifest.files.contains_key(*file))
        || [".js", ".css"].iter().any(|ext| {
            !manifest
                .files
                .keys()
                .any(|file| file.starts_with("assets/") && file.ends_with(ext))
        })
    {
        return Err(fail());
    }
    for (file, expected) in &manifest.files {
        if file
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
            || file.contains('\\')
        {
            return Err(fail());
        }
        let route = format!("/{file}");
        let asset = assets.iter().find(|a| a.0 == route).ok_or_else(fail)?;
        if asset.2.is_empty() || format!("{:x}", Sha256::digest(asset.2)) != *expected {
            return Err(fail());
        }
    }
    Ok(manifest)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_bundle_rejects_missing_changed_and_incompatible_resources() {
        let assets = super::super::UI_ASSETS;
        validate(assets, super::super::API_VERSION).unwrap();
        assert!(
            validate(&[], super::super::API_VERSION)
                .unwrap_err()
                .starts_with("UiBundleInvalid:")
        );
        assert!(validate(&assets[1..], super::super::API_VERSION).is_err());
        assert!(validate(assets, "99").is_err());
        let mut changed = assets.to_vec();
        let entry = changed.iter_mut().find(|a| a.0 == "/index.html").unwrap();
        entry.2 = b"corrupt";
        assert!(validate(&changed, super::super::API_VERSION).is_err());
    }

    // highgrade: HG-0061-S3
    #[test]
    fn incomplete_bundle_is_rejected_even_with_consistent_manifest_hashes() {
        let original = super::super::UI_ASSETS;
        for omitted in [
            "fonts/Manrope.ttf",
            "brand/wordmark.svg",
            "licenses/packages.json",
            ".js",
            ".css",
        ] {
            let mut manifest: serde_json::Value = serde_json::from_slice(
                original
                    .iter()
                    .find(|a| a.0 == "/highgrade-ui.json")
                    .unwrap()
                    .2,
            )
            .unwrap();
            let file = manifest["files"]
                .as_object()
                .unwrap()
                .keys()
                .find(|file| file.ends_with(omitted))
                .unwrap()
                .clone();
            manifest["files"].as_object_mut().unwrap().remove(&file);
            let bytes = serde_json::to_vec(&manifest).unwrap();
            let mut assets: Vec<_> = original
                .iter()
                .copied()
                .filter(|a| a.0 != format!("/{file}"))
                .collect();
            assets
                .iter_mut()
                .find(|a| a.0 == "/highgrade-ui.json")
                .unwrap()
                .2 = &bytes;
            assert!(validate(&assets, API_VERSION).is_err(), "missing {file}");
        }
    }
}
