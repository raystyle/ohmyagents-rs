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
enum AgentsCmd {
    /// 配置四家状态栏（幂等：claude/codex/kimi/grok 各自配置面，脚本随 oma 释放）
    Statusline {
        /// 指定 agent（claude/codex/kimi/grok）；缺省四家都配
        names: Vec<String>,
        /// 打印 ~/.oma/statusline.toml 定制示例模板后退出（D18）
        #[arg(long)]
        example: bool,
        /// 部署自备状态栏脚本（D18 整脚本替换；调用契约：首参 agent 名、stdin 喂 agent JSON、stdout 单行）
        #[arg(long, conflicts_with_all = ["example", "builtin"])]
        script: Option<PathBuf>,
        /// 还原内嵌脚本（撤销 --script 的自备替换）
        #[arg(long, conflicts_with = "example")]
        builtin: bool,
    },
    /// 四家 hook 与状态栏全平台无头验收（D17）：状态栏脚本直跑加 hook 无头落盘，任一非跳过项失败退出 1
    Verify {
        /// 指定 agent（claude/codex/grok/kimi）；缺省四家全验
        names: Vec<String>,
        /// 单家无头会话最长秒数（超时杀进程不算失败，判据只看 state 落盘）
        #[arg(long)]
        timeout: Option<u64>,
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
            Some(AgentsCmd::Statusline {
                names,
                example,
                script,
                builtin,
            }) => cmd_agents_statusline(names, example, script, builtin),
            Some(AgentsCmd::Verify { names, timeout }) => cmd_agents_verify(names, timeout),
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

/// `oma agents statusline [名] [--example] [--script 路径] [--builtin]`：
/// 配置四家状态栏（幂等）。--script 部署自备脚本（D18 整脚本替换），
/// --builtin 还原内嵌。
fn cmd_agents_statusline(
    names: Vec<String>,
    example: bool,
    script: Option<PathBuf>,
    builtin: bool,
) -> Result<(), String> {
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
    // 整脚本替换先行（同一部署文件名，后续 merge 指向不变）。
    if let Some(src) = &script {
        oma::statusline::deploy_custom_script(&home, src)?;
    } else if builtin {
        oma::statusline::restore_builtin_script(&home)?;
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
    if let Some(src) = &script {
        println!("statusline.custom=true");
        println!("statusline.script={}", src.display());
    } else if builtin {
        println!("statusline.custom=false");
    } else if oma::statusline::custom_active(&home) {
        println!("statusline.custom=true");
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
            "hint": format!("ome install {}", r.agent),
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
