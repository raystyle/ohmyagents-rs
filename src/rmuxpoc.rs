use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use rmux_sdk::{
    EnsureSession, InfoSnapshot, Pane, PaneId, PaneProcessState, ProcessCommandSpec, ProcessSpec,
    Rmux, RmuxEndpoint, Session, SessionName, TerminalSizeSpec,
};

use crate::catalog::RmuxPin;
use crate::rmux::{self, prepend_path};

/// Dedicated daemon endpoint. Never `RmuxEndpoint::Default`.
pub fn poc_endpoint(tag: &str) -> RmuxEndpoint {
    let pid = std::process::id();
    if cfg!(windows) {
        // CLI `-S` on Windows only accepts `\\.\pipe\rmux-...`. The name is
        // still dedicated (pid+tag), never the platform Default discovery pipe.
        RmuxEndpoint::WindowsPipe(format!(r"\\.\pipe\rmux-omapoc-{pid}-{tag}"))
    } else {
        let dir = std::env::temp_dir().join(format!("ohmyagents-poc-{pid}-{tag}"));
        let _ = std::fs::create_dir_all(&dir);
        RmuxEndpoint::UnixSocket(dir.join("socket"))
    }
}

pub fn endpoint_label(ep: &RmuxEndpoint) -> String {
    match ep {
        RmuxEndpoint::Default => "Default".into(),
        RmuxEndpoint::WindowsPipe(name) => format!("WindowsPipe:{name}"),
        RmuxEndpoint::UnixSocket(path) => format!("UnixSocket:{}", path.display()),
        other => format!("{other:?}"),
    }
}

pub fn assert_dedicated(ep: &RmuxEndpoint) -> Result<(), String> {
    if ep.is_default() {
        return Err("endpoint resolved to Default".into());
    }
    Ok(())
}

pub fn is_job_object_error(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("os error 5") || e.contains("access is denied")
}

pub fn is_transport_closed(err: &str) -> bool {
    let e = err.to_ascii_lowercase();
    e.contains("closed the transport") || e.contains("no server running")
}

pub fn poc_session_name(tag: &str) -> Result<SessionName, String> {
    let raw = format!("omapoc{}{tag}", std::process::id());
    SessionName::new(&raw).map_err(|e| e.to_string())
}

pub fn prepare_env() {
    std::env::set_var("RMUX_DISABLE_TINY_CLI", "1");
    std::env::set_var("TERM", "xterm-256color");
    std::env::remove_var("NO_COLOR");
}

/// Pin must already be installed (`oma check`). POCs do not download.
pub fn gate() -> Result<rmux::Report, String> {
    let pin = RmuxPin::load()?;
    match rmux::ensure(&pin, false) {
        Ok(report) => {
            if let Some(dir) = report.layout.dispatcher.parent() {
                prepend_path(dir);
            }
            Ok(report)
        }
        Err(e) => Err(format!("{e}; run `oma check` first")),
    }
}

async fn try_connect_or_start(endpoint: RmuxEndpoint) -> Result<Rmux, String> {
    Rmux::builder()
        .endpoint(endpoint)
        .default_timeout(Duration::from_secs(20))
        .connect_or_start()
        .await
        .map_err(|e| format!("connect_or_start: {e}"))
}

async fn try_connect(endpoint: RmuxEndpoint) -> Result<Rmux, String> {
    Rmux::builder()
        .endpoint(endpoint)
        .default_timeout(Duration::from_secs(20))
        .connect()
        .await
        .map_err(|e| format!("connect: {e}"))
}

/// Connect, starting a dedicated daemon. If this process is trapped in a
/// Windows Job Object (os error 5), start the helper via WMI so the daemon
/// is not a job child, then `connect()` without start.
pub async fn connect(tag: &str) -> Result<Rmux, String> {
    let report = gate()?;
    connect_with(&report, tag).await
}

pub async fn connect_with(report: &rmux::Report, tag: &str) -> Result<Rmux, String> {
    prepare_env();
    let endpoint = poc_endpoint(tag);
    match try_connect_or_start(endpoint.clone()).await {
        Ok(rmux) => {
            println!("poc.daemon.start=connect_or_start");
            Ok(rmux)
        }
        Err(e) if cfg!(windows) && is_job_object_error(&e) => {
            println!("poc.daemon.start=wmi-breakaway");
            let pid = start_daemon_outside_job(&report.layout.helper, &endpoint)?;
            println!("poc.daemon.wmi.pid={pid}");
            wait_connect(endpoint).await
        }
        Err(e) => Err(e),
    }
}

async fn wait_connect(endpoint: RmuxEndpoint) -> Result<Rmux, String> {
    let mut last = "connect never attempted".to_string();
    for _ in 0..40 {
        match try_connect(endpoint.clone()).await {
            Ok(rmux) => return Ok(rmux),
            Err(e) => last = e,
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Err(format!(
        "daemon did not accept connect after WMI start: {last}"
    ))
}

fn start_daemon_outside_job(helper: &Path, endpoint: &RmuxEndpoint) -> Result<u32, String> {
    if !cfg!(windows) {
        return Err("job-object breakaway is Windows-only".into());
    }
    let pipe = match endpoint {
        RmuxEndpoint::WindowsPipe(p) => p.clone(),
        _ => return Err("WMI start needs WindowsPipe".into()),
    };
    let cmdline = format!(
        "\"{}\" --__internal-daemon {} --config-default --config-quiet",
        helper.display(),
        pipe
    );
    let escaped = cmdline.replace('\'', "''");
    let script = format!(
        "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = '{escaped}' }}; if ($null -eq $r) {{ throw 'wmi returned null' }}; Write-Output $r.ReturnValue; Write-Output $r.ProcessId"
    );
    let out = Command::new(pwsh_bin())
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|e| format!("pwsh WMI: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() {
        return Err(format!(
            "WMI start failed: {} {}",
            stdout.trim(),
            stderr.trim()
        ));
    }
    let mut lines = stdout.lines().filter(|l| !l.trim().is_empty());
    let code = lines.next().unwrap_or("").trim();
    let pid = lines.next().unwrap_or("").trim();
    if code != "0" {
        return Err(format!(
            "Win32_Process.Create return={code} pid={pid} cmdline={cmdline}"
        ));
    }
    pid.parse::<u32>()
        .map_err(|_| format!("WMI pid not a number: {pid}"))
}

fn pwsh_bin() -> String {
    which::which("pwsh")
        .or_else(|_| which::which("pwsh.exe"))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "pwsh".into())
}

pub fn pwsh_keep_alive(inner: &str) -> Vec<String> {
    vec![
        pwsh_bin(),
        "-NoProfile".into(),
        "-Command".into(),
        format!("{inner}; Start-Sleep -Seconds 120"),
    ]
}

/// Marker stays on screen; process stays alive for locate/layout.
pub fn keep_alive_echo(marker: &str) -> Vec<String> {
    if cfg!(windows) {
        pwsh_keep_alive(&format!("Write-Host '{marker}'"))
    } else {
        vec![
            "sh".into(),
            "-c".into(),
            format!("printf '%s\\n' '{marker}'; sleep 120"),
        ]
    }
}

/// Interactive shell so `send_text` + Enter can run a command.
pub fn interactive_shell_argv() -> Vec<String> {
    if cfg!(windows) {
        vec![pwsh_bin(), "-NoProfile".into(), "-NoExit".into()]
    } else {
        vec!["sh".into()]
    }
}

/// Fake permission prompt. Typing `y` then Enter should print ALLOWED.
pub fn fake_dialog_argv() -> Vec<String> {
    if cfg!(windows) {
        pwsh_keep_alive(
            "Write-Host 'Allow this action? [y/n]'; $r = Read-Host; if ($r -match '^y') { Write-Host 'ALLOWED' } else { Write-Host 'DENIED' }",
        )
    } else {
        vec![
            "sh".into(),
            "-c".into(),
            "printf 'Allow this action? [y/n]\\n'; read r; case $r in y|Y) printf 'ALLOWED\\n';; *) printf 'DENIED\\n';; esac; sleep 120".into(),
        ]
    }
}

pub async fn create_only(
    rmux: &Rmux,
    name: SessionName,
    argv: Vec<String>,
) -> Result<Session, String> {
    let mut process = ProcessSpec::default();
    process.process_command = Some(ProcessCommandSpec::Argv(argv));
    rmux.ensure_session(
        EnsureSession::named(name)
            .create_only()
            .detached(true)
            .size(TerminalSizeSpec::new(120, 32))
            .process(process),
    )
    .await
    .map_err(|e| format!("ensure_session CreateOnly: {e}"))
}

pub async fn reuse_only(rmux: &Rmux, name: SessionName) -> Result<Session, String> {
    rmux.ensure_session(EnsureSession::named(name).reuse_only())
        .await
        .map_err(|e| format!("ensure_session ReuseOnly: {e}"))
}

pub async fn kill_session(rmux: &Rmux, name: &SessionName) -> Result<(), String> {
    let run = match rmux.cmd(["kill-session", "-t", name.as_str()]).await {
        Ok(run) => run,
        Err(e) => {
            let msg = e.to_string();
            if is_transport_closed(&msg) {
                return Ok(());
            }
            return Err(format!("kill-session: {msg}"));
        }
    };
    if run.exit.unwrap_or(1) != 0 {
        let err = String::from_utf8_lossy(&run.stderr);
        if err.contains("can't find session") || err.contains("no session") {
            return Ok(());
        }
        return Err(format!(
            "kill-session exit={:?} stderr={}",
            run.exit,
            err.trim()
        ));
    }
    Ok(())
}

pub async fn kill_handle(session: &Session) -> Result<bool, String> {
    match session.kill().await {
        Ok(existed) => Ok(existed),
        Err(e) => {
            let msg = e.to_string();
            if is_transport_closed(&msg) {
                Ok(true)
            } else {
                Err(format!("session.kill: {msg}"))
            }
        }
    }
}

pub fn pane_running_pid(info: &InfoSnapshot, pane_id: PaneId) -> Result<u32, String> {
    let pane = info
        .pane(pane_id)
        .or_else(|| info.panes.first())
        .ok_or_else(|| "info snapshot has no pane".to_string())?;
    match pane.process {
        PaneProcessState::Running { pid: Some(pid) } => Ok(pid),
        PaneProcessState::Running { pid: None } => Err("pane running but pid unknown".into()),
        ref other => Err(format!("pane not running: {other:?}")),
    }
}

pub async fn running_pid(pane: &Pane) -> Result<u32, String> {
    let id = pane
        .id()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "pane has no live id".to_string())?;
    let info = pane.info().await.map_err(|e| e.to_string())?;
    pane_running_pid(&info, id)
}

pub fn state_path(root: &std::path::Path, agent: &str) -> PathBuf {
    root.join(".ohmyagents")
        .join("state")
        .join(format!("{agent}.json"))
}

pub fn write_state(root: &std::path::Path, agent: &str, state: &str) -> Result<PathBuf, String> {
    let path = state_path(root, agent);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let body =
        format!("{{\"state\":\"{state}\",\"event\":\"poc-dialog\",\"agent\":\"{agent}\"}}\n");
    std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedicated_endpoint_is_never_default() {
        let ep = poc_endpoint("ep");
        assert!(!ep.is_default());
        assert_ne!(endpoint_label(&ep), "Default");
        assert_dedicated(&ep).unwrap();
        if cfg!(windows) {
            match ep {
                RmuxEndpoint::WindowsPipe(name) => {
                    assert!(
                        name.starts_with(r"\\.\pipe\rmux-omapoc-"),
                        "windows pipe must be explicit rmux- prefix: {name}"
                    );
                }
                other => panic!("expected WindowsPipe, got {other:?}"),
            }
        }
    }

    #[test]
    fn job_object_error_detects_os_error_5() {
        assert!(is_job_object_error("connect_or_start: os error 5"));
        assert!(is_job_object_error("Access is denied. (os error 5)"));
        assert!(!is_job_object_error("session not found"));
        assert!(is_transport_closed("rmux daemon closed the transport"));
    }
}
