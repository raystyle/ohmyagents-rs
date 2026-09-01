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
    // Update an existing oma handler in place (registration shape evolves:
    // bare name -> absolute path), or append when absent.
    let mut replaced = false;
    for group in arr.iter_mut() {
        let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        else {
            continue;
        };
        for handler in hooks.iter_mut() {
            let ours = handler
                .get("command")
                .and_then(|c| c.as_str())
                .map(|c| is_ours(c))
                .unwrap_or(false);
            if ours {
                if handler != &our_handler {
                    *handler = our_handler.clone();
                    changed = true;
                }
                replaced = true;
            }
        }
    }
    if !replaced {
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
    // codex's command is a full command line. On Windows codex 0.149 runs it
    // through PowerShell, where `"exe" hook` is a parse error: the call
    // operator `&` is required. The plain command stays sh-shaped for other
    // platforms; the hook exec environment never inherits our PATH, so the
    // exe is absolute either way.
    let exe = oma_exe().display().to_string();
    json!({
        "type": "command",
        "command": format!("\"{exe}\" hook"),
        "commandWindows": format!("& \"{exe}\" hook"),
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
///
/// Trust is pre-seeded by replicating codex's own identity scheme (S015
/// source): key `<config.toml abs>:<event_label>:<group>:<handler>`, hash
/// over the normalized handler identity (canonical key-sorted JSON, sha256).
/// Seeding the hash means the TUI never needs to prompt; the settle fallback
/// (auto-confirm dialogs) covers any drift between our replica and codex.
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
    let feature_missing = feats.get("hooks").and_then(|v| v.as_bool()) != Some(true);
    if feature_missing {
        feats.insert("hooks".into(), toml::Value::Boolean(true));
    }

    // Pre-seed [hooks.state."<key>"] trusted_hash for every oma handler in
    // the final hooks.json (real indices, not assumption zero).
    let final_hooks = read_json(&path)?;
    let entries = codex_trust_entries(&final_hooks, &cfg)?;
    let states = table
        .entry("hooks".to_string())
        .or_insert_with(|| {
            let mut hooks_tbl = toml::map::Map::new();
            hooks_tbl.insert("state".to_string(), toml::Value::Table(toml::map::Map::new()));
            toml::Value::Table(hooks_tbl)
        });
    let state_table = match states {
        toml::Value::Table(t) => t
            .entry("state".to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new())),
        _ => return Err("codex [hooks] is not a table".into()),
    };
    let state_map = match state_table {
        toml::Value::Table(t) => t,
        _ => return Err("codex [hooks.state] is not a table".into()),
    };
    let mut trust_changed = false;
    for (key, hash) in entries {
        let current = state_map
            .get(&key)
            .and_then(|v| v.get("trusted_hash"))
            .and_then(|v| v.as_str());
        if current != Some(hash.as_str()) {
            let mut m = toml::map::Map::new();
            m.insert("trusted_hash".into(), toml::Value::String(hash));
            state_map.insert(key, toml::Value::Table(m));
            trust_changed = true;
        }
    }
    if trust_changed || feature_missing {
        toml_write(&cfg, &toml)?;
        report.wrote.push(cfg.display().to_string());
    } else {
        report.skipped.push(cfg.display().to_string());
    }
    Ok(())
}

/// Strip the Windows canonicalization prefix codex never sees (`\\?\`),
/// because the trust key must match the path form codex derives from its own
/// project discovery.
fn plain_absolute(path: &Path) -> String {
    let s = path.display().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn event_label(event: &str) -> String {
    event.chars()
        .flat_map(|c| if c.is_ascii_uppercase() { vec!['_', c.to_ascii_lowercase()] } else { vec![c] })
        .collect::<String>()
        .trim_start_matches('_')
        .to_string()
}

/// codex matcher semantics (S015 source): these events ignore matchers, so
/// the hashed identity drops the key entirely (TOML drops nulls).
fn hashed_matcher(event: &str, matcher: Option<&str>) -> Option<String> {
    match event {
        "UserPromptSubmit" | "Stop" | "Interrupt" => None,
        _ => matcher.filter(|m| !m.is_empty()).map(String::from),
    }
}

fn canonical_json(value: &Json) -> Json {
    match value {
        Json::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                sorted.insert(key.clone(), canonical_json(&map[key]));
            }
            Json::Object(sorted)
        }
        Json::Array(items) => Json::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}

/// Replicate codex `hook_hash` (S015 source): identity over the normalized
/// handler (commandWindows dropped, timeout clamped per event), serialized
/// as canonical key-sorted JSON, sha256, `sha256:<hex>`.
fn codex_hook_hash(event: &str, matcher: Option<&str>, handler: &Json) -> Result<String, String> {
    let command = handler
        .get("command")
        .and_then(|c| c.as_str())
        .ok_or("handler missing command")?;
    let windows_cmd = handler.get("commandWindows").and_then(|c| c.as_str());
    let effective = if cfg!(windows) { windows_cmd.unwrap_or(command) } else { command };
    let timeout = handler.get("timeout").and_then(|t| t.as_u64());
    let timeout = match event {
        "SessionEnd" | "Interrupt" => timeout.unwrap_or(1).clamp(1, 3),
        _ => timeout.unwrap_or(600).max(1),
    };
    let r#async = handler.get("async").and_then(|a| a.as_bool()).unwrap_or(false);
    let mut entry = serde_json::Map::new();
    entry.insert("type".into(), json!("command"));
    entry.insert("command".into(), json!(effective));
    entry.insert("timeout".into(), json!(timeout));
    entry.insert("async".into(), json!(r#async));
    if let Some(sm) = handler.get("statusMessage").and_then(|s| s.as_str()) {
        entry.insert("statusMessage".into(), json!(sm));
    }
    if let Some(limit) = handler.get("additionalContextLimit").and_then(|s| s.as_u64()) {
        entry.insert("additionalContextLimit".into(), json!(limit));
    }

    let mut identity = serde_json::Map::new();
    identity.insert("event_name".into(), json!(event_label(event)));
    if let Some(m) = hashed_matcher(event, matcher) {
        identity.insert("matcher".into(), json!(m));
    }
    identity.insert("hooks".into(), Json::Array(vec![Json::Object(entry)]));

    let canonical = canonical_json(&Json::Object(identity));
    let bytes = serde_json::to_vec(&canonical).map_err(|e| e.to_string())?;
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{digest:x}"))
}

/// Walk the final hooks.json and produce (key, trusted_hash) pairs for every
/// oma-owned handler at its real group/handler indices.
fn codex_trust_entries(hooks_json: &Json, config_toml: &Path) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let Some(events) = hooks_json.get("hooks").and_then(|h| h.as_object()) else {
        return Ok(out);
    };
    let key_source = plain_absolute(config_toml);
    for (event, groups) in events {
        let Some(groups) = groups.as_array() else { continue };
        for (gi, group) in groups.iter().enumerate() {
            let matcher = group.get("matcher").and_then(|m| m.as_str());
            let Some(handlers) = group.get("hooks").and_then(|h| h.as_array()) else { continue };
            for (hi, handler) in handlers.iter().enumerate() {
                let command = handler.get("command").and_then(|c| c.as_str()).unwrap_or("");
                if !is_ours(command) {
                    continue;
                }
                let hash = codex_hook_hash(event, matcher, handler)?;
                let key = format!(
                    "{}:{}:{}:{}",
                    key_source,
                    event_label(event),
                    gi,
                    hi
                );
                out.push((key, hash));
            }
        }
    }
    Ok(out)
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

/// 命令图（S016「命令即 skill」）：意图到命令的映射，SKILL.md 由它生成。
/// 新增子命令在此补一行，`oma init` 重跑即同步（带生成标记才覆写）。
const COMMAND_MAP: &[(&str, &str)] = &[
    ("oma spawn [--agents a,b] [--stub]", "拉起或重连本项目多路 agent 会话（1-4 路；缺省已装交集）"),
    ("oma status", "看各路 pid、进程名、终端态、hook 态"),
    ("oma send <agent> \"<文本>\"", "向某路发任务（多行自动三段式粘贴）"),
    ("oma task <agent> \"<文本>\"", "带产物等待的任务委派：oma 阻塞等 DONE，产物在任务目录 output.md"),
    ("oma run \"<文本>\" [--assign a,b]", "状态门分派：闲路才发，忙路跳过不堵其它路"),
    ("oma settle [--wait N]", "自检测并自动确认信任/审查框"),
    ("oma cleanup", "只杀本会话（不动 daemon 与其它会话）"),
    ("oma trace sessions|timeline|blocks|agent|file|search", "检索本项目各 agent 的意图操作块与编辑轨迹（四家原生会话库联邦读）"),
    ("oma serve [--port N]", "起 HTTP 编排面（GET / 直出可视化网页）"),
    ("oma mcp", "作为 MCP server 跑 stdio（六操作加 trace 检索 tools）"),
    ("oma doctor", "只读诊断信任库、已装二进制与状态链"),
    ("oma agents install [名]", "安装缺失 agent（oma 自管根 ~/.ohmyagents）"),
];

/// 生成标记：只有带它的 SKILL.md 才允许 oma 覆写（用户手改过的跳过）。
const SKILL_MARKER: &str = "<!-- generated by oma init; rerun oma init to sync the command map -->";

/// 旧版静态 skill 全文：识别后升级为命令图生成版。
const LEGACY_SKILL_MD: &str = "---\nname: ohmyagents\ndescription: Oh My Agents 项目编排说明与状态通道\n---\n\n# Oh My Agents\n\n本项目会话由 oma 编排。agent 状态在 `.ohmyagents/state/`；委派与诊断经 oma CLI。\n";

fn skill_md() -> String {
    let mut s = String::new();
    s.push_str("---\nname: ohmyagents\ndescription: oma 项目编排命令图：会话拉起、状态、委派、自愈、轨迹检索\n---\n\n");
    s.push_str("# Oh My Agents 命令图\n\n");
    s.push_str(SKILL_MARKER);
    s.push_str("\n\n本项目会话由 oma 编排：agent 状态写 `.ohmyagents/state/`，会话清单在 `.ohmyagents/session.json`。\n\n");
    s.push_str("| 意图 | 命令 |\n| --- | --- |\n");
    for (cmd, intent) in COMMAND_MAP {
        s.push_str(&format!("| {intent} | `{cmd}` |\n"));
    }
    s.push_str("\n## 任务目录协议\n\n收到带「任务协议」尾注的委派时，按 `.ohmyagents/tasks/<id>/` 目录操作：\n\n1. 提示词全文在 `prompt.md`（可随时重读）；\n2. 产物写到 `output.md`（先写全内容）；\n3. **最后**创建空文件 `DONE` 表示完成——oma 只认 DONE 不认 output 存在，顺序不能反。\n\n裸 `oma` 进 REPL；六会话命令加 `--json` 出信封。细则见仓库 `docs\\references\\R002`。\n");
    s
}

/// 单点写入语义：缺文件写；旧静态版升级；带标记的同步覆写；无标记的用户内容跳过。
fn write_skill(path: &Path, report: &mut DeployReport) -> Result<(), String> {
    let generated = skill_md();
    match std::fs::read_to_string(path) {
        Ok(existing) => {
            if existing == generated {
                report.skipped.push(path.display().to_string());
            } else if existing.contains(SKILL_MARKER) || existing == LEGACY_SKILL_MD {
                write_text(path, &generated)?;
                report.wrote.push(format!("{} (regen)", path.display()));
            } else {
                report.skipped.push(format!("{} (user-owned)", path.display()));
            }
        }
        Err(_) => {
            write_text(path, &generated)?;
            report.wrote.push(path.display().to_string());
        }
    }
    Ok(())
}

const AGENTS_MD: &str = "# AGENTS\n\n本项目会话由 Oh My Agents（oma）编排：agent 状态写 `.ohmyagents/state/`，委派与诊断经 oma CLI。\n";

/// Skills: `.agents/skills/ohmyagents` is the source; Claude and Grok and
/// Kimi get copies (Claude does not scan .agents/skills, S008).
fn deploy_skills(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let source = root.join(".agents").join("skills").join("ohmyagents");
    let skill = source.join("SKILL.md");
    write_skill(&skill, report)?;
    for target in [".claude", ".grok", ".kimi-code"] {
        let copy = root.join(target).join("skills").join("ohmyagents").join("SKILL.md");
        write_skill(&copy, report)?;
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
    fn codex_trust_identity_shape_and_determinism() {
        assert_eq!(event_label("SessionStart"), "session_start");
        assert_eq!(event_label("UserPromptSubmit"), "user_prompt_submit");
        // These events drop the matcher from the hashed identity.
        assert_eq!(hashed_matcher("UserPromptSubmit", Some("*")), None);
        assert_eq!(hashed_matcher("PreToolUse", Some("*")), Some("*".into()));
        assert_eq!(hashed_matcher("Stop", Some("*")), None);

        let handler = serde_json::json!({
            "type": "command",
            "command": "oma",
            "commandWindows": "D:\\bin\\oma.exe",
            "timeout": 10,
            "async": false
        });
        let h1 = codex_hook_hash("PreToolUse", Some("*"), &handler).unwrap();
        let h2 = codex_hook_hash("PreToolUse", Some("*"), &handler).unwrap();
        assert_eq!(h1, h2, "hash must be deterministic");
        assert!(h1.starts_with("sha256:"), "{h1}");
        // Matcher participates for matcher-respecting events.
        let h3 = codex_hook_hash("PreToolUse", Some("Bash"), &handler).unwrap();
        assert_ne!(h1, h3);
        // Timeout clamps for SessionEnd.
        let se = codex_hook_hash("SessionEnd", None, &handler).unwrap();
        let se_clamped = codex_hook_hash(
            "SessionEnd",
            None,
            &serde_json::json!({"type":"command","command":"oma","timeout":99}),
        )
        .unwrap();
        // Both clamp to 3, so identical identity except async default: differ
        // only if fields differ; same fields -> same hash.
        let se_again = codex_hook_hash(
            "SessionEnd",
            None,
            &serde_json::json!({"type":"command","command":"oma","timeout":3,"async":false}),
        )
        .unwrap();
        assert_eq!(se_clamped, se_again, "clamped 99 and explicit 3 converge");
        assert_ne!(se, se_clamped, "10 vs 99 clamp differently from 3");
    }

    #[test]
    fn codex_trust_entries_use_real_indices_and_skip_foreign() {
        let hooks = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "Bash", "hooks": [
                        {"type": "command", "command": "C:\\tools\\fmt.sh"}
                    ]},
                    {"matcher": "*", "hooks": [
                        {"type": "command", "command": "oma",
                         "commandWindows": "D:\\oma.exe", "timeout": 10}
                    ]}
                ]
            }
        });
        let cfg = Path::new(r"D:\\proj\\.codex\\config.toml");
        let entries = codex_trust_entries(&hooks, cfg).unwrap();
        assert_eq!(entries.len(), 1, "foreign handlers are not trusted for");
        let (key, hash) = &entries[0];
        assert!(
            key.ends_with(":pre_tool_use:1:0"),
            "ours sits at group 1 handler 0, got {key}"
        );
        assert!(key.starts_with("D:"), "key_source is the plain config path");
        assert!(hash.starts_with("sha256:"));
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

#[cfg(test)]
mod skill_tests {
    use super::*;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp(tag: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-skill-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn skill_md_carries_marker_and_full_command_map() {
        let s = skill_md();
        assert!(s.contains(SKILL_MARKER));
        for (cmd, intent) in COMMAND_MAP {
            assert!(s.contains(cmd), "missing cmd {cmd}");
            assert!(s.contains(intent), "missing intent {intent}");
        }
    }

    #[test]
    fn write_skill_fresh_legacy_upgrade_user_owned() {
        // 1) 缺文件：写入生成版。
        let d = tmp("fresh");
        let p = d.join("SKILL.md");
        let mut r = DeployReport { wrote: Vec::new(), skipped: Vec::new() };
        write_skill(&p, &mut r).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains(SKILL_MARKER));
        // 2) 幂等：再跑 skipped。
        let mut r2 = DeployReport { wrote: Vec::new(), skipped: Vec::new() };
        write_skill(&p, &mut r2).unwrap();
        assert!(r2.wrote.is_empty() && r2.skipped.len() == 1);
        // 3) 旧静态版：识别升级。
        let legacy = d.join("legacy.md");
        std::fs::write(&legacy, LEGACY_SKILL_MD).unwrap();
        let mut r3 = DeployReport { wrote: Vec::new(), skipped: Vec::new() };
        write_skill(&legacy, &mut r3).unwrap();
        assert!(std::fs::read_to_string(&legacy).unwrap().contains(SKILL_MARKER));
        // 4) 用户内容：无标记不动。
        let user = d.join("user.md");
        std::fs::write(&user, "我的私货 skill").unwrap();
        let mut r4 = DeployReport { wrote: Vec::new(), skipped: Vec::new() };
        write_skill(&user, &mut r4).unwrap();
        assert_eq!(std::fs::read_to_string(&user).unwrap(), "我的私货 skill");
        let _ = std::fs::remove_dir_all(&d);
    }
}
