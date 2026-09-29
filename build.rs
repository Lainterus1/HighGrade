use std::{env, fs, path::Path};
fn collect(root: &Path, dir: &Path, entries: &mut Vec<String>) {
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
            entries.push(format!(
                "({route:?},{mime:?},include_bytes!({:?}))",
                p.canonicalize().unwrap().to_string_lossy()
            ));
        }
    }
}
fn main() {
    println!("cargo:rerun-if-changed=ui/dist");
    let root = Path::new("ui/dist");
    let mut entries = Vec::new();
    if root.join("index.html").is_file() {
        collect(root, root, &mut entries);
        entries.sort()
    }
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("ui_assets.rs"),
        format!(
            "static UI_ASSETS:&[(&str,&str,&[u8])]=&[{}];",
            entries.join(",")
        ),
    )
    .unwrap();
}
