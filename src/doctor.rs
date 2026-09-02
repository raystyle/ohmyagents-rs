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
    /// missing, statusline off, stale session): surfaced for `oma doctor`,
    /// never counted by `blocked()`.
    Warn,
    Block,
}

impl Status {
    fn as_str(self) -> &'static str {
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

fn codex_statusline_on(home: &Path) -> bool {
    toml_file(&home.join(".codex").join("config.toml"))
        .as_ref()
        .and_then(|t| t.get("tui"))
        .and_then(|tui| tui.get("status_line"))
        .and_then(|sl| sl.as_array())
        .is_some_and(|parts| {
            parts
                .iter()
                .any(|p| p.as_str().is_some_and(|s| s.contains(STATUSLINE_MARKER)))
        })
}

fn kimi_statusline_on(home: &Path) -> bool {
    toml_file(&home.join(".kimi-code").join("tui.toml"))
        .as_ref()
        .and_then(|t| t.get("status_line"))
        .and_then(|sl| sl.get("command"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| c.contains(STATUSLINE_MARKER))
}

fn grok_statusline_on(home: &Path) -> bool {
    toml_file(&home.join(".grok").join("config.toml"))
        .as_ref()
        .and_then(|t| t.get("ui"))
        .and_then(|ui| ui.get("status_line"))
        .and_then(|sl| sl.get("command"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| c.contains(STATUSLINE_MARKER))
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

/// JSON 形 hook 注册（claude settings、grok ohmyagents-state.json）的 oma
/// 形态：bare（PATH 解析，跨环境共享）/ absolute（单环境）/ none。
fn json_hooks_form(v: Option<&Json>) -> &'static str {
    let Some(events) = v.and_then(|v| v.get("hooks")).and_then(|h| h.as_object()) else {
        return "none";
    };
    let mut ours = false;
    let mut bare = false;
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
                if !c.contains('/') && !c.contains('\\') {
                    bare = true;
                }
            }
        }
    }
    if bare {
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

fn push_hooks_form(out: &mut Vec<Finding>, agent: &str, form: &str, path: &Path) {
    match form {
        "bare" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Ok,
            path,
            "form=bare (PATH-resolved; one registration serves every environment)",
        ),
        "absolute" => push_status(
            out,
            agent,
            "hooks.form",
            Status::Ok,
            path,
            "form=absolute (single-environment; bare once oma is on PATH)",
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

// ===== 会话健康 =====

/// 会话清单态：无 manifest 是合法部署前态（不误报）；有则列路数，活性由
/// 调用方探测注入（None = 未探，测试注入口）。
fn session_finding(root: &Path, alive: Option<bool>) -> Finding {
    let path = root.join(".ohmyagents").join("session.json");
    match crate::orch::read_manifest_for(root) {
        None => Finding {
            agent: "oma".into(),
            check: "session",
            status: Status::Ok,
            path: path.display().to_string(),
            detail: "no session manifest; oma spawn creates one".into(),
        },
        Some(m) => {
            let routes: Vec<&str> = m.agents.iter().map(|a| a.name.as_str()).collect();
            let (status, detail) = match alive {
                Some(true) => (
                    Status::Ok,
                    format!("daemon answering; routes={}", routes.join(",")),
                ),
                Some(false) => (
                    Status::Warn,
                    "daemon not answering (stale manifest); oma spawn reconciles".into(),
                ),
                None => (
                    Status::Ok,
                    format!("manifest present; routes={}", routes.join(",")),
                ),
            };
            Finding {
                agent: "oma".into(),
                check: "session",
                status,
                path: path.display().to_string(),
                detail,
            }
        }
    }
}

/// rmux 只读探活：manifest 在时才调（`list-sessions` 不 attach）；rmux 未
/// 检出返回 None（部署诊断不替 `oma check` 装运行时）。
fn daemon_alive(root: &Path) -> Option<bool> {
    let pin = crate::catalog::RmuxPin::load().ok()?;
    let report = crate::rmux::detect(&pin).ok()?;
    Some(crate::rmuxpoc::label_alive(
        &report.layout.dispatcher,
        &crate::orch::label(root),
    ))
}

/// Read-only. Does not attach, send-keys, or wait on TUI.
pub fn diagnose(root: &Path) -> Result<Diagnosis, String> {
    let root = abs_display(root);
    let home = dirs::home_dir().ok_or_else(|| "cannot resolve home dir".to_string())?;
    let mut findings = Vec::new();

    // 部署诊断共享事实：状态栏脚本与 pwsh 探测一次（S025），登录态用统一
    // 时间基准（S026）。
    let oma_root = crate::install::oma_home().ok();
    let sl_script_ok = oma_root
        .as_deref()
        .map(crate::statusline::script_path)
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
    let claude_hooks_form = {
        let form = json_hooks_form(json_file(&claude_shared).as_ref());
        if form != "none" {
            form
        } else {
            json_hooks_form(json_file(&claude_local).as_ref())
        }
    };
    push_hooks_form(&mut findings, "claude", claude_hooks_form, &claude_shared);
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
    let codex_hooks_files = path_is_file(&root.join(".codex").join("hooks.json"))
        || proj_toml
            .as_ref()
            .and_then(|t| t.get("hooks"))
            .and_then(|h| h.as_table())
            .is_some_and(|h| h.keys().any(|k| k != "state"));
    let hook_hash = user_toml
        .as_ref()
        .map(|t| hook_state_trusted(t))
        .unwrap_or(false)
        || proj_toml
            .as_ref()
            .map(|t| hook_state_trusted(t))
            .unwrap_or(false);
    push(
        &mut findings,
        "codex",
        "trust.hooks",
        if codex_hooks_files { hook_hash } else { true },
        &codex_user,
        if !codex_hooks_files {
            "n/a no hooks.json / [hooks] (yolo does not bypass hook trust)"
        } else if hook_hash {
            "hooks.state trusted_hash present"
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
    let (codex_unix, codex_win) =
        codex_hooks_sides(json_file(&root.join(".codex").join("hooks.json")).as_ref());
    if codex_unix || codex_win {
        let mut sides = Vec::new();
        if codex_unix {
            sides.push("command(unix)");
        }
        if codex_win {
            sides.push("commandWindows(windows)");
        }
        push_status(
            &mut findings,
            "codex",
            "hooks.form",
            Status::Ok,
            &root.join(".codex").join("hooks.json"),
            format!(
                "per-OS fields ours: {} (absolute by design)",
                sides.join(", ")
            ),
        );
    } else {
        push_hooks_form(
            &mut findings,
            "codex",
            "none",
            &root.join(".codex").join("hooks.json"),
        );
    }
    push_statusline(
        &mut findings,
        "codex",
        codex_statusline_on(&home),
        &home.join(".codex").join("config.toml"),
        sl_script_ok,
        sl_pwsh_missing,
    );

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
    push_status(
        &mut findings,
        "kimi",
        "hooks.form",
        Status::Ok,
        &kimi_proj,
        "n/a no project-level hook registration (S015)",
    );
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
    let grok_state_json = root
        .join(".grok")
        .join("hooks")
        .join("ohmyagents-state.json");
    push_hooks_form(
        &mut findings,
        "grok",
        json_hooks_form(json_file(&grok_state_json).as_ref()),
        &grok_state_json,
    );
    push_statusline(
        &mut findings,
        "grok",
        grok_statusline_on(&home),
        &grok_cfg,
        sl_script_ok,
        sl_pwsh_missing,
    );

    let state_dir = root.join(".ohmyagents").join("state");
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

    let manifest_present = crate::orch::read_manifest_for(&root).is_some();
    let alive = if manifest_present {
        daemon_alive(&root)
    } else {
        None
    };
    findings.push(session_finding(&root, alive));

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
        let state = root.join(".ohmyagents").join("state");
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join("codex.json"), r#"{"state":"blocked"}"#).unwrap();
        let d = diagnose(&root).expect("diagnose");
        assert_eq!(d.status("codex", "state"), Some(Status::Block));
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
    fn hooks_form_classifies_bare_absolute_none() {
        let bare = json!({"hooks": {"SessionStart": [{"hooks": [{"command": "oma hook --agent claude"}]}]}});
        assert_eq!(json_hooks_form(Some(&bare)), "bare");
        let absolute =
            json!({"hooks": {"SessionStart": [{"hooks": [{"command": "D:\\x\\oma.exe hook"}]}]}});
        assert_eq!(json_hooks_form(Some(&absolute)), "absolute");
        let foreign = json!({"hooks": {"SessionStart": [{"hooks": [{"command": "echo hi"}]}]}});
        assert_eq!(json_hooks_form(Some(&foreign)), "none");
        assert_eq!(json_hooks_form(None), "none");
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
        let foreign = json!({"hooks": {"PreToolUse": [{"hooks": [
            {"type": "command", "command": "echo hi"}
        ]}]}});
        assert_eq!(codex_hooks_sides(Some(&foreign)), (false, false));
        assert_eq!(codex_hooks_sides(None), (false, false));
    }

    #[test]
    fn session_finding_without_manifest_is_info_not_block() {
        let root = temp_root("session");
        fs::create_dir_all(&root).unwrap();
        let f = session_finding(&root, None);
        assert_eq!(
            (f.agent.as_str(), f.check, f.status),
            ("oma", "session", Status::Ok)
        );
        fs::create_dir_all(root.join(".ohmyagents")).unwrap();
        fs::write(
            root.join(".ohmyagents").join("session.json"),
            r#"{"stub":true,"agents":[{"name":"claude","pane_id":3}]}"#,
        )
        .unwrap();
        let dead = session_finding(&root, Some(false));
        assert_eq!(dead.status, Status::Warn);
        assert!(dead.detail.contains("stale"));
        let live = session_finding(&root, Some(true));
        assert_eq!(live.status, Status::Ok);
        assert!(live.detail.contains("claude"));
        let unprobed = session_finding(&root, None);
        assert_eq!(unprobed.status, Status::Ok);
        assert!(unprobed.detail.contains("claude"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn deploy_diagnosis_rows_never_block() {
        // 契约：登录态/状态栏/hook 形态/会话行是部署诊断面，只 ok|warn，
        // 不得混入 block——blocked() 语义仍只对交互阻塞负责。
        let root = temp_root("warn");
        fs::create_dir_all(&root).unwrap();
        let d = diagnose(&root).expect("diagnose");
        for f in &d.findings {
            if matches!(f.check, "login" | "statusline" | "hooks.form" | "session") {
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
