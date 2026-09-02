use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::Value;

use oma::agents;
use oma::catalog::RmuxPin;
use oma::doctor;
use oma::hook;
use oma::install;
use oma::orch;
use oma::rmux::{self, bin_dir, ensure, managed_root, prepend_path, CheckError, Source};
use oma::trace;
use oma::yolo;

#[derive(Parser)]
#[command(name = "oma", about = "Oh My Agents：通用智能体多路复用任务编排器")]
struct Cli {
    /// REPL：不开 HTTP 编排面
    #[arg(long)]
    no_web: bool,
    /// REPL：打印 URL 后尝试打开浏览器（失败只警告）
    #[arg(long)]
    open: bool,
    /// REPL：用 shell 桩替代真实 agent（验收与调试）
    #[arg(long)]
    stub: bool,
    /// REPL：指定 agent 列表（逗号分隔）；缺省取已装交集
    #[arg(long, value_delimiter = ',')]
    agents: Option<Vec<String>>,
    #[command(subcommand)]
    command: Option<Commands>,
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
    /// Agent hook 入口：读事件写 `.ohmyagents/state`。oma 会话走 env；用户手拉会话按 payload cwd 回退写项目状态文件
    Hook {
        /// 事件名或四态（idle/working/blocked/unknown）；省略则读 stdin JSON
        event: Option<String>,
        /// agent 名（注册参数注入；回退写项目状态文件用）
        #[arg(long)]
        agent: Option<String>,
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
        /// 输出 JSON 信封（与 HTTP/MCP 同形）而非 marker 行
        #[arg(long)]
        json: bool,
    },
    /// 只读列出本项目会话各 agent 的 pid、进程名、终端态与 hook 态
    Status {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
        /// 输出 JSON 信封（与 HTTP/MCP 同形）而非 marker 行
        #[arg(long)]
        json: bool,
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
        /// 输出 JSON 信封（与 HTTP/MCP 同形）而非 marker 行
        #[arg(long)]
        json: bool,
    },
    /// 向某路 agent 发单个按键（受守卫：codex 拒 C-c——一个 C-c 杀进程，M001）
    Key {
        /// 目标 agent 名
        agent: String,
        /// 键名（Enter/Esc/t/Up/Down/...，直传 rmux send-keys 语义）
        key: String,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 带产物等待的任务委派：建任务目录（prompt.md）发 agent，阻塞等产物
    ///（agent 写 output.md 后创建 DONE；oma task 等 DONE 出现打产物退出）
    Task {
        /// 目标 agent 名（claude/codex/grok/kimi）
        agent: Option<String>,
        /// 任务文本（全文落 prompt.md，send 带协议尾注）
        text: Option<String>,
        /// 等产物秒数；0 无限等（缺省 600，上限 86400）
        #[arg(long, default_value_t = 600)]
        timeout: u64,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
        #[command(subcommand)]
        cmd: Option<TaskCmd>,
    },
    /// 只杀本项目的会话（不动 daemon 与其它会话）
    Cleanup {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
        /// 输出 JSON 信封（与 HTTP/MCP 同形）而非 marker 行
        #[arg(long)]
        json: bool,
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
        /// 输出 JSON 信封（与 HTTP/MCP 同形）而非 marker 行
        #[arg(long)]
        json: bool,
    },
    /// 自检测并自动确认信任框（各家自己持久化信任；预置信任的兜底）
    Settle {
        /// 全局扫描窗口秒数（上限 600；窗口内反复扫全部路等晚出现的屏）
        #[arg(long, default_value_t = 30)]
        wait: u64,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
        /// 输出 JSON 信封（与 HTTP/MCP 同形）而非 marker 行
        #[arg(long)]
        json: bool,
    },
    /// 检索项目的 agent 意图操作块与编辑轨迹（查询时读各家原生会话库）
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },
    /// 起 HTTP 编排面（后台守护：start 即调即退，stop/status 管理）
    Serve {
        #[command(subcommand)]
        cmd: Option<ServeCmd>,
        /// 监听端口（无子命令直启时用）
        #[arg(long, default_value_t = 7900)]
        port: u16,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// serve 守护进程本体（由 `oma serve start` 孤儿化拉起）
    #[command(hide = true)]
    ServeDaemon {
        /// 监听端口
        #[arg(long)]
        port: u16,
        /// 项目根
        #[arg(long)]
        project: PathBuf,
    },
    /// 作为 MCP server 跑在 stdio（六操作 tools 加 trace 检索 tools）
    Mcp {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
        /// 不起 server，打印各客户端的注册配置片段后退出
        #[arg(long)]
        print_config: bool,
    },
    /// oma 自身管理（self update 自更新）
    #[command(name = "self")]
    SelfGroup {
        #[command(subcommand)]
        cmd: SelfSub,
    },
    /// 生成 shell 补全脚本到 stdout（S016 吸收）
    Completions {
        /// 目标 shell
        shell: clap_complete::Shell,
    },
    /// 强制重新打开一路 agent 实例（关闭旧窗格再开新一路；不动会话与其它路）
    Respawn {
        /// agent 名（claude/codex/grok/kimi）
        agent: String,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
        /// 输出 JSON 信封（与 HTTP/MCP 同形）
        #[arg(long)]
        json: bool,
    },
    /// 起官方 web 镜像（rmux web-share）：operator 可操作真 attach
    Web {
        /// 单路 agent 名；缺省整会话镜像（全窗格加 session 控制）
        agent: Option<String>,
        /// 只读旁观（缺省 operator 可操作）
        #[arg(long)]
        spectator: bool,
        /// 有效期秒数（缺省 3600）
        #[arg(long, default_value_t = 3600)]
        ttl: u64,
        /// 免 PIN 直连（本地场景；缺省保留 PIN 防外发）
        #[arg(long)]
        no_pin: bool,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SelfSub {
    /// oma 自更新：GitHub Releases 拉新版自替换；封版前用 --git 源码安装
    Update {
        /// 仓库（owner/name）；缺省 raystyle/OhMyAgents
        #[arg(long)]
        repo: Option<String>,
        /// 走 cargo install --git 源码安装（封版前主路径）
        #[arg(long)]
        git: bool,
        /// 同版本也重装
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ServeCmd {
    /// 后台启动编排面（即调即退；已活直接返回地址）
    Start {
        /// 监听端口
        #[arg(long, default_value_t = 7900)]
        port: u16,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 停止后台编排面（按记录 pid 杀）
    Stop {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 查看后台编排面状态
    Status {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum TaskCmd {
    /// 列任务目录与完成态（id / agent / DONE 有无）
    List {
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 看一个任务：元数据加产物（有则全量打印）
    Show {
        /// 任务 id（如 t001）
        id: String,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
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
    /// 配置四家状态栏（幂等：claude/codex/kimi/grok 各自配置面，脚本随 oma 释放）
    Statusline {
        /// 指定 agent（claude/codex/kimi/grok）；缺省四家都配
        names: Vec<String>,
    },
    /// 提供商别名簿（~/.ohmyagents/providers.toml，标准 sops 托管）
    Providers {
        /// 打印示例模板（含 sops 托管说明）后退出
        #[arg(long)]
        example: bool,
    },
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
    let Some(command) = cli.command else {
        return cmd_repl(cli.no_web, cli.open, cli.stub, cli.agents);
    };
    match command {
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
            Some(AgentsCmd::Install { names, force, root }) => {
                cmd_agents_install(names, force, root)
            }
            Some(AgentsCmd::Update { names, force, root }) => cmd_agents_update(names, force, root),
            Some(AgentsCmd::Statusline { names }) => cmd_agents_statusline(names),
            Some(AgentsCmd::Providers { example }) => cmd_agents_providers(example),
        },
        Commands::Hook { event, agent } => cmd_hook(event, agent),
        Commands::Spawn {
            agents,
            stub,
            project,
            json,
        } => tokio_block(cmd_spawn(agents, stub, project, json)),
        Commands::Status { project, json } => tokio_block(cmd_status(project, json)),
        Commands::Send {
            agent,
            text,
            confirm,
            project,
            json,
        } => tokio_block(cmd_send(agent, text, confirm, project, json)),
        Commands::Key {
            agent,
            key,
            project,
        } => tokio_block(cmd_key(agent, key, project)),
        Commands::Task {
            agent,
            text,
            timeout,
            project,
            cmd,
        } => match cmd {
            Some(TaskCmd::List { project: inner }) => cmd_task_list(inner.or(project)),
            Some(TaskCmd::Show { id, project: inner }) => cmd_task_show(id, inner.or(project)),
            None => tokio_block(cmd_task(agent, text, timeout, project)),
        },
        Commands::Cleanup { project, json } => tokio_block(cmd_cleanup(project, json)),
        Commands::Run {
            text,
            assign,
            confirm,
            project,
            json,
        } => tokio_block(cmd_run(text, assign, confirm, project, json)),
        Commands::Settle {
            wait,
            project,
            json,
        } => tokio_block(cmd_settle(wait, project, json)),
        Commands::Trace { cmd } => cmd_trace(cmd),
        Commands::Serve { cmd, port, project } => match cmd {
            None => cmd_serve(port, project),
            Some(ServeCmd::Start { port, project }) => cmd_serve_start(port, project),
            Some(ServeCmd::Stop { project }) => cmd_serve_stop(project),
            Some(ServeCmd::Status { project }) => cmd_serve_status(project),
        },
        Commands::ServeDaemon { port, project } => cmd_serve(port, Some(project)),
        Commands::Mcp {
            project,
            print_config,
        } => cmd_mcp(project, print_config),
        Commands::SelfGroup { cmd } => match cmd {
            SelfSub::Update { repo, git, force } => oma::update::run(
                &repo.unwrap_or_else(|| oma::update::DEFAULT_REPO.into()),
                git,
                force,
            ),
        },
        Commands::Completions { shell } => cmd_completions(shell),
        Commands::Respawn {
            agent,
            project,
            json,
        } => tokio_block(cmd_respawn(agent, project, json)),
        Commands::Web {
            agent,
            spectator,
            ttl,
            no_pin,
            project,
        } => tokio_block(cmd_web(agent, spectator, ttl, no_pin, project)),
    }
}

/// 重新打开一路 agent 实例：关闭旧窗格再开新一路（会话与其它路不动）。
async fn cmd_respawn(agent: String, project: Option<PathBuf>, json: bool) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        return print_json("respawn", &root, oma::api::respawn(&root, &agent).await);
    }
    let link = orch::connect(&root, false).await?;
    let pane_id = orch::respawn(&link, &root, &agent).await?;
    println!("respawn.agent={agent}");
    println!("respawn.pane={pane_id}");
    println!("respawn.scope=pane-only");
    println!("respawn.ok=true");
    Ok(())
}

/// 起官方 web 镜像：缺省全会话各一路；打印 URL 与 PIN（PIN 等同键盘权限，勿外传）。
async fn cmd_web(
    agent: Option<String>,
    spectator: bool,
    ttl: u64,
    no_pin: bool,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    let manifest = orch::read_manifest_for(&root)
        .ok_or_else(|| "no session manifest; run `oma spawn` first".to_string())?;
    // 校验：指定 agent 必须在本会话（api 层只认 agent 名，不重复收集）。
    if let Some(a) = &agent {
        if !manifest.agents.iter().any(|m| &m.name == a) {
            return Err(format!("agent {a} not in this session"));
        }
    }
    let v = match agent.as_deref() {
        Some(a) => {
            let v = oma::api::web_share(&root, Some(a), spectator, ttl, None, no_pin).await?;
            println!("web.{a}.url={}", v["url"].as_str().unwrap_or("-"));
            println!("web.{a}.pin={}", v["pin"].as_str().unwrap_or("-"));
            println!("web.{a}.expires={}", v["expires"].as_str().unwrap_or("-"));
            v
        }
        None => {
            // 整会话镜像：一个 URL 全窗格、operator 可编辑、带分屏控制。
            let v = oma::api::web_share(&root, None, spectator, ttl, None, no_pin).await?;
            println!("web.session.url={}", v["url"].as_str().unwrap_or("-"));
            println!("web.session.pin={}", v["pin"].as_str().unwrap_or("-"));
            println!(
                "web.session.expires={}",
                v["expires"].as_str().unwrap_or("-")
            );
            v
        }
    };
    // 公网中继 + 免 PIN 的显著警示（用户定调，P0026 后续）。
    if let Some(w) = v["warning"].as_str() {
        println!("web.warning={w}");
    }
    println!(
        "web.mode={}",
        if spectator { "spectator" } else { "operator" }
    );
    println!("web.ok=true");
    Ok(())
}

/// 裸 `oma` 进 REPL：spawn 默认不阻塞、起网页打印 URL、行循环分派。
fn cmd_repl(
    no_web: bool,
    open: bool,
    stub: bool,
    agents: Option<Vec<String>>,
) -> Result<(), String> {
    tokio_block(oma::repl::run(oma::repl::ReplArgs {
        agents,
        stub,
        no_web,
        open,
    }))
}

fn tokio_block<F: std::future::Future<Output = Result<(), String>>>(fut: F) -> Result<(), String> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?
        .block_on(fut)
}

/// `oma serve start`：即调即退——后台拉起，端口就绪后返回。
#[cfg(feature = "server")]
fn cmd_serve_start(port: u16, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let addr = oma::servectl::serve_start(&root, port)?;
    println!("serve.start.addr={addr}");
    println!("serve.start.kanban={addr}/");
    println!("serve.start.ok=true");
    Ok(())
}

#[cfg(not(feature = "server"))]
fn cmd_serve_start(_port: u16, _project: Option<PathBuf>) -> Result<(), String> {
    Err("oma serve needs the `server` feature; rebuild with --features server".to_string())
}

/// `oma serve stop`：按记录 pid 杀后台编排面。
fn cmd_serve_stop(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    match oma::servectl::serve_stop(&root)? {
        true => {
            println!("serve.stop.killed=true");
            println!("serve.stop.ok=true");
        }
        false => {
            println!("serve.stop.killed=false");
            println!("serve.stop.ok=true");
        }
    }
    Ok(())
}

/// `oma serve status`：探活并打记录。
fn cmd_serve_status(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let (live, rec) = oma::servectl::serve_status(&root);
    match rec {
        Some(r) => {
            println!("serve.status.pid={}", r.pid);
            println!("serve.status.port={}", r.port);
            println!("serve.status.project={}", r.project);
            println!("serve.status.live={live}");
        }
        None => println!("serve.status.live=false"),
    }
    println!("serve.status.ok=true");
    Ok(())
}

/// serve 是常驻命令：feature 缺失时给出可行动的报错而不是静默装死。
#[cfg(feature = "server")]
fn cmd_serve(port: u16, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    tokio_block(oma::server::serve(root, port))
}

#[cfg(not(feature = "server"))]
fn cmd_serve(_port: u16, _project: Option<PathBuf>) -> Result<(), String> {
    Err("oma serve needs the `server` feature; rebuild with --features server".to_string())
}

/// MCP stdio 常驻：stdout 是协议通道，一切进度只能进 stderr。
/// `--print-config` 只打印注册片段不起 server（任何构建形态可用）。
#[cfg(feature = "mcp")]
fn cmd_mcp(project: Option<PathBuf>, print_config: bool) -> Result<(), String> {
    let root = project_root(project)?;
    if print_config {
        return print_mcp_config(&root);
    }
    tokio_block(oma::mcp::run(root))
}

#[cfg(not(feature = "mcp"))]
fn cmd_mcp(project: Option<PathBuf>, print_config: bool) -> Result<(), String> {
    if print_config {
        let root = project_root(project)?;
        return print_mcp_config(&root);
    }
    Err("oma mcp needs the `mcp` feature; rebuild with --features mcp".to_string())
}

/// 注册片段：exe 绝对路径 + --project 锚定。给 Claude Code、codex 与通用
/// mcpServers 三种消费形态（挑自己的抄一段）。
fn print_mcp_config(root: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let exe = exe.display().to_string();
    let proj = root.display().to_string();
    println!("# Claude Code（shell 里执行一次）:");
    println!("claude mcp add oma -- \"{exe}\" mcp --project \"{proj}\"");
    println!();
    // TOML 转义（relay3 kimi5 订正：literal string 根本不允许单引号，
    // 双写也不合法——literal 仅当值不含 ' 时用；含则退 basic string 双引
    // 号加反斜杠转义）；JSON 走 serde_json。
    let toml_str = |s: &str| {
        if s.contains('\'') {
            format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
        } else {
            format!("'{s}'")
        }
    };
    println!("# codex（写入 ~/.codex/config.toml 的 [mcp_servers] 段）:");
    println!("[mcp_servers.oma]");
    println!("command = {}", toml_str(&exe));
    println!("args = ['mcp', '--project', {}]", toml_str(&proj));
    println!();
    let json = serde_json::json!({
        "mcpServers": {
            "oma": { "command": exe, "args": ["mcp", "--project", proj] }
        }
    });
    println!("# 通用 mcpServers（JSON 配置的消费端）:");
    println!("{json}");
    Ok(())
}

/// --json 出口：信封进 stdout（机器面），业务失败先吐信封再向上传播退出非 0。
fn print_json(command: &str, root: &Path, outcome: Result<Value, String>) -> Result<(), String> {
    let env = oma::api::envelope(command, root, outcome);
    let text = serde_json::to_string_pretty(&env).map_err(|e| e.to_string())?;
    println!("{text}");
    match env.get("ok").and_then(|v| v.as_bool()) {
        Some(true) => Ok(()),
        _ => Err(env
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("command failed")
            .to_string()),
    }
}

/// completions：clap_complete 生成，stdout 直接吐脚本。
fn cmd_completions(shell: clap_complete::Shell) -> Result<(), String> {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "oma", &mut std::io::stdout());
    Ok(())
}

async fn cmd_spawn(
    wanted: Option<Vec<String>>,
    stub: bool,
    project: Option<PathBuf>,
    json: bool,
) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        // 三通道同语义单点（Round3 grok1/codex7：json 分支此前手写副本且
        // 顺序与 api 层分叉）。
        let mut out = oma::api::spawn(&root, wanted, stub).await;
        if let Ok(v) = &mut out {
            oma::api::spawn_finalize(&root, v).await;
        }
        return print_json("spawn", &root, out);
    }
    println!("spawn.project={}", root.display());
    println!("spawn.stub={stub}");
    // 先验 plan 再 connect（Round3 claude3 遗留：`--agents typo` 会先拉起
    // daemon 与 boot keeper 会话再报错，残留到下次成功 spawn；api 通道
    // 本来就是这个顺序）。
    let plan = orch::plan_agents(wanted, stub)?;
    let link = orch::connect(&root, true).await?;
    println!("spawn.label={}", link.label);
    let out = orch::reconcile(&link, &root, &plan).await?;
    println!("spawn.session={}", orch::session_name(&root)?.as_str());
    println!("spawn.attached={}", out.attached.join(","));
    println!("spawn.respawned={}", out.respawned.join(","));
    println!("spawn.removed={}", out.removed.join(","));
    println!(
        "spawn.mode={}",
        if out.attached.is_empty() {
            "new"
        } else {
            "reconcile"
        }
    );
    // 收尾单点（readiness → auto-settle；Round3 grok1：与三通道同序）。
    let mut v = serde_json::json!({ "respawned": out.respawned });
    oma::api::spawn_finalize(&root, &mut v).await;
    if let Some(alerts) = v["alerts"].as_array() {
        for a in alerts {
            eprintln!("spawn.alert={}", a.as_str().unwrap_or_default());
        }
    }
    println!("spawn.waited=readiness+settle");
    println!("spawn.ok=true");
    Ok(())
}

async fn cmd_status(project: Option<PathBuf>, json: bool) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        return print_json("status", &root, oma::api::status(&root).await);
    }
    // 双读者（S016）：TTY 走人读表格，管道与测试走 marker 行。
    if std::io::stdout().is_terminal() {
        let link = orch::connect(&root, false).await?;
        let (panes, warning) = orch::status(&link, &root).await?;
        println!(
            "oma status: {} (session {})",
            root.display(),
            orch::session_name(&root)?.as_str()
        );
        if let Some(w) = warning {
            println!("warning: {w}");
        }
        println!();
        print!("{}", oma::repl::render_status_table(&panes));
        return Ok(());
    }
    println!("status.project={}", root.display());
    println!("status.session={}", orch::session_name(&root)?.as_str());
    let link = orch::connect(&root, false).await?;
    let (panes, warning) = orch::status(&link, &root).await?;
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
    if let Some(w) = warning {
        println!("status.warning={w}");
    }
    println!("status.ok=true");
    Ok(())
}

/// `oma task <agent> "<文本>"`：建任务目录、发任务、阻塞等产物（DONE
/// 标记协议）。后台形态：shell 后台跑本命令（&），产物落盘后进程自退。
async fn cmd_task(
    agent: Option<String>,
    text: Option<String>,
    timeout: u64,
    project: Option<PathBuf>,
) -> Result<(), String> {
    let root = project_root(project)?;
    let (agent, text) = match (agent, text) {
        (Some(a), Some(t)) => (a, t),
        _ => return Err(
            "usage: oma task <agent> \"<text>\" [--timeout N] | oma task list | oma task show <id>"
                .into(),
        ),
    };
    let (id, dir) = oma::task::task_new(&root, &agent, &text).await?;
    println!("task.id={id}");
    println!("task.agent={agent}");
    println!("task.dir={}", dir.display());
    println!(
        "task.waiting=done-marker timeout={}s",
        if timeout == 0 { 0 } else { timeout.min(86_400) }
    );
    match oma::task::task_wait(&root, &id, timeout) {
        Ok(output) => {
            println!("task.done={id}");
            println!("--- output.md ---");
            print!("{output}");
            Ok(())
        }
        Err(e) => {
            eprintln!("oma: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_task_list(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    for (id, agent, done) in oma::task::task_list(&root)? {
        println!("task.list.{id}.agent={agent}");
        println!("task.list.{id}.done={done}");
    }
    Ok(())
}

fn cmd_task_show(id: String, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let meta = oma::task::task_show(&root, &id)?;
    println!("task.show.{id}.agent={}", meta.agent);
    println!("task.show.{id}.created={}", meta.created);
    // marker 行单行契约（Round3 codex5）：多行提示词压成单行（换行转义）。
    let one_line = meta.text.replace('\n', "\\n");
    println!("task.show.{id}.text={one_line}");
    let dir = root.join(".ohmyagents").join("tasks").join(&id);
    println!("task.show.{id}.done={}", dir.join("DONE").exists());
    match std::fs::read_to_string(dir.join("output.md")) {
        Ok(output) => {
            println!("--- output.md ---");
            print!("{output}");
        }
        Err(_) => println!("task.show.{id}.output=pending"),
    }
    Ok(())
}

/// `oma agents statusline [名]`：配置 claude/codex 状态栏（幂等）。
fn cmd_agents_statusline(names: Vec<String>) -> Result<(), String> {
    let home = install::oma_home()?;
    let supported = ["claude", "codex", "kimi", "grok"];
    let do_all = names.is_empty();
    let unknown: Vec<String> = names
        .iter()
        .filter(|n| !supported.contains(&n.as_str()))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "statusline supports claude/codex/kimi/grok only: {}",
            unknown.join(",")
        ));
    }
    if do_all || names.iter().any(|n| n == "claude") {
        let p = oma::statusline::merge_claude(&home)?;
        println!("statusline.claude={p}");
    }
    if do_all || names.iter().any(|n| n == "codex") {
        let p = oma::statusline::merge_codex(&home)?;
        println!("statusline.codex={p}");
    }
    if do_all || names.iter().any(|n| n == "kimi") {
        let p = oma::statusline::merge_kimi(&home)?;
        println!("statusline.kimi={p}");
    }
    if do_all || names.iter().any(|n| n == "grok") {
        let p = oma::statusline::merge_grok(&home)?;
        println!("statusline.grok={p}");
    }
    // The bar renders through pwsh on every platform; without it the merged
    // config is inert. Advisory, never fatal (P0027).
    if oma::statusline::pwsh_on_path() {
        println!("statusline.pwsh=found");
    } else {
        println!("statusline.pwsh=missing");
        println!("statusline.warn=pwsh-not-on-path-statusline-will-not-run");
    }
    println!("statusline.ok=true");
    Ok(())
}

/// `oma agents providers [--example]`：提供商别名簿（列出 + 模板）。
fn cmd_agents_providers(example: bool) -> Result<(), String> {
    if example {
        println!("{}", oma::providers::EXAMPLE_TOML.trim_end());
        return Ok(());
    }
    let path = oma::providers::store_path()?;
    println!("providers.store={}", path.display());
    let book = oma::providers::load()?;
    let aliases = oma::providers::aliases(&book);
    if aliases.is_empty() {
        println!("providers.defined=0");
        println!("providers.hint=oma agents providers --example 查看模板；spawn 用 --agents claude@zhipu 注入");
        return Ok(());
    }
    for alias in aliases {
        let provider = book.providers.get(&alias).unwrap();
        for (agent, launch) in &provider.agents {
            // 只列键名不列值——密钥值即使误配明文也不回显。
            println!(
                "providers.entry={alias}.{agent} env_keys={} argv={} env_keys_list={}",
                launch.env.len(),
                launch.argv.len(),
                launch.env.keys().cloned().collect::<Vec<_>>().join(",")
            );
        }
    }
    println!("providers.hint=spawn --agents claude@zhipu,codex@deepseek 按别名注入 env/argv");
    Ok(())
}

/// `oma key`：发单键的受守卫入口（裸 rmux CLI 绕守卫曾实杀一路 codex，
/// 2026-09-01）。codex 的 C-c 被拒并给出替代建议。
async fn cmd_key(agent: String, key: String, project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    match oma::api::key(&root, &agent, &key).await {
        Ok(_) => {
            println!("key.agent={agent}");
            println!("key.sent={key}");
            println!("key.ok=true");
            Ok(())
        }
        Err(e) => {
            eprintln!("oma: {e}");
            std::process::exit(1);
        }
    }
}

async fn cmd_send(
    agent: String,
    text: String,
    confirm: Option<String>,
    project: Option<PathBuf>,
    json: bool,
) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        return print_json(
            "send",
            &root,
            oma::api::send(&root, &agent, &text, confirm.as_deref()).await,
        );
    }
    let link = orch::connect(&root, false).await?;
    orch::send(&link, &root, &agent, &text, confirm.as_deref()).await?;
    // 锁外开始确认（Round2 grok1/kimi2：拆分后 CLI 文本路径漏接，最常用
    // 通道反而丢了 send.alert）。
    orch::send_start_alerts(&link, &root, &agent).await;
    println!("send.ok=true");
    Ok(())
}

async fn cmd_cleanup(project: Option<PathBuf>, json: bool) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        return print_json("cleanup", &root, oma::api::cleanup(&root).await);
    }
    let link = orch::connect(&root, false).await?;
    let existed = orch::cleanup(&link, &root).await?;
    println!("cleanup.killed={existed}");
    println!("cleanup.scope=session");
    println!("cleanup.ok=true");
    Ok(())
}

async fn cmd_settle(wait: u64, project: Option<PathBuf>, json: bool) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        return print_json("settle", &root, oma::api::settle(&root, wait).await);
    }
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
    json: bool,
) -> Result<(), String> {
    let root = project_root(project)?;
    if json {
        // 全路被门挡：api::run 直接 Err（relay6 grok1 裁决），信封
        // ok:false、退出非 0，与文本通道、R002 契约一致。
        return print_json(
            "run",
            &root,
            oma::api::run(&root, &text, assign, confirm.as_deref()).await,
        );
    }
    let link = orch::connect(&root, false).await?;
    let outcome = orch::run(&link, &root, &text, assign, confirm.as_deref()).await?;
    if outcome.sent.is_empty() {
        // 全拦即失败（R002 契约；relay6 grok1 裁决源头在 api 层统一）。
        for (agent, reason) in &outcome.skipped {
            eprintln!("run.skipped={agent}:{reason}");
        }
        return Err("every lane gated; nothing dispatched".into());
    }
    println!("run.task.id={}", outcome.task_id);
    println!("run.sent={}", outcome.sent.join(","));
    for (agent, reason) in &outcome.skipped {
        println!("run.skipped={agent}:{reason}");
    }
    // 锁外开始确认（同 cmd_send，Round2 补接）。
    for agent in &outcome.sent {
        orch::send_start_alerts(&link, &root, agent).await;
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
            Ok(install::InstallOutcome::Installed {
                version,
                probed,
                path,
            }) => {
                println!("install.{name}.status=installed version={version}");
                match &probed {
                    Some(v) => println!("install.{name}.probe={v}"),
                    None => {
                        // 失败才补分类（S021）：illegal-instruction 即指令集不匹配。
                        let kind = oma::caps::classify_probe_exit(
                            std::process::Command::new(&path)
                                .arg("--version")
                                .output()
                                .ok()
                                .and_then(|o| o.status.code()),
                        );
                        println!("install.{name}.probe=unavailable({kind})");
                    }
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
        TraceCmd::Timeline {
            agent,
            file,
            limit,
            project,
        } => {
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
        TraceCmd::Search {
            query,
            agent,
            limit,
            project,
        } => {
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
        TraceCmd::File {
            file,
            agent,
            limit,
            project,
        } => {
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
        TraceCmd::Blocks {
            agent,
            limit,
            project,
        } => {
            let project = resolve(project);
            print_block_timeline(&project, agent.as_deref(), limit);
            Ok(())
        }
        TraceCmd::Agent {
            name,
            limit,
            project,
        } => {
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

fn cmd_hook(event: Option<String>, agent: Option<String>) -> Result<(), String> {
    match hook::run(event.as_deref(), agent.as_deref()) {
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
    let raw = match project {
        Some(p) => p,
        None => std::env::current_dir().map_err(|e| format!("cwd: {e}"))?,
    };
    // 相对路径必须在此展开为绝对（M031 二犯）：cwd/working_directory 传进
    // rmux 后由 **daemon 侧**解析——WMI 起的 daemon 在 System32，`"."` 会被
    // 解析成 C:\Windows\System32，四路 agent 全部落在系统目录（项目级
    // yolo/hook/AGENTS.md 全不加载，claude 挂权限框、hook 全静默）。
    if raw.is_relative() {
        let cwd = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
        return Ok(cwd.join(raw));
    }
    Ok(raw)
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
        println!("init.hooks.skipped.count={}", deployed.skipped.len());
        // Cross-environment marker (P0027): bare hooks resolve `oma` through
        // each OS's own PATH and survive shared project dirs.
        if let Some(form) = deployed.form {
            println!("init.hooks.form={form}");
        }
        for w in &deployed.warns {
            println!("init.hooks.warn={w}");
        }
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
