mod support;
use highgrade::specs;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};
use support::TestDir;
struct Server {
    child: Child,
    host: String,
    token: String,
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Server {
    fn start(root: &Path, explicit: bool) -> Self {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_highgrade"));
        cmd.arg("ui")
            .args(["--no-open", "true"])
            .current_dir(root)
            .env("PATH", "")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if explicit {
            cmd.arg("--root").arg(root);
        }
        let mut child = cmd.spawn().unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.as_mut().unwrap())
            .read_line(&mut line)
            .unwrap();
        let info: Value = serde_json::from_str(&line).expect("startup JSON without token");
        assert!(info.get("token").is_none());
        let host = info["ui_url"]
            .as_str()
            .unwrap()
            .strip_prefix("http://")
            .unwrap()
            .to_string();
        assert!(host.starts_with("127.0.0.1:"));
        let mut s = Self {
            child,
            host,
            token: String::new(),
        };
        let (status, body) = s.http("GET", "/api/session", None, &[]);
        assert_eq!(status, 200);
        s.token = serde_json::from_str::<Value>(&body).unwrap()["token"]
            .as_str()
            .unwrap()
            .into();
        s
    }
    fn http(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        headers: &[(&str, &str)],
    ) -> (u16, String) {
        let mut stream = TcpStream::connect(&self.host).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let body = body.map(|v| v.to_string()).unwrap_or_default();
        let host = headers
            .iter()
            .find(|(k, _)| *k == "Host")
            .map(|(_, v)| *v)
            .unwrap_or(&self.host);
        let api_version = headers
            .iter()
            .find(|(k, _)| *k == "X-HighGrade-Api")
            .map(|(_, v)| *v)
            .unwrap_or("2");
        write!(stream,"{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\nX-HighGrade-Api: {api_version}\r\n",body.len()).unwrap();
        for (k, v) in headers {
            if *k != "Host" && *k != "X-HighGrade-Api" {
                write!(stream, "{k}: {v}\r\n").unwrap();
            }
        }
        write!(stream, "\r\n{body}").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
        let body = response.split_once("\r\n\r\n").unwrap().1.to_string();
        (status, body)
    }
    fn post(&self, path: &str, body: Value) -> (u16, String) {
        self.http(
            "POST",
            path,
            Some(body),
            &[
                ("Origin", &format!("http://{}", self.host)),
                ("X-HighGrade-Token", &self.token),
            ],
        )
    }
    fn stop(&mut self) {
        assert_eq!(self.post("/api/shutdown", json!({})).0, 200);
        assert!(self.child.wait().unwrap().success());
        assert!(TcpStream::connect(&self.host).is_err());
    }
}
fn call(root: &Path, op: &str, args: &[(&str, &str)]) -> highgrade::Report {
    specs::command(
        root,
        op,
        &args
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<BTreeMap<_, _>>(),
    )
    .unwrap()
}
fn sha(root: &Path) -> String {
    specs::load(root).unwrap().1
}
fn create(root: &Path) {
    call(
        root,
        "spec-new",
        &[("--title", "Спека"), ("--expected", &sha(root))],
    );
    fs::write(root.join("fixture.json"),serde_json::to_vec(&json!({"goal":"Цель","rationale":"Причина","scope":"Область","tasks":[{"id":"HG-0001-T1","description":"Проверить","done":false}],"operations":[{"action":"add","requirement":{"id":"HG-0001-R1","title":"Условие","statement":"Исходный текст","scenarios":[{"id":"HG-0001-S1","given":"Дано","when":"Действие","then":"Результат","verification":"Сверка"}]}}]})).unwrap()).unwrap();
    call(
        root,
        "spec-edit",
        &[
            ("--id", "HG-0001"),
            ("--expected", &sha(root)),
            ("--input", "fixture.json"),
        ],
    );
}
// highgrade: HG-0053-S1, HG-0053-S2, HG-0053-S4, HG-0053-S6, HG-0053-S7, HG-0059-S2
#[test]
fn isolated_servers_share_cli_cas_and_run_without_node() {
    let a = TestDir::new("hg-http-a-");
    let b = TestDir::new("hg-http-b-");
    fs::create_dir(a.join("same")).unwrap();
    fs::create_dir(b.join("same")).unwrap();
    let a = a.join("same");
    let b = b.join("same");
    create(&a);
    create(&b);
    let mut one = Server::start(&a, false);
    let mut two = Server::start(&b, true);
    assert_ne!(one.host, two.host);
    for (s, p) in [(&one, &a), (&two, &b)] {
        let (_, body) = s.http("GET", "/api/session", None, &[]);
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["api_version"], "2");
        assert_eq!(
            Path::new(v["project"]["root"].as_str().unwrap()),
            highgrade::paths::root(p).unwrap()
        );
        assert_eq!(s.http("GET", "/", None, &[]).0, 200);
        let (status, body) = s.http("GET", "/highgrade-ui.json", None, &[]);
        assert_eq!(status, 200);
        let bundle: Value = serde_json::from_str(&body).unwrap();
        let script = bundle["files"]
            .as_object()
            .unwrap()
            .keys()
            .find(|p| p.ends_with(".js"))
            .unwrap();
        assert_eq!(s.http("GET", &format!("/{script}"), None, &[]).0, 200);
    }
    let before_version_check = sha(&a);
    assert_eq!(
        one.http("GET", "/api/specs", None, &[("X-HighGrade-Api", "1")])
            .0,
        409
    );
    assert_eq!(sha(&a), before_version_check);
    let before_b = sha(&b);
    let old = sha(&a);
    let (status, body) = one.post(
        "/api/specs/HG-0001/edit",
        json!({"expected":old,"input":{"goal":"HTTP edit <script>"}}),
    );
    assert_eq!(status, 200);
    let report: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["result"]["change"]["goal"], "HTTP edit <script>");
    assert_eq!(sha(&b), before_b);
    let old = sha(&a);
    fs::write(a.join("patch.json"), br#"{"goal":"CLI edit"}"#).unwrap();
    call(
        &a,
        "spec-edit",
        &[
            ("--id", "HG-0001"),
            ("--expected", &old),
            ("--input", "patch.json"),
        ],
    );
    let before = sha(&a);
    assert_eq!(
        one.post(
            "/api/specs/HG-0001/edit",
            json!({"expected":old,"input":{"goal":"stale"}})
        )
        .0,
        409
    );
    assert_eq!(sha(&a), before);
    // Incomplete request is disconnected before its body; it must not mutate anything.
    let mut stream = TcpStream::connect(&one.host).unwrap();
    write!(stream,"POST /api/specs/HG-0001/edit HTTP/1.1\r\nHost: {}\r\nOrigin: http://{}\r\nX-HighGrade-Api: 2\r\nX-HighGrade-Token: {}\r\nContent-Type: application/json\r\nContent-Length: 500\r\n\r\n{{",one.host,one.host,one.token).unwrap();
    drop(stream);
    one.stop();
    assert_eq!(sha(&a), before);
    two.stop();
}
// highgrade: HG-0053-S3
#[test]
fn hostile_origins_tokens_paths_and_extra_fields_never_mutate() {
    let p = TestDir::new("hg-http-security-");
    create(&p);
    fs::write(p.join("outside.txt"), "control").unwrap();
    let mut s = Server::start(&p, true);
    let before = sha(&p);
    assert_eq!(
        s.http("GET", "/api/session", None, &[("Host", "attacker.example")])
            .0,
        403
    );
    assert_eq!(
        s.http(
            "GET",
            "/api/session",
            None,
            &[("Origin", "https://attacker.example")]
        )
        .0,
        403
    );
    assert_eq!(
        s.http(
            "GET",
            "/api/session",
            None,
            &[("Sec-Fetch-Site", "cross-site")]
        )
        .0,
        403
    );
    let body = json!({"expected":before,"input":{"goal":"bad"}});
    assert_eq!(
        s.http(
            "POST",
            "/api/specs/HG-0001/edit",
            Some(body.clone()),
            &[("Origin", &format!("http://{}", s.host))]
        )
        .0,
        403
    );
    assert_eq!(
        s.http(
            "POST",
            "/api/specs/HG-0001/edit",
            Some(body),
            &[
                ("Origin", "https://attacker.example"),
                ("X-HighGrade-Token", &s.token)
            ]
        )
        .0,
        403
    );
    assert_ne!(
        s.post(
            "/api/specs/..%2Foutside.txt/edit",
            json!({"expected":before,"input":{"goal":"bad"}})
        )
        .0,
        200
    );
    assert_ne!(
        s.post(
            "/api/specs/HG-0001/run",
            json!({"expected":before,"input":{}})
        )
        .0,
        200
    );
    assert_ne!(
        s.post(
            "/api/specs/HG-0001/edit",
            json!({"expected":before,"input":{},"root":".."})
        )
        .0,
        200
    );
    assert_eq!(sha(&p), before);
    assert_eq!(
        fs::read_to_string(p.join("outside.txt")).unwrap(),
        "control"
    );
    s.stop();
}
// highgrade: HG-0053-S5
#[test]
fn missing_empty_corrupt_and_interrupted_catalogs_remain_distinct() {
    let p = TestDir::new("hg-http-states-");
    let mut s = Server::start(&p, true);
    let (status, body) = s.http("GET", "/api/specs", None, &[]);
    assert_eq!(status, 404);
    assert!(body.contains("CatalogMissing"));
    assert!(!p.join("specs").exists());
    call(&p, "spec-init", &[]);
    let (status, body) = s.http("GET", "/api/specs", None, &[]);
    assert_eq!(status, 200);
    let v: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["result"]["changes"], json!([]));
    fs::write(p.join("specs/transaction.json"), "{}").unwrap();
    let (status, body) = s.http("GET", "/api/specs", None, &[]);
    assert_eq!(status, 422);
    assert!(body.contains("RecoveryRequired"));
    assert!(p.join("specs/transaction.json").exists());
    fs::remove_file(p.join("specs/transaction.json")).unwrap();
    fs::write(p.join("specs/catalog.json"), "bad json").unwrap();
    let (status, body) = s.http("GET", "/api/specs", None, &[]);
    assert_eq!(status, 422);
    assert!(body.contains("InvalidFormat"));
    assert_eq!(
        fs::read_to_string(p.join("specs/catalog.json")).unwrap(),
        "bad json"
    );
    s.stop();
}

// highgrade: HG-0053-S3, HG-0053-S5
#[test]
fn unsupported_record_cannot_be_read_or_edited_by_direct_url() {
    let p = TestDir::new("hg-http-unsupported-");
    create(&p);
    let path = p.join("specs/changes/HG-0001/spec.json");
    let mut v: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    v.as_object_mut().unwrap().remove("scoped_baseline");
    fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
    let before = sha(&p);
    let mut s = Server::start(&p, true);
    assert_eq!(s.http("GET", "/api/specs/HG-0001", None, &[]).0, 422);
    assert_eq!(
        s.post(
            "/api/specs/HG-0001/edit",
            json!({"expected":before,"input":{"goal":"blocked"}})
        )
        .0,
        422
    );
    let (_, body) = s.http("GET", "/api/specs", None, &[]);
    let report: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["result"]["excluded_count"], 1);
    assert_eq!(sha(&p), before);
    s.stop();
}
// highgrade: HG-0053-S7
#[test]
fn server_shutdown_during_write_request_never_reports_false_success() {
    let p = TestDir::new("hg-http-shutdown-");
    create(&p);
    let before = sha(&p);
    let mut s = Server::start(&p, true);
    let mut stream = TcpStream::connect(&s.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(stream,"POST /api/specs/HG-0001/edit HTTP/1.1\r\nHost: {}\r\nOrigin: http://{}\r\nX-HighGrade-Token: {}\r\nX-HighGrade-Api: 2\r\nExpect: 100-continue\r\nContent-Type: application/json\r\nContent-Length: 500\r\n\r\n",s.host,s.host,s.token).unwrap();
    let mut interim = [0u8; 25];
    stream.read_exact(&mut interim).unwrap();
    assert_eq!(&interim, b"HTTP/1.1 100 Continue\r\n\r\n");
    stream.write_all(b"{\"expected\":").unwrap();
    s.child.kill().unwrap();
    s.child.wait().unwrap();
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    assert!(!response.contains("200 OK"));
    assert_eq!(sha(&p), before);
    // Graceful shutdown drains an already accepted request before process exit.
    let mut s = Server::start(&p, true);
    let body = json!({"expected":before,"input":{"goal":"drained"}}).to_string();
    let mut stream = TcpStream::connect(&s.host).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(stream,"POST /api/specs/HG-0001/edit HTTP/1.1\r\nHost: {}\r\nOrigin: http://{}\r\nX-HighGrade-Token: {}\r\nX-HighGrade-Api: 2\r\nExpect: 100-continue\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",s.host,s.host,s.token,body.len()).unwrap();
    stream.read_exact(&mut interim).unwrap();
    assert_eq!(s.post("/api/shutdown", json!({})).0, 200);
    stream.write_all(body.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(s.child.wait().unwrap().success());
    assert_eq!(
        specs::load(&p).unwrap().0.changes["HG-0001"].goal,
        "drained"
    );
}
