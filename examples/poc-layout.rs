//! 2x2 layout via `split_with` + argv. No empty-shell split then spawn.

use std::process::ExitCode;
use std::time::Duration;

use oma::rmuxpoc;
use rmux_sdk::SplitDirection;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-layout: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=layout");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("lay").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("lay")?;
    println!("poc.session={}", name.as_str());

    let session =
        rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::keep_alive_echo("PANE0")).await?;
    let root = session.pane(0, 0);
    let result = layout_inner(&root).await;
    let _ = session.kill().await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn layout_inner(root: &rmux_sdk::Pane) -> Result<(), String> {
    if !root.exists().await.map_err(|e| e.to_string())? {
        return Err("root pane 0.0 missing".into());
    }

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
        ("0.0", root, "PANE0"),
        ("right", &right, "PANE1"),
        ("down-left", &down_left, "PANE2"),
        ("down-right", &down_right, "PANE3"),
    ];
    for (label, pane, marker) in panes {
        if !pane.exists().await.map_err(|e| e.to_string())? {
            return Err(format!("{label} pane missing"));
        }
        let pid = rmuxpoc::running_pid(pane).await?;
        println!("poc.pane.{label}.pid={pid}");
        pane.expect_visible_text()
            .to_contain(marker)
            .timeout(Duration::from_secs(15))
            .await
            .map_err(|e| format!("{label} missing {marker}: {e}"))?;
        println!("poc.pane.{label}.marker={marker}");
    }
    println!("poc.layout=2x2");
    Ok(())
}
