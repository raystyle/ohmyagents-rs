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

/// Connect to an arbitrary dedicated endpoint, starting the daemon via WMI
/// when trapped in a Job Object. Shared by POCs and product orchestration.
pub async fn connect_dedicated(
    report: &rmux::Report,
    ep: RmuxEndpoint,
) -> Result<Rmux, String> {
    prepare_env();
    match try_connect_or_start(ep.clone()).await {
        Ok(rmux) => Ok(rmux),
        Err(e) if cfg!(windows) && is_job_object_error(&e) => {
            start_daemon_outside_job(&report.layout.helper, &ep)?;
            wait_connect(ep).await
        }
        Err(e) => Err(e),
    }
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

/// OS-side pid -> process name lookup. One query per batch; pids that are
/// dead (or never existed) simply do not appear in the map. The SDK only
/// surfaces `pid` (`PaneProcessState::Running`), never the process name.
pub fn process_names(pids: &[u32]) -> Result<std::collections::HashMap<u32, String>, String> {
    let mut names = std::collections::HashMap::new();
    if pids.is_empty() {
        return Ok(names);
    }
    if cfg!(windows) {
        let filter = pids
            .iter()
            .map(|p| format!("ProcessId={p}"))
            .collect::<Vec<_>>()
            .join(" OR ");
        let script = format!(
            "Get-CimInstance Win32_Process -Filter '{filter}' | ForEach-Object {{ '{{0}}={{1}}' -f $_.ProcessId, $_.Name }}"
        );
        let out = Command::new(pwsh_bin())
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("pwsh CIM: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "CIM query failed: {} {}",
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            if let Some((pid, name)) = line.split_once('=') {
                if let Ok(pid) = pid.trim().parse::<u32>() {
                    names.insert(pid, name.trim().to_string());
                }
            }
        }
    } else {
        let list = pids
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let out = Command::new("ps")
            .args(["-p", &list, "-o", "pid=,comm="])
            .output()
            .map_err(|e| format!("ps: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "ps query failed: {} {}",
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        for line in String::from_utf8_lossy(&out.stdout).lines() {
            let mut parts = line.split_whitespace();
            if let (Some(pid), Some(name)) = (parts.next(), parts.next()) {
                if let Ok(pid) = pid.parse::<u32>() {
                    names.insert(pid, name.to_string());
                }
            }
        }
    }
    Ok(names)
}

/// Locate guard: pane pid must map to the expected process. Dead pid and
/// name mismatch are both hard errors -- never warn-and-continue before a
/// send. Returns the actual name on success.
pub fn expect_process(
    names: &std::collections::HashMap<u32, String>,
    pid: u32,
    expected: &str,
) -> Result<String, String> {
    let actual = names
        .get(&pid)
        .ok_or_else(|| format!("pid {pid} not found (dead or recycled); expected {expected}"))?;
    if !actual.to_ascii_lowercase().contains(&expected.to_ascii_lowercase()) {
        return Err(format!(
            "pid {pid} is '{actual}', expected '{expected}' -- refusing to send"
        ));
    }
    Ok(actual.clone())
}

/// 1b terminal-semantic fallback states (S010). Maps onto the oma four:
/// Ready -> idle, Running -> working, Confirm/Password -> blocked,
/// Unknown -> unknown (never idle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermState {
    Ready,
    Running,
    Confirm,
    Password,
    Unknown,
}

impl TermState {
    /// oma four-state mapping; Quiet never appears here because Quiet is a
    /// Drive-sync signal, not a state source.
    pub fn oma_state(self) -> &'static str {
        match self {
            TermState::Ready => "idle",
            TermState::Running => "working",
            TermState::Confirm | TermState::Password => "blocked",
            TermState::Unknown => "unknown",
        }
    }
}

/// Keyword tables are English-only and tail-scoped, mirroring clum's
/// `terminal_state` gaps: Chinese prompts fall through to Unknown instead of
/// being misclassified (extend before production, see S010).
pub const PASSWORD_TAIL_KEYWORDS: &[&str] = &["password:", "[sudo] password", "passphrase:"];
pub const CONFIRM_TAIL_KEYWORDS: &[&str] =
    &["[y/n]", "(y/n)", "are you sure", "continue?", "proceed?"];

/// Minimal terminal-state classifier over a captured pane grid.
///
/// Priority mirrors clum `detect_terminal_state`: password (tail, wins even
/// at col 0) > confirm (tail) > hidden cursor means running > live shell
/// prompt (tail must be the prompt row with a visible cursor) > Unknown.
pub fn detect_terminal_state(
    lines: &[String],
    cursor_row: u16,
    cursor_visible: bool,
) -> TermState {
    // Tail = last non-empty line plus its row index.
    let tail = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, l.trim()))
        .next_back();
    let Some((tail_row, tail)) = tail else {
        return TermState::Unknown;
    };
    let lower = tail.to_lowercase();
    if PASSWORD_TAIL_KEYWORDS.iter().any(|k| lower.contains(k)) {
        return TermState::Password;
    }
    if CONFIRM_TAIL_KEYWORDS.iter().any(|k| lower.contains(k)) {
        return TermState::Confirm;
    }
    if !cursor_visible {
        // Mid-render or alternate screen: treat as working, never idle.
        return TermState::Running;
    }
    // pwsh -NoProfile prompt: "PS <path>>". Ready only when the visible
    // caret is parked on that prompt row.
    if tail.starts_with("PS ") && tail.ends_with('>') && cursor_row == tail_row as u16 {
        return TermState::Ready;
    }
    // Unix shell prompt（sh/bash/zsh）：PS1 缺省以 "$ " 收尾（root 为
    // "# "、zsh 为 "% "），trim 后提示符落在行尾；已输入命令的行（如
    // "$ sleep 30"）不以提示符收尾，天然不误判。裸 "%" 才算 zsh 提示符，
    // 后缀匹配会吃掉 "42%" 进度行。
    if (tail.ends_with('$') || tail.ends_with('#') || tail == "%")
        && cursor_row == tail_row as u16
    {
        return TermState::Ready;
    }
    TermState::Unknown
}

/// Classify a live snapshot.
pub fn classify_snapshot(snap: &rmux_sdk::PaneSnapshot) -> TermState {
    detect_terminal_state(&snap.visible_lines(), snap.cursor.row, snap.cursor.visible)
}

/// Drive-policy guard: some keys are poison for some agents. Codex runs
/// `--no-alt-screen`, so one C-c kills the process (M001); other agents may
/// be interrupted. Checked before any send, never warn-and-continue.
pub fn check_send_key(agent: &str, key: &str) -> Result<(), String> {
    let is_interrupt = key.eq_ignore_ascii_case("c-c") || key == "^C" || key == "\u{3}";
    if is_interrupt && agent.eq_ignore_ascii_case("codex") {
        return Err(format!(
            "refusing to send '{key}' to codex: one C-c kills the process (M001); retry with Enter or a task-level abort"
        ));
    }
    Ok(())
}

/// Run one rmux CLI invocation against a labeled daemon.
pub fn run_cli(rmux_bin: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new(rmux_bin)
        .args(args)
        .output()
        .map_err(|e| format!("spawn {}: {e}", rmux_bin.display()))
}

pub fn run_cli_checked(rmux_bin: &Path, args: &[&str], what: &str) -> Result<String, String> {
    let out = run_cli(rmux_bin, args)?;
    if !out.status.success() {
        return Err(format!(
            "{what} exit={:?} stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Is the labeled daemon answering yet?
pub fn label_alive(rmux_bin: &Path, label: &str) -> bool {
    run_cli(rmux_bin, &["-L", label, "list-sessions"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Ask a live labeled daemon for its real pipe name (`#{socket_path}`) and
/// pid. The name carries a random salt, so it must be queried, not derived.
pub fn label_socket_path(rmux_bin: &Path, label: &str) -> Result<(String, u32), String> {
    let pipe = run_cli_checked(
        rmux_bin,
        &["-L", label, "display-message", "-p", "#{socket_path}"],
        "display-message socket_path",
    )?;
    let pid = run_cli_checked(
        rmux_bin,
        &["-L", label, "display-message", "-p", "#{pid}"],
        "display-message pid",
    )?;
    let pid: u32 = pid
        .trim()
        .parse()
        .map_err(|e| format!("daemon pid {pid:?}: {e}"))?;
    Ok((pipe.trim().to_string(), pid))
}

/// Boot a labeled daemon outside the caller's Windows job object via WMI.
/// The boot command is `new-session -d` (a bare server would exit-empty).
pub fn wmi_new_session(rmux_bin: &Path, argv: &[String]) -> Result<(), String> {
    let mut inner = String::new();
    inner.push_str(&format!("& '{}'", rmux_bin.display()));
    for arg in argv {
        inner.push_str(&format!(" '{arg}'"));
    }
    let escaped = inner.replace('\'', "''");
    let script = format!(
        "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = 'pwsh -NoProfile -Command {escaped}' }}; if ($null -eq $r) {{ throw 'wmi returned null' }}; if ($r.ReturnValue -ne 0) {{ throw \"wmi create return=$($r.ReturnValue)\" }}"
    );
    let out = Command::new(pwsh_bin())
        .args(["-NoProfile", "-Command", &script])
        .output()
        .map_err(|e| format!("pwsh WMI: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "WMI new-session failed: {} {}",
            String::from_utf8_lossy(&out.stdout).trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Boot a labeled daemon outside the caller's lifetime.
/// Windows: WMI escapes the Job Object (kill-on-close would reap the daemon
/// with the parent). Unix: no job object exists and the tmux-shaped server
/// daemonizes itself, so a plain detached spawn suffices—null stdio plus a
/// fresh process group so a terminal SIGHUP cannot take the daemon down.
pub fn boot_new_session(rmux_bin: &Path, argv: &[String]) -> Result<(), String> {
    #[cfg(windows)]
    {
        wmi_new_session(rmux_bin, argv)
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        Command::new(rmux_bin)
            .args(argv)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("spawn new-session: {e}"))
    }
}

/// Boot (or reuse) a labeled daemon and return its real pipe name plus pid.
/// Boot keeper session must not collide with the product session name.
pub fn ensure_label_daemon(
    rmux_bin: &Path,
    label: &str,
    boot_session: &str,
) -> Result<(String, u32), String> {
    if !label_alive(rmux_bin, label) {
        let mut argv: Vec<String> = vec![
            "-L".into(),
            label.into(),
            "new-session".into(),
            "-d".into(),
            "-s".into(),
            boot_session.into(),
            "-x".into(),
            "120".into(),
            "-y".into(),
            "32".into(),
        ];
        argv.extend(interactive_shell_argv());
        boot_new_session(rmux_bin, &argv)?;
        let mut last = "never probed".to_string();
        for _ in 0..40 {
            if label_alive(rmux_bin, label) {
                break;
            }
            last = "daemon not answering yet".into();
            std::thread::sleep(Duration::from_millis(250));
        }
        if !label_alive(rmux_bin, label) {
            return Err(format!("labeled daemon {label} never became ready: {last}"));
        }
    }
    label_socket_path(rmux_bin, label)
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

    #[test]
    fn expect_process_throws_on_dead_pid_and_mismatch() {
        let mut names = std::collections::HashMap::new();
        names.insert(4242u32, "pwsh.exe".to_string());
        assert_eq!(expect_process(&names, 4242, "pwsh").unwrap(), "pwsh.exe");
        // Case-insensitive contains, so PWSH.EXE matches "pwsh".
        assert!(expect_process(&names, 4242, "PWsh").is_ok());
        // Dead / recycled pid: must throw, not map to some default.
        assert!(expect_process(&names, 4000000, "pwsh").is_err());
        // Name mismatch: must throw before any send.
        assert!(expect_process(&names, 4242, "notepad").is_err());
    }

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn classifier_password_wins_even_at_col_zero() {
        // clum P0 parity: password rule fires before any shell check.
        let l = lines(&["work", "[sudo] password for ray:"]);
        assert_eq!(detect_terminal_state(&l, 2, true), TermState::Password);
        assert_eq!(detect_terminal_state(&l, 2, false), TermState::Password);
    }

    #[test]
    fn classifier_confirm_and_chinese_gap() {
        let l = lines(&["Allow this action? [y/n]"]);
        assert_eq!(detect_terminal_state(&l, 0, true), TermState::Confirm);
        // Chinese text still classifies when an ASCII marker is present.
        let zh_mixed = lines(&["是否继续？(y/n)"]);
        assert_eq!(detect_terminal_state(&zh_mixed, 0, true), TermState::Confirm);
        // Known gap (S010): pure-Chinese confirm words are not in the keyword
        // tables and must fall through to Unknown, never Ready.
        let zh = lines(&["是否继续？"]);
        assert_eq!(detect_terminal_state(&zh, 0, true), TermState::Unknown);
        assert_ne!(detect_terminal_state(&zh, 0, true).oma_state(), "idle");
        let zh_pw = lines(&["密码："]);
        assert_eq!(detect_terminal_state(&zh_pw, 0, true), TermState::Unknown);
    }

    #[test]
    fn classifier_ready_needs_prompt_row_and_visible_cursor() {
        let l = lines(&["PS D:\\ohmyagents>"]);
        assert_eq!(detect_terminal_state(&l, 0, true), TermState::Ready);
        // Same prompt but the caret sits elsewhere: not Ready.
        assert_eq!(detect_terminal_state(&l, 5, true), TermState::Unknown);
        // Prompt text present but cursor hidden: mid-render, working.
        assert_eq!(detect_terminal_state(&l, 0, false), TermState::Running);
        // Typed command on the prompt line (mid-command): never Ready.
        let mid = lines(&["PS D:\\ohmyagents> Start-Sleep -Seconds 8"]);
        assert_eq!(detect_terminal_state(&mid, 0, true), TermState::Unknown);
    }

    #[test]
    fn classifier_ready_accepts_unix_shell_prompts() {
        // Linux stub 实测画面：裸 "$" 提示符整屏堆叠（resize 重印）。
        let bare = lines(&["$"]);
        assert_eq!(detect_terminal_state(&bare, 0, true), TermState::Ready);
        // 常见 PS1 形态：user@host:path$ 与 root 的 #。
        let bash = lines(&["ray@ai-lab:~/proj$"]);
        assert_eq!(detect_terminal_state(&bash, 0, true), TermState::Ready);
        let root = lines(&["root@ai-lab:~#"]);
        assert_eq!(detect_terminal_state(&root, 0, true), TermState::Ready);
        // 裸 "%" 是 zsh 提示符；后缀 "%" 会吃掉进度行，所以只认裸形态。
        let zsh = lines(&["%"]);
        assert_eq!(detect_terminal_state(&zsh, 0, true), TermState::Ready);
        let progress = lines(&["downloading 42%"]);
        assert_eq!(detect_terminal_state(&progress, 0, true), TermState::Unknown);
        // 已输入命令（"$ sleep 30"）不以提示符收尾：不 Ready。
        let mid = lines(&["$ sleep 30"]);
        assert_eq!(detect_terminal_state(&mid, 0, true), TermState::Unknown);
        // 光标不在提示符行：不 Ready。
        assert_eq!(detect_terminal_state(&bare, 5, true), TermState::Unknown);
    }

    #[test]
    fn classifier_hidden_cursor_is_running_and_empty_is_unknown() {
        let l = lines(&["some output"]);
        assert_eq!(detect_terminal_state(&l, 0, false), TermState::Running);
        let empty: Vec<String> = Vec::new();
        assert_eq!(detect_terminal_state(&empty, 0, true), TermState::Unknown);
    }

    #[test]
    fn send_key_guard_blocks_c_c_for_codex_only() {
        assert!(check_send_key("codex", "C-c").is_err());
        assert!(check_send_key("CODEX", "c-c").is_err());
        assert!(check_send_key("codex", "\u{3}").is_err());
        // Interrupts stay legal for other agents, and everything else is fine.
        assert!(check_send_key("claude", "C-c").is_ok());
        assert!(check_send_key("codex", "Enter").is_ok());
        assert!(check_send_key("kimi", "y").is_ok());
    }
}
