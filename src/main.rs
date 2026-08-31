use std::path::PathBuf;

use clap::{Parser, Subcommand};

use oma::agents;
use oma::catalog::RmuxPin;
use oma::doctor;
use oma::hook;
use oma::orch;
use oma::rmux::{self, bin_dir, ensure, managed_root, prepend_path, CheckError, Source};
use oma::yolo;

#[derive(Parser)]
#[command(name = "oma", about = "Oh My Agents：通用智能体多路复用任务编排器")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 检测 rmux 版本与哈希；缺 pin 版本则按 catalog 安装完整包
    Check {
        /// 只诊断，不下载安装
        #[arg(long)]
        no_install: bool,
    },
    /// 项目级 yolo 落盘（本 POC 不含 hook/skill）
    Init {
        /// 写项目级无阻塞键（当前 init 的全部工作）
        #[arg(long)]
        yolo: bool,
        /// 预写用户家目录信任库（claude/codex/kimi/grok）
        #[arg(long)]
        pretrust: bool,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 只读诊断 yolo / 信任 / 二进制 / state；不 attach
    Doctor {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 检测本机已装哪些 agent（PATH、OMA_AGENT_PATH、OMA_*_BIN、默认目录）
    Agents,
    /// Agent hook 入口：读事件写 `.ohmyagents/state`。缺 OHMYAGENTS_STATE_FILE 则静默退出
    Hook {
        /// 事件名或四态（idle/working/blocked/unknown）；省略则读 stdin JSON
        event: Option<String>,
    },
    /// 在项目专属会话里拉起多路 agent（1-4 路；默认已装交集）
    Spawn {
        /// 指定 agent 列表（逗号分隔）；缺省取已装交集
        #[arg(long, value_delimiter = ',')]
        agents: Option<Vec<String>>,
        /// 用 shell 桩替代真实 agent（验收与调试）
        #[arg(long)]
        stub: bool,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 只读列出本项目会话各 agent 的 pid、进程名、终端态与 hook 态
    Status {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 向会话内某路 agent 发单行任务（文本与 Enter 分发）
    Send {
        /// 目标 agent 名（claude/codex/grok/kimi）
        agent: String,
        /// 单行任务文本
        text: String,
        /// 期望在画面上看到的确认短头
        #[arg(long)]
        confirm: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 只杀本项目的会话（不动 daemon 与其它会话）
    Cleanup {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("oma: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check { no_install } => cmd_check(no_install),
        Commands::Init {
            yolo,
            pretrust,
            project,
        } => cmd_init(yolo, pretrust, project),
        Commands::Doctor { project } => cmd_doctor(project),
        Commands::Agents => {
            agents::print_reports(&agents::detect());
            Ok(())
        }
        Commands::Hook { event } => cmd_hook(event),
        Commands::Spawn {
            agents,
            stub,
            project,
        } => tokio_block(cmd_spawn(agents, stub, project)),
        Commands::Status { project } => tokio_block(cmd_status(project)),
        Commands::Send {
            agent,
            text,
            confirm,
            project,
        } => tokio_block(cmd_send(agent, text, confirm, project)),
        Commands::Cleanup { project } => tokio_block(cmd_cleanup(project)),
    }
}

fn tokio_block<F: std::future::Future<Output = Result<(), String>>>(
    fut: F,
) -> Result<(), String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?
        .block_on(fut)
}

async fn cmd_spawn(
    wanted: Option<Vec<String>>,
    stub: bool,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    let plan = orch::plan_agents(wanted, stub)?;
    println!("spawn.project={}", root.display());
    println!("spawn.stub={stub}");
    let names: Vec<&str> = plan.agents.iter().map(|(n, _)| n.as_str()).collect();
    println!("spawn.agents={}", names.join(","));
    let rmux = orch::connect(&root).await?;
    let manifest = orch::spawn(&rmux, &root, &plan).await?;
    println!(
        "spawn.session={}",
        orch::session_name(&root)?.as_str()
    );
    println!("spawn.manifest.agents={}", manifest.agents.len());
    println!("spawn.blocking=false");
    println!("spawn.ok=true");
    Ok(())
}

async fn cmd_status(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    println!("status.project={}", root.display());
    println!(
        "status.session={}",
        orch::session_name(&root)?.as_str()
    );
    let rmux = orch::connect(&root).await?;
    let panes = orch::status(&rmux, &root).await?;
    for p in &panes {
        println!(
            "status.pane.{}.pid={}",
            p.agent,
            p.pid.map(|v| v.to_string()).unwrap_or_else(|| "-".into())
        );
        println!(
            "status.pane.{}.proc={}",
            p.agent,
            p.process.as_deref().unwrap_or("-")
        );
        println!("status.pane.{}.terminal={}", p.agent, p.terminal);
        println!(
            "status.pane.{}.hook={}",
            p.agent,
            p.hook_state.as_deref().unwrap_or("silent")
        );
    }
    println!("status.panes={}", panes.len());
    println!("status.ok=true");
    Ok(())
}

async fn cmd_send(
    agent: String,
    text: String,
    confirm: Option<String>,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    let rmux = orch::connect(&root).await?;
    orch::send(&rmux, &root, &agent, &text, confirm.as_deref()).await?;
    println!("send.ok=true");
    Ok(())
}

async fn cmd_cleanup(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let rmux = orch::connect(&root).await?;
    let existed = orch::cleanup(&rmux, &root).await?;
    println!("cleanup.killed={existed}");
    println!("cleanup.scope=session");
    println!("cleanup.ok=true");
    Ok(())
}

fn cmd_hook(event: Option<String>) -> Result<(), String> {
    match hook::run(event.as_deref()) {
        Ok(Some(path)) => {
            if std::env::var_os("OMA_HOOK_VERBOSE").is_some() {
                eprintln!("oma.hook.wrote={}", path.display());
            }
        }
        Ok(None) => {}
        Err(e) => {
            // Never fail the agent session over a state-file write.
            if std::env::var_os("OMA_HOOK_VERBOSE").is_some() {
                eprintln!("oma hook: {e}");
            }
        }
    }
    Ok(())
}

fn project_root(project: Option<PathBuf>) -> Result<PathBuf, String> {
    match project {
        Some(p) => Ok(p),
        None => std::env::current_dir().map_err(|e| format!("cwd: {e}")),
    }
}

fn cmd_init(yolo: bool, pretrust: bool, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    std::fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    // This POC's init is yolo persistence. --yolo is the documented name; omitting it still writes.
    let report = yolo::apply_project_yolo(&root)?;
    println!("init.flag.yolo={yolo}");
    for p in &report.wrote {
        println!("init.wrote={p}");
    }
    if pretrust {
        let trust = yolo::apply_pretrust(&root)?;
        for p in &trust.wrote {
            println!("init.pretrust.wrote={p}");
        }
        println!("init.pretrust=wrote");
    } else {
        println!("init.pretrust=skipped");
    }
    println!("init.project={}", root.display());
    println!("init.scope=yolo");
    Ok(())
}

fn cmd_doctor(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let d = doctor::diagnose(&root)?;
    doctor::print_diagnosis(&d);
    if d.blocked() {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_check(no_install: bool) -> Result<(), String> {
    let pin = RmuxPin::load()?;
    let (os, arch) = rmux::host_os_arch();
    let asset = pin.asset_for(os, arch).map_err(|e| e.to_string())?;

    match ensure(&pin, !no_install) {
        Ok(report) => {
            if let Some(dir) = report.layout.dispatcher.parent() {
                prepend_path(dir);
            }
            let src = match report.source {
                Source::Managed => "managed",
                Source::Path => "PATH",
            };
            println!("rmux.ok=true");
            println!("rmux.source={src}");
            println!("rmux.path={}", report.layout.dispatcher.display());
            println!("rmux.helper={}", report.layout.helper.display());
            println!("rmux.daemon={}", report.layout.daemon.display());
            println!("rmux.version={}", report.version);
            println!("rmux.pin={}", pin.version);
            println!("rmux.asset={}", asset.name);
            println!("rmux.asset_sha256={}", asset.sha256);
            println!("rmux.dispatcher_sha256={}", report.dispatcher_sha256);
            println!("rmux.helper_sha256={}", report.helper_sha256);
            println!("rmux.daemon_sha256={}", report.daemon_sha256);
            if let Some(a) = report.archive_sha256 {
                println!("rmux.archive_sha256={a}");
            }
            if report.version != pin.version {
                return Err(format!(
                    "version {} does not match pin {}",
                    report.version, pin.version
                ));
            }
            let managed = managed_root(&pin).map_err(|e| e.to_string())?;
            println!("rmux.managed_root={}", managed.display());
            println!("rmux.bin_dir={}", bin_dir(&managed).display());
            Ok(())
        }
        Err(CheckError::Missing { reason }) => Err(format!(
            "{reason}; rerun without --no-install to download {}",
            pin.tag
        )),
        Err(CheckError::Message(m)) => Err(m),
    }
}
