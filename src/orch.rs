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
pub fn project_slug(root: &Path) -> String {
    let norm = root
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase();
    let hex = format!("{:x}", Sha256::digest(norm.as_bytes()));
    hex[..8].to_string()
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

fn read_manifest(root: &Path) -> Option<Manifest> {
    let text = std::fs::read_to_string(manifest_path(root)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_manifest(root: &Path, m: &Manifest) -> Result<(), String> {
    let path = manifest_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let body = serde_json::to_string_pretty(m).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))
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
        let manifest = read_manifest(root)
            .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
        if !manifest.pipe.is_empty() {
            if let Ok(rmux) = sdk_connect(&manifest.pipe).await {
                return Ok(Link {
                    rmux,
                    rmux_bin,
                    label,
                    pipe: manifest.pipe,
                });
            }
        }
        if rmuxpoc::label_alive(&rmux_bin, &label) {
            let (pipe, _) = rmuxpoc::label_socket_path(&rmux_bin, &label)?;
            let rmux = sdk_connect(&pipe).await?;
            let mut healed = manifest;
            healed.pipe = pipe.clone();
            write_manifest(root, &healed)?;
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
    vec![
        format!("OHMYAGENTS_PROJECT={}", root.display()),
        format!("OHMYAGENTS_AGENT={agent}"),
        format!(
            "OHMYAGENTS_STATE_FILE={}",
            state_file(root, agent).display()
        ),
    ]
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
        println!("spawn.pane.{agent}={id}");
        m.agents.push(ManifestAgent {
            name: agent.clone(),
            pane_id: id.as_u32() as u64,
        });
    }
    write_manifest(root, &m)?;

    // The product session now keeps the daemon alive; drop the boot keeper.
    let boot = boot_session_name(root);
    let _ = rmuxpoc::run_cli_checked(
        &link.rmux_bin,
        &["-L", link.label.as_str(), "kill-session", "-t", &boot],
        "kill boot session",
    );
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
    read_manifest(root)
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
pub async fn status(link: &Link, root: &Path) -> Result<Vec<PaneStatus>, String> {
    let manifest = read_manifest(root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;

    let mut entries = Vec::new();
    for a in &manifest.agents {
        let pane = pane_for(&session, a.pane_id).await?;
        let info = pane.info().await.map_err(|e| format!("info: {e}"))?;
        let pid = pane
            .id()
            .await
            .map_err(|e| e.to_string())?
            .and_then(|id| info.pane(id))
            .and_then(|p| match p.process {
                PaneProcessState::Running { pid: Some(pid) } => Some(pid),
                _ => None,
            });
        entries.push((a.name.clone(), pane, pid));
    }

    let pids: Vec<u32> = entries.iter().filter_map(|(_, _, p)| *p).collect();
    let names = process_names(&pids).unwrap_or_default();

    let mut out = Vec::new();
    for (agent, pane, pid) in entries {
        let process = pid.and_then(|p| names.get(&p).cloned());
        let terminal = match pane.snapshot().await {
            Ok(snap) => classify_snapshot(&snap).oma_state(),
            Err(_) => "unknown",
        };
        out.push(PaneStatus {
            process,
            terminal,
            hook_state: hook_state(root, &agent),
            agent,
            pid,
        });
    }
    Ok(out)
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
    let manifest = read_manifest(root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
    let entry = manifest
        .agents
        .iter()
        .find(|a| a.name == agent)
        .ok_or_else(|| format!("agent {agent} not in this session"))?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let pane = pane_for(&session, entry.pane_id).await?;

    let pid = rmuxpoc::running_pid(&pane).await?;
    let expected = if manifest.stub { "pwsh" } else { agent };
    let names = process_names(&[pid])?;
    let actual = expect_process(&names, pid, expected)?;

    let multiline = text.contains('\n') || text.contains('\r');
    if multiline {
        paste_three_step(link, &pane, entry.pane_id, text)?;
    } else {
        pane.send_text(text.to_string())
            .await
            .map_err(|e| format!("send_text: {e}"))?;
    }
    // Enter is always its own dispatch; the pasted payload carries none.
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    println!("send.agent={agent}");
    println!("send.proc={actual}");
    println!(
        "send.split={}",
        if multiline { "paste-buffer-p+Enter" } else { "text+Enter" }
    );

    if let Some(marker) = confirm {
        pane.expect_visible_text()
            .to_contain(marker)
            .timeout(Duration::from_secs(20))
            .await
            .map_err(|e| format!("confirm marker {marker} not visible: {e}"))?;
        println!("send.confirm={marker}");
    }
    Ok(())
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
const TRUST_DIALOGS: &[(&str, &str)] = &[
    // (marker, confirm key)
    ("trust this folder", "Enter"), // claude workspace trust (P0009)
];

pub async fn settle(
    link: &Link,
    root: &Path,
    wait_secs: u64,
) -> Result<Vec<(String, String)>, String> {
    let manifest = read_manifest(root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(&link.rmux, name).await?;
    let deadline =
        std::time::Instant::now() + Duration::from_secs(wait_secs);

    let mut outcomes = Vec::new();
    for agent in &manifest.agents {
        let pane = pane_for(&session, agent.pane_id).await?;
        let mut dismissed: Vec<String> = Vec::new();
        loop {
            let lines = match pane.snapshot().await {
                Ok(snap) => snap.visible_lines(),
                Err(_) => Vec::new(),
            };
            let tail: String = lines
                .iter()
                .rev()
                .take(12)
                .rev()
                .cloned()
                .collect::<Vec<String>>()
                .join("\n")
                .to_lowercase();
            let hit = TRUST_DIALOGS
                .iter()
                .find(|(marker, _)| tail.contains(marker));
            let Some((marker, key)) = hit else { break };
            if std::time::Instant::now() > deadline {
                break;
            }
            let key = key.to_string();
            let marker = marker.to_string();
            pane.send_key(&key)
                .await
                .map_err(|e| format!("settle {}: {e}", agent.name))?;
            dismissed.push(format!("{marker}:{key}"));
            tokio::time::sleep(Duration::from_millis(1500)).await;
        }
        let outcome = if dismissed.is_empty() {
            "none".to_string()
        } else {
            format!("dismissed={}", dismissed.join(","))
        };
        println!("settle.pane.{}={}", agent.name, outcome);
        outcomes.push((agent.name.clone(), outcome));
    }
    Ok(outcomes)
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
}

/// Layer 2 wins when it speaks; a silent hook falls back to the 1b terminal
/// verdict. Anything but a clear idle blocks the dispatch (never resend).
pub fn gate(hook_state: Option<&str>, terminal: &str) -> Gate {
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

fn next_task_id(root: &Path) -> String {
    let dir = tasks_dir(root);
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
    format!("t{:03}", max + 1)
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
    let manifest = read_manifest(root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
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
            list
        }
        None => in_session,
    };
    if targets.is_empty() {
        return Err("no agents to dispatch to".into());
    }

    let panes = status(link, root).await?;
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
        }
    }

    let task_id = next_task_id(root);
    if !sent.is_empty() {
        let record = TaskRecord {
            id: task_id.clone(),
            text: text.to_string(),
            created: now,
            assigned,
        };
        write_task(root, &record)?;
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
        assert_eq!(project_slug(a).len(), 8);
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
        assert_eq!(read_manifest(&root).unwrap().agents[0].pane_id, 7);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plan_agents_stub_shape_and_count_guard() {
        let plan = plan_agents(Some(vec!["claude".into(), "codex".into()]), true).unwrap();
        assert_eq!(plan.agents.len(), 2);
        assert!(plan.agents[0].1.first().unwrap().contains("pwsh"));
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
        assert_eq!(next_task_id(&root), "t001");
        let rec = TaskRecord {
            id: "t001".into(),
            text: "demo".into(),
            created: 42,
            assigned: [("claude".to_string(), 42u64)].into_iter().collect(),
        };
        let path = write_task(&root, &rec).unwrap();
        assert!(path.ends_with("t001.json"));
        assert_eq!(next_task_id(&root), "t002");
        let back: TaskRecord =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back.assigned["claude"], 42);
        let _ = std::fs::remove_dir_all(&root);
    }
}
