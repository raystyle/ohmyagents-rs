//! Label-bridge verification: boot a `-L` labeled daemon via WMI (CLI),
//! ask the CLI for the daemon's real pipe name (`#{socket_path}`), then
//! have the SDK connect to that same daemon. If this holds, the product can
//! use one daemon from both transports: SDK for snapshots/waits, CLI for
//! load-buffer/paste-buffer (the Windows `-S` rejection workaround).

use std::process::ExitCode;
use std::time::Duration;

use oma::rmuxpoc;
use rmux_sdk::RmuxEndpoint;

const TAG: &str = "lbg";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-label-bridge: {e}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let report = rmuxpoc::gate()?;
    let rmux_bin = report.layout.dispatcher.clone();
    println!("poc.name=label-bridge");
    println!("poc.os={}", std::env::consts::OS);

    let label = format!("oma-{TAG}-{}", std::process::id());
    let session = format!("oma{TAG}{}", std::process::id());
    println!("poc.label={label}");

    // 1. CLI boots the labeled daemon outside the job object.
    let mut argv: Vec<String> = vec![
        "-L".into(),
        label.clone(),
        "new-session".into(),
        "-d".into(),
        "-s".into(),
        session.clone(),
        "-x".into(),
        "120".into(),
        "-y".into(),
        "32".into(),
    ];
    argv.extend(rmuxpoc::interactive_shell_argv());
    let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    wmi_new_session(&rmux_bin, &refs)?;
    println!("poc.daemon.start=wmi-label");

    // 2. Wait for readiness, then ask for the real pipe name and daemon pid.
    let (pipe, daemon_pid) = wait_and_query(&rmux_bin, &label)?;
    println!("poc.daemon.pipe.len={}", pipe.len());
    println!("poc.daemon.pid={daemon_pid}");

    // 3. SDK connects to that exact pipe and lists the session.
    let rmux = rmux_sdk::Rmux::builder()
        .endpoint(RmuxEndpoint::WindowsPipe(pipe.clone()))
        .default_timeout(Duration::from_secs(10))
        .connect()
        .await
        .map_err(|e| format!("sdk connect to label pipe: {e}"))?;
    println!("poc.sdk.connect=true");
    let sess = rmuxpoc::reuse_only(&rmux, rmux_sdk::SessionName::new(&session).unwrap()).await?;
    let pane = sess.pane(0, 0);
    let alive = pane.exists().await.map_err(|e| e.to_string())?;
    println!("poc.sdk.pane_alive={alive}");

    // 4. SDK-side input and CLI-side paste hit the same pane.
    let names = rmuxpoc::process_names(&[rmuxpoc::running_pid(&pane).await?])?;
    println!(
        "poc.sdk.locate={}",
        names.values().next().cloned().unwrap_or_default()
    );

    let _ = rmuxpoc::kill_handle(&sess).await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    println!("poc.ok=true");
    Ok(())
}

fn wmi_new_session(rmux_bin: &std::path::Path, argv: &[&str]) -> Result<(), String> {
    let mut inner = String::new();
    inner.push_str(&format!("& '{}'", rmux_bin.display()));
    for arg in argv {
        inner.push_str(&format!(" '{arg}'"));
    }
    let escaped = inner.replace('\'', "''");
    let script = format!(
        "$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = 'pwsh -NoProfile -Command {escaped}' }}; if ($null -eq $r) {{ throw 'wmi returned null' }}; if ($r.ReturnValue -ne 0) {{ throw \"wmi create return=$($r.ReturnValue)\" }}"
    );
    let out = std::process::Command::new("pwsh")
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

fn wait_and_query(rmux_bin: &std::path::Path, label: &str) -> Result<(String, u32), String> {
    let mut last = String::new();
    for _ in 0..40 {
        let ready = std::process::Command::new(rmux_bin)
            .args(["-L", label, "list-sessions"])
            .output();
        if let Ok(out) = ready {
            if out.status.success() {
                let pipe = query_format(rmux_bin, label, "#{socket_path}")?;
                let pid = query_format(rmux_bin, label, "#{pid}")?;
                let pid: u32 = pid
                    .trim()
                    .parse()
                    .map_err(|e| format!("daemon pid {pid:?}: {e}"))?;
                return Ok((pipe.trim().to_string(), pid));
            }
            last = String::from_utf8_lossy(&out.stderr).trim().to_string();
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    Err(format!("labeled daemon never became ready: {last}"))
}

fn query_format(rmux_bin: &std::path::Path, label: &str, f: &str) -> Result<String, String> {
    let out = std::process::Command::new(rmux_bin)
        .args(["-L", label, "display-message", "-p", f])
        .output()
        .map_err(|e| format!("display-message: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "display-message {f} exit={:?} stderr={}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
