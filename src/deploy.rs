//! `oma init` hook/skill deployment（D28：hook 注册与 shim 常驻用户级）。
//! 用户级注册面（用户裁 2026-09-11「hook 应用户全局」，对齐 codex 用户层）：
//! claude `~/.claude/settings.json`（settings 家族用户层生效，S015）、codex
//! `~/.codex/hooks.json` 加 `~/.codex/config.toml` features 与 trusted_hash
//! 预种、grok `~/.grok/hooks/ohmyagents-state.json`（global 层）、kimi
//! `~/.kimi-code/config.toml [[hooks]]`（kimi 仅用户级，S015）。shim 常驻
//! `~/.oma/hooks/`，状态按 session 分键写 `~/.oma/state/`（D28）。项目级
//! 旧注册与 `.oma/hooks/` 由 init 迁移退役（未 init 项目零数据根因消除）。
//! skills 与 AGENTS/CLAUDE 说明仍是项目级（项目内语义）。Schemas are
//! first-hand verified in S015 (official docs + openai/codex, xai-org/grok-build,
//! MoonshotAI/kimi-code sources).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value as Json};

use crate::yolo::{ensure_parent, read_json, read_toml, toml_write, write_json, write_text};

#[derive(Default)]
pub struct DeployReport {
    pub wrote: Vec<String>,
    pub skipped: Vec<String>,
    /// Hook command form chosen this run: "user" (D28: registrations live in
    /// the four agents' user-level configs and point at the self-contained
    /// state writer in `~/.oma/hooks/`, zero oma dependency).
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
    // ours 条目的陈旧判据是「不等于本次要写的 shim 命令」——老形态（bare
    // oma、旧 exe 绝对路径、D27 项目级 shim 路径）、重复条目都覆盖：弃后
    // 统一补一条现行注册，重部署恒单条（D28 后现行命令指向 ~/.oma/hooks/）。
    let ours_cmd = our_handler
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stale = |c: &str| -> bool { is_ours(c) && !c.to_ascii_lowercase().eq(&ours_cmd) };
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
    // no-ops), or append when none survived. Same-shape duplicates collapse:
    // after the update pass every ours-handler equals our_handler, so keep
    // the first and drop the rest (review-verified: identical duplicates
    // previously survived redeploy).
    let mut replaced = false;
    let mut seen_current = false;
    for group in arr.iter_mut() {
        let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        else {
            continue;
        };
        let before = hooks.len();
        hooks.retain(|handler| {
            let ours = handler
                .get("command")
                .and_then(|c| c.as_str())
                .map(|c| is_ours(c))
                .unwrap_or(false);
            if !ours {
                return true;
            }
            if *handler == our_handler && !seen_current {
                seen_current = true;
                return true;
            }
            false
        });
        if hooks.len() != before {
            changed = true;
        }
        for handler in hooks.iter_mut() {
            let ours = handler
                .get("command")
                .and_then(|c| c.as_str())
                .map(|c| is_ours(c))
                .unwrap_or(false);
            if ours && *handler != our_handler {
                *handler = our_handler.clone();
                changed = true;
            }
            if ours {
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

/// oma-owned if either per-OS field names us (codex shape).
fn handler_is_ours(handler: &Json) -> bool {
    ["command", "commandWindows"].iter().any(|k| {
        handler
            .get(*k)
            .and_then(|c| c.as_str())
            .is_some_and(is_ours)
    })
}

/// 从 JSON 形 hook 注册（claude / grok）里剥除全部 ours 处理器；空组、空
/// 事件与空 hooks 对象一并清掉。返回是否变更（D28 项目面退役）。
fn strip_ours_handlers(settings: &mut Json) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let Some(groups) = obj.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(false);
    };
    let mut changed = false;
    for (_event, arr) in groups.iter_mut() {
        let Some(groups_arr) = arr.as_array_mut() else {
            continue;
        };
        for group in groups_arr.iter_mut() {
            let Some(hooks) = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(|h| h.as_array_mut())
            else {
                continue;
            };
            let before = hooks.len();
            hooks.retain(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .map(|c| !is_ours(c))
                    .unwrap_or(true)
            });
            if hooks.len() != before {
                changed = true;
            }
        }
    }
    // 空组与空事件键摘除；hooks 对象空了连键一起摘。
    for (_event, arr) in groups.iter_mut() {
        if let Some(groups_arr) = arr.as_array_mut() {
            let before = groups_arr.len();
            groups_arr.retain(|g| {
                !g.as_object().is_some_and(|g| {
                    g.get("hooks")
                        .and_then(|h| h.as_array())
                        .is_some_and(|a| a.is_empty())
                })
            });
            if groups_arr.len() != before {
                changed = true;
            }
        }
    }
    let before = groups.len();
    groups.retain(|_, arr| !arr.as_array().is_some_and(|a| a.is_empty()));
    if groups.len() != before {
        changed = true;
    }
    if groups.is_empty() {
        obj.remove("hooks");
    }
    Ok(changed)
}

/// codex 形（command/commandWindows 双字段）的 ours 剥除（D28 项目面退役）。
fn strip_ours_codex_handlers(settings: &mut Json) -> Result<bool, String> {
    let Some(obj) = settings.as_object_mut() else {
        return Err("settings root is not an object".into());
    };
    let Some(groups) = obj.get_mut("hooks").and_then(|h| h.as_object_mut()) else {
        return Ok(false);
    };
    let mut changed = false;
    for (_event, arr) in groups.iter_mut() {
        let Some(groups_arr) = arr.as_array_mut() else {
            continue;
        };
        for group in groups_arr.iter_mut() {
            let Some(hooks) = group
                .as_object_mut()
                .and_then(|g| g.get_mut("hooks"))
                .and_then(|h| h.as_array_mut())
            else {
                continue;
            };
            let before = hooks.len();
            hooks.retain(|h| !handler_is_ours(h));
            if hooks.len() != before {
                changed = true;
            }
        }
    }
    for (_event, arr) in groups.iter_mut() {
        if let Some(groups_arr) = arr.as_array_mut() {
            let before = groups_arr.len();
            groups_arr.retain(|g| {
                !g.as_object().is_some_and(|g| {
                    g.get("hooks")
                        .and_then(|h| h.as_array())
                        .is_some_and(|a| a.is_empty())
                })
            });
            if groups_arr.len() != before {
                changed = true;
            }
        }
    }
    let before = groups.len();
    groups.retain(|_, arr| !arr.as_array().is_some_and(|a| a.is_empty()));
    if groups.len() != before {
        changed = true;
    }
    if groups.is_empty() {
        obj.remove("hooks");
    }
    Ok(changed)
}

/// 退役写入：变更则回写；根对象只剩 `{}`（整文件只有 ours 内容）则删文件。
fn retire_json_file(
    path: &Path,
    strip: fn(&mut Json) -> Result<bool, String>,
    report: &mut DeployReport,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut settings = read_json(path)?;
    if !settings.is_object() {
        return Ok(());
    }
    if strip(&mut settings)? {
        if settings.as_object().is_some_and(|o| o.is_empty()) {
            fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
            report.wrote.push(format!("{} (retired)", path.display()));
        } else {
            write_json(path, &settings)?;
            report
                .wrote
                .push(format!("{} (retired-ours)", path.display()));
        }
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

fn claude_handler(oma: &Path, side: OsSide) -> Json {
    json!({
        "type": "command",
        "command": shim_command_ps_or_sh("claude", oma, side),
        "timeout": 10,
    })
}

/// D27：注册命令指向自包含状态 shim。D28 起常驻 `~/.oma/hooks/`（oma 自管
/// 根，跨项目共享、oma 轮换无痛）。Windows 用**无引号正斜杠绝对路径**加
/// 参数：settings.json 是双消费者（claude 本体经 `/usr/bin/bash -c`、grok
/// 经 PowerShell，M047/M059），`&` 调用操作符在 bash 是语法错误、双引号
/// 路径在 PS 是 ParserError，唯无引号正斜杠三吃（bash / PowerShell / cmd
/// 实测 2026-09-10 全 exit 0 落盘）；边界：路径含空格时该形态在各 shell
/// 都裂，部署侧 warn（doctor 提示）。POSIX 一律 sh 路径直引（引号在 sh
/// 合法且必要）。
fn shim_command_ps_or_sh(agent: &str, oma: &Path, side: OsSide) -> String {
    match side {
        OsSide::Windows => format!(
            "{} {}",
            crate::pathutil::forward_slash(&oma.join("hooks").join("oma-state.cmd")),
            agent
        ),
        OsSide::Unix => format!(
            "\"{}\" {}",
            oma.join("hooks").join("oma-state.sh").display(),
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
/// so reruns converge byte-identically on both sides. D28：shim 在 oma 自管
/// 根（用户级注册只写本侧家目录文件，异侧字段保留语义沿用不动）。
fn codex_handler_value(base: &Json, oma: &Path, session_end: bool, side: OsSide) -> Json {
    // 注册指向自包含 shim（零 oma 依赖）。Windows 侧用无引号正斜杠绝对
    // 路径加参数（M059 统一形态：codex 的 hook 经会话环境 shell 执行
    // ——session/mod.rs 的 environment.shell.derive_exec_args，Windows 缺省
    // PowerShell；该形态在 bash / PS / cmd 三吃）。Unix 用 sh 路径直引。
    // `command` 为 schema 必填（M055）：无异侧保留值时落 bare oma 兜底。
    let foreign = |key: &str| base.get(key).filter(|v| v.is_string()).cloned();
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), json!("command"));
    if side == OsSide::Unix {
        obj.insert(
            "command".into(),
            json!(format!(
                "\"{}\" codex",
                oma.join("hooks").join("oma-state.sh").display()
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
                "{} codex",
                crate::pathutil::forward_slash(&oma.join("hooks").join("oma-state.cmd"))
            )),
        );
    }
    obj.insert("timeout".into(), json!(if session_end { 3 } else { 10 }));
    Json::Object(obj)
}

/// codex merge with per-OS field ownership. Never stale-drops oma entries:
/// the foreign OS's absolute path inside the foreign field is live on that
/// OS（D28 用户级注册后此语义主要保护既有异侧残留与人为混写）。
fn merge_codex_hook_event(
    settings: &mut Json,
    event: &str,
    session_end: bool,
    side: OsSide,
    oma: &Path,
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
    // 全量已见集合而非单值（review 复核残余：A,B,A 序列单值判定会漏掉尾
    // A；集合语义按形态全等去重，异形各自保留）。
    let mut seen: HashSet<Json> = HashSet::new();
    for group in arr.iter_mut() {
        let Some(hooks) = group
            .as_object_mut()
            .and_then(|g| g.get_mut("hooks"))
            .and_then(|h| h.as_array_mut())
        else {
            continue;
        };
        let before = hooks.len();
        hooks.retain(|handler| {
            if !handler_is_ours(handler) {
                return true;
            }
            let next = codex_handler_value(handler, oma, session_end, side);
            seen.insert(next) // 同形重复：保首条弃余
        });
        if hooks.len() != before {
            changed = true;
        }
        for handler in hooks.iter_mut() {
            if !handler_is_ours(handler) {
                continue;
            }
            let next = codex_handler_value(handler, oma, session_end, side);
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
            "hooks": [codex_handler_value(&Json::Null, oma, session_end, side)]
        }));
        changed = true;
    }
    Ok(changed)
}

fn grok_handler(oma: &Path, side: OsSide) -> Json {
    // Windows：grok 只认可整串 spawn 的单路径（M048），指向 baked 包装；
    // Unix：sh 直带参。
    let command = match side {
        OsSide::Windows => oma
            .join("hooks")
            .join("oma-state-grok.cmd")
            .display()
            .to_string(),
        OsSide::Unix => shim_command_ps_or_sh("grok", oma, side),
    };
    json!({
        "type": "command",
        "command": command,
        "timeout": 10,
    })
}

/// Claude：`~/.claude/settings.json` 用户层（S015 settings 家族；hooks 与
/// statusline 各占一键，互不干扰）。事件集含 PermissionRequest。
fn deploy_claude_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
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
    let path = user_home.join(".claude").join("settings.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    report.form = Some("user");
    let mut changed = false;
    for event in events {
        changed |= merge_hook_event(&mut settings, event, claude_handler(oma, side))?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

/// Codex：`~/.codex/hooks.json`（JSON layer；config.toml [hooks] 是双表示，
/// 两者都非空触发警告，故只用 hooks.json 一层）+ `~/.codex/config.toml`
/// `[features] hooks` 与 `[hooks.state]` trusted_hash 预种。Notification
/// does not exist in Codex (S015)。
///
/// Trust is pre-seeded by replicating codex's own identity scheme (S015
/// source): key `<config.toml abs>:<event_label>:<group>:<handler>`, hash
/// over the normalized handler identity (canonical key-sorted JSON, sha256)。
/// 用户层的 key source 即用户 config.toml 路径。
fn deploy_codex_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let events = [
        ("SessionStart", false),
        ("UserPromptSubmit", false),
        ("PreToolUse", false),
        ("PermissionRequest", false),
        ("PostToolUse", false),
        ("Stop", false),
        ("SessionEnd", true),
    ];
    let path = user_home.join(".codex").join("hooks.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    for (event, session_end) in events {
        changed |= merge_codex_hook_event(&mut settings, event, session_end, side, oma)?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }

    // [features] hooks = true in user config.toml.
    let cfg = user_home.join(".codex").join("config.toml");
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

    // Pre-seed [hooks.state]."<key>" trusted_hash for every oma handler in
    // the final hooks.json (real indices, not assumption zero).
    let final_hooks = read_json(&path)?;
    let entries = codex_trust_entries(&final_hooks, &cfg)?;
    let mut trust_changed = false;
    // 单一来源卫生（M056 顺带）：hook 定义只在 hooks.json；config.toml 里
    // [hooks] 下除 state 外的定义键（旧部署或它源残留）部署时清掉。
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

/// Grok：`~/.grok/hooks/ohmyagents-state.json`（global 层 Directory 源，
/// Claude 同构 JSON）。No PermissionRequest event exists (S015)。
fn deploy_grok_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let events = [
        "SessionStart",
        "UserPromptSubmit",
        "PreToolUse",
        "PostToolUse",
        "Notification",
        "Stop",
        "SessionEnd",
    ];
    let path = user_home
        .join(".grok")
        .join("hooks")
        .join("ohmyagents-state.json");
    let mut settings = read_json(&path)?;
    if !settings.is_object() {
        settings = json!({});
    }
    let mut changed = false;
    for event in events {
        changed |= merge_hook_event(&mut settings, event, grok_handler(oma, side))?;
    }
    if changed {
        write_json(&path, &settings)?;
        report.wrote.push(path.display().to_string());
    } else {
        report.skipped.push(path.display().to_string());
    }
    Ok(())
}

/// kimi `[[hooks]]` 表项（S015 schema `.strict()` 只收 event/matcher/
/// command/timeout 四字段；matcher 不填匹配全部）。
fn kimi_hook_entry(event: &str, command: &str) -> toml::Value {
    let mut m = toml::map::Map::new();
    m.insert("event".into(), toml::Value::String(event.into()));
    m.insert("command".into(), toml::Value::String(command.into()));
    m.insert("timeout".into(), toml::Value::Integer(10));
    toml::Value::Table(m)
}

/// kimi hook 命令串：不带引号的正斜杠裸命令行（M058 实证：kimi 对整串
/// 命令朴素消费，引号形态静默不执行；TOML basic string 反斜杠是转义符，
/// 一律正斜杠）。
fn kimi_hook_command(oma: &Path, side: OsSide) -> String {
    match side {
        OsSide::Windows => format!(
            "{} kimi",
            crate::pathutil::forward_slash(&oma.join("hooks").join("oma-state.cmd"))
        ),
        OsSide::Unix => format!("{} kimi", oma.join("hooks").join("oma-state.sh").display()),
    }
}

/// Kimi：`~/.kimi-code/config.toml` `[[hooks]]` 扁平数组合并（S015：kimi
/// 仅用户级，项目级 hook 注册不存在）。陈旧 ours（bare oma、旧路径、异形）
/// 弃后按事件补现行单条，同形去重；外来条目保留。
fn apply_kimi_hooks(
    toml: &mut toml::Value,
    command: &str,
    events: &[&str],
) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("kimi config.toml is not a table".into()),
    };
    let arr = table
        .entry("hooks".to_string())
        .or_insert_with(|| toml::Value::Array(Vec::new()));
    let items = match arr {
        toml::Value::Array(a) => a,
        _ => return Err("kimi [hooks] is not an array of tables".into()),
    };
    let mut changed = false;
    let before = items.len();
    // 现形保留（事件对应的现行条目），陈旧/异形弃。
    items.retain(|h| {
        let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
            return true;
        };
        if !is_ours(c) {
            return true;
        }
        let ev = h.get("event").and_then(|e| e.as_str()).unwrap_or("");
        *h == kimi_hook_entry(ev, command)
    });
    if items.len() != before {
        changed = true;
    }
    // 同形重复去重（保首条；toml::Value 不可哈希，键取 (event, command)）。
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let before = items.len();
    items.retain(|h| {
        let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
            return true;
        };
        if !is_ours(c) {
            return true;
        }
        let ev = h
            .get("event")
            .and_then(|e| e.as_str())
            .unwrap_or("")
            .to_string();
        seen.insert((ev, c.to_string()))
    });
    if items.len() != before {
        changed = true;
    }
    for event in events {
        let want = kimi_hook_entry(event, command);
        if !items.contains(&want) {
            items.push(want);
            changed = true;
        }
    }
    Ok(changed)
}

/// Kimi 用户级部署（hook 注册 + skill 目录说明归项目面 deploy_kimi_retire
/// 之外；kimi 项目侧本就无注册面）。
fn deploy_kimi_user(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
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
    let config = user_home.join(".kimi-code").join("config.toml");
    let mut toml = read_toml(&config)?;
    if apply_kimi_hooks(&mut toml, &kimi_hook_command(oma, side), &events)? {
        toml_write(&config, &toml)?;
        report.wrote.push(config.display().to_string());
    } else {
        report.skipped.push(config.display().to_string());
    }
    Ok(())
}

/// 用户级部署总入口（可注入：测试传临时 user_home 与 oma 根；生产传真实
/// 家目录与 `install::oma_home()`）。shim 先落（三平台全侧），四家注册
/// 幂等合并。
pub fn deploy_user_hooks_with(
    user_home: &Path,
    oma: &Path,
    side: OsSide,
    report: &mut DeployReport,
) -> Result<(), String> {
    let (shim_wrote, shim_warns) = crate::shim::deploy_shims(oma)?;
    for p in shim_wrote {
        report.wrote.push(p.display().to_string());
    }
    report.warns.extend(shim_warns);
    // M059 边界：Windows 注册是无引号正斜杠形态，oma 根路径含空格三 shell
    // 全裂（用户级注册路径固定在家目录，此坑随家目录出现）。
    if side == OsSide::Windows && oma.to_string_lossy().contains(' ') {
        report.warns.push(
            "oma home path contains spaces: Windows hook registrations are \
             unquoted forward-slash forms that break in every shell; relocate \
             ~/.oma (or the user profile) to a space-free path"
                .to_string(),
        );
    }
    deploy_claude_user(user_home, oma, side, report)?;
    deploy_codex_user(user_home, oma, side, report)?;
    deploy_grok_user(user_home, oma, side, report)?;
    deploy_kimi_user(user_home, oma, side, report)?;
    Ok(())
}

/// 生产入口：真实家目录 + oma 自管根。
pub fn deploy_user_hooks(report: &mut DeployReport) -> Result<(), String> {
    let user_home = crate::pathutil::user_home()?;
    let oma = crate::install::oma_home()?;
    deploy_user_hooks_with(&user_home, &oma, host_side(), report)
}

/// 项目面退役（D28）：摘除项目级 ours hook 注册（claude/codex/grok）、
/// 删除 oma 部署的项目 shim（`.oma/hooks/` 三件，带生成标记才删）。
/// kimi 项目侧无注册面；项目 `.oma/state/` 旧状态文件保留（状态栏旧协议
/// 兼容读，不属注册面）。
pub fn retire_project_hooks_with(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let root = crate::pathutil::abs_display(root);
    retire_json_file(
        &root.join(".claude").join("settings.json"),
        strip_ours_handlers,
        report,
    )?;
    retire_json_file(
        &root.join(".codex").join("hooks.json"),
        strip_ours_codex_handlers,
        report,
    )?;
    retire_json_file(
        &root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json"),
        strip_ours_handlers,
        report,
    )?;
    // 项目 shim 退役：oma 生成的三件删（内容带标记才动，用户自置同名文件
    // 不碰）；hooks 目录空了连目录摘。旧名 .ohmyagents 同查（D14 前部署）。
    for base in [root.join(".oma"), root.join(".ohmyagents")] {
        let hooks_dir = base.join("hooks");
        for name in ["oma-state.cmd", "oma-state-grok.cmd", "oma-state.sh"] {
            let p = hooks_dir.join(name);
            if let Ok(text) = fs::read_to_string(&p) {
                if text.contains("generated by oma init") {
                    fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
                    report.wrote.push(format!("{} (retired)", p.display()));
                }
            }
        }
        if hooks_dir.is_dir()
            && fs::read_dir(&hooks_dir)
                .map(|d| d.flatten().next().is_none())
                .unwrap_or(false)
        {
            let _ = fs::remove_dir(&hooks_dir);
        }
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

/// 命令图（S016「命令即 skill」）：意图到命令的映射，SKILL.md 由它生成。
/// 新增子命令在此补一行，`oma init` 重跑即同步（带生成标记才覆写）。
const COMMAND_MAP: &[(&str, &str)] = &[
    (
        "oma init [--project PATH]",
        "部署 hook/skill/yolo 键（幂等；hook 注册用户级常驻 ~/.oma/hooks/ shim，状态按 session 分键，项目旧注册自动退役，D28）",
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
        "配置四家状态栏（写入面幂等；状态由用户级 hook 落盘供给；--example 定制模板，--script 自备脚本整替换，--builtin 还原）",
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
    s.push_str("\n\n本项目由 oma 部署配置：hook 状态写用户级 `~/.oma/state/`（session 分键），供状态栏 `agent:state` 机读标记消费。\n\n");
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

const AGENTS_MD: &str = "# AGENTS\n\n本项目会话由 Oh My Agents（oma）编排：agent 状态写用户级 `~/.oma/state/`，委派与诊断经 oma CLI。\n";

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

/// Kimi 项目面：仅 skill 目录布局（S015：kimi 项目级 hook 注册不存在）。
fn deploy_kimi_project(root: &Path, report: &mut DeployReport) -> Result<(), String> {
    let dir = root.join(".kimi-code").join("skills").join("ohmyagents");
    ensure_parent(&dir.join("SKILL.md"))?;
    report.skipped.push(dir.display().to_string());
    Ok(())
}

/// Deploy the full init surface (D28)：用户级 hook 注册加 shim（真实家目录
/// 与 oma 根）、本项目旧注册与 shim 退役、项目级 skill 与说明。
pub fn deploy_all(root: &Path) -> Result<DeployReport, String> {
    let user_home = crate::pathutil::user_home()?;
    let oma = crate::install::oma_home()?;
    deploy_all_with(root, &user_home, &oma, host_side())
}

/// Test seam：user_home 与 oma 根注入（不碰真实家目录），side 注入双测。
pub fn deploy_all_with(
    root: &Path,
    user_home: &Path,
    oma: &Path,
    side: OsSide,
) -> Result<DeployReport, String> {
    // abs_display（非裸 canonicalize）：剥掉 Windows `\\?\` 前缀，路径要进
    // 注册命令与 codex 信任键，带前缀 cmd 侧不可执行。
    let root = crate::pathutil::abs_display(root);
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    let mut report = DeployReport::default();
    deploy_user_hooks_with(user_home, oma, side, &mut report)?;
    retire_project_hooks_with(&root, &mut report)?;
    deploy_skills(&root, &mut report)?;
    deploy_kimi_project(&root, &mut report)?;
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

    /// 收集 JSON 形注册的全部 command 串（跨事件全量）。
    fn collect_commands(p: &Path) -> Vec<String> {
        let v: Json = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();
        v["hooks"]
            .as_object()
            .unwrap()
            .values()
            .filter_map(|g| g.as_array())
            .flatten()
            .filter_map(|grp| grp.get("hooks").and_then(|h| h.as_array()))
            .flatten()
            .filter_map(|h| h.get("command").and_then(|c| c.as_str()).map(String::from))
            .collect()
    }

    /// 单事件内的 ours command 数（收敛判据按事件论：八事件各一条）。
    fn ours_in_event(p: &Path, event: &str) -> Vec<String> {
        let v: Json = serde_json::from_str(&fs::read_to_string(p).unwrap()).unwrap();
        v["hooks"][event]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|grp| grp.get("hooks").and_then(|h| h.as_array()))
            .flatten()
            .filter_map(|h| h.get("command").and_then(|c| c.as_str()))
            .filter(|c| is_ours(c))
            .map(String::from)
            .collect()
    }

    #[test]
    fn user_deploys_merge_and_are_idempotent() {
        let user = fresh_dir("user");
        let oma = fresh_dir("oma");
        // Foreign hook must survive every deploy.
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(
            &claude,
            r#"{"statusLine": {"type": "command", "command": "x"},
                "hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "C:\\tools\\fmt.sh"}]}]}}"#,
        )
        .unwrap();
        let kimi = user.join(".kimi-code").join("config.toml");
        ensure_parent(&kimi).unwrap();
        write_text(
            &kimi,
            "theme = \"dark\"\n\n[[hooks]]\nevent = \"Stop\"\ncommand = \"my-tool\"\ntimeout = 5\n",
        )
        .unwrap();

        let mut first = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut first).unwrap();
        assert_eq!(first.form.as_deref(), Some("user"));
        assert!(
            first.wrote.iter().any(|p| p.contains(".claude")),
            "claude user settings written: {:?}",
            first.wrote
        );
        assert!(oma.join("hooks").join("oma-state.cmd").exists());
        assert!(oma.join("hooks").join("oma-state.sh").exists());

        // statusLine 键与外来 hook 保留，ours 注册指向用户级 shim。
        let v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        assert!(v.get("statusLine").is_some(), "statusLine key survives");
        let stop = v["hooks"]["Stop"].as_array().unwrap();
        assert!(stop
            .iter()
            .any(|g| g["hooks"][0]["command"].as_str() == Some("C:\\tools\\fmt.sh")));
        let ours = stop
            .iter()
            .find(|g| {
                g["hooks"][0]["command"]
                    .as_str()
                    .unwrap()
                    .contains("oma-state")
            })
            .unwrap();
        let claude_cmd = ours["hooks"][0]["command"].as_str().unwrap();
        if cfg!(windows) {
            assert!(!claude_cmd.contains('"'), "{claude_cmd}");
            assert!(!claude_cmd.starts_with('&'), "{claude_cmd}");
            assert!(
                claude_cmd.ends_with("/hooks/oma-state.cmd claude"),
                "{claude_cmd}"
            );
        } else {
            assert!(
                claude_cmd.ends_with("oma-state.sh\" claude"),
                "{claude_cmd}"
            );
        }
        assert!(v["hooks"]["PermissionRequest"].is_array());

        // codex 用户层：hooks.json + config.toml features/trust。
        let codex: Json = serde_json::from_str(
            &fs::read_to_string(user.join(".codex").join("hooks.json")).unwrap(),
        )
        .unwrap();
        let handler = &codex["hooks"]["SessionEnd"][0]["hooks"][0];
        if cfg!(windows) {
            let cw = handler["commandWindows"].as_str().unwrap();
            assert!(!cw.contains('"') && !cw.starts_with('&'), "{cw}");
            assert!(cw.ends_with("/hooks/oma-state.cmd codex"), "{cw}");
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
        let codex_toml = fs::read_to_string(user.join(".codex").join("config.toml")).unwrap();
        assert!(codex_toml.contains("hooks = true"));
        assert!(
            codex_toml.contains("trusted_hash"),
            "user-layer trust seeded: {codex_toml}"
        );

        // grok 用户层 global hooks 文件。
        let grok_cmds = collect_commands(
            &user
                .join(".grok")
                .join("hooks")
                .join("ohmyagents-state.json"),
        );
        assert!(grok_cmds.iter().any(|c| c.contains("oma-state")));
        if cfg!(windows) {
            let g = grok_cmds.iter().find(|c| c.contains("oma-state")).unwrap();
            assert!(g.ends_with("oma-state-grok.cmd"), "M048 single path: {g}");
        }

        // kimi [[hooks]]：外来条目存活、ours 八事件补齐、strict 四字段。
        let kimi_toml = fs::read_to_string(&kimi).unwrap();
        assert!(kimi_toml.contains("theme"), "foreign key survives");
        assert!(kimi_toml.contains("\"my-tool\""), "foreign hook survives");
        assert!(kimi_toml.contains("oma-state"));
        let kv: toml::Value = toml::from_str(&kimi_toml).unwrap();
        let hooks = kv.get("hooks").and_then(|h| h.as_array()).unwrap();
        let ours: Vec<&toml::Value> = hooks
            .iter()
            .filter(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c.contains("oma-state"))
            })
            .collect();
        assert_eq!(ours.len(), 8, "eight events registered: {ours:?}");
        for h in &ours {
            assert!(h.get("event").is_some());
            assert_eq!(h.get("matcher"), None, "matcher omitted = match all");
            assert_eq!(
                h.get("timeout").and_then(|t| t.as_integer()),
                Some(10),
                "strict schema fields only"
            );
        }

        // 幂等：重部署零写入。
        let mut second = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut second).unwrap();
        assert!(
            second.wrote.is_empty(),
            "redeploy must write nothing: {:?}",
            second.wrote
        );

        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn legacy_and_project_forms_heal_to_user_shim() {
        // 用户级文件里的老形态（bare oma、旧 exe、D27 项目级 shim 路径）与
        // 重复条目：重部署一律收敛到用户级 shim 单条。
        let user = fresh_dir("heal");
        let oma = fresh_dir("oma-heal");
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(
            &claude,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "oma hook --agent claude"},
                {"type": "command", "command": "D:\\old\\oma.exe hook --agent claude"},
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\oma-state.cmd claude"}]}]}}"#,
        )
        .unwrap();

        let mut r = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r).unwrap();
        // 注入的三条旧形态都在 Stop 事件：收敛为现行单条；全文件无旧路径残留。
        let ours = ours_in_event(&claude, "Stop");
        assert_eq!(ours.len(), 1, "legacy forms collapse to one: {ours:?}");
        assert!(ours[0].contains("oma-state"), "{}", ours[0]);
        let cmds = collect_commands(&claude);
        assert!(!cmds.iter().any(|c| c.contains("D:\\old")), "old exe gone");
        assert!(
            !cmds.iter().any(|c| c.contains(".oma\\hooks")),
            "project shim path healed to user-level"
        );
        // 其余事件各一条现行注册。
        for event in ["SessionStart", "PreToolUse"] {
            assert_eq!(ours_in_event(&claude, event).len(), 1, "{event}");
        }
        // 幂等。
        let mut second = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut second).unwrap();
        assert!(second.wrote.is_empty(), "idempotent after healing");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn codex_user_duplicates_collapse_and_sides_preserve() {
        let user = fresh_dir("codexuser");
        let oma = fresh_dir("oma-cu");
        let path = user.join(".codex").join("hooks.json");
        ensure_parent(&path).unwrap();
        // 异侧字段保留语义沿用：Unix 侧写 command，Windows 侧只重写
        // commandWindows 且 command 字节保留。
        write_text(
            &path,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "\"/mnt/c/old/oma\" hook --agent codex",
                 "commandWindows": "& \"D:\\old2\\oma.exe\" hook --agent codex", "timeout": 10}]}]}}"#,
        )
        .unwrap();
        let mut r = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut r).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let h = &v["hooks"]["Stop"][0]["hooks"][0];
        assert_eq!(
            h["command"].as_str(),
            Some("\"/mnt/c/old/oma\" hook --agent codex"),
            "foreign-OS field preserved verbatim"
        );
        let cw = h["commandWindows"].as_str().unwrap();
        assert!(!cw.contains('"') && !cw.contains('&'), "{cw}");
        assert!(cw.ends_with("/hooks/oma-state.cmd codex"), "{cw}");
        assert!(!cw.contains("old2"), "owned field rewritten: {cw}");

        // 同形重复植入后收敛。
        let mut v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let canonical = v["hooks"]["Stop"][0]["hooks"][0].clone();
        v["hooks"]["Stop"][0]["hooks"] = Json::Array(vec![canonical.clone(), canonical]);
        fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
        let mut r2 = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, OsSide::Windows, &mut r2).unwrap();
        let v: Json = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let n = v["hooks"]["Stop"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .filter(|h| handler_is_ours(h))
            .count();
        assert_eq!(n, 1, "identical duplicates collapse: {v}");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn retire_strips_project_registrations_and_shims() {
        // v0.5.3 形项目（项目注册 + 项目 shim + 外来 hook）一次 init 后退役。
        let user = fresh_dir("retire-user");
        let oma = fresh_dir("retire-oma");
        let root = fresh_dir("retire-proj");
        // 项目级注册（v0.5.3 部署形态）。
        let claude = root.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(
            &claude,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "C:\\tools\\fmt.sh"},
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\oma-state.cmd claude"}]}]}}"#,
        )
        .unwrap();
        let codex = root.join(".codex").join("hooks.json");
        ensure_parent(&codex).unwrap();
        write_text(
            &codex,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "oma hook --agent codex"}]}]}}"#,
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
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\oma-state-grok.cmd"}]}]}}"#,
        )
        .unwrap();
        // 项目 shim 三件（带生成标记）+ 用户自置同名文件保护判据。
        let shims = root.join(".oma").join("hooks");
        fs::create_dir_all(&shims).unwrap();
        fs::write(shims.join("oma-state.cmd"), "rem generated by oma init\r\n").unwrap();
        fs::write(shims.join("oma-state.sh"), "# generated by oma init\n").unwrap();
        fs::write(
            shims.join("oma-state-grok.cmd"),
            "@echo off\r\nrem generated by oma init\r\n",
        )
        .unwrap();
        fs::write(shims.join("my-own.cmd"), "@echo off\r\n").unwrap();

        let report = deploy_all_with(&root, &user, &oma, host_side()).unwrap();

        // 外来 hook 保留、ours 全摘。
        let v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        let cmds = collect_commands(&claude);
        assert_eq!(
            cmds,
            vec!["C:\\tools\\fmt.sh".to_string()],
            "ours stripped, foreign kept"
        );
        assert!(
            v.get("hooks").is_some(),
            "hooks key stays while foreign hooks live"
        );
        // codex hooks.json 只剩 ours → 整文件删除。
        assert!(
            !codex.exists(),
            "ours-only hooks.json removed: {:?}",
            report.wrote
        );
        // grok 同理。
        assert!(!grok.exists(), "ours-only grok hooks removed");
        // shim：oma 生成的删、用户自置的留。
        assert!(!shims.join("oma-state.cmd").exists());
        assert!(!shims.join("oma-state.sh").exists());
        assert!(!shims.join("oma-state-grok.cmd").exists());
        assert!(shims.join("my-own.cmd").exists(), "user file untouched");
        // hooks 目录非空（my-own.cmd）不删。
        assert!(shims.is_dir());
        // skills 仍项目级部署。
        assert!(root
            .join(".agents")
            .join("skills")
            .join("ohmyagents")
            .join("SKILL.md")
            .exists());

        // 幂等：再跑零退役写入（用户级注册也零写入）。
        let second = deploy_all_with(&root, &user, &oma, host_side()).unwrap();
        assert!(
            second.wrote.is_empty(),
            "redeploy writes nothing: {:?}",
            second.wrote
        );

        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retire_keeps_user_owned_files_deletes_ours_only() {
        // 用户自己的 settings.json（无 ours）退役不动；只有 ours 的删文件。
        let root = fresh_dir("retire2");
        let claude = root.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        let user_body = r#"{"permissions": {"defaultMode": "acceptEdits"}}"#;
        write_text(&claude, user_body).unwrap();
        let ours_only = root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json");
        ensure_parent(&ours_only).unwrap();
        write_text(
            &ours_only,
            r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
                {"type": "command", "command": "D:\\proj\\.oma\\hooks\\oma-state-grok.cmd"}]}]}}"#,
        )
        .unwrap();
        let mut report = DeployReport::default();
        retire_project_hooks_with(&root, &mut report).unwrap();
        // 无 ours 的用户文件逐字节不动。
        assert_eq!(fs::read_to_string(&claude).unwrap(), user_body);
        // 只有 ours 的整文件删除。
        assert!(!ours_only.exists(), "ours-only file removed");
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
    fn identical_duplicate_ours_handlers_collapse_to_one() {
        // 同形重复条目收敛（claude 用户面）。
        let user = fresh_dir("dedup");
        let oma = fresh_dir("oma-dd");
        let claude = user.join(".claude").join("settings.json");
        ensure_parent(&claude).unwrap();
        write_text(&claude, r#"{"hooks": {}}"#).unwrap();
        let mut r = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r).unwrap();
        let mut v: Json = serde_json::from_str(&fs::read_to_string(&claude).unwrap()).unwrap();
        let canonical = v["hooks"]["Stop"][0]["hooks"][0].clone();
        let dup = json!({ "matcher": "*", "hooks": [canonical.clone(), canonical.clone()] });
        let single = json!({ "matcher": "*", "hooks": [canonical.clone()] });
        v["hooks"]["Stop"]
            .as_array_mut()
            .unwrap()
            .extend([single, dup.clone(), dup]);
        fs::write(&claude, serde_json::to_string(&v).unwrap()).unwrap();
        let mut r2 = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r2).unwrap();
        let ours = ours_in_event(&claude, "Stop");
        assert_eq!(ours.len(), 1, "duplicates must collapse: {ours:?}");
        let mut r3 = DeployReport::default();
        deploy_user_hooks_with(&user, &oma, host_side(), &mut r3).unwrap();
        assert!(
            r3.wrote.is_empty(),
            "idempotent after dedup: {:?}",
            r3.wrote
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
    }

    #[test]
    fn kimi_merge_dedupes_and_upgrades_bare_forms() {
        // bare oma 老条目升级为用户级 shim 形态，外来保留，幂等。
        let user = fresh_dir("kimi");
        let oma = fresh_dir("oma-kimi");
        let cfg = user.join(".kimi-code").join("config.toml");
        ensure_parent(&cfg).unwrap();
        write_text(
            &cfg,
            "[[hooks]]\nevent = \"Stop\"\ncommand = \"oma hook --agent kimi\"\ntimeout = 30\n\
             [[hooks]]\nevent = \"Stop\"\ncommand = \"my-tool\"\n",
        )
        .unwrap();
        let mut toml = read_toml(&cfg).unwrap();
        let cmd = kimi_hook_command(&oma, host_side());
        assert!(apply_kimi_hooks(&mut toml, &cmd, &["SessionStart", "Stop"]).unwrap());
        // 再跑不变（幂等判据）。
        assert!(!apply_kimi_hooks(&mut toml, &cmd, &["SessionStart", "Stop"]).unwrap());
        let arr = toml.get("hooks").and_then(|h| h.as_array()).unwrap();
        assert!(arr
            .iter()
            .any(|h| h.get("command").and_then(|c| c.as_str()) == Some("my-tool")));
        assert!(!arr
            .iter()
            .any(|h| h.get("command").and_then(|c| c.as_str()) == Some("oma hook --agent kimi")),
            "bare form upgraded away");
        assert_eq!(arr.len(), 3, "sessionstart + stop ours + foreign my-tool");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&oma);
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
