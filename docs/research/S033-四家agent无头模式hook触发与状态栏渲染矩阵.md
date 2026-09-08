# 四家 agent 无头模式 hook 触发与状态栏渲染矩阵

- 日期：2026-09-08
- 关联：PRD D17 第 2 轮前置研究；S025（状态栏矩阵）、M045（codex 无外部命令面）
- 方法：官方文档与官方仓库 issue/changelog 抓取取证（explore 子代理执行）

## 汇总表

| agent | 无头入口 | 无头 hook 触发 | 状态栏无头可验否 |
| --- | --- | --- | --- |
| claude | `claude -p "<prompt>"` | 是（现行版；`--bare` 跳过；2026 上半年版本 per-turn hook 有已知缺口，约 2.1.89 起修复） | 否（仅 TUI 底栏；无官方无头途径，只能手工喂 mock JSON 跑脚本） |
| codex | `codex exec "<prompt>"` | 有条件是（须 TUI 信任或 `--dangerously-bypass-hook-trust`；0.13x 至 0.14x 有多个派发 bug，需钉版本实测；`notify` 在 exec 确认触发） | 否（`tui.status_line` 只有内置项 ID，本就无外部命令） |
| grok | `grok -p "<prompt>"`（`--single`） | 是（官方明示 headless 下 hooks still apply；逐事件矩阵未列，建议实测） | 否（状态栏是 pager/TUI 行，无无头途径） |
| kimi | `kimi -p "<prompt>"`（`--prompt`） | 是（print 与 TUI 同引擎链路派发；唯独 SessionEnd 在 print 不触发，print 退出只 dispose 不走 close） | 否（`[status_line].command` 只替换 TUI footer；`kimi doctor` 仅校验配置不渲染） |

## 逐家取证

### claude

- 无头入口 `claude -p`（--print），配 `--output-format text|json|stream-json` [实证： code.claude.com/docs/en/headless]。
- 无头 hook：现行版本触发；官方明示「Without `--bare`, a `-p` session runs the hooks in a project's `.claude/settings.json`」；stream-json 输出含 SessionStart/Setup hook 的 hook_started/hook_progress/hook_response 事件；SIGTERM 跑 SessionEnd [实证： 同源]。限定：`--bare` 显式跳过 hooks/skills/MCP [实证： 同源]；2026 上半年 issue #40506 / #71022 报告 `-p` 下 per-turn hook 不触发（机器人关闭非确认修复），CHANGELOG 2.1.89 起有 headless hook defer 条目 [推断： 约 2.1.89 起修复，版本分界需实测钉死]。
- 状态栏：statusLine 定义即界面底栏，`-p` 无界面不渲染；官方建议的验证姿势是手工 `echo '<mock JSON>' | ./statusline.sh` 绕过 Claude 直跑脚本 [实证： docs.claude.com/en/docs/claude-code/statusline]。
- TUI 渲染外部命令确认：statusLine type=command，stdout 首行成栏文本，JSON 走 stdin，300ms 更新上限 [实证： 同源]。

### codex

- 无头入口 `codex exec "<prompt>"`（子命令），配 `--json` 事件流 [实证： developers.openai.com/codex/noninteractive]。
- hook：体系完整（13 事件，hooks.json 或 config.toml [hooks]，[features].hooks 开关）[实证： developers.openai.com/codex/hooks]。exec 下有信任门槛：非托管 hook 须 trusted_hash，信任提示只在 TUI 出现，exec 下未信任 hook 静默不跑；官方旁路 `--dangerously-bypass-hook-trust` [实证： 同源]。bug 史：0.136 至 0.138 bypass 旗标也不派发（issue #26383 / #26452 open）、0.142.5 hooks 不发现回归（#30835 open）、#32491 称 exec 跳过已信任项目 hooks 除非传 bypass [推断： 当前版本 exec 加 bypass 可触发，须钉版本实测]。`notify` 命令 exec 下确认触发（issue #18309 反向证据）[实证]。
- 状态栏：`tui.status_line` 是 TUI footer 内置项 ID 列表，无外部命令（M045 复核确认），exec 无 footer [实证： developers.openai.com/codex/config-reference]。

### grok

- 无头入口 `grok -p`（--single），配 `--output-format` / `--always-approve` / `--max-turns` / `--cwd` [实证： grok-build 仓 https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/14-headless-mode.md]。
- hook：headless 文档明示「Deny rules, hooks, and admin locks still apply」 [实证： 同源]；事件面 13 个（含 PostToolUseFailure / PreCompact / PostCompact），Claude 形态 JSON [实证： https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/10-hooks.md]；逐事件无头矩阵未列 [推断： 建议实测]。
- 状态栏：定义为 pager 底部可选行，渲染时机全是 TUI 事件；headless 无输出途径 [实证： https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/25-status-line.md]。TUI 渲染外部命令确认：stdin 喂 JSON、stdout 画栏、最多 5 行、ANSI 支持、10s 超时 [实证： 同源]。

### kimi

- 无头入口 `kimi -p`（--prompt），配 `--output-format stream-json` / `--yolo` [实证： kimi.com/code docs config-files 节]；`-p` 模式硬编码自动批准工具调用 [实证： 同源]。
- hook：print 模式触发，issue #2779 反向证据（0.34.0 回归是 TUI 不触发而 `-p` 正常），第三方核实 0.30.0 上 config.toml `[[hooks]]` print 模式 fire fine（plugin 来源 hooks 不加载）[实证： MoonshotAI/kimi-code#2779 加 docs.little-loops.ai/kimi/hook-events]。事件面 13+（含 SessionHeartbeat）[实证： kimi.com/code docs hooks 节]。源码补强后修正两处：plugin hooks「不加载」系旧版行为，现版 main 无模式门控同样加载；SessionEnd 在 print 不触发（见末节源码补强）。
- 状态栏：`tui.toml [status_line].command` stdout 首行替换 TUI footer，stdin 传 JSON 快照（model/cwd/git/权限模式/plan/context 用量/session id/version），300ms 上限、1s 节流、失败回落内置布局、`/reload-tui` 热载 [实证： config-files 文档 tui.toml 节]；print 模式无 footer 不渲染 [推断]；`kimi doctor` 仅校验配置合法性不输出状态栏 [实证： 同源]。

## 对 D17 的含义

[推断： 由汇总表直接得出]

1. 状态栏四家都无法无头拿到渲染输出，验收只能两层分离：脚本本体手工喂 mock JSON 直跑断言（四家官方均支持此姿势），TUI 真实渲染归交互式冒烟。
2. hook 面无头可验三家（claude / grok / kimi）；codex 须先过信任闸（bypass 旗标）并钉住无派发 bug 的版本。
3. 「状态栏必须经 agent TUI 渲染才算数」这条判据路线在无头验收里不成立，若坚持 TUI 渲染，D17 就不能叫无头验收，二者互斥需用户裁定。

## 源码补强（2026-09-08，用户点拨三家开源可读源码）

> 浅克隆 main 逐文件取证，三路并行子代理执行；claude 闭源维持文档层结论。行号对应当日 main。

### codex（openai/codex，Rust）

- exec 与 TUI 共享 core turn 循环（exec 起 in-process app-server）：`exec/src/lib.rs:1127` [实证]。逐事件派发点：SessionStart `core/src/session/turn.rs:287`、UserPromptSubmit `turn.rs:693`、PreToolUse `core/src/tools/registry.rs:568`、PostToolUse `registry.rs:684`、Stop `turn.rs:554`、SessionEnd `core/src/session/handlers.rs:431`（root-only）、Interrupt `hook_runtime.rs:486` [实证]。exec e2e 只钉 SessionStart（`exec/tests/suite/hooks.rs:8`），其余事件由同环路推出 [推断]。
- 信任闸单点 `hooks/src/engine/discovery.rs:713`：enabled 加（bypass 或 Managed/Trusted）才进 handlers；未信任 hook 静默跳过且 exec 无任何用户可见提示 [实证]。第二道闸：项目目录信任，未信任层 config loaded but disabled（`config/src/loader/mod.rs:118`）[实证]。
- bypass 丢失 bug 已修：PR #26434（commit d007b0852 在 main），exec 重建 thread 时转发 override（`exec/src/lib.rs:1400`）；#26383 / #30835 / #32491 仍 OPEN [实证： gh API]。
- `tui.status_line` 是 30 个内置项封闭枚举（`tui/src/bottom_pane/status_line_setup.rs:56-158`），无外部命令 variant，非法 ID 仅告警一次 [实证]。

### kimi（MoonshotAI/kimi-code，TypeScript）

- print 与 TUI 共用 agent-core-v2 DI app（`apps/kimi-code/src/cli/v2/run-v2-print.ts:166` bootstrap nonInteractive），ExternalHooksFeature 模块副作用注册、两入口同载 [实证]。
- 逐事件 [实证]：SessionStart（`features/externalHooks/app/sessionExternalHooksService.ts:63`）、UserPromptSubmit（`agentExternalHooksService.ts:196`，可 block）、PreToolUse/PostToolUse/PostToolUseFailure（`:163-178`）、Stop（`:238`，可续跑）、Notification/PreCompact/PostCompact/SubagentStart/SubagentStop 等同链路；SessionHeartbeat 60s unref 定时器，`-p` 超 60s 才触发。
- SessionEnd print 不触发 [实证]：只在显式 close()/archive() 发出（`sessionLifecycleService.ts:414`），print 清理只 `app.dispose()`（`run-v2-print.ts:234`）绕过 close；TUI 退出走 closeSession 才触发。
- plugin hooks 现版 print 同样加载 [实证]：`pluginService.ts:239` enabledHooks 触发 loadOnce，加载链无任何 print/headless 门控（全仓 nonInteractive 仅命中权限策略）；S033 文档层「不加载」结论证伪（旧版行为）。
- #2779 回归至今未修：issue 与修复 PR #2822 均 OPEN；main 上 runner 仍只在构造与 plugin reload 时建索引（`externalHooksRunnerService.ts:35-39`），无 config-change 订阅 [实证]。
- `-p` 自动批准：`run-v2-print.ts:482` 强制 setMode('auto') 加 nonInteractive 剔除危险命令询问策略（`permissionPolicyService.ts:42`）[实证]。
- 状态栏纯 TUI footer 第 1 行 [实证]：loadTuiConfig 调用点仅 TUI 入口；执行器 `tui/utils/status-line-command.ts:30-115`（sh/cmd -c 起、stdin 10 字段 JSON、stdout 首行、300ms 超时、1s 节流、失败回落内置布局）；渲染点唯一 `tui/components/chrome/footer.ts:238-314`；print 链路零引用。

### grok（xai-org/grok-build，Rust，rev a549186d）

- ACP 双进程结构，headless 进程内拉起同一 agent（`xai-grok-pager/src/headless.rs:863`），hook 派发全在 agent 侧会话 actor，与 headless 无关 [实证]。
- 全部 13 事件 headless 与 TUI 完全同路、无 headless 门禁 [实证]：spawn 无条件 discover_hooks（`xai-grok-shell/src/session/acp_session_impl/spawn.rs:1321`）；SessionStart 在 mark_headless 之前发出（`agent_ops.rs:4922`）；SessionEnd `run_loop.rs:54`；PreToolUse `tool_calls.rs:1242`；PostToolUse/Failure `hook_dispatch.rs:291/375`；Stop/SubagentStop `stop_gate.rs:260`；PreCompact/PostCompact `compaction.rs:957/1861` 等。headless 命中逐条核对无一处 hook 短路。
- hook 源：`~/.grok/hooks/*.json` 加 hooks-paths 注册加三层 TOML hooks 表（requirements/managed/config.toml，权威序，`xai-grok-config/src/loader.rs:319`）；项目源受 folder trust 门禁 [实证]。
- 「Deny rules, hooks, admin locks still apply」逐条坐实：CLI deny 注入 agent 配置（`headless.rs:834`）、hook 先于权限系统求值、spawn 查 yolo_policy_lock（`spawn.rs:338`）[实证]。
- 状态栏 `type=command`：stdin JSON envelope、stdout 前 5 行加 64KiB 上限、10s 硬超时、refresh_interval 1 至 86400s 仅 command 型 [实证：`xai-grok-pager/src/app/status_line/command.rs:13/229/338` 等]；渲染与执行子系统全在 pager AppView 内，headless 无 status_line 任何命中与旁路（capability 协商默认关，headless 从不发）[实证]。

### 补强后的 D17 判据底座

[推断： 由三路源码证据直接得出]

1. hook 无头可验矩阵最终形：claude 全事件（现行版，文档层）、codex 全事件（须 bypass 旗标，e2e 仅钉 SessionStart）、grok 全 13 事件（源码钉死无门禁）、kimi 除 SessionEnd 外全事件（SessionEnd 须归 TUI 或不纳入无头判据）。
2. 状态栏无头验收四家全部只能脚本直跑（mock JSON 喂脚本断言 stdout），无一例外；TUI 渲染只能交互式冒烟。
3. codex 信任闸 exec 下静默跳过无任何提示，验收脚本必须显式带 `--dangerously-bypass-hook-trust`，否则假绿。
