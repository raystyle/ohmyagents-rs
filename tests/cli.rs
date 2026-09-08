//! CLI smoke tests for the read-only / deploy commands (R004 layer: integration).
//! Assertions stick to stable surfaces only: exit codes and marker lines.
//! D15 后本仓是纯部署配置工具：编排面（spawn/status/send/serve/mcp）
//! 与其 rmux 闸门测试已随删除面一并移除；trace（只读检索，与 rmux 无耦合）
//! 经 D19 全量恢复。

use assert_cmd::Command;
use predicates::str::contains;

/// Unique per-call suffix: same-millisecond parallel tests must not share a
/// temp dir.
static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn oma() -> Command {
    Command::cargo_bin("oma").unwrap()
}

#[test]
fn help_lists_the_deploy_surface() {
    // 命令面契约（D15 收窄、D19 恢复 trace）：帮助里是部署配置面加只读检索面。
    let out = oma()
        .args(["--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    for cmd in ["init", "doctor", "agents", "hook", "self", "completions", "trace"] {
        assert!(s.contains(cmd), "help must list {cmd}");
    }
}

#[test]
fn trace_sessions_on_empty_project_is_zero() {
    // A fresh temp project has no agent sessions: trace must exit 0 with a
    // zero count (read-only federation over the native session stores).
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-trace-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    oma()
        .args(["trace", "sessions", "--project"])
        .arg(&tmp)
        .assert()
        .success()
        .stdout(contains("trace.sessions.count=0"));
    oma()
        .args(["trace", "timeline", "--project"])
        .arg(&tmp)
        .assert()
        .success()
        .stdout(contains("trace.edits.count=0"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn agents_lists_detection_lines() {
    oma()
        .args(["agents"])
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
    oma()
        .args(["hook", "blocked"])
        .env_remove("OHMYAGENTS_STATE_FILE")
        .assert()
        .success();
}

#[test]
fn hook_secret_guard_blocks_with_exit_2() {
    // S030：PreToolUse 命中 block 级密钥 → exit 2（agent 侧拒工具调用）。
    // token 运行时拼接构造，测试源码不落字面密钥（防线 5）。
    let tok = format!("{}{}", "ghp_", "abcdefghijklmnopqrstuvwxyz0123456789");
    let payload = format!(
        "{{\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{{\"command\":\"curl -H bearauth:{tok} https://x\"}}}}"
    );
    oma()
        .args(["hook", "--agent", "claude"])
        .env_remove("OHMYAGENTS_STATE_FILE")
        .env_remove("OHMYAGENTS_AGENT")
        .write_stdin(payload)
        .assert()
        .code(2)
        .stderr(predicates::str::contains("secretguard"));
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
    // CPU 能力段恒在（S021）：agent=cpu check=caps。
    oma()
        .args(["doctor", "--project"])
        .arg(&tmp)
        .assert()
        .failure()
        .stdout(contains("doctor."))
        .stdout(contains("check=caps"))
        .stdout(contains("avx2="));
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
    oma()
        .args(["init", "--project"])
        .arg(&tmp)
        .assert()
        .success()
        .stdout(contains("init.scope=full"))
        .stdout(contains("init.hooks.wrote.count="))
        .stdout(contains("init.hooks.form="));
    // claude registration shape: exactly one oma handler per event, a
    // single command-line string (Grok imports this file; exec form plus
    // args is ParserError on Windows PowerShell, M047), and the command
    // is bare or host-absolute — never a POSIX path left by another OS's
    // writer (P0027 shared-dir guard).
    let settings: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(tmp.join(".claude").join("settings.json")).unwrap(),
    )
    .unwrap();
    for (_event, groups) in settings["hooks"].as_object().unwrap() {
        let ours: Vec<&serde_json::Value> = groups
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter(|h| h["command"].as_str().is_some_and(|c| c.contains("oma")))
            .collect();
        assert_eq!(ours.len(), 1, "one oma handler per event");
        assert!(
            ours[0].get("args").is_none(),
            "Grok PowerShell ParserError if command is the exe and args follow"
        );
        assert!(ours[0]["command"]
            .as_str()
            .unwrap()
            .contains("hook --agent claude"));
        assert_eq!(ours[0]["timeout"], 10);
        let cmd = ours[0]["command"].as_str().unwrap();
        assert!(
            !cmd.contains("/mnt/"),
            "foreign-OS path must not survive: {cmd}"
        );
    }
    // The S015 matrix lands in the project, not the user home.
    // 相对路径用正斜杠：Windows 文件 API 同样接受，两平台通用。
    for rel in [
        ".claude/settings.json",
        ".codex/hooks.json",
        ".grok/hooks/ohmyagents-state.json",
        ".agents/skills/ohmyagents/SKILL.md",
        ".kimi-code/skills/ohmyagents/SKILL.md",
        "CLAUDE.md",
        "AGENTS.md",
    ] {
        assert!(tmp.join(rel).exists(), "missing {rel}");
    }
    // Kimi has no project-level hook registration (S015): the config.toml
    // the yolo pass writes must carry no hooks table.
    let kimi_cfg =
        std::fs::read_to_string(tmp.join(".kimi-code").join("config.toml")).unwrap_or_default();
    assert!(
        !kimi_cfg.contains("[[hooks]]"),
        "kimi config must stay hook-free"
    );
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
    oma()
        .args(["init", "--yolo", "--project"])
        .arg(&tmp2)
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
fn init_rerun_is_byte_idempotent() {
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-init-idem-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let rels = [
        ".claude/settings.json",
        ".codex/hooks.json",
        ".grok/hooks/ohmyagents-state.json",
    ];
    let read_all = |tmp: &std::path::Path| -> Vec<String> {
        rels.iter()
            .map(|r| std::fs::read_to_string(tmp.join(r)).unwrap())
            .collect()
    };
    oma()
        .args(["init", "--project"])
        .arg(&tmp)
        .assert()
        .success();
    let after_first = read_all(&tmp);
    // Second run rewrites nothing: the hook registrations converge.
    oma()
        .args(["init", "--project"])
        .arg(&tmp)
        .assert()
        .success()
        .stdout(contains("init.hooks.wrote.count=0"));
    assert_eq!(read_all(&tmp), after_first, "rerun must be byte-identical");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn dies_statusline_unknown_agent() {
    // Unknown names fail before any home config is touched.
    oma()
        .args(["agents", "statusline", "no-such-agent"])
        .assert()
        .failure()
        .stderr(contains("claude/codex/kimi/grok"));
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
    oma()
        .args(["agents", "install", "nope", "--root"])
        .arg(&tmp)
        .assert()
        .failure()
        .stdout(contains("install.nope.status=failed"))
        .stdout(contains("unknown agent nope"))
        .stderr(contains("failed to install"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn agents_install_prints_deprecation_notice_pointing_to_ome() {
    // D07 迁册（ohmyagents#5）：install 入口先打 deprecated 提示指向 ome install；
    // 提示走 stderr，stdout 的 kv 输出面（R011）不受污染。未知名触网前快败。
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-deprec-install-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    oma()
        .args(["agents", "install", "nope", "--root"])
        .arg(&tmp)
        .assert()
        .failure()
        .stderr(contains("oma.deprecated"))
        .stderr(contains("ome install"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn agents_update_prints_deprecation_notice_pointing_to_ome() {
    // D07 迁册：update 入口同口径提示（升级通道语义由 ome 裁决）；
    // 未知名在解析最新版前快败，不触网。
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-deprec-update-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    oma()
        .args(["agents", "update", "nope", "--root"])
        .arg(&tmp)
        .assert()
        .failure()
        .stderr(contains("oma.deprecated"))
        .stderr(contains("ome install"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn completions_emit_shell_scripts() {
    for shell in ["bash", "powershell"] {
        let out = oma()
            .args(["completions", shell])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let s = String::from_utf8_lossy(&out);
        assert!(
            !s.is_empty() && s.contains("oma"),
            "{shell} script mentions oma"
        );
    }
    let out = oma()
        .args(["completions", "bash"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert!(String::from_utf8_lossy(&out).contains("_oma"));
}

// ===== Agent 友好 IO 契约（issue #1，与 ome S003 同构）=====

#[test]
fn format_json_doctor_envelope_parses_and_blocked_exits_one() {
    let tmp = std::env::temp_dir().join(format!(
        "oma-fmt-doctor-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = oma()
        .args(["--format", "json", "doctor", "--project"])
        .arg(&tmp)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("envelope parses");
    assert_eq!(v["ok"], serde_json::json!(true));
    assert_eq!(v["data"]["blocked"], serde_json::json!(true));
    assert!(v["data"]["findings"].as_array().unwrap().len() > 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn format_jsonl_agents_rows_each_parse() {
    let out = oma()
        .args(["--format", "jsonl", "agents"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 1, "至少一路 agent 行");
    for l in &lines {
        let v: serde_json::Value = serde_json::from_str(l).expect("jsonl 逐行可解析");
        assert!(v.get("agent").is_some(), "行带 agent 字段：{l}");
    }
    // 字段序契约（preserve_order）：installed 行首键 agent、次键 status。
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let keys: Vec<String> = first
        .as_object()
        .unwrap()
        .keys()
        .map(String::clone)
        .collect();
    assert_eq!(keys.first().map(String::as_str), Some("agent"));
}

#[test]
fn json_shorthand_works_after_subcommand() {
    let tmp = std::env::temp_dir().join(format!(
        "oma-fmt-sh-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let out = oma()
        .args(["doctor", "--json", "--project"])
        .arg(&tmp)
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_slice(&out).expect("--json 简写出信封");
    assert_eq!(v["data"]["blocked"], serde_json::json!(true));
    let _ = std::fs::remove_dir_all(&tmp);
}

// ===== `oma agents verify`（D17 无头验收）=====

#[test]
fn dies_verify_unknown_agent() {
    // 未知名在任何验收动作前快败，报支持面。
    oma()
        .args(["agents", "verify", "no-such-agent"])
        .assert()
        .failure()
        .stderr(contains("claude/codex/grok/kimi"));
}

/// 全 skip 环境的 env 罩子：PATH 空目录加 OMA_HOME 空目录，摘掉全部 agent 指引
/// 环境变量。默认目录源（~/.local/bin 等）无法罩住（dirs 走系统 API 不看
/// env），所以调用方要先自检仍检出 installed 就 skip。
fn verify_empty_env(cmd: &mut Command, sandbox: &std::path::Path) {
    cmd.env("PATH", sandbox.join("empty-path"))
        .env("OMA_HOME", sandbox.join("empty-oma-home"));
    for key in [
        "OMA_AGENT_PATH",
        "CODEX_HOME",
        "OMA_CLAUDE_BIN",
        "CLAUDE_BIN",
        "OMA_CODEX_BIN",
        "CODEX_BIN",
        "OMA_GROK_BIN",
        "GROK_BIN",
        "OMA_KIMI_BIN",
        "KIMI_BIN",
        "KIMI_CODE_BIN",
    ] {
        cmd.env_remove(key);
    }
}

#[test]
fn verify_all_skip_exits_zero_when_no_agents_detected() {
    let tmp = std::env::temp_dir().join(format!(
        "oma-cli-verify-skip-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(tmp.join("empty-path")).unwrap();
    std::fs::create_dir_all(tmp.join("empty-oma-home")).unwrap();
    // 自检：罩子下仍有 agent 检出（默认目录源），本机造不出全缺，skip。
    let mut probe = oma();
    verify_empty_env(&mut probe, &tmp);
    let out = probe.args(["agents"]).assert().success().get_output().stdout.clone();
    if String::from_utf8_lossy(&out).contains("status=installed") {
        eprintln!("skip: default-dir agent installs survive the env sandbox on this host");
        let _ = std::fs::remove_dir_all(&tmp);
        return;
    }
    let mut cmd = oma();
    verify_empty_env(&mut cmd, &tmp);
    cmd.args(["agents", "verify"])
        .assert()
        .success()
        .stdout(contains("verify.claude=skip"))
        .stdout(contains("verify.codex=skip"))
        .stdout(contains("verify.grok=skip"))
        .stdout(contains("verify.kimi=skip"))
        .stdout(contains("verify.ok=true"));
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn verify_live_headless_acceptance_for_installed_agents() {
    // 闸门（R004）：依赖真 agent 二进制与登录态，消耗极少量真实 token；
    // binary 不在则 eprintln skip 并 return。判据只押 hook state 落盘
    // （SessionStart/UserPromptSubmit 先于模型调用，S033）。
    let out = oma().args(["agents"]).assert().success().get_output().stdout.clone();
    let text = String::from_utf8_lossy(&out).into_owned();
    let mut ran = 0;
    for name in ["claude", "codex", "grok", "kimi"] {
        let installed = text
            .lines()
            .any(|l| l.starts_with(&format!("agent={name} ")) && l.contains("status=installed"));
        if !installed {
            eprintln!("skip: {name} not installed");
            continue;
        }
        ran += 1;
        oma()
            .args(["agents", "verify", name, "--timeout", "90"])
            .assert()
            .success()
            .stdout(contains(format!("verify.{name}.hook=ok")));
    }
    if ran == 0 {
        eprintln!("skip: no agent binaries on this host");
    }
}

#[test]
fn json_and_format_are_mutually_exclusive() {
    oma()
        .args(["--json", "--format", "json", "agents"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn structured_error_goes_to_stderr_as_single_line_json() {
    // oma 契约（与 ome 裸数据的分道点）：结构化模式 stderr 单行 JSON 错误行
    // （人称与机器双通道），退出码非 0。
    let out = oma()
        .args(["--format", "json", "agents", "statusline", "no-such-agent"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "stderr 单行：{err}");
    let v: serde_json::Value = serde_json::from_str(lines[0]).expect("stderr 单行 JSON");
    assert_eq!(v["code"], serde_json::json!("error"));
    assert!(v["message"].as_str().is_some());
}
