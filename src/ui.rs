//! One loopback process, one immutable project root. No shell or arbitrary paths in the API.
use crate::{Report, Result, paths, specs};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path as RoutePath, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::{net::TcpListener, sync::Notify};

mod bundle;
const API_VERSION: &str = bundle::API_VERSION;
include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));
#[derive(Clone)]
struct App {
    root: PathBuf,
    host: String,
    token: String,
    style_nonce: String,
    stop: Arc<Notify>,
}
fn error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({"error":{"code":code,"message":message}})),
    )
        .into_response()
}
async fn boundary(State(app): State<App>, req: Request, next: Next) -> Response {
    let headers = req.headers();
    let header = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());
    let origin = format!("http://{}", app.host);
    if header("host") != Some(app.host.as_str())
        || header("origin").is_some_and(|v| v != origin)
        || header("sec-fetch-site").is_some_and(|v| !matches!(v, "same-origin" | "none"))
    {
        return error(
            StatusCode::FORBIDDEN,
            "OriginRejected",
            "Требуется локальная страница HighGrade",
        );
    }
    if req.method() != axum::http::Method::GET
        && (header("origin") != Some(origin.as_str())
            || header("x-highgrade-token") != Some(app.token.as_str()))
    {
        return error(
            StatusCode::FORBIDDEN,
            "SessionRejected",
            "Сеанс устарел; обновите страницу",
        );
    }
    if req.uri().path().starts_with("/api/")
        && req.uri().path() != "/api/session"
        && header("x-highgrade-api") != Some(API_VERSION)
    {
        return error(
            StatusCode::CONFLICT,
            "ApiVersionMismatch",
            "Версии страницы и сервера различаются; обновите страницу",
        );
    }
    let mut response = next.run(req).await;
    for (name, value) in [
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_static(value),
        );
    }
    let policy = format!(
        "default-src 'self'; script-src 'self'; style-src 'self' 'nonce-{}'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        app.style_nonce
    );
    response.headers_mut().insert(
        "content-security-policy",
        HeaderValue::from_str(&policy).unwrap(),
    );
    response
}
fn catalog(root: &Path) -> Result<()> {
    let (store, sha) = specs::load(root)?;
    if sha == "absent" {
        return Err("CatalogMissing: каталог спецификаций отсутствует".into());
    }
    if store.schema_version < 3 {
        return Err("CatalogUnsupported: требуется каталог v3/v4".into());
    }
    Ok(())
}
fn failure(message: String) -> Response {
    let code = message.split(':').next().unwrap_or("CatalogError");
    let status = if code.contains("Conflict") || code.contains("Stale") {
        StatusCode::CONFLICT
    } else if code == "CatalogMissing" || code == "ChangeMissing" {
        StatusCode::NOT_FOUND
    } else {
        StatusCode::UNPROCESSABLE_ENTITY
    };
    error(status, code, &message)
}
async fn blocking(f: impl FnOnce() -> Result<Report> + Send + 'static) -> Response {
    match tokio::task::spawn_blocking(f).await {
        Ok(Ok(report)) => {
            let status = if report.exit_code() == 0 {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            (status, Json(report)).into_response()
        }
        Ok(Err(e)) => failure(e),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "WorkerFailed",
            "Операция не подтверждена; перечитайте состояние",
        ),
    }
}
async fn session(State(app): State<App>) -> Json<Value> {
    Json(
        json!({"api_version":API_VERSION,"token":app.token,"project":{"name":app.root.file_name().unwrap_or_default().to_string_lossy(),"root":app.root.to_string_lossy()}}),
    )
}
async fn list(State(app): State<App>) -> Response {
    blocking(move || specs::ui_list(&app.root)).await
}
async fn read(State(app): State<App>, RoutePath(id): RoutePath<String>) -> Response {
    blocking(move || specs::ui_read(&app.root, &id)).await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteInput {
    expected: Option<String>,
    expected_local: Option<String>,
    input: Value,
}
async fn write(
    State(app): State<App>,
    RoutePath((id, operation)): RoutePath<(String, String)>,
    body: std::result::Result<Json<WriteInput>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let body = match body {
        Ok(Json(v)) => v,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "InvalidRequest",
                "Ожидается JSON с expected и input",
            );
        }
    };
    let op = match operation.as_str() {
        "edit" => "spec-edit",
        "attention" => "spec-attention",
        _ => {
            return error(
                StatusCode::NOT_FOUND,
                "OperationNotAllowed",
                "Операция недоступна",
            );
        }
    };
    let cas = match (body.expected, body.expected_local) {
        (Some(value), None) => ("--expected", value),
        (None, Some(value)) if op == "spec-edit" => ("--expected-local", value),
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "InvalidCAS",
                "Укажите ровно один снимок: expected либо expected_local для правки",
            );
        }
    };
    blocking(move || {
        catalog(&app.root)?;
        let mut options = BTreeMap::from([("--id".into(), id), (cas.0.into(), cas.1)]);
        if op == "spec-edit" {
            options.insert("--validate".into(), "true".into());
        }
        specs::command_input(&app.root, op, &options, body.input)
    })
    .await
}
async fn shutdown(State(app): State<App>) -> Json<Value> {
    app.stop.notify_one();
    Json(json!({"stopping":true}))
}
async fn index(State(app): State<App>) -> Response {
    if let Some((_, mime, bytes)) = UI_ASSETS.iter().find(|(p, _, _)| *p == "/index.html") {
        let html = String::from_utf8_lossy(bytes).replacen(
            "<head>",
            &format!(
                "<head><meta name=\"highgrade-style-nonce\" content=\"{}\">",
                app.style_nonce
            ),
            1,
        );
        return ([(axum::http::header::CONTENT_TYPE, *mime)], html).into_response();
    }
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "UiBundleInvalid",
        "Переустановите полный выпуск",
    )
}
async fn asset(RoutePath(path): RoutePath<String>) -> Response {
    match UI_ASSETS.iter().find(|(p, _, _)| *p == format!("/{path}")) {
        Some((_, mime, bytes)) => {
            ([(axum::http::header::CONTENT_TYPE, *mime)], *bytes).into_response()
        }
        None => error(StatusCode::NOT_FOUND, "ResourceMissing", "Ресурс не найден"),
    }
}
fn router(app: App) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/session", get(session))
        .route("/api/specs", get(list))
        .route("/api/specs/{id}", get(read))
        .route("/api/specs/{id}/{operation}", post(write))
        .route("/api/shutdown", post(shutdown))
        .route("/{*path}", get(asset))
        .fallback(|| async {
            error(StatusCode::NOT_FOUND, "RouteMissing", "Адрес не найден")
        })
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(middleware::from_fn_with_state(app.clone(), boundary))
        .with_state(app)
}
pub fn check() -> Result<Report> {
    let manifest = bundle::validate(UI_ASSETS, API_VERSION)?;
    let mut report = Report::new("ui-check");
    report.measurements.push(json!({
        "cli_version": manifest.cli_version,
        "api_version": manifest.api_version,
        "source_sha": manifest.source_sha,
        "files": manifest.files.len(),
        "launcher_protocol": crate::launcher::PROTOCOL,
    }));
    Ok(report)
}
pub fn run(root: &Path, no_open: bool) -> Result<Report> {
    bundle::validate(UI_ASSETS, API_VERSION)?;
    let root = paths::root(root)?;
    tokio::runtime::Runtime::new().map_err(|e|e.to_string())?.block_on(async move {
        let listener=TcpListener::bind("127.0.0.1:0").await.map_err(|e|e.to_string())?;
        let host=listener.local_addr().map_err(|e|e.to_string())?.to_string();
        let url=format!("http://{host}");
        let mut bytes=[0u8;32];getrandom::fill(&mut bytes).map_err(|e|e.to_string())?;
        let token=bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();
        getrandom::fill(&mut bytes).map_err(|e|e.to_string())?;
        let style_nonce=bytes.iter().map(|b|format!("{b:02x}")).collect::<String>();
        let stop=Arc::new(Notify::new());
        let app=App {root:root.clone(),host,token,style_nonce,stop:stop.clone()};
        println!("{}",json!({"ui_url":url,"root":root,"stop":"Ctrl+C или POST /api/shutdown из текущего сеанса"}));
        std::io::stdout().flush().map_err(|e|e.to_string())?;
        if !no_open {
            #[cfg(windows)] let opened=std::process::Command::new("explorer.exe").arg(&url).spawn();
            #[cfg(target_os="macos")] let opened=std::process::Command::new("open").arg(&url).spawn();
            #[cfg(all(unix,not(target_os="macos")))] let opened=std::process::Command::new("xdg-open").arg(&url).spawn();
            if let Err(e)=opened {eprintln!("Браузер не открыт: {e}. Откройте {url}");}
        }
        axum::serve(listener,router(app)).with_graceful_shutdown(async move {
            tokio::select! {_=tokio::signal::ctrl_c()=>{},_=stop.notified()=>{}}
        }).await.map_err(|e|e.to_string())?;
        Ok(Report::new("ui"))
    })
}
