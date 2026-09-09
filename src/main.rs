use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::Value;

use oma::agents;
use oma::doctor;
use oma::hook;
use oma::install;
use oma::trace;
use oma::yolo;

#[derive(Parser)]
#[command(
    name = "oma",
    version,
    about = "Oh My Agents：Agent 全平台 token、hook 与状态栏部署配置工具"
)]
struct Cli {
    /// JSON 信封输出（--format json 简写）
    #[arg(long, global = true, conflicts_with = "format")]
    json: bool,
    /// 输出格式：kv（marker 行，缺省）| json（信封）| jsonl（列表逐行对象）
    #[arg(long, global = true)]
    format: Option<String>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// 项目级 yolo 落盘（默认全套：yolo 键加 hook/skill 注册）
    Init {
        /// 写项目级无阻塞键（仅 yolo，不落 hook/skill）
        #[arg(long)]
        yolo: bool,
        /// 预写用户家目录信任库（claude/codex/kimi/grok）
        #[arg(long)]
        pretrust: bool,
        /// 项目根；默认当前目录
        #[arg(long)]
        project: Option<PathBuf>,
    },
    /// 只读诊断 yolo / 信任 / 二进制 / state / 登录态 / hook 形态 / 状态栏
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
    /// Agent hook 入口：读事件写 `.oma/state`。用户手拉会话按 payload cwd 回退写项目状态文件
    Hook {
        /// 事件名或四态（idle/working/blocked/unknown）；省略则读 stdin JSON
        event: Option<String>,
        /// agent 名（注册参数注入；回退写项目状态文件用）
        #[arg(long)]
        agent: Option<String>,
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
    /// 项目内四家 agent 对话历史检索（六视图联邦读，P0013/P0014；D19 恢复）
    Trace {
        #[command(subcommand)]
        cmd: TraceCmd,
    },
}

#[derive(Subcommand)]
enum SelfSub {
    /// oma 自更新：dev 滚动源或 latest 正式版自替换；封版前用 --git 源码安装
    Update {
        /// 仓库（owner/name）；缺省 raystyle/ohmyagents-rs
        #[arg(long)]
        repo: Option<String>,
        /// 走正式稳定通道（releases/latest，封版 tag 触发）；缺省 dev 滚动源
        #[arg(long)]
        stable: bool,
        /// 走 cargo install --git 源码安装（封版前主路径）
        #[arg(long)]
        git: bool,
        /// 同版本也重装
        #[arg(long)]
        force: bool,
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
enum SecretsCmd {
    /// 链路初始化：app.key 生成 + age 身份包裹进 identity.enc + meta 落盘
    Init,
    /// 写入一个键（值走 stdin，秘密不进 argv；base64 后 sops 加密入 vault）
    Set {
        /// 键名（A-Z0-9_，如 DEEPSEEK_API_KEY）
        name: String,
    },
    /// 解 vault 出对应 shell 的会话 env 语句（profile 块的后端）
    Env {
        /// 目标 shell：pwsh | bash | zsh | nu
        #[arg(long)]
        shell: String,
    },
    /// 向 shell profile 写懒注入块（幂等，标志行包裹）
    Inject {
        /// 目标 shell：pwsh | bash | zsh | nu；缺省四个都写
        shells: Vec<String>,
    },
    /// 链路体检（redacted：只报存在性）
    Status,
}

#[derive(Subcommand)]
enum AgentsCmd {
    /// 配置四家状态栏（幂等：claude/codex/kimi/grok 各自配置面，脚本随 oma 释放）
    Statusline {
        /// 指定 agent（claude/codex/kimi/grok）；缺省四家都配
        names: Vec<String>,
        /// 打印 ~/.oma/statusline.toml 定制示例模板后退出（D18）
        #[arg(long)]
        example: bool,
    },
    /// 引导设备码登录（grok/kimi）：转发 URL 加 code 给用户，等浏览器侧完成
    Login {
        /// agent 名（grok/kimi）
        names: Vec<String>,
        /// 等浏览器侧完成的最长秒数；0 不限时
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
    /// 密钥一钥两密文存储与四 shell 懒注入（S031，对齐 ohmycloud D20 与 ohmypwsh 懒注入）
    Secrets {
        #[command(subcommand)]
        cmd: Option<SecretsCmd>,
    },
    /// 提供商别名簿（~/.oma/providers.toml，标准 sops 托管）
    Providers {
        /// 打印示例模板（含 sops 托管说明）后退出
        #[arg(long)]
        example: bool,
    },
    /// 安装缺失的 agent（已 deprecated，D07 迁册 ome：请用 ome install；oma 自管根 ~/.oma；已装任何来源即跳过）
    Install {
        /// agent 名列表；缺省 = catalog 全部的缺失者
        names: Vec<String>,
        /// 已装也重装（oma 自管根）
        #[arg(long)]
        force: bool,
        /// 自定义 oma 应用数据根；缺省 OMA_HOME 环境变量或 ~/.oma
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// 四家 hook 与状态栏全平台无头验收（D17）：状态栏脚本直跑加 hook 无头落盘，任一非跳过项失败退出 1
    Verify {
        /// 指定 agent（claude/codex/grok/kimi）；缺省四家全验
        names: Vec<String>,
        /// 单家无头会话最长秒数（超时杀进程不算失败，判据只看 state 落盘）
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// 解析最新版并升级 oma 自管安装（已 deprecated，D07 迁册 ome：agent 升级归 ome），取证 sha256 后写回用户本地 pin
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
        oma::fmtio::error_exit(e);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    oma::fmtio::init(cli.json, cli.format.as_deref())?;
    let Some(command) = cli.command else {
        // 裸 oma：无编排面后没有默认动作，打印帮助退出。
        let mut cmd = Cli::command();
        cmd.print_help().map_err(|e| e.to_string())?;
        println!();
        return Ok(());
    };
    match command {
        Commands::Init {
            yolo,
            pretrust,
            project,
        } => cmd_init(yolo, pretrust, project),
        Commands::Doctor { project } => cmd_doctor(project),
        Commands::Agents { cmd } => match cmd {
            None => {
                // 结构化三态（issue #1 契约）：json 信封；jsonl 逐 agent 行。
                match oma::fmtio::mode() {
                    oma::fmtio::Format::Json => {
                        let rows: Vec<Value> =
                            agents::detect().iter().map(agent_report_row).collect();
                        let v = serde_json::json!({
                            "installed": rows.iter().filter(|r| r["status"] == "installed").count(),
                            "missing": rows.iter().filter(|r| r["status"] == "missing").count(),
                            "agents": rows,
                        });
                        let cwd = std::env::current_dir().unwrap_or_default();
                        print_json("agents", &cwd, Ok(v))?;
                    }
                    oma::fmtio::Format::Jsonl => {
                        let rows: Vec<Value> =
                            agents::detect().iter().map(agent_report_row).collect();
                        oma::fmtio::print_jsonl(&rows);
                    }
                    oma::fmtio::Format::Kv => agents::print_reports(&agents::detect()),
                }
                Ok(())
            }
            Some(AgentsCmd::Install { names, force, root }) => {
                cmd_agents_install(names, force, root)
            }
            Some(AgentsCmd::Update { names, force, root }) => cmd_agents_update(names, force, root),
            Some(AgentsCmd::Statusline { names, example }) => cmd_agents_statusline(names, example),
            Some(AgentsCmd::Verify { names, timeout }) => cmd_agents_verify(names, timeout),
            Some(AgentsCmd::Login { names, timeout }) => cmd_agents_login(names, timeout),
            Some(AgentsCmd::Secrets { cmd }) => cmd_agents_secrets(cmd),
            Some(AgentsCmd::Providers { example }) => cmd_agents_providers(example),
        },
        Commands::Hook { event, agent } => cmd_hook(event, agent),
        Commands::SelfGroup { cmd } => match cmd {
            SelfSub::Update {
                repo,
                stable,
                git,
                force,
            } => oma::update::run(
                &repo.unwrap_or_else(|| oma::update::DEFAULT_REPO.into()),
                if stable {
                    oma::update::Channel::Latest
                } else {
                    oma::update::Channel::Dev
                },
                git,
                force,
            ),
        },
        Commands::Completions { shell } => cmd_completions(shell),
        Commands::Trace { cmd } => cmd_trace(cmd),
    }
}

/// --json 出口：信封进 stdout（机器面），业务失败先吐信封再向上传播退出非 0。
fn print_json(command: &str, root: &Path, outcome: Result<Value, String>) -> Result<(), String> {
    let env = oma::fmtio::envelope(command, root, outcome);
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

/// `oma agents login [名]`：设备码登录引导（grok/kimi，S026）——转发
/// URL 加 code、等浏览器侧完成、以落盘凭据确认。
fn cmd_agents_login(names: Vec<String>, timeout: u64) -> Result<(), String> {
    if names.is_empty() {
        return Err("login needs an agent: oma agents login grok|kimi".into());
    }
    let mut any_failed = false;
    for name in &names {
        let out = oma::login::run(name, timeout)?;
        println!("login.ok={} detail={}", out.ok, out.detail);
        any_failed |= !out.ok;
    }
    if any_failed {
        std::process::exit(1);
    }
    Ok(())
}

/// `oma agents secrets`：一钥两密文存储与四 shell 懒注入（S031）。
fn cmd_agents_secrets(cmd: Option<SecretsCmd>) -> Result<(), String> {
    use SecretsCmd as S;
    let root = oma::install::oma_home()?;
    match cmd {
        None | Some(S::Status) => {
            for line in oma::secrets::status(&root) {
                println!("{line}");
            }
            Ok(())
        }
        Some(S::Init) => {
            for line in oma::secrets::init(&root)? {
                println!("{line}");
            }
            Ok(())
        }
        Some(S::Set { name }) => {
            let value = oma::secrets::read_stdin_value()?;
            for line in oma::secrets::set(&root, &name, &value)? {
                println!("{line}");
            }
            Ok(())
        }
        Some(S::Env { shell }) => {
            print!("{}", oma::secrets::env_lines(&root, &shell)?);
            Ok(())
        }
        Some(S::Inject { shells }) => {
            let targets = if shells.is_empty() {
                vec!["pwsh", "bash", "zsh", "nu"]
            } else {
                shells.iter().map(String::as_str).collect()
            };
            for shell in targets {
                for line in oma::secrets::inject(shell)? {
                    println!("{line}");
                }
            }
            Ok(())
        }
    }
}

/// `oma agents statusline [名] [--example]`：配置四家状态栏（幂等）。
fn cmd_agents_statusline(names: Vec<String>, example: bool) -> Result<(), String> {
    if example {
        println!("{}", oma::statusline::EXAMPLE_TOML.trim_end());
        return Ok(());
    }
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

/// `oma agents verify [名...] [--timeout N]`：四家 hook 与状态栏无头验收（D17）。
/// 缺省四家全验；skip（未装）不算失败，任一非跳过项失败退出 1。
fn cmd_agents_verify(names: Vec<String>, timeout: Option<u64>) -> Result<(), String> {
    let outcomes = oma::verify::run(&names, timeout.unwrap_or(oma::verify::DEFAULT_TIMEOUT_SECS))?;
    for line in oma::verify::render(&outcomes) {
        println!("{line}");
    }
    if oma::verify::any_fail(&outcomes) {
        std::process::exit(1);
    }
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
        println!("providers.hint=oma agents providers --example 查看模板");
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
    Ok(())
}

fn cmd_agents_install(
    names: Vec<String>,
    force: bool,
    root: Option<PathBuf>,
) -> Result<(), String> {
    // D07 迁册（ohmyagents#5）：agent 二进制下装部署归 ome，本命令 deprecated
    // 但保留兼容——提示走 stderr，不污染 stdout 的 kv/json 输出面（R011）。
    eprintln!(
        "oma.deprecated=agents install moved to ome (D07); use: ome install <agent>; this command still works"
    );
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
    // D07 迁册：升级通道语义由 ome 裁决（ohmyagents#2 余项），提示先指向 ome install。
    eprintln!(
        "oma.deprecated=agents update moved to ome (D07); use: ome install <agent> (update channel decided by ome); this command still works"
    );
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

fn cmd_hook(event: Option<String>, agent: Option<String>) -> Result<(), String> {
    match hook::run(event.as_deref(), agent.as_deref()) {
        Ok(outcome) => {
            if let Some(path) = outcome.state_file {
                if std::env::var_os("OMA_HOOK_VERBOSE").is_some() {
                    eprintln!("oma.hook.wrote={}", path.display());
                }
            }
            if let Some(g) = outcome.guard {
                if g.block {
                    // exit 2 = agent 侧拒工具调用，stderr 原因回给模型（S030）。
                    eprintln!("oma secretguard: {}", g.reasons.join("; "));
                    std::process::exit(2);
                }
                if std::env::var_os("OMA_HOOK_VERBOSE").is_some() && !g.findings.is_empty() {
                    eprintln!("oma.secretguard.findings={}", g.findings.len());
                }
            }
        }
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
    // 相对路径必须在此展开为绝对（M031）：下游按收到路径原样落盘与注册，
    // 相对路径经工作目录漂移会落错位置。
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

/// agents 检测行 → 结构化对象（字段序与 kv 行序一致，preserve_order）。
fn agent_report_row(r: &agents::Report) -> Value {
    match &r.hit {
        Some(h) => serde_json::json!({
            "agent": r.agent,
            "status": "installed",
            "source": h.source.as_str(),
            "path": h.path.display().to_string(),
            "version": h.version.as_deref().unwrap_or("-"),
            "extras": h.extras.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        }),
        None => serde_json::json!({
            "agent": r.agent,
            "status": "missing",
            "hint": format!("oma agents install {}", r.agent),
        }),
    }
}

fn cmd_doctor(project: Option<PathBuf>) -> Result<(), String> {
    let root = project_root(project)?;
    let d = doctor::diagnose(&root)?;
    let findings: Vec<Value> = d
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "agent": f.agent,
                "check": f.check,
                "status": f.status.as_str(),
                "path": f.path,
                "detail": f.detail,
            })
        })
        .collect();
    // 结构化三态（issue #1 契约）：json 信封；jsonl 逐 finding 行对象；
    // blocked 退出码 1 在三种模式下一致。
    match oma::fmtio::mode() {
        oma::fmtio::Format::Json => {
            let v = serde_json::json!({ "blocked": d.blocked(), "findings": findings });
            print_json("doctor", &root, Ok(v))?;
        }
        oma::fmtio::Format::Jsonl => {
            oma::fmtio::print_jsonl(&findings);
        }
        oma::fmtio::Format::Kv => doctor::print_diagnosis(&d),
    }
    if d.blocked() {
        std::process::exit(1);
    }
    Ok(())
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
