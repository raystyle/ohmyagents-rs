//! Diagnostic: dump each pane's SDK snapshot lines for a project session
//! (the live grid, which capture-pane cannot see for alt-screen TUIs).

use std::path::PathBuf;

use oma::orch;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let root = PathBuf::from(std::env::args().nth(1).expect("usage: poc-dump <project>"));
    match run(&root).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-dump: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run(root: &std::path::Path) -> Result<(), String> {
    let link = orch::connect(root, false).await?;
    let manifest = orch::read_manifest_for(root)
        .ok_or_else(|| "no manifest".to_string())?;
    let name = orch::session_name(root)?;
    let session = oma::rmuxpoc::reuse_only(&link.rmux, name).await?;
    for agent in &manifest.agents {
        let pane = oma::orch::pane_for_test(&session, agent.pane_id).await?;
        println!("=== {} (%{}) ===", agent.name, agent.pane_id);
        match pane.snapshot().await {
            Ok(snap) => {
                for (i, line) in snap.visible_lines().iter().enumerate() {
                    if !line.trim().is_empty() {
                        println!("{:02}|{}", i, line);
                    }
                }
                println!("-- cursor row={} col={} visible={}", snap.cursor.row, snap.cursor.col, snap.cursor.visible);
            }
            Err(e) => println!("snapshot error: {e}"),
        }
    }
    Ok(())
}
