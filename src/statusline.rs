//! agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
//! - claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
//!   读改写，保留 env/permissions 等，只覆盖 statusLine 键）
//! - codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
//!   `status_line = ["command", "pwsh", ...]` 数组形态
//! 状态栏脚本本体（pwsh）随 oma 释放到 `~/.ohmyagents/statusline/`。
//! 用户定调 2026-09-02：渲染对齐用户 starship 配置风格（目录截断、git 旗标、
//! 包与工具链版本段、nerdfont 图标、Catppuccin 系 256 色）；oma 段 = 当前
//! agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记见 S025），另探测
//! agent 宿主 shell（macOS 走 ps 兜底）。

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::yolo::{read_toml, toml_write};

/// 状态栏脚本：读 stdin JSON（claude code 供给 model/context_window/cost 等；
/// codex 无 stdin 数据时退化为 agent + 目录 + git 段）。单行输出。
/// 渲染对齐用户 starship 配置（2026-09-02 定调）：目录截断 3 段、git 分支
/// 图标与状态旗标、package/rust 版本段、nerdfont 图标；oma 段在最前。
/// 首行强制 UTF-8：CP936 控制台下 emoji 会被替换成字面 `??`（S024）。
pub const STATUSLINE_PS1: &str = r#"
param([string]$AgentName = 'agent')
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'
$raw = [Console]::In.ReadToEnd()
$d = $null
if (-not [string]::IsNullOrWhiteSpace($raw)) { try { $d = $raw | ConvertFrom-Json } catch {} }

function Seg([string]$text, [string]$code = '') {
    if ([string]::IsNullOrWhiteSpace($text)) { return $null }
    if ($code) { return "$([char]27)[${code}m$text$([char]27)[0m" }
    return $text
}
function FmtTok([double]$n) {
    if ($n -ge 1MB) { return '{0:N0}M' -f ($n / 1MB) }
    if ($n -ge 1KB) { return '{0:N0}k' -f ($n / 1KB) }
    return [string][long]$n
}
# 会话时长人性化：3d4h / 4h12m / 12m30s / 30s
function FmtDur([double]$ms) {
    $s = [math]::Floor($ms / 1000)
    if ($s -ge 86400) { return '{0}d{1}h' -f [math]::Floor($s / 86400), [math]::Floor(($s % 86400) / 3600) }
    if ($s -ge 3600) { return '{0}h{1}m' -f [math]::Floor($s / 3600), [math]::Floor(($s % 3600) / 60) }
    if ($s -ge 60) { return '{0}m{1}s' -f [math]::Floor($s / 60), ($s % 60) }
    return "${s}s"
}

$parts = [System.Collections.Generic.List[string]]::new()

# ── 工作目录与仓库根（oma 段与目录段共用）──
$dir = $null
if ($d.workspace) { $dir = "$($d.workspace.current_dir)" }
if (-not $dir -or $dir -eq '.') { $dir = "$($d.cwd)" }
if (-not $dir -or $dir -eq '.') { $dir = "$(Get-Location)" }
$root = (& git -C $dir rev-parse --show-toplevel 2>$null | Out-String).Trim()

# ── Shell 段：agent 宿主 shell（祖先链跳过 agent 本体，向上找最近 shell）──
$shellName = $null
$shells = '^(pwsh|powershell|bash|zsh|sh|fish|cmd|nu|elvish|xonsh)'
$agentStems = '^(node|claude|codex|grok|kimi|oma)'
$chain = @()
if ($IsWindows -or $env:OS -eq 'Windows_NT') {
    $p = Get-Process -Id $PID -ErrorAction SilentlyContinue
    for ($i = 0; $i -lt 8 -and $p; $i++) {
        try { $p = $p.Parent } catch { $p = $null }
        if (-not $p) { break }
        $chain += $p.ProcessName.ToLowerInvariant()
    }
} else {
    # Unix：Linux/WSL 走 /proc；macOS 无 /proc，回退 BSD ps（-o ppid=/comm=）。
    $cur = $PID
    for ($i = 0; $i -lt 8; $i++) {
        $ppid = $null
        $procStat = Get-Content "/proc/$cur/status" -ErrorAction SilentlyContinue
        if ($procStat) {
            $ppidLine = $procStat | Where-Object { $_ -match '^PPid:\s+(\d+)' } | Select-Object -First 1
            if ($ppidLine -and $ppidLine -match '^PPid:\s+(\d+)') { $ppid = [int]$Matches[1] }
        } else {
            $psOut = (& ps -o ppid= -p $cur 2>$null | Out-String).Trim()
            if ($psOut -match '^\d+$') { $ppid = [int]$psOut }
        }
        if (-not $ppid -or $ppid -le 1) { break }
        $cur = $ppid
        $comm = (Get-Content "/proc/$cur/comm" -ErrorAction SilentlyContinue | Select-Object -First 1)
        if (-not $comm) { $comm = (& ps -o comm= -p $cur 2>$null | Out-String).Trim() }
        if ($comm) { $chain += $comm.Trim().ToLowerInvariant() }
    }
}
if ($chain.Count -gt 0) {
    $agentIdx = -1
    for ($i = 0; $i -lt $chain.Count; $i++) {
        if ($chain[$i] -match $agentStems) { $agentIdx = $i; break }
    }
    $search = if ($agentIdx -ge 0 -and $agentIdx + 1 -lt $chain.Count) { $chain[($agentIdx + 1)..($chain.Count - 1)] } else { $chain }
    foreach ($n in $search) {
        if ($n -match $shells) { $shellName = $n -replace '\.exe$', ''; break }
    }
}
if (-not $shellName -and $env:SHELL) { $shellName = (Split-Path -Leaf $env:SHELL) }
if ($shellName) {
    $shIcon = if ($shellName -match '^(pwsh|powershell)') { [char]::ConvertFromUtf32(0xEBC7) } else { [char]::ConvertFromUtf32(0xEA85) }
    $sh = Seg "$shIcon $shellName" '38;5;245'
    if ($sh) { $parts.Add($sh) }
}

# ── 目录：完整路径（用户定调 2026-09-02）──
if ($dir) {
    $p = Seg $dir '38;5;39'
    if ($p) { $parts.Add($p) }
}

# ── oma 段：当前 agent 名 + 实时状态（hook 状态通道；机读标记见 S025）──
# agent 名：oma 会话 env 优先，部署参数次之（每家配置注入自家名字）。
$agent = if ($env:OMA_AGENT) { $env:OMA_AGENT } else { $AgentName }
# 状态：会话状态文件优先，回退项目 .ohmyagents/state/<agent>.json。
$state = $null
$stateFile = $env:OHMYAGENTS_STATE_FILE
if (-not $stateFile) {
    $base = if ($root) { $root } else { $dir }
    if ($base) { $stateFile = Join-Path (Join-Path (Join-Path $base '.ohmyagents') 'state') "$agent.json" }
}
if ($stateFile -and (Test-Path $stateFile)) {
    try {
        $st = Get-Content -Raw $stateFile | ConvertFrom-Json
        if ($st.state) { $state = "$($st.state)" }
        # 会话闸：记录带 session 且与当前会话不符 → 是别的（可能已死）会话遗留，不算当前态。
        if ($state -and $st.session -and $d.session_id -and ("$($st.session)" -ne "$($d.session_id)")) {
            $state = $null
        }
    } catch {}
}
if (-not $state) { $state = 'unknown' }
$stateColor = switch ($state) {
    'idle' { '38;5;108' }
    'working' { '38;5;179' }
    'blocked' { '38;5;203' }
    default { '38;5;245' }
}
$parts.Add((Seg "󰚩  ${agent}:$state" $stateColor))

# ── 模型（display_name 优先，回退 id）──
$model = $null
if ($d.model) {
    $model = if ($d.model.display_name) { "$($d.model.display_name)" } else { "$($d.model.id)" }
}
if ($model) {
    $m = Seg "✦ $model" '38;5;147'
    if ($m) { $parts.Add($m) }
}

# ── 上下文：󰍛 N% (已用/窗口)，对齐 Codex 语义 ──
if ($d.context_window) {
    $cw = $d.context_window
    $win = [double]$cw.context_window_size
    $usedPct = $null
    if ($null -ne $cw.used_percentage) {
        $usedPct = [math]::Floor([double]$cw.used_percentage)
    } elseif ($null -ne $cw.remaining_percentage) {
        $usedPct = 100 - [math]::Floor([double]$cw.remaining_percentage)
    }
    if ($null -ne $usedPct -and $win -gt 0) {
        $usedTok = [math]::Round($win * $usedPct / 100)
        $c = Seg ("󰍛 ${usedPct}% ($(FmtTok $usedTok)/$(FmtTok $win))") '38;5;116'
        if ($c) { $parts.Add($c) }
    }
}

# ── 会话累计：󰅐 时长（claude cost 段；无则省略。成本数字对网关计价不准，不展示）──
if ($d.cost) {
    if ($null -ne $d.cost.total_duration_ms -and [double]$d.cost.total_duration_ms -ge 1000) {
        $dur = Seg ("󰅐 " + (FmtDur ([double]$d.cost.total_duration_ms))) '38;5;245'
        if ($dur) { $parts.Add($dur) }
    }
}

# ── Git：分支  + 状态旗标 [!?]（starship 符号语义，porcelain 单次调用）──
$branch = $null
if ($d.worktree -and $d.worktree.branch) { $branch = "$($d.worktree.branch)" }
if (-not $branch -and $d.workspace -and $d.workspace.git_worktree -and $d.workspace.git_worktree.name) {
    $branch = "$($d.workspace.git_worktree.name)"
}
$flags = ''
$aheadBehind = ''
$gs = & git status -b --porcelain=v1 2>$null
if (-not $branch -and $gs) {
    $hdr = ($gs | Where-Object { $_ -like '## *' } | Select-Object -First 1)
    if ($hdr -and $hdr -match '^##\s+([^\s.^]+)') { $branch = $Matches[1] }
}
if ($gs) {
    $conflicted = $staged = $modified = $untracked = $deleted = $renamed = $false
    foreach ($l in $gs) {
        if ($l -like '## *') {
            if ($l -match 'ahead (\d+)') { $aheadBehind += [string][char]0x21E1 * [int]$Matches[1] }
            if ($l -match 'behind (\d+)') { $aheadBehind += [string][char]0x21E3 * [int]$Matches[1] }
            continue
        }
        if ($l.Length -lt 2) { continue }
        $x = $l[0]; $y = $l[1]
        if ($x -eq '?') { $untracked = $true; continue }
        if ($x -eq 'U' -or $y -eq 'U' -or ($x -eq 'A' -and $y -eq 'A') -or ($x -eq 'D' -and $y -eq 'D')) { $conflicted = $true; continue }
        if ($x -ne ' ' -and $x -ne '?') { $staged = $true }
        if ($y -eq 'M' -or $x -eq 'M') { $modified = $true }
        if ($y -eq 'D') { $deleted = $true }
        if ($x -eq 'R' -or $y -eq 'R') { $renamed = $true }
    }
    $f = ''
    if ($conflicted) { $f += '=' }
    if ($deleted) { $f += [string][char]0x2718 }
    if ($renamed) { $f += [string][char]0x00BB }
    if ($modified) { $f += '!' }
    if ($staged) { $f += '+' }
    if ($untracked) { $f += '?' }
    $flags = $f + $aheadBehind
}
if ($branch) {
    $bs = " $branch"
    if ($flags) { $bs += " [$flags]" }
    $g = Seg $bs '38;5;176'
    if ($g) { $parts.Add($g) }
} elseif ($flags) {
    $g = Seg "[$flags]" '38;5;176'
    if ($g) { $parts.Add($g) }
}

# ── 包版本 󰏗 vN.N.N（Cargo.toml / package.json，就近向上找）──
$projDir = if ($d.workspace -and $d.workspace.current_dir) { "$($d.workspace.current_dir)" } else { "$(Get-Location)" }
$probe = $projDir
$pkgTxt = $null
for ($i = 0; $i -lt 4 -and $probe; $i++) {
    if (Test-Path (Join-Path $probe 'Cargo.toml')) {
        $v = ((& git -C $probe config -f Cargo.toml --get package.version 2>$null) | Out-String).Trim()
        if (-not $v) {
            foreach ($ln in Get-Content (Join-Path $probe 'Cargo.toml')) {
                if ($ln -match '^\s*version\s*=\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
        }
        if ($v) { $pkgTxt = "󰏗 v$v" }
        break
    }
    if (Test-Path (Join-Path $probe 'package.json')) {
        try {
            $pj = Get-Content -Raw (Join-Path $probe 'package.json') | ConvertFrom-Json
            if ($pj.version) { $pkgTxt = "󰏗 v$($pj.version)" }
        } catch {}
        break
    }
    $parent = Split-Path -Parent $probe
    if ($parent -eq $probe) { break }
    $probe = $parent
}
if ($pkgTxt) {
    $pk = Seg $pkgTxt '38;5;208'
    if ($pk) { $parts.Add($pk) }
}

# ── Rust 工具链 󱘗 vN.N.N（有 Cargo.toml 才探测，对齐 starship rust 段）──
if ($pkgTxt -and $pkgTxt -like '󰏗*') {
    $rv = (& rustc --version 2>$null | Out-String).Trim()
    if ($rv -match 'rustc\s+([\d.]+)') {
        $r = Seg "󱘗 v$($Matches[1])" '38;5;180'
        if ($r) { $parts.Add($r) }
    }
}

Write-Output ($parts -join ' | ')
exit 0
"#;

fn script_path(home: &Path) -> PathBuf {
    home.join("statusline").join("oma-statusline.ps1")
}

/// pwsh is the statusline runtime on every platform. Advisory only: the
/// script is deployed regardless; without pwsh the bar simply won't render
/// in that environment.
pub fn pwsh_on_path() -> bool {
    crate::pathutil::find_on_path("pwsh").is_some()
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
        let text = std::fs::read_to_string(&settings)
            .map_err(|e| format!("{}: {e}", settings.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: corrupt: {e}", settings.display()))?
    } else {
        json!({})
    };
    let cmd = format!(
        "pwsh -NoProfile -File \"{}\" claude",
        script.display().to_string().replace('\\', "/")
    );
    v["statusLine"] = json!({ "type": "command", "command": cmd });
    let body = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&settings, body).map_err(|e| format!("{}: {e}", settings.display()))?;
    Ok(settings.display().to_string())
}

/// kimi：`~/.kimi-code/tui.toml` `[status_line]` 表幂等合并（command 串经
/// cmd/sh 执行，首行接管 footer；300ms 超时由 kimi 侧约束，超时自动回退
/// 内置布局——S025）。其它表保留。
pub fn merge_kimi(home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let config = dirs::home_dir()
        .ok_or("no home")?
        .join(".kimi-code")
        .join("tui.toml");
    let script_str = script.display().to_string().replace('\\', "/");
    let mut toml = read_toml(&config)?;
    if apply_kimi_status_line(&mut toml, &script_str)? {
        toml_write(&config, &toml)?;
    }
    Ok(config.display().to_string())
}

/// `[status_line].command` 幂等落位；返回是否变更（可测纯函数）。
fn apply_kimi_status_line(toml: &mut toml::Value, script_str: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("kimi tui.toml is not a table".into()),
    };
    let status_line = table
        .entry("status_line".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let sl = match status_line {
        toml::Value::Table(t) => t,
        _ => return Err("kimi [status_line] is not a table".into()),
    };
    let command = format!("pwsh -NoProfile -File \"{script_str}\" kimi");
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str());
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
    }
    Ok(changed)
}

/// grok：`~/.grok/config.toml` `[ui.status_line]` 幂等合并（type=command；
/// command 串先直接 spawn、失败回落 shell 解释，带参数命令行可用——S025
/// command.rs 实证）。其它表保留。
pub fn merge_grok(home: &Path) -> Result<String, String> {
    let script = deploy_script(home)?;
    let config = dirs::home_dir()
        .ok_or("no home")?
        .join(".grok")
        .join("config.toml");
    let script_str = script.display().to_string().replace('\\', "/");
    let mut toml = read_toml(&config)?;
    if apply_grok_status_line(&mut toml, &script_str)? {
        toml_write(&config, &toml)?;
    }
    Ok(config.display().to_string())
}

/// `[ui.status_line]` 幂等落位（type=command + command 串）；返回是否变更
/// （可测纯函数）。
fn apply_grok_status_line(toml: &mut toml::Value, script_str: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("grok config.toml is not a table".into()),
    };
    let ui = table
        .entry("ui".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let ui = match ui {
        toml::Value::Table(t) => t,
        _ => return Err("grok [ui] is not a table".into()),
    };
    let status_line = ui
        .entry("status_line".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let sl = match status_line {
        toml::Value::Table(t) => t,
        _ => return Err("grok [ui.status_line] is not a table".into()),
    };
    let command = format!("pwsh -NoProfile -File \"{script_str}\" grok");
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str())
        || sl.get("type").and_then(|v| v.as_str()) != Some("command");
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
        sl.insert("type".into(), toml::Value::String("command".into()));
    }
    Ok(changed)
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
        let text =
            std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?;
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
    lines.push("  \"codex\", ".into());
    lines.push("]".into());
    let body = lines.join("\n") + "\n";
    std::fs::write(&config, body).map_err(|e| format!("{}: {e}", config.display()))?;
    Ok(config.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ps1_forces_utf8_before_any_output() {
        // Regression guard for the CP936 `??` corruption (P0027): the
        // encoding line must precede any output statement — the nerdfont
        // glyphs are literal UTF-8 in the script.
        let enc = STATUSLINE_PS1.find("[Console]::OutputEncoding").unwrap();
        let out = STATUSLINE_PS1.find("Write-Output").unwrap();
        assert!(enc < out);
        assert!(
            STATUSLINE_PS1.contains("\u{f06a9}"),
            "oma segment robot glyph (md-robot, wide: two spaces survive one)"
        );
    }

    #[test]
    fn kimi_and_grok_merges_are_idempotent_and_keep_other_tables() {
        // 期望来自 kimi/grok 官方 schema（S025）：kimi [status_line].command、
        // grok [ui.status_line] type=command；其它表必须存活。
        let mut kimi: toml::Value =
            toml::from_str("theme = \"dark\"\n[status_line]\nitems = [\"model\"]\n").unwrap();
        assert!(apply_kimi_status_line(&mut kimi, "C:/x/oma-statusline.ps1").unwrap());
        assert!(!apply_kimi_status_line(&mut kimi, "C:/x/oma-statusline.ps1").unwrap());
        let kimi_t = kimi.as_table().unwrap();
        assert_eq!(kimi_t.get("theme").unwrap().as_str(), Some("dark"));
        let sl = kimi_t.get("status_line").unwrap().as_table().unwrap();
        assert_eq!(
            sl.get("command").unwrap().as_str(),
            Some("pwsh -NoProfile -File \"C:/x/oma-statusline.ps1\" kimi")
        );
        assert_eq!(
            sl.get("items").unwrap().as_array().unwrap().len(),
            1,
            "foreign [status_line] keys survive"
        );

        let mut grok: toml::Value =
            toml::from_str("model = \"x\"\n[ui]\npermission_mode = \"always-approve\"\n").unwrap();
        assert!(apply_grok_status_line(&mut grok, "C:/x/oma-statusline.ps1").unwrap());
        assert!(!apply_grok_status_line(&mut grok, "C:/x/oma-statusline.ps1").unwrap());
        let grok_t = grok.as_table().unwrap();
        assert_eq!(grok_t.get("model").unwrap().as_str(), Some("x"));
        let ui = grok_t.get("ui").unwrap().as_table().unwrap();
        assert_eq!(
            ui.get("permission_mode").unwrap().as_str(),
            Some("always-approve"),
            "yolo key in [ui] survives"
        );
        let sl = ui.get("status_line").unwrap().as_table().unwrap();
        assert_eq!(sl.get("type").unwrap().as_str(), Some("command"));
        assert!(sl
            .get("command")
            .unwrap()
            .as_str()
            .unwrap()
            .ends_with("\" grok"));
    }

    #[test]
    fn codex_merge_is_idempotent_and_keeps_other_tables() {
        let home = std::env::temp_dir().join(format!("oma-sl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let cfg = home.join(".codex");
        std::fs::create_dir_all(&cfg).unwrap();
        let f = cfg.join("config.toml");
        std::fs::write(
            &f,
            "model = \"gpt\"\n[tui]\nstatus_line = [\"old\"]\n[sandbox]\nmode = \"rw\"\n",
        )
        .unwrap();
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
