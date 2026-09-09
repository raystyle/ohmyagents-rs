//! agent 状态栏配置（用户定调 2026-09-01，参考 ohmypwsh 幂等合并形态）：
//! - claude code：`~/.claude/settings.json` 合并 `statusLine` 块（serde_json
//!   读改写，保留 env/permissions 等，只覆盖 statusLine 键）
//! - codex：`~/.codex/config.toml` 顶层 `[tui]` 段整段替换（幂等），
//!   `status_line` 为内置项 ID 数组（Codex 无外部命令面，S016）
//! 状态栏脚本本体（pwsh）随 oma 释放到 `~/.oma/statusline/`。
//! 用户定调 2026-09-02：渲染对齐用户 starship 配置风格（目录截断、git 旗标、
//! 包与工具链版本段、nerdfont 图标、Catppuccin 系 256 色）；oma 段 = 当前
//! agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记见 S025），另探测
//! agent 宿主 shell（macOS 走 ps 兜底）。

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::yolo::{read_toml, toml_write};

/// 状态栏脚本 HEAD：param、首行强制 UTF-8（CP936 控制台下 emoji 会被替换成
/// 字面 `??`，S024）、stdin JSON 解析（claude code 供给 model 等；codex 无
/// stdin 数据时退化）、Seg 与 FmtTok、FmtDur 助手、`$parts` 收集器。
const PS1_HEAD: &str = r#"
param([string]$AgentName = 'agent')
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'
$raw = [Console]::In.ReadToEnd()
$d = $null
if (-not [string]::IsNullOrWhiteSpace($raw)) { try { $d = $raw | ConvertFrom-Json } catch {} }
# Grok TUI 字体没有 Nerd 私用区字形，PUA 图标显示成替换符（M046）。
$nerd = $AgentName -ne 'grok'

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
"#;

/// COMMON：工作目录与仓库根发现（目录段与 oma 段共用）。段序含 dir 或 oma
/// 才拼入：rev-parse 是子进程，无人消费时省掉（kimi 300ms 预算，S025）。
const PS1_COMMON: &str = r#"
# ── 工作目录与仓库根（oma 段与目录段共用）──
$dir = $null
if ($d.workspace) { $dir = "$($d.workspace.current_dir)" }
if (-not $dir -or $dir -eq '.') { $dir = "$($d.cwd)" }
if (-not $dir -or $dir -eq '.') { $dir = "$(Get-Location)" }
$root = (& git -C $dir rev-parse --show-toplevel 2>$null | Out-String).Trim()
"#;

/// shell 段：agent 宿主 shell（祖先链跳过 agent 本体，向上找最近 shell）。
const SEG_SHELL: &str = r#"
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
    $shKey = if ($shellName -match '^(pwsh|powershell)') { 'shell-pwsh' } else { 'shell' }
    $shLabel = ApplyFmt (Tmpl 'shell') @{ icon = (Ico $shKey); name = $shellName }
    $sh = Seg $shLabel '38;5;245'
    if ($sh) { $parts.Add($sh) }
}
"#;

/// dir 段：完整路径（用户定调 2026-09-02）。
const SEG_DIR: &str = r#"
# ── 目录：完整路径（用户定调 2026-09-02）──
if ($dir) {
    $p = Seg (ApplyFmt (Tmpl 'dir') @{ path = $dir }) '38;5;39'
    if ($p) { $parts.Add($p) }
}
"#;

/// oma 段：当前 agent 名 + 实时四态（hook 状态通道 + 会话闸，机读标记 S025）。
const SEG_OMA: &str = r#"
# ── oma 段：当前 agent 名 + 实时状态（hook 状态通道；机读标记见 S025）──
# agent 名：oma 会话 env 优先，部署参数次之（每家配置注入自家名字）。
$agent = if ($env:OMA_AGENT) { $env:OMA_AGENT } else { $AgentName }
# 状态：会话状态文件优先，回退项目 .oma/state/<agent>.json（旧名 .ohmyagents）。
$state = $null
$stateFile = $env:OHMYAGENTS_STATE_FILE
if (-not $stateFile) {
    $base = if ($root) { $root } else { $dir }
    if ($base) {
        $omaDir = Join-Path $base '.oma'
        if (-not (Test-Path $omaDir)) { $omaDir = Join-Path $base '.ohmyagents' }
        $stateFile = Join-Path (Join-Path $omaDir 'state') "$agent.json"
    }
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
$omaTxt = ApplyFmt (Tmpl 'oma') @{ icon = (Ico 'oma'); agent = $agent; state = $state }
$parts.Add((Seg $omaTxt $stateColor))
"#;

/// model 段：display_name 优先，回退 id。
const SEG_MODEL: &str = r#"
# ── 模型（display_name 优先，回退 id）──
$model = $null
if ($d.model) {
    $model = if ($d.model.display_name) { "$($d.model.display_name)" } else { "$($d.model.id)" }
}
if ($model) {
    $modelTxt = ApplyFmt (Tmpl 'model') @{ icon = (Ico 'model'); model = $model }
    $m = Seg $modelTxt '38;5;147'
    if ($m) { $parts.Add($m) }
}
"#;

/// context 段：已用百分比（已用/窗口），对齐 Codex 语义。
const SEG_CONTEXT: &str = r#"
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
        $ctxTxt = ApplyFmt (Tmpl 'context') @{ icon = (Ico 'context'); pct = [string]$usedPct; used = (FmtTok $usedTok); window = (FmtTok $win) }
        $c = Seg $ctxTxt '38;5;116'
        if ($c) { $parts.Add($c) }
    }
}
"#;

/// duration 段：会话累计时长（claude cost 段；无则省略）。
const SEG_DURATION: &str = r#"
# ── 会话累计：󰅐 时长（claude cost 段；无则省略。成本数字对网关计价不准，不展示）──
if ($d.cost) {
    if ($null -ne $d.cost.total_duration_ms -and [double]$d.cost.total_duration_ms -ge 1000) {
        $durTxt = ApplyFmt (Tmpl 'duration') @{ icon = (Ico 'duration'); duration = (FmtDur ([double]$d.cost.total_duration_ms)) }
        $dur = Seg $durTxt '38;5;245'
        if ($dur) { $parts.Add($dur) }
    }
}
"#;

/// git 段：分支与状态旗标 [!?]（starship 符号语义，porcelain 单次调用）。
const SEG_GIT: &str = r#"
# ── Git：分支  + 状态旗标 [!?]（starship 符号语义，porcelain 单次调用）──
$branch = $null
if ($d.worktree -and $d.worktree.branch) { $branch = "$($d.worktree.branch)" }
if (-not $branch -and $d.workspace -and $d.workspace.branch) { $branch = "$($d.workspace.branch)" }
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
            if ($l -match 'ahead (\d+)') {
                $n = [int]$Matches[1]
                $aheadBehind += if ($nerd) { [string][char]0x21E1 * $n } else { "+$n" }
            }
            if ($l -match 'behind (\d+)') {
                $n = [int]$Matches[1]
                $aheadBehind += if ($nerd) { [string][char]0x21E3 * $n } else { "-$n" }
            }
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
    if ($deleted) { $f += if ($nerd) { [string][char]0x2718 } else { 'x' } }
    if ($renamed) { $f += if ($nerd) { [string][char]0x00BB } else { '>' } }
    if ($modified) { $f += '!' }
    if ($staged) { $f += '+' }
    if ($untracked) { $f += '?' }
    $flags = $f + $aheadBehind
}
if ($branch -or $flags) {
    $branchTxt = if ($branch) { " $branch" } else { '' }
    $flagTxt = if ($flags) { " [$flags]" } else { '' }
    $g = Seg (ApplyFmt (Tmpl 'git') @{ branch = $branchTxt; flags = $flagTxt }) '38;5;176'
    if ($g) { $parts.Add($g) }
}
"#;

/// PROBE：projKind 与包版本文本探测（包版本与七个工具链段共享 `$projKind`；
/// D11 first-match 序 rust / node / python 先于 zig / go / cpp）。段序含
/// package 或任一工具链段才拼入（文件读加 git 子进程，无人消费时省掉）。
const PS1_PROBE: &str = r#"
# ── 包版本 󰏗 vN.N.N（Cargo.toml / package.json，就近向上找）──
$projDir = if ($d.workspace -and $d.workspace.current_dir) { "$($d.workspace.current_dir)" } else { "$(Get-Location)" }
$probe = $projDir
$pkgVer = $null
$projKind = $null
for ($i = 0; $i -lt 4 -and $probe; $i++) {
    if (Test-Path (Join-Path $probe 'Cargo.toml')) {
        $v = ((& git -C $probe config -f Cargo.toml --get package.version 2>$null) | Out-String).Trim()
        if (-not $v) {
            foreach ($ln in Get-Content (Join-Path $probe 'Cargo.toml')) {
                if ($ln -match '^\s*version\s*=\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
        }
        if ($v) { $pkgVer = "v$v" }
        $projKind = 'rust'
        break
    }
    if (Test-Path (Join-Path $probe 'package.json')) {
        try {
            $pj = Get-Content -Raw (Join-Path $probe 'package.json') | ConvertFrom-Json
            if ($pj.version) { $pkgVer = "v$($pj.version)" }
        } catch {}
        $projKind = 'node'
        break
    }
    if ((Test-Path (Join-Path $probe 'pyproject.toml')) -or
        (Test-Path (Join-Path $probe 'uv.lock')) -or
        (Test-Path (Join-Path $probe 'requirements.txt'))) {
        foreach ($ln in Get-Content (Join-Path $probe 'pyproject.toml') -ErrorAction SilentlyContinue) {
            if ($ln -match '^\s*version\s*=\s*"([^"]+)"') { $pkgVer = "v$($Matches[1])"; break }
        }
        $projKind = 'python'
        break
    }
    if (Test-Path (Join-Path $probe 'build.zig')) {
        $v = $null
        $zon = Join-Path $probe 'build.zig.zon'
        if (Test-Path $zon) {
            foreach ($ln in Get-Content $zon -ErrorAction SilentlyContinue) {
                if ($ln -match '\.version\s*=\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
            if ($v) { $pkgVer = "v$v" }
        }
        $projKind = 'zig'
        break
    }
    if (Test-Path (Join-Path $probe 'go.mod')) {
        $projKind = 'go'
        break
    }
    if ((Test-Path (Join-Path $probe 'CMakeLists.txt')) -or
        (Test-Path (Join-Path $probe 'meson.build'))) {
        $v = $null
        foreach ($ln in Get-Content (Join-Path $probe 'CMakeLists.txt') -ErrorAction SilentlyContinue) {
            if ($ln -match '(?i)project\s*\([^)]*VERSION\s+([\d.]+)') { $v = $Matches[1]; break }
        }
        if (-not $v) {
            foreach ($ln in Get-Content (Join-Path $probe 'meson.build') -ErrorAction SilentlyContinue) {
                if ($ln -match "version\s*:\s*'([^']+)'") { $v = $Matches[1]; break }
                if (-not $v -and $ln -match 'version\s*:\s*"([^"]+)"') { $v = $Matches[1]; break }
            }
        }
        if ($v) { $pkgVer = "v$v" }
        $projKind = 'cpp'
        break
    }
    $parent = Split-Path -Parent $probe
    if ($parent -eq $probe) { break }
    $probe = $parent
}
"#;

/// package 段：包版本渲染（文本来自 PROBE）。
const SEG_PACKAGE: &str = r#"
if ($pkgVer) {
    $pk = Seg (ApplyFmt (Tmpl 'package') @{ icon = (Ico 'package'); version = $pkgVer }) '38;5;208'
    if ($pk) { $parts.Add($pk) }
}
"#;

/// python 段：Python 工具链（Grok ASCII 路径跳过工具链子进程，M046）。
const SEG_PYTHON: &str = r#"
# ── Python 工具链 󰌠 vN.N.N（pyproject/uv.lock/requirements 项目）──
# Grok TUI 跳过工具链子进程：慢且图标会豆腐（M046）。
if ($nerd -and $projKind -eq 'python') {
    $pv = (& python --version 2>$null | Out-String).Trim()
    if ($pv -match 'Python\s+([\d.]+)') {
        $py = Seg (ApplyFmt (Tmpl 'python') @{ icon = (Ico 'python'); version = "v$($Matches[1])" }) '38;5;143'
        if ($py) { $parts.Add($py) }
    }
}
"#;

/// rust 段：Rust 工具链（projKind 判型才探测）。
const SEG_RUST: &str = r#"
# ── Rust 工具链 󱘗 vN.N.N（Cargo.toml 项目才探测——projKind 判，不再
#    「有包版本就探测」：TS 项目曾因此误出 rust 段）──
if ($nerd -and $projKind -eq 'rust') {
    $rv = (& rustc --version 2>$null | Out-String).Trim()
    if ($rv -match 'rustc\s+([\d.]+)') {
        $r = Seg (ApplyFmt (Tmpl 'rust') @{ icon = (Ico 'rust'); version = "v$($Matches[1])" }) '38;5;180'
        if ($r) { $parts.Add($r) }
    }
}
"#;

/// node 段：Node/TS 工具链（TS 就绪再叠 ts 版本，不起 tsc 子进程）。
const SEG_NODE: &str = r#"
# ── Node/TS 工具链 󰎙 vN.N.N（package.json 项目；TS 就绪再叠 󰛦 vM.M.M，
#    typescript 版本就近读 node_modules 不起 tsc 子进程）──
if ($nerd -and $projKind -eq 'node') {
    $nv = (& node --version 2>$null | Out-String).Trim()
    if ($nv -match 'v?([\d.]+)') {
        $n = Seg (ApplyFmt (Tmpl 'node') @{ icon = (Ico 'node'); version = "v$($Matches[1])" }) '38;5;078'
        if ($n) { $parts.Add($n) }
    }
    $tsProbe = $projDir
    for ($j = 0; $j -lt 4 -and $tsProbe; $j++) {
        $tsPj = Join-Path $tsProbe 'node_modules\typescript\package.json'
        if (Test-Path $tsPj) {
            try {
                $tj = Get-Content -Raw $tsPj | ConvertFrom-Json
                if ($tj.version) {
                    $t = Seg (ApplyFmt (Tmpl 'ts') @{ icon = (Ico 'ts'); version = "v$($tj.version)" }) '38;5;067'
                    if ($t) { $parts.Add($t) }
                }
            } catch {}
            break
        }
        $p2 = Split-Path -Parent $tsProbe
        if ($p2 -eq $tsProbe) { break }
        $tsProbe = $p2
    }
}
"#;

/// zig 段：Zig 工具链 seti-zig U+E6A9（D11）。
const SEG_ZIG: &str = r#"
# ── Zig 工具链 seti-zig U+E6A9（cmap: CaskaydiaCove 与 0xProto 2026-09-07）──
if ($nerd -and $projKind -eq 'zig') {
    $zv = (& zig version 2>$null | Out-String).Trim()
    if ($zv -match '^([\d.]+)') {
        $z = Seg (ApplyFmt (Tmpl 'zig') @{ icon = (Ico 'zig'); version = "v$($Matches[1])" }) '38;5;178'
        if ($z) { $parts.Add($z) }
    }
}
"#;

/// go 段：Go 工具链 seti-go U+E627（D11）。
const SEG_GO: &str = r#"
# ── Go 工具链 seti-go U+E627 ──
if ($nerd -and $projKind -eq 'go') {
    $gv = (& go version 2>$null | Out-String).Trim()
    if ($gv -match 'go([\d.]+)') {
        $goSeg = Seg (ApplyFmt (Tmpl 'go') @{ icon = (Ico 'go'); version = "v$($Matches[1])" }) '38;5;080'
        if ($goSeg) { $parts.Add($goSeg) }
    }
}
"#;

/// cpp 段：C/C++ 工具链 seti-cpp U+E646（c++/g++/clang++，静默失败）。
const SEG_CPP: &str = r#"
# ── C/C++ 工具链 seti-cpp U+E646（c++/g++/clang++，静默失败）──
if ($nerd -and $projKind -eq 'cpp') {
    $cv = (& c++ --version 2>$null | Out-String).Trim()
    if (-not $cv) { $cv = (& g++ --version 2>$null | Out-String).Trim() }
    if (-not $cv) { $cv = (& clang++ --version 2>$null | Out-String).Trim() }
    if ($cv -match '(\d+\.\d+(?:\.\d+)?)') {
        $cx = Seg (ApplyFmt (Tmpl 'cpp') @{ icon = (Ico 'cpp'); version = "v$($Matches[1])" }) '38;5;110'
        if ($cx) { $parts.Add($cx) }
    }
}
"#;

/// TAIL：单行收口输出。
const PS1_TAIL: &str = "\nWrite-Output ($parts -join ' | ')\nexit 0\n";

/// 段 id 到脚本块查表。未知 id 报错：拼装无法命中段块。
fn segment_block(id: &str) -> Result<&'static str, String> {
    Ok(match id {
        "shell" => SEG_SHELL,
        "dir" => SEG_DIR,
        "oma" => SEG_OMA,
        "model" => SEG_MODEL,
        "context" => SEG_CONTEXT,
        "duration" => SEG_DURATION,
        "git" => SEG_GIT,
        "package" => SEG_PACKAGE,
        "python" => SEG_PYTHON,
        "rust" => SEG_RUST,
        "node" => SEG_NODE,
        "zig" => SEG_ZIG,
        "go" => SEG_GO,
        "cpp" => SEG_CPP,
        _ => return Err(format!("unknown statusline segment: {id}")),
    })
}

/// 默认段序：等于拆段前脚本顺序（D18 定制面 segments 键缺省回落此序）。
pub(crate) const DEFAULT_SEGMENTS: &[&str] = &[
    "shell", "dir", "oma", "model", "context", "duration", "git", "package", "python", "rust",
    "node", "zig", "go", "cpp",
];

/// 内嵌默认模板（D18）。键 = 段 id；`context-ascii` 是 grok 的结构差异项
/// （nerd 版带 used/window 括号对，ascii 版只有百分比加 ctx 后缀）。
/// 各段可用占位符见 R002；git 段默认前导空格在 branch / flags 变量里。
const DEFAULT_TEMPLATES: &[(&str, &str)] = &[
    ("shell", "{icon}{name}"),
    ("dir", "{path}"),
    ("oma", "{icon}{agent}:{state}"),
    ("model", "{icon}{model}"),
    ("context", "{icon}{pct}% ({used}/{window})"),
    ("context-ascii", "{pct}% ctx"),
    ("duration", "{icon}{duration}"),
    ("git", "{branch}{flags}"),
    ("package", "{icon}{version}"),
    ("python", "{icon}{version}"),
    ("rust", "{icon}{version}"),
    ("node", "{icon}{version}"),
    ("ts", "{icon}{version}"),
    ("zig", "{icon}{version}"),
    ("go", "{icon}{version}"),
    ("cpp", "{icon}{version}"),
];

/// 内嵌默认图标（码位与拆段前脚本逐字对齐；oma 机器人宽字形跟两空格）。
/// Grok ASCII 路径图标恒空串（M046）。
const DEFAULT_ICONS: &[(&str, &str)] = &[
    ("shell-pwsh", "\u{ebc7} "),
    ("shell", "\u{ea85} "),
    ("oma", "\u{f06a9}  "),
    ("model", "\u{2726} "),
    ("context", "\u{f035b} "),
    ("duration", "\u{f0150} "),
    ("package", "\u{f03d7} "),
    ("python", "\u{f0320} "),
    ("rust", "\u{f1617} "),
    ("node", "\u{f0399} "),
    ("ts", "\u{f06e6} "),
    ("zig", "\u{e6a9} "),
    ("go", "\u{e627} "),
    ("cpp", "\u{e646} "),
];

/// 配置块的取用与占位替换助手（随烘焙块注入，紧跟 HEAD）。
const PS1_CFG_HELPERS: &str = r#"
function Tmpl([string]$k) {
    if (-not $nerd -and $slTmpl.ContainsKey("$k-ascii")) { return "$($slTmpl["$k-ascii"])" }
    return "$($slTmpl[$k])"
}
function Ico([string]$k) {
    if ($nerd -and $slIcon.ContainsKey($k)) { return "$($slIcon[$k])" }
    return ''
}
function ApplyFmt([string]$fmt, [hashtable]$vars) {
    foreach ($k in @($vars.Keys)) { $fmt = $fmt.Replace(('{' + $k + '}'), [string]$vars[$k]) }
    return $fmt
}
"#;

/// ps1 单引号字面量（内嵌单引号加倍；用户值无法越出字面量）。
fn ps1_sq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn lookup_override<'a>(user: &'a [(String, String)], key: &str) -> Option<&'a str> {
    user.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// 烘焙定制块：`$slTmpl` / `$slIcon` 已并入用户覆盖（脚本侧零回落逻辑，
/// 默认全键在场）。值经单引号转义，用户串无法越出字面量（模板注入不成立）。
fn render_cfg_block(cfg: &StatuslineConfig) -> String {
    let mut out = String::from(
        "\n# ── D18 定制烘焙：模板与图标（~/.oma/statusline.toml 键级回落内嵌默认）──\n$slTmpl = @{\n",
    );
    for (k, v) in DEFAULT_TEMPLATES {
        let merged = lookup_override(&cfg.template, k).unwrap_or(v);
        out.push_str(&format!("    {} = {}\n", ps1_sq(k), ps1_sq(merged)));
    }
    out.push_str("}\n$slIcon = @{\n");
    for (k, v) in DEFAULT_ICONS {
        let merged = lookup_override(&cfg.icons, k).unwrap_or(v);
        out.push_str(&format!("    {} = {}\n", ps1_sq(k), ps1_sq(merged)));
    }
    out.push_str("}\n");
    out.push_str(PS1_CFG_HELPERS);
    out
}

/// 按段序拼装状态栏脚本：HEAD 加烘焙定制块加（按需）COMMON / PROBE 加段块
/// 加 TAIL。COMMON 只在段序含 dir / oma 时拼入（rev-parse 子进程无人消费时
/// 省掉）；PROBE 只在段序含 package 或任一工具链段时拼入。重复段 id、
/// 未知段 id、未知模板或图标键报错；段序为空产出空栏（用户显式所为）。
pub(crate) fn assemble_statusline_ps1(
    order: &[&str],
    cfg: &StatuslineConfig,
) -> Result<String, String> {
    for (k, _) in &cfg.template {
        if !DEFAULT_TEMPLATES.iter().any(|(dk, _)| *dk == k) {
            return Err(format!("unknown statusline template key: {k}"));
        }
    }
    for (k, _) in &cfg.icons {
        if !DEFAULT_ICONS.iter().any(|(dk, _)| *dk == k) {
            return Err(format!("unknown statusline icon key: {k}"));
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = String::with_capacity(14 * 1024);
    out.push_str(PS1_HEAD);
    out.push_str(&render_cfg_block(cfg));
    if order.iter().any(|id| *id == "dir" || *id == "oma") {
        out.push_str(PS1_COMMON);
    }
    if order.iter().any(|id| {
        matches!(
            *id,
            "package" | "python" | "rust" | "node" | "zig" | "go" | "cpp"
        )
    }) {
        out.push_str(PS1_PROBE);
    }
    for id in order {
        if !seen.insert(id) {
            return Err(format!("duplicate statusline segment: {id}"));
        }
        out.push_str(segment_block(id)?);
    }
    out.push_str(PS1_TAIL);
    Ok(out)
}

/// 默认脚本：默认段序加全默认定制拼装（静态合法，失败即程序性 bug）。
pub(crate) fn default_statusline_ps1() -> String {
    assemble_statusline_ps1(DEFAULT_SEGMENTS, &StatuslineConfig::default())
        .expect("default segment order is valid")
}

/// `~/.oma/statusline.toml` 用户级定制（D18）。键级缺省回落内嵌默认：
/// 没写的键用默认，写下的键生效；坏文件硬错退出 1。
#[derive(Debug, Default, PartialEq)]
pub struct StatuslineConfig {
    /// `segments`：段 id 数组即全量序（显隐加顺序）；键缺省回落
    /// `DEFAULT_SEGMENTS`。
    pub segments: Option<Vec<String>>,
    /// `[template]`：段格式串（键 = 段 id；`<段>-ascii` 为 grok 结构差异项）；
    /// 键级回落 `DEFAULT_TEMPLATES`。
    pub template: Vec<(String, String)>,
    /// `[icons]`：图标映射（含 `ts`、`shell-pwsh` 子项键）；键级回落
    /// `DEFAULT_ICONS`。Grok ASCII 路径图标恒空（M046）。
    pub icons: Vec<(String, String)>,
    /// `[codex] items`：codex 内置项 ID 子集（原样透传，未知 id codex 侧
    /// 静默跳过）；键缺省回落 `CODEX_STATUS_LINE_ITEMS`。
    pub codex_items: Option<Vec<String>>,
}

pub(crate) fn config_path(home: &Path) -> PathBuf {
    home.join("statusline.toml")
}

/// 读用户定制配置；文件不存在回落全默认（不是错误）。
pub fn read_config(home: &Path) -> Result<StatuslineConfig, String> {
    let p = config_path(home);
    if !p.exists() {
        return Ok(StatuslineConfig::default());
    }
    let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    parse_config(&text).map_err(|e| format!("{}: {e}", p.display()))
}

/// 纯函数：解析配置（可测）。坏文件硬错（拼装层面无法兜底）；缺键回落。
fn parse_config(text: &str) -> Result<StatuslineConfig, String> {
    let v: toml::Value = toml::from_str(text).map_err(|e| format!("parse: {e}"))?;
    let mut cfg = StatuslineConfig::default();
    if let Some(segs) = v.get("segments") {
        let arr = segs.as_array().ok_or("segments 必须是段 id 字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(s.as_str().ok_or("segments 元素必须是字符串")?.to_string());
        }
        cfg.segments = Some(out);
    }
    for (key, slot) in [("template", &mut cfg.template), ("icons", &mut cfg.icons)] {
        if let Some(t) = v.get(key) {
            let t = t.as_table().ok_or_else(|| format!("{key} 必须是表"))?;
            for (k, val) in t {
                let s = val
                    .as_str()
                    .ok_or_else(|| format!("{key}.{k} 必须是字符串"))?;
                slot.push((k.clone(), s.to_string()));
            }
        }
    }
    if let Some(items) = v.get("codex").and_then(|c| c.get("items")) {
        let arr = items.as_array().ok_or("[codex] items 必须是字符串数组")?;
        let mut out = Vec::with_capacity(arr.len());
        for s in arr {
            out.push(
                s.as_str()
                    .ok_or("[codex] items 元素必须是字符串")?
                    .to_string(),
            );
        }
        cfg.codex_items = Some(out);
    }
    Ok(cfg)
}

/// 段序生效值：用户清单或默认序（未知与重复 id 由拼装器拒）。
fn effective_order(cfg: &StatuslineConfig) -> Result<Vec<&str>, String> {
    Ok(match &cfg.segments {
        Some(segs) => segs.iter().map(String::as_str).collect(),
        None => DEFAULT_SEGMENTS.to_vec(),
    })
}

pub(crate) fn script_path(home: &Path) -> PathBuf {
    home.join("statusline").join("oma-statusline.ps1")
}

pub(crate) fn grok_cmd_path(home: &Path) -> PathBuf {
    home.join("statusline").join("oma-statusline-grok.cmd")
}

/// Windows Grok `[ui.status_line].command` must be a single spawnable path.
/// grok-build `command.rs` does `Command::new(entire_string)` first and only
/// falls back to a shell on `NotFound` (or Unix ENOEXEC). A `pwsh -File "..."`
/// line contains quotes and slashes, so Windows returns ERROR_INVALID_NAME
/// 123 and paints `[status line: could not start the script: ...]` (M048).
const STATUSLINE_GROK_CMD: &str =
    "@echo off\r\npwsh -NoProfile -File \"%~dp0oma-statusline.ps1\" grok\r\n";

fn grok_command_line(script_str: &str) -> String {
    #[cfg(windows)]
    {
        match script_str.rsplit_once('/') {
            Some((dir, _)) => format!("{dir}/oma-statusline-grok.cmd"),
            None => "oma-statusline-grok.cmd".into(),
        }
    }
    #[cfg(not(windows))]
    {
        format!("pwsh -NoProfile -File \"{script_str}\" grok")
    }
}

/// pwsh is the statusline runtime on every platform. Advisory only: the
/// script is deployed regardless; without pwsh the bar simply won't render
/// in that environment.
pub fn pwsh_on_path() -> bool {
    crate::pathutil::find_on_path("pwsh").is_some()
}

/// 自备脚本标记（D18 整脚本替换）：`<部署脚本>.custom`，内容为源路径。
/// 在场时 `deploy_script` 不覆写内嵌拼装产物。
pub(crate) fn marker_path(home: &Path) -> PathBuf {
    let mut s = script_path(home).into_os_string();
    s.push(".custom");
    PathBuf::from(s)
}

/// 自备脚本当前是否在场（kv 面 statusline.custom 用）。
pub fn custom_active(home: &Path) -> bool {
    marker_path(home).exists()
}

/// 释放状态栏脚本（幂等覆写）。按 `~/.oma/statusline.toml` 生成时烘焙：
/// segments 键控段序与显隐，缺省回落内嵌默认（D18）。自备脚本标记在场时
/// 跳过覆写（只保 grok .cmd 壳，见 D18 整脚本替换）。
pub fn deploy_script(home: &Path) -> Result<PathBuf, String> {
    let p = script_path(home);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    if !marker_path(home).exists() {
        let cfg = read_config(home)?;
        let order = effective_order(&cfg)?;
        let script = assemble_statusline_ps1(&order, &cfg).map_err(|e| {
            // 段清单来自用户配置时，错误带上文件出处才可操作。
            if cfg.segments.is_some() {
                format!("{}: {e}", config_path(home).display())
            } else {
                e
            }
        })?;
        std::fs::write(&p, script).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    let cmd = grok_cmd_path(home);
    std::fs::write(&cmd, STATUSLINE_GROK_CMD).map_err(|e| format!("{}: {e}", cmd.display()))?;
    Ok(p)
}

/// 部署用户自备脚本（D18 整脚本替换）：拷到部署位，agent 配置命令行不动
/// （claude / kimi / grok 调用约定不变：首参 agent 名，stdin 喂 agent JSON，
/// stdout 单行状态栏）；marker 记源路径，此后无 `--script` 的重跑跳过内嵌
/// 覆盖。codex 无脚本面（M045），对 codex 只有 `[codex] items` 生效。
pub fn deploy_custom_script(home: &Path, src: &Path) -> Result<PathBuf, String> {
    let text =
        std::fs::read_to_string(src).map_err(|e| format!("--script {}: {e}", src.display()))?;
    if text.trim().is_empty() {
        return Err(format!("--script {}: empty script", src.display()));
    }
    let p = script_path(home);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
    std::fs::write(marker_path(home), format!("{}\n", src.display()))
        .map_err(|e| format!("{}: {e}", marker_path(home).display()))?;
    Ok(p)
}

/// 还原内嵌脚本：删自备标记后重释放（`--builtin`）。
pub fn restore_builtin_script(home: &Path) -> Result<PathBuf, String> {
    let m = marker_path(home);
    if m.exists() {
        std::fs::remove_file(&m).map_err(|e| format!("{}: {e}", m.display()))?;
    }
    deploy_script(home)
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

/// grok：`~/.grok/config.toml` `[ui.status_line]` 幂等合并（type=command）。
/// Windows 写 `.cmd` 单路径（M048）；Unix 仍写 `pwsh -File` 命令行（NotFound
/// 才回落 sh -c）。其它表保留。
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
    let command = grok_command_line(script_str);
    let changed = sl.get("command").and_then(|v| v.as_str()) != Some(command.as_str())
        || sl.get("type").and_then(|v| v.as_str()) != Some("command");
    if changed {
        sl.insert("command".into(), toml::Value::String(command));
        sl.insert("type".into(), toml::Value::String("command".into()));
    }
    Ok(changed)
}

/// `oma agents statusline --example` 打印的带注释全量示例（存到
/// `~/.oma/statusline.toml` 生效）。
pub const EXAMPLE_TOML: &str = r#"# ~/.oma/statusline.toml —— 状态栏用户级定制（D18）
# 生成时烘焙：oma agents statusline 每次运行读本文件重拼脚本后落盘，
# 改完本文件重跑一次 oma agents statusline 生效。
# 键级缺省回落：没写的键用内嵌默认；坏文件硬错退出 1。

# 段落清单：段 id 数组即全量（显隐加顺序）；缺省 = 内嵌默认 14 段全量序。
# 可用段 id：shell / dir / oma / model / context / duration / git / package
#           / python / rust / node / zig / go / cpp
# 例（隐藏 shell 与时长段、git 提到目录前）：
#   segments = ["dir", "git", "oma", "model", "context", "package",
#               "python", "rust", "node", "zig", "go", "cpp"]
segments = ["shell", "dir", "oma", "model", "context", "duration", "git", "package", "python", "rust", "node", "zig", "go", "cpp"]

# 段内模板（[template]）：每段一条格式串；`<段>-ascii` 是 grok 的 ASCII 形
#（缺省同用 nerd 模板、图标恒空）。可用占位符：
#   shell {icon}{name} / dir {path} / oma {icon}{agent}{state}
#   model {icon}{model} / context {icon}{pct}{used}{window} / duration {icon}{duration}
#   git {branch}{flags} / package 与七工具链段（含 ts）{icon}{version}
# 例（oma 段去图标改方括号态）：
# [template]
# oma = "{agent}[{state}]"

# 图标映射（[icons]）：键级回落；oma 机器人宽字形默认跟两空格。
# 可用键：shell / shell-pwsh / oma / model / context / duration / package
#         / python / rust / node / ts / zig / go / cpp
# 例：
# [icons]
# rust = "R "

# codex 内置项子集（[codex] items）：替换写入 ~/.codex/config.toml 的
# [tui].status_line 内置项 ID 清单；未知 id codex 侧静默跳过（S016）。
# 例（只要分支与目录）：
# [codex]
# items = ["current-dir", "git-branch"]
"#;

/// Codex `[tui].status_line` is an ordered list of built-in item IDs
/// (ohmypwsh S016, openai/codex 0.148+). Unknown strings are silently
/// skipped, so a command argv (`"command", "pwsh", "-File", ...`) empties
/// the bar. oma cannot inject a custom script here.
const CODEX_STATUS_LINE_ITEMS: &[&str] = &[
    "run-state",
    "model-with-reasoning",
    "context-remaining",
    "used-tokens",
    "permissions",
    "current-dir",
    "git-branch",
    "branch-changes",
];

fn render_codex_tui_section(items: &[&str]) -> String {
    let mut lines = vec!["[tui]".to_string(), "status_line = [".to_string()];
    let last = items.len().saturating_sub(1);
    for (i, id) in items.iter().enumerate() {
        let comma = if i == last { "" } else { "," };
        lines.push(format!("  \"{id}\"{comma}"));
    }
    lines.push("]".into());
    lines.push("status_line_use_colors = true".into());
    lines.join("\n")
}

fn strip_tui_section(text: &str) -> String {
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
    lines.join("\n")
}

/// Codex: replace the `[tui]` table with built-in item IDs (ohmypwsh S016).
/// Does not deploy the pwsh script; Codex has no command-backed status line.
/// `[codex] items`（D18）用户清单原样透传：codex 对未知 id 静默跳过，oma
/// 不校验清单合法性；键缺省回落内嵌推荐八项。
pub fn merge_codex(home: &Path) -> Result<String, String> {
    let cfg = read_config(home)?;
    let items: Vec<&str> = match &cfg.codex_items {
        Some(list) => list.iter().map(String::as_str).collect(),
        None => CODEX_STATUS_LINE_ITEMS.to_vec(),
    };
    let config = dirs::home_dir()
        .ok_or("no home")?
        .join(".codex")
        .join("config.toml");
    let existing = if config.exists() {
        std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?
    } else {
        String::new()
    };
    let kept = strip_tui_section(&existing);
    let kept = kept.trim_end();
    let body = if kept.is_empty() {
        format!("{}\n", render_codex_tui_section(&items))
    } else {
        format!("{kept}\n\n{}\n", render_codex_tui_section(&items))
    };
    if let Some(dir) = config.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&config, body).map_err(|e| format!("{}: {e}", config.display()))?;
    Ok(config.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 独占临时目录（单测内 fs 落盘判据用；用完即删）。
    fn scratch(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("oma-sl-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_config_reads_segments_and_falls_back_on_missing_key() {
        // 期望值来自 D18 裁定：segments 写下即全量序，键缺省回落默认。
        assert_eq!(parse_config("").unwrap().segments, None);
        assert_eq!(
            parse_config("segments = [\"oma\", \"git\"]\n")
                .unwrap()
                .segments,
            Some(vec!["oma".to_string(), "git".to_string()])
        );
        // segments = [] 是显式空清单（空栏），不回落默认。
        assert_eq!(
            parse_config("segments = []\n").unwrap().segments,
            Some(vec![])
        );
    }

    #[test]
    fn dies_parse_config_rejects_malformed() {
        assert!(parse_config("segments = ").is_err(), "truncated toml");
        assert!(
            parse_config("segments = \"git\"\n").is_err(),
            "not an array"
        );
        assert!(parse_config("segments = [1]\n").is_err(), "not strings");
    }

    #[test]
    fn deploy_script_honors_segment_order_from_config() {
        let home = scratch("order");
        std::fs::write(
            home.join("statusline.toml"),
            "segments = [\"git\", \"oma\"]\n",
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        let g = text.find("# ── Git：").unwrap();
        let o = text.find("# ── oma 段").unwrap();
        assert!(o > g, "git before oma per config");
        assert!(!text.contains("# ── Shell 段"), "shell hidden");
        assert!(text.contains("rev-parse"), "COMMON in for oma");
        assert!(!text.contains("Cargo.toml"), "PROBE gated off");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn deploy_script_without_config_is_default_order() {
        let home = scratch("default");
        let p = deploy_script(&home).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("# ── Shell 段"));
        assert_eq!(
            text,
            default_statusline_ps1(),
            "no config = default assembly"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn dies_deploy_script_rejects_unknown_segment_from_config() {
        let home = scratch("unknown");
        std::fs::write(home.join("statusline.toml"), "segments = [\"nope\"]\n").unwrap();
        let err = deploy_script(&home).unwrap_err();
        assert!(err.contains("unknown statusline segment"), "{err}");
        assert!(
            err.contains("statusline.toml"),
            "error names the config file: {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn parse_config_reads_template_and_icons_tables() {
        let cfg = parse_config("[template]\noma = '[{state}] {agent}'\n\n[icons]\noma = '>'\n\n")
            .unwrap();
        assert_eq!(
            cfg.template,
            vec![("oma".to_string(), "[{state}] {agent}".to_string())]
        );
        assert_eq!(cfg.icons, vec![("oma".to_string(), ">".to_string())]);
    }

    #[test]
    fn dies_parse_config_rejects_non_string_template_or_icon_value() {
        assert!(parse_config("[template]\noma = 1\n").is_err());
        assert!(parse_config("[icons]\noma = true\n").is_err());
        assert!(parse_config("template = \"x\"\n").is_err(), "not a table");
    }

    #[test]
    fn cfg_block_merges_user_over_defaults() {
        let cfg = StatuslineConfig {
            template: vec![("oma".to_string(), "{agent}[{state}]".to_string())],
            icons: vec![("rust".to_string(), "R ".to_string())],
            ..Default::default()
        };
        let block = render_cfg_block(&cfg);
        assert!(
            block.contains("'oma' = '{agent}[{state}]'"),
            "user template wins:\n{block}"
        );
        assert!(block.contains("'rust' = 'R '"), "user icon wins:\n{block}");
        assert!(
            block.contains("'model' = '{icon}{model}'"),
            "untouched defaults survive"
        );
        assert!(
            block.contains("'oma' = '\u{f06a9}  '"),
            "default oma icon keeps the wide-glyph double space"
        );
    }

    #[test]
    fn cfg_values_cannot_escape_single_quote_literals() {
        // 注入判据：用户值内嵌单引号必须加倍，无法越出 ps1 字面量。
        let cfg = StatuslineConfig {
            template: vec![("oma".to_string(), "a'; Remove-Item x; '".to_string())],
            ..Default::default()
        };
        let block = render_cfg_block(&cfg);
        assert!(
            block.contains("'a''; Remove-Item x; '''"),
            "single quotes doubled: {block}"
        );
    }

    #[test]
    fn dies_assemble_rejects_unknown_template_or_icon_key() {
        let cfg = StatuslineConfig {
            template: vec![("nope".to_string(), "x".to_string())],
            ..Default::default()
        };
        let err = assemble_statusline_ps1(DEFAULT_SEGMENTS, &cfg).unwrap_err();
        assert!(err.contains("unknown statusline template key"), "{err}");
        let cfg = StatuslineConfig {
            icons: vec![("nope".to_string(), "x".to_string())],
            ..Default::default()
        };
        let err = assemble_statusline_ps1(DEFAULT_SEGMENTS, &cfg).unwrap_err();
        assert!(err.contains("unknown statusline icon key"), "{err}");
    }

    #[test]
    fn template_override_changes_rendered_output() {
        // pwsh 闸门 skip（R004 形态）：无 pwsh 环境不跑行为判据。
        if !pwsh_on_path() {
            return;
        }
        let home = scratch("tmpl");
        std::fs::write(
            home.join("statusline.toml"),
            "[template]\noma = '{agent}[{state}]'\n",
        )
        .unwrap();
        let p = deploy_script(&home).unwrap();
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut child = Command::new("pwsh")
            .arg("-NoProfile")
            .arg("-File")
            .arg(&p)
            .arg("claude")
            // 封闭性：cwd 钉 scratch（无 git 无 state 文件，状态必 unknown）；
            // oma 段 env 优先于部署参数，宿主会话的 OMA_AGENT 会盖掉传入的
            // claude（本仓 agent 会话里跑测试即翻车）。
            .current_dir(&home)
            .env_remove("OMA_AGENT")
            .env_remove("OHMYAGENTS_STATE_FILE")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"{}").unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(out.status.success());
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("claude[unknown]"),
            "user template wins: {stdout}"
        );
        assert!(
            !stdout.contains('\u{f06a9}'),
            "custom template drops the icon: {stdout}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn custom_script_marker_survives_plain_rerun_and_restores() {
        let home = scratch("custom");
        let src = home.join("my-statusline.ps1");
        std::fs::write(&src, "Write-Output 'my-bar'\n").unwrap();
        let p = deploy_custom_script(&home, &src).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "Write-Output 'my-bar'\n"
        );
        let marker = marker_path(&home);
        assert!(marker.exists(), "marker written");
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            format!("{}\n", src.display()),
            "marker records the source path"
        );
        // 无 --script 的重跑不覆写自备脚本（只保 grok 壳）。
        deploy_script(&home).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "Write-Output 'my-bar'\n",
            "custom script must survive plain rerun"
        );
        assert!(home
            .join("statusline")
            .join("oma-statusline-grok.cmd")
            .exists());
        // --builtin 还原内嵌：marker 删除、内容回拼装产物。
        restore_builtin_script(&home).unwrap();
        assert!(!marker.exists());
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            default_statusline_ps1()
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn dies_deploy_custom_script_rejects_missing_or_empty() {
        let home = scratch("custom-bad");
        let err = deploy_custom_script(&home, &home.join("nope.ps1")).unwrap_err();
        assert!(err.starts_with("--script"), "{err}");
        let empty = home.join("empty.ps1");
        std::fs::write(&empty, "   \n").unwrap();
        let err = deploy_custom_script(&home, &empty).unwrap_err();
        assert!(err.contains("empty script"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn codex_items_config_overrides_builtin_list() {
        // 期望值：用户清单原样透传（含未知 id——codex 侧静默跳过，oma 不拦）。
        let cfg =
            parse_config("[codex]\nitems = [\"current-dir\", \"git-branch\", \"nope\"]\n").unwrap();
        assert_eq!(
            cfg.codex_items,
            Some(vec![
                "current-dir".to_string(),
                "git-branch".to_string(),
                "nope".to_string()
            ])
        );
        let tui = render_codex_tui_section(&["current-dir", "git-branch", "nope"]);
        assert!(tui.contains("\"current-dir\""));
        assert!(tui.contains("\"nope\""));
        assert!(
            !tui.contains("run-state"),
            "default list replaced, not merged"
        );
        let last = tui
            .lines()
            .find(|l| l.trim_start().starts_with('"') && l.contains("nope"))
            .unwrap();
        assert!(!last.trim().ends_with(','), "no trailing comma: {last}");
        // 键缺省：无 [codex] items 时回落内嵌推荐八项。
        assert!(parse_config("").unwrap().codex_items.is_none());
    }

    #[test]
    fn dies_parse_config_rejects_malformed_codex_items() {
        assert!(parse_config("[codex]\nitems = \"x\"\n").is_err());
        assert!(parse_config("[codex]\nitems = [1]\n").is_err());
    }

    #[test]
    fn ps1_forces_utf8_before_any_output() {
        // Regression guard for the CP936 `??` corruption (P0027): the
        // encoding line must precede any output statement — the nerdfont
        // glyphs are literal UTF-8 in the script.
        let ps1 = default_statusline_ps1();
        let enc = ps1.find("[Console]::OutputEncoding").unwrap();
        let out = ps1.find("Write-Output").unwrap();
        assert!(enc < out);
        assert!(
            ps1.contains("\u{f06a9}"),
            "oma segment robot glyph (md-robot, wide: two spaces survive one)"
        );
        assert!(
            ps1.contains("$nerd = $AgentName -ne 'grok'"),
            "Grok TUI has no Nerd PUA glyphs; script must take the ASCII path (M046)"
        );
        assert!(
            ps1.contains("Join-Path $base '.oma'") && ps1.contains("Join-Path $base '.ohmyagents'"),
            "D14: statusline dual-reads .oma then legacy .ohmyagents"
        );
        assert!(
            ps1.contains("build.zig")
                && ps1.contains("go.mod")
                && ps1.contains("CMakeLists.txt")
                && ps1.contains("meson.build"),
            "D11 projKind must probe zig / go / cpp markers"
        );
        let cargo = ps1.find("Cargo.toml").expect("rust probe");
        let zig = ps1.find("build.zig").expect("zig probe");
        assert!(
            cargo < zig,
            "D11 first-match: rust/node/python stay ahead of zig/go/cpp"
        );
        assert!(
            ps1.contains("'zig' = '\u{e6a9} '")
                && ps1.contains("'go' = '\u{e627} '")
                && ps1.contains("'cpp' = '\u{e646} '"),
            "D11 icons baked into the default icon map: seti-zig E6A9, seti-go E627, seti-cpp E646 (CaskaydiaCove and 0xProto cmap 2026-09-07)"
        );
        assert!(
            ps1.contains("if ($nerd) { [string][char]0x2718 } else { 'x' }"),
            "deleted flag must not emit U+2718 on the Grok ASCII path"
        );
    }

    #[test]
    fn assemble_keeps_default_segment_order() {
        // 期望值来自段块的注释标记（源内容，独立于拼装逻辑）。
        let ps1 = default_statusline_ps1();
        let mut last = 0usize;
        for marker in [
            "# ── Shell 段",
            "# ── 目录：",
            "# ── oma 段",
            "# ── 模型（",
            "# ── 上下文：",
            "# ── 会话累计：",
            "# ── Git：",
            "if ($pkgVer) {",
            "# ── Python 工具链",
            "# ── Rust 工具链",
            "# ── Node/TS 工具链",
            "# ── Zig 工具链",
            "# ── Go 工具链",
            "# ── C/C++ 工具链",
        ] {
            let at = ps1
                .find(marker)
                .unwrap_or_else(|| panic!("missing {marker}"));
            assert!(at > last, "{marker} out of order at {at} (prev {last})");
            last = at;
        }
    }

    #[test]
    fn assemble_reorders_and_drops_segments() {
        let ps1 = assemble_statusline_ps1(&["git", "oma"], &StatuslineConfig::default()).unwrap();
        let g = ps1.find("# ── Git：").unwrap();
        let o = ps1.find("# ── oma 段").unwrap();
        assert!(o > g, "git must render before oma in this order");
        assert!(!ps1.contains("# ── Shell 段"), "shell dropped");
        assert!(!ps1.contains("# ── Python 工具链"), "python dropped");
    }

    #[test]
    fn assemble_gates_common_and_probe_on_consumers() {
        let bare = assemble_statusline_ps1(&["model"], &StatuslineConfig::default()).unwrap();
        assert!(
            !bare.contains("rev-parse"),
            "COMMON skipped without dir/oma consumers"
        );
        assert!(
            !bare.contains("Cargo.toml"),
            "PROBE skipped without package/toolchain consumers"
        );
        let probe_only =
            assemble_statusline_ps1(&["package"], &StatuslineConfig::default()).unwrap();
        assert!(probe_only.contains("Cargo.toml"), "PROBE in for package");
        assert!(probe_only.contains("if ($pkgVer) {"));
        let common_only = assemble_statusline_ps1(&["oma"], &StatuslineConfig::default()).unwrap();
        assert!(common_only.contains("rev-parse"), "COMMON in for oma");
        assert!(!common_only.contains("Cargo.toml"));
    }

    #[test]
    fn dies_assemble_rejects_unknown_segment() {
        let err =
            assemble_statusline_ps1(&["model", "nope"], &StatuslineConfig::default()).unwrap_err();
        assert!(err.contains("unknown statusline segment"), "{err}");
    }

    #[test]
    fn dies_assemble_rejects_duplicate_segment() {
        let err =
            assemble_statusline_ps1(&["git", "git"], &StatuslineConfig::default()).unwrap_err();
        assert!(err.contains("duplicate statusline segment"), "{err}");
    }

    #[test]
    fn grok_cmd_wrapper_invokes_ps1_with_grok_agent() {
        // Oracle: grok-build command.rs spawns the configured string as a
        // program path; the wrapper bakes the agent name so the command
        // value can stay a single path (M048).
        assert!(STATUSLINE_GROK_CMD.contains("@echo off"));
        assert!(STATUSLINE_GROK_CMD.contains("oma-statusline.ps1"));
        assert!(STATUSLINE_GROK_CMD.contains(" grok"));
        assert!(
            !STATUSLINE_GROK_CMD.contains("claude") && !STATUSLINE_GROK_CMD.contains("kimi"),
            "wrapper is Grok-only"
        );
    }

    #[test]
    fn grok_windows_command_is_bare_cmd_path() {
        // Oracle: grok-build `Command::new(entire_string)`; a shell line with
        // quotes is ERROR_INVALID_NAME 123, which is not NotFound, so the
        // shell fallback never runs (M048).
        let cmd = grok_command_line("C:/Users/ray/.ohmyagents/statusline/oma-statusline.ps1");
        #[cfg(windows)]
        {
            assert_eq!(
                cmd,
                "C:/Users/ray/.ohmyagents/statusline/oma-statusline-grok.cmd"
            );
            assert!(
                !cmd.contains('"'),
                "quotes in the program name are 123: {cmd}"
            );
            assert!(
                !cmd.contains("pwsh"),
                "args after the path are not passed: {cmd}"
            );
        }
        #[cfg(not(windows))]
        {
            assert_eq!(
                cmd,
                "pwsh -NoProfile -File \"C:/Users/ray/.ohmyagents/statusline/oma-statusline.ps1\" grok"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn dies_windows_pwsh_shell_line_is_invalid_filename() {
        // Independent oracle: Win32 ERROR_INVALID_NAME = 123. grok-build
        // command.rs only shells out on NotFound, so this error is painted.
        let cmd = r#"pwsh -NoProfile -File "C:/Users/ray/.ohmyagents/statusline/oma-statusline.ps1" grok"#;
        let err = std::process::Command::new(cmd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect_err("shell line must not be a valid program name");
        assert_eq!(err.raw_os_error(), Some(123), "{err}");
        assert_ne!(err.kind(), std::io::ErrorKind::NotFound, "{err:?}");
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
        let grok_cmd = sl.get("command").unwrap().as_str().unwrap();
        assert_eq!(grok_cmd, grok_command_line("C:/x/oma-statusline.ps1"));
        #[cfg(windows)]
        assert_eq!(grok_cmd, "C:/x/oma-statusline-grok.cmd");
        #[cfg(not(windows))]
        assert!(grok_cmd.ends_with("\" grok"));
    }

    #[test]
    fn codex_tui_section_is_builtin_ids_not_command_argv() {
        let tui = render_codex_tui_section(CODEX_STATUS_LINE_ITEMS);
        assert!(tui.contains("run-state"));
        assert!(tui.contains("git-branch"));
        assert!(tui.contains("status_line_use_colors = true"));
        assert!(
            !tui.contains("pwsh") && !tui.contains("oma-statusline"),
            "Codex silently skips unknown IDs; command argv empties the bar: {tui}"
        );
        let last = tui
            .lines()
            .find(|l| l.trim_start().starts_with('"') && l.contains("branch-changes"))
            .unwrap();
        assert!(
            !last.trim().ends_with(','),
            "TOML 1.0 rejects trailing commas: {last}"
        );
    }

    #[test]
    fn strip_tui_section_keeps_other_tables() {
        let text = "model = \"gpt\"\n[tui]\nstatus_line = [\"old\"]\n[sandbox]\nmode = \"rw\"\n";
        let out = strip_tui_section(text);
        assert!(out.contains("model = \"gpt\""));
        assert!(out.contains("[sandbox]"));
        assert!(!out.contains("\"old\""));
        assert!(!out.contains("[tui]"));
    }
}
