# rmux状态判断与hook补充

> 2026-08-29。rmux 能判断「pane 活着、画面静了、进程换了」，不能判断「这个 coding agent 空闲、干活还是卡住」。编排器要的后一类状态必须用各家 lifecycle hook 补上。对照 win-rmux hooks、evo-harness `STATE_MAP`、herdr 权威表、rmux 0.10 SDK。

## 背景

`doctor` / `status` / `run --assign` 都要回答：这个 pane 里的 agent 能不能接任务。rmux 文档写得很直：foreground 是 best-effort，**不试图给进程分类成 agent 名**（<https://github.com/Helvesec/rmux/blob/main/docs/scripting-sdk.md>）。win-rmux 把 judge 从「CPU 猜」改成「hook 自报 idle/working/blocked」，就是补这个洞。

## 关键结论

编排器叠四层，不要只用一层：[推断: rmux 只暴露终端态；herdr / evo-harness 把 agent 忙闲另开通道]

| 层 | 来源 | 能判什么 | 不能当什么 |
| --- | --- | --- | --- |
| 0 存活 | `list-panes`、pane_pid、remain-on-exit | pane 在不在、进程死没死 | agent 忙闲 |
| 1 终端 | `wait-pane --quiet`、`Quiet` load-state、snapshot 稳定 | 画面是否静止，适合 **Drive 同步** | agent idle。Claude 深思时画面也可静 |
| 2 语义 | 各家 hook 写 `.ohmyagents/state/<agent>.json` | idle / working / blocked / unknown | 进程是否还在；Codex `Stop` 常缺 |
| 3 任务 | `.ohmyagents/tasks/<id>.json` | 这个 agent 指哪条任务 | 运行时忙闲 |

层 1 给 spawn/drive 用（发前静默、超时只补 Enter）。层 2 给委派和 doctor 用。层 0 失败则整路作废。禁止用 CPU 当主判据。[经验: win-rmux 2026-08-21 Claude 深思低增量会误报未提交然后重发]

## rmux 原生有什么

SDK / CLI 已经很多，但都是 **终端** 状态，不是 **agent** 状态。

存活与进程：

- `list-panes`、`pane_pid`、`pane_current_command`（TUI 上不可靠，win-rmux 要求 pid 反查）
- `foreground_state`：daemon 侧周期探测，可能滞后约一秒；Windows 报 ConPTY 根进程；**不分类 agent 名**
- `state_events`：标题、option、关闭、可选 foreground。`Closed` 是流结束，不是 idle。`DiedKept` 时 pane 还能 snapshot
- `collect-until-exit` / `--pane-exit`：进程退出，coding agent 正常工作时不会退

画面与等待：

- `wait-pane --quiet --stable-for`、`wait_until_stable_for`、`expect_stable`
- 官方定义：`Quiet` = 渲染快照在窗口内不变，**不推断提示符**
- `--text` / `--next-text` / `--visible-text`、locator `get-by-text`
- `send-keys --wait quiet`：超时不等于没发出去（win-rmux）

输出：

- `output_stream`、`render_stream`、`surface_stream`、`recover_output`（0.10）
- 网页镜像走字节流，可以 **给人看**，不能当 busy/idle 权威
- 备屏 TUI：`capture-pane` 常空，长回复要写文件再读（win-rmux observe）

这些足够做「Drive 前等静默」「短头是否还在输入行」，不够做「能不能委派下一条任务」。

## 为什么必须 hook

coding agent 是备屏 TUI：思考时画面可能静、工具跑时画面可能滚、权限框是 UI 不是进程退出。rmux 看见的是「某个 exe 还在、某段时间没新像素」，看不见「模型在跑 / 等你按 y / 输入框空了」。

herdr 用 lifecycle hook 当权威，没有 hook 才退屏幕清单。本仓不跑 herdr，但语义相同：hook 报 idle/working/blocked。

没有 hook 时的失败模式（已踩过）：

- 只看 `--quiet`：深思被当成 idle，过早委派下一条
- 只看 CPU：Claude 深思误报未提交，重发排队
- 只看 capture：备屏空，误判死
- 只看 `pane_current_command`：可能标成错的 agent

## hook 补哪些判断

事件到四态（抄 evo-harness `STATE_MAP`，再按官方事件名对齐）：

| 语义 | 典型事件 | 编排器用它做什么 |
| --- | --- | --- |
| idle | SessionStart、Stop、Interrupt、SessionEnd | 可 `run`/`send`；doctor 绿 |
| working | UserPromptSubmit、PreToolUse、PostToolUse、PostToolUseFailure、SubagentStart/Stop、PreCompact | 不重复 drive 全文；可记「该任务进行中」 |
| blocked | PermissionRequest（Codex/Kimi）；Claude 无此标准事件 | 停该路委派，doctor 非 0；不要对 Codex 用 `C-c` 清 |
| unknown | Notification（tips 和权限混在一起） | **不要**当成 idle（evo 曾因此狂催） |

Claude 补 blocked：没有 `PermissionRequest` 就不要假装有。Notification 保持 unknown。对话框仍靠 Drive 发前扫屏（DIALOGS）兜底，那是按键层，不是状态权威。

Codex `Stop` 常不触发：state 会停在 working。doctor 把「working 超过阈值且 pane 仍在」标 stale，Drive 用短头/输入行残留确认，不自动重发全文。

Kimi 另有 Interrupt。Grok 事件名可能是 camelCase / snake，hook 脚本要做双形态归一（evo 已做）。

## 和 rmux 层怎么接

hook **不**替代 wait-pane。推荐顺序：

1. spawn：层 0 进程活着即返回（无阻塞启动，见 yolo 文）
2. Drive 前：层 1 `--quiet --stable-for` 短窗口，避免往忙 TUI 里塞键
3. Drive 后：层 2 等 working 或短头离开输入框；超时只补 Enter
4. 委派下一条：层 2 idle 或层 3 任务文件显示该 agent 未 assigned；blocked 则跳过该路
5. doctor：层 0 + 层 2 + 层 3 + yolo 键，全部读磁盘/进程表，不 attach

Windows 上 hook 若改走 `rmux set-environment`，必须注入专用 pipe 名。默认仍写项目 JSON，不依赖 hook 连上 daemon。

## 实现要点

- spawn `-e OHMYAGENTS_AGENT` `-e OHMYAGENTS_STATE_FILE` `-e OHMYAGENTS_PROJECT`；hook 对不上就 exit 0
- 注册项目级（Claude settings、Codex 须先信任、Grok 项目 hooks 要 folder-trust、Kimi 暂用户 config + 项目脚本）
- `matcher: "*"`（Claude）；Codex 持久化 `hooks.state.trusted_hash`，不要每次 `--dangerously-bypass-hook-trust`
- PostToolUse 也刷 working 时间戳，长 bash 不算静默（evo 注释）
- 网页只显示层 2 的副本，权威仍是文件

## 待办

- 本机装 rmux 后对照 `wait-pane --help` 与 `Pane::wait_for_load_state`
- Codex 0.149.1 复测 Stop 是否仍不触发
- Claude 无 PermissionRequest 时 blocked 是否只能靠扫屏
