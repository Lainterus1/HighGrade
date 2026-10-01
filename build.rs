use std::{env, fs, path::Path};
#[path = "src/ui/bundle.rs"]
mod bundle;
fn collect(root: &Path, dir: &Path, entries: &mut Vec<(String, String, Vec<u8>, String)>) {
    for item in fs::read_dir(dir).expect("UI assets directory") {
        let item = item.unwrap();
        let p = item.path();
        let kind = item.file_type().unwrap();
        assert!(!kind.is_symlink(), "UI assets must not be symlinks");
        if kind.is_dir() {
            collect(root, &p, entries)
        } else if kind.is_file() {
            let route = format!(
                "/{}",
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            );
            let mime = match p.extension().and_then(|v| v.to_str()).unwrap_or("") {
                "html" => "text/html; charset=utf-8",
                "js" => "text/javascript; charset=utf-8",
                "css" => "text/css; charset=utf-8",
                "svg" => "image/svg+xml",
                "ttf" => "font/ttf",
                _ => "application/octet-stream",
            };
            entries.push((
                route,
                mime.into(),
                fs::read(&p).unwrap(),
                p.canonicalize().unwrap().to_string_lossy().into_owned(),
            ));
        }
    }
}
fn main() {
    println!("cargo:rerun-if-changed=ui/dist");
    println!("cargo:rerun-if-changed=src/ui/bundle.rs");
    println!("cargo:rerun-if-env-changed=HIGHGRADE_SOURCE_SHA");
    let root = Path::new("ui/dist");
    let mut entries = Vec::new();
    if root.exists() {
        assert!(
            !fs::symlink_metadata(root).unwrap().file_type().is_symlink(),
            "UI assets must not be symlinks"
        );
        collect(root, root, &mut entries);
        entries.sort();
        let assets: Vec<_> = entries
            .iter()
            .map(|(route, mime, bytes, _)| (route.as_str(), mime.as_str(), bytes.as_slice()))
            .collect();
        let manifest = bundle::validate(&assets, bundle::API_VERSION)
            .unwrap_or_else(|error| panic!("{error}"));
        if let Ok(sha) = env::var("HIGHGRADE_SOURCE_SHA") {
            assert_eq!(
                manifest.source_sha.as_deref(),
                Some(sha.as_str()),
                "UiBundleInvalid: source revision does not match the build"
            );
        }
    } else if env::var("PROFILE").as_deref() == Ok("release") {
        panic!(
            "UiBundleInvalid: release requires ui/dist; run npm ci && npm run build in ui first"
        );
    } else {
        println!(
            "cargo:warning=CLI-only development build: UI and global installation require a complete ui/dist"
        );
    }
    let generated: Vec<_> = entries
        .iter()
        .map(|(route, mime, _, path)| format!("({route:?},{mime:?},include_bytes!({path:?}))"))
        .collect();
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("ui_assets.rs"),
        format!(
            "static UI_ASSETS:&[(&str,&str,&[u8])]=&[{}];",
            generated.join(",")
        ),
    )
    .unwrap();
}
