//! Product session orchestration: spawn / status / send / cleanup over a
//! stable per-project session (P0006). The POC layer (`rmuxpoc`) proved the
//! primitives; this module binds them to a reconnectable session identity:
//! one project -> one session name, one daemon endpoint, and a manifest that
//! maps each agent to its daemon-stable pane id.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rmux_sdk::{
    EnsureSession, Pane, PaneId, PaneProcessState, ProcessCommandSpec, ProcessSpec, Rmux,
    RmuxEndpoint, Session, SessionName, SplitDirection, TerminalSizeSpec,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agents;
use crate::catalog::RmuxPin;
use crate::rmux::{self, prepend_path};
use crate::rmuxpoc::{
    self, check_send_key, classify_snapshot, expect_process, is_transport_closed, process_names,
};

pub const AGENTS: [&str; 4] = ["claude", "codex", "grok", "kimi"];

/// Stable session identity: same project path -> same slug, so repeated
/// spawn/status/send/cleanup calls reconnect instead of piling sessions.
/// 中7 加固：**纯词法归一**（相对挂 cwd、清 `.`/`..`、斜杠统一）——不用
/// canonicalize：它的 best-effort 回退在「目录创建前后」算出不同 slug
/// （实踩：rm 后 spawn，label 时目录不存在回退原样、session 时已建又归
/// 一，同进程两个身份）。小写折叠仅 Windows（盘符等价），Unix 大小写敏
/// 感保留；hash 取前 16 hex（8 位 32bit 碰撞不可忽略）。
pub fn project_slug(root: &Path) -> String {
    let abs = if root.is_relative() {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(root)
    } else {
        root.to_path_buf()
    };
    let mut clean = PathBuf::new();
    for c in abs.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other.as_os_str()),
        }
    }
    let joined = clean.to_string_lossy().replace('\\', "/");
    let norm: String = if cfg!(windows) {
        joined.to_ascii_lowercase()
    } else {
        joined
    };
    let norm = norm.trim_end_matches('/');
    let hex = format!("{:x}", Sha256::digest(norm.as_bytes()));
    hex[..16].to_string()
}

pub fn session_name(root: &Path) -> Result<SessionName, String> {
    let raw = format!("oma-{}", project_slug(root));
    SessionName::new(&raw).map_err(|e| e.to_string())
}

/// The CLI label namespace for this project's daemon. Stable across
/// invocations; the real pipe name (with salt) is queried, not derived.
pub fn label(root: &Path) -> String {
    format!("oma-{}", project_slug(root))
}

/// The boot keeper must not have the product session name as a prefix:
/// rmux `-t` matching is prefix-based, so `oma-<slug>` would otherwise
/// resolve against `oma-<slug>-boot` and look like the session exists.
fn boot_session_name(root: &Path) -> String {
    format!("oma-boot-{}", project_slug(root))
}

fn endpoint_from_pipe(pipe: &str) -> RmuxEndpoint {
    if cfg!(windows) {
        RmuxEndpoint::WindowsPipe(pipe.to_string())
    } else {
        RmuxEndpoint::UnixSocket(PathBuf::from(pipe))
    }
}

fn state_file(root: &Path, agent: &str) -> PathBuf {
    root.join(".ohmyagents")
        .join("state")
        .join(format!("{agent}.json"))
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join(".ohmyagents").join("session.json")
}

/// Spawn manifest: the agent -> stable pane id map that survives CLI
/// invocations (pane ids are stable for one daemon lifetime), plus the
/// label endpoint so later commands can reach the same daemon.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub stub: bool,
    #[serde(default)]
    pub label: String,
    /// Real daemon pipe path (salted); healed on reconnect when stale.
    #[serde(default)]
    pub pipe: String,
    pub agents: Vec<ManifestAgent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ManifestAgent {
    pub name: String,
    pub pane_id: u64,
}

/// 读清单：区分「文件不在」（None）与「在但解析失败」（Some(Err)）——
/// 后者带错误上下文，恢复路径才能对症（P0026 高2）。
fn read_manifest(root: &Path) -> Result<Option<Manifest>, String> {
    let path = manifest_path(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    match serde_json::from_str(&text) {
        Ok(m) => Ok(Some(m)),
        Err(e) => Err(format!("{}: corrupt manifest: {e}", path.display())),
    }
}

fn read_manifest_opt(root: &Path) -> Option<Manifest> {
    read_manifest(root).ok().flatten()
}

/// 拿不到清单就报标准引导语；损坏时透传 corrupt 上下文（P0026 高2）。
fn read_manifest_req(root: &Path) -> Result<Manifest, String> {
    read_manifest(root)?.ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())
}

/// 原子写：先落同目录临时文件再 rename（P0026 高1a）——直写被并发读
/// 会撞见半截 JSON，rename 在同卷上是原子替换。
fn write_manifest(root: &Path, m: &Manifest) -> Result<(), String> {
    let path = manifest_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let body = serde_json::to_string_pretty(m).map_err(|e| e.to_string())? + "\n";
    // tmp 名带 pid 加进程内原子序号（Round2 kimi4：pid 只防跨进程；同进程
    // 内锁外 heal 写与持锁写仍共用 tmp，一方 rename 后另一方 NotFound）。
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = path.with_extension(format!("json.{}.{seq}.tmp", std::process::id()));
    std::fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A live handle to the project daemon from both transports: the SDK for
/// snapshots/waits/input and the CLI binary for load-buffer/paste-buffer
/// (Windows rejects every `-S` form, so the CLI rides the `-L` label).
pub struct Link {
    pub rmux: Rmux,
    pub rmux_bin: PathBuf,
    pub label: String,
    pub pipe: String,
}

async fn sdk_connect(pipe: &str) -> Result<Rmux, String> {
    rmuxpoc::prepare_env();
    Rmux::builder()
        .endpoint(endpoint_from_pipe(pipe))
        .default_timeout(Duration::from_secs(20))
        .connect()
        .await
        .map_err(|e| format!("connect {pipe}: {e}"))
}

/// Connect to the project's labeled daemon. Booting it (spawn path) is the
/// caller's distinction: `boot` runs when no manifest points at a live one.
/// A stale pipe heals by re-querying `#{socket_path}` while the label lives.
pub async fn connect(root: &Path, boot: bool) -> Result<Link, String> {
    let pin = RmuxPin::load()?;
    let report =
        rmux::ensure(&pin, false).map_err(|e| format!("{e}; run `oma check` first"))?;
    if let Some(dir) = report.layout.dispatcher.parent() {
        prepend_path(dir);
    }
    let rmux_bin = report.layout.dispatcher.clone();
    let label = label(root);

    if !boot {
        // P0026 高2：manifest 缺失/损坏不再挡死建链——label 活即连。否则
        // 「session 在但 manifest 坏」时连 cleanup 都拿不到 link，形成只有
        // 手工 rmux 才能解的死局。
        let manifest = read_manifest(root).ok().flatten();
        if let Some(m) = manifest.as_ref() {
            if !m.pipe.is_empty() {
                if let Ok(rmux) = sdk_connect(&m.pipe).await {
                    return Ok(Link {
                        rmux,
                        rmux_bin,
                        label,
                        pipe: m.pipe.clone(),
                    });
                }
            }
        }
        if rmuxpoc::label_alive(&rmux_bin, &label) {
            let (pipe, _) = rmuxpoc::label_socket_path(&rmux_bin, &label)?;
            let rmux = sdk_connect(&pipe).await?;
            // 清单还在就顺手治愈 pipe；缺失/损坏不伪造——由 reconcile 引导
            // cleanup 重建（cleanup 本身已不依赖 manifest）。
            if let Some(mut healed) = manifest {
                healed.pipe = pipe.clone();
                write_manifest(root, &healed)?;
            }
            return Ok(Link {
                rmux,
                rmux_bin,
                label,
                pipe,
            });
        }
        return Err("session daemon is gone; run `oma spawn` to start a new one".into());
    }

    let boot_session = boot_session_name(root);
    let (pipe, _pid) = rmuxpoc::ensure_label_daemon(&rmux_bin, &label, &boot_session)?;
    let rmux = sdk_connect(&pipe).await?;
    Ok(Link {
        rmux,
        rmux_bin,
        label,
        pipe,
    })
}

pub struct SpawnPlan {
    pub agents: Vec<(String, Vec<String>)>,
    pub stub: bool,
}

/// Resolve which agents to launch: explicit list wins, otherwise the
/// installed intersection; `--stub` replaces every agent with a shell stub.
pub fn plan_agents(wanted: Option<Vec<String>>, stub: bool) -> Result<SpawnPlan, String> {
    // 重复名入口拒绝（Round1 kimi7/codex9：`--agents claude,claude` 过校验
    // 后 manifest 两条同名记录，find 永远只命中第一条成黑洞）。stub 分支
    // 同样过这道闸（顺带挡非法字符进 state 文件路径，Round1 kimi10）。
    if let Some(list) = &wanted {
        let mut seen = std::collections::BTreeSet::new();
        for n in list {
            if !n.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(format!("invalid agent name: {n}"));
            }
            if !seen.insert(n.clone()) {
                return Err(format!("duplicate agent in --agents: {n}"));
            }
        }
    }
    if stub {
        let names = wanted.unwrap_or_else(|| AGENTS.iter().map(|s| s.to_string()).collect());
        let agents = names
            .into_iter()
            .map(|n| (n, rmuxpoc::interactive_shell_argv()))
            .collect();
        return validate_count(SpawnPlan { agents, stub });
    }
    let names = match wanted {
        Some(list) => {
            let missing: Vec<String> = list
                .iter()
                .filter(|n| agents::find(n).is_none())
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Err(format!(
                    "agent(s) not found: {}; see `oma agents` for detection details",
                    missing.join(", ")
                ));
            }
            list
        }
        None => {
            let found: Vec<String> = AGENTS
                .iter()
                .filter(|a| agents::find(a).is_some())
                .map(|s| s.to_string())
                .collect();
            if found.is_empty() {
                return Err(
                    "no supported agent installed; run `oma agents`, or use --stub for a stub session"
                        .into(),
                );
            }
            found
        }
    };
    let agents = names
        .into_iter()
        .map(|name| {
            let hit = agents::find(&name).expect("validated above");
            (name, vec![hit.path.to_string_lossy().into_owned()])
        })
        .collect();
    validate_count(SpawnPlan { agents, stub })
}

fn validate_count(plan: SpawnPlan) -> Result<SpawnPlan, String> {
    let n = plan.agents.len();
    if n == 0 || n > 4 {
        return Err(format!("1 to 4 agents required, got {n}"));
    }
    Ok(plan)
}

fn env_entries(root: &Path, agent: &str) -> Vec<String> {
    let mut env = vec![
        format!("OHMYAGENTS_PROJECT={}", root.display()),
        format!("OHMYAGENTS_AGENT={agent}"),
        format!(
            "OHMYAGENTS_STATE_FILE={}",
            state_file(root, agent).display()
        ),
    ];
    // claude 路清掉子会话标记（P0019 真路实测）：oma 从 Claude Code 会话里
    // 拉起的 claude 会继承 CHILD_SESSION 而关闭 transcript，联邦 trace 检索
    // 不到该路的会话记录。覆盖为空值并显式开启 session 持久化。
    if agent == "claude" {
        env.push("CLAUDE_CODE_CHILD_SESSION=".into());
        env.push("CLAUDE_CODE_FORCE_SESSION_PERSISTENCE=1".into());
    }
    env
}

fn root_spec(argv: &[String], env: Vec<String>) -> ProcessSpec {
    let mut process = ProcessSpec::default();
    process.process_command = Some(ProcessCommandSpec::Argv(argv.to_vec()));
    process.environment = Some(env);
    process
}

/// Lay out 1/2/4 panes and record the agent -> pane id manifest. The boot
/// keeper session (which pulled the daemon up) dies once the product session
/// exists, so the daemon stays only for the real session.
pub async fn spawn(link: &Link, root: &Path, plan: &SpawnPlan) -> Result<Manifest, String> {
    let rmux = &link.rmux;
    let name = session_name(root)?;
    if rmuxpoc::reuse_only(rmux, name.clone()).await.is_ok() {
        return Err(format!(
            "session {} already exists; use oma status / send / cleanup",
            name.as_str()
        ));
    }
    std::fs::create_dir_all(root.join(".ohmyagents").join("state"))
        .map_err(|e| format!("state dir: {e}"))?;

    let first = &plan.agents[0];
    let session = rmux
        .ensure_session(
            EnsureSession::named(name)
                .create_only()
                .detached(true)
                .size(TerminalSizeSpec::new(120, 32))
                .working_directory(root.display().to_string())
                .process(root_spec(&first.1, env_entries(root, &first.0))),
        )
        .await
        .map_err(|e| format!("ensure_session: {e}"))?;

    let mut handles: Vec<Pane> = vec![session.pane(0, 0)];
    if plan.agents.len() >= 2 {
        let second = &plan.agents[1];
        let pane = split_spawn(
            &handles[0],
            SplitDirection::Right,
            &second.1,
            env_entries(root, &second.0),
            &second.0,
            root,
        )
        .await?;
        handles.push(pane);
    }
    if plan.agents.len() >= 3 {
        let third = &plan.agents[2];
        let pane = split_spawn(
            &handles[0],
            SplitDirection::Down,
            &third.1,
            env_entries(root, &third.0),
            &third.0,
            root,
        )
        .await?;
        handles.push(pane);
    }
    if plan.agents.len() == 4 {
        let fourth = &plan.agents[3];
        let pane = split_spawn(
            &handles[1],
            SplitDirection::Down,
            &fourth.1,
            env_entries(root, &fourth.0),
            &fourth.0,
            root,
        )
        .await?;
        handles.push(pane);
    }

    let mut m = Manifest {
        stub: plan.stub,
        label: link.label.clone(),
        pipe: link.pipe.clone(),
        agents: Vec::new(),
    };
    for ((agent, _argv), pane) in plan.agents.iter().zip(&handles) {
        let id = pane
            .id()
            .await
            .map_err(|e| format!("pane id: {e}"))?
            .ok_or_else(|| format!("pane for {agent} has no live id"))?;
        eprintln!("spawn.pane.{agent}={id}");
        m.agents.push(ManifestAgent {
            name: agent.clone(),
            pane_id: id.as_u32() as u64,
        });
    }
    write_manifest(root, &m)?;

    // The product session now keeps the daemon alive; drop the boot keeper
    // **before** relayout（kimi 四轮低项：relayout 时 boot 壳还在，label 下
    // 两个 session，select-layout 无 -t 的目标解析靠「最近 session」隐含
    // 假设，可能打到 boot 空壳上静默失效）。
    let boot = boot_session_name(root);
    let _ = rmuxpoc::run_cli_checked(
        &link.rmux_bin,
        &["-L", link.label.as_str(), "kill-session", "-t", &boot],
        "kill boot session",
    );
    // 新开会话也按路数定型（grok 复核：split 序列是 Right/Right/Down，3 路
    // 新开成上 2 下 1，不是用户要的三列）；幂等重排，失败只警告。
    relayout(link, m.agents.len());
    Ok(m)
}

async fn split_spawn(
    pane: &Pane,
    dir: SplitDirection,
    argv: &[String],
    env: Vec<String>,
    title: &str,
    cwd: &Path,
) -> Result<Pane, String> {
    let mut builder = pane
        .split_with(dir)
        .spawn(argv.to_vec())
        .title(title)
        .cwd(cwd.to_path_buf());
    for entry in env {
        let (k, v) = entry.split_once('=').expect("K=V form");
        builder = builder.env(k, v);
    }
    builder.await.map_err(|e| format!("split {dir:?}: {e}"))
}

async fn pane_for(session: &Session, id: u64) -> Result<Pane, String> {
    session
        .pane_by_id(PaneId::new(id as u32))
        .await
        .map_err(|e| format!("pane {id}: {e}"))
}

/// Read the manifest for a project (diagnostics and examples).
pub fn read_manifest_for(root: &Path) -> Option<Manifest> {
    read_manifest_opt(root)
}

/// Resolve one agent lane to its live Pane: manifest 定位 pane_id，再经
/// session pane_by_id 拿句柄。传输面（SSE 画面等）用，与 send 同源。
pub async fn pane_for_agent(
    link: &Link,
    root: &Path,
    agent: &str,
) -> Result<(u64, Pane), String> {
    let manifest = read_manifest_req(root)?;
    let entry = manifest
        .agents
        .iter()
        .find(|a| a.name == agent)
        .ok_or_else(|| format!("agent {agent} not in this session"))?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let pane = pane_for(&session, entry.pane_id).await?;
    Ok((entry.pane_id, pane))
}

/// Pane lookup for diagnostics and examples.
pub async fn pane_for_test(session: &Session, id: u64) -> Result<Pane, String> {
    pane_for(session, id).await
}

pub struct PaneStatus {
    pub agent: String,
    pub pid: Option<u32>,
    pub process: Option<String>,
    pub terminal: &'static str,
    pub hook_state: Option<String>,
}

fn hook_state(root: &Path, agent: &str) -> Option<String> {
    let text = std::fs::read_to_string(state_file(root, agent)).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("state").and_then(|s| s.as_str()).map(String::from)
}

/// Read-only status: layer 0 (alive/pid) + locate (process name) + layer 1b
/// (terminal semantics) + layer 2 (state file when present).
/// 状态 + 可选告警（中10：进程名批查失败不再伪装成 process=null 正常
/// 态，进 warning 让上层透传给用户）。
pub async fn status(link: &Link, root: &Path) -> Result<(Vec<PaneStatus>, Option<String>), String> {
    let manifest = read_manifest_req(root)?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;

    let mut entries = Vec::new();
    for a in &manifest.agents {
        // 一路 pane 消失（agent 进程退出、TUI 被对话框退出）不拖垮整份状态：
        // 该路降级为 dead，其余照报（P0019 真路验收实踩：codex 路退出后
        // status 整条报错，死路本身正是要看的信息）。
        let pane = match pane_for(&session, a.pane_id).await {
            Ok(p) => p,
            Err(_) => {
                entries.push((a.name.clone(), None, None));
                continue;
            }
        };
        // info/id 同样降级（Round1 claude4 / Round3 grok2 两轮未修：resolve
        // 成功后、info 前格子被并发 kill 时整条 status 上抛，且 run 的状态
        // 门跟着整单失败——与「一路不拖垮其它路」相反）。
        let pid = match pane.info().await {
            Ok(info) => pane
                .id()
                .await
                .ok()
                .flatten()
                .and_then(|id| info.pane(id))
                .and_then(|p| match p.process {
                    PaneProcessState::Running { pid: Some(pid) } => Some(pid),
                    _ => None,
                }),
            Err(_) => None,
        };
        entries.push((a.name.clone(), Some(pane), pid));
    }

    let pids: Vec<u32> = entries.iter().filter_map(|(_, _, p)| *p).collect();
    // pwsh+CIM 批查是秒级同步子进程：放 spawn_blocking，不占 tokio worker
    //（P0026 高4）；失败降级为空表但带 warning（中10）。
    let pids_for_blocking = pids.clone();
    let names_result = tokio::task::spawn_blocking(move || process_names(&pids_for_blocking))
        .await
        .map_err(|e| format!("process_names join: {e}"));
    let warning = match &names_result {
        Ok(Ok(_)) => None,
        Ok(Err(e)) => Some(format!("process lookup failed: {e}")),
        Err(e) => Some(e.clone()),
    };
    let names = match names_result {
        Ok(Ok(n)) => n,
        _ => Default::default(),
    };

    let mut out = Vec::new();
    for (agent, pane, pid) in entries {
        let process = pid.and_then(|p| names.get(&p).cloned());
        let terminal = match pane {
            None => "dead",
            Some(pane) => match pane.snapshot().await {
                Ok(snap) => classify_snapshot(&snap).oma_state(),
                Err(_) => "unknown",
            },
        };
        out.push(PaneStatus {
            process,
            terminal,
            hook_state: hook_state(root, &agent),
            agent,
            pid,
        });
    }
    Ok((out, warning))
}


// ---------------------------------------------------------------------------
// 和解式编排（P0024）：命令面只见 agent 实例，服务/会话/窗口/窗格/PTY 全部
// 绑在 agent 背后。三态：新开（无会话整体拉起）、附加（已开且活直接绑）、
// 重新打开（死路或 respawn 强制：关旧开新）。
// ---------------------------------------------------------------------------

pub struct ReconcileOutcome {
    pub attached: Vec<String>,
    pub respawned: Vec<String>,
    /// 精确集合收掉的多余路（不在本次 plan 里的）。
    pub removed: Vec<String>,
}

/// 新拉路的就绪确认（用户定调 2026-09-01：命令持续到任务正常开始才退出，
/// 有阻塞就告警）。**由消费面在 reconcile 之后、会话锁外调用**（kimi 四轮
/// 中3：放 reconcile 内会随 HTTP /spawn 进 gate，4 路全重开最坏 80s 堵
/// send/run/cleanup；中2：新开会话分支同样要过这里——respawned 含全部
/// 新拉路）。每路等 TUI 就绪（idle 稳定/working/画面变化任一，20s）；
/// 死路（进程秒退）由「pane gone」覆盖。返回 alerts 供三通道出口。
pub async fn await_lanes_ready(
    link: &Link,
    root: &Path,
    agents: &[String],
) -> Vec<String> {
    let mut alerts = Vec::new();
    let Ok(name) = session_name(root) else {
        return alerts;
    };
    let Ok(session) = rmuxpoc::reuse_only(&link.rmux, name).await else {
        return alerts;
    };
    let manifest = match read_manifest_req(root) {
        Ok(m) => m,
        Err(_) => return alerts,
    };
    for agent in agents {
        let Some(entry) = manifest.agents.iter().find(|a| a.name == *agent) else {
            continue;
        };
        match pane_for(&session, entry.pane_id).await {
            Ok(pane) => {
                alerts.extend(
                    await_task_start(&pane, agent, Duration::from_secs(20), true).await,
                );
            }
            Err(_) => alerts.push(format!("{agent}: pane gone right after respawn")),
        }
    }
    alerts
}

/// agent 实例活判据：pane 存在 + pid 活 + 进程名匹配（stub 记 pwsh，真 agent 记本名）。
async fn agent_alive(
    session: &Session,
    pane_id: u64,
    stub: bool,
    agent: &str,
) -> bool {
    let Ok(pane) = pane_for(session, pane_id).await else {
        return false;
    };
    let Ok(pid) = rmuxpoc::running_pid(&pane).await else {
        return false;
    };
    // stub 进程名平台化（Round1 claude9：非 Windows 的桩是 sh，硬编码
    // pwsh 会让 Unix 上 stub 路每次判死反复重开）。
    let stub_proc = if cfg!(windows) { "pwsh" } else { "sh" };
    let expected = if stub { stub_proc } else { agent };
    // 同步 pwsh 批查放 blocking 池（P0026 高4）。
    tokio::task::spawn_blocking(move || process_names(&[pid]))
        .await
        .ok()
        .and_then(|r| r.ok())
        .and_then(|names| expect_process(&names, pid, expected).ok())
        .is_some()
}

/// 和解：会话不在整体新开；在则逐路判活（附加）或重开（split 回一路并回写 manifest）。
/// 取代旧「会话已存在即拒绝」——附加正是防叠格的正确形态。
pub async fn reconcile(
    link: &Link,
    root: &Path,
    plan: &SpawnPlan,
) -> Result<ReconcileOutcome, String> {
    let name = session_name(root)?;
    if rmuxpoc::reuse_only(&link.rmux, name.clone()).await.is_err() {
        let m = spawn(link, root, plan).await?;
        return Ok(ReconcileOutcome {
            attached: Vec::new(),
            respawned: m.agents.iter().map(|a| a.name.clone()).collect(),
            removed: Vec::new(),
        });
    }
    std::fs::create_dir_all(root.join(".ohmyagents").join("state"))
        .map_err(|e| format!("state dir: {e}"))?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let mut m = read_manifest(root)?.ok_or_else(|| {
        "session exists but manifest is missing; run `oma cleanup` then `oma spawn`".to_string()
    })?;

    // 精确集合（用户定调 2026-09-01 二次，取代中8「补缺不移除」）：命令面
    // 要几路就几路——`--agents codex` 就是一路。**先补后收**（grok 三轮
    // 警告实炸：单路换单路时先收唯一活路 → 会话空 → daemon 随末 session
    // 退 → 补路无 daemon 可用）；收格失败容忍（格可能已死，幂等 kill 会
    // 放过）。
    let wanted: Vec<&str> = plan.agents.iter().map(|(a, _)| a.as_str()).collect();
    let surplus: Vec<String> = m
        .agents
        .iter()
        .map(|a| a.name.clone())
        .filter(|n| !wanted.contains(&n.as_str()))
        .collect();
    let mut removed = Vec::new();
    let mut attached = Vec::new();
    let mut respawned = Vec::new();
    // 判活按本次计划的 stub 语义（P0026 中8）：旧会话是 stub、本次真身时，
    // stub pane 视为死路重开成真 agent；manifest 的 stub 也随计划回写。
    for (agent, argv) in &plan.agents {
        let alive = match m.agents.iter().find(|a| &a.name == agent) {
            Some(entry) => agent_alive(&session, entry.pane_id, plan.stub, agent).await,
            None => false,
        };
        if alive {
            attached.push(agent.clone());
            eprintln!("reconcile.{agent}=attached");
            continue;
        }
        // 死/缺：**先分后杀**（kimi 四轮高项：先杀唯一 pane 会终结会话、
        // daemon 随末 session 退——单路 respawn/stub 切换必踩，M040 同根因
        // 的另一半）。先 split 新格、回写 pane_id，再清旧格（kill 失败容忍：
        // 旧格多活几秒无副作用，防堆积由 kill 幂等兜底）。
        let old_pane = m.agents.iter().find(|a| &a.name == agent).map(|e| e.pane_id);
        // 从主窗格右侧分回一路（窗格复杂性在此，命令面不感知）。
        let base = session.pane(0, 0);
        let pane = split_spawn(
            &base,
            SplitDirection::Right,
            argv,
            env_entries(root, agent),
            agent,
            root,
        )
        .await?;
        let id = pane
            .id()
            .await
            .map_err(|e| format!("pane id: {e}"))?
            .ok_or_else(|| format!("respawned pane for {agent} has no live id"))?;
        let pid_u32 = id.as_u32() as u64;
        eprintln!("reconcile.{agent}=respawned pane={pid_u32}");
        match m.agents.iter_mut().find(|a| &a.name == agent) {
            Some(entry) => entry.pane_id = pid_u32,
            None => m.agents.push(ManifestAgent {
                name: agent.clone(),
                pane_id: pid_u32,
            }),
        }
        // 每路即落盘（Round2 grok3/kimi3：循环外统一写时，下一路 split 失
        // 走 `?` 早退会让本路新格成无人认领的孤儿，逐次累积）。
        write_manifest(root, &m)?;
        if let Some(old) = old_pane {
            if pane_for(&session, old).await.is_ok() {
                // kill 失败 = 孤儿 pane（codex 五轮中3：manifest 已指向新格，
                // 旧格此后无人重试）——强告警人工处理，不静默。
                if let Err(e) = kill_pane(link, &session, old).await {
                    eprintln!("spawn.alert={agent}: orphan pane %{old} left behind ({e}) — close it manually");
                }
            }
        }
        respawned.push(agent.clone());
    }
    // 计划路全部就位后才收多余路——任何时刻会话不空、daemon 不退。
    for agent in &surplus {
        if let Some(entry) = m.agents.iter().find(|a| &a.name == agent) {
            let pane_id = entry.pane_id;
            if pane_for(&session, pane_id).await.is_ok() {
                // kill 失败（pane 仍在）不移出 manifest（codex 复核中5）：
                // 移了会留孤儿 pane 且 relayout 的路数与实际不符。
                if let Err(e) = kill_pane(link, &session, pane_id).await {
                    eprintln!("reconcile.{agent}=remove-failed ({e}); lane kept");
                    continue;
                }
            }
        }
        m.agents.retain(|a| &a.name != agent);
        removed.push(agent.clone());
        eprintln!("reconcile.{agent}=removed");
    }
    // stub 随本次计划回写（中8）：manifest 反映当前语义，后续 send/status
    // 的进程名判定（pwsh vs agent 本名）不再拿旧会话语义误判。
    m.stub = plan.stub;
    write_manifest(root, &m)?;
    // 无条件按路数定型（grok 复核：纯附加时原布局可能是残留乱格；重排幂
    // 等且是毫秒级 CLI，「有动作才排」的省略不值得留缺口）。
    relayout(link, m.agents.len());
    Ok(ReconcileOutcome { attached, respawned, removed })
}

/// 布局自愈（2026-09-01 用户定调）：按**实际路数**选形态——1 路全屏、
/// 2/3 路左右列分（even-horizontal，单格即全屏）、4 路 2x2（tiled）。
/// respawn/死路重开的 kill+split 会留不规则网格（实测 kimi 独占半屏），
/// 此处一键重排——幂等，活 pane 内容不动只动边框；label 单 session 无
/// -t 歧义。失败只警告不阻塞主流程。
fn relayout(link: &Link, lanes: usize) {
    let layout = match lanes {
        1 | 2 | 3 => "even-horizontal",
        _ => "tiled",
    };
    if let Err(e) = rmuxpoc::run_cli_checked(
        &link.rmux_bin,
        &["-L", link.label.as_str(), "select-layout", layout],
        "select-layout",
    ) {
        eprintln!("relayout.{layout}=failed ({e})");
    }
}

/// 关单窗格（kill-pane 只打该格，不动会话与其它路）。幂等：pane 已不在
/// 视为成功；其余失败上抛（P0026 高3：吞错会留旧进程/旧格，重开即堆积）。
async fn kill_pane(link: &Link, session: &Session, pane_id: u64) -> Result<(), String> {
    let target = format!("%{pane_id}");
    if rmuxpoc::run_cli_checked(
        &link.rmux_bin,
        &["-L", link.label.as_str(), "kill-pane", "-t", &target],
        "kill-pane",
    )
    .is_ok()
    {
        return Ok(());
    }
    // kill 报错但 pane 确已不在（并发退出等）：幂等成功。
    if pane_for(session, pane_id).await.is_err() {
        return Ok(());
    }
    Err(format!("kill-pane %{pane_id} failed and pane still present"))
}

/// 强制重新打开一路 agent 实例：**先分后杀**（kimi 四轮高项：单路会话
/// 先杀唯一 pane 会终结会话、daemon 随退，M040 同根因另一半）——split
/// 新格、回写 pane_id，再清旧格（kill 失败容忍，旧格多活几秒无副作用）。
pub async fn respawn(link: &Link, root: &Path, agent: &str) -> Result<u64, String> {
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let mut m = read_manifest_req(root)?;
    // 成员校验（Round1 codex3/claude7：respawn 此前绕过校验，不在会话的
    // 名字被静默加路、4 路可加到第 5 路——破坏精确集合语义）。
    if !m.agents.iter().any(|a| a.name == agent) {
        return Err(format!(
            "agent {agent} not in this session; use `oma spawn --agents ...` to change the lane set"
        ));
    }
    let argv = respawn_argv(root, &m, agent)?;
    let old_pane = m.agents.iter().find(|a| a.name == agent).map(|e| e.pane_id);
    let base = session.pane(0, 0);
    let pane = split_spawn(
        &base,
        SplitDirection::Right,
        &argv,
        env_entries(root, agent),
        agent,
        root,
    )
    .await?;
    let id = match pane
        .id()
        .await
        .map_err(|e| format!("pane id: {e}"))?
    {
        Some(id) => id,
        None => {
            // id 拿不到（进程秒退等）：pane 句柄无 id 可 kill（relay3 kimi1：
            // 此前的「清理」是空操作）——诚实告警人工跟，manifest 未动。
            eprintln!("respawn.alert={agent}: new pane has no live id; it may linger — close manually if visible");
            return Err(format!("respawn pane for {agent} has no live id"));
        }
    };
    let pid_u32 = id.as_u32() as u64;
    // **先写盘再杀旧**（relay2 grok3：与 reconcile 增量落盘同序——先杀后
    // 写时 write 失败磁盘指向已杀的旧 pane，新格成孤儿；注释此前就写的
    // 是这个顺序，代码没跟上）。
    match m.agents.iter_mut().find(|a| a.name == agent) {
        Some(entry) => entry.pane_id = pid_u32,
        None => m.agents.push(ManifestAgent {
            name: agent.to_string(),
            pane_id: pid_u32,
        }),
    }
    write_manifest(root, &m)?;
    if let Some(old) = old_pane {
        if pane_for(&session, old).await.is_ok() {
            // 同上（codex 五轮中3）：kill 失败打孤儿强告警。
            if let Err(e) = kill_pane(link, &session, old).await {
                eprintln!("respawn.alert={agent}: orphan pane %{old} left behind ({e}) — close it manually");
            }
        }
    }
    relayout(link, m.agents.len());
    Ok(pid_u32)
}

/// 重开一路用的 argv：真 agent 沿用安装探测交集逻辑，stub 会话用 shell 桩。
fn respawn_argv(root: &Path, m: &Manifest, agent: &str) -> Result<Vec<String>, String> {
    if m.stub {
        return Ok(rmuxpoc::interactive_shell_argv());
    }
    let _ = root;
    let found = agents::find(agent)
        .map(|p| p.path)
        .ok_or_else(|| format!("agent {agent} not found; see `oma agents`"))?;
    Ok(vec![found.display().to_string()])
}

/// 发单个按键（受守卫入口，2026-09-01 用户定调：「对 Codex 发 C-c 是禁项」
/// 应在代码层警告——裸 rmux CLI 绕过守卫曾实杀一路 codex）。守卫：
/// `check_send_key`（codex 拒 C-c，M001 一个 C-c 杀进程）。
pub async fn key(
    link: &Link,
    root: &Path,
    agent: &str,
    key: &str,
) -> Result<(), String> {
    let (_, pane) = pane_for_agent(link, root, agent).await?;
    rmuxpoc::check_send_key(agent, key)?;
    pane.send_key(key.to_string())
        .await
        .map_err(|e| format!("send_key {key}: {e}"))
}

/// Two send shapes share one guard chain (agent key policy, manifest
/// locate, process locate):
/// - single line: SDK `send_text` then `send_key("Enter")` (two dispatches)
/// - multi line: three-step paste over the CLI label (`load-buffer` +
///   `paste-buffer -p`), Enter still a separate dispatch (S005 iron rule)
pub async fn send(
    link: &Link,
    root: &Path,
    agent: &str,
    text: &str,
    confirm: Option<&str>,
) -> Result<(), String> {
    check_send_key(agent, "Enter")?;
    let manifest = read_manifest_req(root)?;
    let entry = manifest
        .agents
        .iter()
        .find(|a| a.name == agent)
        .ok_or_else(|| format!("agent {agent} not in this session"))?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let pane = pane_for(&session, entry.pane_id).await?;

    let pid = rmuxpoc::running_pid(&pane).await?;
    let stub_proc = if cfg!(windows) { "pwsh" } else { "sh" };
    let expected = if manifest.stub { stub_proc } else { agent };
    // 同步 pwsh 批查放 blocking 池（P0026 高4）。
    let names = tokio::task::spawn_blocking(move || process_names(&[pid]))
        .await
        .map_err(|e| format!("process_names join: {e}"))??;
    let actual = expect_process(&names, pid, expected)?;

    // 发送前快照做 baseline（中6）：残留同文本会让 contains 立即满足。
    let baseline: Option<String> = pane
        .snapshot()
        .await
        .ok()
        .map(|s| s.visible_lines().join("\n"));

    let multiline = text.contains('\n') || text.contains('\r');
    if multiline {
        paste_three_step(link, &pane, entry.pane_id, text)?;
    } else {
        pane.send_text(text.to_string())
            .await
            .map_err(|e| format!("send_text: {e}"))?;
    }
    // S005 铁律「隔开发」：等载荷末行短头在画面可见再单独 Enter——rmux 原生
    // 静默等待，不盲 sleep（P0010 实证：紧连的 Enter 被 codex TUI 吞）。
    // 超时不致命：降级为旧形态照发 Enter 并留痕。
    // 中6：发送前 baseline 已含同文本（上一轮残留）时不再误判——改等
    // 「画面变化后仍在」，拿不到变化按超时降级。
    let last_line = text.lines().last().unwrap_or("");
    let head: String = last_line.trim().chars().take(24).collect();
    if !head.is_empty() {
        match await_new_text(&pane, &head, baseline.as_deref(), Duration::from_secs(5)).await {
            Ok(true) => eprintln!("send.echo=visible"),
            Ok(false) => eprintln!("send.echo=timeout; Enter 照发"),
            Err(e) => eprintln!("send.echo=stale-wait ({e}); Enter 照发"),
        }
    }
    // Enter is always its own dispatch; the pasted payload carries none.
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    eprintln!("send.agent={agent}");
    eprintln!("send.proc={actual}");
    eprintln!(
        "send.split={}",
        if multiline { "paste-buffer-p+Enter" } else { "text+Enter" }
    );

    if let Some(marker) = confirm {
        // 中6：marker 若已在发送前画面（残留），等「变化后仍在」才认。
        await_new_text(&pane, marker, baseline.as_deref(), Duration::from_secs(20))
            .await
            .map_err(|e| format!("confirm marker {marker} not re-visible: {e}"))?
            .then_some(())
            .ok_or_else(|| format!("confirm marker {marker} not visible in 20s"))?;
        eprintln!("send.confirm={marker}");
    }
    // 任务开始确认（用户定调 2026-09-01：命令持续到任务正常开始才退出，
    // 有阻塞就告警）——**拆成锁外收尾**（Round1 grok3/kimi4：HTTP /send
    // 持 gate 跨 15s 等待把写操作全堵死，与 spawn 移出锁外自相矛盾）：
    // 本函数只做锁内的粘贴/Enter/短头确认；开始确认由调用方在锁外跑
    // `send_start_alerts`，alerts 进 CLI stderr 与三通道信封 data（grok2）。
    Ok(())
}

/// send 的锁外收尾：任务开始确认（15s），返回 alerts。CLI 打 stderr、
/// api 层进信封——由各通道在**会话锁外**调用。
pub async fn send_start_alerts(link: &Link, root: &Path, agent: &str) -> Vec<String> {
    let mut alerts = Vec::new();
    let Ok((_, pane)) = pane_for_agent(link, root, agent).await else {
        alerts.push(format!("{agent}: pane gone right after send"));
        return alerts;
    };
    let found = await_task_start(&pane, agent, Duration::from_secs(15), false).await;
    for a in &found {
        eprintln!("send.alert={a}");
    }
    // Round2 kimi1：found 必须进返回值——只 eprintln 会让三通道信封
    // alerts 恒空，锁外确认的收益被整体丢弃。
    alerts.extend(found);
    alerts
}

/// 等一路「任务真开始」：双信号任一即认——①分类器 working；②画面内容
/// 在变（grok 类 spinner 分类器不识别，但干活时画面必变）。命中
/// blocked（确认/密码框）立即告警；超时（含停在 unknown/idle，如文本
/// 被首屏吞、提交没触发）告警。`accept_idle`：spawn 场景 TUI 就绪（idle
/// 稳定屏）即算过；send 场景 idle 超时说明任务没启动。
async fn await_task_start(
    pane: &Pane,
    agent: &str,
    timeout: Duration,
    accept_idle: bool,
) -> Vec<String> {
    let deadline = std::time::Instant::now() + timeout;
    let mut last: Option<u64> = None;
    let mut idle_stable = 0;
    loop {
        let snap = match pane.snapshot().await {
            Ok(s) => s,
            Err(e) => {
                return vec![format!("{agent}: snapshot failed while awaiting start ({e})")];
            }
        };
        let state = classify_snapshot(&snap).oma_state();
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            snap.visible_lines().hash(&mut h);
            h.finish()
        };
        if state == "working" {
            return Vec::new();
        }
        // blocked 判定先于画面变化信号（codex 五轮高项：先 idle 后弹框时
        // 首帧变化会先命中返回空，把「任务被阻塞」误报成「已开始」）。
        if state == "blocked" {
            return vec![format!("{agent}: blocked (confirm/password dialog) — task NOT started")];
        }
        if let Some(prev) = last {
            if prev != hash {
                return Vec::new();
            }
        }
        if state == "idle" && accept_idle {
            // 就绪语义：idle 且画面已稳定（连续两帧同 hash）即认。
            idle_stable += 1;
            if idle_stable >= 2 {
                return Vec::new();
            }
        }
        if std::time::Instant::now() > deadline {
            return vec![format!(
                "{agent}: still '{state}' after {}s — task may not have started (text swallowed? not submitted?)",
                timeout.as_secs()
            )];
        }
        last = Some(hash);
        tokio::time::sleep(Duration::from_millis(800)).await;
    }
}

/// 等目标文本「新出现」：baseline 不含时走 rmux 原生静默等待（最优路径）；
/// baseline 已含（上轮残留）时轮询快照等「内容变化后目标仍在」（中6）。
/// 返回 Ok(true)=等到，Ok(false)=超时（调用方自定降级），Err=轮询失败。
async fn await_new_text(
    pane: &Pane,
    needle: &str,
    baseline: Option<&str>,
    timeout: Duration,
) -> Result<bool, String> {
    let stale = baseline.is_some_and(|b| b.contains(needle));
    if !stale {
        return pane
            .expect_visible_text()
            .to_contain(needle)
            .timeout(timeout)
            .await
            .map(|_| true)
            .map_err(|e| e.to_string());
    }
    let baseline = baseline.unwrap_or_default().to_string();
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(400)).await;
        let now = pane
            .snapshot()
            .await
            .map_err(|e| format!("snapshot: {e}"))?;
        let text = now.visible_lines().join("\n");
        if text != baseline && text.contains(needle) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Three-step paste: payload file (UTF-8, no ESC, sender never wraps
/// bracketed-paste escapes) -> `load-buffer` -> `paste-buffer -p` onto the
/// pane. Targeting rides the stable pane id (`%N`), same source as the SDK.
fn paste_three_step(link: &Link, _pane: &Pane, pane_id: u64, text: &str) -> Result<(), String> {
    if text.contains('\u{1b}') {
        return Err("payload must not contain ESC; bracketed-paste wrappers belong to the daemon".into());
    }
    let file = std::env::temp_dir().join(format!(
        "oma-paste-{}-{}.txt",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::write(&file, text).map_err(|e| format!("{}: {e}", file.display()))?;
    let buffer = format!("oma-paste-{}", std::process::id());
    let target = format!("%{pane_id}");
    let path = file.display().to_string();
    let label = link.label.as_str();

    let result = (|| -> Result<(), String> {
        rmuxpoc::run_cli_checked(
            &link.rmux_bin,
            &["-L", label, "load-buffer", "-b", &buffer, &path],
            "load-buffer",
        )?;
        rmuxpoc::run_cli_checked(
            &link.rmux_bin,
            &["-L", label, "paste-buffer", "-p", "-b", &buffer, "-t", &target],
            "paste-buffer",
        )?;
        Ok(())
    })();
    // Buffer and payload file are scratch; clean up either way.
    let _ = rmuxpoc::run_cli_checked(
        &link.rmux_bin,
        &["-L", label, "delete-buffer", "-b", &buffer],
        "delete-buffer",
    );
    let _ = std::fs::remove_file(&file);
    result
}

/// Session-scoped cleanup: never the daemon-wide stop. The manifest goes
/// away with the session.
pub async fn cleanup(link: &Link, root: &Path) -> Result<bool, String> {
    let name = session_name(root)?;
    let existed = match rmuxpoc::reuse_only(&link.rmux, name).await {
        Ok(session) => match session.kill().await {
            Ok(existed) => existed,
            Err(e) if is_transport_closed(&e.to_string()) => true,
            Err(e) => return Err(format!("kill session: {e}")),
        },
        Err(e) if is_transport_closed(&e) => true,
        Err(e)
            if e.contains("cannot find")
                || e.contains("can't find")
                || e.contains("no session") =>
        {
            false
        }
        Err(e) => return Err(e),
    };
    let _ = std::fs::remove_file(manifest_path(root));
    Ok(existed)
}

// ---------------------------------------------------------------------------
// oma settle: self-detect trust dialogs and confirm them so the agent
// persists trust itself (the fallback for pre-seeded trust, P0010).
// ---------------------------------------------------------------------------

/// Whitelisted trust-dialog markers only. Task-semantic confirmations
/// ("run this command?") are never auto-answered here.
/// (marker, key 序列)。P0019 真路实测三态：claude 信任框默认焦点即信任项；
/// kimi 默认焦点在 Don't trust（Enter 会选不信任并退出，先上移）；
/// codex 升级提示屏选 2 Skip（自动升级不属 settle 职责，保守项）。
const TRUST_DIALOGS: &[(&str, &[&str])] = &[
    ("do you trust the files", &["Enter"]),
    ("don't trust", &["Up", "Enter"]),
    ("update available", &["2", "Enter"]),
    // codex hooks 审查屏（2026-09-01 实拍）：oma init 部署的项目级 hooks
    // 首启需 review，`t` 一键 trust all 后面板仍开着，补 Esc 关闭回工作区。
    ("hooks need review", &["t", "Esc"]),
];

pub async fn settle(
    link: &Link,
    root: &Path,
    wait_secs: u64,
) -> Result<Vec<(String, String)>, String> {
    // 钳位在唯一实现点（Round2 grok2/kimi5：只钳 api 层时，CLI 文本
    // `--wait <巨大 u64>` 直下本函数仍会 Instant 加法溢出 panic）。
    let wait_secs = wait_secs.min(600);
    let manifest = read_manifest_req(root)?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let deadline =
        std::time::Instant::now() + Duration::from_secs(wait_secs);

    // 窗口内外层循环（codex 复核抓的真缺陷：原「每路首扫未命中即 break」
    // 让 wait_secs 只对命中后超时生效，config 扫描后才出现的屏等不到）：
    // 每轮快扫全部路、命中的当场处理，全空时稍歇再扫直到窗口结束——
    // 窗口是全局共享的，不会被第一路吃光。
    let mut outcomes: Vec<(String, Vec<String>)> = manifest
        .agents
        .iter()
        .map(|a| (a.name.clone(), Vec::new()))
        .collect();
    // stalled 路：按后 marker 顽固不消失（屏不在白名单语义或按键无效）——
    // 冷却到窗口结束不再按（grok 复核抓的死循环：外层重扫同 marker 立即
    // 重按，直到 deadline 耗尽，重复提交风险回来了）。
    let mut stalled: std::collections::HashSet<String> = Default::default();
    loop {
        let mut any_hit = false;
        for agent in &manifest.agents {
            if stalled.contains(&agent.name) {
                continue;
            }
            let Some(entry) = outcomes.iter_mut().find(|(n, _)| n == &agent.name) else {
                continue;
            };
            let pane = match pane_for(&session, agent.pane_id).await {
                Ok(p) => p,
                Err(_) => continue,
            };
            let lines = match pane.snapshot().await {
                Ok(snap) => snap.visible_lines(),
                Err(_) => continue,
            };
            // 行级短行匹配（中12 收紧）：marker 须命中**单行**且该行是对话框
            // 形态（trimmed ≤ 80 列）——P0019 三态实测（claude 问句行、kimi
            // 菜单项、codex 屏顶标题）都是短行；正文段落里的同词多在长行，
            // 全屏子串会把普通输出误当菜单自动按键。
            let hit = TRUST_DIALOGS.iter().find(|(marker, _)| {
                lines.iter().any(|l| {
                    let t = l.trim();
                    t.chars().count() <= 80 && t.to_lowercase().contains(marker)
                })
            });
            let Some((marker, keys)) = hit else { continue };
            if std::time::Instant::now() > deadline {
                break;
            }
            any_hit = true;
            let marker = marker.to_string();
            for key in keys.iter() {
                pane.send_key(*key)
                    .await
                    .map_err(|e| format!("settle {}: {e}", agent.name))?;
                tokio::time::sleep(Duration::from_millis(400)).await;
            }
            // 按后确认（2026-09-01 实踩：升级屏关掉后快照仍是旧帧，立即
            // 重按一轮会把「2」落进输入框提交成任务）——等 marker 从屏上
            // 消失（最多 3s）；顽固不消失**不重按**（防重复提交），记
            // stalled（kimi 四轮中4：stalled 必须进 outcome，否则三通道
            // 把未自愈当已自愈），该路冷却人工接手。
            let marker_still = |lines: &Vec<String>| {
                lines.iter().any(|l| {
                    let t = l.trim();
                    t.chars().count() <= 80 && t.to_lowercase().contains(&marker)
                })
            };
            let confirm_deadline = std::time::Instant::now() + Duration::from_secs(3);
            let mut confirmed = false;
            while std::time::Instant::now() < confirm_deadline {
                tokio::time::sleep(Duration::from_millis(400)).await;
                match pane.snapshot().await {
                    Ok(snap) if !marker_still(&snap.visible_lines()) => {
                        confirmed = true;
                        break;
                    }
                    _ => {}
                }
            }
            if confirmed {
                entry.1.push(format!("{marker}:{}", keys.join("+")));
            } else {
                eprintln!("settle.{}.stalled={marker}: marker still on screen; NOT re-sending keys", agent.name);
                entry.1.push(format!("stalled={marker}:{}", keys.join("+")));
                stalled.insert(agent.name.clone());
            }
        }
        if std::time::Instant::now() > deadline {
            break;
        }
        if !any_hit {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    let mut result = Vec::new();
    for (agent, dismissed) in outcomes {
        let outcome = if dismissed.is_empty() {
            "none".to_string()
        } else {
            let stalled_part: Vec<String> = dismissed
                .iter()
                .filter(|d| d.starts_with("stalled="))
                .cloned()
                .collect();
            let ok_part: Vec<String> = dismissed
                .iter()
                .filter(|d| !d.starts_with("stalled="))
                .cloned()
                .collect();
            let mut parts = Vec::new();
            if !ok_part.is_empty() {
                parts.push(format!("dismissed={}", ok_part.join(",")));
            }
            if !stalled_part.is_empty() {
                parts.push(stalled_part.join(","));
            }
            parts.join(";")
        };
        eprintln!("settle.pane.{agent}={outcome}");
        result.push((agent, outcome));
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// oma run: dispatch with a per-agent state gate (S009 order) and the layer-3
// task mapping file.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    Dispatch,
    Blocked,
    Busy,
    /// 死路（pane 没了）：与 blocked/blocked 分开报——该 respawn 不是等。
    Dead,
}

/// Layer 2 wins when it speaks; a silent hook falls back to the 1b terminal
/// verdict. Anything but a clear idle blocks the dispatch (never resend).
pub fn gate(hook_state: Option<&str>, terminal: &str) -> Gate {
    // 死路优先（relay1 codex 低项：pane 已消失但 state 文件残留 hook
    // "idle"/"working" 的陈旧组合会让 run 按 Dispatch/Busy 走到 send 报错，
    // 而不是按 dead 引导 respawn）。
    if terminal == "dead" {
        return Gate::Dead;
    }
    match hook_state {
        Some("idle") => Gate::Dispatch,
        Some("blocked") => Gate::Blocked,
        Some("working") | Some("unknown") => Gate::Busy,
        _ => match terminal {
            "idle" => Gate::Dispatch,
            "blocked" => Gate::Blocked,
            _ => Gate::Busy,
        },
    }
}

fn tasks_dir(root: &Path) -> PathBuf {
    root.join(".ohmyagents").join("tasks")
}

/// 分配任务 id：scan 出初值后用 `create_new` 原子占位，撞号自增重试
/// （P0026 高1b：纯 scan-then-increment 在并发 run 下会拿到同号互覆）。
/// 占位是零字节探测文件，随后 write_task 原子覆写。
fn alloc_task_id(root: &Path) -> Result<String, String> {
    let dir = tasks_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut max = 0u32;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let stem = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(num) = stem.strip_prefix('t').and_then(|n| n.parse::<u32>().ok()) {
                max = max.max(num);
            }
        }
    }
    let mut n = max + 1;
    loop {
        let id = format!("t{n:03}");
        let probe = dir.join(format!("{id}.json"));
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
            Ok(_) => return Ok(id),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                n += 1;
                continue;
            }
            Err(e) => return Err(format!("{}: {e}", probe.display())),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskRecord {
    pub id: String,
    pub text: String,
    pub created: u64,
    pub assigned: std::collections::BTreeMap<String, u64>,
}

fn write_task(root: &Path, record: &TaskRecord) -> Result<PathBuf, String> {
    let dir = tasks_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{}.json", record.id));
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(record).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

pub struct RunOutcome {
    pub task_id: String,
    pub sent: Vec<String>,
    pub skipped: Vec<(String, String)>,
}

/// Dispatch one task to the session's agents: gate each lane by state, send
/// through the guarded path, and record what was actually assigned.
pub async fn run(
    link: &Link,
    root: &Path,
    text: &str,
    assign: Option<Vec<String>>,
    confirm: Option<&str>,
) -> Result<RunOutcome, String> {
    let manifest = read_manifest_req(root)?;
    let in_session: Vec<String> = manifest.agents.iter().map(|a| a.name.clone()).collect();
    let targets: Vec<String> = match assign {
        Some(list) => {
            let unknown: Vec<String> = list
                .iter()
                .filter(|n| !in_session.contains(n))
                .cloned()
                .collect();
            if !unknown.is_empty() {
                return Err(format!(
                    "agent(s) not in this session: {}",
                    unknown.join(", ")
                ));
            }
            // dedupe 保序（Round3 codex3：`--assign codex,codex` 同路连发两
            // 次；plan_agents 已拒重复名，run 补同门——重复直接忽略而非报
            // 错，CLI 拼写失误不该挡派发）。
            let mut seen = std::collections::BTreeSet::new();
            list.into_iter()
                .filter(|n| seen.insert(n.clone()))
                .collect()
        }
        None => in_session,
    };
    if targets.is_empty() {
        return Err("no agents to dispatch to".into());
    }

    let (panes, _) = status(link, root).await?;
    let by_agent: std::collections::HashMap<String, &PaneStatus> = panes
        .iter()
        .map(|p| (p.agent.clone(), p))
        .collect();

    let mut sent = Vec::new();
    let mut skipped = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut assigned = std::collections::BTreeMap::new();

    for agent in &targets {
        let verdict = by_agent
            .get(agent)
            .map(|p| gate(p.hook_state.as_deref(), p.terminal))
            .unwrap_or(Gate::Busy);
        match verdict {
            Gate::Dispatch => match send(link, root, agent, text, confirm).await {
                Ok(()) => {
                    assigned.insert(agent.clone(), now);
                    sent.push(agent.clone());
                }
                Err(e) => skipped.push((agent.clone(), e)),
            },
            Gate::Blocked => skipped.push((agent.clone(), "blocked".into())),
            Gate::Busy => skipped.push((agent.clone(), "busy".into())),
            Gate::Dead => skipped.push((agent.clone(), "dead".into())),
        }
    }

    // 占位与写盘都在确有派发时才发生（P0026 高1b：并发 run 撞号）。
    let task_id = if !sent.is_empty() {
        alloc_task_id(root)?
    } else {
        String::new()
    };
    if !sent.is_empty() {
        let record = TaskRecord {
            id: task_id.clone(),
            text: text.to_string(),
            created: now,
            assigned,
        };
        // 写失败清掉 alloc 留下的零字节占位（codex 复核低项：空 tNNN.json
        // 会成为将来 trace/task 读取的脏数据）。
        if let Err(e) = write_task(root, &record) {
            // 记账失败不整单报错（Round3 claude4 遗留：send 已实际打进各
            // 路，操作者看到 Err 重试会重复派发同一任务）——告警留痕，
            // 派发结果照常返回。
            let _ = std::fs::remove_file(tasks_dir(root).join(format!("{task_id}.json")));
            eprintln!("run.alert=dispatched but task record failed: {e}");
        }
    }
    Ok(RunOutcome {
        task_id,
        sent,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per-call suffix: same-millisecond parallel tests must not
    /// share (and mutually delete) a temp dir.
    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn slug_is_stable_and_path_sensitive() {
        let a = Path::new("D:\\code\\alpha");
        let a2 = Path::new("D:/code/alpha/");
        assert_eq!(project_slug(a), project_slug(a2));
        assert_ne!(project_slug(a), project_slug(Path::new("D:\\code\\beta")));
        // 中7：hash 前 16 hex（64bit，碰撞可忽略）。
        assert_eq!(project_slug(a).len(), 16);
        let name = session_name(a).unwrap();
        assert!(name.as_str().starts_with("oma-"));
    }

    #[test]
    fn env_entries_point_at_project_state_file() {
        let root = Path::new("D:\\code\\alpha");
        let env = env_entries(root, "codex");
        assert_eq!(env[0], format!("OHMYAGENTS_PROJECT={}", root.display()));
        assert_eq!(env[1], "OHMYAGENTS_AGENT=codex");
        assert!(env[2].ends_with(r".ohmyagents\state\codex.json"));
    }

    #[test]
    fn manifest_round_trips() {
        let root = std::env::temp_dir().join(format!(
            "oma-orch-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let m = Manifest {
            stub: true,
            label: "oma-abcd1234".into(),
            pipe: r"\\.\pipe\rmux-oma-abcd1234".into(),
            agents: vec![ManifestAgent {
                name: "claude".into(),
                pane_id: 7,
            }],
        };
        write_manifest(&root, &m).unwrap();
        assert_eq!(read_manifest(&root).unwrap().unwrap().agents[0].pane_id, 7);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_agents_stub_shape_and_count_guard() {
        let plan = plan_agents(Some(vec!["claude".into(), "codex".into()]), true).unwrap();
        assert_eq!(plan.agents.len(), 2);
        // Round2 kimi22：非 Windows 桩是 sh，按平台断言。
        if cfg!(windows) {
            assert!(plan.agents[0].1.first().unwrap().contains("pwsh"));
        } else {
            assert!(plan.agents[0].1.first().unwrap().contains("sh"));
        }
        let five = plan_agents(
            Some(vec![
                "a".into(),
                "b".into(),
                "c".into(),
                "d".into(),
                "e".into(),
            ]),
            true,
        );
        assert!(five.is_err());
        assert!(plan_agents(Some(vec![]), true).is_err());
    }

    #[test]
    fn gate_layer2_wins_and_silence_falls_back_to_1b() {
        use Gate::{Blocked, Busy, Dispatch};
        // Layer 2 speaks: hook wins regardless of the terminal verdict.
        assert_eq!(gate(Some("idle"), "working"), Dispatch);
        assert_eq!(gate(Some("blocked"), "idle"), Blocked);
        assert_eq!(gate(Some("working"), "idle"), Busy);
        assert_eq!(gate(Some("unknown"), "idle"), Busy);
        // Silent hook: the 1b terminal verdict decides; only idle dispatches.
        assert_eq!(gate(None, "idle"), Dispatch);
        assert_eq!(gate(None, "blocked"), Blocked);
        assert_eq!(gate(None, "working"), Busy);
        assert_eq!(gate(None, "unknown"), Busy);
    }

    #[test]
    fn task_ids_increment_and_records_round_trip() {
        let root = std::env::temp_dir().join(format!(
            "oma-run-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(alloc_task_id(&root).unwrap(), "t001");
        let rec = TaskRecord {
            id: "t001".into(),
            text: "demo".into(),
            created: 42,
            assigned: [("claude".to_string(), 42u64)].into_iter().collect(),
        };
        let path = write_task(&root, &rec).unwrap();
        assert!(path.ends_with("t001.json"));
        // 占位文件残留（write_task 已覆写同号）后继续分配不回退。
        assert_eq!(alloc_task_id(&root).unwrap(), "t002");
        let back: TaskRecord =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back.assigned["claude"], 42);
        let _ = std::fs::remove_dir_all(&root);
    }
}
