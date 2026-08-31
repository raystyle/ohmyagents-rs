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

pub fn endpoint(root: &Path) -> RmuxEndpoint {
    let slug = project_slug(root);
    if cfg!(windows) {
        RmuxEndpoint::WindowsPipe(format!(r"\\.\pipe\rmux-oma-{slug}"))
    } else {
        let dir = std::env::temp_dir().join(format!("ohmyagents-oma-{slug}"));
        let _ = std::fs::create_dir_all(&dir);
        RmuxEndpoint::UnixSocket(dir.join("socket"))
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
/// invocations (pane ids are stable for one daemon lifetime).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub stub: bool,
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

/// Connect to the project's dedicated daemon, starting it when absent (WMI
/// breakaway under a Job Object, same escape as the POC layer).
pub async fn connect(root: &Path) -> Result<Rmux, String> {
    let pin = RmuxPin::load()?;
    let report =
        rmux::ensure(&pin, false).map_err(|e| format!("{e}; run `oma check` first"))?;
    if let Some(dir) = report.layout.dispatcher.parent() {
        prepend_path(dir);
    }
    rmuxpoc::connect_dedicated(&report, endpoint(root)).await
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

/// Lay out 1/2/4 panes and record the agent -> pane id manifest.
pub async fn spawn(rmux: &Rmux, root: &Path, plan: &SpawnPlan) -> Result<Manifest, String> {
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
        )
        .await?;
        handles.push(pane);
    }

    let mut m = Manifest {
        stub: plan.stub,
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
    Ok(m)
}

async fn split_spawn(
    pane: &Pane,
    dir: SplitDirection,
    argv: &[String],
    env: Vec<String>,
    title: &str,
) -> Result<Pane, String> {
    let mut builder = pane.split_with(dir).spawn(argv.to_vec()).title(title);
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
pub async fn status(rmux: &Rmux, root: &Path) -> Result<Vec<PaneStatus>, String> {
    let manifest = read_manifest(root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(rmux, name).await?;

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

/// Two-step send with the full guard chain: agent key policy, locate, then
/// text and Enter as separate sends (M001 / S005 iron rule).
pub async fn send(
    rmux: &Rmux,
    root: &Path,
    agent: &str,
    text: &str,
    confirm: Option<&str>,
) -> Result<(), String> {
    if text.contains('\n') || text.contains('\r') {
        return Err("multi-line text is not supported yet; send single-line tasks".into());
    }
    check_send_key(agent, "Enter")?;
    let manifest = read_manifest(root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
    let entry = manifest
        .agents
        .iter()
        .find(|a| a.name == agent)
        .ok_or_else(|| format!("agent {agent} not in this session"))?;
    let name = session_name(root)?;
    let session = rmuxpoc::reuse_only(rmux, name).await?;
    let pane = pane_for(&session, entry.pane_id).await?;

    let pid = rmuxpoc::running_pid(&pane).await?;
    let expected = if manifest.stub { "pwsh" } else { agent };
    let names = process_names(&[pid])?;
    let actual = expect_process(&names, pid, expected)?;

    pane.send_text(text.to_string())
        .await
        .map_err(|e| format!("send_text: {e}"))?;
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    println!("send.agent={agent}");
    println!("send.proc={actual}");
    println!("send.split=text+Enter");

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

/// Session-scoped cleanup: never the daemon-wide stop. The manifest goes
/// away with the session.
pub async fn cleanup(rmux: &Rmux, root: &Path) -> Result<bool, String> {
    let name = session_name(root)?;
    let existed = match rmuxpoc::reuse_only(rmux, name).await {
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

#[cfg(test)]
mod tests {
    use super::*;

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
            "oma-orch-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let m = Manifest {
            stub: true,
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
}
