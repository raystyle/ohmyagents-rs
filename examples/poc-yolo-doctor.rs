//! Project-level yolo persistence + non-blocking prompt-block diagnosis.
//!
//! Uses a temp project. Does not write user-home trust stores unless
//! `OMA_POC_PRETRUST=1`. Does not spawn agents or attach to panes.

use std::fs;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use oma::doctor::{diagnose, print_diagnosis, Status};
use oma::yolo::apply_project_yolo;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-yolo-doctor: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let root = std::env::temp_dir().join(format!(
        "oma-poc-yolo-doctor-{}-{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    println!("poc.project={}", root.display());

    println!("poc.phase=before");
    let before = diagnose(&root)?;
    print_diagnosis(&before);
    for (agent, check) in [("claude", "yolo"), ("codex", "yolo")] {
        if before.status(agent, check) != Some(Status::Block) {
            let _ = fs::remove_dir_all(&root);
            return Err(format!(
                "expected {agent}/{check} block before yolo, got {:?}",
                before.status(agent, check)
            ));
        }
    }

    let wrote = apply_project_yolo(&root)?;
    for p in &wrote.wrote {
        println!("poc.wrote={p}");
    }

    println!("poc.phase=after-yolo");
    let after = diagnose(&root)?;
    print_diagnosis(&after);
    for (agent, check) in [
        ("claude", "yolo"),
        ("claude", "skip_prompt"),
        ("codex", "yolo"),
        ("kimi", "yolo"),
    ] {
        if after.status(agent, check) != Some(Status::Ok) {
            let _ = fs::remove_dir_all(&root);
            return Err(format!(
                "expected {agent}/{check} ok after yolo, got {:?}",
                after.status(agent, check)
            ));
        }
    }

    let trust_still_blocks = after
        .findings
        .iter()
        .any(|f| f.check.starts_with("trust.") && f.status == Status::Block);
    println!("poc.yolo_file_ok=true");
    println!("poc.trust_still_blocks={trust_still_blocks}");
    println!("poc.doctor.blocked={}", after.blocked());
    println!("poc.attach=false");

    if std::env::var_os("OMA_POC_PRETRUST").is_some() {
        let trust = oma::yolo::apply_pretrust(&root)?;
        for p in &trust.wrote {
            println!("poc.pretrust.wrote={p}");
        }
        let trusted = diagnose(&root)?;
        print_diagnosis(&trusted);
        println!("poc.phase=after-pretrust");
        println!("poc.doctor.blocked={}", trusted.blocked());
    } else {
        println!("poc.pretrust=skipped");
    }

    let _ = fs::remove_dir_all(&root);
    Ok(())
}
