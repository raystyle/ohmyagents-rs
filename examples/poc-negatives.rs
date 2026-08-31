//! negatives: forbidden paths must fail before doing damage. C-c to a codex
//! stub throws before the send; cleanup kills only its own session while a
//! sibling session and the daemon stay alive; the daemon-wide kill command
//! never appears in the product sources.

use std::path::PathBuf;
use std::process::ExitCode;

use oma::rmuxpoc;
use rmux_sdk::Session;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-negatives: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=negatives");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("neg").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("neg")?;
    println!("poc.session={}", name.as_str());

    let session = rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::interactive_shell_argv())
        .await?;
    let result = negatives_inner(&rmux, &session).await;
    // Same cleanup rule as every POC: session-scoped kill only.
    let _ = rmuxpoc::kill_handle(&session).await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn negatives_inner(rmux: &rmux_sdk::Rmux, session: &Session) -> Result<(), String> {
    // 1. C-c to codex throws before the send and leaves the pane alive.
    let pane = session.pane(0, 0);
    let pid = rmuxpoc::running_pid(&pane).await?;
    println!("poc.pane.pid={pid}");

    match guarded_send_key(&pane, pid, "codex", "C-c").await {
        Err(e) => {
            println!("poc.negatives.c_c_codex.throw=true");
            println!("poc.negatives.c_c_codex.reason={e}");
        }
        Ok(()) => return Err("C-c to codex must throw before sending".into()),
    }
    // The stub survived: same pid, pane still listed.
    let pid_after = rmuxpoc::running_pid(&pane).await?;
    if pid_after != pid {
        return Err(format!("pane process changed {pid} -> {pid_after}; guard leaked a send"));
    }
    println!("poc.negatives.c_c_codex.survived=true");

    // Same key is legal for another agent; the guard is per-agent policy.
    let names = rmuxpoc::process_names(&[pid])?;
    let actual = rmuxpoc::expect_process(&names, pid, "pwsh")?;
    println!("poc.negatives.locate.proc={actual}");
    rmuxpoc::check_send_key("claude", "C-c")?;
    println!("poc.negatives.c_c_claude.allowed=true");
    // And codex still accepts non-interrupt keys.
    rmuxpoc::check_send_key("codex", "Enter")?;
    println!("poc.negatives.enter_codex.allowed=true");

    // 2. Cleanup kills only its own session: a sibling session on the same
    //    dedicated daemon must keep living after ours is killed.
    let sibling_name = rmuxpoc::poc_session_name("ng2")?;
    let sibling =
        rmuxpoc::create_only(rmux, sibling_name.clone(), rmuxpoc::keep_alive_echo("SIBLING"))
            .await?;
    let self_killed = rmuxpoc::kill_handle(session).await?;
    println!("poc.negatives.self_killed={self_killed}");
    if session.exists().await.map_err(|e| e.to_string())? {
        return Err("own session still listed after kill".into());
    }
    if !sibling.exists().await.map_err(|e| e.to_string())? {
        return Err("sibling session died with ours: cleanup hit more than its session".into());
    }
    println!("poc.negatives.sibling_alive=true");
    // The daemon itself still answers while the sibling lives: a ReuseOnly
    // round trip needs a live transport.
    rmuxpoc::reuse_only(rmux, sibling_name.clone()).await?;
    println!("poc.negatives.daemon_alive=true");
    let _ = rmuxpoc::kill_handle(&sibling).await?;

    // 3. The daemon-wide kill command never appears in the product sources
    //    (P0005 acceptance: negatives must not join src\ success paths).
    //    Examples may state the rule in comments, hence src-only scanning.
    for file in source_files()? {
        let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        if text.contains('k') && text.contains("ill-server") {
            return Err(format!("daemon-wide kill leaked into {}", file.display()));
        }
    }
    println!("poc.negatives.no_kill_server=true");

    println!("poc.negatives=all-guarded");
    Ok(())
}

async fn guarded_send_key(
    pane: &rmux_sdk::Pane,
    pid: u32,
    agent: &str,
    key: &str,
) -> Result<(), String> {
    rmuxpoc::check_send_key(agent, key)?;
    let names = rmuxpoc::process_names(&[pid])?;
    rmuxpoc::expect_process(&names, pid, "pwsh")?;
    pane.send_key(key)
        .await
        .map_err(|e| format!("send_key {key}: {e}"))
}

fn source_files() -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir("src").map_err(|e| format!("src: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    Ok(out)
}
