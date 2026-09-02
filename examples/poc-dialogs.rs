//! Residual dialog: hook writes blocked, then sendkeys click through.
//!
//! Hook reports via `.ohmyagents/state` (not an rmux pipe). Drive uses
//! `send_text` / `send_key` against a fake Allow prompt.

use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use oma::hook;
use oma::rmuxpoc;
use serde_json::Value as Json;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-dialogs: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=dialogs");
    println!("poc.os={}", std::env::consts::OS);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let root =
        std::env::temp_dir().join(format!("oma-poc-dialogs-{}-{}", std::process::id(), stamp));
    std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    println!("poc.project={}", root.display());

    let rmux = rmuxpoc::connect("dlg").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("dlg")?;
    println!("poc.session={}", name.as_str());

    let session = rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::fake_dialog_argv()).await?;
    let pane = session.pane(0, 0);
    let result = dialogs_inner(&pane, &root).await;
    let _ = session.kill().await;
    let _ = std::fs::remove_dir_all(&root);
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn dialogs_inner(pane: &rmux_sdk::Pane, root: &std::path::Path) -> Result<(), String> {
    pane.expect_visible_text()
        .to_contain("Allow this action?")
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("fake dialog not visible: {e}"))?;
    println!("poc.dialog.visible=true");

    let state_file = rmuxpoc::state_path(root, "claude");
    std::env::set_var("OHMYAGENTS_STATE_FILE", &state_file);
    std::env::set_var("OHMYAGENTS_AGENT", "claude");
    std::env::set_var("OHMYAGENTS_PROJECT", root);
    let wrote = hook::run(Some("PermissionRequest"), None)?
        .state_file
        .ok_or_else(|| "oma hook wrote nothing".to_string())?;
    std::env::remove_var("OHMYAGENTS_STATE_FILE");
    std::env::remove_var("OHMYAGENTS_AGENT");
    std::env::remove_var("OHMYAGENTS_PROJECT");
    println!("poc.hook.wrote={}", wrote.display());

    let body = std::fs::read_to_string(&wrote).map_err(|e| format!("{}: {e}", wrote.display()))?;
    let v: Json = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    if v["state"] != "blocked" {
        return Err(format!("expected blocked, got {}", v["state"]));
    }
    println!("poc.hook.state=blocked");
    println!("poc.channel=state-file");

    pane.send_text("y")
        .await
        .map_err(|e| format!("send_text y: {e}"))?;
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    println!("poc.sendkeys=y+Enter");

    pane.expect_visible_text()
        .to_contain("ALLOWED")
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("dialog not dismissed: {e}"))?;
    println!("poc.dialog.cleared=true");

    std::env::set_var("OHMYAGENTS_STATE_FILE", &state_file);
    std::env::set_var("OHMYAGENTS_AGENT", "claude");
    let _ = hook::run(Some("Stop"), None)?;
    std::env::remove_var("OHMYAGENTS_STATE_FILE");
    std::env::remove_var("OHMYAGENTS_AGENT");
    let after = std::fs::read_to_string(&state_file)
        .map_err(|e| format!("{}: {e}", state_file.display()))?;
    let v2: Json = serde_json::from_str(&after).map_err(|e| e.to_string())?;
    if v2["state"] != "idle" {
        return Err(format!("expected idle after Stop, got {}", v2["state"]));
    }
    println!("poc.hook.state.after=idle");
    Ok(())
}
