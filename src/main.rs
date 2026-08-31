use std::path::PathBuf;

use clap::{Parser, Subcommand};

use oma::agents;
use oma::catalog::RmuxPin;
use oma::doctor;
use oma::hook;
use oma::install;
use oma::orch;
use oma::trace;
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
    /// 检测本机已装哪些 agent（PATH、OMA_AGENT_PATH、OMA_*_BIN、oma 自管根、默认目录）
    Agents {
        #[command(subcommand)]
        cmd: Option<AgentsCmd>,
    },
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
    /// 把任务分派给会话内多路 agent（状态门：一路 blocked 不堵其它路）
    Run {
        /// 任务文本（单行两段式，多行三段式粘贴）
        text: String,
        /// 指定分派路（逗号分隔）；缺省全会话
        #[arg(long, value_delimiter = ',')]
        assign: Option<Vec<String>>,
        /// 期望在画面上看到的确认短头
        #[arg(long)]
        confirm: Option<String>,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 自检测并自动确认信任框（各家自己持久化信任；预置信任的兜底）
    Settle {
        /// 每路最长等待秒数
        #[arg(long, default_value_t = 30)]
        wait: u64,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 检索项目的 agent 意图操作块与编辑轨迹（查询时读各家原生会话库）
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },
}

#[derive(Subcommand)]
enum TraceCmd {
    /// 列项目内各 agent 的会话
    Sessions {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 列编辑事件（按 operation_id 归组的意图操作块）
    Timeline {
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 文件过滤（glob；解析失败退子串）
        #[arg(long)]
        file: Option<String>,
        /// 条数上限（1-1000）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 按正则检索 patch、file、双意图四域（非法正则退字面子串）
    Search {
        query: String,
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 单文件的 agent 修改轨迹：谁、何时、基于什么意图改了这个文件
    File {
        /// 项目内相对路径（可用 glob）
        file: String,
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 意图操作块视图：一个 operation_id 一块（一次工具调用，可能多文件）
    Blocks {
        /// 只看某家 agent
        #[arg(long)]
        agent: Option<String>,
        /// 条数上限（1-1000，取最新 N 块）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// agent 轨迹：某家 agent 在项目内的操作块时间线
    Agent {
        /// agent 名（claude/codex/grok/kimi）
        name: String,
        /// 条数上限（1-1000，取最新 N 块）
        #[arg(long, default_value_t = 100)]
        limit: usize,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// 安装缺失的 agent（oma 自管根 ~/.ohmyagents；已装任何来源即跳过；github 主 CDN 兜底）
    Install {
        /// agent 名列表；缺省 = catalog 全部的缺失者
        names: Vec<String>,
        /// 已装也重装（oma 自管根）
        #[arg(long)]
        force: bool,
        /// 自定义 oma 应用数据根；缺省 OMA_HOME 环境变量或 ~/.ohmyagents
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// 解析最新版并升级 oma 自管安装，取证 sha256 后写回用户本地 pin
    Update {
        /// agent 名列表；缺省 = catalog 全部
        names: Vec<String>,
        /// 已是最新也强制重取重装
        #[arg(long)]
        force: bool,
        /// 自定义 oma 应用数据根
        #[arg(long)]
        root: Option<PathBuf>,
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
        Commands::Agents { cmd } => match cmd {
            None => {
                agents::print_reports(&agents::detect());
                Ok(())
            }
            Some(AgentsCmd::Install { names, force, root }) => cmd_agents_install(names, force, root),
            Some(AgentsCmd::Update { names, force, root }) => cmd_agents_update(names, force, root),
        },
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
        Commands::Run {
            text,
            assign,
            confirm,
            project,
        } => tokio_block(cmd_run(text, assign, confirm, project)),
        Commands::Settle { wait, project } => tokio_block(cmd_settle(wait, project)),
        Commands::Trace { cmd } => cmd_trace(cmd),
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
    let link = orch::connect(&root, true).await?;
    println!("spawn.label={}", link.label);
    let manifest = orch::spawn(&link, &root, &plan).await?;
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
    let link = orch::connect(&root, false).await?;
    let panes = orch::status(&link, &root).await?;
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
    let link = orch::connect(&root, false).await?;
    orch::send(&link, &root, &agent, &text, confirm.as_deref()).await?;
    println!("send.ok=true");
    Ok(())
}

async fn cmd_cleanup(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let link = orch::connect(&root, false).await?;
    let existed = orch::cleanup(&link, &root).await?;
    println!("cleanup.killed={existed}");
    println!("cleanup.scope=session");
    println!("cleanup.ok=true");
    Ok(())
}

async fn cmd_settle(wait: u64, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let link = orch::connect(&root, false).await?;
    orch::settle(&link, &root, wait).await?;
    println!("settle.scope=trust-dialogs");
    println!("settle.ok=true");
    Ok(())
}

async fn cmd_run(
    text: String,
    assign: Option<Vec<String>>,
    confirm: Option<String>,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    let link = orch::connect(&root, false).await?;
    let outcome = orch::run(&link, &root, &text, assign, confirm.as_deref()).await?;
    println!("run.task.id={}", outcome.task_id);
    println!("run.sent={}", outcome.sent.join(","));
    for (agent, reason) in &outcome.skipped {
        println!("run.skipped={agent}:{reason}");
    }
    if outcome.sent.is_empty() {
        eprintln!("oma: every lane was gated ({} skipped); nothing dispatched", outcome.skipped.len());
        std::process::exit(1);
    }
    println!("run.ok=true");
    Ok(())
}

fn cmd_agents_install(
    names: Vec<String>,
    force: bool,
    root: Option<PathBuf>,
) -> Result<(), String> {
    let home = root.map(Ok).unwrap_or_else(install::oma_home)?;
    let catalog = install::resolve_catalog(&home)?;
    let mut failed = 0u32;
    for (name, result) in install::install_missing(&catalog, &names, &home, force) {
        match result {
            Ok(install::InstallOutcome::Installed { version, probed, path }) => {
                println!("install.{name}.status=installed version={version}");
                match &probed {
                    Some(v) => println!("install.{name}.probe={v}"),
                    None => println!("install.{name}.probe=unavailable"),
                }
                println!("install.{name}.path={}", path.display());
            }
            Ok(install::InstallOutcome::Skipped { detail }) => {
                println!("install.{name}.status=skipped detail={detail}");
            }
            Err(e) => {
                failed += 1;
                println!("install.{name}.status=failed detail={e}");
            }
        }
    }
    println!("install.home={}", home.display());
    if failed > 0 {
        Err(format!("{failed} agent(s) failed to install"))
    } else {
        Ok(())
    }
}

fn cmd_agents_update(names: Vec<String>, force: bool, root: Option<PathBuf>) -> Result<(), String> {
    let home = root.map(Ok).unwrap_or_else(install::oma_home)?;
    let catalog = install::resolve_catalog(&home)?;
    let wanted: Vec<String> = if names.is_empty() {
        catalog.agents.iter().map(|p| p.name.clone()).collect()
    } else {
        names
    };
    let mut failed = 0u32;
    for name in &wanted {
        match install::update_agent(&home, name, force) {
            Ok(install::UpdateOutcome::Updated { from, to }) => {
                println!("update.{name}.status=updated from={from} to={to}");
            }
            Ok(install::UpdateOutcome::UpToDate { version }) => {
                println!("update.{name}.status=uptodate version={version}");
            }
            Ok(install::UpdateOutcome::Skipped { detail }) => {
                println!("update.{name}.status=skipped detail={detail}");
            }
            Err(e) => {
                failed += 1;
                println!("update.{name}.status=failed detail={e}");
            }
        }
    }
    println!("update.home={}", home.display());
    if failed > 0 {
        Err(format!("{failed} agent(s) failed to update"))
    } else {
        Ok(())
    }
}

fn cmd_trace(cmd: TraceCmd) -> Result<(), String> {
    let clip = |s: &str| -> String { s.chars().take(80).collect() };
    let resolve = |p: Option<PathBuf>| {
        p.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    };
    match cmd {
        TraceCmd::Sessions { project } => {
            let project = resolve(project);
            let sessions = trace::list_sessions(&project);
            for s in &sessions {
                println!(
                    "trace.session agent={} id={} started={} file={}",
                    s.agent,
                    s.id,
                    s.started_at.as_deref().unwrap_or("-"),
                    s.file.display()
                );
            }
            println!("trace.sessions.count={}", sessions.len());
            Ok(())
        }
        TraceCmd::Timeline { agent, file, limit, project } => {
            let project = resolve(project);
            let filter = trace::TraceFilter {
                agent: agent.as_deref(),
                file_glob: file.as_deref(),
                limit,
            };
            let events = trace::apply_filter(trace::timeline(&project), &filter);
            for e in &events {
                println!(
                    "trace.edit agent={} session={} op={} file={} kind={} tool={} ts={} intent={} op_intent={}",
                    e.agent,
                    e.session_id,
                    e.operation_id(),
                    e.file.as_deref().unwrap_or("-"),
                    e.kind.as_str(),
                    e.tool.as_deref().unwrap_or("-"),
                    e.ts.as_deref().unwrap_or("-"),
                    clip(e.user_intent.as_deref().unwrap_or("-")),
                    clip(e.op_intent.as_deref().unwrap_or("-")),
                );
            }
            println!("trace.edits.count={}", events.len());
            Ok(())
        }
        TraceCmd::Search { query, agent, limit, project } => {
            let project = resolve(project);
            // 先全量匹配再截断：limit 若在匹配前生效会把候选池截没。
            let filter = trace::TraceFilter {
                agent: agent.as_deref(),
                file_glob: None,
                limit: trace::MAX_LIMIT,
            };
            let mut events: Vec<_> = trace::apply_filter(trace::timeline(&project), &filter)
                .into_iter()
                .filter(|e| trace::search_matches(e, &query))
                .collect();
            let block_count = trace::group_blocks(&events).len();
            events.truncate(limit.clamp(1, trace::MAX_LIMIT));
            for e in &events {
                println!(
                    "trace.hit agent={} session={} op={} file={} kind={} intent={} op_intent={}",
                    e.agent,
                    e.session_id,
                    e.operation_id(),
                    e.file.as_deref().unwrap_or("-"),
                    e.kind.as_str(),
                    clip(e.user_intent.as_deref().unwrap_or("-")),
                    clip(e.op_intent.as_deref().unwrap_or("-")),
                );
            }
            println!("trace.hits.count={}", events.len());
            println!("trace.blocks.count={block_count}");
            Ok(())
        }
        TraceCmd::File { file, agent, limit, project } => {
            let project = resolve(project);
            // 文件维度轨迹：按传入路径或 glob 过滤，时间正序展示该文件的完整修改史。
            let filter = trace::TraceFilter {
                agent: agent.as_deref(),
                file_glob: Some(&file),
                limit: limit.clamp(1, trace::MAX_LIMIT),
            };
            let events = trace::apply_filter(trace::timeline(&project), &filter);
            for e in &events {
                println!(
                    "trace.file agent={} session={} op={} kind={} tool={} ts={} intent={} op_intent={}",
                    e.agent,
                    e.session_id,
                    e.operation_id(),
                    e.kind.as_str(),
                    e.tool.as_deref().unwrap_or("-"),
                    e.ts.as_deref().unwrap_or("-"),
                    clip(e.user_intent.as_deref().unwrap_or("-")),
                    clip(e.op_intent.as_deref().unwrap_or("-")),
                );
            }
            println!("trace.file.edits={}", events.len());
            Ok(())
        }
        TraceCmd::Blocks { agent, limit, project } => {
            let project = resolve(project);
            print_block_timeline(&project, agent.as_deref(), limit);
            Ok(())
        }
        TraceCmd::Agent { name, limit, project } => {
            let project = resolve(project);
            let known = ["claude", "codex", "grok", "kimi"];
            if !known.contains(&name.as_str()) {
                return Err(format!("unknown agent {name}; known: {}", known.join(", ")));
            }
            print_block_timeline(&project, Some(&name), limit);
            Ok(())
        }
    }
}

/// 操作块时间线：时间正序展示最新 N 块（与 timeline 的「最新 N 条」语义一致）。
fn print_block_timeline(project: &std::path::Path, agent: Option<&str>, limit: usize) {
    let clip = |s: &str| -> String { s.chars().take(80).collect() };
    let filter = trace::TraceFilter {
        agent,
        file_glob: None,
        limit: trace::MAX_LIMIT,
    };
    let events = trace::apply_filter(trace::timeline(project), &filter);
    let mut blocks = trace::group_blocks(&events);
    let n = limit.clamp(1, trace::MAX_LIMIT);
    if blocks.len() > n {
        // 丢最旧，保时间正序。
        let cut = blocks.len() - n;
        blocks.drain(0..cut);
    }
    for b in &blocks {
        println!(
            "trace.block op={} agent={} session={} edits={} files={} kinds={} ts={} intent={} op_intent={}",
            b.op,
            b.agent,
            b.session_id,
            b.edits,
            b.files.join(","),
            b.kinds.join("+"),
            b.first_ts.as_deref().unwrap_or("-"),
            clip(b.user_intent.as_deref().unwrap_or("-")),
            clip(b.op_intent.as_deref().unwrap_or("-")),
        );
    }
    println!("trace.blocks.count={}", blocks.len());
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
    // Default init is the full deployment: yolo keys plus project-level
    // hook/skill registration (S015 matrix). --yolo narrows to keys only.
    let report = yolo::apply_project_yolo(&root)?;
    println!("init.flag.yolo={yolo}");
    for p in &report.wrote {
        println!("init.wrote={p}");
    }
    if !yolo {
        let deployed = oma::deploy::apply_project_hooks(&root)?;
        for p in &deployed.wrote {
            println!("init.hooks.wrote={p}");
        }
        println!("init.hooks.wrote.count={}", deployed.wrote.len());
        println!(
            "init.hooks.skipped.count={}",
            deployed.skipped.len()
        );
    } else {
        println!("init.hooks=skipped");
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
    println!("init.scope={}", if yolo { "yolo" } else { "full" });
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
