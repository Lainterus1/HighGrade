mod support;
use std::{fs, process::Command};
use support::TestDir;
const EXE: &str = env!("CARGO_BIN_EXE_highgrade");
// highgrade: HG-0068-S1, HG-0068-S2
#[test]
fn launcher_preparation_is_explicit_idempotent_and_preserves_foreign_files() {
    let root = TestDir::new("hg-launchers-");
    let run = |apply: bool| {
        let mut cmd = Command::new(EXE);
        cmd.args(["ui-launcher", "--root"]).arg(&*root);
        if apply {
            cmd.args(["--apply", "true"]);
        }
        cmd.output().unwrap()
    };
    let preview = run(false);
    assert!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stdout)
    );
    assert!(!root.join("HighGrade UI.sh").exists());
    assert!(run(true).status.success());
    let before = fs::read(root.join("HighGrade UI.sh")).unwrap();
    assert!(!before.contains(&b'\r'));
    let batch = fs::read(root.join("HighGrade UI.cmd")).unwrap();
    assert!(batch.windows(2).any(|pair| pair == b"\r\n"));
    assert!(!batch.windows(3).any(|triple| triple == b"\r\r\n"));
    assert!(run(true).status.success());
    assert_eq!(before, fs::read(root.join("HighGrade UI.sh")).unwrap());
    fs::remove_file(root.join("HighGrade UI.sh")).unwrap();
    fs::write(root.join("HighGrade UI.cmd"), b"user file").unwrap();
    let conflict = run(true);
    assert!(!conflict.status.success());
    assert!(!root.join("HighGrade UI.sh").exists());
    assert_eq!(
        fs::read(root.join("HighGrade UI.cmd")).unwrap(),
        b"user file"
    );
    let doctor = Command::new(EXE)
        .args(["doctor", "--root"])
        .arg(&*root)
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&doctor.stdout).contains("UiLauncherModified"));
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let e = entry.unwrap();
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_tree(&e.path(), &target)
        } else {
            fs::copy(e.path(), target).unwrap();
        }
    }
}
fn candidate_at(path: &std::path::Path, release: &str) {
    copy_tree(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("kit"),
        path,
    );
    let p = path.join("manifest.json");
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&p).unwrap()).unwrap();
    v["release"] = release.into();
    fs::write(p, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}
fn launch(project: &std::path::Path, profile: &std::path::Path) {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::Stdio;
    #[cfg(unix)]
    let mut cmd = Command::new(project.join("HighGrade UI.sh"));
    #[cfg(windows)]
    let mut cmd = {
        let mut c = Command::new("cmd.exe");
        c.args(["/D", "/C"]).arg(project.join("HighGrade UI.cmd"));
        c
    };
    let mut child = cmd
        .arg("--no-open")
        .env("HOME", profile)
        .env("USERPROFILE", profile)
        .current_dir(profile)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.as_mut().unwrap())
        .read_line(&mut line)
        .unwrap();
    if line.is_empty() {
        let output = child.wait_with_output().unwrap();
        panic!(
            "launcher failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
    let startup: serde_json::Value = serde_json::from_str(&line).unwrap();
    let host = startup["ui_url"]
        .as_str()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    let request = |method: &str, path: &str, extra: &str| {
        let mut stream = std::net::TcpStream::connect(host).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        write!(stream,"{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: 0\r\n{extra}\r\n").unwrap();
        let mut s = String::new();
        stream.read_to_string(&mut s).unwrap();
        s
    };
    let session = request("GET", "/api/session", "");
    let session: serde_json::Value =
        serde_json::from_str(session.split_once("\r\n\r\n").unwrap().1).unwrap();
    let shutdown = request(
        "POST",
        "/api/shutdown",
        &format!(
            "Origin: http://{host}\r\nX-HighGrade-Api: 2\r\nX-HighGrade-Token: {}\r\n",
            session["token"].as_str().unwrap()
        ),
    );
    assert!(shutdown.starts_with("HTTP/1.1 200"));
    assert!(child.wait().unwrap().success());
    assert_eq!(
        session["project"]["root"],
        highgrade::paths::root(project)
            .unwrap()
            .to_string_lossy()
            .as_ref()
    );
}
fn fault_candidate(area: &std::path::Path, legacy: bool) -> std::path::PathBuf {
    let name = if legacy {
        "legacy-shim"
    } else {
        "rollback-shim"
    };
    let source = area.join(format!("{name}.rs"));
    let executable = area.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    let code = r#"
use std::{env, fs, process::{Command, exit}};
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("ui") && args.get(1).map(String::as_str) == Some("--check") {
        if LEGACY {
            println!("{}", LEGACY_REPORT);
            return;
        }
        let exe = env::current_exe().unwrap();
        let release = exe.parent().unwrap();
        if release.file_name().unwrap() == "launcher-broken" {
            let active = release.parent().unwrap().parent().unwrap().join("active.json");
            if fs::read_to_string(active).unwrap().contains("launcher-broken") { exit(1); }
        }
    }
    exit(Command::new(REAL_EXE).args(args).status().unwrap().code().unwrap_or(1));
}
"#;
    let legacy_report = serde_json::json!({"operation":"ui-check","status":"passed","measurements":[{"cli_version":env!("CARGO_PKG_VERSION"),"api_version":"2","files":1}]}).to_string();
    fs::write(&source, format!("const REAL_EXE: &str = {EXE:?};\nconst LEGACY: bool = {legacy};\nconst LEGACY_REPORT: &str = {legacy_report:?};\n{code}")).unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "launcher_fault"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}
// highgrade: HG-0068-S1, HG-0068-S3
#[test]
fn same_project_launchers_survive_move_and_global_update() {
    use highgrade::global;
    let area = TestDir::new("hg-launch-update-");
    let profile = area.join("Профиль пользователя");
    let project = area.join("Проект & ! % ' UI");
    fs::create_dir_all(&profile).unwrap();
    fs::create_dir_all(&project).unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("kit");
    let exe = std::path::Path::new(EXE);
    global::install(&profile, &source, exe).unwrap();
    highgrade::launcher::prepare(&project, true).unwrap();
    let bytes = fs::read(project.join("HighGrade UI.sh")).unwrap();
    let cmd_bytes = fs::read(project.join("HighGrade UI.cmd")).unwrap();
    launch(&project, &profile);
    let moved = area.join("Перенесённый проект & ! % ' UI");
    fs::rename(&project, &moved).unwrap();
    launch(&moved, &profile);
    let next = area.join("candidate");
    candidate_at(&next, "launcher-next");
    let preview = global::update(&profile, Some(&next), Some(exe), false, None).unwrap();
    global::update(
        &profile,
        Some(&next),
        Some(exe),
        true,
        preview.measurements[0]["candidate_sha256"].as_str(),
    )
    .unwrap();
    launch(&moved, &profile);
    assert_eq!(
        global::status(&profile).unwrap().measurements[0]["release"],
        "launcher-next"
    );
    assert_eq!(fs::read(moved.join("HighGrade UI.sh")).unwrap(), bytes);
    assert_eq!(fs::read(moved.join("HighGrade UI.cmd")).unwrap(), cmd_bytes);
    {
        // Real executable fixtures also exercise candidate validation and rollback on Windows.
        let shim = fault_candidate(&area, true);
        let legacy = area.join("legacy-candidate");
        candidate_at(&legacy, "launcher-legacy");
        let error = global::update(&profile, Some(&legacy), Some(&shim), false, None).unwrap_err();
        assert!(error.contains("CandidateUiBundleInvalid"), "{error}");
        assert_eq!(
            global::status(&profile).unwrap().measurements[0]["release"],
            "launcher-next"
        );
        let shim = fault_candidate(&area, false);
        let bad = area.join("bad-candidate");
        candidate_at(&bad, "launcher-broken");
        let p = global::update(&profile, Some(&bad), Some(&shim), false, None).unwrap();
        let error = global::update(
            &profile,
            Some(&bad),
            Some(&shim),
            true,
            p.measurements[0]["candidate_sha256"].as_str(),
        )
        .unwrap_err();
        assert!(
            error.contains("GlobalPostcheckFailedRestoredPrevious"),
            "{error}"
        );
        assert_eq!(
            global::status(&profile).unwrap().measurements[0]["release"],
            "launcher-next"
        );
        launch(&moved, &profile);
        assert_eq!(fs::read(moved.join("HighGrade UI.sh")).unwrap(), bytes);
        assert_eq!(fs::read(moved.join("HighGrade UI.cmd")).unwrap(), cmd_bytes);
    }
}

#[cfg(unix)]
// highgrade: HG-0068-S2
#[test]
fn launcher_rejects_links_and_reports_missing_installation() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = TestDir::new("hg-launch-safe-");
    let profile = TestDir::new("hg-launch-empty-profile-");
    fs::write(root.join("foreign"), b"keep").unwrap();
    symlink("foreign", root.join("HighGrade UI.sh")).unwrap();
    assert!(highgrade::launcher::prepare(&root, true).is_err());
    assert!(!root.join("HighGrade UI.cmd").exists());
    assert_eq!(fs::read(root.join("foreign")).unwrap(), b"keep");
    fs::remove_file(root.join("HighGrade UI.sh")).unwrap();
    highgrade::launcher::prepare(&root, true).unwrap();
    let shell = root.join("HighGrade UI.sh");
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o600)).unwrap();
    highgrade::launcher::prepare(&root, true).unwrap();
    assert_ne!(
        fs::metadata(&shell).unwrap().permissions().mode() & 0o100,
        0
    );
    let output = Command::new(shell)
        .env("HOME", &*profile)
        .arg("--no-open")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not installed"));
}
