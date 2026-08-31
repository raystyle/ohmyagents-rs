//! 传输无关的编排操作层（P0011）：六操作在此返回结构化 JSON。
//! HTTP（`server` feature）与 MCP（`mcp` feature，后续）共用这一份，
//! CLI 保持 main.rs 的行式打印不动——三传输零逻辑重复指这里对 orch 的收敛。

use serde_json::{json, Value};
use std::path::Path;

use crate::orch;

/// 拉起会话：返回项目、会话名与 manifest 概要。
pub async fn spawn(
    root: &Path,
    agents: Option<Vec<String>>,
    stub: bool,
) -> Result<Value, String> {
    let plan = orch::plan_agents(agents, stub)?;
    let link = orch::connect(root, true).await?;
    let manifest = orch::spawn(&link, root, &plan).await?;
    let names: Vec<&str> = manifest.agents.iter().map(|a| a.name.as_str()).collect();
    Ok(json!({
        "project": root.display().to_string(),
        "session": orch::session_name(root)?.as_str(),
        "label": link.label,
        "stub": manifest.stub,
        "agents": names,
        "panes": manifest.agents.iter().map(|a| a.pane_id).collect::<Vec<_>>(),
    }))
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
}
