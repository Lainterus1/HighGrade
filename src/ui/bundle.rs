//! Validate the complete embedded bundle before opening a listener or any project.
use crate::{Result, hash};
use serde::Deserialize;
use std::collections::BTreeMap;
type Asset<'a> = (&'a str, &'a str, &'a [u8]);
#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    api_version: String,
    cli_version: String,
    files: BTreeMap<String, String>,
}
pub(super) fn validate(assets: &[Asset<'_>], version: &str) -> Result<()> {
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
        || !manifest.files.contains_key("index.html")
        || !manifest.files.contains_key("licenses/Manrope-OFL.txt")
        || !manifest.files.contains_key("licenses/Lucide-LICENSE.txt")
    {
        return Err(fail());
    }
    for (file, expected) in manifest.files {
        let route = format!("/{file}");
        let asset = assets.iter().find(|a| a.0 == route).ok_or_else(fail)?;
        if hash(asset.2) != expected {
            return Err(fail());
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn embedded_bundle_rejects_missing_changed_and_incompatible_resources() {
        let assets = super::super::UI_ASSETS;
        validate(assets, "1").unwrap();
        assert!(
            validate(&[], "1")
                .unwrap_err()
                .starts_with("UiBundleInvalid:")
        );
        assert!(validate(&assets[1..], "1").is_err());
        assert!(validate(assets, "99").is_err());
        let mut changed = assets.to_vec();
        let entry = changed.iter_mut().find(|a| a.0 == "/index.html").unwrap();
        entry.2 = b"corrupt";
        assert!(validate(&changed, "1").is_err());
    }
}
