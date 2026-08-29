# agent状态通道

2026-08-29

四路跑起来之后，CLI 得知道谁在干活、谁卡在权限框、谁空闲可发下一条。herdr 把这事做成 pane 上的一等状态；rmux 没有同款 API。win-rmux 和 evo-harness 各挖了一条通道。我们借语义，不借 herdr 进程。

## herdr 怎么报状态

herdr 认 `idle` / `working` / `blocked`（展示层还有 `done`）。有完整 lifecycle hook 的 agent，安装 integration 之后以 hook 为准；否则退屏幕清单（<https://herdr.dev/docs/agents/> 状态权威表）。

上报入口是 `herdr pane report-agent`，pane 里的进程能看见 `HERDR_PANE_ID`、`HERDR_BIN_PATH`、`HERDR_SOCKET_PATH`（<https://herdr.dev/docs/integrations/>）。Claude 的 integration 脚本长这样：`bash '.../herdr-agent-state.sh' session`，装在 `~/.claude/settings.json` 的 `SessionStart`。(herdr issue #1630 用户贴出的 settings 片段)

Oh My Agents **不**把 herdr 当运行时。没有 `HERDR_PANE_ID`，`report-agent` 无处可报。要抄的是：事件映射到四态，身份用环境变量注入，hook 脚本在非本系统会话里静默退出。

## win-rmux：rmux 环境变量

rmux 没有 `pane report-agent`，有 `set-environment` / `show-environment`。(`D:\sourcecode\win-rmux\skills\win-rmux\references\hooks.md` L5–14)

```powershell
rmux set-environment -t $unit AGENT_STATE_codex working
rmux show-environment -t $unit AGENT_STATE_codex
```

身份靠 spawn 时 `-e WIN_RMUX_UNIT= -e WIN_RMUX_AGENT=`。hook 脚本读 stdin 的 `hook_event_name`，没有这两个环境变量就 exit 0。(同文件 L47–57)

坑也写在同一页：Codex 要 `--dangerously-bypass-hook-trust` 否则静默跳过；Claude 的 JSON 条目要 `matcher: "*"`；`PermissionRequest` 不是 Claude 标准事件，可能永不报 `blocked`；Codex 的 `Stop` 实测不触发，状态卡在 `working`。(hooks.md L32–38)

这条通道绑死 rmux session。hook 进程还得找得到同一个 daemon（Windows 上还得是那根 named pipe）。项目级约束时，hook 从项目目录起，PATH 和 pipe 名都要随 spawn 注入，不能假设用户全局 `rmux` 默认 socket。

## evo-harness：项目里的 state 文件

evo-harness 不往 rmux 环境变量写状态，往 `EVO_STATE_FILE` 指的 JSON 写。(`scripts/agent_state_hook.py` L13–16、L88–109)

```json
{"state":"working","event":"userpromptsubmit","unit":"...","run":"...","ts":...}
```

事件名做了 claude / grok 双形态归一：`hook_event_name` 或 `hookEventName`，去掉下划线再 lower。(L101–105) `Notification` 映成 `unknown`，不映成 `idle`——曾经因此狂催还在干活的 agent。(L35–36 注释)

`done` 不落四态，是 monitor 看到 `idle` 且 `worker.json` 的 `seen=false` 之后派生的。(文件头 L8–11) herdr 的展示语义在这里被拆开了。

## 我们用哪条

(项目级 + 跨平台 + 网页只观察)

主通道：**项目目录里的文件**，不写用户家，不依赖 hook 进程能连上 rmux。

```text
<project>/.ohmyagents/state/claude.json
```

spawn 注入 `OHMYAGENTS_PROJECT`、`OHMYAGENTS_AGENT`、`OHMYAGENTS_STATE_FILE`。各家项目 hook 的 `command` 调 **`oma hook`**（stdin 事件 JSON，或 `oma hook blocked`）。子命令把事件映成 idle/working/blocked/unknown，原子写 state 文件。缺 `OHMYAGENTS_STATE_FILE` 或项目对不上则 exit 0——这样即使某家 agent 只肯加载用户级 hook，也不会污染别的仓库。不连 rmux 管道。

四态与 herdr / evo-harness 对齐：`idle | working | blocked | unknown`。事件映射从 `agent_state_hook.py` 的 `STATE_MAP` 抄，Claude 用 `Notification` 顶不存在的 `PermissionRequest`。(evo-harness `install_hooks.py` L8–10)

CLI `status` 读这四个 JSON。网页 control 通道只推 `highlight` 和状态字，不充当权威。rmux `show-environment` 可作调试回退，不作默认。

Codex `Stop` 不触发时，文件会停在 `working`。Drive 侧仍用短头确认；status 对超时的 `working` 标 stale，不拿 CPU 猜。(win-rmux 2026-08-21 Codex Stop 不触发；CPU 法对 Claude 深思会误报)

## 所以

语义跟 herdr：idle / working / blocked。[经验: herdr.dev agents 状态表] 实现跟 evo-harness 的文件总线，落在启动的那个项目的 `.ohmyagents/state/`。[实证: 2026-08-29 poc-dialogs `oma hook` 写 blocked 再 idle] 不调用 `herdr pane report-agent`，默认不写全局 hook，不把 rmux 环境变量当唯一通道。

rmux 自己还能判存活和画面静默，那些**不是** agent 忙闲。分层与事件表见《rmux状态判断与hook补充》。

## hook 报阻塞要不要 rmux/SDK 管道

**不要。** 报阻塞和点掉阻塞是两条相反的路，不要合成一条 named pipe。[推断: hook 短命进程找不到专用 pipe；win-rmux set-environment 是反例]

| 方向 | 谁说话 | 通道 | 为什么 |
| --- | --- | --- | --- |
| 报阻塞 | agent 的 hook 进程 → 编排器 | 写 `<project>/.ohmyagents/state/<agent>.json` 的 `state=blocked` | hook 是短命子进程，常常找不到本会话专用 pipe；项目级约束也不该让 hook 去连 daemon |
| 点掉阻塞 | 编排器 → pane | 已有 drive：`send_text` / `send_key`（Enter、y、Down） | 这是往 TTY 打键，本来就要 rmux SDK；不是 hook 上报口 |

win-rmux 用 `rmux set-environment` 当上报口，hook 必须命中**同一根** named pipe / 同一个 daemon。Windows 上 PATH、默认 socket、Job Object 一错，状态就静默丢。本仓不走这条。

rmux-sdk 的 `output_stream` / `state_events` 是观察 PTY 画面和 pane 生死，读不到 hook 的 JSON，不能当 blocked 权威。`doctor` 已经在读 `.ohmyagents/state/*.json`。

spawn 时注入 `OHMYAGENTS_STATE_FILE`（绝对路径）。hook 缺这个变量或项目对不上就 exit 0。设置和 CLI 参数清不掉的框：优先用等待原语与 `terminal_state` 从画面判 confirm/password（见《clum等待原语作为hook兜底状态》）；hook 文件仍可作为可选加速，不是唯一兜底。[推断: YouMind clum 报告 + 本仓 hook 沉默失败模式]
