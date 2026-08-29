//! Dedicated rmux endpoint. Never `RmuxEndpoint::Default`.
//!
//! Windows: named pipe `\\.\pipe\ohmyagents-poc-<pid>-ep`.
//! Linux/mac: unix socket under temp. This example is accepted on Windows first.

use std::process::ExitCode;
use std::time::Duration;

use oma::rmuxpoc;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-endpoint: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let report = rmuxpoc::gate()?;
    println!("poc.name=endpoint");
    println!("poc.os={}", std::env::consts::OS);
    println!("poc.rmux={}", report.layout.dispatcher.display());
    println!("poc.rmux.version={}", report.version);

    let planned = rmuxpoc::poc_endpoint("ep");
    rmuxpoc::assert_dedicated(&planned)?;
    println!("poc.endpoint.planned={}", rmuxpoc::endpoint_label(&planned));

    let rmux = rmuxpoc::connect("ep").await?;
    let live = rmux.endpoint();
    rmuxpoc::assert_dedicated(live)?;
    println!("poc.endpoint.live={}", rmuxpoc::endpoint_label(live));
    println!("poc.endpoint.default=false");

    let caps = rmux
        .capabilities()
        .await
        .map_err(|e| format!("capabilities: {e}"))?;
    println!("poc.caps.count={}", caps.len());
    for cap in caps.iter().take(8) {
        println!("poc.cap={cap}");
    }

    // Last session gone => dedicated daemon exits. Do not kill-server.
    let name = rmuxpoc::poc_session_name("ep")?;
    let session = rmuxpoc::create_only(&rmux, name.clone(), rmuxpoc::keep_alive_echo("EP")).await?;
    let _ = rmuxpoc::kill_handle(&session).await;
    println!("poc.session.cleanup={}", name.as_str());
    tokio::time::sleep(Duration::from_millis(50)).await;
    println!("poc.kill_server=false");
    println!("poc.ok=true");
    Ok(())
}
