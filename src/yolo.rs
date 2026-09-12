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

/// 用户级 yolo 与非阻塞键（D28 第 2 轮裁定 2026-09-11：yolo 模式与非阻塞
/// 模式也用户级，覆盖 D25「oma --yolo 面向项目级」口径；历史脉络 = 用户级
/// 需求自 POC 期「不改家目录」约束起被逐面翻正：状态栏 P0027、kimi M058、
/// hook D28、本面）。四家写入面：claude `~/.claude/settings.json`
/// （permissions.defaultMode 加 skipDangerousModePermissionPrompt 加
/// enableAllProjectMcpServers）、codex `~/.codex/config.toml`
/// （sandbox_mode 加 approval_policy）、kimi `~/.kimi-code/config.toml`
/// （default_permission_mode）、grok `~/.grok/config.toml`
/// （[ui] permission_mode）。项目级旧键由 init 退役（retire_project_yolo）。
pub fn apply_user_yolo_with(user_home: &Path) -> Result<ApplyReport, String> {
    let mut wrote = Vec::new();

    let claude_user = user_home.join(".claude").join("settings.json");
    let mut shared = read_json(&claude_user)?;
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
        obj.insert("skipDangerousModePermissionPrompt".into(), Json::Bool(true));
        // 用户级 enableAll 覆盖所有项目（per-project enabledMcpjsonServers
        // 名单是项目语义，不上用户级）。
        obj.insert("enableAllProjectMcpServers".into(), Json::Bool(true));
    }
    write_json(&claude_user, &shared)?;
    wrote.push(claude_user.display().to_string());

    let codex_user = user_home.join(".codex").join("config.toml");
    let mut ctoml = read_toml(&codex_user)?;
    {
        let t = table_mut(&mut ctoml)?;
        t.insert(
            "sandbox_mode".into(),
            Toml::String("danger-full-access".into()),
        );
        t.insert("approval_policy".into(), Toml::String("never".into()));
    }
    write_toml(&codex_user, &ctoml)?;
    wrote.push(codex_user.display().to_string());

    let kimi_user = user_home.join(".kimi-code").join("config.toml");
    let mut ktoml = read_toml(&kimi_user)?;
    table_mut(&mut ktoml)?.insert(
        "default_permission_mode".into(),
        Toml::String("yolo".into()),
    );
    write_toml(&kimi_user, &ktoml)?;
    wrote.push(kimi_user.display().to_string());

    let grok_cfg = user_home.join(".grok").join("config.toml");
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

/// 项目级 yolo 与非阻塞键（D28 第 3 轮裁定 2026-09-11：yolo 命令分两级，
/// `hst init --project-yolo` 显式选项目级；v0.5.3 前的默认行为收编为显式
/// 旗标）。写入面：claude 项目 `.claude/settings.json` 加
/// `settings.local.json`、codex 项目 `.codex/config.toml`（yolo 键加项目
/// 信任预种）、kimi 项目 `.kimi-code/config.toml`；grok 无项目级面
/// （permission_mode 只能写用户级，S025）。
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
        let key = if cfg!(windows) {
            native.to_ascii_lowercase()
        } else {
            native.clone()
        };
        let proj = projects
            .entry(key)
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

/// 生产入口：真实家目录。
pub fn apply_user_yolo() -> Result<ApplyReport, String> {
    let home = crate::pathutil::user_home()?;
    apply_user_yolo_with(&home)
}

/// 项目级旧 yolo 键退役（D28 第 2 轮）：oma 写过的项目面键摘除（值等于
/// ours 落值才动，用户自设其它值保留），整文件只剩空对象/空表时删文件。
/// 返回变更描述（deploy report 收录）。
pub fn retire_project_yolo(root: &Path) -> Result<Vec<String>, String> {
    let root = abs_display(root);
    let mut changed = Vec::new();

    // claude 项目 settings.json：permissions.defaultMode == bypass 摘。
    let claude_shared = root.join(".claude").join("settings.json");
    if claude_shared.exists() {
        let mut v = read_json(&claude_shared)?;
        if v.is_object() {
            let mut dirty = false;
            if v.get("permissions")
                .and_then(|p| p.get("defaultMode"))
                .and_then(|m| m.as_str())
                == Some("bypassPermissions")
            {
                if let Some(p) = v.get_mut("permissions").and_then(|p| p.as_object_mut()) {
                    p.remove("defaultMode");
                    dirty = true;
                    if p.is_empty() {
                        v.as_object_mut().unwrap().remove("permissions");
                    }
                }
            }
            if dirty {
                if v.as_object().is_some_and(|o| o.is_empty()) {
                    fs::remove_file(&claude_shared)
                        .map_err(|e| format!("{}: {e}", claude_shared.display()))?;
                } else {
                    write_json(&claude_shared, &v)?;
                }
                changed.push(format!("{} (retired-yolo)", claude_shared.display()));
            }
        }
    }

    // claude 项目 settings.local.json：skip / enableAll 摘（ours 值才动）。
    let claude_local = root.join(".claude").join("settings.local.json");
    if claude_local.exists() {
        let mut v = read_json(&claude_local)?;
        if v.is_object() {
            let obj = v.as_object_mut().unwrap();
            let mut dirty = false;
            for key in [
                "skipDangerousModePermissionPrompt",
                "enableAllProjectMcpServers",
            ] {
                if obj.get(key).and_then(|x| x.as_bool()) == Some(true) {
                    obj.remove(key);
                    dirty = true;
                }
            }
            if dirty {
                if obj.is_empty() {
                    fs::remove_file(&claude_local)
                        .map_err(|e| format!("{}: {e}", claude_local.display()))?;
                } else {
                    write_json(&claude_local, &v)?;
                }
                changed.push(format!("{} (retired-yolo)", claude_local.display()));
            }
        }
    }

    // codex 项目 config.toml：sandbox/approval 等值摘（项目信任键留给
    // pretrust 面的用户 store，不动）。
    let codex = root.join(".codex").join("config.toml");
    if codex.exists() {
        let mut t = read_toml(&codex)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            for (k, want) in [
                ("sandbox_mode", "danger-full-access"),
                ("approval_policy", "never"),
            ] {
                if map.get(k).and_then(|v| v.as_str()) == Some(want) {
                    map.remove(k);
                    dirty = true;
                }
            }
            if dirty {
                if map.is_empty() {
                    fs::remove_file(&codex).map_err(|e| format!("{}: {e}", codex.display()))?;
                } else {
                    write_toml(&codex, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", codex.display()));
            }
        }
    }

    // kimi 项目 config.toml：default_permission_mode == yolo 摘。
    let kimi = root.join(".kimi-code").join("config.toml");
    if kimi.exists() {
        let mut t = read_toml(&kimi)?;
        if let Toml::Table(map) = &mut t {
            let mut dirty = false;
            if map.get("default_permission_mode").and_then(|v| v.as_str()) == Some("yolo") {
                map.remove("default_permission_mode");
                dirty = true;
            }
            if dirty {
                if map.is_empty() {
                    fs::remove_file(&kimi).map_err(|e| format!("{}: {e}", kimi.display()))?;
                } else {
                    write_toml(&kimi, &t)?;
                }
                changed.push(format!("{} (retired-yolo)", kimi.display()));
            }
        }
    }

    Ok(changed)
}

/// Trust stores in the user home. Not hook registration.
pub fn apply_pretrust(root: &Path) -> Result<ApplyReport, String> {
    let root = abs_display(root);
    let home = crate::pathutil::user_home()?;
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
    fn user_yolo_clears_file_prompt_blocks_and_retires_project_keys() {
        let _g = crate::pathutil::ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let user = fresh_dir();
        let root = fresh_dir();
        std::env::set_var("HST_USER_HOME", &user);
        let before = diagnose(&root).expect("diagnose");
        assert_eq!(before.status("claude", "yolo"), Some(Status::Block));
        assert_eq!(before.status("codex", "yolo"), Some(Status::Block));

        let report = apply_user_yolo_with(&user).expect("apply");
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("settings.json") && p.contains(".claude")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("config.toml") && p.contains(".codex")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("config.toml") && p.contains(".kimi-code")));
        assert!(report
            .wrote
            .iter()
            .any(|p| p.ends_with("config.toml") && p.contains(".grok")));

        let shared = read_json(&user.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            shared["permissions"]["defaultMode"].as_str(),
            Some("bypassPermissions")
        );
        assert_eq!(
            shared["skipDangerousModePermissionPrompt"].as_bool(),
            Some(true)
        );
        assert_eq!(shared["enableAllProjectMcpServers"].as_bool(), Some(true));

        let after = diagnose(&root).expect("diagnose after");
        assert_eq!(after.status("claude", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("claude", "skip_prompt"), Some(Status::Ok));
        assert_eq!(after.status("codex", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("kimi", "yolo"), Some(Status::Ok));
        assert_eq!(after.status("grok", "yolo"), Some(Status::Ok));
        // 信任面不被 yolo 清（pretrust 是另一个面）。
        assert_eq!(after.status("claude", "trust.project"), Some(Status::Block));
        assert_eq!(after.status("codex", "trust.project"), Some(Status::Block));

        // 项目级旧键退役：v0.5.3 形项目摘 ours 值、留用户自设值、空文件删。
        fs::create_dir_all(root.join(".claude")).unwrap();
        write_json(
            &root.join(".claude").join("settings.json"),
            &json!({"permissions": {"defaultMode": "bypassPermissions", "allow": ["Bash"]}}),
        )
        .unwrap();
        write_json(
            &root.join(".claude").join("settings.local.json"),
            &json!({"skipDangerousModePermissionPrompt": true}),
        )
        .unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        fs::write(
            root.join(".codex").join("config.toml"),
            "sandbox_mode = \"danger-full-access\"
approval_policy = \"never\"
model = \"gpt\"
",
        )
        .unwrap();
        fs::create_dir_all(root.join(".kimi-code")).unwrap();
        fs::write(
            root.join(".kimi-code").join("config.toml"),
            "default_permission_mode = \"yolo\"
",
        )
        .unwrap();
        let changed = retire_project_yolo(&root).unwrap();
        assert_eq!(changed.len(), 4, "four files retired: {changed:?}");
        let v = read_json(&root.join(".claude").join("settings.json")).unwrap();
        assert_eq!(
            v["permissions"]["allow"][0].as_str(),
            Some("Bash"),
            "user-curated permission entries survive"
        );
        assert!(
            v["permissions"].get("defaultMode").is_none(),
            "ours key gone"
        );
        assert!(
            !root.join(".claude").join("settings.local.json").exists(),
            "ours-only local file deleted"
        );
        let codex = fs::read_to_string(root.join(".codex").join("config.toml")).unwrap();
        assert!(codex.contains("model"), "foreign key survives");
        assert!(!codex.contains("sandbox_mode"), "ours key gone");
        assert!(
            !root.join(".kimi-code").join("config.toml").exists(),
            "ours-only kimi project config deleted"
        );
        // 再跑幂等（无变更）。
        assert!(retire_project_yolo(&root).unwrap().is_empty());

        std::env::remove_var("HST_USER_HOME");
        let _ = fs::remove_dir_all(&user);
        let _ = fs::remove_dir_all(&root);
    }
}
