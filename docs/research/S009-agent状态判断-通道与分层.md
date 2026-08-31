# S009-agent状态判断-通道与分层

> 2026-08-31。由《agent状态通道》（2026-08-29，通道选型：文件总线而非 rmux 环境变量）与《rmux状态判断与hook补充》（2026-08-29，四层模型）合并，并按《S010-clum等待原语作为hook兜底状态》把层表改写为含 1b 终端语义兜底的现行版。回答一个问题：**这个 pane 里的 agent 能不能接下一条任务**。

## 背景

rmux 能判「pane 活着、画面静了、进程换了」，判不了「agent 空闲、干活、卡框」——官方写明 foreground 是 best-effort、不分类 agent 名。herdr 把忙闲做成 pane 一等状态；本仓不引入 herdr 运行时，借语义：lifecycle hook 写状态文件，hook 沉默时用终端语义兜底。

## 关键结论

### 1. 分层模型（现行版，含 1b）

表内能力与限制为 rmux 官方文档与 SDK 源码口径，1b 层据 S010 源码核实。[实证: rmux 官方 scripting-sdk；S010 clum 核实]

| 层 | 来源 | 能判什么 | 不能当什么 |
| --- | --- | --- | --- |
| 0 存活 | `list-panes`、pane pid | pane 在不在、进程死没死 | agent 忙闲；层 0 失败整路作废 |
| 1 终端静止 | `wait-pane --quiet --stable-for`、Quiet load-state | 画面是否静止，适合 Drive 同步 | agent idle（深思时画面也静） |
| **1b 终端语义** | 等待原语 + `terminal_state` 分类器（password/confirm/editor/pager/REPL/shell） | hook 沉默时从画面判 confirm/password 等阻塞形态；`wait_for_text` 等执行证据 | 完整忙闲语义（中文确认词有缺口） |
| 2 语义（可选加速） | 各家 hook 写 `.ohmyagents/state/<agent>.json` | idle/working/blocked/unknown，最快最准 | hook 沉默即失效（Codex `Stop` 常缺、Claude 无 PermissionRequest）；**不再是唯一权威** |
| 3 任务 | `.ohmyagents/tasks/<id>.json` | 该 agent 指哪条任务 | 运行时忙闲 |

裁决：**权威链 = 层 0 先行，层 2 有则用，沉默走 1b 终端语义兜底**；层 1 只给 Drive 同步。禁止 CPU 当主判据。[推断: 分层与通道对照；Quiet 不当 idle 经验: rmux 官方定义 + win-rmux 2026-08-21 误报现场]

2026-08-31 `examples/poc-state.rs` 实证：1b 分类器（`detect_terminal_state`，rmuxpoc 共用层）在 hook 全沉默下判出 ready/confirm/password；Quiet 静止成立时 verdict 非 idle。[实证: poc-state 标记行]

### 2. 通道选型：项目内文件总线

- 主通道：`<project>/.ohmyagents/state/<agent>.json`。spawn 注入 `OHMYAGENTS_PROJECT` / `OHMYAGENTS_AGENT` / `OHMYAGENTS_STATE_FILE`；各家项目 hook 的 `command` 调 `oma hook`（stdin 事件 JSON 或 `oma hook blocked`），原子写文件；缺环境变量或项目对不上则 exit 0——某家 agent 只肯加载用户级 hook 时也不污染别的仓库。[实证: 2026-08-29 poc-dialogs `oma hook` 写 blocked 再 idle]
- 不用 `rmux set-environment` 当上报口（win-rmux 反例）：hook 短命子进程常找不到本会话专用 pipe；Windows 上 PATH、默认 socket、Job Object 一错状态就静默丢。[经验: win-rmux hooks.md 2026-08-21]
- 不引入 herdr 运行时：没有 `HERDR_PANE_ID`，`report-agent` 无处可报；借的是四态语义与「有 hook 用 hook、无 hook 退屏幕」的分层思想。[经验: herdr.dev agents 状态表]
- SDK `output_stream` / `state_events` 是观察 PTY 画面与 pane 生死，读不到 hook JSON，不当 blocked 权威。[实证: rmux-sdk 0.10.0 文档]

### 3. 报阻塞与点掉阻塞分路

| 方向 | 谁说话 | 通道 | 为什么 |
| --- | --- | --- | --- |
| 报阻塞 | agent 的 hook 进程到编排器 | 写 state 文件 `state=blocked` | hook 是短命子进程，常找不到专用 pipe；项目级约束不该让 hook 连 daemon |
| 点掉阻塞 | 编排器到 pane | Drive：`send_text` / `send_key`（Enter、y、Down） | 往 TTY 打键本来就要 rmux；不是 hook 上报口 |

两条相反的路不合成一条 named pipe。[推断: hook 短命进程找不到专用 pipe；win-rmux set-environment 反例]

### 4. 事件映射（evo-harness STATE_MAP 加本仓对齐）

映射表与双形态归一来自 evo-harness `agent_state_hook.py` 与各家官方事件名；Codex `Stop` 缺失为 win-rmux 实测。[经验: evo-harness STATE_MAP；实证: win-rmux 2026-08-21]

| 语义 | 典型事件 | 编排器做什么 |
| --- | --- | --- |
| idle | SessionStart、Stop、Interrupt、SessionEnd | 可 run/send；doctor 绿 |
| working | UserPromptSubmit、Pre/PostToolUse(Failure)、SubagentStart/Stop、PreCompact | 不重复 drive 全文 |
| blocked | PermissionRequest（Claude/Codex/Kimi 都有；Claude 还可 hook 程序化 allow/deny）；Grok **无**此事件，等待审批态走 1b 画面兜底，PermissionDenied 只代表已拒 | 停该路委派，doctor 非 0；不对 Codex 发 `C-c` |
| unknown | Notification（tips 与权限混杂） | **不当 idle**（evo 曾因此狂催） |

2026-08-31 订正（一手核实见 S015）：Claude 现有标准 `PermissionRequest` 事件并支持 `decision.behavior: allow/deny` 裁决，旧口径「Claude 无此事件、Notification 顶替归 unknown」作废；Grok（grok-build）事件集无 PermissionRequest，只有 PermissionDenied（已拒）与 Notification。事件名四家全 PascalCase、stdin 都含 `hook_event_name` 与 `cwd`，`oma hook` 双形态归一继续成立。[实证: 官方 hooks reference 与三家源码]

事件名做 claude/grok 双形态归一（`hook_event_name` / `hookEventName` 去下划线 lower）。Claude 无 PermissionRequest 就不假装有；对话框靠 Drive 发前扫屏兜底（按键层不是状态权威）。Codex `Stop` 常不触发：state 停在 working，doctor 把「working 超阈值且 pane 仍在」标 stale，Drive 短头确认，不自动重发。

### 5. 与 rmux 层的推荐顺序

本节为第 1 节分层模型在编排器各阶段的落地推导。[推断: 分层对照推导]

1. spawn：层 0 活着即返回（无阻塞启动）
2. Drive 前：层 1 `--quiet --stable-for` 短窗口，不往忙 TUI 塞键
3. Drive 后：层 2 等 working 或短头离开输入行；超时只补 Enter
4. 委派下一条：层 2 idle，或层 3 任务文件未 assigned；blocked 跳过该路；层 2 沉默走 1b
5. doctor：层 0 + 层 2 + 层 3 + yolo 键，全读磁盘进程表，不 attach

### 6. rmux 原生能力清单（都是终端态不是 agent 态）

清单为 rmux 0.10 官方文档与命令核验表口径；`foreground_state` 不分类 agent 名为官方明文。[实证: rmux scripting-sdk；win-rmux command-verification]

- 存活：`list-panes`、pane pid、`state_events`（`Closed` 是流结束不是 idle）、`collect-until-exit`
- 画面：`wait-pane --quiet`、`--text` / `--visible-text`、locator `get-by-text`；`send-keys --wait quiet` 超时不等于没发出
- 输出：`output_stream` / `render_stream` / `surface_stream` / `recover_output`；备屏 TUI `capture-pane` 常空，长回复写文件再读
- `foreground_state`：daemon 周期探测约滞后一秒，Windows 报 ConPTY 根进程，不分类 agent 名

### 7. 实现要点

注册与信任前置来自 S006 信任门结论；`matcher` 与 trusted_hash 为各家官方要求；poc-dialogs 已实证 hook 链路。[经验: 各家官方；实证: 2026-08-29 poc-dialogs]

- spawn 注入三个 `OHMYAGENTS_*`；hook 对不上 exit 0
- 注册项目级（Claude settings、Codex 须先信任、Grok 项目 hooks 要 folder-trust、Kimi 暂用户 config 加项目脚本）
- `matcher: "*"`（Claude）；Codex 持久化 `hooks.state.trusted_hash` 不用每次 bypass flag
- PostToolUse 也刷 working 时间戳（长 bash 不算静默）
- 网页只显示层 2 副本，权威是文件；1b 兜底的分类器细节见《S010-clum等待原语作为hook兜底状态》

## 踩坑沉淀

| 坑 | 正解 |
| --- | --- |
| 只看 `--quiet` 判 idle | 深思被当 idle 过早委派；Quiet 只给 Drive 同步 |
| 只看 CPU | Claude 深思低增量误报重发 |
| 只看 capture | 备屏空误判死 |
| Notification 映 idle | 映 unknown |
| set-environment 当上报口 | hook 找不到专用 pipe 状态静默丢；走文件 |
| hook 沉默就报 idle | 走 1b 终端语义兜底（terminal_state / wait_for_text） |

## 待办

- Codex 0.149.1 复测 Stop 是否仍不触发
- 1b 分类器的中文确认词缺口（见 clum 篇待办）

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| web | herdr.dev docs agents / integrations；issue #1630 | 2026-08-29 | 四态语义、report-agent 与环境变量机制 |
| 本地 | win-rmux `references/hooks.md` | 2026-08-21 | set-environment 通道反例、各家 hook 坑 |
| 本地 | evo-harness `agent_state_hook.py` / `install_hooks.py` | 2026-08-24 | STATE_MAP、双形态归一、文件总线 |
| web | Helvesec/rmux `docs/scripting-sdk.md` | 2026-08-29 | foreground best-effort 不分类；Quiet 定义 |
| 本地 | 本仓 `poc-dialogs` | 2026-08-29 | `oma hook` 写状态实证 |
| 本地 | 《S010-clum等待原语作为hook兜底状态》 | 2026-08-29 | 1b 层与权威链改写的依据 |
