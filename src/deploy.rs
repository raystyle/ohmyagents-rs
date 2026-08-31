//! `oma init` hook/skill deployment. Project-level files only, never the
//! user home. Schemas are first-hand verified in S015 (official docs +
//! openai/codex, xai-org/grok-build, MoonshotAI/kimi-code sources).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};

use crate::yolo::{ensure_parent, read_json, read_toml, toml_write, write_json, write_text};

pub struct DeployReport {
    pub wrote: Vec<String>,
    pub skipped: Vec<String>,
}

/// oma-owned handler marker: the current exe, or a stale oma binary whose
/// entry should be replaced (path moved between builds). Matches the bare
/// name `oma`, `oma.exe`, and test-harness binaries like `oma-<hash>.exe`.
fn oma_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("oma"))
}

fn is_ours(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if lower.contains(&oma_exe().display().to_string().to_ascii_lowercase()) {
        return true;
    }
    // First whitespace token covers both the exec form (bare path) and the
    // Grok shell form (`"C:\...\oma.exe" hook`).
    let first = lower.split_whitespace().next().unwrap_or("");
    let stem = first
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .trim_end_matches(".exe");
    stem == "oma" || stem.starts_with("oma-")
}

/// JSON arrays of handler groups under settings["hooks"][event], append-only:
/// drop stale oma entries, keep foreign ones, add ours exactly once.
fn merge_hook_event(
    settings: &mut Json,
    event: &str,
    our_handler: Json,
) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let groups = obj
        .entry("hooks".to_string())
        .or_insert_with(|| json!({}));
    if !groups.is_object() {
        *groups = json!({});
    }
    let groups = groups
        .as_object_mut()
        .ok_or_else(|| "hooks is not an object".to_string())?;
    let entry = groups
        .entry(event.to_string())
        .or_insert_with(|| json!([]));
    if !entry.is_array() {
        *entry = json!([]);
    }
    let arr = entry
        .as_array_mut()
        .ok_or_else(|| "event groups is not an array".to_string())?;
    let mut changed = false;
    // Drop stale oma handlers (a different oma path is embedded) so
    // redeploy stays single-entry. Bare names ("oma") and commands that
    // already reference the current exe are kept.
    let current = oma_exe().display().to_string().to_ascii_lowercase();
    let stale = |c: &str| -> bool {
        let c = c.to_ascii_lowercase();
        is_ours(&c) && (c.contains('\\') || c.contains('/')) && !c.contains(&current)
    };
    for group in arr.iter_mut() {
        if let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        {
            let before = hooks.len();
            hooks.retain(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| !stale(c))
                    .unwrap_or(true)
            });
            if hooks.len() != before {
                changed = true;
            }
        }
    }
    arr.retain(|g| !g.as_object().is_some_and(|g| g.get("hooks").and_then(|h| h.as_array()).is_some_and(|a| a.is_empty())));
    let already = arr.iter().any(|g| {
        g.get("hooks")
            .and_then(|h| h.as_array())
            .is_some_and(|hs| {
                hs.iter().any(|h| {
                    h.get("command")
                        .and_then(|c| c.as_str())
                        .is_some_and(|c| is_ours(c))
                })
            })
    });
    if !already {
        arr.push(json!({ "matcher": "*", "hooks": [our_handler] }));
        changed = true;
    }
    Ok(changed)
}

fn claude_handler() -> Json {
    // Exec form: command must be a real executable; args carry "hook".
    json!({
        "type": "command",
        "command": oma_exe().display().to_string(),
        "args": ["hook"],
        "timeout": 10,
    })
}

fn codex_handler(session_end: bool) -> Json {
    // commandWindows is Codex's platform-specific command field (S015).
    json!({
        "type": "command",
        "command": "oma",
        "commandWindows": oma_exe().display().to_string(),
        "timeout": if session_end { 3 } else { 10 },
    })
}

fn grok_handler() -> Json {
    // Grok has a single command string (no args array); the runner has an
    // sh -c branch. Quote the exe path.
    json!({
        "type": "command",
        "command": format!("\"{}\" hook", oma_exe().display()),
        "timeout": 10,
    })
}

/// Claude: `.claude/settings.json`, events per S015 (incl. PermissionRequest).
fn deploy_claude(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let events = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "PermissionRequest",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let path = root.join(".claude").join("settings.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    for event in events {
        changed |= merge_hook_event(&mut settings, event, claude_handler())?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

/// Codex: project `.codex/hooks.json` (JSON layer; config.toml [hooks] is the
/// twin representation and both non-empty triggers a warning, so we use one).
/// Notification does not exist in Codex (S015).
fn deploy_codex(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let events = [
        ("SessionStart", false),
        ("UserPromptSubmit", false),
        ("PreToolUse", false),
        ("PermissionRequest", false),
        ("PostToolUse", false),
        ("Stop", false),
        ("SessionEnd", true),
    ];
    let path = root.join(".codex").join("hooks.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    for (event, session_end) in events {
        changed |= merge_hook_event(&mut settings, event, codex_handler(session_end))?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }

    // [features] hooks = true in project config.toml (win-rmux precedent).
    let cfg = root.join(".codex").join("config.toml");
    let mut toml = read_toml(&cfg)?;
    let table = match &mut toml {
        toml::Value::Table(t) => t,
        _ => return Err("codex config.toml is not a table".into()),
    };
    let features = table
        .entry("features".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let feats = match features {
        toml::Value::Table(t) => t,
        _ => return Err("codex [features] is not a table".into()),
    };
    if feats.get("hooks").and_then(|v| v.as_bool()) != Some(true) {
        feats.insert("hooks".into(), toml::Value::Boolean(true));
        toml_write(&cfg, &toml)?;
        report.wrote.push(cfg.display().to_string());
    } else {
        report.skipped.push(cfg.display().to_string());
    }
    Ok(())
}

/// Grok: `.grok/hooks/ohmyagents-state.json`, Claude-isomorphic JSON.
/// No PermissionRequest event exists (S015).
fn deploy_grok(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let events = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let path = root.join(".grok").join("hooks").join("ohmyagents-state.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    for event in events {
        changed |= merge_hook_event(&mut settings, event, grok_handler())?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

/// Kimi: no project-level hook registration exists (S015: local.toml schema
/// only accepts workspace.additional_dir). We only lay out the skill dir.
fn deploy_kimi(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let dir = root.join(".kimi-code").join("skills").join("ohmyagents");
    ensure_parent(&dir.join("SKILL.md"))?;
    report.skipped.push(dir.display().to_string());
    Ok(())
}

const SKILL_MD: &str = "---\nname: ohmyagents\ndescription: Oh My Agents 项目编排说明与状态通道\n---\n\n# Oh My Agents\n\n本项目会话由 oma 编排。agent 状态在 `.ohmyagents/state/`；委派与诊断经 oma CLI。\n";

const AGENTS_MD: &str = "# AGENTS\n\n本项目会话由 Oh My Agents（oma）编排：agent 状态写 `.ohmyagents/state/`，委派与诊断经 oma CLI。\n";

/// Skills: `.agents/skills/ohmyagents` is the source; Claude and Grok and
/// Kimi get copies (Claude does not scan .agents/skills, S008).
fn deploy_skills(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let source = root.join(".agents").join("skills").join("ohmyagents");
    let skill = source.join("SKILL.md");
    if !skill.exists() {
        write_text(&skill, SKILL_MD)?;
        report.wrote.push(skill.display().to_string());
    } else {
        report.skipped.push(skill.display().to_string());
    }
    for target in [".claude", ".grok", ".kimi-code"] {
        let copy = root.join(target).join("skills").join("ohmyagents").join("SKILL.md");
        if !copy.exists() {
            write_text(&copy, SKILL_MD)?;
            report.wrote.push(copy.display().to_string());
        } else {
            report.skipped.push(copy.display().to_string());
        }
    }
    Ok(())
}

/// AGENTS.md only when absent (never overwrite user content); CLAUDE.md is a
/// one-line @AGENTS.md include.
fn deploy_instructions(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let agents = root.join("AGENTS.md");
    if !agents.exists() {
        write_text(&agents, AGENTS_MD)?;
        report.wrote.push(agents.display().to_string());
    } else {
        report.skipped.push(agents.display().to_string());
    }
    let claude = root.join("CLAUDE.md");
    if !claude.exists() {
        write_text(&claude, "@AGENTS.md\n")?;
        report.wrote.push(claude.display().to_string());
    } else {
        report.skipped.push(claude.display().to_string());
    }
    Ok(())
}

/// Deploy the full project tree. Merge-only for hooks, idempotent, and it
/// never touches the user home.
pub fn apply_project_hooks(root: &Path) -> Result<DeployReport, String> {
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut report = DeployReport {
        wrote: Vec::new(),
        skipped: Vec::new(),
    };
    deploy_claude(&root, &mut report)?;
    deploy_codex(&root, &mut report)?;
    deploy_grok(&root, &mut report)?;
    deploy_kimi(&root, &mut report)?;
    deploy_skills(&root, &mut report)?;
    deploy_instructions(&root, &mut report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique per-call suffix: same-millisecond parallel tests must not
    /// share (and mutually delete) a temp dir.
    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn fresh_dir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-deploy-test-{tag}-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn deploys_merges_and_is_idempotent() {
        let root = fresh_dir("full");
        // Foreign hook and foreign skill must survive every deploy.
        let settings = root.join(".claude").join("settings.json");
        ensure_parent(&settings).unwrap();
        write_text(
            &settings,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "C:\\tools\\fmt.sh"}]}]}}"#,
        )
        .unwrap();
        write_text(&root.join("AGENTS.md"), "# 用户自己的说明\n").unwrap();

        let first = apply_project_hooks(&root).unwrap();
        assert!(first.wrote.iter().any(|p| p.ends_with("settings.json")));
        assert!(first.wrote.iter().any(|p| p.ends_with("hooks.json")));
        assert!(first
            .wrote
            .iter()
            .any(|p| p.ends_with("ohmyagents-state.json")));
        // User-owned AGENTS.md must not be rewritten.
        assert!(first.skipped.iter().any(|p| p.ends_with("AGENTS.md")));
        assert_eq!(
            fs::read_to_string(root.join("AGENTS.md")).unwrap(),
            "# 用户自己的说明\n"
        );

        let v: Json = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let stop = v["hooks"]["Stop"].as_array().unwrap();
        // Foreign group kept plus ours appended.
        assert!(stop
            .iter()
            .any(|g| g["hooks"][0]["command"].as_str() == Some("C:\\tools\\fmt.sh")));
        let ours = stop
            .iter()
            .find(|g| g["hooks"][0]["command"].as_str().unwrap().contains("oma"))
            .unwrap();
        assert_eq!(ours["hooks"][0]["args"], json!(["hook"]));
        assert!(v["hooks"]["PermissionRequest"].is_array());

        let codex: Json =
            serde_json::from_str(&fs::read_to_string(root.join(".codex").join("hooks.json")).unwrap())
                .unwrap();
        assert!(codex["hooks"]["SessionEnd"][0]["hooks"][0]["commandWindows"]
            .as_str()
            .unwrap()
            .contains("oma"));
        assert_eq!(codex["hooks"]["SessionEnd"][0]["hooks"][0]["timeout"], 3);
        assert!(codex["hooks"].get("Notification").is_none());
        let codex_toml =
            fs::read_to_string(root.join(".codex").join("config.toml")).unwrap();
        assert!(codex_toml.contains("hooks = true"));

        let grok: Json = serde_json::from_str(
            &fs::read_to_string(root.join(".grok").join("hooks").join("ohmyagents-state.json"))
                .unwrap(),
        )
        .unwrap();
        assert!(grok["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .ends_with(" hook"));
        assert!(grok["hooks"].get("PermissionRequest").is_none());

        // Kimi: skill dir only, no hook registration anywhere in the project.
        assert!(root.join(".kimi-code").join("skills").join("ohmyagents").join("SKILL.md").exists());
        assert!(!root.join(".kimi-code").join("config.toml").exists());

        // Skills copied to every family dir; CLAUDE.md include created.
        assert!(root.join(".agents").join("skills").join("ohmyagents").join("SKILL.md").exists());
        assert!(root.join(".grok").join("skills").join("ohmyagents").join("SKILL.md").exists());
        assert!(root.join(".claude").join("skills").join("ohmyagents").join("SKILL.md").exists());
        assert_eq!(fs::read_to_string(root.join("CLAUDE.md")).unwrap(), "@AGENTS.md\n");

        // Second deploy: nothing changes on disk.
        let before = fs::read_to_string(&settings).unwrap();
        let second = apply_project_hooks(&root).unwrap();
        assert!(second.wrote.is_empty(), "redeploy must write nothing: {:?}", second.wrote);
        assert_eq!(fs::read_to_string(&settings).unwrap(), before);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_oma_entries_are_replaced() {
        let root = fresh_dir("stale");
        let settings = root.join(".claude").join("settings.json");
        ensure_parent(&settings).unwrap();
        write_text(
            &settings,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "D:\\old\\oma.exe", "args": ["hook"]}]}]}}"#,
        )
        .unwrap();
        apply_project_hooks(&root).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let commands: Vec<&str> = v["hooks"]["Stop"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter_map(|h| h["command"].as_str())
            .collect();
        assert_eq!(commands.len(), 1, "stale entry must be replaced, got {commands:?}");
        assert!(!commands[0].contains("D:\\old"), "stale path must not survive");
        let _ = fs::remove_dir_all(&root);
    }
}
