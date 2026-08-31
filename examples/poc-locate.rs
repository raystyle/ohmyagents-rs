//! locate: pane pid -> OS process name. Mismatch throws before any send.

use std::collections::HashMap;
use std::process::ExitCode;
use std::time::Duration;

use oma::rmuxpoc;
use rmux_sdk::{Pane, SplitDirection};

/// A pid that cannot belong to a live process in this POC run.
const DEAD_PID: u32 = 4_000_000;
const GUARD_MARKER: &str = "OMA-LOCATE-GUARD";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-locate: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=locate");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("loc").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("loc")?;
    println!("poc.session={}", name.as_str());

    // Root is interactive so the positive guarded send has a live prompt.
    let session =
        rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::interactive_shell_argv()).await?;
    let root = session.pane(0, 0);
    let right = root
        .split_with(SplitDirection::Right)
        .spawn(rmuxpoc::keep_alive_echo("PANE1"))
        .await
        .map_err(|e| format!("split Right: {e}"))?;
    let down_left = root
        .split_with(SplitDirection::Down)
        .spawn(rmuxpoc::keep_alive_echo("PANE2"))
        .await
        .map_err(|e| format!("split Down from 0.0: {e}"))?;
    let down_right = right
        .split_with(SplitDirection::Down)
        .spawn(rmuxpoc::keep_alive_echo("PANE3"))
        .await
        .map_err(|e| format!("split Down from right: {e}"))?;

    let panes = [
        ("root", &root),
        ("right", &right),
        ("down-left", &down_left),
        ("down-right", &down_right),
    ];
    let mut pids = Vec::new();
    for (label, pane) in panes {
        if !pane.exists().await.map_err(|e| e.to_string())? {
            return Err(format!("{label} pane missing"));
        }
        let pid = rmuxpoc::running_pid(pane).await?;
        println!("poc.pane.{label}.pid={pid}");
        pids.push(pid);
    }
    println!("poc.layout=2x2");

    let result = locate_inner(&root, &right, &pids).await;
    let _ = session.kill().await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn locate_inner(root: &Pane, right: &Pane, pids: &[u32]) -> Result<(), String> {
    // One OS query for all live pids plus a provably dead one.
    let mut query = pids.to_vec();
    query.push(DEAD_PID);
    let names = rmuxpoc::process_names(&query)?;
    println!("poc.lookup=os({})", std::env::consts::OS);

    // Happy path: every pane's pid resolves to the spawned stub shell.
    for pid in pids {
        let actual = rmuxpoc::expect_process(&names, *pid, "pwsh")?;
        println!("poc.pid.{pid}.proc={actual}");
    }

    // Dead pid: lookup must throw, never silently fall back.
    match rmuxpoc::expect_process(&names, DEAD_PID, "pwsh") {
        Err(e) => {
            println!("poc.deadpid.throw=true");
            println!("poc.deadpid.reason={e}");
        }
        Ok(actual) => return Err(format!("dead pid probe must throw, resolved to {actual}")),
    }

    // Name mismatch: must throw before any send, not warn-and-continue.
    let live = pids[0];
    match rmuxpoc::expect_process(&names, live, "notepad") {
        Err(e) => {
            println!("poc.mismatch.throw=true");
            println!("poc.mismatch.reason={e}");
        }
        Ok(actual) => return Err(format!("mismatch probe must throw, got {actual}")),
    }

    // Guarded send, positive: guard passes, Enter reaches the prompt.
    let root_pid = rmuxpoc::running_pid(root).await?;
    root.send_text(format!("echo {GUARD_MARKER}"))
        .await
        .map_err(|e| format!("send_text: {e}"))?;
    send_key_checked(root, &names, root_pid, "pwsh", "Enter").await?;
    root.expect_visible_text()
        .to_contain(GUARD_MARKER)
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("visible text missing {GUARD_MARKER}: {e}"))?;
    println!("poc.guard.send={GUARD_MARKER}");

    // Guarded send, negative: wrong expectation throws, nothing is sent.
    let right_pid = rmuxpoc::running_pid(right).await?;
    match send_key_checked(right, &names, right_pid, "claude", "Enter").await {
        Err(_) => println!("poc.guard.throw=true"),
        Ok(()) => return Err("guarded send with wrong process must throw".into()),
    }

    println!("poc.locate=pid-to-name");
    Ok(())
}

/// Guard lives in front of every key send: locate first, send only on match.
async fn send_key_checked(
    pane: &Pane,
    names: &HashMap<u32, String>,
    pid: u32,
    expected: &str,
    key: &str,
) -> Result<(), String> {
    rmuxpoc::expect_process(names, pid, expected)?;
    pane.send_key(key)
        .await
        .map_err(|e| format!("send_key {key}: {e}"))
}
