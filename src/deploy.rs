//! `oma init` hook/skill deployment. Project-level files only, never the
//! user home. Schemas are first-hand verified in S015 (official docs +
//! openai/codex, xai-org/grok-build, MoonshotAI/kimi-code sources).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};

use crate::yolo::{ensure_parent, read_json, read_toml, toml_write, write_json, write_text};

#[derive(Default)]
pub struct DeployReport {
    pub wrote: Vec<String>,
    pub skipped: Vec<String>,
    /// Hook command form chosen this run: "shim" (D27: registrations point at
    /// the self-contained state writer in `<project>/.oma/hooks/`, zero oma
    /// dependency). Set by deploy_claude.
    pub form: Option<&'static str>,
    /// Advisory warnings (non-fatal), e.g. jq missing from PATH at deploy time
    /// (state shim falls back to findstr parsing).
    pub warns: Vec<String>,
}

/// oma-owned handler marker: the current exe, or a stale oma binary whose
/// entry should be replaced (path moved between builds). Matches the bare
/// name `oma`, `oma.exe`, and test-harness binaries like `oma-<hash>.exe`.
fn oma_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("oma"))
}

pub(crate) fn is_ours(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    if lower.contains(&oma_exe().display().to_string().to_ascii_lowercase()) {
        return true;
    }
    // First whitespace token covers both the exec form (bare path) and the
    // Grok shell form (`"C:\...\oma.exe" hook`). The codex Windows form puts
    // the PowerShell call operator first (`& "exe" hook`), so strip a leading
    // `&` before taking the token — without this the Windows-side field of a
    // shared project reads as foreign (doctor misses it; redeploy appends a
    // duplicate once the embedded exe path goes stale).
    let first = lower
        .trim_start_matches('&')
        .split_whitespace()
        .next()
        .unwrap_or("");
    let stem = first
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .trim_matches('"')
        .trim_end_matches(".exe");
    stem == "oma" || stem.starts_with("oma-") || stem.starts_with("oma-state")
}

/// JSON arrays of handler groups under settings["hooks"][event], append-only:
/// drop stale oma entries, keep foreign ones, add ours exactly once.
fn merge_hook_event(settings: &mut Json, event: &str, our_handler: Json) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let groups = obj.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !groups.is_object() {
        *groups = json!({});
    }
    let groups = groups
        .as_object_mut()
        .ok_or_else(|| "hooks is not an object".to_string())?;
    let entry = groups.entry(event.to_string()).or_insert_with(|| json!([]));
    if !entry.is_array() {
        *entry = json!([]);
    }
    let arr = entry
        .as_array_mut()
        .ok_or_else(|| "event groups is not an array".to_string())?;
    let mut changed = false;
    // D27：ours 条目的陈旧判据是「不等于本次要写的 shim 命令」——老形态
    // （bare oma、旧 exe 绝对路径）、项目搬迁后的旧 shim 路径、重复条目都
    // 覆盖：弃后统一补一条现行 shim，重部署恒单条。异侧 OS 的 sh 形态同理
    // 被本侧形态收敛（claude/grok 单 command 字段，无 per-OS 所有权）。
    let ours_cmd = our_handler
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stale = |c: &str| -> bool {
        is_ours(c) && !c.to_ascii_lowercase().eq(&ours_cmd)
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
    arr.retain(|g| {
        !g.as_object().is_some_and(|g| {
            g.get("hooks")
                .and_then(|h| h.as_array())
                .is_some_and(|a| a.is_empty())
        })
    });
    // Update an existing oma handler in place (byte-equal survivors are
    // no-ops), or append when none survived.
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

fn claude_handler(root: &Path, side: OsSide) -> Json {
    json!({
        "type": "command",
        "command": shim_command_ps_or_sh("claude", root, side),
        "timeout": 10,
    })
}

/// D27：注册命令指向自包含状态 shim（项目 `.oma/hooks/`，零 oma 依赖）。
/// PowerShell 消费面（claude / grok 加载 settings.json，M047）用调用操作符；
/// POSIX 一律 sh 路径直引。
fn shim_command_ps_or_sh(agent: &str, root: &Path, side: OsSide) -> String {
    match side {
        OsSide::Windows => format!(
            "& \"{}\" {}",
            root.join(".oma").join("hooks").join("oma-state.cmd").display(),
            agent
        ),
        OsSide::Unix => format!(
            "\"{}\" {}",
            root.join(".oma").join("hooks").join("oma-state.sh").display(),
            agent
        ),
    }
}

/// Which OS consumes a codex registration field: `command` on Unix,
/// `commandWindows` on Windows (S015). Injected so tests exercise both
/// sides from one host.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OsSide {
    Windows,
    Unix,
}

pub fn host_side() -> OsSide {
    if cfg!(windows) {
        OsSide::Windows
    } else {
        OsSide::Unix
    }
}

/// Build the field-ownership form of a codex handler: the deploying side's
/// field is rewritten to the shim path; the foreign side's field survives
/// from `base` byte-verbatim (absent stays absent). Keys keep a fixed order
/// so reruns converge byte-identically on both sides.
fn codex_handler_value(base: &Json, root: &Path, session_end: bool, side: OsSide) -> Json {
    // D27：注册指向自包含 shim（零 oma 依赖）。M057 根修：codex 的 hook 经
    // 会话环境 shell 执行（session/mod.rs：environment.shell.derive_exec_args），
    // Windows 缺省是 PowerShell——引号裸路径加参数是 PS ParserError（M047 同
    // 型），调用操作符 `& "path" codex` 才对；M056 的「cmd 直引号形态」前提
    // 错（当时只经 cmd /c 直测验证，未经 codex 实跑）。Unix 用 sh 路径直引。
    // `command` 为 schema 必填（M055）：无异侧保留值时落 bare oma 兜底。
    let foreign = |key: &str| base.get(key).filter(|v| v.is_string()).cloned();
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!("command"));
    if side == OsSide::Unix {
        obj.insert(
            "command".into(),
            json!(format!(
                "\"{}\" codex",
                root.join(".oma").join("hooks").join("oma-state.sh").display()
            )),
        );
    } else if let Some(v) = foreign("command") {
        obj.insert("command".into(), v);
    } else {
        obj.insert("command".into(), json!("oma hook --agent codex"));
    }
    if side == OsSide::Unix {
        if let Some(v) = foreign("commandWindows") {
            obj.insert("commandWindows".into(), v);
        }
    } else {
        obj.insert(
            "commandWindows".into(),
            json!(format!(
                "& \"{}\" codex",
                root.join(".oma").join("hooks").join("oma-state.cmd").display()
            )),
        );
    }
    obj.insert("timeout".into(), json!(if session_end { 3 } else { 10 }));
    Json::Object(obj)
}

/// codex merge with per-OS field ownership. Never stale-drops oma entries:
/// the foreign OS's absolute path inside the foreign field is live on that
/// OS, and a shared project dir must carry both sides at once (P0027).
/// Trust seeding is unaffected: deploy_codex re-reads the final file and
/// codex derives its state key from the config.toml path per OS, so the
/// two sides' entries coexist instead of overwriting each other.
fn merge_codex_hook_event(
    settings: &mut Json,
    event: &str,
    session_end: bool,
    side: OsSide,
    root: &Path,
) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let groups = obj.entry("hooks".to_string()).or_insert_with(|| json!({}));
    if !groups.is_object() {
        *groups = json!({});
    }
    let groups = groups
        .as_object_mut()
        .ok_or_else(|| "hooks is not an object".to_string())?;
    let entry = groups.entry(event.to_string()).or_insert_with(|| json!([]));
    if !entry.is_array() {
        *entry = json!([]);
    }
    let arr = entry
        .as_array_mut()
        .ok_or_else(|| "event groups is not an array".to_string())?;
    let mut changed = false;
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
            if !handler_is_ours(handler) {
                continue;
            }
            let next = codex_handler_value(handler, root, session_end, side);
            if handler != &next {
                *handler = next;
                changed = true;
            }
            replaced = true;
        }
    }
    if !replaced {
        arr.push(json!({
            "matcher": "*",
            "hooks": [codex_handler_value(&Json::Null, root, session_end, side)]
        }));
        changed = true;
    }
    Ok(changed)
}

fn grok_handler(root: &Path, side: OsSide) -> Json {
    // Windows：grok 只认可整串 spawn 的单路径（M048），指向 baked 包装；
    // Unix：sh 直带参。
    let command = match side {
        OsSide::Windows => root
            .join(".oma")
            .join("hooks")
            .join("oma-state-grok.cmd")
            .display()
            .to_string(),
        OsSide::Unix => shim_command_ps_or_sh("grok", root, side),
    };
    json!({
        "type": "command",
        "command": command,
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
    report.form = Some("shim");
    let mut changed = false;
    for event in events {
        changed |= merge_hook_event(&mut settings, event, claude_handler(root, host_side()))?;
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
fn deploy_codex(root: &Path, report: &mut DeployReport, side: OsSide) -> Result<(), String> {
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
        changed |= merge_codex_hook_event(&mut settings, event, session_end, side, &root)?;
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
    let mut trust_changed = false;
    // 单一来源卫生（M056 顺带，用户建议）：hook 定义只在 hooks.json；config.toml
    // 里 [hooks] 下除 state 外的定义键（旧部署或它源残留）部署时清掉，消
    // codex 的双 representation 警告。trust state 不动。
    if let Some(toml::Value::Table(hooks_tbl)) = table.get_mut("hooks") {
        let stale_defs: Vec<String> = hooks_tbl
            .keys()
            .filter(|k| k.as_str() != "state")
            .cloned()
            .collect();
        if !stale_defs.is_empty() {
            for k in stale_defs {
                hooks_tbl.remove(&k);
            }
            trust_changed = true;
        }
    }
    let states = table.entry("hooks".to_string()).or_insert_with(|| {
        let mut hooks_tbl = toml::map::Map::new();
        hooks_tbl.insert(
            "state".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
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
    event
        .chars()
        .flat_map(|c| {
            if c.is_ascii_uppercase() {
                vec!['_', c.to_ascii_lowercase()]
            } else {
                vec![c]
            }
        })
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

/// oma-owned if either per-OS field names us. Merge already used both;
/// trust seeding must match, or a Windows-only `commandWindows` handler
/// (P0027 field ownership on a fresh deploy) is skipped and `[hooks.state]`
/// is written empty.
fn handler_is_ours(handler: &Json) -> bool {
    ["command", "commandWindows"].iter().any(|k| {
        handler
            .get(*k)
            .and_then(|c| c.as_str())
            .is_some_and(is_ours)
    })
}

/// Command string Codex on this OS actually runs: Windows prefers
/// `commandWindows`, Unix prefers `command`, each falling back to the
/// other so a single-field handler still hashes.
fn effective_codex_command(handler: &Json) -> Result<&str, String> {
    let unix = handler.get("command").and_then(|c| c.as_str());
    let windows = handler.get("commandWindows").and_then(|c| c.as_str());
    let picked = if cfg!(windows) {
        windows.or(unix)
    } else {
        unix.or(windows)
    };
    picked.ok_or_else(|| "handler missing command and commandWindows".into())
}

/// Replicate codex `hook_hash` (S015 source): identity over the normalized
/// handler (commandWindows dropped, timeout clamped per event), serialized
/// as canonical key-sorted JSON, sha256, `sha256:<hex>`.
fn codex_hook_hash(event: &str, matcher: Option<&str>, handler: &Json) -> Result<String, String> {
    let effective = effective_codex_command(handler)?;
    let timeout = handler.get("timeout").and_then(|t| t.as_u64());
    let timeout = match event {
        "SessionEnd" | "Interrupt" => timeout.unwrap_or(1).clamp(1, 3),
        _ => timeout.unwrap_or(600).max(1),
    };
    let r#async = handler
        .get("async")
        .and_then(|a| a.as_bool())
        .unwrap_or(false);
    let mut entry = serde_json::Map::new();
    entry.insert("type".into(), json!("command"));
    entry.insert("command".into(), json!(effective));
    entry.insert("timeout".into(), json!(timeout));
    entry.insert("async".into(), json!(r#async));
    if let Some(sm) = handler.get("statusMessage").and_then(|s| s.as_str()) {
        entry.insert("statusMessage".into(), json!(sm));
    }
    if let Some(limit) = handler
        .get("additionalContextLimit")
        .and_then(|s| s.as_u64())
    {
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
fn codex_trust_entries(
    hooks_json: &Json,
    config_toml: &Path,
) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let Some(events) = hooks_json.get("hooks").and_then(|h| h.as_object()) else {
        return Ok(out);
    };
    let key_source = plain_absolute(config_toml);
    for (event, groups) in events {
        let Some(groups) = groups.as_array() else {
            continue;
        };
        for (gi, group) in groups.iter().enumerate() {
            let matcher = group.get("matcher").and_then(|m| m.as_str());
            let Some(handlers) = group.get("hooks").and_then(|h| h.as_array()) else {
                continue;
            };
            for (hi, handler) in handlers.iter().enumerate() {
                if !handler_is_ours(handler) {
                    continue;
                }
                let hash = codex_hook_hash(event, matcher, handler)?;
                let key = format!("{}:{}:{}:{}", key_source, event_label(event), gi, hi);
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
    let path = root
        .join(".grok")
        .join("hooks")
        .join("ohmyagents-state.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    for event in events {
        changed |= merge_hook_event(&mut settings, event, grok_handler(root, host_side()))?;
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
    (
        "oma init [--project PATH]",
        "部署本项目 hook/skill/yolo 键（幂等，四环境自适应；hook 注册指向 .oma/hooks/ 自包含状态 shim，D27）",
    ),
    (
        "oma doctor",
        "只读诊断信任库、二进制、登录态、hook 形态与状态栏",
    ),
    (
        "oma agents",
        "检测四家 agent 已装情况（PATH/环境变量/oma 自管根/默认目录四源）",
    ),
    (
        "oma agents statusline [名] [--example] [--script 路径] [--builtin]",
        "配置四家状态栏（写入面幂等；状态由 hook 落盘供给；--example 定制模板，--script 自备脚本整替换，--builtin 还原）",
    ),
    (
        "oma self update",
        "oma 自更新（缺省 dev 滚动源，按资产 sha256 判新）",
    ),
    (
        "oma trace sessions|timeline|blocks|agent|file|search",
        "项目内四家 agent 对话历史检索（六视图联邦读原生会话库，只读）",
    ),
    (
        "oma agents verify [名] [--timeout N]",
        "无头验收四家 agent：状态栏脚本 mock 直跑加 hook 无头落盘（S033 两层判据）",
    ),
    (
        "oma diagnose cache [别名...]",
        "网关缓存探测：逐别名双连判前缀缓存命中矩阵（D21；打真 API 烧最小 token）",
    ),
    (
        "oma diagnose agents",
        "agent 配置活性检测：指向、别名在册、key 活性、thinking 上限对照（D21）",
    ),
    (
        "oma skill [--write]",
        "生成 oma 自身 SKILL.md（从活命令树自适应渲染，新命令自动出现；--write 落用户级 ~/.claude/skills/，D22）",
    ),
];

/// 生成标记：只有带它的 SKILL.md 才允许 oma 覆写（用户手改过的跳过）。
const SKILL_MARKER: &str = "<!-- generated by oma init; rerun oma init to sync the command map -->";

/// 旧版静态 skill 全文：识别后升级为命令图生成版。
const LEGACY_SKILL_MD: &str = "---\nname: ohmyagents\ndescription: Oh My Agents 项目编排说明与状态通道\n---\n\n# Oh My Agents\n\n本项目会话由 oma 编排。agent 状态在 `.ohmyagents/state/`；委派与诊断经 oma CLI。\n";

fn skill_md() -> String {
    let mut s = String::new();
    s.push_str("---\nname: ohmyagents\ndescription: oma 部署配置命令图：init、诊断、hook、状态栏、trace\n---\n\n");
    s.push_str("# Oh My Agents 命令图\n\n");
    s.push_str(SKILL_MARKER);
    s.push_str("\n\n本项目由 oma 部署配置：hook 状态写 `.oma/state/`，供状态栏 `agent:state` 机读标记消费。\n\n");
    s.push_str("| 意图 | 命令 |\n| --- | --- |\n");
    for (cmd, intent) in COMMAND_MAP {
        s.push_str(&format!("| {intent} | `{cmd}` |\n"));
    }
    s.push_str("\n全部命令加 `--json` 出信封。细则见仓库 `docs\\references\\R002`。\n");
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
                report
                    .skipped
                    .push(format!("{} (user-owned)", path.display()));
            }
        }
        Err(_) => {
            write_text(path, &generated)?;
            report.wrote.push(path.display().to_string());
        }
    }
    Ok(())
}

const AGENTS_MD: &str = "# AGENTS\n\n本项目会话由 Oh My Agents（oma）编排：agent 状态写 `.oma/state/`，委派与诊断经 oma CLI。\n";

/// Skills: `.agents/skills/ohmyagents` is the source; Claude and Grok and
/// Kimi get copies (Claude does not scan .agents/skills, S008).
fn deploy_skills(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let source = root.join(".agents").join("skills").join("ohmyagents");
    let skill = source.join("SKILL.md");
    write_skill(&skill, report)?;
    for target in [".claude", ".grok", ".kimi-code"] {
        let copy = root
            .join(target)
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md");
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
    apply_project_hooks_with(root, host_side())
}

/// Test seam: codex field side is injected so unit tests exercise both
/// sides from one host.
pub fn apply_project_hooks_with(root: &Path, side: OsSide) -> Result<DeployReport, String> {
    // abs_display（非裸 canonicalize）：剥掉 Windows `\\?\` 前缀，路径要进
    // 注册命令与 codex 信任键，带前缀 cmd 侧不可执行。
    let root = crate::pathutil::abs_display(root);
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut report = DeployReport::default();
    // D27：先落自包含状态 shim（三平台脚本全侧落齐，幂等；jq 探测在内）。
    let (shim_wrote, shim_warns) = crate::shim::deploy_shims(&root)?;
    for p in shim_wrote {
        report.wrote.push(p.display().to_string());
    }
    report.warns.extend(shim_warns);
    deploy_claude(&root, &mut report)?;
    deploy_codex(&root, &mut report, side)?;
    deploy_grok(&root, &mut report)?;
    deploy_kimi(&root, &mut report)?;
    deploy_skills(&root, &mut report)?;
    deploy_instructions(&root, &mut report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_ours_handles_windows_call_operator_form() {
        // codex commandWindows 是 `& "exe" hook`：调用操作符在首 token 前，
        // 不剥掉会把 Windows 侧字段误判成外来者（跨环境共享目录必踩）。
        assert!(is_ours(r#"& "D:\cargo\bin\oma.exe" hook --agent codex"#));
        assert!(is_ours(r#""C:\somewhere\oma.exe" hook"#));
        assert!(is_ours("oma hook --agent claude"));
        assert!(is_ours(r#""/home/ray/.cargo/bin/oma" hook --agent codex"#));
        assert!(!is_ours(r#"& "D:\tools\echo.exe" args"#));
        assert!(!is_ours("echo hi"));
    }

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

        let first = apply_project_hooks_with(&root, host_side()).unwrap();
        assert!(first.wrote.iter().any(|p| p.ends_with("settings.json")));
        assert!(first.wrote.iter().any(|p| p.ends_with("hooks.json")));
        assert!(first
            .wrote
            .iter()
            .any(|p| p.ends_with("ohmyagents-state.json")));
        assert!(
            first.wrote.iter().any(|p| p.ends_with("oma-state.cmd")),
            "state shims deploy with the registrations: {:?}",
            first.wrote
        );
        assert_eq!(first.form.as_deref(), Some("shim"));
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
            .find(|g| g["hooks"][0]["command"].as_str().unwrap().contains("oma-state"))
            .unwrap();
        assert!(ours["hooks"][0].get("args").is_none());
        let claude_cmd = ours["hooks"][0]["command"].as_str().unwrap();
        if cfg!(windows) {
            // PowerShell 消费面（M047）：调用操作符加引号路径加 agent 参数。
            assert!(claude_cmd.starts_with("& \""), "{claude_cmd}");
            assert!(claude_cmd.ends_with("oma-state.cmd\" claude"), "{claude_cmd}");
        } else {
            assert!(claude_cmd.ends_with("oma-state.sh\" claude"), "{claude_cmd}");
        }
        assert!(v["hooks"]["PermissionRequest"].is_array());

        let codex: Json = serde_json::from_str(
            &fs::read_to_string(root.join(".codex").join("hooks.json")).unwrap(),
        )
        .unwrap();
        // Field ownership: host side owns its field; `command` is codex
        // schema-REQUIRED（0.149.1 起缺字段整份 hooks 解析失败）——Windows
        // 侧新部署也落 bare oma 兜底，Unix 侧仍不发明 commandWindows。
        let handler = &codex["hooks"]["SessionEnd"][0]["hooks"][0];
        if cfg!(windows) {
            // M057：PowerShell 调用操作符形态（codex hook 走会话环境 shell，
            // Windows 缺省 PS；直引号路径加参数是 ParserError）。
            assert!(handler["commandWindows"]
                .as_str()
                .unwrap()
                .starts_with("& \""));
            assert!(handler["commandWindows"]
                .as_str()
                .unwrap()
                .ends_with("oma-state.cmd\" codex"));
            assert_eq!(
                handler["command"].as_str(),
                Some("oma hook --agent codex"),
                "schema-required fallback must be present on Windows"
            );
        } else {
            assert!(handler["command"]
                .as_str()
                .unwrap()
                .ends_with("oma-state.sh\" codex"));
            assert!(
                handler.get("commandWindows").is_none(),
                "Unix fresh deploy must not invent the foreign-OS field"
            );
        }
        assert_eq!(handler["timeout"], 3);
        assert!(codex["hooks"].get("Notification").is_none());
        let codex_toml = fs::read_to_string(root.join(".codex").join("config.toml")).unwrap();
        assert!(codex_toml.contains("hooks = true"));
        assert!(
            codex_toml.contains("trusted_hash"),
            "host-side-only field must still seed [hooks.state]: {codex_toml}"
        );

        let grok: Json = serde_json::from_str(
            &fs::read_to_string(
                root.join(".grok")
                    .join("hooks")
                    .join("ohmyagents-state.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let grok_cmd = grok["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap();
        // Windows：grok 只认可整串 spawn 的单路径（M048 baked 包装）；Unix：sh 直带参。
        if cfg!(windows) {
            assert!(grok_cmd.ends_with("oma-state-grok.cmd"), "{grok_cmd}");
            assert!(!grok_cmd.contains(' '), "single spawnable path: {grok_cmd}");
        } else {
            assert!(grok_cmd.ends_with("oma-state.sh\" grok"), "{grok_cmd}");
        }
        assert!(grok["hooks"].get("PermissionRequest").is_none());

        // Kimi: skill dir only, no hook registration anywhere in the project.
        assert!(root
            .join(".kimi-code")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md")
            .exists());
        assert!(!root.join(".kimi-code").join("config.toml").exists());

        // Skills copied to every family dir; CLAUDE.md include created.
        assert!(root
            .join(".agents")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md")
            .exists());
        assert!(root
            .join(".grok")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md")
            .exists());
        assert!(root
            .join(".claude")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md")
            .exists());
        assert_eq!(
            fs::read_to_string(root.join("CLAUDE.md")).unwrap(),
            "@AGENTS.md\n"
        );

        // Second deploy: nothing changes on disk.
        let before = fs::read_to_string(&settings).unwrap();
        let second = apply_project_hooks_with(&root, host_side()).unwrap();
        assert!(
            second.wrote.is_empty(),
            "redeploy must write nothing: {:?}",
            second.wrote
        );
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
        // Timeout clamps for SessionEnd. Feed a command without
        // commandWindows so the identity is platform-independent: on
        // Windows the windows field would leak into the hash and the
        // convergence below would hold only there.
        let se = codex_hook_hash(
            "SessionEnd",
            None,
            &serde_json::json!({"type":"command","command":"oma","timeout":10,"async":false}),
        )
        .unwrap();
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
        assert_eq!(se, se_clamped, "10 and 99 both clamp to 3 and converge");
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
    fn codex_trust_entries_seed_windows_only_commandwindows() {
        // P0027 fresh Windows deploy writes commandWindows and omits command.
        // Seeding must still emit a hash or [hooks.state] stays empty.
        let hooks = serde_json::json!({
            "hooks": {
                "PreToolUse": [
                    {"matcher": "*", "hooks": [
                        {"type": "command",
                         "commandWindows": "& \"D:\\ohmyenv\\cargo\\bin\\oma.exe\" hook --agent codex",
                         "timeout": 10}
                    ]}
                ]
            }
        });
        let cfg = Path::new(r"D:\proj\.codex\config.toml");
        let entries = codex_trust_entries(&hooks, cfg).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "commandWindows-only oma handler must be seeded: {entries:?}"
        );
        assert!(
            entries[0].0.ends_with(":pre_tool_use:0:0"),
            "got {}",
            entries[0].0
        );
        assert!(entries[0].1.starts_with("sha256:"));
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
        apply_project_hooks_with(&root, host_side()).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let commands: Vec<&str> = v["hooks"]["Stop"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter_map(|h| h["command"].as_str())
            .collect();
        assert_eq!(
            commands.len(),
            1,
            "stale entry must be replaced, got {commands:?}"
        );
        assert!(
            !commands[0].contains("D:\\old"),
            "stale path must not survive"
        );
        assert!(
            commands[0].contains("oma-state"),
            "healed to the shim form: {}",
            commands[0]
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_forms_are_healed_to_shim_and_duplicates_collapse() {
        // v0.5.0 及更早的注册形态（bare oma、旧 exe 绝对路径、旧 oma-state
        // 路径）与重复条目：重部署一律收敛到本侧 shim 单条。
        let root = fresh_dir("heal");
        let settings = root.join(".claude").join("settings.json");
        ensure_parent(&settings).unwrap();
        write_text(
            &settings,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "oma hook --agent claude"},
                {"type": "command", "command": "D:\\old\\oma.exe hook --agent claude"},
                {"type": "command", "command": "D:\\moved\\.oma\\hooks\\oma-state.cmd claude"}]}]}}"#,
        )
        .unwrap();
        let grok = root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json");
        ensure_parent(&grok).unwrap();
        write_text(
            &grok,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "\"/mnt/d/old/oma\" hook"}]}]}}"#,
        )
        .unwrap();

        let first = apply_project_hooks_with(&root, host_side()).unwrap();
        assert_eq!(first.form, Some("shim"));
        let collect = |p: &Path| -> Vec<String> {
            let v: Json = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();
            v["hooks"]["Stop"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|g| g["hooks"].as_array().unwrap().iter())
                .filter_map(|h| h["command"].as_str().map(String::from))
                .collect()
        };
        let claude_cmds = collect(&settings);
        let ours: Vec<&String> = claude_cmds
            .iter()
            .filter(|c| is_ours(c))
            .collect();
        assert_eq!(ours.len(), 1, "legacy forms collapse to one: {claude_cmds:?}");
        assert!(ours[0].contains("oma-state"), "{}", ours[0]);
        assert!(
            !claude_cmds.iter().any(|c| c.contains("D:\\old")),
            "old exe path must not survive"
        );
        let grok_cmds = collect(&grok);
        assert!(
            grok_cmds
                .iter()
                .any(|c| c.contains("oma-state") && !c.contains("/mnt/d/old")),
            "grok healed off the old oma path: {grok_cmds:?}"
        );

        // 幂等：收敛后重部署零写入。
        let second = apply_project_hooks_with(&root, host_side()).unwrap();
        assert!(
            second.wrote.is_empty(),
            "no rewrite after healing: {:?}",
            second.wrote
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn foreign_os_absolute_entry_is_healed_to_shim() {
        // The migration path for this very repo: WSL-written /mnt/d paths
        // consumed by a Windows session.
        let root = fresh_dir("heal2");
        let settings = root.join(".claude").join("settings.json");
        ensure_parent(&settings).unwrap();
        write_text(
            &settings,
            r#"{"hooks": {"UserPromptSubmit": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "/mnt/d/ohmyagents/target/debug/oma", "args": ["hook"]}]}]}}"#,
        )
        .unwrap();
        apply_project_hooks_with(&root, host_side()).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
        let commands: Vec<&str> = v["hooks"]["UserPromptSubmit"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter_map(|h| h["command"].as_str())
            .collect();
        assert_eq!(commands.len(), 1, "healed to a single entry: {commands:?}");
        assert!(commands[0].contains("oma-state"), "{}", commands[0]);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn codex_preserves_foreign_os_field_and_is_idempotent() {
        let root = fresh_dir("codexside");
        let path = root.join(".codex").join("hooks.json");
        ensure_parent(&path).unwrap();
        // Unix side ran first: its command plus a Windows-shaped twin field.
        write_text(
            &path,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "\"/mnt/d/oma\" hook",
                 "commandWindows": "& \"D:\\old\\oma.exe\" hook", "timeout": 10}]}]}}"#,
        )
        .unwrap();

        // Windows run: owns commandWindows, must preserve command verbatim.
        apply_project_hooks_with(&root, OsSide::Windows).unwrap();
        let after_win: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let h = &after_win["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(h["command"].as_str(), Some("\"/mnt/d/oma\" hook"));
        let win_cmd = h["commandWindows"].as_str().unwrap().to_string();
        // M057：PS 调用操作符形态——codex hook 经会话环境 shell（Windows
        // 缺省 PowerShell）执行，`& "path" codex` 才合法；M056 的直引号
        // 形态在 PS 下是 ParserError（实测 codex 报 hook Failed）。
        assert!(win_cmd.starts_with("& \""), "{win_cmd}");
        assert!(
            win_cmd.ends_with("oma-state.cmd\" codex"),
            "{win_cmd}"
        );
        assert!(!win_cmd.contains("old"), "owned field rewritten: {win_cmd}");

        // Same side again: byte-identical, nothing rewritten.
        let before = fs::read_to_string(&path).unwrap();
        let second = apply_project_hooks_with(&root, OsSide::Windows).unwrap();
        assert!(second.wrote.is_empty(), "idempotent: {:?}", second.wrote);
        assert_eq!(fs::read_to_string(&path).unwrap(), before);

        // Unix run: owns command, must preserve commandWindows verbatim.
        apply_project_hooks_with(&root, OsSide::Unix).unwrap();
        let after_unix: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let h = &after_unix["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(h["commandWindows"].as_str(), Some(win_cmd.as_str()));
        assert!(
            h["command"].as_str().unwrap().ends_with("oma-state.sh\" codex"),
            "unix-owned field rewritten to sh shim"
        );
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
        let mut r = DeployReport::default();
        write_skill(&p, &mut r).unwrap();
        assert!(std::fs::read_to_string(&p).unwrap().contains(SKILL_MARKER));
        // 2) 幂等：再跑 skipped。
        let mut r2 = DeployReport::default();
        write_skill(&p, &mut r2).unwrap();
        assert!(r2.wrote.is_empty() && r2.skipped.len() == 1);
        // 3) 旧静态版：识别升级。
        let legacy = d.join("legacy.md");
        std::fs::write(&legacy, LEGACY_SKILL_MD).unwrap();
        let mut r3 = DeployReport::default();
        write_skill(&legacy, &mut r3).unwrap();
        assert!(std::fs::read_to_string(&legacy)
            .unwrap()
            .contains(SKILL_MARKER));
        // 4) 用户内容：无标记不动。
        let user = d.join("user.md");
        std::fs::write(&user, "我的私货 skill").unwrap();
        let mut r4 = DeployReport::default();
        write_skill(&user, &mut r4).unwrap();
        assert_eq!(std::fs::read_to_string(&user).unwrap(), "我的私货 skill");
        let _ = std::fs::remove_dir_all(&d);
    }
}
