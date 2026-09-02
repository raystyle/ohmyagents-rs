//! MCP 适配层（P0011 切片 3，feature `mcp`）：oma 作为 MCP server 暴露
//! 编排六操作与 trace 检索 tools，stdio 传输（无网络面）。
//! 工具体全部转调 api 层——与 HTTP 同一份编排核心；返回沿用同一信封
//! `{ok, data|error, meta}`（S016）。业务失败走 `structured_error`（caller
//! 可见），只有基础设施故障才上 JSON-RPC 错误。
//! 铁律：本模块与它调到的任何代码都不得向 stdout 打印——stdout 是
//! JSON-RPC 通道（orch 进度行已迁 stderr）。

use std::path::{Path, PathBuf};

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::transport::io::stdio;
use rmcp::{
    schemars, serve_server, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api;

#[derive(Clone)]
pub struct OmaMcp {
    root: PathBuf,
    /// 会话写串行化（Round1 四家全中：rmcp 对每个 JSON-RPC 请求 spawn
    /// 独立任务并发执行，两个并发 oma_send 的三段式粘贴会交错——buffer
    /// 名同 pid 互撞、cleanup 与 spawn 构成 manifest TOCTOU）。与 HTTP
    /// gate 同语义；就绪确认/settle 在锁外（spawn_finalize 内）。
    gate: std::sync::Arc<tokio::sync::Mutex<()>>,
}

/// stdio 起服务：stdin/stdout 走 MCP 协议，进度只进 stderr。
pub async fn run(root: PathBuf) -> Result<(), String> {
    let service = OmaMcp::new(root);
    let transport = stdio();
    let server = serve_server(service, transport)
        .await
        .map_err(|e| format!("mcp init: {e}"))?;
    eprintln!("mcp.ok=true transport=stdio");
    server
        .waiting()
        .await
        .map(|_| ())
        .map_err(|e| format!("mcp wait: {e}"))
}

impl OmaMcp {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            gate: std::sync::Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}

/// 信封化：与 HTTP server 同形，传输只换壳。
fn envelope(
    command: &str,
    root: &Path,
    outcome: Result<Value, String>,
) -> Result<CallToolResult, McpError> {
    let meta = json!({ "command": command, "project": root.display().to_string() });
    Ok(match outcome {
        Ok(data) => CallToolResult::structured(json!({ "ok": true, "data": data, "meta": meta })),
        Err(e) => {
            CallToolResult::structured_error(json!({ "ok": false, "error": e, "meta": meta }))
        }
    })
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SpawnParams {
    /// 要拉起的 agent 名单；缺省取已装交集
    agents: Option<Vec<String>>,
    /// 用 shell 桩替代真实 agent（验收与调试）
    stub: Option<bool>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SendParams {
    /// 目标 agent 名（claude/codex/grok/kimi）
    agent: String,
    /// 任务文本（多行自动走三段式粘贴）
    text: String,
    /// 期望在画面上看到的确认短头
    confirm: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct RunParams {
    /// 任务文本
    text: String,
    /// 指定分派路；缺省全会话
    assign: Option<Vec<String>>,
    /// 期望在画面上看到的确认短头
    confirm: Option<String>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SettleParams {
    /// 全局扫描窗口秒数（grok 复核订正：窗口内反复扫全部路等晚出现的屏，非每路各一份）
    wait: Option<u64>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct TimelineParams {
    /// 只看某家 agent
    agent: Option<String>,
    /// 文件过滤（glob；解析失败退子串）
    file: Option<String>,
    /// 条数上限（1-1000）
    limit: Option<usize>,
}

#[derive(Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// 正则（非法正则退字面子串）
    query: String,
    /// 只看某家 agent
    agent: Option<String>,
    /// 条数上限（1-1000）
    limit: Option<usize>,
}

#[tool_router]
impl OmaMcp {
    #[tool(
        description = "在项目专属会话拉起多路终端 agent（claude/codex/grok/kimi）；agents 缺省取已装交集，stub 用 shell 桩；拉起后自动过一轮信任框（同 CLI/HTTP 通道）"
    )]
    async fn oma_spawn(
        &self,
        Parameters(SpawnParams { agents, stub }): Parameters<SpawnParams>,
    ) -> Result<CallToolResult, McpError> {
        let out = {
            let _guard = self.gate.lock().await;
            api::spawn(&self.root, agents, stub.unwrap_or(false)).await
        };
        let mut out = out;
        if let Ok(v) = &mut out {
            api::spawn_finalize(&self.root, v).await;
        }
        envelope("spawn", &self.root, out)
    }

    #[tool(description = "只读列出会话各路 agent 的 pid、进程名、终端态与 hook 态")]
    async fn oma_status(&self) -> Result<CallToolResult, McpError> {
        envelope("status", &self.root, api::status(&self.root).await)
    }

    #[tool(
        description = "向会话内某路 agent 发任务文本（多行自动走三段式粘贴）；confirm 为期望可见的确认短头；开始确认与告警同 CLI/HTTP"
    )]
    async fn oma_send(
        &self,
        Parameters(SendParams {
            agent,
            text,
            confirm,
        }): Parameters<SendParams>,
    ) -> Result<CallToolResult, McpError> {
        // 锁内粘贴、锁外确认（同 HTTP 形态）。
        let mut out = {
            let _guard = self.gate.lock().await;
            api::send_locked(&self.root, &agent, &text, confirm.as_deref()).await
        };
        if out.is_ok() {
            api::send_finalize(&self.root, &agent, out.as_mut().unwrap()).await;
        }
        envelope("send", &self.root, out)
    }

    #[tool(
        description = "状态门分派任务到多路 agent：一路 blocked/busy 跳过不堵其它路；assign 指定分派路，缺省全会话"
    )]
    async fn oma_run(
        &self,
        Parameters(RunParams {
            text,
            assign,
            confirm,
        }): Parameters<RunParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut out = {
            let _guard = self.gate.lock().await;
            api::run_locked(&self.root, &text, assign, confirm.as_deref()).await
        };
        if let Ok(v) = &mut out {
            api::run_finalize(&self.root, v).await;
        }
        envelope("run", &self.root, out)
    }

    #[tool(
        description = "自检测并自动确认信任/审查框（各家自己持久化信任；密码类永不自动）；wait 为全局扫描窗口秒数（显式 settle 缺省 30，上限 600）"
    )]
    async fn oma_settle(
        &self,
        Parameters(SettleParams { wait }): Parameters<SettleParams>,
    ) -> Result<CallToolResult, McpError> {
        let _guard = self.gate.lock().await;
        envelope(
            "settle",
            &self.root,
            api::settle(&self.root, wait.unwrap_or(30)).await,
        )
    }

    #[tool(description = "只杀本项目会话并清 manifest；不动 daemon 与其它会话")]
    async fn oma_cleanup(&self) -> Result<CallToolResult, McpError> {
        let _guard = self.gate.lock().await;
        envelope("cleanup", &self.root, api::cleanup(&self.root).await)
    }

    #[tool(description = "检索项目内各 agent 的原生会话（四家联邦：claude/codex/grok/kimi）")]
    fn oma_trace_sessions(&self) -> Result<CallToolResult, McpError> {
        envelope(
            "trace.sessions",
            &self.root,
            Ok(api::trace_sessions(&self.root)),
        )
    }

    #[tool(
        description = "检索项目编辑轨迹（意图操作块元素视图）：每条编辑带 operation_id、kind、双意图"
    )]
    fn oma_trace_timeline(
        &self,
        Parameters(TimelineParams { agent, file, limit }): Parameters<TimelineParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit
            .unwrap_or(crate::trace::DEFAULT_LIMIT)
            .clamp(1, crate::trace::MAX_LIMIT);
        envelope(
            "trace.timeline",
            &self.root,
            Ok(api::trace_timeline(
                &self.root,
                agent.as_deref(),
                file.as_deref(),
                limit,
            )),
        )
    }

    #[tool(
        description = "按正则检索 patch、file、双意图四域（非法正则退字面子串）；命中带 patch 全文"
    )]
    fn oma_trace_search(
        &self,
        Parameters(SearchParams {
            query,
            agent,
            limit,
        }): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = limit
            .unwrap_or(crate::trace::DEFAULT_LIMIT)
            .clamp(1, crate::trace::MAX_LIMIT);
        envelope(
            "trace.search",
            &self.root,
            Ok(api::trace_search(
                &self.root,
                &query,
                agent.as_deref(),
                limit,
            )),
        )
    }
}

#[tool_handler]
impl ServerHandler for OmaMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "oma 编排器：拉起多路 agent 会话（oma_spawn）、看状态（oma_status）、发任务（oma_send）、\
             状态门分派（oma_run）、自愈信任（oma_settle）、收尾（oma_cleanup）；另有四家 agent 的\
             项目轨迹检索（oma_trace_sessions/timeline/search）。会话按项目持久，可跨调用重连。",
        )
    }
}
