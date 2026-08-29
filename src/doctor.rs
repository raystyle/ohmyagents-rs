use std::fs;
use std::path::Path;

use serde_json::Value as Json;
use toml::Value as Toml;

use crate::agents;
use crate::pathutil::{abs_display, forward_slash, keys_match, native_slash};
use crate::yolo::kimi_workspace_key;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Block,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
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
    out.push(Finding {
        agent: agent.to_string(),
        check,
        status: if ok { Status::Ok } else { Status::Block },
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

/// Read-only. Does not attach, send-keys, or wait on TUI.
pub fn diagnose(root: &Path) -> Result<Diagnosis, String> {
    let root = abs_display(root);
    let home = dirs::home_dir().ok_or_else(|| "cannot resolve home dir".to_string())?;
    let mut findings = Vec::new();

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
}
