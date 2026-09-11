use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value as Json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use toml::Value as Toml;

use crate::agents;
use crate::pathutil::{abs_display, forward_slash, keys_match, native_slash};
use crate::yolo::kimi_workspace_key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    /// Deploy-diagnosis gap that does not block an interactive run (login
    /// missing, statusline off): surfaced for `oma doctor`,
    /// never counted by `blocked()`.
    Warn,
    Block,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Warn => "warn",
            Status::Block => "block",
        }
    }
}

#[derive(Debug)]
pub struct Finding {
    pub agent: String,
    pub check: &'static str,
    pub status: Status,
    pub path: String,
    pub detail: String,
}

#[derive(Debug)]
pub struct Diagnosis {
    pub findings: Vec<Finding>,
}

impl Diagnosis {
    pub fn blocked(&self) -> bool {
        self.findings.iter().any(|f| f.status == Status::Block)
    }

    pub fn status(&self, agent: &str, check: &str) -> Option<Status> {
        self.findings
            .iter()
            .find(|f| f.agent == agent && f.check == check)
            .map(|f| f.status)
    }
}

fn json_file(path: &Path) -> Option<Json> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn toml_file(path: &Path) -> Option<Toml> {
    let text = fs::read_to_string(path).ok()?;
    toml::from_str(&text).ok()
}

fn json_bool(v: &Json, key: &str) -> bool {
    v.get(key).and_then(|x| x.as_bool()) == Some(true)
}

fn push_binary(out: &mut Vec<Finding>, agent: &str) {
    match agents::find(agent) {
        Some(h) => {
            let mut detail = format!("source={}", h.source.as_str());
            if let Some(v) = &h.version {
                detail.push(' ');
                detail.push_str(v);
            }
            if !h.extras.is_empty() {
                detail.push_str(" extras=");
                detail.push_str(
                    &h.extras
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            push(out, agent, "binary", true, &h.path, detail);
        }
        None => push(
            out,
            agent,
            "binary",
            false,
            Path::new(agent),
            "not on PATH, OMA_AGENT_PATH, OMA_*_BIN, or default locations",
        ),
    }
}

fn push(
    out: &mut Vec<Finding>,
    agent: &str,
    check: &'static str,
    ok: bool,
    path: &Path,
    detail: impl Into<String>,
) {
    let status = if ok { Status::Ok } else { Status::Block };
    push_status(out, agent, check, status, path, detail);
}

fn push_status(
    out: &mut Vec<Finding>,
    agent: &str,
    check: &'static str,
    status: Status,
    path: &Path,
    detail: impl Into<String>,
) {
    out.push(Finding {
        agent: agent.to_string(),
        check,
        status,
        path: path.display().to_string(),
        detail: detail.into(),
    });
}

fn toml_str<'a>(t: &'a Toml, key: &str) -> Option<&'a str> {
    t.get(key).and_then(|v| v.as_str())
}

fn projects_trusted(toml: &Toml, root: &Path) -> bool {
    let Some(projects) = toml.get("projects").and_then(|v| v.as_table()) else {
        return false;
    };
    let native = native_slash(root);
    let fwd = forward_slash(root);
    for (k, v) in projects {
        if keys_match(k, &native) || keys_match(k, &fwd) {
            return toml_str(v, "trust_level") == Some("trusted");
        }
    }
    false
}

fn claude_project_entry<'a>(cj: &'a Json, root: &Path) -> Option<&'a Json> {
    let projects = cj.get("projects")?.as_object()?;
    let native = native_slash(root);
    let fwd = forward_slash(root);
    for (k, v) in projects {
        if keys_match(k, &native) || keys_match(k, &fwd) {
            return Some(v);
        }
    }
    None
}

fn has_hooks_settings(v: &Json) -> bool {
    v.get("hooks").and_then(|h| h.as_object()).is_some_and(|o| {
        o.values()
            .any(|x| x.as_array().is_some_and(|a| !a.is_empty()))
    })
}

fn project_mcp_configured(root: &Path, settings: Option<&Json>) -> bool {
    root.join(".mcp.json").is_file()
        || settings
            .and_then(|v| v.get("mcpServers"))
            .and_then(|m| m.as_object())
            .is_some_and(|o| !o.is_empty())
}

fn toml_mcp_configured(t: &Toml) -> bool {
    t.get("mcp_servers")
        .and_then(|v| v.as_table())
        .is_some_and(|m| !m.is_empty())
}

fn mcp_servers_approved(v: &Json) -> bool {
    json_bool(v, "enableAllProjectMcpServers")
        || v.get("enabledMcpjsonServers")
            .and_then(|x| x.as_array())
            .is_some_and(|a| a.iter().any(|s| s.as_str().is_some_and(|n| !n.is_empty())))
}

fn skill_dir_is_plugin(skills: &Path) -> bool {
    fs::read_dir(skills)
        .map(|rd| {
            rd.flatten().any(|e| {
                let p = e.path();
                p.join(".claude-plugin").join("plugin.json").is_file()
                    || p.join("plugin.json").is_file()
            })
        })
        .unwrap_or(false)
}

fn dir_nonempty(path: &Path) -> bool {
    path.is_dir()
        && fs::read_dir(path)
            .map(|rd| rd.flatten().next().is_some())
            .unwrap_or(false)
}

fn hook_state_trusted(toml: &Toml) -> bool {
    let Some(hooks) = toml.get("hooks").and_then(|v| v.as_table()) else {
        return false;
    };
    let Some(state) = hooks.get("state").and_then(|v| v.as_table()) else {
        return false;
    };
    state.values().any(|v| {
        v.get("trusted_hash")
            .and_then(|h| h.as_str())
            .is_some_and(|s| !s.is_empty())
    })
}

fn path_is_file(path: &Path) -> bool {
    path.is_file()
}

fn grok_folder_trusted(toml: &Toml, root: &Path) -> bool {
    let Some(folders) = toml.get("folders").and_then(|v| v.as_table()) else {
        return false;
    };
    let native = native_slash(root);
    let fwd = forward_slash(root);
    for (k, v) in folders {
        if keys_match(k, &native) || keys_match(k, &fwd) {
            return v.get("trusted").and_then(|x| x.as_bool()) == Some(true);
        }
    }
    false
}

fn kimi_trust_ok(home: &Path, root: &Path) -> bool {
    let key = kimi_workspace_key(root);
    let file = home.join(".kimi-code").join("workspace-trust").join(&key);
    let Some(v) = json_file(&file) else {
        return false;
    };
    let Some(stored) = v.get("root").and_then(|x| x.as_str()) else {
        return false;
    };
    keys_match(stored, &native_slash(root)) || keys_match(stored, &forward_slash(root))
}

// ===== 登录态（S026 判据）与部署诊断扩展 =====

/// grok 登录态：`~/.grok/auth.json` 是 scope → 凭据 map。判据来自 S026
/// 源码取证加本机文件结构实证：条目有 `key` 或 `refresh_token` 即有凭据；
/// 过期看 `expires_at`（RFC3339），缺省按 `create_time + 30 天`兜底，提前
/// 300s 视过期；过期但 refresh_token 在则 agent 下次运行自动刷新。
fn grok_login_state(v: Option<&Json>, now: OffsetDateTime) -> (Status, String) {
    let Some(map) = v.and_then(|v| v.as_object()) else {
        return (
            Status::Warn,
            "auth.json missing; grok login --device-code".into(),
        );
    };
    for (scope, cred) in map {
        let has_key = cred
            .get("key")
            .and_then(|x| x.as_str())
            .is_some_and(|s| !s.is_empty());
        let has_refresh = cred
            .get("refresh_token")
            .and_then(|x| x.as_str())
            .is_some_and(|s| !s.is_empty());
        if !has_key && !has_refresh {
            continue;
        }
        let raw_exp = cred.get("expires_at").and_then(|x| x.as_str());
        let expires = raw_exp
            .and_then(|s| OffsetDateTime::parse(s, &Rfc3339).ok())
            .or_else(|| {
                let created = cred.get("create_time").and_then(|x| x.as_str())?;
                OffsetDateTime::parse(created, &Rfc3339)
                    .ok()
                    .map(|t| t + time::Duration::days(30))
            });
        return match expires {
            Some(e) if e > now + time::Duration::seconds(300) => (
                Status::Ok,
                raw_exp
                    .map(|s| format!("scope={scope} expires_at={s}"))
                    .unwrap_or_else(|| format!("scope={scope}")),
            ),
            Some(_) if has_refresh => (
                Status::Warn,
                format!("scope={scope} expired; refresh_token present (auto-refresh on next run)"),
            ),
            Some(_) => (
                Status::Warn,
                format!("scope={scope} expired, no refresh_token; grok login --device-code"),
            ),
            None => (
                Status::Ok,
                format!("scope={scope} no expiry timestamps (treated live)"),
            ),
        };
    }
    (
        Status::Warn,
        "no credential entries; grok login --device-code".into(),
    )
}

/// kimi 登录态：`~/.kimi-code/credentials/kimi-code.json`。判据来自 S026
/// 源码取证：`hasToken()` 只看 access_token 非空（不看过期，刷新按动态
/// 阈值自动做）；空串是 401/403 墓碑（吊销态，需重登）；expires_at 是
/// Unix 秒。
fn kimi_login_state(v: Option<&Json>, now_secs: i64) -> (Status, String) {
    let Some(v) = v else {
        return (Status::Warn, "credentials file missing; kimi login".into());
    };
    match v.get("access_token").and_then(|x| x.as_str()) {
        Some(t) if !t.is_empty() => {
            let detail = match v.get("expires_at").and_then(|x| x.as_i64()) {
                Some(e) if e > now_secs => {
                    format!("access_token present, expires in {}s", e - now_secs)
                }
                Some(e) => format!(
                    "access_token present, expired {}s ago (auto-refresh threshold)",
                    now_secs - e
                ),
                None => "access_token present, no expires_at".to_string(),
            };
            (Status::Ok, detail)
        }
        Some(_) => (
            Status::Warn,
            "access_token empty-string tombstone (revoked); kimi login again".into(),
        ),
        None => (Status::Warn, "no access_token field; kimi login".into()),
    }
}

/// 当下登录态（grok/kimi）：只读诊断落盘凭据（登录引导面已随 D20 移除）。
pub(crate) fn login_state(agent: &str) -> Option<(Status, String)> {
    let home = crate::pathutil::user_home().ok()?;
    match agent {
        "grok" => {
            let p = home.join(".grok").join("auth.json");
            Some(grok_login_state(
                json_file(&p).as_ref(),
                OffsetDateTime::now_utc(),
            ))
        }
        "kimi" => {
            let p = home
                .join(".kimi-code")
                .join("credentials")
                .join("kimi-code.json");
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            Some(kimi_login_state(json_file(&p).as_ref(), now))
        }
        _ => None,
    }
}

// ===== 状态栏形态（S025 落位） =====

const STATUSLINE_MARKER: &str = "oma-statusline";

fn claude_statusline_on(home: &Path) -> bool {
    json_file(&home.join(".claude").join("settings.json"))
        .as_ref()
        .and_then(|v| v.get("statusLine"))
        .and_then(|s| s.get("command"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| c.contains(STATUSLINE_MARKER))
}

/// Codex `[tui].status_line` kinds. Command-argv is the oma 2026-09-01
/// misfit: Codex only accepts built-in item IDs and skips the rest, so
/// the bar goes empty (M045).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodexStatusline {
    Builtin,
    CommandArgv,
    Missing,
}

fn looks_like_codex_command_argv(s: &str) -> bool {
    s.contains(STATUSLINE_MARKER)
        || s == "pwsh"
        || s == "-NoProfile"
        || s == "-File"
        || s == "command"
}

fn is_codex_builtin_status_item(s: &str) -> bool {
    matches!(
        s,
        "run-state"
            | "status"
            | "model-name"
            | "model-with-reasoning"
            | "reasoning"
            | "current-dir"
            | "project-root"
            | "project"
            | "project-name"
            | "git-branch"
            | "pull-request-number"
            | "branch-changes"
            | "permissions"
            | "approval-mode"
            | "approval"
            | "context-remaining"
            | "context-used"
            | "context-window-size"
            | "used-tokens"
            | "total-input-tokens"
            | "total-output-tokens"
            | "five-hour-limit"
            | "weekly-limit"
            | "codex-version"
            | "thread-credits"
            | "estimated-thread-cost"
            | "session-id"
            | "thread-id"
            | "fast-mode"
            | "raw-output"
            | "thread-title"
            | "workspace-headline"
            | "task-progress"
    )
}

fn codex_statusline_state(home: &Path) -> CodexStatusline {
    let Some(parts) = toml_file(&home.join(".codex").join("config.toml"))
        .as_ref()
        .and_then(|t| t.get("tui"))
        .and_then(|tui| tui.get("status_line"))
        .and_then(|sl| sl.as_array())
        .cloned()
    else {
        return CodexStatusline::Missing;
    };
    if parts.is_empty() {
        return CodexStatusline::Missing;
    }
    if parts
        .iter()
        .any(|p| p.as_str().is_some_and(looks_like_codex_command_argv))
    {
        return CodexStatusline::CommandArgv;
    }
    if parts
        .iter()
        .any(|p| p.as_str().is_some_and(is_codex_builtin_status_item))
    {
        return CodexStatusline::Builtin;
    }
    CodexStatusline::Missing
}

fn kimi_statusline_on(home: &Path) -> bool {
    toml_file(&home.join(".kimi-code").join("tui.toml"))
        .as_ref()
        .and_then(|t| t.get("status_line"))
        .and_then(|sl| sl.get("command"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| c.contains(STATUSLINE_MARKER))
}

/// Grok `[ui.status_line].command` kinds. A `pwsh -File "..."` shell line
/// contains quotes, so Windows `Command::new(entire_string)` returns
/// ERROR_INVALID_NAME 123 and never falls back to a shell (M048).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrokStatusline {
    CmdPath,
    PwshFile,
    Missing,
}

fn grok_statusline_state(home: &Path) -> GrokStatusline {
    let toml = toml_file(&home.join(".grok").join("config.toml"));
    let Some(cmd) = toml
        .as_ref()
        .and_then(|t| t.get("ui"))
        .and_then(|ui| ui.get("status_line"))
        .and_then(|sl| sl.get("command"))
        .and_then(|c| c.as_str())
        .map(str::trim)
        .filter(|c| c.contains(STATUSLINE_MARKER))
    else {
        return GrokStatusline::Missing;
    };
    if cmd.ends_with("oma-statusline-grok.cmd") && !cmd.contains("pwsh") {
        return GrokStatusline::CmdPath;
    }
    if cmd.contains("pwsh") && cmd.contains("-File") {
        return GrokStatusline::PwshFile;
    }
    GrokStatusline::Missing
}

fn push_statusline(
    out: &mut Vec<Finding>,
    agent: &str,
    on: bool,
    cfg: &Path,
    script_ok: bool,
    pwsh_missing: bool,
) {
    let (status, mut detail) = if !on {
        (
            Status::Warn,
            format!("not configured; oma agents statusline {agent}"),
        )
    } else if !script_ok {
        (
            Status::Warn,
            "configured but oma-statusline.ps1 missing; rerun oma agents statusline".into(),
        )
    } else {
        (Status::Ok, "oma bar configured".into())
    };
    if pwsh_missing {
        detail.push_str("; pwsh not on PATH (bar will not render)");
    }
    push_status(out, agent, "statusline", status, cfg, detail);
}

// ===== hook 注册形态（P0027 口径） =====

/// 命令串首 token 解析（剥调用操作符与包裹引号）：`& "D:/x/oma-state.cmd" codex`
/// 与 `D:/x/oma-state.cmd codex` 都得 `D:/x/oma-state.cmd`。
fn command_target(c: &str) -> std::path::PathBuf {
    let first = c
        .trim_start()
        .trim_start_matches('&')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('"');
    std::path::PathBuf::from(first)
}

/// JSON 形 hook 注册（claude settings、grok ohmyagents-state.json）的 oma
/// 形态：shim（D27 自包含状态写入且脚本在位）/ shim-dead（指向 shim 但脚本
/// 缺失：项目搬迁或 shim 被删）/ bare（PATH 解析，D27 前跨环境形态）/
/// absolute（单环境）/ args（M047 病理）/ none。
fn json_hooks_form(v: Option<&Json>) -> &'static str {
    let Some(events) = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object()) else {
        return "none";
    };
    let mut ours = false;
    let mut shim = false;
    let mut shim_dead = false;
    let mut bare = false;
    let mut has_args = false;
    for group in events.values().filter_map(|g| g.as_array()).flatten() {
        let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
            continue;
        };
        for h in hooks {
            let Some(c) = h.get("command").and_then(|c| c.as_str()) else {
                continue;
            };
            if crate::deploy::is_ours(c) {
                ours = true;
                if h.get("args")
                    .and_then(|a| a.as_array())
                    .is_some_and(|a| !a.is_empty())
                {
                    has_args = true;
                }
                if c.contains("oma-state") {
                    if command_target(c).is_file() {
                        shim = true;
                    } else {
                        shim_dead = true;
                    }
                }
                if !c.contains('/') && !c.contains('\\') {
                    bare = true;
                }
            }
        }
    }
    if has_args {
        "args"
    } else if shim_dead && !shim {
        "shim-dead"
    } else if shim {
        "shim"
    } else if bare {
        "bare"
    } else if ours {
        "absolute"
    } else {
        "none"
    }
}

/// codex `.codex/hooks.json` 里 ours 处理器占据的 per-OS 字段：
/// command（Unix 侧）/ commandWindows（Windows 侧）。绝对路径是设计态
/// （hook exec 环境不继承 PATH，P0027）。
fn codex_hooks_sides(v: Option<&Json>) -> (bool, bool) {
    let Some(events) = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object()) else {
        return (false, false);
    };
    let mut unix_side = false;
    let mut win_side = false;
    for group in events.values().filter_map(|g| g.as_array()).flatten() {
        let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
            continue;
        };
        for h in hooks {
            if h.get("command")
                .and_then(|c| c.as_str())
                .is_some_and(crate::deploy::is_ours)
            {
                unix_side = true;
            }
            if h.get("commandWindows")
                .and_then(|c| c.as_str())
                .is_some_and(crate::deploy::is_ours)
            {
                win_side = true;
            }
        }
    }
    (unix_side, win_side)
}

/// codex 各侧有效字段（Windows 看 commandWindows、Unix 看 command）的形态
/// 分类（review 验收补：此前不分 shim / bare / absolute 一律 ok，死链也
/// ok，D08「写了配置仍 ok」同族）。None = 该侧无 ours 条目；多条 ours 取
/// 最差形态（shim 好、bare/absolute 次、shim-dead 最差）。
fn codex_side_form(v: Option<&Json>, windows_side: bool) -> Option<&'static str> {
    let events = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object())?;
    let key = if windows_side {
        "commandWindows"
    } else {
        "command"
    };
    let rank = |f: &str| match f {
        "shim" => 0,
        "bare" | "absolute" => 1,
        _ => 2, // shim-dead
    };
    let mut form: Option<&'static str> = None;
    for group in events.values().filter_map(|g| g.as_array()).flatten() {
        let Some(hooks) = group.get("hooks").and_then(|h| h.as_array()) else {
            continue;
        };
        for h in hooks {
            let Some(c) = h.get(key).and_then(|c| c.as_str()) else {
                continue;
            };
            if !crate::deploy::is_ours(c) {
                continue;
            }
            let f = if c.contains("oma-state") {
                if command_target(c).is_file() {
                    "shim"
                } else {
                    "shim-dead"
                }
            } else if !c.contains('/') && !c.contains('\\') {
                "bare"
            } else {
                "absolute"
            };
            form = Some(match form {
                None => f,
                Some(prev) if rank(prev) >= rank(f) => prev,
                Some(_) => f,
            });
        }
    }
    form
}

fn push_hooks_form(out: &mut Vec<Finding>, agent: &str, form: &str, path: &Path) {
    match form {
        "shim" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Ok,
            path,
            "form=shim (self-contained state writer in ~/.oma/hooks; user-level \
             registration, zero oma dependency, D27/D28)",
        ),
        "shim-dead" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "form=shim-dead (registration points at a missing ~/.oma/hooks script; \
             rerun oma init)",
        ),
        "bare" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "form=bare (pre-D27 registration pins the oma binary; oma init upgrades to shim)",
        ),
        "args" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "command+args is PowerShell ParserError under Grok (M047); oma init",
        ),
        "absolute" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "form=absolute (pre-D27 registration pins the oma binary; oma init upgrades to shim)",
        ),
        _ => push_status(
            out,
            agent,
            "hooks.form",
            Status::Warn,
            path,
            "no oma hooks; oma init deploys",
        ),
    }
}

/// kimi `[[hooks]]`（用户级 config.toml）里是否有 ours 条目（D28：kimi 的
/// 注册面只有用户级）。
fn kimi_hooks_ours(toml: Option<&Toml>) -> bool {
    toml.and_then(|t| t.get("hooks"))
        .and_then(|h| h.as_array())
        .is_some_and(|arr| {
            arr.iter().any(|h| {
                h.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(crate::deploy::is_ours)
            })
        })
}

/// 项目级 ours 注册残留提示（D28 退役迁移判据：v0.5.3 前的项目部署遗留）。
fn push_project_residue(out: &mut Vec<Finding>, agent: &str, ours: bool, path: &Path) {
    if ours {
        push_status(
            out,
            agent,
            "hooks.retired",
            Status::Warn,
            path,
            "project-level oma hooks remain; rerun oma init to retire (D28)",
        );
    }
}

/// Read-only. Does not attach, send-keys, or wait on TUI.
pub fn diagnose(root: &Path) -> Result<Diagnosis, String> {
    let root = abs_display(root);
    let home = crate::pathutil::user_home()?;
    let mut findings = Vec::new();

    // 部署诊断共享事实：状态栏脚本与 pwsh 探测一次（S025），登录态用统一
    // 时间基准（S026）。
    let oma_root = crate::install::oma_home().ok();
    let sl_script_ok = oma_root
        .as_deref()
        .map(crate::statusline::script_path)
        .is_some_and(|p| p.is_file());
    let sl_grok_ok = sl_script_ok
        && oma_root
            .as_deref()
            .map(crate::statusline::grok_cmd_path)
            .is_some_and(|p| p.is_file());
    let sl_pwsh_missing = !crate::statusline::pwsh_on_path();
    let now = OffsetDateTime::now_utc();
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // CPU 能力段（S021）：Bun 系要 AVX/AVX2、Rust 原生常要 AVX-512，缺了
    // 表现为 agent 启动即崩——先摆出事实面，探针异常退出另有分类。
    let caps = crate::caps::detect();
    findings.push(Finding {
        agent: "cpu".into(),
        check: "caps".into(),
        status: Status::Ok,
        path: caps.arch.into(),
        detail: crate::caps::caps_line(&caps),
    });

    let claude_shared = root.join(".claude").join("settings.json");
    let yolo_claude = json_file(&claude_shared)
        .as_ref()
        .and_then(|v| v.get("permissions"))
        .and_then(|p| p.get("defaultMode"))
        .and_then(|m| m.as_str())
        == Some("bypassPermissions");
    push(
        &mut findings,
        "claude",
        "yolo",
        yolo_claude,
        &claude_shared,
        if yolo_claude {
            "permissions.defaultMode=bypassPermissions"
        } else {
            "missing permissions.defaultMode=bypassPermissions (tool prompt will block)"
        },
    );

    let claude_local = root.join(".claude").join("settings.local.json");
    let user_claude = home.join(".claude").join("settings.json");
    let skip = json_file(&claude_local)
        .as_ref()
        .map(|v| json_bool(v, "skipDangerousModePermissionPrompt"))
        .unwrap_or(false)
        || json_file(&user_claude)
            .as_ref()
            .map(|v| json_bool(v, "skipDangerousModePermissionPrompt"))
            .unwrap_or(false);
    push(
        &mut findings,
        "claude",
        "skip_prompt",
        skip,
        if claude_local.exists() {
            &claude_local
        } else {
            &user_claude
        },
        if skip {
            "skipDangerousModePermissionPrompt=true"
        } else {
            "missing skipDangerousModePermissionPrompt (bypass confirm dialog)"
        },
    );

    let claude_json = home.join(".claude.json");
    let cj = json_file(&claude_json);
    let claude_entry = cj.as_ref().and_then(|v| claude_project_entry(v, &root));
    let folder_ok = claude_entry
        .map(|e| json_bool(e, "hasTrustDialogAccepted"))
        .unwrap_or(false);
    let onboard_ok = cj
        .as_ref()
        .map(|v| json_bool(v, "hasCompletedOnboarding"))
        .unwrap_or(false);
    push(
        &mut findings,
        "claude",
        "trust.project",
        folder_ok,
        &claude_json,
        if folder_ok {
            "hasTrustDialogAccepted (folder; parent path also counts)"
        } else {
            "folder TrustDialog would block (hasTrustDialogAccepted)"
        },
    );
    let claude_has_hooks = json_file(&claude_shared)
        .as_ref()
        .map(has_hooks_settings)
        .unwrap_or(false)
        || json_file(&claude_local)
            .as_ref()
            .map(has_hooks_settings)
            .unwrap_or(false);
    push(
        &mut findings,
        "claude",
        "trust.hooks",
        !claude_has_hooks || folder_ok,
        &claude_json,
        if !claude_has_hooks {
            "n/a no project hooks in .claude/settings*.json"
        } else if folder_ok {
            "covered_by trust.project (interactive holds settings hooks until folder trusted)"
        } else {
            "project hooks present; interactive would hold them until hasTrustDialogAccepted"
        },
    );
    let claude_mcp = project_mcp_configured(&root, json_file(&claude_shared).as_ref())
        || project_mcp_configured(&root, json_file(&claude_local).as_ref());
    let user_mcp_ok = json_file(&user_claude)
        .as_ref()
        .map(mcp_servers_approved)
        .unwrap_or(false);
    let local_mcp_ok = json_file(&claude_local)
        .as_ref()
        .map(mcp_servers_approved)
        .unwrap_or(false);
    let project_mcp_ok = json_file(&claude_shared)
        .as_ref()
        .map(mcp_servers_approved)
        .unwrap_or(false);
    let mcp_ok = user_mcp_ok || (folder_ok && (local_mcp_ok || project_mcp_ok));
    push(
        &mut findings,
        "claude",
        "trust.mcp",
        if claude_mcp { mcp_ok } else { true },
        if user_mcp_ok {
            &user_claude
        } else if claude_local.exists() {
            &claude_local
        } else {
            &claude_shared
        },
        if !claude_mcp {
            "n/a no project MCP servers"
        } else if user_mcp_ok {
            "enableAll/enabledMcpjsonServers in user settings (honored while untrusted)"
        } else if folder_ok && (local_mcp_ok || project_mcp_ok) {
            "enableAll/enabledMcpjsonServers after workspace trust"
        } else if local_mcp_ok || project_mcp_ok {
            "project/local MCP approval ignored until workspace trusted (v2.1.196+)"
        } else {
            "MCPServerApprovalDialog pending"
        },
    );
    let claude_skills_dir = root.join(".claude").join("skills");
    let claude_commands_dir = root.join(".claude").join("commands");
    let claude_skills = dir_nonempty(&claude_skills_dir) || dir_nonempty(&claude_commands_dir);
    let claude_skill_plugin = skill_dir_is_plugin(&claude_skills_dir);
    push(
        &mut findings,
        "claude",
        "trust.skill",
        !claude_skills || folder_ok,
        if dir_nonempty(&claude_skills_dir) {
            &claude_skills_dir
        } else {
            &claude_commands_dir
        },
        if !claude_skills {
            "n/a no .claude/skills or .claude/commands"
        } else if folder_ok && claude_skill_plugin {
            "covered_by trust.project (skills-dir plugin + folder trust)"
        } else if folder_ok {
            "covered_by trust.project (skills/commands load after folder trust)"
        } else if claude_skill_plugin {
            "skills-dir plugin and project skills blocked until hasTrustDialogAccepted"
        } else {
            "project skills/commands blocked until hasTrustDialogAccepted"
        },
    );
    push(
        &mut findings,
        "claude",
        "onboarding",
        onboard_ok,
        &claude_json,
        if onboard_ok {
            "hasCompletedOnboarding"
        } else {
            "onboarding dialog would block"
        },
    );
    push_binary(&mut findings, "claude");
    // D28：hook 注册用户级；形态看 ~/.claude/settings.json（项目级文件只查
    // 残留）。
    let claude_user_settings = home.join(".claude").join("settings.json");
    push_hooks_form(
        &mut findings,
        "claude",
        json_hooks_form(json_file(&claude_user_settings).as_ref()),
        &claude_user_settings,
    );
    let claude_proj_form = {
        let form = json_hooks_form(json_file(&claude_shared).as_ref());
        if form != "none" {
            form
        } else {
            json_hooks_form(json_file(&claude_local).as_ref())
        }
    };
    push_project_residue(
        &mut findings,
        "claude",
        claude_proj_form != "none",
        &claude_shared,
    );
    push_statusline(
        &mut findings,
        "claude",
        claude_statusline_on(&home),
        &home.join(".claude").join("settings.json"),
        sl_script_ok,
        sl_pwsh_missing,
    );

    let codex_proj = root.join(".codex").join("config.toml");
    let codex_user = home.join(".codex").join("config.toml");
    let proj_toml = toml_file(&codex_proj);
    let user_toml = toml_file(&codex_user);
    let yolo_codex = proj_toml
        .as_ref()
        .map(|t| {
            toml_str(t, "sandbox_mode") == Some("danger-full-access")
                && toml_str(t, "approval_policy") == Some("never")
        })
        .unwrap_or(false);
    push(
        &mut findings,
        "codex",
        "yolo",
        yolo_codex,
        &codex_proj,
        if yolo_codex {
            "sandbox_mode=danger-full-access approval_policy=never"
        } else {
            "missing project sandbox/approval yolo keys"
        },
    );
    // Codex only loads the project config layer after the user store trusts the
    // path. Project `.codex/config.toml` [projects] does not clear the dialog.
    let trust_codex = user_toml
        .as_ref()
        .map(|t| projects_trusted(t, &root))
        .unwrap_or(false);
    push(
        &mut findings,
        "codex",
        "trust.project",
        trust_codex,
        &codex_user,
        if trust_codex {
            "user projects trust_level=trusted"
        } else {
            "untrusted project skips .codex layer and shows trust dialog"
        },
    );
    // D28：hook 注册用户级 ~/.codex/hooks.json；信任预种在 ~/.codex/config.toml
    // 的 [hooks.state]（key source = 用户 config 路径）。项目 hooks.json 只查
    // 残留。
    let codex_user_hooks_path = home.join(".codex").join("hooks.json");
    let codex_user_hooks = json_file(&codex_user_hooks_path);
    let codex_user_ours = codex_hooks_sides(codex_user_hooks.as_ref()) != (false, false);
    let user_hook_hash = user_toml.as_ref().map(hook_state_trusted).unwrap_or(false);
    push(
        &mut findings,
        "codex",
        "trust.hooks",
        if codex_user_ours {
            user_hook_hash
        } else {
            true
        },
        &codex_user_hooks_path,
        if !codex_user_ours {
            "n/a no oma hooks in user hooks.json (yolo does not bypass hook trust)"
        } else if user_hook_hash {
            "user [hooks.state] trusted_hash present"
        } else {
            "hook trust untrusted|modified; need trusted_hash or --dangerously-bypass-hook-trust"
        },
    );
    let codex_skills = dir_nonempty(&root.join(".agents").join("skills"))
        || dir_nonempty(&root.join(".codex").join("skills"));
    push(
        &mut findings,
        "codex",
        "trust.skill",
        if codex_skills { trust_codex } else { true },
        &root.join(".agents").join("skills"),
        if !codex_skills {
            "n/a no .agents/skills or .codex/skills"
        } else if trust_codex {
            "covered_by trust.project"
        } else {
            "project skills skipped until trust.project"
        },
    );
    let codex_mcp_json = root.join(".mcp.json");
    let codex_mcp =
        path_is_file(&codex_mcp_json) || proj_toml.as_ref().is_some_and(toml_mcp_configured);
    push(
        &mut findings,
        "codex",
        "trust.mcp",
        !codex_mcp || trust_codex,
        if path_is_file(&codex_mcp_json) {
            &codex_mcp_json
        } else {
            &codex_proj
        },
        if !codex_mcp {
            "n/a no project MCP servers"
        } else if trust_codex {
            "covered_by trust.project (Codex skips project MCP until trusted)"
        } else {
            "project MCP skipped until trust.project"
        },
    );
    push_binary(&mut findings, "codex");
    let codex_json = codex_user_hooks;
    let (codex_unix, codex_win) = codex_hooks_sides(codex_json.as_ref());
    if codex_unix || codex_win {
        // 分侧辨形（review 验收补）：warn 判据只看宿主侧字段——非宿主侧在
        // 单环境部署下恒为 M055 bare 兜底（init 修不掉，warn 即空转）；两
        // 侧 shim-dead/absolute 都是真缺陷仍降 warn。
        let host_windows = cfg!(windows);
        let unix_form = codex_side_form(codex_json.as_ref(), false);
        let win_form = codex_side_form(codex_json.as_ref(), true);
        // 宿主侧必须 shim；非宿主侧 bare 是 M055 兜底设计态（不 warn），
        // absolute / shim-dead 任何一侧都是真缺陷（warn）。
        let side_ok = |f: &str, is_host: bool| {
            if is_host {
                f == "shim"
            } else {
                f != "shim-dead" && f != "absolute"
            }
        };
        let mut sides = Vec::new();
        let mut ok = true;
        if codex_unix {
            let f = unix_form.unwrap_or("absolute");
            ok &= side_ok(f, !host_windows);
            sides.push(format!("command(unix)={f}"));
        }
        if codex_win {
            let f = win_form.unwrap_or("absolute");
            ok &= side_ok(f, host_windows);
            sides.push(format!("commandWindows(windows)={f}"));
        }
        push_status(
            &mut findings,
            "codex",
            "hooks.form",
            if ok { Status::Ok } else { Status::Warn },
            &codex_user_hooks_path,
            format!(
                "per-OS fields ours: {} (shim by design, D27/D28; host side must be shim, \
                 dead/absolute anywhere = rerun oma init)",
                sides.join(", ")
            ),
        );
    } else {
        push_hooks_form(&mut findings, "codex", "none", &codex_user_hooks_path);
    }
    push_project_residue(
        &mut findings,
        "codex",
        codex_hooks_sides(json_file(&root.join(".codex").join("hooks.json")).as_ref())
            != (false, false),
        &root.join(".codex").join("hooks.json"),
    );
    {
        let cfg = home.join(".codex").join("config.toml");
        match codex_statusline_state(&home) {
            CodexStatusline::Builtin => push_status(
                &mut findings,
                "codex",
                "statusline",
                Status::Ok,
                &cfg,
                "oma bar configured (built-in item IDs)",
            ),
            CodexStatusline::CommandArgv => push_status(
                &mut findings,
                "codex",
                "statusline",
                Status::Warn,
                &cfg,
                "status_line is command-argv; Codex only accepts built-in item IDs (S016); oma agents statusline",
            ),
            CodexStatusline::Missing => push_statusline(
                &mut findings,
                "codex",
                false,
                &cfg,
                sl_script_ok,
                sl_pwsh_missing,
            ),
        }
    }

    let kimi_proj = root.join(".kimi-code").join("config.toml");
    let kimi_user = home.join(".kimi-code").join("config.toml");
    let kimi_mode = toml_file(&kimi_proj)
        .as_ref()
        .and_then(|t| toml_str(t, "default_permission_mode").map(|s| s.to_string()))
        .or_else(|| {
            toml_file(&kimi_user)
                .as_ref()
                .and_then(|t| toml_str(t, "default_permission_mode").map(|s| s.to_string()))
        });
    let yolo_kimi = matches!(kimi_mode.as_deref(), Some("yolo" | "auto"));
    push(
        &mut findings,
        "kimi",
        "yolo",
        yolo_kimi,
        if kimi_proj.exists() {
            &kimi_proj
        } else {
            &kimi_user
        },
        match kimi_mode.as_deref() {
            Some(m) => format!("default_permission_mode={m}"),
            None => "missing default_permission_mode auto|yolo".into(),
        },
    );
    let trust_kimi = kimi_trust_ok(&home, &root);
    let kimi_trust_path = home
        .join(".kimi-code")
        .join("workspace-trust")
        .join(kimi_workspace_key(&root));
    push(
        &mut findings,
        "kimi",
        "trust.project",
        trust_kimi,
        &kimi_trust_path,
        if trust_kimi {
            "workspace-trust file for this root"
        } else {
            "folder trust dialog defaults to Don't trust"
        },
    );
    push(
        &mut findings,
        "kimi",
        "trust.hooks",
        true,
        &kimi_user,
        "n/a no hook-trust dialog (hooks live in user config.toml)",
    );
    let kimi_skills_dir = root.join(".kimi-code").join("skills");
    let agents_skills_dir = root.join(".agents").join("skills");
    let kimi_skills = dir_nonempty(&kimi_skills_dir) || dir_nonempty(&agents_skills_dir);
    push(
        &mut findings,
        "kimi",
        "trust.skill",
        !kimi_skills || trust_kimi,
        if dir_nonempty(&kimi_skills_dir) {
            &kimi_skills_dir
        } else {
            &agents_skills_dir
        },
        if !kimi_skills {
            "n/a no project skills"
        } else if trust_kimi {
            "covered_by trust.project"
        } else {
            "project skills present; folder trust dialog defaults to Don't trust"
        },
    );
    push_binary(&mut findings, "kimi");
    let kimi_cred = home
        .join(".kimi-code")
        .join("credentials")
        .join("kimi-code.json");
    let (kimi_login_st, kimi_login_detail) =
        kimi_login_state(json_file(&kimi_cred).as_ref(), now_secs);
    push_status(
        &mut findings,
        "kimi",
        "login",
        kimi_login_st,
        &kimi_cred,
        kimi_login_detail,
    );
    // D28：kimi 注册面只有用户级 ~/.kimi-code/config.toml [[hooks]]。
    if kimi_hooks_ours(toml_file(&kimi_user).as_ref()) {
        push_status(
            &mut findings,
            "kimi",
            "hooks.form",
            Status::Ok,
            &kimi_user,
            "form=shim (user-level [[hooks]] points at ~/.oma/hooks; D28)",
        );
    } else {
        push_hooks_form(&mut findings, "kimi", "none", &kimi_user);
    }
    push_statusline(
        &mut findings,
        "kimi",
        kimi_statusline_on(&home),
        &home.join(".kimi-code").join("tui.toml"),
        sl_script_ok,
        sl_pwsh_missing,
    );

    let grok_cfg = home.join(".grok").join("config.toml");
    let grok_tf = home.join(".grok").join("trusted_folders.toml");
    let grok_mode = toml_file(&grok_cfg).as_ref().and_then(|t| {
        t.get("ui")
            .and_then(|ui| toml_str(ui, "permission_mode"))
            .or_else(|| toml_str(t, "permission_mode"))
            .map(|s| s.to_string())
    });
    let yolo_grok = grok_mode.as_deref() == Some("always-approve");
    push(
        &mut findings,
        "grok",
        "yolo",
        yolo_grok,
        &grok_cfg,
        match grok_mode.as_deref() {
            Some(m) => format!("permission_mode={m} (user config only)"),
            None => "missing [ui] permission_mode=always-approve in ~/.grok/config.toml".into(),
        },
    );
    let trust_grok = toml_file(&grok_tf)
        .as_ref()
        .map(|t| grok_folder_trusted(t, &root))
        .unwrap_or(false);
    let grok_markers = path_is_file(&root.join(".mcp.json"))
        || path_is_file(&root.join(".envrc"))
        || path_is_file(&root.join(".cursor").join("mcp.json"))
        || path_is_file(&root.join(".cursor").join("hooks.json"))
        || path_is_file(&root.join(".grok").join("lsp.json"))
        || path_is_file(&claude_shared)
        || path_is_file(&claude_local)
        || dir_nonempty(&root.join(".grok").join("hooks"))
        || dir_nonempty(&root.join(".grok").join("plugins"))
        || dir_nonempty(&root.join(".grok").join("agents"))
        || dir_nonempty(&root.join(".claude").join("agents"))
        || dir_nonempty(&root.join(".grok").join("roles"))
        || dir_nonempty(&root.join(".grok").join("personas"))
        || dir_nonempty(&root.join(".grok").join("workflows"));
    push(
        &mut findings,
        "grok",
        "trust.project",
        trust_grok || !grok_markers,
        &grok_tf,
        if trust_grok {
            "trusted_folders.toml trusted=true (MCP/LSP/hooks/plugins share this store)"
        } else if !grok_markers {
            "n/a no repo-local code-exec configs; Grok skips the prompt"
        } else {
            "folder trust missing; --trust writes the same store"
        },
    );
    let grok_hooks = dir_nonempty(&root.join(".grok").join("hooks"))
        || path_is_file(&root.join(".cursor").join("hooks.json"));
    push(
        &mut findings,
        "grok",
        "trust.hooks",
        if grok_hooks { trust_grok } else { true },
        &root.join(".grok").join("hooks"),
        if !grok_hooks {
            "n/a no project .grok/hooks (empty repo skips the prompt)"
        } else if trust_grok {
            "covered_by trust.project"
        } else {
            "project hooks silently skipped until folder trusted"
        },
    );
    let grok_mcp = path_is_file(&root.join(".mcp.json"))
        || path_is_file(&root.join(".cursor").join("mcp.json"));
    push(
        &mut findings,
        "grok",
        "trust.mcp",
        if grok_mcp { trust_grok } else { true },
        &root.join(".mcp.json"),
        if !grok_mcp {
            "n/a no .mcp.json (other mcp markers still use folder store)"
        } else if trust_grok {
            "covered_by trust.project"
        } else {
            "repo-local MCP gated by folder trust"
        },
    );
    let grok_skills = dir_nonempty(&root.join(".grok").join("skills"));
    push(
        &mut findings,
        "grok",
        "trust.skill",
        true,
        &root.join(".grok").join("skills"),
        if grok_skills {
            "n/a skills are not a folder-trust trigger (source kinds omit skills)"
        } else {
            "n/a no .grok/skills"
        },
    );
    push_binary(&mut findings, "grok");
    let grok_auth = home.join(".grok").join("auth.json");
    let (grok_login_st, grok_login_detail) = grok_login_state(json_file(&grok_auth).as_ref(), now);
    push_status(
        &mut findings,
        "grok",
        "login",
        grok_login_st,
        &grok_auth,
        grok_login_detail,
    );
    // D28：grok 注册面在全局层 ~/.grok/hooks/*.json；项目文件只查残留。
    let grok_state_json = home
        .join(".grok")
        .join("hooks")
        .join("ohmyagents-state.json");
    push_hooks_form(
        &mut findings,
        "grok",
        json_hooks_form(json_file(&grok_state_json).as_ref()),
        &grok_state_json,
    );
    push_project_residue(
        &mut findings,
        "grok",
        json_hooks_form(
            json_file(
                &root
                    .join(".grok")
                    .join("hooks")
                    .join("ohmyagents-state.json"),
            )
            .as_ref(),
        ) != "none",
        &root
            .join(".grok")
            .join("hooks")
            .join("ohmyagents-state.json"),
    );
    match grok_statusline_state(&home) {
        GrokStatusline::CmdPath => {
            #[cfg(windows)]
            push_statusline(
                &mut findings,
                "grok",
                true,
                &grok_cfg,
                sl_grok_ok,
                sl_pwsh_missing,
            );
            #[cfg(not(windows))]
            push_status(
                &mut findings,
                "grok",
                "statusline",
                Status::Warn,
                &grok_cfg,
                "status_line is a Windows .cmd path; oma agents statusline grok",
            );
        }
        GrokStatusline::PwshFile => {
            #[cfg(windows)]
            push_status(
                &mut findings,
                "grok",
                "statusline",
                Status::Warn,
                &grok_cfg,
                "status_line is pwsh -File shell line; Grok Command::new paints os error 123 (M048); oma agents statusline grok",
            );
            #[cfg(not(windows))]
            push_statusline(
                &mut findings,
                "grok",
                true,
                &grok_cfg,
                sl_script_ok,
                sl_pwsh_missing,
            );
        }
        GrokStatusline::Missing => push_statusline(
            &mut findings,
            "grok",
            false,
            &grok_cfg,
            sl_grok_ok,
            sl_pwsh_missing,
        ),
    }

    // D28 用户级状态面：`~/.oma/state/*.json`。用户级状态无法归因到本项
    // 目（可能来自任何项目的会话），blocked 一律 warn 不 block——doctor
    // 的 Block 语义仍只对本项目交互阻塞负责（项目级旧状态文件照旧扫，
    // blocked = Block）。
    if let Some(user_state) = crate::install::oma_home().ok().map(|h| h.join("state")) {
        if user_state.is_dir() {
            if let Ok(rd) = fs::read_dir(&user_state) {
                for ent in rd.flatten() {
                    let p = ent.path();
                    let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    if !name.ends_with(".json") {
                        continue;
                    }
                    let Some(agent) = ["claude", "codex", "grok", "kimi"].iter().find(|a| {
                        name == format!("{a}.json") || name.starts_with(&format!("{a}-"))
                    }) else {
                        continue;
                    };
                    let keyed = name.contains('-');
                    let blocked = json_file(&p)
                        .as_ref()
                        .and_then(|v| v.get("state"))
                        .and_then(|s| s.as_str())
                        == Some("blocked");
                    findings.push(Finding {
                        agent: agent.to_string(),
                        check: "state",
                        status: if blocked { Status::Warn } else { Status::Ok },
                        path: p.display().to_string(),
                        detail: if blocked {
                            if keyed {
                                "state=blocked (session-keyed, project unknown)".into()
                            } else {
                                "state=blocked (user-level latest, project unknown)".into()
                            }
                        } else {
                            "state not blocked".into()
                        },
                    });
                }
            }
        }
    }

    let state_dir = crate::pathutil::project_dir(&root).join("state");
    if state_dir.is_dir() {
        if let Ok(rd) = fs::read_dir(&state_dir) {
            for ent in rd.flatten() {
                let p = ent.path();
                if p.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let blocked = json_file(&p)
                    .as_ref()
                    .and_then(|v| v.get("state"))
                    .and_then(|s| s.as_str())
                    == Some("blocked");
                let agent = p.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
                findings.push(Finding {
                    agent: agent.to_string(),
                    check: "state",
                    status: if blocked { Status::Block } else { Status::Ok },
                    path: p.display().to_string(),
                    detail: if blocked {
                        "state=blocked".into()
                    } else {
                        "state not blocked".into()
                    },
                });
            }
        }
    }

    Ok(Diagnosis { findings })
}

pub fn print_diagnosis(d: &Diagnosis) {
    for f in &d.findings {
        println!(
            "agent={} check={} status={} path={} detail={}",
            f.agent,
            f.check,
            f.status.as_str(),
            f.path,
            f.detail.replace('\n', " ")
        );
    }
    println!("doctor.blocked={}", d.blocked());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    #[test]
    fn state_blocked_is_a_finding() {
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-state-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let state = crate::pathutil::project_dir(&root).join("state");
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join("codex.json"), r#"{"state":"blocked"}"#).unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("codex", "state"), Some(Status::Block));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn user_level_state_never_blocks_project_diagnosis() {
        // D28 用户级状态面：OMA_HOME 缝注入（共享 env 锁与 hook 测试互斥）。
        // 用户级 blocked（latest 与 session 键两种）不升 Block——无法归因本
        // 项目；项目级旧文件 blocked 仍 Block。
        let _g = crate::pathutil::ENV_LOCK.lock().unwrap();
        let oma = temp_root("ustate");
        fs::create_dir_all(oma.join("state")).unwrap();
        fs::write(
            oma.join("state").join("claude.json"),
            r#"{"state":"blocked"}"#,
        )
        .unwrap();
        fs::write(
            oma.join("state").join("kimi-s9.json"),
            r#"{"state":"blocked"}"#,
        )
        .unwrap();
        fs::write(
            oma.join("state").join("grok.json"),
            r#"{"state":"working"}"#,
        )
        .unwrap();
        std::env::set_var("OMA_HOME", &oma);
        let root = temp_root("ustate-proj");
        let proj_state = crate::pathutil::project_dir(&root).join("state");
        fs::create_dir_all(&proj_state).unwrap();
        fs::write(proj_state.join("codex.json"), r#"{"state":"blocked"}"#).unwrap();
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("OMA_HOME");
        // 用户级 blocked：warn 不 block。
        assert_eq!(d.status("claude", "state"), Some(Status::Warn));
        let kimi_rows: Vec<&Finding> = d
            .findings
            .iter()
            .filter(|f| f.agent == "kimi" && f.check == "state")
            .collect();
        assert!(
            kimi_rows.iter().any(|f| f.status == Status::Warn),
            "keyed blocked row is warn: {kimi_rows:?}"
        );
        // 正常态照旧 ok。
        assert_eq!(d.status("grok", "state"), Some(Status::Ok));
        // 项目级旧文件 blocked 仍 Block（本项目归因）。
        assert_eq!(d.status("codex", "state"), Some(Status::Block));
        assert!(d.blocked());
        let _ = fs::remove_dir_all(&oma);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn empty_project_splits_trust_kinds() {
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-kinds-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(&root).unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(d.status("claude", "trust.hooks"), Some(Status::Ok));
        assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Ok));
        assert_eq!(d.status("claude", "trust.skill"), Some(Status::Ok));
        assert_eq!(d.status("kimi", "trust.hooks"), Some(Status::Ok));
        assert_eq!(d.status("grok", "trust.skill"), Some(Status::Ok));
        assert_eq!(d.status("grok", "trust.project"), Some(Status::Ok));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn claude_mcp_project_approval_ignored_until_folder_trust() {
        let _g = crate::pathutil::ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-mcp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(root.join(".claude")).unwrap();
        fs::write(
            root.join(".mcp.json"),
            r#"{"mcpServers":{"demo":{"command":"echo"}}}"#,
        )
        .unwrap();
        fs::write(
            root.join(".claude").join("settings.json"),
            r#"{"enableAllProjectMcpServers":true}"#,
        )
        .unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "trust.project"), Some(Status::Block));
        let user = dirs::home_dir()
            .unwrap()
            .join(".claude")
            .join("settings.json");
        let user_ok = json_file(&user)
            .as_ref()
            .map(mcp_servers_approved)
            .unwrap_or(false);
        if user_ok {
            assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Ok));
        } else {
            assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Block));
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn regular_claude_skills_block_without_folder_trust() {
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-skill-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(root.join(".claude").join("skills").join("demo")).unwrap();
        fs::write(
            root.join(".claude")
                .join("skills")
                .join("demo")
                .join("SKILL.md"),
            "# demo\n",
        )
        .unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(d.status("claude", "trust.skill"), Some(Status::Block));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn yolo_alone_does_not_clear_mcp_or_skill_trust() {
        let _g = crate::pathutil::ENV_LOCK.lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "oma-doctor-yolo-gates-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        fs::create_dir_all(root.join(".claude").join("skills").join("demo")).unwrap();
        fs::write(
            root.join(".claude")
                .join("skills")
                .join("demo")
                .join("SKILL.md"),
            "# demo\n",
        )
        .unwrap();
        fs::write(
            root.join(".mcp.json"),
            r#"{"mcpServers":{"demo":{"command":"echo"}}}"#,
        )
        .unwrap();
        crate::yolo::apply_project_yolo(&root).expect("yolo");
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("claude", "yolo"), Some(Status::Ok));
        assert_eq!(d.status("claude", "trust.skill"), Some(Status::Block));
        let user = dirs::home_dir()
            .unwrap()
            .join(".claude")
            .join("settings.json");
        let user_ok = json_file(&user)
            .as_ref()
            .map(mcp_servers_approved)
            .unwrap_or(false);
        if user_ok {
            assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Ok));
        } else {
            assert_eq!(d.status("claude", "trust.mcp"), Some(Status::Block));
        }
        let _ = fs::remove_dir_all(&root);
    }

    // ===== 部署诊断扩展（S025/S026 判据） =====

    fn temp_root(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "oma-doctor-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ))
    }

    #[test]
    fn grok_login_covers_s026_rules() {
        let now = OffsetDateTime::parse("2026-09-02T12:00:00Z", &Rfc3339).unwrap();
        let live =
            json!({"https://auth.x.ai::u": {"key": "k", "expires_at": "2026-09-02T13:00:00Z"}});
        assert_eq!(grok_login_state(Some(&live), now).0, Status::Ok);
        let refreshable =
            json!({"s": {"key": "k", "refresh_token": "r", "expires_at": "2026-09-02T11:00:00Z"}});
        let (st, detail) = grok_login_state(Some(&refreshable), now);
        assert_eq!(st, Status::Warn);
        assert!(detail.contains("refresh_token"));
        // 提前 300s 视过期：now+299s 落过期分支，now+301s 存活
        let edge_in = json!({"s": {"key": "k", "expires_at": "2026-09-02T12:04:59Z"}});
        assert_eq!(grok_login_state(Some(&edge_in), now).0, Status::Warn);
        let edge_out = json!({"s": {"key": "k", "expires_at": "2026-09-02T12:05:01Z"}});
        assert_eq!(grok_login_state(Some(&edge_out), now).0, Status::Ok);
        // 无 expires_at 按 create_time + 30 天兜底（2026-08-05 加 30 天 = 09-04）
        let created_live = json!({"s": {"key": "k", "create_time": "2026-08-05T00:00:00Z"}});
        assert_eq!(grok_login_state(Some(&created_live), now).0, Status::Ok);
        let created_stale = json!({"s": {"key": "k", "create_time": "2026-07-01T00:00:00Z"}});
        let (st, detail) = grok_login_state(Some(&created_stale), now);
        assert_eq!(st, Status::Warn);
        assert!(detail.contains("grok login"));
        assert_eq!(grok_login_state(None, now).0, Status::Warn);
        let no_creds = json!({"s": {"email": "x@y"}});
        assert_eq!(grok_login_state(Some(&no_creds), now).0, Status::Warn);
    }

    #[test]
    fn kimi_login_tombstone_is_distinct_from_missing() {
        let now = 1_800_000_000i64;
        let live = json!({"access_token": "t", "refresh_token": "r", "expires_at": now + 3600});
        assert_eq!(kimi_login_state(Some(&live), now).0, Status::Ok);
        // hasToken 不看过期（S026）：过期仍有 token 仍是登录态，刷新自动做
        let expired = json!({"access_token": "t", "expires_at": now - 10});
        assert_eq!(kimi_login_state(Some(&expired), now).0, Status::Ok);
        let tombstone = json!({"access_token": "", "expires_at": now});
        let (st, detail) = kimi_login_state(Some(&tombstone), now);
        assert_eq!(st, Status::Warn);
        assert!(detail.contains("revoked"));
        assert_eq!(kimi_login_state(None, now).0, Status::Warn);
        let no_field = json!({"refresh_token": "r"});
        assert_eq!(kimi_login_state(Some(&no_field), now).0, Status::Warn);
    }

    #[test]
    fn hooks_form_classifies_shim_bare_absolute_none() {
        // shim 判定含在位探针：夹具真文件判 shim，死链判 shim-dead（review
        // 验收补：此前死链也 ok）。
        let dir = std::env::temp_dir().join(format!(
            "oma-doctor-shim-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let alive = dir.join(".oma").join("hooks").join("oma-state.cmd");
        std::fs::create_dir_all(alive.parent().unwrap()).unwrap();
        std::fs::write(&alive, "@echo off\r\n").unwrap();
        let alive_fwd = alive.to_string_lossy().replace('\\', "/");
        let shim = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": format!("{alive_fwd} claude")}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&shim)), "shim");
        let shim_grok = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": format!("& \"{alive_fwd}\" grok")}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&shim_grok)), "shim");
        let dead = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": "D:/moved-away/.oma/hooks/oma-state.cmd claude"}
        ]}]}});
        assert_eq!(json_hooks_form(Some(&dead)), "shim-dead");
        let _ = std::fs::remove_dir_all(&dir);
        let bare = json!({"hooks": {"SessionStart": [{"hooks": [{"command": "oma hook --agent claude"}]}]}});
        assert_eq!(json_hooks_form(Some(&bare)), "bare");
        let absolute =
            json!({"hooks": {"SessionStart": [{"hooks": [{"command": "D:\\x\\oma.exe hook"}]}]}});
        assert_eq!(json_hooks_form(Some(&absolute)), "absolute");
        let foreign = json!({"hooks": {"SessionStart": [{"hooks": [{"command": "echo hi"}]}]}});
        assert_eq!(json_hooks_form(Some(&foreign)), "none");
        assert_eq!(json_hooks_form(None), "none");
        let args = json!({"hooks": {"SessionStart": [{"hooks": [{
            "command": "oma",
            "args": ["hook", "--agent", "claude"]
        }]}]}});
        assert_eq!(
            json_hooks_form(Some(&args)),
            "args",
            "Grok loads Claude settings; command+args is ParserError (M047)"
        );
    }

    #[test]
    fn codex_side_form_classifies_shim_bare_dead() {
        // 期望来自注册形态语义（M059/M055）：shim 需脚本在位；bare 无路径
        // 分隔符；shim-dead 指 oma-state 但文件缺失；absolute 其余带路径。
        let dir = std::env::temp_dir().join(format!(
            "oma-doctor-codex-form-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let alive = dir.join(".oma").join("hooks").join("oma-state.cmd");
        std::fs::create_dir_all(alive.parent().unwrap()).unwrap();
        std::fs::write(&alive, "@echo off\r\n").unwrap();
        let fwd = alive.to_string_lossy().replace('\\', "/");
        let shim_win = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": "oma hook --agent codex", "commandWindows": format!("{fwd} codex")}
        ]}]}});
        assert_eq!(codex_side_form(Some(&shim_win), true), Some("shim"));
        // M055 兜底：command 是 bare（设计态，非宿主侧不 warn 的判据基础）。
        assert_eq!(codex_side_form(Some(&shim_win), false), Some("bare"));
        let dead = json!({"hooks": {"SessionStart": [{"hooks": [
            {"command": "\"/p/x/.oma/hooks/oma-state.sh\" codex"}
        ]}]}});
        assert_eq!(codex_side_form(Some(&dead), false), Some("shim-dead"));
        let absolute = json!({"hooks": {"SessionStart": [{"hooks": [
            {"commandWindows": "\"D:\\old\\oma.exe\" hook --agent codex"}
        ]}]}});
        assert_eq!(codex_side_form(Some(&absolute), true), Some("absolute"));
        // 多条 ours 取最差：shim 加 dead 共存判 dead。
        let mixed = json!({"hooks": {"SessionStart": [
            {"hooks": [{"commandWindows": format!("{fwd} codex")}]},
            {"hooks": [{"commandWindows": "D:/moved/.oma/hooks/oma-state.cmd codex"}]}
        ]}});
        assert_eq!(codex_side_form(Some(&mixed), true), Some("shim-dead"));
        assert_eq!(codex_side_form(None, true), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn codex_hooks_sides_read_per_os_fields() {
        let both = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command",
             "command": "\"/home/ray/.cargo/bin/oma\" hook --agent codex",
             "commandWindows": "& \"D:\\cargo\\oma.exe\" hook --agent codex",
             "timeout": 10}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&both)), (true, true));
        let unix_only = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "command": "\"/home/ray/.cargo/bin/oma\" hook --agent codex"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&unix_only)), (true, false));
        let win_only = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "commandWindows": "& \"D:\\cargo\\oma.exe\" hook --agent codex"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&win_only)), (false, true));
        let foreign = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "command": "echo hi"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&foreign)), (false, false));
        assert_eq!(codex_hooks_sides(None), (false, false));
    }

    #[test]
    fn codex_statusline_state_classifies_argv_builtin_missing() {
        let root = temp_root("codex-sl");
        fs::create_dir_all(root.join(".codex")).unwrap();
        assert_eq!(codex_statusline_state(&root), CodexStatusline::Missing);
        fs::write(
            root.join(".codex").join("config.toml"),
            "[tui]\nstatus_line = [\"command\", \"pwsh\", \"-File\", \"C:/x/oma-statusline.ps1\"]\n",
        )
        .unwrap();
        assert_eq!(codex_statusline_state(&root), CodexStatusline::CommandArgv);
        fs::write(
            root.join(".codex").join("config.toml"),
            "[tui]\nstatus_line = [\"run-state\", \"git-branch\"]\n",
        )
        .unwrap();
        assert_eq!(codex_statusline_state(&root), CodexStatusline::Builtin);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn grok_statusline_state_classifies_cmd_pwsh_missing() {
        // Oracle: grok-build command.rs Command::new(entire string); Windows
        // shell line with quotes is ERROR_INVALID_NAME 123 (M048).
        let root = temp_root("grok-sl");
        fs::create_dir_all(root.join(".grok")).unwrap();
        assert_eq!(grok_statusline_state(&root), GrokStatusline::Missing);
        fs::write(
            root.join(".grok").join("config.toml"),
            "[ui.status_line]\ntype = \"command\"\ncommand = \"pwsh -NoProfile -File \\\"C:/x/oma-statusline.ps1\\\" grok\"\n",
        )
        .unwrap();
        assert_eq!(grok_statusline_state(&root), GrokStatusline::PwshFile);
        fs::write(
            root.join(".grok").join("config.toml"),
            "[ui.status_line]\ntype = \"command\"\ncommand = \"C:/Users/ray/.ohmyagents/statusline/oma-statusline-grok.cmd\"\n",
        )
        .unwrap();
        assert_eq!(grok_statusline_state(&root), GrokStatusline::CmdPath);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn codex_user_hooks_trust_needs_user_store_hash() {
        // D28：codex 注册与信任都在用户层。OMA_USER_HOME 注入临时家：ours 在
        // 用户 hooks.json 而 ~/.codex/config.toml 的 [hooks.state] 空 →
        // Block；预种 trusted_hash 后翻 Ok（共享 env 锁防并行互踩）。
        let _g = crate::pathutil::ENV_LOCK.lock().unwrap();
        let user = temp_root("codex-user-trust");
        fs::create_dir_all(user.join(".codex")).unwrap();
        fs::write(
            user.join(".codex").join("hooks.json"),
            r#"{"hooks":{"PreToolUse":[{"matcher":"*","hooks":[{"type":"command","command":"\"/home/x/.oma/hooks/oma-state.sh\" codex","timeout":10}]}]}}"#,
        )
        .unwrap();
        fs::write(
            user.join(".codex").join("config.toml"),
            "[features]\nhooks = true\n\n[hooks.state]\n",
        )
        .unwrap();
        let root = temp_root("codex-user-trust-proj");
        fs::create_dir_all(&root).unwrap();
        std::env::set_var("OMA_USER_HOME", &user);
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(
            d.status("codex", "trust.hooks"),
            Some(Status::Block),
            "empty user [hooks.state] must not pass"
        );
        // 预种 trusted_hash 后通过。
        fs::write(
            user.join(".codex").join("config.toml"),
            "[features]\nhooks = true\n\n[hooks.state]\n\"x:y:0:0\" = { trusted_hash = \"sha256:abc\" }\n",
        )
        .unwrap();
        let d = diagnose(&root).expect("diagnose");
        std::env::remove_var("OMA_USER_HOME");
        assert_eq!(d.status("codex", "trust.hooks"), Some(Status::Ok));
        let finding = d
            .findings
            .iter()
            .find(|f| f.agent == "codex" && f.check == "trust.hooks")
            .expect("trust.hooks row");
        assert!(
            finding.path.contains(".codex"),
            "path should be the user store, got {}",
            finding.path
        );
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn deploy_diagnosis_rows_never_block() {
        // 契约：登录态/状态栏/hook 形态行是部署诊断面，只 ok|warn，
        // 不得混入 block——blocked() 语义仍只对交互阻塞负责。
        let root = temp_root("warn");
        fs::create_dir_all(&root).unwrap();
        let d = diagnose(&root).expect("diagnose");
        for f in &d.findings {
            if matches!(f.check, "login" | "statusline" | "hooks.form") {
                assert_ne!(
                    f.status,
                    Status::Block,
                    "{} {} must not block",
                    f.agent,
                    f.check
                );
            }
        }
    }
}
