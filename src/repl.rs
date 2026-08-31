//! REPL（P0016，CLI 通道的交互面）：`oma` 裸调用进入。
//! 会话已存在则重连（不叠格）；默认起 HTTP 编排面打印 URL（`--no-web` 关，
//! `--open` 才尝试开浏览器，失败只警告——弹不出浏览器不是错误）。
//! 行循环：`all <文本>` 状态门分派、`<agent> <文本>` 单路发送、`status` 表格、
//! `web` 打印 URL、`quit` 只 detach（拆会话用 cleanup）。
//! stdin 阻塞读放独立线程喂 tokio mpsc——REPL 每个 await 都给 serve 任务让路。

use std::io::{BufRead, IsTerminal, Write};

use tokio::sync::mpsc;

use crate::api;
use crate::orch;

pub struct ReplArgs {
    pub agents: Option<Vec<String>>,
    pub stub: bool,
    pub no_web: bool,
    pub open: bool,
}

/// REPL 命令解析（纯函数，行为由单测锁定）。
#[derive(Debug, PartialEq, Eq)]
pub enum ReplCommand {
    /// `all <文本>`：状态门分派全会话。
    Run(String),
    /// `<agent> <文本>`：单路发送。
    Send(String, String),
    Status,
    Web,
    Quit,
}

pub fn parse_repl_line(line: &str) -> Option<ReplCommand> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (head, rest) = match line.split_once(' ') {
        Some((h, r)) => (h, r.trim()),
        None => (line, ""),
    };
    match head {
        "quit" | "exit" => Some(ReplCommand::Quit),
        "status" => Some(ReplCommand::Status),
        "web" => Some(ReplCommand::Web),
        "all" if !rest.is_empty() => Some(ReplCommand::Run(rest.to_string())),
        "claude" | "codex" | "grok" | "kimi" if !rest.is_empty() => {
            Some(ReplCommand::Send(head.to_string(), rest.to_string()))
        }
        _ => None,
    }
}

const HELP: &str = "oma.repl.help=all|claude|codex|grok|kimi <文本>|status|web|quit";

/// 打开浏览器：Windows start / Unix xdg-open；失败只警告。
fn open_browser(url: &str) {
    let spawned = if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if spawned.is_err() {
        eprintln!("oma: 打不开浏览器，地址是 {url}");
    }
}

pub async fn run(args: ReplArgs) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
    println!("oma.repl=true");
    println!("oma.project={}", root.display());

    // 会话已存在则重连，不叠格。
    start_session(&root, &args).await?;

    // web：默认起编排面；端口被占则顺延试几个。
    #[cfg(feature = "server")]
    let web_url = if args.no_web {
        println!("oma.web=disabled");
        None
    } else {
        let mut bound = None;
        for port in 7900..=7909 {
            if let Ok(addr) =
                crate::server::serve_in_background(root.clone(), port).await
            {
                bound = Some(format!("http://{addr}"));
                break;
            }
        }
        match bound {
            Some(url) => {
                println!("oma.web={url}");
                Some(url)
            }
            None => {
                eprintln!("oma: 编排面端口 7900-7909 都被占，网页不可用（CLI 通道不受影响）");
                None
            }
        }
    };
    #[cfg(not(feature = "server"))]
    let web_url: Option<String> = {
        println!("oma.web=disabled");
        None
    };

    if args.open {
        if let Some(url) = web_url.as_deref() {
            open_browser(url);
        }
    }
    println!("{HELP}");

    // stdin 阻塞读放独立线程；REPL 循环在 mpsc 上 await，serve 任务得以被轮询。
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let interactive = std::io::stdin().is_terminal();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    while let Some(line) = rx.recv().await {
        if interactive {
            print!("> ");
            let _ = std::io::stdout().flush();
        }
        match parse_repl_line(&line) {
            Some(ReplCommand::Quit) => {
                println!("oma.repl.quit=detach");
                break;
            }
            Some(ReplCommand::Status) => print_status(&root).await,
            Some(ReplCommand::Web) => match web_url.as_deref() {
                Some(url) => println!("oma.web={url}"),
                None => println!("oma.web=disabled"),
            },
            Some(ReplCommand::Run(text)) => match api::run(&root, &text, None, None).await {
                Ok(v) => {
                    println!(
                        "run.task={} sent={}",
                        v["task"].as_str().unwrap_or("-"),
                        v["sent"].as_array().map(|a| a.len()).unwrap_or(0)
                    );
                }
                Err(e) => eprintln!("oma: {e}"),
            },
            Some(ReplCommand::Send(agent, text)) => {
                match api::send(&root, &agent, &text, None).await {
                    Ok(_) => println!("send.ok=true agent={agent}"),
                    Err(e) => eprintln!("oma: {e}"),
                }
            }
            None => eprintln!("oma: 未知或残缺命令；可用见 {HELP}"),
        }
    }
    // EOF 或 quit：只 detach，会话留存。
    Ok(())
}

/// 起会话或重连；返回值仅为统一错误出口。
async fn start_session(root: &std::path::Path, args: &ReplArgs) -> Result<(), String> {
    if orch::read_manifest_for(root).is_some() {
        println!("oma.session=reconnected {}", orch::session_name(root)?.as_str());
        return Ok(());
    }
    let v = api::spawn(root, args.agents.clone(), args.stub).await?;
    let agents: Vec<String> = v["agents"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    println!("oma.session=spawned {} agents={}", orch::session_name(root)?.as_str(), agents.join(","));
    Ok(())
}

async fn print_status(root: &std::path::Path) {
    match orch::connect(root, false).await {
        Ok(link) => match orch::status(&link, root).await {
            Ok(panes) => print!("{}", render_status_table(&panes)),
            Err(e) => eprintln!("oma: {e}"),
        },
        Err(e) => eprintln!("oma: {e}"),
    }
}

/// TTY 人读表格（S016 吸收，CLI 通道共用）：列宽取表头与单元格最大字符宽，
/// 两空格槽。管道与测试面不在此函数的职责内（cmd_status 负责分流）。
pub fn render_status_table(panes: &[orch::PaneStatus]) -> String {
    let headers = ["AGENT", "PID", "PROCESS", "TERMINAL", "HOOK"];
    let rows: Vec<Vec<String>> = panes
        .iter()
        .map(|p| {
            vec![
                p.agent.clone(),
                p.pid.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
                p.process.clone().unwrap_or_else(|| "-".into()),
                p.terminal.to_string(),
                p.hook_state.clone().unwrap_or_else(|| "silent".into()),
            ]
        })
        .collect();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in &rows {
        for (i, c) in r.iter().enumerate() {
            widths[i] = widths[i].max(c.chars().count());
        }
    }
    let render = |cells: Vec<String>| -> String {
        cells
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let pad = " ".repeat(widths[i] - c.chars().count());
                format!("{c}{pad}")
            })
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    let mut out = String::new();
    out.push_str(&render(headers.iter().map(|s| s.to_string()).collect()));
    out.push('\n');
    out.push_str(&render(widths.iter().map(|w| "-".repeat(*w)).collect()));
    out.push('\n');
    for r in rows {
        out.push_str(&render(r));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repl_parser_routes_all_forms() {
        assert_eq!(
            parse_repl_line("all 把任务做完"),
            Some(ReplCommand::Run("把任务做完".into()))
        );
        assert_eq!(
            parse_repl_line("  claude   echo hi  "),
            Some(ReplCommand::Send("claude".into(), "echo hi".into()))
        );
        assert_eq!(parse_repl_line("status"), Some(ReplCommand::Status));
        assert_eq!(parse_repl_line("web"), Some(ReplCommand::Web));
        assert_eq!(parse_repl_line("quit"), Some(ReplCommand::Quit));
        assert_eq!(parse_repl_line("exit"), Some(ReplCommand::Quit));
    }

    #[test]
    fn repl_parser_rejects_degenerate_lines() {
        // 空行、裸 all、裸 agent 名（无文本）、未知命令都不进分派。
        assert_eq!(parse_repl_line(""), None);
        assert_eq!(parse_repl_line("   "), None);
        assert_eq!(parse_repl_line("all"), None);
        assert_eq!(parse_repl_line("claude"), None);
        assert_eq!(parse_repl_line("bogus whatever"), None);
    }

    fn pane(agent: &str, pid: Option<u32>, terminal: &'static str) -> orch::PaneStatus {
        orch::PaneStatus {
            agent: agent.into(),
            pid,
            process: Some("pwsh.exe".into()),
            terminal,
            hook_state: None,
        }
    }

    #[test]
    fn status_table_aligns_columns() {
        let panes = vec![pane("claude", Some(123), "idle"), pane("codex", Some(45678), "idle")];
        let table = render_status_table(&panes);
        let mut lines = table.lines();
        let header = lines.next().expect("header");
        let sep = lines.next().expect("separator");
        let row1 = lines.next().expect("row1");
        let row2 = lines.next().expect("row2");
        assert!(header.contains("AGENT") && header.contains("TERMINAL") && header.contains("HOOK"));
        assert!(sep.starts_with("------"), "separator dashes under AGENT column");
        // 对齐契约：同列单元格起始字符位一致（idle 与 pid 列各验一次）。
        assert_eq!(row1.find("idle"), row2.find("idle"));
        assert_eq!(row1.find("123"), row2.find("45678"));
    }

    #[test]
    fn status_table_marks_missing_fields() {
        let table = render_status_table(&[pane("grok", None, "blocked")]);
        let row = table.lines().nth(2).expect("row");
        assert!(row.contains("-") && row.contains("silent") && row.contains("blocked"));
    }
}
