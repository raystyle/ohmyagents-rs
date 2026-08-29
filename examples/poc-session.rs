//! CreateOnly session: duplicate fails, ReuseOnly attaches, cleanup is kill-session only.

use std::process::ExitCode;

use oma::rmuxpoc;
use rmux_sdk::EnsureSession;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-session: {e}");
            if rmuxpoc::is_job_object_error(&e) {
                eprintln!("poc.skip=job-object");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let _report = rmuxpoc::gate()?;
    println!("poc.name=session");
    println!("poc.os={}", std::env::consts::OS);

    let rmux = rmuxpoc::connect("sess").await?;
    rmuxpoc::assert_dedicated(rmux.endpoint())?;

    let primary = rmuxpoc::poc_session_name("a")?;
    let keeper = rmuxpoc::poc_session_name("b")?;
    println!("poc.session.primary={}", primary.as_str());
    println!("poc.session.keeper={}", keeper.as_str());

    let created =
        rmuxpoc::create_only(&rmux, primary.clone(), rmuxpoc::keep_alive_echo("SESS-A")).await?;
    if !created.was_created() {
        let _ = created.kill().await;
        return Err("CreateOnly did not report was_created".into());
    }
    if !created.exists().await.map_err(|e| e.to_string())? {
        let _ = created.kill().await;
        return Err("CreateOnly session does not exist".into());
    }
    println!("poc.create_only=ok");

    let dup = rmux
        .ensure_session(EnsureSession::named(primary.clone()).create_only())
        .await;
    match dup {
        Ok(session) => {
            let _ = session.kill().await;
            let _ = created.kill().await;
            return Err("second CreateOnly succeeded; expected duplicate error".into());
        }
        Err(e) => {
            println!("poc.create_only.duplicate=error");
            println!("poc.create_only.duplicate.err={e}");
        }
    }

    let reused = match rmuxpoc::reuse_only(&rmux, primary.clone()).await {
        Ok(s) => s,
        Err(e) => {
            let _ = created.kill().await;
            return Err(e);
        }
    };
    if reused.was_created() {
        let _ = created.kill().await;
        return Err("ReuseOnly reported was_created".into());
    }
    println!("poc.reuse_only=ok");

    let keep = match rmuxpoc::create_only(&rmux, keeper.clone(), rmuxpoc::keep_alive_echo("SESS-B"))
        .await
    {
        Ok(s) => s,
        Err(e) => {
            let _ = rmuxpoc::kill_handle(&created).await;
            return Err(e);
        }
    };
    println!("poc.keeper.created={}", keep.was_created());
    let keep_up = keep.exists().await.map_err(|e| e.to_string())?;
    println!("poc.keeper.exists_before_kill={keep_up}");
    if !keep_up {
        let _ = rmuxpoc::kill_handle(&created).await;
        return Err("keeper session missing before kill".into());
    }
    let before = keep.list_session_names().await.map_err(|e| e.to_string())?;
    println!(
        "poc.sessions.before={}",
        before
            .iter()
            .map(|n| n.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );

    let killed = rmuxpoc::kill_handle(&created).await?;
    println!("poc.kill_session.primary={killed}");
    println!("poc.kill_server=false");

    let primary_gone = match reused.exists().await {
        Ok(exists) => !exists,
        Err(e) if rmuxpoc::is_transport_closed(&e.to_string()) => true,
        Err(e) => {
            let _ = rmuxpoc::kill_handle(&keep).await;
            return Err(e.to_string());
        }
    };
    if !primary_gone {
        let _ = rmuxpoc::kill_handle(&keep).await;
        return Err("primary still exists after kill-session".into());
    }
    let keep_up = match keep.exists().await {
        Ok(exists) => exists,
        Err(e) if rmuxpoc::is_transport_closed(&e.to_string()) => false,
        Err(e) => return Err(e.to_string()),
    };
    if !keep_up {
        return Err(
            "keeper session disappeared; kill-session must not take the daemon with it".into(),
        );
    }
    println!("poc.keeper.alive=true");

    let names = keep.list_session_names().await.map_err(|e| e.to_string())?;
    let listed: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    println!("poc.sessions.after={}", listed.join(","));
    if listed.iter().any(|n| *n == primary.as_str()) {
        let _ = rmuxpoc::kill_handle(&keep).await;
        return Err("primary still listed after kill-session".into());
    }
    if !listed.iter().any(|n| *n == keeper.as_str()) {
        let _ = rmuxpoc::kill_handle(&keep).await;
        return Err("keeper missing from list-sessions".into());
    }

    let _ = rmuxpoc::kill_handle(&keep).await;
    println!("poc.ok=true");
    Ok(())
}
