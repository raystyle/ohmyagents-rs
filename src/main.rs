use std::path::PathBuf;

use clap::{Parser, Subcommand};

use oma::agents;
use oma::catalog::RmuxPin;
use oma::doctor;
use oma::hook;
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
    }
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
