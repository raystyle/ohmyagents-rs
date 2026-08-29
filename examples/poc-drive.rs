//! Drive: `send_text` then `send_key("Enter")`. Text must not carry a newline.

use std::process::ExitCode;
use std::time::Duration;

use oma::rmuxpoc;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-drive: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=drive");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("drv").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;
    let name = rmuxpoc::poc_session_name("drv")?;
    println!("poc.session={}", name.as_str());

    let session =
        rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::interactive_shell_argv()).await?;
    let pane = session.pane(0, 0);
    let result = drive_inner(&pane).await;
    let _ = session.kill().await;
    println!("poc.kill_session=true");
    println!("poc.kill_server=false");
    result?;
    println!("poc.ok=true");
    Ok(())
}

async fn drive_inner(pane: &rmux_sdk::Pane) -> Result<(), String> {
    if !pane.exists().await.map_err(|e| e.to_string())? {
        return Err("pane 0.0 missing".into());
    }

    const MARKER: &str = "OMA-POC-DRIVE";
    pane.send_text(format!("echo {MARKER}"))
        .await
        .map_err(|e| format!("send_text: {e}"))?;
    pane.send_key("Enter")
        .await
        .map_err(|e| format!("send_key Enter: {e}"))?;
    println!("poc.drive.split=send_text+Enter");

    pane.expect_visible_text()
        .to_contain(MARKER)
        .timeout(Duration::from_secs(15))
        .await
        .map_err(|e| format!("visible text missing {MARKER}: {e}"))?;
    println!("poc.drive.marker={MARKER}");
    Ok(())
}
