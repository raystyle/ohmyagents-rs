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
| kimi | `kimi -p "<prompt>"`（`--prompt`） | 是（issue 与第三方核实确认 `[[hooks]]` print 模式触发；plugin 来源 hooks print 模式不加载） | 否（`[status_line].command` 只替换 TUI footer；`kimi doctor` 仅校验配置不渲染） |

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
- hook：print 模式触发，issue #2779 反向证据（0.34.0 回归是 TUI 不触发而 `-p` 正常），第三方核实 0.30.0 上 config.toml `[[hooks]]` print 模式 fire fine（plugin 来源 hooks 不加载）[实证： MoonshotAI/kimi-code#2779 加 docs.little-loops.ai/kimi/hook-events]。事件面 13+（含 SessionHeartbeat）[实证： kimi.com/code docs hooks 节]。
- 状态栏：`tui.toml [status_line].command` stdout 首行替换 TUI footer，stdin 传 JSON 快照（model/cwd/git/权限模式/plan/context 用量/session id/version），300ms 上限、1s 节流、失败回落内置布局、`/reload-tui` 热载 [实证： config-files 文档 tui.toml 节]；print 模式无 footer 不渲染 [推断]；`kimi doctor` 仅校验配置合法性不输出状态栏 [实证： 同源]。

## 对 D17 的含义

[推断： 由汇总表直接得出]

1. 状态栏四家都无法无头拿到渲染输出，验收只能两层分离：脚本本体手工喂 mock JSON 直跑断言（四家官方均支持此姿势），TUI 真实渲染归交互式冒烟。
2. hook 面无头可验三家（claude / grok / kimi）；codex 须先过信任闸（bypass 旗标）并钉住无派发 bug 的版本。
3. 「状态栏必须经 agent TUI 渲染才算数」这条判据路线在无头验收里不成立，若坚持 TUI 渲染，D17 就不能叫无头验收，二者互斥需用户裁定。
