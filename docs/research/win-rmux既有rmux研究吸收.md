# win-rmux既有rmux研究吸收

> 2026-08-29。全量吸收 `D:\sourcecode\win-rmux` 的 rmux 研究：根文档、SKILL、`references/` 全部 14 篇、hooks/scripts、以及上一轮未读的 `.rmux_tasks`。对照本仓定位与 CLI `oma`。不覆盖 win-rmux 原文。本文件取代同日首轮摘要（当时未读命令核验表与 `.rmux_tasks`）。

## 需求

- 研究：win-rmux 里已经写过、测过的 rmux 用法，哪些必须进 `oma`，哪些因定位不同丢弃；与本仓 drive / 状态 / 跨平台 / yolo 四篇对照。
- 核查：
  1. 回车键名是 `Enter` 还是 `C-m`。
  2. `has-session` 在 PowerShell 里是否可靠。
  3. tiny CLI 是否要禁用。
  4. Job Object 下 `new-session -d` 是否报 `os error 5`。
  5. 备屏 TUI 的 capture 是否为空。
  6. 对 Codex 发 `C-c` 是否会退出进程。
  7. `paste-buffer -p` / `load-buffer` 是否在 win-rmux 本机测过。
  8. `send-keys -H` 是否测过。
  9. `D:\ohmyenv\rmux` 是否仍是本机安装路径。
  10. `.rmux_tasks` 里的 hardened-guard 对 `oma` 有没有硬约束。

意图：混合。锚点：`D:\sourcecode\win-rmux`（skill `v0.1.5`，标明 rmux 0.10.0）、`Helvesec/rmux` v0.10.0、本仓已有研究。

## 结论

### 1. 仓库里 rmux 研究实际有哪些

| 层 | 路径 | 角色 |
| --- | --- | --- |
| 协作规则 | `AGENTS.md` | 实测坑摘要；pwsh 7；回归步骤 |
| 产品入口 | `skills/win-rmux/SKILL.md` | 六原语 + 前置守卫 + yolo 表 + 任务原语 |
| 用法研究 | `references/rmux-usage.md` | 2026-08-19 起：daemon 模型、launcher、drive 四步、NO_COLOR |
| 唯一坑表 | `references/troubleshooting.md` | launch / drive / 备屏 / agent / send-keys / 版本 |
| 命令面 | `commands.md` + `command-verification.md` | `list-commands` 全表；2026-08-20 真实 server 实测 |
| 环境 | `environment.md` / `options.md` / `overview.md` | tiny/full/daemon、pipe、默认选项 |
| 扩展 | `extensions.md` / `web-share.md` / `formats.md` / `keybindings.md` | wait-pane、find-panes、`-H`、paste-buffer `-p` |
| 状态 | `hooks.md` + `hooks/win-rmux-agent-state.ps1` + `scripts/install-agent-hooks.ps1` | 全局 hook；`AGENT_STATE_<name>` |
| 任务 | `task-workflows.md` | research / review-cycle；产物在 `.rmux_tasks/` |
| 安全复核 | `.rmux_tasks/skill-review/research/research-claude.md` + `poc-claude/hardened-guard.ps1` | 2026-08-21：kill-server 启发式、全局 hook、全量 User env 拷贝评为 HIGH |

`.reasonix/` 是 Reasonix 会话快照，不含 rmux 用法结论，本轮不吸收。[实证: 2026-08-29 读 `D:\sourcecode\win-rmux` SKILL 与 14 篇 references]

### 2. 必须吸收的内核

win-rmux 把 rmux 用成「一个会话里多 agent pane 的远程驱动」。本仓 CLI 是 `oma`，项目名仍是 ohmyagents，默认不弹 wt，但 Windows 上同一套 daemon 坑都在。

| 项 | win-rmux 口径 | 本仓用法 |
| --- | --- | --- |
| 进程模型 | 公开 `rmux.exe` 是 tiny 分发器；完整实现 `libexec\rmux\rmux.exe`；daemon 是 `libexec\rmux\rmux.exe --__internal-daemon` | 安装不能只拷一个 exe；一律 `RMUX_DISABLE_TINY_CLI=1` |
| 专用 socket | `-S` / `-L`；Windows `\\.\pipe\rmux-...` | 按项目 hash 独立 pipe，见《跨平台与无浏览器》 |
| 会话名唯一 | 禁止复用 `execution-unit`；撞名不静默叠窗格 | `oma` 会话按项目，不往别人的 session split |
| 清理范围 | 默认 `kill-session`；`kill-server` 是最后手段 | `oma cleanup` 只杀本 session |
| 存在性 | `has-session` 命令本身：存在 0、不存在 1；pwsh `if` 包一层不可靠 | `list-sessions -F '#{session_name}'` + 包含判断 |
| Drive | 文本 `-l` 与 `Enter` 分发；`--wait quiet` 超时不等于没发出 | 见 drive 文；Enter 用键名 `Enter` 或 `-H 0d`，不用 `C-m` |
| locate | `pane_current_command` 不可靠；`pane_pid` 反查进程名 | doctor / send 前校验，不匹配则中止 |
| 长 prompt | send-keys 会截断 | 写文件再短指令 `Read <path>` |
| 中文 | send-keys 乱码 | 走 `paste-buffer` UTF-8；win-rmux 当时用 ASCII 将就是权宜 |
| 备屏 | `alternate-screen on`，capture / snapshot 常空 | 结论写文件；镜像走 PTY 流 |
| 状态 | hook 报 idle/working/blocked；Codex Stop 常不触发 | 见《rmux状态判断与hook补充》；CPU 只辅助 |
| Codex `C-c` | 单次即退出应用（2026-08-21 三次崩溃全因此） | 严禁预清 |
| 环境 | `NO_COLOR` 会进 daemon；Codex 沙箱硬编码注入 | spawn 前 `Remove-Item Env:NO_COLOR`，设 `TERM`/`COLORTERM`；PATH 用 Machine+User |
| Job Object | 宿主内 `new-session -d` 报 os error 5 | `oma` 默认后台；从 agent/CI 拉起时再走 wt 退路 |
| `show-environment` | 只显示显式 set-environment，看不见继承的 API key | 不要用它判断密钥是否在 |
| `stream-pane` | 持续阻塞、不自行退出（核验表 TIMEOUT） | 勿前台裸跑 |
| `--wait` | 本版 `--wait` 只支持 `quiet`；`--wait-text` 是独立开关 | 与 extensions.md、troubleshooting 六 一致 |
| `remain-on-exit` | 默认 off，进程退即关 pane | spawn 要 keep-alive，否则 agent 崩了格就没了 |
| `respawn-pane -k` | 对 Codex 实测不稳、布局重排 | 不要靠 respawn 救 Codex pane |
| paste | `paste-buffer -p` 与 `load-buffer` 核验表均为 OK | 与本仓三段式一致，优先走这条，不自包 bracketed paste |
| 默认 shell | `show-options` 写 `cmd.exe`，无命令时新格实测常是 `pwsh.exe` | 始终显式 argv spawn agent，不依赖 default-shell |

### 3. 不要照搬的

| win-rmux | 本仓 |
| --- | --- |
| 默认前台弹 wt，三席上 2 下 1 | `oma` 是编排入口；网页可选观察；不默认 Visible；四路是 2x2，不要抄第三个 `split-window -f -v` |
| 全局安装 agent hook（`~/.codex` / `~/.claude` / `~/.kimi-code`） | 项目级；见 hook 文 |
| `refresh-user-env.ps1` 把全部 User 环境变量拷进进程 | 密钥 allowlist；不要整表灌进 yolo pane |
| 前置守卫启发式 `kill-server` | 查询失败绝不杀；hardened-guard 的 `Get-RmuxServerState` 才是正确形状 |
| launcher 问用户撞名 `[a]/[c]` | 编排器用 `--force` 或拒绝，不交互 Read-Host |
| research / review-cycle 任务原语与 `.rmux_tasks` 目录规范 | 以后可做成 `oma run` 工作流，不是现在的 CLI 形状 |
| 驱动 prompt 强制 ASCII | 用 paste-buffer 后应能中文；短指令仍可 ASCII |
| demo-orchestration 的 `C-m` | 以 win-rmux 实测为准：`C-m` 是字面量 `^M` |
| `rmux claude` teammate | 要 Git Bash；内层 socket 随机，外层看不见。`oma` 自己 spawn `claude` |
| `web-share` / share.rmux.io | 观察面是本仓可选 HTTP + xterm.js，默认 loopback，不是公网 share |
| `rmux setup tmux-shim` | Windows 明确不支持 |
| 默认单位名 `execution-unit` | 按项目 hash 命名，禁止撞名静默叠窗格 |

### 4. 六原语对照 `oma`

| 原语 | win-rmux | `oma` |
| --- | --- | --- |
| launch | wt + launcher.ps1 | `oma` / `oma spawn`，默认不阻塞；Job Object 才 wt |
| locate | pid 反查；不匹配 throw | doctor / send 内建 |
| drive | 两段式 send-keys | 三段式 load-buffer + paste-buffer -p + Enter |
| observe | 文件产物优先 | 同；网页镜像是观察面 |
| judge | `AGENT_STATE_*` + idle | `.ohmyagents/state` + 任务 JSON；Quiet 不当 idle |
| recover/close | attach / kill-session | `--open` 可选网页；`oma cleanup` |

### 5. 核查结果

| 主张 | 结论 | 证据 |
| --- | --- | --- |
| 回车用 `Enter`，`C-m` 无效 | 成立 | AGENTS、troubleshooting 五、keybindings |
| `has-session` 在 pwsh 不可靠 | 部分成立 | 命令本身：存在 0、不存在 1（command-verification）；不可靠的是 pwsh `if` 包一层，应用 `list-sessions` |
| 要 `RMUX_DISABLE_TINY_CLI=1` | 成立 | environment.md；官方 README / v0.7.0 起 |
| Job Object 下 os error 5 | 成立（win-rmux 实测；本仓未复测） | troubleshooting 一；rmux-usage launcher |
| 备屏 capture 空 | 成立 | options `alternate-screen on`；troubleshooting 三 |
| Codex 单次 `C-c` 退出 | 成立 | troubleshooting 五，2026-08-21 |
| `paste-buffer -p` / `load-buffer` 可用 | 成立 | command-verification 两轮均为 OK；官方 man / dispatch |
| `send-keys -H` 已在本机测过 | 未能核实 | `list-commands` 有 `[-FHKlMRX]`，keybindings 写「十六进制字节」；核验表只跑了裸 `send-keys`。evo-harness 用 `-H 0d`，装上 rmux 后再 `rmux send-keys --help` |
| 安装路径 `D:\ohmyenv\rmux` | 已过时（2026-08-29） | `Test-Path D:\ohmyenv\rmux` = False；User/Machine PATH 无 rmux；`Get-Command rmux` 失败 |
| hardened-guard 对 `oma` 有硬约束 | 成立 | 见下节 |

### 6. 从 hardened-guard 收来的硬约束

`.rmux_tasks/skill-review/research/research-claude.md`（2026-08-21）把 SKILL 前置守卫评为多条 HIGH。POC `hardened-guard.ps1` 已实现修正。`oma` 默认应对齐 POC，而不是对齐 SKILL 出厂守卫：

1. 永不启发式 `kill-server`。区分无进程 / 无 server / 查询失败 / 正常；查询失败不杀。
2. 会话带所有权标记；别人的 session 不碰；自己的残留也要显式 `--force` 才 `kill-session -t`。
3. hook 安装默认不改用户家；项目级 opt-in。
4. User 环境只 allowlist 密钥，不整表拷贝。
5. launch：先直接 `new-session -d`；`os error 5` 且交互桌面有 `wt` 才退路；无头环境 fail-fast。
6. 固定 `Start-Sleep 10` 换成截止轮询：pane 数 + 进程名齐才算就绪。
7. locate 不匹配硬中止，不要 warn-and-continue（会把 prompt 打进裸 pwsh）。
8. 生成的 launcher 不写仓库 cwd，写临时目录；unit 名校验 `[A-Za-z0-9_-]+`。

### 7. 命令核验表对实现的含义

`command-verification.md`（2026-08-20，rmux 0.10.0，Windows 10.0.26100）不是「哪些命令存在」的目录，是「headless 真实 server 上跑过」的记录。

对 `oma` 有用的 OK：`new-session` / `split-window` / `list-*` / `has-session`（存在）/ `send-keys` / `capture-pane` / `paste-buffer -p` / `load-buffer` / `set-environment` / `wait-pane` / `find-panes` / `find-sessions` / `pane-snapshot` / `broadcast-keys`。

预期 ERR / TIMEOUT，不要当 bug：

- 无 attach 客户端：`attach-session`（not a terminal）、`display-menu`、`command-prompt` 等
- `stream-pane --lines` TIMEOUT（持续流）
- `collect-pane-output` 缺 `--until-pane-exit` 则 ERR
- `rmux claude` 无 Git Bash 则 ERR
- `setup tmux-shim` 仅 Unix

扩展命令 `wait-pane` / `find-panes` / `stream-pane` 不在 `list-commands` 里，但 `--help` 与核验表都证明可用（rmux-usage 已写）。实现不要用 `list-commands` 当扩展命令是否存在的唯一判据。

### 8. 对实现的硬约束（合并后）

1. 每个 `oma` 进程设 `RMUX_DISABLE_TINY_CLI=1`，清 `NO_COLOR`，`TERM=xterm-256color`。
2. 会话探测用 `list-sessions`，不用 `has-session` 当 `$LASTEXITCODE` 的 `if`。
3. 四路 2x2：两次 `-h`/`-v` 对半，不要把 win-rmux 的第三个 `-f -v`（上 2 下 1）抄过来。
4. Drive：load-buffer + paste-buffer `-p`；Enter 单独发；quiet 超时只补 Enter；长文走文件。
5. locate 用 pid，不信 `pane_current_command`；错位 throw。
6. cleanup 只 `kill-session`；查询失败不 `kill-server`。
7. 从 Codex/Claude 宿主拉起时准备 wt 退路，默认 CLI 不弹窗。
8. Codex pane 永不 `C-c`，不 `respawn-pane -k`。
9. 状态权威在项目 `.ohmyagents/state`，不在全局 hook，也不把 `wait-pane Quiet` 当 idle。

### 9. 与本仓已有研究的对照

| 本仓文 | 本轮 | 处置 |
| --- | --- | --- |
| 《drive铁律与三段式粘贴》 | 两段式 send-keys 是 win-rmux 下限；三段式 paste 被核验表支持 | 维持三段式；Enter 优先键名，`-H 0d` 待装 rmux 后核 |
| 《rmux状态判断与hook补充》 | hook 通道实测、Codex Stop 缺失、CPU 对深思误报 | 维持四层；吸收「CPU 必须 -Sum」 |
| 《项目级hook与skill》 | 全局安装是 SKILL 出厂默认，复核评为 HIGH | 维持项目级；hardened-guard 的 opt-in 是正例 |
| 《跨平台与无浏览器》 | web-share 不是无浏览器方案 | 维持 `--no-web` + 本仓 HTTP；不默认 web-share |
| 《yolo与无阻塞启动》 | yolo flags 与 blast-radius 警告必须保留 | 维持配置落盘优先；flags 单次覆盖 |

## 事实源

| 类型 | 定位 | 日期 | 对应 | 提供了什么 |
| --- | --- | --- | --- | --- |
| 本地 | `D:\sourcecode\win-rmux\AGENTS.md` | 当前树 | 研究 1、核查 1-3 | 坑摘要、pwsh 约束、v0.1.5 |
| 本地 | `skills/win-rmux/SKILL.md` | 当前树 | 研究 1-3、核查 4-6 | 六原语、守卫、yolo、默认前台 |
| 本地 | `references/rmux-usage.md` | 2026-08-19 | 研究 1、核查 3-4 | daemon PID 模型、launcher、NO_COLOR 根因 |
| 本地 | `references/troubleshooting.md` | 当前树 | 核查 1-6 | 唯一坑表 |
| 本地 | `references/command-verification.md` | 2026-08-20 | 核查 2、7、8 | 90+ 命令真实退出码；paste-buffer OK |
| 本地 | `references/environment.md` `overview.md` `options.md` `extensions.md` `hooks.md` `keybindings.md` `formats.md` `web-share.md` `commands.md` `task-workflows.md` `README.md` | 当前树 | 研究 1-3 | 命令面、选项、扩展、状态、任务 |
| 本地 | `hooks/win-rmux-agent-state.ps1` `scripts/install-agent-hooks.ps1` `scripts/refresh-user-env.ps1` | 当前树 | 研究 3、核查 10 | 全局 hook、整表 User env |
| 本地 | `.rmux_tasks/skill-review/research/research-claude.md` `poc-claude/hardened-guard.ps1` | 2026-08-21 | 核查 10 | HIGH 项与 POC 守卫 |
| github | `Helvesec/rmux` latest `v0.10.0`（2026-08-04）README / ARCHITECTURE / man `paste-buffer` / `RMUX_DISABLE_TINY_CLI` | 2026-08-04；本轮 `gh repo view` 2026-08-29 | 研究 1、核查 3、7 | 官方布局与逃生开关；与 win-rmux 标明版本一致 |
| github | `raystyle/win-rmux`（README 安装说明） | 当前 | 研究 1 | skill 发布面；本轮未再拉远端树 |
| 本机 | `Test-Path D:\ohmyenv\rmux`；`Get-Command rmux` | 2026-08-29 | 核查 9 | 路径过期，PATH 无 rmux |
| x | 本轮 keyword / semantic 检索 | 2026-08-29 | 全部 | 无对齐增量（tmux-ide 等跑题） |

## 缺口

- 本仓仍未安装 rmux；Job Object / wt 退路、`send-keys -H`、中文 paste-buffer 未在 `oma` 进程里复测。
- `command-verification` 的 `send-keys` 是裸命令，没有 `-H 0d` 行。
- win-rmux 标明 0.10.0；GitHub 最新仍是 v0.10.0（2026-08-04）。装上后应用 `rmux list-commands` 当场核，不把核验表当永远快照。
- X 对本吸收无贡献。
- 未读 `.reasonix/tasks/*.jsonl`（判定为 IDE 会话，不是 rmux 研究）。
- 未对照 `C:\Users\ray\.claude\skills\claude-skills\win-rmux` 安装副本与 `D:\sourcecode\win-rmux` 是否字节一致（目录清单相同，内容未 diff）。
