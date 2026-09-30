mod support;
use highgrade::{global, hash};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::TestDir;

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_highgrade"))
}
fn source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("kit")
}
fn inventory(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().into(),
                    hash(&fs::read(path).unwrap()),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

// highgrade: HG-0061-S3
#[test]
fn ui_check_is_rootless_read_only_and_rejects_conflicting_options() {
    let root = TestDir::new("hg-ui-check-");
    fs::create_dir(root.join("specs")).unwrap();
    fs::write(
        root.join("specs/catalog.json"),
        b"invalid project must not be read",
    )
    .unwrap();
    let before = inventory(&root);
    let output = Command::new(exe())
        .args(["ui", "--check", "true"])
        .current_dir(&root)
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["operation"], "ui-check");
    assert_eq!(report["status"], "passed");
    assert_eq!(
        report["measurements"][0]["cli_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(report["measurements"][0]["api_version"], "2");
    assert!(report["measurements"][0]["files"].as_u64().unwrap() > 0);
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ui_url"));
    for args in [
        vec!["ui", "--check", "false"],
        vec!["ui", "--check", "true", "--root", "missing"],
        vec!["ui", "--check", "true", "--no-open", "true"],
    ] {
        let output = Command::new(exe())
            .args(args)
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
    assert_eq!(inventory(&root), before);
}

// highgrade: HG-0061-S3
#[test]
fn incompatible_ui_candidate_is_rejected_before_install_or_update_writes() {
    let damaged = TestDir::new("hg-ui-damaged-");
    let executable = damaged.join(format!("highgrade{}", std::env::consts::EXE_SUFFIX));
    fs::copy(exe(), &executable).unwrap();
    let mut bytes = fs::read(&executable).unwrap();
    let needle = b"\"api_version\": \"2\"";
    let positions: Vec<_> = bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, window)| (window == needle).then_some(index))
        .collect();
    assert_eq!(positions.len(), 1);
    bytes[positions[0] + needle.len() - 2] = b'9';
    fs::write(&executable, bytes).unwrap();
    let version = Command::new(&executable).arg("--version").output().unwrap();
    assert!(version.status.success());
    let output = Command::new(&executable)
        .args(["ui", "--check", "true"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("UiBundleInvalid"));
    let profile = TestDir::new("hg-ui-install-");
    assert!(
        global::install(&profile, &source(), &executable)
            .unwrap_err()
            .contains("UiBundleInvalid")
    );
    assert!(fs::read_dir(&*profile).unwrap().next().is_none());
    global::install(&profile, &source(), &exe()).unwrap();
    let before = inventory(&profile);
    for apply in [false, true] {
        assert!(
            global::update(
                &profile,
                Some(&source()),
                Some(&executable),
                apply,
                apply.then_some("not-a-preview-fingerprint")
            )
            .unwrap_err()
            .contains("UiBundleInvalid")
        );
        assert_eq!(inventory(&profile), before);
    }
}
