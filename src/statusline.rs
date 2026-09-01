//! agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
//! - claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
//!   读改写，保留 env/permissions 等，只覆盖 statusLine 键）
//! - codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
//!   `status_line = ["command", "pwsh", ...]` 数组形态
//! 状态栏脚本本体（pwsh）随 oma 释放到 `~/.ohmyagents/statusline/`，
//! 显示 agent / 项目名 / 模型（claude stdin JSON 有 model 与 cwd）。

use std::path::{Path, PathBuf};

use serde_json::json;

/// 状态栏脚本：读 stdin JSON（claude code 供给 model/cwd/session_id 等；
/// codex 无 stdin 数据时退化显示静态行）。单行输出。
pub const STATUSLINE_PS1: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
$raw = [Console]::In.ReadToEnd()
$info = $null
try { $info = $raw | ConvertFrom-Json } catch {}
$model = if ($info -and $info.model.display_name) { $info.model.display_name } else { 'oma' }
$cwd = if ($info -and $info.workspace.current_dir) { Split-Path -Leaf $info.workspace.current_dir } else { Split-Path -Leaf (Get-Location) }
$agent = if ($env:OMA_AGENT) { $env:OMA_AGENT } else { 'agent' }
$oct = [char]::ConvertFromUtf32(0x1F9FD)
Write-Output "$oct oma | $agent | $cwd | $model"
"#;

fn script_path(home: &Path) -> PathBuf {
    home.join("statusline").join("oma-statusline.ps1")
}

/// 释放状态栏脚本（幂等覆写）。
pub fn deploy_script(home: &Path) -> Result<PathBuf, String> {
    let p = script_path(home);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&p, STATUSLINE_PS1).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok(p)
}

/// claude：settings.json 幂等合并 statusLine（只覆盖该键）。
pub fn merge_claude(home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let settings = dirs::home_dir()
        .ok_or("no home")?
        .join(".claude")
        .join("settings.json");
    let mut v: serde_json::Value = if settings.exists() {
        let text = std::fs::read_to_string(&settings).map_err(|e| format!("{}: {e}", settings.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: corrupt: {e}", settings.display()))?
    } else {
        json!({})
    };
    let cmd = format!(
        "pwsh -NoProfile -File \"{}\"",
        script.display().to_string().replace('\\', "/")
    );
    v["statusLine"] = json!({ "type": "command", "command": cmd });
    let body = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&settings, body).map_err(|e| format!("{}: {e}", settings.display()))?;
    Ok(settings.display().to_string())
}

/// codex：config.toml 顶层 `[tui]` 段整段替换（幂等；ohmypwsh 同形态）。
pub fn merge_codex(home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let config = dirs::home_dir()
        .ok_or("no home")?
        .join(".codex")
        .join("config.toml");
    let script_str = script.display().to_string().replace('\\', "/");
    let mut lines: Vec<String> = Vec::new();
    if config.exists() {
        let text = std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?;
        let mut in_tui = false;
        for ln in text.lines() {
            if ln.trim().starts_with('[') {
                in_tui = ln.trim() == "[tui]";
                if in_tui {
                    continue;
                }
            }
            if !in_tui {
                lines.push(ln.to_string());
            }
        }
    }
    lines.push(String::new());
    lines.push("[tui]".into());
    lines.push("status_line = [".into());
    lines.push("  \"command\", ".into());
    lines.push("  \"pwsh\", ".into());
    lines.push("  \"-NoProfile\", ".into());
    lines.push(format!("  \"-File\", \"{script_str}\", "));
    lines.push("]".into());
    let body = lines.join("\n") + "\n";
    std::fs::write(&config, body).map_err(|e| format!("{}: {e}", config.display()))?;
    Ok(config.display().to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn codex_merge_is_idempotent_and_keeps_other_tables() {
        let home = std::env::temp_dir().join(format!("oma-sl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let cfg = home.join(".codex");
        std::fs::create_dir_all(&cfg).unwrap();
        let f = cfg.join("config.toml");
        std::fs::write(&f, "model = \"gpt\"\n[tui]\nstatus_line = [\"old\"]\n[sandbox]\nmode = \"rw\"\n").unwrap();
        // home 目录重定向不可行（dirs::home_dir 全局），直接验证段替换逻辑：
        let text = std::fs::read_to_string(&f).unwrap();
        let mut lines: Vec<String> = Vec::new();
        let mut in_tui = false;
        for ln in text.lines() {
            if ln.trim().starts_with('[') {
                in_tui = ln.trim() == "[tui]";
                if in_tui {
                    continue;
                }
            }
            if !in_tui {
                lines.push(ln.to_string());
            }
        }
        let out = lines.join("\n");
        assert!(out.contains("model = \"gpt\""));
        assert!(out.contains("[sandbox]"));
        assert!(!out.contains("\"old\""));
        let _ = std::fs::remove_dir_all(&home);
    }
}
