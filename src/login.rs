//! `oma agents login [grok|kimi]`：设备码登录引导（S026 落地项）。
//!
//! 两家登录流都是「URL 加 user_code 落 stderr」的可复制形态，纯 eprintln /
//! process.stderr.write、无 TTY 依赖（源码实证：grok-build auth/device_code.rs、
//! kimi-code cli/sub/login-flow.ts）——子进程捕获转发给用户，等浏览器侧完成
//! 后扫成功标记，最终以 doctor 的登录态判据（落盘文件）确认，不单信标记。

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::agents;

pub struct LoginPrompt {
    pub url: String,
    pub code: String,
}

#[derive(Debug)]
pub struct LoginOutcome {
    pub ok: bool,
    pub detail: String,
}

/// grok 输出契约（auth/device_code.rs 实证）：提示行（open this URL / Then
/// enter this code / Confirm this code）与值之间各有一个 `eprintln!()` 空行，
/// 取提示行后第一个非空行分别是 URL 与 user_code。
fn grok_extract_prompt(lines: &[String]) -> Option<LoginPrompt> {
    let next_nonempty = |from: usize| -> Option<&str> {
        lines
            .iter()
            .skip(from)
            .map(|s| s.trim())
            .find(|s| !s.is_empty())
    };
    let mut url = None;
    let mut code = None;
    for (i, ln) in lines.iter().enumerate() {
        if ln.contains("open this URL in your browser") {
            if let Some(v) = next_nonempty(i + 1) {
                if v.starts_with("http") {
                    url = Some(v.to_string());
                }
            }
        }
        if ln.contains("Then enter this code") || ln.contains("Confirm this code in your browser") {
            if let Some(v) = next_nonempty(i + 1) {
                code = Some(v.to_string());
            }
        }
    }
    Some(LoginPrompt {
        url: url?,
        code: code?,
    })
}

/// kimi 输出契约（cli/sub/login-flow.ts 实证）：URL 与 user_code 同行尾缀
/// （`... device login: <url>` / `... enter code: <userCode>`）。
fn kimi_extract_prompt(lines: &[String]) -> Option<LoginPrompt> {
    let mut url = None;
    let mut code = None;
    for ln in lines {
        if let Some((_, rest)) = ln.split_once("device login:") {
            let t = rest.trim();
            if t.starts_with("http") {
                url = Some(t.to_string());
            }
        }
        if let Some((_, rest)) = ln.split_once("enter code:") {
            let t = rest.trim();
            if !t.is_empty() {
                code = Some(t.to_string());
            }
        }
    }
    Some(LoginPrompt {
        url: url?,
        code: code?,
    })
}

fn extract_prompt(agent: &str, lines: &[String]) -> Option<LoginPrompt> {
    match agent {
        "grok" => grok_extract_prompt(lines),
        "kimi" => kimi_extract_prompt(lines),
        _ => None,
    }
}

/// 成功标记：grok `✓ Signed in`（含 `✓ Signed in as <email>`，flow.rs 实证）；
/// kimi `Logged in to <provider>.`（login-flow.ts 实证）。
fn is_success(agent: &str, line: &str) -> bool {
    match agent {
        "grok" => line.contains("✓ Signed in"),
        "kimi" => line.starts_with("Logged in to"),
        _ => false,
    }
}

/// 失败行（供 detail 转述）：kimi 明确打 `Login cancelled` / `Login failed`；
/// grok 走 anyhow 错误面（`Error: ...`），以非零退出码为准。
fn is_failure_line(agent: &str, line: &str) -> bool {
    match agent {
        "kimi" => line.starts_with("Login cancelled") || line.starts_with("Login failed"),
        "grok" => line.starts_with("Error:"),
        _ => false,
    }
}

/// 排空一个子进程流防背压，逐行送回主循环解析。**不向用户转发**原始
/// stderr：设备码流的价值在跨机操作（URL 加 code 拿到任何机器完成），
/// 子进程自己的输出（含 grok 试开浏览器的噪音）只做解析素材，失败时才
/// 取尾部做诊断。
fn spawn_reader<R: std::io::Read + Send + 'static>(stream: R) -> Receiver<String> {
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    rx
}

/// 引导一路登录。`timeout_secs` 是等浏览器侧完成的最长秒数，0 不限时。
pub fn run(agent: &str, timeout_secs: u64) -> Result<LoginOutcome, String> {
    if !matches!(agent, "grok" | "kimi") {
        return Err(format!(
            "login supports grok/kimi only (claude/codex 走各自原生登录): {agent}"
        ));
    }
    let hit = agents::find(agent).ok_or_else(|| {
        format!("agent {agent} not found; run `oma agents install {agent}` first")
    })?;
    let mut cmd = Command::new(&hit.path);
    cmd.arg("login");
    if agent == "grok" {
        // 设备码流天生无头友好（S026）；缺省 loopback 只适合本机有浏览器。
        cmd.arg("--device-code");
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    println!("login.agent={agent}");
    println!("login.binary={}", hit.path.display());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn {}: {e}", hit.path.display()))?;

    let rx_err = spawn_reader(child.stderr.take().expect("stderr piped"));
    let rx_out = spawn_reader(child.stdout.take().expect("stdout piped"));

    let deadline = if timeout_secs == 0 {
        None
    } else {
        Some(Instant::now() + Duration::from_secs(timeout_secs))
    };
    let mut seen: Vec<String> = Vec::new();
    let mut prompt_sent = false;
    let mut success_seen = false;
    let mut failure_line: Option<String> = None;
    let mut tail: Vec<String> = Vec::new();
    let timed_out = loop {
        let recv = |rx: &Receiver<String>| -> Result<String, RecvTimeoutError> {
            match deadline {
                Some(d) => rx.recv_timeout(d.saturating_duration_since(Instant::now())),
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            }
        };
        // 优先 stderr（进度流），stdout 只是陪排空。
        let got = match recv(&rx_err) {
            Ok(line) => Some(line),
            Err(RecvTimeoutError::Timeout) => break true,
            Err(RecvTimeoutError::Disconnected) => match recv(&rx_out) {
                Ok(line) => Some(line),
                Err(RecvTimeoutError::Timeout) => break true,
                Err(RecvTimeoutError::Disconnected) => break false,
            },
        };
        let Some(line) = got else { continue };
        if is_success(agent, &line) {
            success_seen = true;
        }
        if failure_line.is_none() && is_failure_line(agent, &line) {
            failure_line = Some(line.clone());
        }
        if !line.trim().is_empty() {
            // 诊断尾部：留最后三行非空（失败时转述，平时丢弃）。
            if tail.len() == 3 {
                tail.remove(0);
            }
            tail.push(line.clone());
        }
        if !prompt_sent {
            seen.push(line);
            if let Some(p) = extract_prompt(agent, &seen) {
                println!("login.url={}", p.url);
                println!("login.code={}", p.code);
                println!("login.waiting=browser-completion");
                println!("login.hint=open the URL on any machine and confirm the code");
                prompt_sent = true;
            }
        }
    };

    if timed_out {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(LoginOutcome {
            ok: false,
            detail: format!(
                "timeout after {timeout_secs}s waiting for browser completion (killed)"
            ),
        });
    }
    let exit = child
        .wait()
        .map_err(|e| format!("wait {}: {e}", hit.path.display()))?;

    // 落盘文件是最终事实：成功标记只是过程信号。
    let state = crate::doctor::login_state(agent);
    if exit.success()
        && state
            .as_ref()
            .is_some_and(|(st, _)| *st == crate::doctor::Status::Ok)
    {
        return Ok(LoginOutcome {
            ok: true,
            detail: state
                .map(|(_, d)| d)
                .unwrap_or_else(|| "credentials verified".into()),
        });
    }
    let mut detail = format!("exit={:?}", exit.code());
    if let Some((st, d)) = state {
        detail.push_str(&format!(" login_state={}? {d}", st.as_str()));
    }
    if success_seen {
        detail.push_str(" (success marker seen but credentials not verified)");
    }
    let diag = failure_line.unwrap_or_else(|| tail.join(" / "));
    if !diag.is_empty() {
        detail.push_str(&format!(" last={diag}"));
    }
    Ok(LoginOutcome { ok: false, detail })
}

#[cfg(test)]
mod tests {
    use super::*;

    // 黄金样例来自源码取证（grok-build auth/device_code.rs 355-390、kimi-code
    // cli/sub/login-flow.ts 46-69），不是实现镜像。

    #[test]
    fn grok_prompt_from_golden_device_code_block() {
        let block: Vec<String> = [
            "",
            "To sign in, open this URL in your browser:",
            "",
            "  https://auth.x.ai/device?user_code=XW29-M4QK",
            "",
            "  (Could not open browser automatically — open the URL above manually.)",
            "",
            "Confirm this code in your browser:",
            "",
            "  XW29-M4QK",
            "",
            "Only continue with a code you requested. Don't share it with anyone.",
            "Waiting for authorization...",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = grok_extract_prompt(&block).expect("prompt");
        assert_eq!(p.url, "https://auth.x.ai/device?user_code=XW29-M4QK");
        assert_eq!(p.code, "XW29-M4QK");
        // 无 uri_complete 的分体形态：Then enter this code
        let split: Vec<String> = [
            "To sign in, open this URL in your browser:",
            "  https://auth.x.ai/activate",
            "Then enter this code:",
            "  ABCD-1234",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = grok_extract_prompt(&split).expect("prompt");
        assert_eq!(p.url, "https://auth.x.ai/activate");
        assert_eq!(p.code, "ABCD-1234");
        // 只有 URL 没有 code 时不误报
        let url_only: Vec<String> = ["To sign in, open this URL in your browser:", "  https://x/"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(grok_extract_prompt(&url_only).is_none());
    }

    #[test]
    fn kimi_prompt_from_golden_login_flow_lines() {
        let block: Vec<String> = [
            "Opening browser for Kimi device login: https://auth.kimi.com/device?code=XW29",
            "If the browser did not open, paste the URL above and enter code: XW29-M4QK",
            "Code expires in 600s.",
            "Waiting for authorization to complete…",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let p = kimi_extract_prompt(&block).expect("prompt");
        assert_eq!(p.url, "https://auth.kimi.com/device?code=XW29");
        assert_eq!(p.code, "XW29-M4QK");
    }

    #[test]
    fn success_and_failure_markers_match_source_strings() {
        assert!(is_success("grok", "✓ Signed in"));
        assert!(is_success("grok", "✓ Signed in as a@b.c"));
        assert!(!is_success("grok", "Waiting for authorization..."));
        assert!(is_success("kimi", "Logged in to Kimi."));
        assert!(!is_success(
            "kimi",
            "Waiting for authorization to complete…"
        ));
        assert!(is_failure_line("kimi", "Login cancelled."));
        assert!(is_failure_line("kimi", "Login failed: device flow expired"));
        assert!(!is_failure_line("kimi", "Logged in to Kimi."));
        assert!(is_failure_line("grok", "Error: device code expired"));
        assert!(!is_failure_line("grok", "Waiting for authorization..."));
    }

    #[test]
    fn unsupported_agent_rejected_before_any_spawn() {
        let err = run("claude", 1).expect_err("claude unsupported");
        assert!(err.contains("grok/kimi only"), "{err}");
    }

    #[test]
    fn extract_prompt_dispatches_by_agent() {
        let lines = vec!["Opening browser for Kimi device login: https://x/".to_string()];
        assert!(extract_prompt("kimi", &lines).is_none());
        assert!(extract_prompt("codex", &lines).is_none());
    }
}
