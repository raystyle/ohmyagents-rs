//! HTTP 适配层（P0011 切片 1，feature `server`）：六操作 RESTish 加 JSON 信封。
//! 信封形（S016 吸收）：`{ok, data|error, meta:{command, project}}`。
//! 状态码约定：请求体解析失败 400；编排操作的业务失败走 200 加 `ok:false`
//! （信封承载语义，传输层只管传输）。只绑 127.0.0.1（本机工具，无鉴权）；
//! 写操作经会话锁串行化（一次一命令，P0011 风险节）。中断 serve 不清会话：
//! 会话跨命令可重连是设计，收尾走 DELETE /session。

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::api;

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
    let app = axum::Router::new()
        .route("/", get(index))
        .route("/spawn", post(spawn))
        .route("/status", get(status))
        .route("/send", post(send))
        .route("/run", post(run))
        .route("/settle", post(settle))
        .route("/session", delete(cleanup))
        .with_state(state);
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

async fn index(State(st): State<Arc<ServeState>>) -> Response {
    let data = json!({
        "name": "oma",
        "endpoints": [
            {"method": "POST", "path": "/spawn", "body": {"agents": ["claude"], "stub": false}},
            {"method": "GET", "path": "/status"},
            {"method": "POST", "path": "/send", "body": {"agent": "claude", "text": "..."}},
            {"method": "POST", "path": "/run", "body": {"text": "...", "assign": ["claude"]}},
            {"method": "POST", "path": "/settle", "body": {"wait": 30}},
            {"method": "DELETE", "path": "/session"},
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
    (StatusCode::OK, Json(envelope(command, root, Some(data), None))).into_response()
}

fn err_reply(command: &str, root: &Path, code: StatusCode, msg: String) -> Response {
    (code, Json(envelope(command, root, None, Some(msg)))).into_response()
}

fn envelope(command: &str, root: &Path, data: Option<Value>, error: Option<String>) -> Value {
    let mut v = json!({
        "ok": error.is_none(),
        "meta": { "command": command, "project": root.display().to_string() },
    });
    if let Some(d) = data {
        v["data"] = d;
    }
    if let Some(e) = error {
        v["error"] = Value::String(e);
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(r"D:\ohmyagents")
    }

    #[test]
    fn envelope_ok_carries_data_and_meta() {
        let v = envelope("spawn", &root(), Some(json!({"agents": ["claude"]})), None);
        assert_eq!(v["ok"], true);
        assert_eq!(v["data"]["agents"][0], "claude");
        assert_eq!(v["meta"]["command"], "spawn");
        assert_eq!(v["meta"]["project"], r"D:\ohmyagents");
        assert!(v.get("error").is_none());
    }

    #[test]
    fn envelope_error_replaces_data() {
        let v = envelope("send", &root(), None, Some("no manifest".into()));
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
