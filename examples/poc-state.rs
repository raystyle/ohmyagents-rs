//! judge: state from terminal semantics. Proves Quiet is not idle, blocked
//! forms classify from the screen, and a silent hook falls back to 1b.

use std::time::Duration;

use oma::rmuxpoc::{self, TermState};
use rmux_sdk::Pane;

const SLEEP_MARKER: &str = "SLEEP_DONE";
const CONFIRM_ANSWER: &str = "ANS=y";
const PASSWORD_DONE: &str = "PW_DONE";

#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-state: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=state");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("sta").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("sta")?;
    println!("poc.session={}", name.as_str());

    let session =
        rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::interactive_shell_argv()).await?;
    let pane = session.pane(0, 0);
    let result = state_inner(&pane).await;
    let _ = session.kill().await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn state_inner(pane: &Pane) -> Result<(), String> {
    if !pane.exists().await.map_err(|e| e.to_string())? {
        return Err("pane 0.0 missing".into());
    }

    // Layer 0 first: pane alive via pid, else the whole verdict is void.
    let pid = rmuxpoc::running_pid(pane).await?;
    println!("poc.layer0.alive.pid={pid}");

    // Layer 2 is silent in this POC: no hook ever writes a state file, so
    // every verdict below comes from the 1b terminal-semantic fallback.
    println!("poc.judge.hook=silent");
    println!("poc.judge.fallback=terminal_state");

    // 1. Ready: caret parked on a live prompt -> idle, may send.
    wait_for_state(pane, TermState::Ready, "initial prompt").await?;
    println!("poc.state.ready={}", TermState::Ready.oma_state());

    // 2. Quiet is not idle: screen static while a command runs.
    send_line(
        pane,
        format!("Start-Sleep -Seconds 8; Write-Host {SLEEP_MARKER}"),
    )
    .await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let quiet = pane
        .wait_until_stable_for(Duration::from_millis(1200))
        .timeout(Duration::from_secs(10))
        .await
        .map_err(|e| format!("wait quiet: {e}"))?;
    println!("poc.quiet.stable=true");
    println!("poc.quiet.revision={}", quiet.revision);
    let ts = classify(pane).await?;
    if ts == TermState::Ready {
        return Err(format!("Quiet misread as idle while pid {pid} still works"));
    }
    println!("poc.quiet.not_idle=true");
    println!("poc.quiet.verdict={}", ts.oma_state());
    println!("poc.quiet.policy=drive-sync-only");

    // Command finishes; verdict returns to idle through the same channel.
    pane.expect_visible_text()
        .to_contain(SLEEP_MARKER)
        .timeout(Duration::from_secs(20))
        .await
        .map_err(|e| format!("visible text missing {SLEEP_MARKER}: {e}"))?;
    wait_for_state(pane, TermState::Ready, "prompt after sleep").await?;
    println!("poc.state.resume={}", TermState::Ready.oma_state());

    // 3. Confirm form: tail keyword -> blocked, then answer via Drive.
    send_line(
        pane,
        format!(
            "Write-Host 'Allow this action? [y/n]'; $r = Read-Host; Write-Host '{CONFIRM_ANSWER}'"
        ),
    )
    .await?;
    wait_for_state(pane, TermState::Confirm, "confirm prompt").await?;
    println!("poc.state.confirm={}", TermState::Confirm.oma_state());
    send_line(pane, "y".into()).await?;
    pane.expect_visible_text()
        .to_contain(CONFIRM_ANSWER)
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("visible text missing {CONFIRM_ANSWER}: {e}"))?;
    println!("poc.confirm.answered=y");
    wait_for_state(pane, TermState::Ready, "prompt after confirm").await?;

    // 4. Password form: tail keyword -> blocked; never type into it. The
    // only key sent afterwards is a bare Enter to let Read-Host return.
    send_line(
        pane,
        "Write-Host 'Password:'; $null = Read-Host -AsSecureString; Write-Host PW_DONE".into(),
    )
    .await?;
    wait_for_state(pane, TermState::Password, "password prompt").await?;
    println!("poc.state.password={}", TermState::Password.oma_state());
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    println!("poc.password.typed=nothing");
    pane.expect_visible_text()
        .to_contain(PASSWORD_DONE)
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("visible text missing {PASSWORD_DONE}: {e}"))?;
    wait_for_state(pane, TermState::Ready, "prompt after password").await?;

    println!("poc.state=terminal-semantics");
    Ok(())
}

async fn send_line(pane: &Pane, text: String) -> Result<(), String> {
    pane.send_text(text)
        .await
        .map_err(|e| format!("send_text: {e}"))?;
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    Ok(())
}

async fn classify(pane: &Pane) -> Result<TermState, String> {
    let snap = pane
        .snapshot()
        .await
        .map_err(|e| format!("snapshot: {e}"))?;
    Ok(rmuxpoc::classify_snapshot(&snap))
}

/// Poll snapshots until the classifier returns the wanted state.
async fn wait_for_state(pane: &Pane, want: TermState, what: &str) -> Result<(), String> {
    for _ in 0..100 {
        if classify(pane).await? == want {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    Err(format!(
        "state {want:?} ({what}) not reached within deadline"
    ))
}
