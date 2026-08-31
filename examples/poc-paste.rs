//! Paste: `load-buffer` + `paste-buffer -p` with a CJK payload over the rmux
//! CLI on a dedicated `-L` label endpoint. The sender never wraps the payload
//! in bracketed-paste escapes; the daemon owns them.
//!
//! Why pure CLI: rmux 0.10.0 on Windows rejects every `-S` form (verified
//! against the pinned binary), so the SDK `cmd` escape hatch -- which injects
//! `-S <pipe>` -- cannot work here, and `-L` pipe names carry a random salt
//! the SDK cannot pre-connect to. The label keeps the endpoint dedicated:
//! label-less `-L` is the per-user default namespace, ours carries pid+tag.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Output};
use std::time::Duration;

use oma::rmuxpoc;

/// Only the executed output line renders this exact sequence; the echoed
/// input line carries a split form, so a match proves the paste ran.
const MARKER: &str = "奥马粘贴成功";
const TAG: &str = "pst";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-paste: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let report = rmuxpoc::gate()?;
    let rmux_bin = report.layout.dispatcher.clone();
    println!("poc.name=paste");
    println!("poc.os={}", std::env::consts::OS);

    let label = format!("oma-poc-{TAG}-{}", std::process::id());
    let session = format!("omapoc{}{TAG}", std::process::id());
    println!("poc.label={label}");
    println!("poc.session={session}");

    let buffer = format!("oma-poc-paste-{}", std::process::id());
    let file = write_payload_file()?;

    let result = paste_flow(&rmux_bin, &label, &session, &buffer, &file);

    let _ = run_cli(
        &rmux_bin,
        &["-L", label.as_str(), "kill-session", "-t", session.as_str()],
    );
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");

    result?;
    let _ = std::fs::remove_file(&file);
    println!("poc.ok=true");
    Ok(())
}

fn paste_flow(
    rmux_bin: &Path,
    label: &str,
    session: &str,
    buffer: &str,
    file: &Path,
) -> Result<(), String> {
    let target = format!("{session}:0.0");

    // Interactive shell pane; the daemon boots outside the caller's job
    // object (Windows refuses in-job breakaway, same trap as the SDK route).
    // `start-server` alone would exit immediately (empty server), so the
    // first WMI command is `new-session` itself: daemon plus keeper session.
    let mut argv: Vec<&str> = vec![
        "-L", label, "new-session", "-d", "-s", session, "-x", "120", "-y", "32",
    ];
    let shell: Vec<String> = rmuxpoc::interactive_shell_argv();
    let shell_refs: Vec<&str> = shell.iter().map(String::as_str).collect();
    argv.extend(shell_refs);
    if cfg!(windows) {
        new_session_outside_job(rmux_bin, &argv)?;
        println!("poc.daemon.start=wmi-label-new-session");
        // The WMI launcher returns before the daemon binds its label pipe;
        // wait until the local client can see the server.
        let mut last = String::new();
        let mut ready = false;
        for _ in 0..40 {
            match run_cli_checked(&rmux_bin, &["-L", label, "list-sessions"], "list-sessions") {
                Ok(()) => {
                    ready = true;
                    break;
                }
                Err(e) => {
                    last = e;
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        }
        if !ready {
            return Err(format!("labeled daemon never became ready: {last}"));
        }
    } else {
        run_cli_checked(&rmux_bin, &argv, "new-session")?;
    }

    // One CJK command line, no newline, no ESC. Bracketed-paste wrappers are
    // the daemon's job (`paste-buffer -p`), never the sender's.
    assert!(
        !payload_line().contains('\u{1b}'),
        "sender must not wrap bracketed-paste escapes"
    );

    let path = file.display().to_string();
    run_cli_checked(
        &rmux_bin,
        &["-L", label, "load-buffer", "-b", buffer, &path],
        "load-buffer",
    )?;
    run_cli_checked(
        &rmux_bin,
        &["-L", label, "paste-buffer", "-p", "-b", buffer, "-t", &target],
        "paste-buffer",
    )?;
    println!("poc.paste.split=load-buffer+paste-buffer-p");

    // Enter is a separate dispatch; the pasted line carries no newline.
    run_cli_checked(
        &rmux_bin,
        &["-L", label, "send-keys", "-t", &target, "Enter"],
        "send-keys",
    )?;

    wait_marker(&rmux_bin, label, &target)?;
    println!("poc.paste.marker={MARKER}");

    run_cli_checked(
        &rmux_bin,
        &["-L", label, "delete-buffer", "-b", buffer],
        "delete-buffer",
    )?;
    Ok(())
}

/// Create the session (booting the labeled daemon) via WMI so the daemon is
/// not a child of this process's Windows job object. The launcher pwsh waits
/// for the command; the daemon survives it.
fn new_session_outside_job(rmux_bin: &Path, argv: &[&str]) -> Result<(), String> {
    let mut inner = String::new();
    inner.push_str(&format!("& '{}'", rmux_bin.display()));
    for arg in argv {
        inner.push_str(&format!(" '{}'", arg));
    }
    let escaped = inner.replace('\'', "''");
    let script = format!(
        "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = 'pwsh -NoProfile -Command {escaped}' }}; if ($null -eq $r) {{ throw 'wmi returned null' }}; if ($r.ReturnValue -ne 0) {{ throw \"wmi create return=$($r.ReturnValue)\" }}"
    );
    let out = Command::new("pwsh")
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

fn payload_line() -> String {
    if cfg!(windows) {
        format!("Write-Host ('奥' + '马粘贴成功')")
    } else {
        format!("printf '%s\\n' '奥''马粘贴成功'")
    }
}

fn write_payload_file() -> Result<PathBuf, String> {
    let path = std::env::temp_dir().join(format!("oma-poc-paste-{}.txt", std::process::id()));
    std::fs::write(&path, payload_line()).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

fn wait_marker(rmux_bin: &Path, label: &str, target: &str) -> Result<(), String> {
    for _ in 0..80 {
        let out = run_cli(
            &rmux_bin,
            &["-L", label, "capture-pane", "-p", "-t", target],
        )?;
        if out.status.success()
            && String::from_utf8_lossy(&out.stdout).contains(MARKER)
        {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(format!("pane never showed {MARKER} within 20s"))
}

fn run_cli(rmux_bin: &Path, args: &[&str]) -> Result<Output, String> {
    Command::new(rmux_bin)
        .args(args)
        .output()
        .map_err(|e| format!("spawn {}: {e}", rmux_bin.display()))
}

fn run_cli_checked(rmux_bin: &Path, args: &[&str], label: &str) -> Result<(), String> {
    let out = run_cli(rmux_bin, args)?;
    if !out.status.success() {
        return Err(format!(
            "{label} exit={:?} stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}
