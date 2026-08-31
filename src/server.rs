//! HTTP 适配层（P0011，feature `server`）：六操作 RESTish 加 JSON 信封加网页直出。
//! 信封形（S016 吸收）：`{ok, data|error, meta:{command, project}}`。
//! 状态码约定：请求体解析失败 400；编排操作的业务失败走 200 加 `ok:false`
//! （信封承载语义，传输层只管传输）。只绑 127.0.0.1（本机工具，无鉴权）；
//! 写操作经会话锁串行化（一次一命令，P0011 风险节）。中断 serve 不清会话：
//! 会话跨命令可重连是设计，收尾走 DELETE /session。
//! web-mirror-server（用户定调命名）：`/share-fe` 托管源码构建的 rmux web-share
//! 前端，可视化后台 agent 任务；`oma web`/`POST /share` 起镜像链接。
//! `GET /` 即 web 镜像页（share-fe 资产目录托管，原配置 dashboard 已删——
//! 编排操作回归 CLI/API/MCP，网页只做可视化）。`GET /` 原 d `docs\web\index.html`（include_str 单文件，无构建链）；
//! `GET /stream/{agent}?from=oldest|now` 把 pane 输出桥成 SSE（P0011 切片 2）。

use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Path as AxPath, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use rmux_sdk::PaneOutputStart;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;

use crate::api;
use crate::orch;

/// rmux share 前端本地自托管（P0022：docs/web/share-src 源码构建，产物在
/// docs/web/share-fe/，serve 运行时读盘——rebuild 前端不用重编 oma）。
/// 目录锚编译期定（serve 可从任意 cwd 起）。
const SHARE_FE_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/web/share-fe");

pub struct ServeState {
    root: PathBuf,
    /// 会话写串行化：spawn/send/run/settle/cleanup 一次一命令。
    gate: Mutex<()>,
}

/// 起编排面：绑定后打 banner，永不主动退出（Ctrl-C 结束进程，会话留存）。
pub async fn serve(root: PathBuf, port: u16) -> Result<(), String> {
    let project = root.display().to_string();
    let state = Arc::new(ServeState {
        root,
        gate: Mutex::new(()),
    });
    let app = router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))?;
    let local = listener.local_addr().map_err(|e| e.to_string())?;
    println!("serve.addr=http://{local}");
    println!("serve.project={project}");
    println!("serve.ok=true");
    axum::serve(listener, app)
        .await
        .map_err(|e| format!("serve: {e}"))
}

/// REPL 内嵌形态（P0016）：后台跑编排面，返回实际地址；
/// 任务挂在当前 runtime，REPL 主循环的每个 await 都给它让路。
pub async fn serve_in_background(root: PathBuf, port: u16) -> Result<SocketAddr, String> {
    let state = Arc::new(ServeState {
        root,
        gate: Mutex::new(()),
    });
    let app = router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("bind {addr}: {e}"))?;
    let local = listener.local_addr().map_err(|e| e.to_string())?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("oma: web server stopped: {e}");
        }
    });
    Ok(local)
}

fn router(state: Arc<ServeState>) -> axum::Router {
    axum::Router::new()
        // 主页即 web 镜像页（用户定调：可视化页面当首页，编排走 CLI/API/MCP）。
        // GET / 自动起整会话镜像并 302 到 #t= ——打开就是多路窗格。
        .route("/", get(home))
        .route("/share-fe", get(share_fe_root))
        .route("/share-fe/", get(share_fe_root))
        .route("/share-fe/{*path}", get(share_fe_asset))
        // 前端 JS 以绝对路径 /_astro/... 引 wasm（SRI 锁字节），根路径原位托管。
        .route("/_astro/{*path}", get(share_fe_astro_asset))
        .route("/api", get(index))
        .route("/spawn", post(spawn))
        .route("/status", get(status))
        .route("/send", post(send))
        .route("/run", post(run))
        .route("/settle", post(settle))
        .route("/session", delete(cleanup))
        .route("/stream/{agent}", get(stream))
        .route("/screen/{agent}", get(screen))
        .route("/share", post(share_session))
        .route("/share/{agent}", post(share_agent))
        .route("/share", get(share_list))
        .route("/share/{id}/stop", delete(share_stop))
        .route("/trace/sessions", get(trace_sessions))
        .route("/trace/timeline", get(trace_timeline))
        .route("/trace/search", get(trace_search))
        .with_state(state)
}

/// SSE 终端镜像（P0019）：`render_stream` 是 daemon 侧 surface 投影（非视觉
/// 输出在 daemon 已过滤），每次更新发全屏 `visible_lines`（JSON 数组）——
/// 网页替换渲染，TUI 画面无 ANSI 转义。`/stream` 的行日志仍保留（append 面）。
async fn screen(
    State(st): State<Arc<ServeState>>,
    AxPath(agent): AxPath<String>,
) -> Response {
    let command = "screen";
    let link = match orch::connect(&st.root, false).await {
        Ok(l) => l,
        Err(e) => return err_reply(command, &st.root, StatusCode::OK, e),
    };
    let (pane_id, pane) = match orch::pane_for_agent(&link, &st.root, &agent).await {
        Ok(v) => v,
        Err(e) => return err_reply(command, &st.root, StatusCode::OK, e),
    };
    let mut render = match pane.render_stream().await {
        Ok(s) => s,
        Err(e) => return err_reply(command, &st.root, StatusCode::OK, format!("open render stream: {e}")),
    };
    // render_stream 只在变化时推送（静屏连接后一直空白）：先补一帧当前快照。
    let first = pane
        .snapshot()
        .await
        .map(|s| s.visible_lines())
        .unwrap_or_default();
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(4);
    tokio::spawn(async move {
        let open = Event::default().event("open").data(pane_id.to_string());
        if tx.send(Ok(open)).await.is_err() {
            return;
        }
        if tx
            .send(Ok(Event::default().data(serde_json::to_string(&first).unwrap_or_default())))
            .await
            .is_err()
        {
            return;
        }
        loop {
            match render.next().await {
                Ok(Some(update)) => {
                    let lines = update.snapshot().visible_lines();
                    if tx
                        .send(Ok(Event::default().data(serde_json::to_string(&lines).unwrap_or_default())))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Ok(None) => {
                    let _ = tx
                        .send(Ok(Event::default().event("end").data("closed")))
                        .await;
                    break;
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(Event::default().event("error").data(e.to_string())))
                        .await;
                    break;
                }
            }
        }
    });
    Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

async fn trace_sessions(State(st): State<Arc<ServeState>>) -> Response {
    ok_reply("trace.sessions", &st.root, api::trace_sessions(&st.root))
}

async fn trace_timeline(
    State(st): State<Arc<ServeState>>,
    Query(q): Query<TraceQ>,
) -> Response {
    let limit = q
        .limit
        .unwrap_or(crate::trace::DEFAULT_LIMIT)
        .clamp(1, crate::trace::MAX_LIMIT);
    ok_reply(
        "trace.timeline",
        &st.root,
        api::trace_timeline(&st.root, q.agent.as_deref(), q.file.as_deref(), limit),
    )
}

async fn trace_search(
    State(st): State<Arc<ServeState>>,
    Query(q): Query<TraceSearchQ>,
) -> Response {
    let limit = q
        .limit
        .unwrap_or(crate::trace::DEFAULT_LIMIT)
        .clamp(1, crate::trace::MAX_LIMIT);
    ok_reply(
        "trace.search",
        &st.root,
        api::trace_search(&st.root, &q.q, q.agent.as_deref(), limit),
    )
}

#[derive(Deserialize)]
struct SpawnReq {
    agents: Option<Vec<String>>,
    stub: Option<bool>,
}

#[derive(Deserialize)]
struct SendReq {
    agent: String,
    text: String,
    confirm: Option<String>,
}

#[derive(Deserialize)]
struct RunReq {
    text: String,
    assign: Option<Vec<String>>,
    confirm: Option<String>,
}

#[derive(Deserialize)]
struct SettleReq {
    wait: Option<u64>,
}

/// 前端静态目录托管：读盘 + 扩展名 MIME + ACAO（crossorigin=anonymous 资源
/// 的 CORS 校验）+ no-store（SRI 与资产演进同步）。路径规范化防穿越。
fn share_fe_read(rel: &str) -> Option<( &'static str, Vec<u8>)> {
    use std::path::Component;
    let base = std::path::Path::new(SHARE_FE_DIR);
    let mut full = base.to_path_buf();
    for c in std::path::Path::new(rel).components() {
        match c {
            Component::Normal(p) => full.push(p),
            _ => return None, // 拒绝 ..、绝对段等一切越界形态
        }
    }
    let data = std::fs::read(&full).ok()?;
    let mime = match full.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "application/javascript",
        Some("css") => "text/css",
        Some("wasm") => "application/wasm",
        Some("json") | Some("webmanifest") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    Some((mime, data))
}

fn share_fe_reply(rel: &str) -> Response {
    match share_fe_read(rel) {
        Some((mime, body)) => (
            [
                (axum::http::header::CONTENT_TYPE, mime),
                (axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
                (axum::http::header::CACHE_CONTROL, "no-store"),
            ],
            body,
        )
            .into_response(),
        None => err_reply("share-fe", Path::new(SHARE_FE_DIR), StatusCode::NOT_FOUND, format!("{rel} not found")),
    }
}

/// 打开即四路窗格：清本项目旧 share、起整会话 operator 镜像（本地免 PIN），
/// 200 直出前端 HTML 并注入 hash-shim（无 hash 时 replace 到 `/#t=`，一次自
/// 载后前端按 hash 连接——302 方案会对 `/` 自旋）。
async fn home(
    State(st): State<Arc<ServeState>>,
    headers: axum::http::header::HeaderMap,
) -> Response {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1:7900")
        .to_string();
    let fe = format!("http://{host}/");
    // 清本项目全部旧 share（pane 形态 target 是 `<session>:%N`，前缀即会话名）。
    if let Ok(list) = api::web_shares(&st.root).await {
        if let Ok(name) = orch::session_name(&st.root) {
            for row in list["shares"].as_array().cloned().unwrap_or_default() {
                if row["target"]
                    .as_str()
                    .is_some_and(|t| t.starts_with(name.as_str()))
                {
                    if let Some(id) = row["id"].as_str() {
                        let _ = api::web_share_stop(&st.root, id).await;
                    }
                }
            }
        }
    }
    let mirror = api::web_share(&st.root, None, false, 43200, Some(&fe), true).await;
    let token = mirror
        .as_ref()
        .ok()
        .and_then(|v| {
            v["url"]
                .as_str()
                .and_then(|u| u.split("#t=").nth(1))
                .map(String::from)
        });
    let Some(token) = token else {
        let msg = mirror
            .err()
            .unwrap_or_else(|| "mirror url missing token".to_string());
        return err_reply("home", &st.root, StatusCode::OK, msg);
    };
    let mut html = match share_fe_read("index.html") {
        Some((_, b)) => String::from_utf8_lossy(&b).into_owned(),
        None => return err_reply("home", &st.root, StatusCode::NOT_FOUND, "share-fe/index.html".into()),
    };
    let shim = format!(
        "<script>if(!location.hash)location.replace('/#t={token}');</script></body>"
    );
    html = html.replacen("</body>", &shim, 1);
    (
        [
            (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        html,
    )
        .into_response()
}

async fn share_fe_root() -> Response {
    share_fe_reply("index.html")
}

async fn share_fe_asset(AxPath(path): AxPath<String>) -> Response {
    share_fe_reply(&path)
}

async fn share_fe_astro_asset(AxPath(path): AxPath<String>) -> Response {
    share_fe_reply(&format!("_astro/{path}"))
}


/// 起一路官方 web 镜像（rmux web-share，P0021）：operator 可操作真 attach，
/// spectator 只看；URL 与 PIN 在 stderr，api 层已合并解析。
async fn share_agent(
    State(st): State<Arc<ServeState>>,
    AxPath(agent): AxPath<String>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let command = "share";
    #[derive(Deserialize)]
    struct ShareReq {
        spectator: Option<bool>,
        ttl: Option<u64>,
        /// 缺省本地托管形态免 PIN；显式 "on" 恢复带 PIN。
        pin: Option<String>,
    }
    let req: ShareReq = if body.trim().is_empty() {
        ShareReq { spectator: None, ttl: None, pin: None }
    } else {
        match parse_body(&body, command, &st.root) {
            Ok(r) => r,
            Err(r) => return r,
        }
    };
    // serve 在跑就默认本地前端（P0021）：share.rmux.io 只是静态资产，能自托管。
    let fe = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|host| format!("http://{host}/"));
    // 前端本地托管即本机场景：免 PIN 直连（req 显式给 pin 才加回）。
    let no_pin = match req.pin.as_deref() {
        None => fe.is_some(),
        Some("on") => false,
        Some(_) => true,
    };
    finish(
        command,
        &st.root,
        api::web_share(&st.root, Some(&agent), req.spectator.unwrap_or(false), req.ttl.unwrap_or(3600), fe.as_deref(), no_pin).await,
    )
}

/// 整会话镜像（P0021 默认形态）：一个 URL 全窗格、operator 可编辑、带分屏控制。
async fn share_session(
    State(st): State<Arc<ServeState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Response {
    let command = "share";
    #[derive(Deserialize)]
    struct ShareReq {
        spectator: Option<bool>,
        ttl: Option<u64>,
        pin: Option<String>,
    }
    let req: ShareReq = if body.trim().is_empty() {
        ShareReq { spectator: None, ttl: None, pin: None }
    } else {
        match parse_body(&body, command, &st.root) {
            Ok(r) => r,
            Err(r) => return r,
        }
    };
    let fe = headers
        .get(axum::http::header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|host| format!("http://{host}/"));
    let no_pin = fe.is_some() && req.pin.as_deref() != Some("on");
    finish(
        command,
        &st.root,
        api::web_share(&st.root, None, req.spectator.unwrap_or(false), req.ttl.unwrap_or(3600), fe.as_deref(), no_pin).await,
    )
}

async fn share_list(State(st): State<Arc<ServeState>>) -> Response {
    finish("shares", &st.root, api::web_shares(&st.root).await)
}

async fn share_stop(State(st): State<Arc<ServeState>>, AxPath(id): AxPath<String>) -> Response {
    let _guard = st.gate.lock().await;
    finish("share.stop", &st.root, api::web_share_stop(&st.root, &id).await)
}

async fn index(State(st): State<Arc<ServeState>>) -> Response {
    let data = json!({
        "name": "oma",
        "page": "/",
        "tui": "/tui",
        "endpoints": [
            {"method": "GET", "path": "/api"},
            {"method": "POST", "path": "/share/{agent}", "body": {"spectator": false, "ttl": 3600}},
            {"method": "GET", "path": "/share"},
            {"method": "DELETE", "path": "/share/{id}/stop"},
            {"method": "POST", "path": "/spawn", "body": {"agents": ["claude"], "stub": false}},
            {"method": "GET", "path": "/status"},
            {"method": "POST", "path": "/send", "body": {"agent": "claude", "text": "..."}},
            {"method": "POST", "path": "/run", "body": {"text": "...", "assign": ["claude"]}},
            {"method": "POST", "path": "/settle", "body": {"wait": 30}},
            {"method": "DELETE", "path": "/session"},
            {"method": "GET", "path": "/stream/{agent}?from=oldest|now"},
            {"method": "GET", "path": "/screen/{agent}"},
            {"method": "GET", "path": "/trace/sessions"},
            {"method": "GET", "path": "/trace/timeline?agent=&file=&limit="},
            {"method": "GET", "path": "/trace/search?q=&agent=&limit="},
        ],
    });
    ok_reply("index", &st.root, data)
}

async fn spawn(State(st): State<Arc<ServeState>>, body: String) -> Response {
    let command = "spawn";
    let req: SpawnReq = match parse_body(&body, command, &st.root) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let _guard = st.gate.lock().await;
    finish(command, &st.root, api::spawn(&st.root, req.agents, req.stub.unwrap_or(false)).await)
}

async fn status(State(st): State<Arc<ServeState>>) -> Response {
    // 只读：不进会话锁，可与写操作并发。
    finish("status", &st.root, api::status(&st.root).await)
}

async fn send(State(st): State<Arc<ServeState>>, body: String) -> Response {
    let command = "send";
    let req: SendReq = match parse_body(&body, command, &st.root) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let _guard = st.gate.lock().await;
    finish(
        command,
        &st.root,
        api::send(&st.root, &req.agent, &req.text, req.confirm.as_deref()).await,
    )
}

async fn run(State(st): State<Arc<ServeState>>, body: String) -> Response {
    let command = "run";
    let req: RunReq = match parse_body(&body, command, &st.root) {
        Ok(r) => r,
        Err(r) => return r,
    };
    let _guard = st.gate.lock().await;
    finish(
        command,
        &st.root,
        api::run(&st.root, &req.text, req.assign, req.confirm.as_deref()).await,
    )
}

async fn settle(State(st): State<Arc<ServeState>>, body: String) -> Response {
    let command = "settle";
    // 空体也接受：等价 {"wait":30}。
    let req: SettleReq = if body.trim().is_empty() {
        SettleReq { wait: None }
    } else {
        match parse_body(&body, command, &st.root) {
            Ok(r) => r,
            Err(r) => return r,
        }
    };
    let _guard = st.gate.lock().await;
    finish(
        command,
        &st.root,
        api::settle(&st.root, req.wait.unwrap_or(30)).await,
    )
}

async fn cleanup(State(st): State<Arc<ServeState>>) -> Response {
    let _guard = st.gate.lock().await;
    finish("cleanup", &st.root, api::cleanup(&st.root).await)
}

#[derive(Deserialize)]
struct StreamQ {
    /// `oldest` 回放留存积压；缺省（`now`）只看新字节。
    from: Option<String>,
}

#[derive(Deserialize)]
struct TraceQ {
    agent: Option<String>,
    file: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct TraceSearchQ {
    q: String,
    agent: Option<String>,
    limit: Option<usize>,
}

/// SSE 画面：pane 的**渲染行**桥成 `data:` 事件（P0019：line_stream 替原始
/// 字节——真 agent TUI 的 ANSI 转义在 daemon 侧已渲染掉，网页不再出转义汤；
/// lossy UTF-8 加按 LF 切行由 SDK 保证）。`open` 事件带 pane_id，`end`/`error`
/// 收尾。拉取任务随接收端断开（tx send 失败）自然终止，流 drop 时自退订。
async fn stream(
    State(st): State<Arc<ServeState>>,
    AxPath(agent): AxPath<String>,
    Query(q): Query<StreamQ>,
) -> Response {
    let command = "stream";
    let link = match orch::connect(&st.root, false).await {
        Ok(l) => l,
        Err(e) => return err_reply(command, &st.root, StatusCode::OK, e),
    };
    let (pane_id, pane) = match orch::pane_for_agent(&link, &st.root, &agent).await {
        Ok(v) => v,
        Err(e) => return err_reply(command, &st.root, StatusCode::OK, e),
    };
    let start = if q.from.as_deref() == Some("oldest") {
        rmux_sdk::PaneOutputStart::Oldest
    } else {
        rmux_sdk::PaneOutputStart::Now
    };
    let mut out = match pane.line_stream_starting_at(start).await {
        Ok(s) => s,
        Err(e) => return err_reply(command, &st.root, StatusCode::OK, format!("open stream: {e}")),
    };
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(16);
    tokio::spawn(async move {
        let open = Event::default().event("open").data(pane_id.to_string());
        if tx.send(Ok(open)).await.is_err() {
            return;
        }
        loop {
            match out.next().await {
                Ok(Some(rmux_sdk::PaneLineItem::Line { text })) => {
                    if tx.send(Ok(Event::default().data(text))).await.is_err() {
                        break;
                    }
                }
                Ok(Some(_)) => continue, // Lag 通知不携带行
                Ok(None) => {
                    let _ = tx
                        .send(Ok(Event::default().event("end").data("closed")))
                        .await;
                    break;
                }
                Err(e) => {
                    let _ = tx
                        .send(Ok(Event::default().event("error").data(e.to_string())))
                        .await;
                    break;
                }
            }
        }
    });
    Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default())
        .into_response()
}

fn parse_body<T: for<'de> Deserialize<'de>>(
    body: &str,
    command: &str,
    root: &Path,
) -> Result<T, Response> {
    serde_json::from_str(body)
        .map_err(|e| err_reply(command, root, StatusCode::BAD_REQUEST, format!("bad json body: {e}")))
}

fn finish(command: &str, root: &Path, outcome: Result<Value, String>) -> Response {
    match outcome {
        Ok(data) => ok_reply(command, root, data),
        Err(e) => err_reply(command, root, StatusCode::OK, e),
    }
}

fn ok_reply(command: &str, root: &Path, data: Value) -> Response {
    (StatusCode::OK, Json(api::envelope(command, root, Ok(data)))).into_response()
}

fn err_reply(command: &str, root: &Path, code: StatusCode, msg: String) -> Response {
    (code, Json(api::envelope(command, root, Err(msg)))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(r"D:\ohmyagents")
    }

    #[test]
    fn envelope_ok_carries_data_and_meta() {
        let v = api::envelope("spawn", &root(), Ok(json!({"agents": ["claude"]})));
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["agents"][0], "claude");
        assert_eq!(v["meta"]["command"], "spawn");
        assert_eq!(v["meta"]["project"], r"D:\ohmyagents");
        assert!(v.get("error").is_none());
    }

    #[test]
    fn envelope_error_replaces_data() {
        let v = api::envelope("send", &root(), Err("no manifest".into()));
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"], "no manifest");
        assert!(v.get("data").is_none());
        assert_eq!(v["meta"]["command"], "send");
    }

    #[test]
    fn settle_accepts_empty_body_semantics() {
        // parse_body 对空串报 400；空体分支在 handler 里先拦（等价 wait=30）。
        let r: Result<SettleReq, Response> = parse_body("", "settle", &root());
        assert!(r.is_err());
    }
}
