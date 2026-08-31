use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value as Json};
use sha2::{Digest, Sha256};
use toml::Value as Toml;

use crate::pathutil::{abs_display, forward_slash, native_slash};

pub struct ApplyReport {
    pub wrote: Vec<String>,
}

pub(crate) fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(())
}

pub(crate) fn write_text(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path)?;
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn read_json(path: &Path) -> Result<Json, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn write_json(path: &Path, value: &Json) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())? + "\n";
    write_text(path, &text)
}

pub(crate) fn read_toml(path: &Path) -> Result<Toml, String> {
    if !path.exists() {
        return Ok(Toml::Table(toml::map::Map::new()));
    }
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(Toml::Table(toml::map::Map::new()));
    }
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_toml(path: &Path, value: &Toml) -> Result<(), String> {
    let text = toml::to_string_pretty(value).map_err(|e| e.to_string())?;
    write_text(path, &text)
}

pub(crate) fn toml_write(path: &Path, value: &Toml) -> Result<(), String> {
    write_toml(path, value)
}

fn table_mut(v: &mut Toml) -> Result<&mut toml::map::Map<String, Toml>, String> {
    match v {
        Toml::Table(t) => Ok(t),
        _ => Err("expected TOML table".into()),
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Project-level yolo files only. Does not write user-home hook registration.
pub fn apply_project_yolo(root: &Path) -> Result<ApplyReport, String> {
    let root = abs_display(root);
    let mut wrote = Vec::new();

    let claude_shared = root.join(".claude").join("settings.json");
    let mut shared = read_json(&claude_shared)?;
    if !shared.is_object() {
        shared = json!({});
    }
    {
        let obj = shared.as_object_mut().unwrap();
        let permissions = obj
            .entry("permissions".to_string())
            .or_insert_with(|| json!({}));
        if !permissions.is_object() {
            *permissions = json!({});
        }
        permissions
            .as_object_mut()
            .unwrap()
            .insert("defaultMode".into(), json!("bypassPermissions"));
    }
    write_json(&claude_shared, &shared)?;
    wrote.push(claude_shared.display().to_string());

    let claude_local = root.join(".claude").join("settings.local.json");
    let mut local = read_json(&claude_local)?;
    if !local.is_object() {
        local = json!({});
    }
    {
        let obj = local.as_object_mut().unwrap();
        obj.insert("skipDangerousModePermissionPrompt".into(), Json::Bool(true));
        apply_mcp_approvals(obj, &root);
    }
    write_json(&claude_local, &local)?;
    wrote.push(claude_local.display().to_string());

    let native = native_slash(&root);
    let native_lc = if cfg!(windows) {
        native.to_ascii_lowercase()
    } else {
        native.clone()
    };
    let codex = root.join(".codex").join("config.toml");
    let mut ctoml = read_toml(&codex)?;
    {
        let t = table_mut(&mut ctoml)?;
        t.insert(
            "sandbox_mode".into(),
            Toml::String("danger-full-access".into()),
        );
        t.insert("approval_policy".into(), Toml::String("never".into()));
        let projects = t
            .entry("projects".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let projects = table_mut(projects)?;
        let proj = projects
            .entry(native_lc.clone())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(proj)?.insert("trust_level".into(), Toml::String("trusted".into()));
    }
    write_toml(&codex, &ctoml)?;
    wrote.push(codex.display().to_string());

    let kimi = root.join(".kimi-code").join("config.toml");
    let mut ktoml = read_toml(&kimi)?;
    table_mut(&mut ktoml)?.insert(
        "default_permission_mode".into(),
        Toml::String("yolo".into()),
    );
    write_toml(&kimi, &ktoml)?;
    wrote.push(kimi.display().to_string());

    Ok(ApplyReport { wrote })
}

/// Trust stores in the user home. Not hook registration.
pub fn apply_pretrust(root: &Path) -> Result<ApplyReport, String> {
    let root = abs_display(root);
    let home = dirs::home_dir().ok_or_else(|| "cannot resolve home dir".to_string())?;
    let mut wrote = Vec::new();
    let native = native_slash(&root);
    let fwd = forward_slash(&root);

    let claude_json = home.join(".claude.json");
    let mut cj = read_json(&claude_json)?;
    if !cj.is_object() {
        cj = json!({});
    }
    {
        let obj = cj.as_object_mut().unwrap();
        obj.insert("hasCompletedOnboarding".into(), Json::Bool(true));
        let projects = obj
            .entry("projects".to_string())
            .or_insert_with(|| json!({}));
        if !projects.is_object() {
            *projects = json!({});
        }
        let entry = projects
            .as_object_mut()
            .unwrap()
            .entry(fwd.clone())
            .or_insert_with(|| json!({}));
        if let Some(m) = entry.as_object_mut() {
            m.insert("hasTrustDialogAccepted".into(), Json::Bool(true));
            m.insert("hasTrustDialogHooksAccepted".into(), Json::Bool(true));
        }
    }
    write_json(&claude_json, &cj)?;
    wrote.push(claude_json.display().to_string());

    let user_claude = home.join(".claude").join("settings.json");
    let mut us = read_json(&user_claude)?;
    if !us.is_object() {
        us = json!({});
    }
    {
        let obj = us.as_object_mut().unwrap();
        obj.insert("skipDangerousModePermissionPrompt".into(), Json::Bool(true));
        apply_mcp_approvals(obj, &root);
    }
    write_json(&user_claude, &us)?;
    wrote.push(user_claude.display().to_string());

    let codex_user = home.join(".codex").join("config.toml");
    let mut cu = read_toml(&codex_user)?;
    {
        let t = table_mut(&mut cu)?;
        let projects = t
            .entry("projects".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let key = if cfg!(windows) {
            native.to_ascii_lowercase()
        } else {
            native.clone()
        };
        let proj = table_mut(projects)?
            .entry(key)
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(proj)?.insert("trust_level".into(), Toml::String("trusted".into()));
    }
    write_toml(&codex_user, &cu)?;
    wrote.push(codex_user.display().to_string());

    let kimi_home = home.join(".kimi-code");
    let key = kimi_workspace_key(&root);
    let ws_path = kimi_home.join("workspaces.json");
    let mut ws = read_json(&ws_path)?;
    if !ws.is_object() {
        ws = json!({ "version": 1, "workspaces": {} });
    }
    {
        let obj = ws.as_object_mut().unwrap();
        obj.entry("version".to_string())
            .or_insert(Json::Number(1.into()));
        let workspaces = obj
            .entry("workspaces".to_string())
            .or_insert_with(|| json!({}));
        if !workspaces.is_object() {
            *workspaces = json!({});
        }
        workspaces.as_object_mut().unwrap().insert(
            key.clone(),
            json!({
                "root": native,
                "name": root.file_name().and_then(|s| s.to_str()).unwrap_or("project"),
            }),
        );
    }
    write_json(&ws_path, &ws)?;
    wrote.push(ws_path.display().to_string());

    let trust_file = kimi_home.join("workspace-trust").join(&key);
    write_json(
        &trust_file,
        &json!({ "root": native, "trustedAt": unix_millis() as u64 }),
    )?;
    wrote.push(trust_file.display().to_string());

    let grok_tf = home.join(".grok").join("trusted_folders.toml");
    let mut gt = read_toml(&grok_tf)?;
    {
        let t = table_mut(&mut gt)?;
        let folders = t
            .entry("folders".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let folder = table_mut(folders)?
            .entry(native.clone())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        let ft = table_mut(folder)?;
        ft.insert("trusted".into(), Toml::Boolean(true));
        ft.insert("decided_at".into(), Toml::Integer(unix_secs() as i64));
    }
    write_toml(&grok_tf, &gt)?;
    wrote.push(grok_tf.display().to_string());

    let grok_cfg = home.join(".grok").join("config.toml");
    let mut gc = read_toml(&grok_cfg)?;
    {
        let t = table_mut(&mut gc)?;
        let ui = t
            .entry("ui".to_string())
            .or_insert_with(|| Toml::Table(toml::map::Map::new()));
        table_mut(ui)?.insert(
            "permission_mode".into(),
            Toml::String("always-approve".into()),
        );
    }
    write_toml(&grok_cfg, &gc)?;
    wrote.push(grok_cfg.display().to_string());

    Ok(ApplyReport { wrote })
}

fn mcp_json_names(root: &Path) -> Vec<String> {
    let v = read_json(&root.join(".mcp.json")).unwrap_or_else(|_| json!({}));
    v.get("mcpServers")
        .and_then(|m| m.as_object())
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default()
}

fn apply_mcp_approvals(obj: &mut serde_json::Map<String, Json>, root: &Path) {
    obj.insert("enableAllProjectMcpServers".into(), Json::Bool(true));
    let mut names = mcp_json_names(root);
    if let Some(existing) = obj.get("enabledMcpjsonServers").and_then(|x| x.as_array()) {
        for s in existing {
            if let Some(n) = s.as_str() {
                if !n.is_empty() && !names.iter().any(|x| x == n) {
                    names.push(n.to_string());
                }
            }
        }
    }
    if !names.is_empty() {
        obj.insert(
            "enabledMcpjsonServers".into(),
            Json::Array(names.into_iter().map(Json::String).collect()),
        );
    }
}

pub fn kimi_workspace_key(root: &Path) -> String {
    let root = abs_display(root);
    let name = root
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");
    let digest = Sha256::digest(native_slash(&root).as_bytes());
    let hex = format!("{digest:x}");
    format!("wd_{}_{}", name, &hex[..12])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doctor::{diagnose, Status};

    /// Unique per-call suffix: same-millisecond parallel tests must not
    /// share (and mutually delete) a temp dir.
    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn fresh_dir() -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-yolo-test-{}-{}-{}",
            std::process::id(),
            unix_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn kimi_workspace_key_shape() {
        let root = fresh_dir();
        let key = kimi_workspace_key(&root);
        let name = root.file_name().unwrap().to_str().unwrap();
        assert!(key.starts_with(&format!("wd_{name}_")), "{key}");
        assert_eq!(key.len(), 3 + name.len() + 1 + 12, "{key}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn project_yolo_clears_file_prompt_blocks() {
        let root = fresh_dir();
        let before = diagnose(&root).expect("diagnose");
        assert_eq!(before.status("claude", "yolo"), Some(Status::Block));
        assert_eq!(before.status("codex", "yolo"), Some(Status::Block));

        let report = apply_project_yolo(&root).expect("apply");
        assert!(report.wrote.iter().any(|p| p.ends_with("settings.json")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("settings.local.json")));

        let shared = read_json(&root.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            shared["permissions"]["defaultMode"].as_str(),
            Some("bypassPermissions")
        );
        let local = read_json(&root.join(".claude").join("settings.local.json")).unwrap();
        assert_eq!(
            local["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true)
        );
        assert_eq!(local["enableAllProjectMcpServers"].as_bool(), Some(true));

        let after = diagnose(&root).expect("diagnose after");
        assert_eq!(after.status("claude", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("claude", "skip_prompt"), Some(Status::Ok));
        assert_eq!(after.status("codex", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("kimi", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(after.status("codex", "trust.project"), Some(Status::Block));
        assert_eq!(after.status("kimi", "trust.project"), Some(Status::Block));
        assert_eq!(after.status("claude", "trust.hooks"), Some(Status::Ok));
        assert_eq!(after.status("claude", "trust.skill"), Some(Status::Ok));
        assert_eq!(after.status("claude", "trust.mcp"), Some(Status::Ok));
        let _ = fs::remove_dir_all(&root);
    }
}
