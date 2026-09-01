//! 传输无关的编排操作层（P0011）：六操作在此返回结构化 JSON。
//! HTTP（`server` feature）与 MCP（`mcp` feature，后续）共用这一份，
//! CLI 保持 main.rs 的行式打印不动——三传输零逻辑重复指这里对 orch 的收敛。

use serde_json::{json, Value};
use std::path::Path;

use crate::orch;

/// 三传输共用的响应信封（S016 吸收）：HTTP 直接吐它，CLI `--json` 吐它，
/// MCP 包成 `structured`。形：`{ok, data|error, meta:{command, project}}`。
pub fn envelope(command: &str, root: &Path, outcome: Result<Value, String>) -> Value {
    let mut v = json!({
        "ok": outcome.is_ok(),
        "meta": { "command": command, "project": root.display().to_string() },
    });
    match outcome {
        Ok(d) => v["data"] = d,
        Err(e) => v["error"] = Value::String(e),
    }
    v
}

/// 和解式拉起（P0024）：会话不在新开；在则活路附加、死路重开。
/// 命令面只见 agent 实例，窗格复杂性绑在背后。
pub async fn spawn(
    root: &Path,
    agents: Option<Vec<String>>,
    stub: bool,
) -> Result<Value, String> {
    let plan = orch::plan_agents(agents, stub)?;
    let link = orch::connect(root, true).await?;
    let out = orch::reconcile(&link, root, &plan).await?;
    let manifest = orch::read_manifest_for(root)
        .ok_or_else(|| "manifest missing after reconcile".to_string())?;
    Ok(json!({
        "project": root.display().to_string(),
        "session": orch::session_name(root)?.as_str(),
        "label": link.label,
        "stub": manifest.stub,
        "attached": out.attached,
        "respawned": out.respawned,
        "agents": manifest.agents.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
        "panes": manifest.agents.iter().map(|a| a.pane_id).collect::<Vec<_>>(),
    }))
}

/// 强制重新打开一路 agent 实例（关闭旧窗格再开新一路）。
pub async fn respawn(root: &Path, agent: &str) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let pane_id = orch::respawn(&link, root, agent).await?;
    Ok(json!({ "agent": agent, "pane": pane_id }))
}

/// 只读状态：各路 pid、进程名、终端态、hook 态。
pub async fn status(root: &Path) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let panes = orch::status(&link, root).await?;
    let rows: Vec<Value> = panes
        .iter()
        .map(|p| {
            json!({
                "agent": p.agent,
                "pid": p.pid,
                "process": p.process,
                "terminal": p.terminal,
                "hook": p.hook_state.as_deref().unwrap_or("silent"),
            })
        })
        .collect();
    Ok(json!({
        "project": root.display().to_string(),
        "session": orch::session_name(root)?.as_str(),
        "panes": rows,
    }))
}

/// 单路发送：守卫链与粘贴细节全在 orch::send。
pub async fn send(
    root: &Path,
    agent: &str,
    text: &str,
    confirm: Option<&str>,
) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    orch::send(&link, root, agent, text, confirm).await?;
    Ok(json!({ "agent": agent, "sent": true }))
}

/// 状态门分派：sent 与 skipped（agent: reason）都进 data。
pub async fn run(
    root: &Path,
    text: &str,
    assign: Option<Vec<String>>,
    confirm: Option<&str>,
) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let outcome = orch::run(&link, root, text, assign, confirm).await?;
    let skipped: Vec<Value> = outcome
        .skipped
        .iter()
        .map(|(agent, reason)| json!({ "agent": agent, "reason": reason }))
        .collect();
    Ok(json!({
        "task": outcome.task_id,
        "sent": outcome.sent,
        "skipped": skipped,
        "dispatched": !outcome.sent.is_empty(),
    }))
}

/// 自愈信任：各路结果（agent: outcome）。
pub async fn settle(root: &Path, wait_secs: u64) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let outcomes = orch::settle(&link, root, wait_secs).await?;
    let rows: Vec<Value> = outcomes
        .iter()
        .map(|(agent, outcome)| json!({ "agent": agent, "outcome": outcome }))
        .collect();
    Ok(json!({ "settled": rows }))
}

/// 收尾：只杀本项目会话。
pub async fn cleanup(root: &Path) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let existed = orch::cleanup(&link, root).await?;
    Ok(json!({ "killed": existed, "scope": "session" }))
}

// ---- 官方 web 镜像（rmux web-share，P0021）----

/// rmux CLI 输出（URL/PIN 在 stderr）。
fn web_share_cli(
    link: &orch::Link,
    args: &[&str],
) -> Result<String, String> {
    let mut full: Vec<&str> = vec!["-L", link.label.as_str()];
    full.extend(args.iter().copied());
    let out = crate::rmuxpoc::run_cli(&link.rmux_bin, &full)?;
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if !out.status.success() {
        return Err(format!("web-share: {}", text.trim()));
    }
    Ok(text)
}

/// 起一路官方 web 镜像：operator 可操作（真 attach），spectator 只看。
/// `agent=None` 是 **session scope**（整会话一个 URL：全窗格、operator 可编辑、
/// 解锁分屏等 session controls——P0021 真路结论：pane scope 无窗格操作）；
/// 给 agent 则单 pane scope。
pub async fn web_share(
    root: &Path,
    agent: Option<&str>,
    spectator: bool,
    ttl: u64,
    frontend_url: Option<&str>,
    no_pin: bool,
) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let target = match agent {
        Some(a) => {
            let (pane_id, _) = orch::pane_for_agent(&link, root, a).await?;
            format!("%{pane_id}")
        }
        None => orch::session_name(root)?.as_str().to_string(),
    };
    let ttl_s = ttl.to_string();
    let mode = if spectator { "--spectator-only" } else { "--operator-only" };
    let mut argv: Vec<&str> = vec!["web-share", "-t", &target, mode, "--ttl", &ttl_s];
    if let Some(fe) = frontend_url {
        argv.extend(["--frontend-url", fe]);
    }
    // 本地场景免 PIN（用户定调：127.0.0.1 直接连接）；官方域外发面保留。
    if no_pin {
        argv.push("--no-pin");
    }
    let text = web_share_cli(&link, &argv)?;
    let url = text
        .lines()
        .filter_map(|l| {
            l.split_whitespace()
                .find(|w| w.starts_with("https://") || w.starts_with("http://"))
        })
        .next()
        .ok_or_else(|| "web-share 输出里没有 URL".to_string())?
        .to_string();
    let pin = text
        .lines()
        .find(|l| l.contains("pin"))
        .and_then(|l| l.split_whitespace().last())
        .unwrap_or("-")
        .to_string();
    let expires = text
        .lines()
        .find(|l| l.contains("expires"))
        .map(|l| l.trim().to_string())
        .unwrap_or_default();
    Ok(json!({ "agent": agent.unwrap_or("*session*"), "url": url, "pin": pin, "expires": expires }))
}

/// 列活动 share（web-share list：`<id> <session>:<pane> ...`）。
pub async fn web_shares(root: &Path) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    let text = web_share_cli(&link, &["web-share", "list"])?;
    let rows: Vec<Value> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| {
            let mut it = l.split_whitespace();
            let id = it.next().unwrap_or("");
            let target = it.next().unwrap_or("");
            json!({ "id": id, "target": target, "raw": l })
        })
        .collect();
    Ok(json!({ "shares": rows }))
}

/// 断开一路 share（--disconnect <id>）。
pub async fn web_share_stop(root: &Path, id: &str) -> Result<Value, String> {
    let link = orch::connect(root, false).await?;
    web_share_cli(&link, &["web-share", "--disconnect", id])?;
    Ok(json!({ "disconnected": id }))
}

/// 轨迹检索三件（P0013 联动）：sessions / timeline / search。
/// 同样走 JSON，供 MCP tools 与将来的 HTTP trace 端点共用。

pub fn trace_sessions(root: &Path) -> Value {
    let rows: Vec<Value> = crate::trace::list_sessions(root)
        .iter()
        .map(|s| {
            json!({
                "agent": s.agent,
                "id": s.id,
                "started_at": s.started_at,
                "file": s.file.display().to_string(),
            })
        })
        .collect();
    json!({ "project": root.display().to_string(), "sessions": rows })
}

fn trace_event_json(e: &crate::trace::TraceEvent, with_patch: bool) -> Value {
    let mut v = json!({
        "agent": e.agent,
        "session": e.session_id,
        "op": e.operation_id(),
        "file": e.file,
        "kind": e.kind.as_str(),
        "tool": e.tool,
        "ts": e.ts,
        "ts_ms": e.ts_ms,
        "intent": e.user_intent,
        "op_intent": e.op_intent,
    });
    if with_patch {
        v["patch"] = match &e.patch {
            Some(p) => Value::String(p.clone()),
            None => Value::Null,
        };
    }
    v
}

pub fn trace_timeline(
    root: &Path,
    agent: Option<&str>,
    file_glob: Option<&str>,
    limit: usize,
) -> Value {
    let filter = crate::trace::TraceFilter {
        agent,
        file_glob,
        limit,
    };
    let events = crate::trace::apply_filter(crate::trace::timeline(root), &filter);
    let rows: Vec<Value> = events.iter().map(|e| trace_event_json(e, false)).collect();
    json!({
        "project": root.display().to_string(),
        "edits": rows,
        "count": rows.len(),
    })
}

pub fn trace_search(root: &Path, query: &str, agent: Option<&str>, limit: usize) -> Value {
    // 先全量匹配再截断：limit 若在匹配前生效会把候选池截没。
    let filter = crate::trace::TraceFilter {
        agent,
        file_glob: None,
        limit: crate::trace::MAX_LIMIT,
    };
    let mut hits: Vec<_> = crate::trace::apply_filter(crate::trace::timeline(root), &filter)
        .into_iter()
        .filter(|e| crate::trace::search_matches(e, query))
        .collect();
    let blocks = crate::trace::group_blocks(&hits).len();
    hits.truncate(limit.clamp(1, crate::trace::MAX_LIMIT));
    let rows: Vec<Value> = hits.iter().map(|e| trace_event_json(e, true)).collect();
    json!({
        "project": root.display().to_string(),
        "hits": rows,
        "hits_count": rows.len(),
        "blocks_count": blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_rejects_unknown_agent_via_plan() {
        // plan_agents 是 spawn 的第一道闸：未知 agent 名在连接 rmux 前就报错。
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt
            .block_on(spawn(Path::new("."), Some(vec!["nope".into()]), false))
            .unwrap_err();
        assert!(err.contains("nope"), "error should name the agent: {err}");
    }

    #[test]
    fn envelope_carries_meta_and_single_payload() {
        let root = Path::new(r"D:\demo");
        let ok = envelope("status", root, Ok(json!({"panes": []})));
        assert_eq!(ok["ok"], true);
        assert_eq!(ok["data"]["panes"].as_array().map(Vec::len), Some(0));
        assert!(ok.get("error").is_none());
        assert_eq!(ok["meta"]["command"], "status");
        let bad = envelope("send", root, Err("no manifest".into()));
        assert_eq!(bad["ok"], false);
        assert_eq!(bad["error"], "no manifest");
        assert!(bad.get("data").is_none());
    }
}
