//! `oma agents verify`（D17）：四家 agent 的 hook 与状态栏全平台无头验收。
//! 两层判据（S033 源码实证底座）：
//! - 状态栏：脚本本体直跑（mock 空 JSON 喂 stdin，断言 stdout 首行
//!   `agent:state` 机读标记，S025）；codex 无外部命令面（M045），改为断言
//!   `~/.codex/config.toml` 的 `[tui] status_line` 含内置项 ID。
//! - hook：临时目录起无头会话（`git init` 是 oma hook 回退写盘的硬前置，
//!   hook.rs 的 fallback 段向上 8 层找 .git），判据只押 SessionStart /
//!   UserPromptSubmit 这类先于模型调用的事件——模型应答失败（无 token、
//!   网络错）不影响判定，state 文件落盘即 ok。kimi 的 SessionEnd 在 print
//!   模式不触发（S033），不纳入判据。

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::agents;

/// verify 认识的四家（与 agents::SPECS 同集）。
pub const SUPPORTED: &[&str] = &["claude", "codex", "grok", "kimi"];
/// 单家无头会话缺省最长秒数。
pub const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// 单层验收结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerVerdict {
    Ok,
    /// codex 状态栏：内置项 ID 面，无外部命令可跑（M045）。
    Builtin,
    Skip(String),
    Fail { reason: String, hint: Option<String> },
}

/// 单家验收结果（两层各一条；skip 时两层不跑）。
#[derive(Debug)]
pub struct AgentOutcome {
    pub agent: String,
    /// 整机 skip 原因（not-installed）；Some 时两层不跑、不算失败。
    pub skip: Option<String>,
    pub statusline: LayerVerdict,
    pub hook: LayerVerdict,
    /// hook 层观测到的四态（ok 时必有）。
    pub hook_state: Option<String>,
}

impl AgentOutcome {
    fn skipped(agent: &str, reason: &str) -> Self {
        AgentOutcome {
            agent: agent.to_string(),
            skip: Some(reason.to_string()),
            statusline: LayerVerdict::Skip(reason.to_string()),
            hook: LayerVerdict::Skip(reason.to_string()),
            hook_state: None,
        }
    }
}

fn wanted_names(names: &[String]) -> Result<Vec<String>, String> {
    if names.is_empty() {
        return Ok(SUPPORTED.iter().map(|s| s.to_string()).collect());
    }
    let unknown: Vec<&str> = names
        .iter()
        .filter(|n| !SUPPORTED.contains(&n.as_str()))
        .map(String::as_str)
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "verify supports claude/codex/grok/kimi only: {}",
            unknown.join(",")
        ));
    }
    Ok(names.to_vec())
}

/// 验收主流程：逐家两层，skip（未装）不算失败。
pub fn run(names: &[String], timeout_secs: u64) -> Result<Vec<AgentOutcome>, String> {
    let wanted = wanted_names(names)?;
    let reports = agents::detect();
    let home = crate::install::oma_home()?;
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("oma"));
    let mut outcomes = Vec::new();
    for name in wanted {
        let hit = reports
            .iter()
            .find(|r| r.agent == name)
            .and_then(|r| r.hit.as_ref());
        let Some(hit) = hit else {
            outcomes.push(AgentOutcome::skipped(&name, "not-installed"));
            continue;
        };
        let statusline = verify_statusline(&name, &home);
        let (hook, hook_state) = verify_hook(&name, &hit.path, &exe, timeout_secs);
        outcomes.push(AgentOutcome {
            agent: name,
            skip: None,
            statusline,
            hook,
            hook_state,
        });
    }
    Ok(outcomes)
}

/// 任一非跳过项失败即 true（进程退出 1 的判据）。
pub fn any_fail(outcomes: &[AgentOutcome]) -> bool {
    outcomes.iter().any(|o| {
        matches!(o.statusline, LayerVerdict::Fail { .. })
            || matches!(o.hook, LayerVerdict::Fail { .. })
    })
}

/// kv marker 行渲染（风格对齐 statusline/install 等现有命令）。
pub fn render(outcomes: &[AgentOutcome]) -> Vec<String> {
    let mut lines = Vec::new();
    for o in outcomes {
        if let Some(reason) = &o.skip {
            lines.push(format!("verify.{}=skip reason={reason}", o.agent));
            continue;
        }
        let failed = matches!(o.statusline, LayerVerdict::Fail { .. })
            || matches!(o.hook, LayerVerdict::Fail { .. });
        lines.push(format!(
            "verify.{}={}",
            o.agent,
            if failed { "fail" } else { "ok" }
        ));
        render_layer(&mut lines, &o.agent, "statusline", &o.statusline);
        render_layer(&mut lines, &o.agent, "hook", &o.hook);
        if let Some(state) = &o.hook_state {
            lines.push(format!("verify.{}.hook.state={state}", o.agent));
        }
    }
    lines.push(format!("verify.ok={}", !any_fail(outcomes)));
    lines
}

fn render_layer(lines: &mut Vec<String>, agent: &str, layer: &str, verdict: &LayerVerdict) {
    match verdict {
        LayerVerdict::Ok => lines.push(format!("verify.{agent}.{layer}=ok")),
        LayerVerdict::Builtin => lines.push(format!("verify.{agent}.{layer}=builtin")),
        LayerVerdict::Skip(reason) => {
            lines.push(format!("verify.{agent}.{layer}=skip reason={reason}"))
        }
        LayerVerdict::Fail { reason, hint } => {
            lines.push(format!("verify.{agent}.{layer}=fail reason={reason}"));
            if let Some(h) = hint {
                lines.push(format!("verify.{agent}.{layer}.hint={h}"));
            }
        }
    }
}

// ── 层 1：状态栏 ──

fn verify_statusline(agent: &str, home: &Path) -> LayerVerdict {
    if agent == "codex" {
        return codex_statusline_builtin();
    }
    let script = match crate::statusline::deploy_script(home) {
        Ok(p) => p,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("deploy-script: {e}"),
                hint: None,
            }
        }
    };
    if !crate::statusline::pwsh_on_path() {
        return LayerVerdict::Fail {
            reason: "pwsh-not-on-path".into(),
            hint: Some("pwsh 是状态栏运行时（全平台同）；装 PowerShell 7 后重跑".into()),
        };
    }
    let mut child = match Command::new("pwsh")
        .arg("-NoProfile")
        .arg("-File")
        .arg(&script)
        .arg(agent)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("pwsh-spawn: {e}"),
                hint: None,
            }
        }
    };
    // 脚本对空/无 stdin 有容错；喂 `{}` 走 claude 形态的 JSON 解析路径。
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(b"{}");
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            return LayerVerdict::Fail {
                reason: format!("pwsh-wait: {e}"),
                hint: None,
            }
        }
    };
    if !out.status.success() {
        return LayerVerdict::Fail {
            reason: format!("script-exit-{}", out.status.code().unwrap_or(-1)),
            hint: None,
        };
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if statusline_marker_ok(agent, &stdout) {
        LayerVerdict::Ok
    } else {
        LayerVerdict::Fail {
            reason: "marker-missing".into(),
            hint: Some(format!(
                "状态栏脚本 stdout 首行应含机读标记 {agent}:<state>（S025）"
            )),
        }
    }
}

/// 纯函数：stdout 首行含 `<agent>:` 机读标记（脚本输出形如 agent:state）。
pub fn statusline_marker_ok(agent: &str, stdout: &str) -> bool {
    let marker = format!("{agent}:");
    stdout
        .lines()
        .next()
        .is_some_and(|l| l.contains(&marker))
}

/// codex：`~/.codex/config.toml` 的 `[tui] status_line` 含内置项 ID 即过。
fn codex_statusline_builtin() -> LayerVerdict {
    let config = match dirs::home_dir() {
        Some(h) => h.join(".codex").join("config.toml"),
        None => {
            return LayerVerdict::Fail {
                reason: "no-home".into(),
                hint: None,
            }
        }
    };
    match fs::read_to_string(&config) {
        Ok(text) if codex_builtin_statusline_ok(&text) => LayerVerdict::Builtin,
        Ok(_) => LayerVerdict::Fail {
            reason: "status_line-missing-builtin-ids".into(),
            hint: Some("跑 `oma agents statusline codex` 写入 [tui] 内置项 ID".into()),
        },
        Err(_) => LayerVerdict::Fail {
            reason: format!("no-config: {}", config.display()),
            hint: Some("跑 `oma agents statusline codex` 写入 [tui] 内置项 ID".into()),
        },
    }
}

/// 纯函数：config.toml 的 `[tui]` 段 `status_line`（单行或多行数组）含
/// run-state 锚（oma 部署的内置项清单恒含，M045 无外部命令面）。
pub fn codex_builtin_statusline_ok(text: &str) -> bool {
    let mut in_tui = false;
    let mut in_list = false;
    for ln in text.lines() {
        let t = ln.trim();
        if t.starts_with('[') && !in_list {
            in_tui = t == "[tui]";
            continue;
        }
        if !in_tui {
            continue;
        }
        if t.starts_with("status_line") && t.contains('[') {
            in_list = true;
        }
        if in_list {
            if t.contains("run-state") {
                return true;
            }
            if t.contains(']') {
                in_list = false;
            }
        }
    }
    false
}

// ── 层 2：hook 无头落盘 ──

fn verify_hook(
    agent: &str,
    bin: &Path,
    exe: &Path,
    timeout_secs: u64,
) -> (LayerVerdict, Option<String>) {
    let tmp = std::env::temp_dir().join(format!(
        "oma-verify-{agent}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let result = verify_hook_in(agent, bin, exe, timeout_secs, &tmp);
    let _ = fs::remove_dir_all(&tmp);
    result
}

fn verify_hook_in(
    agent: &str,
    bin: &Path,
    exe: &Path,
    timeout_secs: u64,
    tmp: &Path,
) -> (LayerVerdict, Option<String>) {
    let fail = |reason: String| (LayerVerdict::Fail {
        reason,
        hint: hook_hint(agent),
    }, None);
    if let Err(e) = fs::create_dir_all(tmp) {
        return fail(format!("tmpdir: {e}"));
    }
    if let Err(e) = git_init(tmp) {
        return fail(format!("git-init: {e}"));
    }
    if let Err(e) = crate::deploy::apply_project_hooks(tmp) {
        return fail(format!("deploy-hooks: {e}"));
    }
    if let Err(e) = crate::yolo::apply_project_yolo(tmp) {
        return fail(format!("deploy-yolo: {e}"));
    }
    // kimi 无 oma 项目级 hook 注册面（deploy.rs 只落 skill 目录）；现行版
    // 项目 .kimi-code/config.toml 的 [[hooks]] print 链路同载（S033），
    // verify 在临时目录补这一份注册。
    if agent == "kimi" {
        if let Err(e) = deploy_kimi_hooks(tmp, exe) {
            return fail(format!("deploy-kimi-hooks: {e}"));
        }
    }
    // grok 项目源 hooks 受 folder trust 门禁（S033：xai-grok-config loader）：
    // 验收临时目录先种子 ~/.grok/trusted_folders.toml，Drop 时摘除（已信任不动）。
    let _grok_guard = if agent == "grok" {
        match grok_trust_add(tmp) {
            Ok(g) => Some(g),
            Err(e) => return fail(format!("grok-trust: {e}")),
        }
    } else {
        None
    };
    let argv = match headless_argv(agent, bin, tmp) {
        Some(a) => a,
        None => return fail(format!("unknown agent {agent}")),
    };
    let mut child = match Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(tmp)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return fail(format!("spawn: {e}")),
    };
    // 超时杀进程不算失败本身：判据只看 state 文件落盘。
    let _exited = wait_with_timeout(&mut child, Duration::from_secs(timeout_secs));
    let state_file = tmp.join(".oma").join("state").join(format!("{agent}.json"));
    match fs::read_to_string(&state_file) {
        Ok(text) => match parse_state(&text) {
            Ok(state) => (LayerVerdict::Ok, Some(state)),
            Err(e) => fail(format!("bad-state: {e}")),
        },
        Err(_) => fail("no-state-file".into()),
    }
}

/// 环境原因失败时的可行动 hint（不只打 fail）。
fn hook_hint(agent: &str) -> Option<String> {
    Some(match agent {
        "codex" => "codex 信任闸 exec 下静默跳过无提示（S033）：确认 \
            --dangerously-bypass-hook-trust 已带；项目目录信任与 \
            [hooks.state] trusted_hash 由 oma init 预种"
            .into(),
        "grok" => "grok 项目源 hooks 受 folder trust 门禁（S033）：verify 已自动 \
            种子并摘除 ~/.grok/trusted_folders.toml 条目；仍失败检查该文件与 grok 版本"
            .into(),
        "kimi" => "kimi 项目级 [[hooks]] 需现行版支持；SessionEnd 在 print \
            不触发（S033），判据只押 SessionStart/UserPromptSubmit"
            .into(),
        _ => "确认 oma 在 PATH（hook 注册为裸命令 oma hook --agent <名>）或重跑 \
            oma init；判据只看 state 落盘，模型应答失败不影响"
            .into(),
    })
}

/// kimi 项目级 hook 注册：[[hooks]] 追加进 <tmp>/.kimi-code/config.toml
/// （yolo 已写 default_permission_mode，表共存）。命令钉当前 oma exe 绝对
/// 路径，不依赖 hook 执行环境的 PATH。
fn deploy_kimi_hooks(root: &Path, exe: &Path) -> Result<(), String> {
    let cfg = root.join(".kimi-code").join("config.toml");
    let mut toml = crate::yolo::read_toml(&cfg)?;
    let table = match &mut toml {
        toml::Value::Table(t) => t,
        _ => return Err("kimi config.toml is not a table".into()),
    };
    let command = format!(
        "\"{}\" hook --agent kimi",
        exe.display().to_string().replace('\\', "/")
    );
    let mut hooks = Vec::new();
    for event in ["SessionStart", "UserPromptSubmit"] {
        let mut entry = toml::map::Map::new();
        entry.insert("event".into(), toml::Value::String(event.into()));
        entry.insert("command".into(), toml::Value::String(command.clone()));
        entry.insert("timeout".into(), toml::Value::Integer(10));
        hooks.push(toml::Value::Table(entry));
    }
    table.insert("hooks".into(), toml::Value::Array(hooks));
    crate::yolo::toml_write(&cfg, &toml)
}

/// grok folder trust 临时条目（S033 项目源 hooks 门禁）。Drop 时摘除：
/// 文件原本不在则整文件删掉；原本在则只摘我们加的 key。
struct GrokTrustGuard {
    file: PathBuf,
    key: String,
    file_existed: bool,
}

impl Drop for GrokTrustGuard {
    fn drop(&mut self) {
        if self.file_existed {
            if let Ok(mut toml) = crate::yolo::read_toml(&self.file) {
                grok_trust_remove(&mut toml, &self.key);
                let _ = crate::yolo::toml_write(&self.file, &toml);
            }
        } else {
            let _ = fs::remove_file(&self.file);
        }
    }
}

/// 纯函数：[folders.<key>] 落 trusted=true（对齐 yolo::apply_pretrust 的
/// grok 段）。返回是否为新增（已信任则不动，调用方也不该摘除）。
fn grok_trust_insert(toml: &mut toml::Value, key: &str) -> Result<bool, String> {
    let table = match toml {
        toml::Value::Table(t) => t,
        _ => return Err("grok trusted_folders.toml is not a table".into()),
    };
    let folders = table
        .entry("folders".to_string())
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let folders = match folders {
        toml::Value::Table(t) => t,
        _ => return Err("grok [folders] is not a table".into()),
    };
    let already = folders
        .get(key)
        .and_then(|v| v.get("trusted"))
        .and_then(|v| v.as_bool())
        == Some(true);
    if already {
        return Ok(false);
    }
    let mut entry = toml::map::Map::new();
    entry.insert("trusted".into(), toml::Value::Boolean(true));
    let decided_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    entry.insert("decided_at".into(), toml::Value::Integer(decided_at));
    folders.insert(key.to_string(), toml::Value::Table(entry));
    Ok(true)
}

/// 纯函数：摘除 [folders.<key>]；folders 空了连表一起摘。
fn grok_trust_remove(toml: &mut toml::Value, key: &str) {
    let toml::Value::Table(table) = toml else {
        return;
    };
    let empty = match table.get_mut("folders") {
        Some(toml::Value::Table(folders)) => {
            folders.remove(key);
            folders.is_empty()
        }
        _ => false,
    };
    if empty {
        table.remove("folders");
    }
}

fn grok_trust_add(root: &Path) -> Result<Option<GrokTrustGuard>, String> {
    let home = dirs::home_dir().ok_or("no home")?;
    let file = home.join(".grok").join("trusted_folders.toml");
    let file_existed = file.exists();
    let key = crate::pathutil::native_slash(root);
    let mut toml = crate::yolo::read_toml(&file)?;
    if !grok_trust_insert(&mut toml, &key)? {
        return Ok(None); // 已信任：不守卫、不摘除
    }
    crate::yolo::toml_write(&file, &toml)?;
    Ok(Some(GrokTrustGuard {
        file,
        key,
        file_existed,
    }))
}

/// hook 回退写盘的硬前置：cwd 向上 8 层内要有 .git（hook.rs）。
fn git_init(dir: &Path) -> Result<(), String> {
    match Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("exit {}", s.code().unwrap_or(-1))),
        // git 不在 PATH：回退只查 .git 存在性，手工落目录等价。
        Err(_) => fs::create_dir_all(dir.join(".git"))
            .map_err(|e| format!("{}: {e}", dir.display())),
    }
}

/// 各家的无头命令行（S033 取证）。codex 的 bypass 旗标必须：否则 hook
/// 信任闸静默跳过无任何提示（假绿）。
pub fn headless_argv(agent: &str, bin: &Path, project: &Path) -> Option<Vec<String>> {
    let bin = bin.display().to_string();
    let prompt = "Reply with OK".to_string();
    match agent {
        "claude" => Some(vec![
            bin,
            "-p".into(),
            prompt,
            "--dangerously-skip-permissions".into(),
        ]),
        "codex" => Some(vec![
            bin,
            "exec".into(),
            "--dangerously-bypass-hook-trust".into(),
            prompt,
        ]),
        "grok" => Some(vec![
            bin,
            "-p".into(),
            prompt,
            "--always-approve".into(),
            "--cwd".into(),
            project.display().to_string(),
        ]),
        "kimi" => Some(vec![bin, "-p".into(), prompt]),
        _ => None,
    }
}

/// 等进程退出或超时；超时杀进程并等回收，返回是否自行退出。
fn wait_with_timeout(child: &mut Child, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(200))
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
            Err(_) => return true,
        }
    }
}

/// 纯函数：state 文件 JSON 的 state 字段 ∈ 四态才作数。
pub fn parse_state(text: &str) -> Result<String, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("state json: {e}"))?;
    let state = v
        .get("state")
        .and_then(|s| s.as_str())
        .ok_or_else(|| "state field missing".to_string())?;
    match state {
        "idle" | "working" | "blocked" | "unknown" => Ok(state.to_string()),
        other => Err(format!("unexpected state {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headless_argv_shapes_per_agent() {
        let bin = Path::new("C:/bin/agent.exe");
        let project = Path::new("C:/tmp/proj");
        let codex = headless_argv("codex", bin, project).unwrap();
        assert_eq!(codex[1], "exec");
        // S033：bypass 旗标必须，否则信任闸静默跳过（假绿）。
        assert!(codex.contains(&"--dangerously-bypass-hook-trust".to_string()));
        let grok = headless_argv("grok", bin, project).unwrap();
        assert!(grok.contains(&"--always-approve".to_string()));
        let cwd_at = grok.iter().position(|a| a == "--cwd").unwrap();
        assert_eq!(grok[cwd_at + 1], project.display().to_string());
        let claude = headless_argv("claude", bin, project).unwrap();
        assert!(claude.contains(&"--dangerously-skip-permissions".to_string()));
        let kimi = headless_argv("kimi", bin, project).unwrap();
        assert_eq!(kimi.len(), 3, "kimi -p 硬编码 auto 批准，无额外线标");
        assert!(headless_argv("bogus", bin, project).is_none());
    }

    #[test]
    fn parse_state_accepts_the_four_states() {
        for s in ["idle", "working", "blocked", "unknown"] {
            let text = format!("{{\"state\": \"{s}\", \"event\": \"sessionstart\"}}");
            assert_eq!(parse_state(&text).unwrap(), s);
        }
    }

    #[test]
    fn dies_parse_state_rejects_foreign_state_and_garbage() {
        // 期望来自 hook.rs 的四态封闭集（独立来源），非被测逻辑镜像。
        assert!(parse_state("{\"state\": \"running\"}").is_err());
        assert!(parse_state("{\"event\": \"stop\"}").is_err());
        assert!(parse_state("not json").is_err());
    }

    #[test]
    fn statusline_marker_must_be_on_first_line_with_agent_prefix() {
        assert!(statusline_marker_ok("claude", "claude:idle | ~/proj\n"));
        // ANSI 色码包裹下标记仍在行内（脚本 Seg 输出）。
        assert!(statusline_marker_ok(
            "grok",
            "\u{1b}[38;5;245mgrok:unknown\u{1b}[0m | x"
        ));
        assert!(!statusline_marker_ok("kimi", "claude:idle"));
        assert!(!statusline_marker_ok("kimi", ""));
        // 标记必须在首行：第二行出现不算。
        assert!(!statusline_marker_ok("kimi", "garbage\nkimi:idle"));
    }

    #[test]
    fn codex_builtin_accepts_multiline_and_singleline_arrays() {
        let multiline = "model = \"gpt\"\n\n[tui]\nstatus_line = [\n  \"run-state\",\n  \"git-branch\"\n]\n";
        assert!(codex_builtin_statusline_ok(multiline));
        let singleline = "[tui]\nstatus_line = [\"run-state\", \"current-dir\"]\n";
        assert!(codex_builtin_statusline_ok(singleline));
        // 段外出现的 run-state 不算数。
        let wrong_section = "[hooks]\nx = \"run-state\"\n[tui]\nother = 1\n";
        assert!(!codex_builtin_statusline_ok(wrong_section));
        // status_line_use_colors 同前缀不触发。
        let colors_only = "[tui]\nstatus_line_use_colors = true\n";
        assert!(!codex_builtin_statusline_ok(colors_only));
    }

    #[test]
    fn skip_outcomes_render_skip_and_do_not_fail() {
        let outcomes = SUPPORTED
            .iter()
            .map(|a| AgentOutcome::skipped(a, "not-installed"))
            .collect::<Vec<_>>();
        assert!(!any_fail(&outcomes), "skip 不算失败（退出码契约）");
        let lines = render(&outcomes);
        assert!(lines.contains(&"verify.claude=skip reason=not-installed".to_string()));
        assert_eq!(lines.last().unwrap(), "verify.ok=true");
    }

    #[test]
    fn any_layer_fail_marks_agent_and_run_failed() {
        let mut o = AgentOutcome::skipped("codex", "not-installed");
        o.skip = None;
        o.statusline = LayerVerdict::Builtin;
        o.hook = LayerVerdict::Fail {
            reason: "no-state-file".into(),
            hint: Some("hint-text".into()),
        };
        let lines = render(&[o]);
        assert!(lines.contains(&"verify.codex=fail".to_string()));
        assert!(lines.contains(&"verify.codex.statusline=builtin".to_string()));
        assert!(lines.contains(&"verify.codex.hook=fail reason=no-state-file".to_string()));
        assert!(lines.contains(&"verify.codex.hook.hint=hint-text".to_string()));
        assert_eq!(lines.last().unwrap(), "verify.ok=false");
    }

    #[test]
    fn grok_trust_insert_remove_roundtrip() {
        // 期望对齐 yolo::apply_pretrust 的 grok 段：[folders.<key>] trusted=true。
        let mut toml = toml::Value::Table(toml::map::Map::new());
        assert!(grok_trust_insert(&mut toml, "D:\\tmp\\x").unwrap());
        let folders = toml.get("folders").unwrap().as_table().unwrap();
        assert_eq!(
            folders.get("D:\\tmp\\x").unwrap().get("trusted"),
            Some(&toml::Value::Boolean(true))
        );
        // 已信任再插返回 false（调用方不摘除用户既有信任）。
        assert!(!grok_trust_insert(&mut toml, "D:\\tmp\\x").unwrap());
        grok_trust_remove(&mut toml, "D:\\tmp\\x");
        assert!(toml.get("folders").is_none(), "空 folders 表连表摘除");
        // 既有别的 key 时只摘我们的。
        let mut t2 = toml::Value::Table(toml::map::Map::new());
        grok_trust_insert(&mut t2, "D:\\keep").unwrap();
        grok_trust_insert(&mut t2, "D:\\tmp\\x").unwrap();
        grok_trust_remove(&mut t2, "D:\\tmp\\x");
        let folders = t2.get("folders").unwrap().as_table().unwrap();
        assert!(folders.get("D:\\keep").is_some());
        assert!(folders.get("D:\\tmp\\x").is_none());
    }

    #[test]
    fn ok_hook_renders_observed_state() {
        let o = AgentOutcome {
            agent: "kimi".into(),
            skip: None,
            statusline: LayerVerdict::Ok,
            hook: LayerVerdict::Ok,
            hook_state: Some("idle".into()),
        };
        let lines = render(&[o]);
        assert!(lines.contains(&"verify.kimi.hook.state=idle".to_string()));
        assert_eq!(lines.last().unwrap(), "verify.ok=true");
    }
}
