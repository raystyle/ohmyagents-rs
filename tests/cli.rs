//! CLI smoke tests for the read-only commands (R004 layer: integration).
//! Assertions stick to stable surfaces only: exit codes and marker lines.
//! Session commands (spawn/status/send/cleanup) need a live daemon and are
//! gated behind the manual acceptance run in P0006, not here.

use assert_cmd::Command;
use predicates::str::contains;

fn oma() -> Command {
    Command::cargo_bin("oma").unwrap()
}

#[test]
fn check_reports_layout_and_pin() {
    oma().args(["check", "--no-install"])
        .assert()
        .success()
        .stdout(contains("rmux.ok=true"))
        .stdout(contains("rmux.source="))
        .stdout(contains("rmux.version=0.10.0"));
}

#[test]
fn agents_lists_detection_lines() {
    oma().args(["agents"])
        .assert()
        .success()
        .stdout(contains("agent=claude"))
        .stdout(contains("agent=codex"))
        .stdout(contains("agent=grok"))
        .stdout(contains("agent=kimi"));
}

#[test]
fn hook_is_silent_without_state_env() {
    // Outside an oma session there is no OHMYAGENTS_STATE_FILE: the hook
    // entry must stay silent and exit 0 (never fail the agent session).
    oma().args(["hook", "blocked"])
        .env_remove("OHMYAGENTS_STATE_FILE")
        .assert()
        .success();
}

#[test]
fn doctor_blocks_on_a_fresh_project_and_says_so() {
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-doctor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    // A fresh project has no yolo keys: doctor exits 1 by contract.
    oma().args(["doctor", "--project"])
        .arg(&tmp)
        .assert()
        .failure()
        .stdout(contains("doctor."));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn dies_send_without_a_session_fails_fast() {
    // No manifest means no session: send must fail with guidance instead
    // of starting a daemon or touching anything.
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-send-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    oma().args([
        "send",
        "claude",
        "hello",
        "--project",
    ])
    .arg(&tmp)
    .assert()
    .failure()
    .stderr(contains("no session manifest"));
    let _ = std::fs::remove_dir_all(&tmp);
}
