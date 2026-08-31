//! CLI smoke tests for the read-only commands (R004 layer: integration).
//! Assertions stick to stable surfaces only: exit codes and marker lines.
//! Session commands (spawn/status/send/cleanup) need a live daemon and are
//! gated behind the manual acceptance run in P0006, not here.

use assert_cmd::Command;
use predicates::str::contains;

/// Unique per-call suffix: same-millisecond parallel tests must not share a
/// temp dir.
static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
        "oma-cli-doctor-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
fn init_full_deploys_hooks_skills_and_yolo() {
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-init-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    oma().args(["init", "--project"]).arg(&tmp)
        .assert()
        .success()
        .stdout(contains("init.scope=full"))
        .stdout(contains("init.hooks.wrote.count="));
    // The S015 matrix lands in the project, not the user home.
    for rel in [
        r".claude\settings.json",
        r".codex\hooks.json",
        r".grok\hooks\ohmyagents-state.json",
        r".agents\skills\ohmyagents\SKILL.md",
        r".kimi-code\skills\ohmyagents\SKILL.md",
        r"CLAUDE.md",
        r"AGENTS.md",
    ] {
        assert!(tmp.join(rel).exists(), "missing {rel}");
    }
    // Kimi has no project-level hook registration (S015): the config.toml
    // the yolo pass writes must carry no hooks table.
    let kimi_cfg = std::fs::read_to_string(tmp.join(".kimi-code").join("config.toml"))
        .unwrap_or_default();
    assert!(!kimi_cfg.contains("[[hooks]]"), "kimi config must stay hook-free");
    // --yolo narrows to keys only: no hook files.
    let tmp2 = std::env::temp_dir().join(format!(
        "oma-cli-init-yolo-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp2).unwrap();
    oma().args(["init", "--yolo", "--project"]).arg(&tmp2)
        .assert()
        .success()
        .stdout(contains("init.scope=yolo"))
        .stdout(contains("init.hooks=skipped"));
    assert!(tmp2.join(".claude").join("settings.json").exists());
    assert!(!tmp2.join(".codex").join("hooks.json").exists());
    let _ = std::fs::remove_dir_all(&tmp);
    let _ = std::fs::remove_dir_all(&tmp2);
}

#[test]
fn agents_install_unknown_name_fails_fast() {
    // Unknown agent is rejected before any network access: the error must
    // name the catalog's known agents.
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-install-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    oma().args(["agents", "install", "nope", "--root"]).arg(&tmp)
        .assert()
        .failure()
        .stdout(contains("install.nope.status=failed"))
        .stdout(contains("unknown agent nope"))
        .stderr(contains("failed to install"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn dies_send_without_a_session_fails_fast() {
    // No manifest means no session: send must fail with guidance instead
    // of starting a daemon or touching anything.
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-send-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
