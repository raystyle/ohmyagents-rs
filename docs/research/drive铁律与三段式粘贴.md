# drive铁律与三段式粘贴

2026-08-29

把字打进四个 TUI，是这条工具最容易抄错的地方。broadcast-demo 看起来能用 `PaneSet.broadcast` 一次贴到所有 pane；evo-harness 和 win-rmux 花了好几轮研究才写成「四铁律 + 三段式」。我们按后者做。

## 铁律从哪来

win-rmux 先写了两段式：文本 `-l` 一次，Enter 另一次；禁止对 Codex 发 `C-c`（`--no-alt-screen` 下一次 Ctrl-C 会把进程干掉）；locate 用 `pane_pid` 反查进程名，不信布局下标。(`D:\sourcecode\win-rmux\skills\win-rmux\SKILL.md` drive / locate 节)

evo-harness 把这套搬到 Python，又叠了 herdr 研究里的 P8.1。(`D:\sourcecode\evo-harness\src\evo_harness\driver.py` 模块注释 L10–17，以及 `drive()` L240–280)

五条：

1. 绝不发 `C-c` 预清。
2. 文本和 Enter 分发，中间用 rmux 原生静默等待，不用盲 `sleep`。
3. 提交后正向确认：短头（prompt 前 10 字）离开输入框，或 hook 变成 `working`。超时只补 Enter，不重发全文。
4. `pane_pid` 反查定位。
5. prompt 拍成单行。

## 三段式：先查，再贴，再回车

`drive()` 不是 `send_text + Enter`。顺序是：

1. **发前扫对话框**（`_sweep_dialogs`）。模态框在场会吞提交。marker 用各 agent 对话框并集，不按 pane 猜是谁。(`driver.py` L349–367；对话框表在 `config.py` `DIALOGS`)
2. **bracketed-paste 感知注入**。`load-buffer` 把文本载入，`paste-buffer -p` 让 **daemon** 决定要不要包 `\x1b[200~…\x1b[201~`。发送侧自包壳会双重包裹。(`driver.py` `_send_paste` L370–403 注释「r10 研究 C3」)
3. 原生 `--quiet --stable-for` 同步。
4. Enter 用 `send-keys -H 0d`（hex 字节）。老 daemon 没有 `-H` 再退 `send_keys("Enter")`。(`_send_enter` L405–418)

broadcast-demo 正好走了被否决的那条路：自己拼 `bracketed_paste()`，再 `PaneSet::broadcast(Input::text(...))`。(`broadcast-demo/src/main.rs` `bracketed_paste` / `send_prompt_to_agent`) Gemini 还要逐字慢打。那是演示竞速，不是会话工具该抄的 drive。

## SDK 够不够用

web-claude-demo 用 git 上的 `rmux-sdk`：`keyboard().type_text`、`press("Enter")`、`PaneSet`。crates.io `0.10.0` 有 `RmuxCommand` 这种命令 DTO，docs.rs 上没把 `load-buffer` / `paste-buffer -p` / `send-keys -H` 列成一等 API。(docs.rs `rmux_sdk::command::RmuxCommand` 页只有 endpoint + kind)

(据 evo-harness 走 `librmux` 的 `cmd()` 逃生舱) Oh My Agents 倾向 Rust `rmux-sdk`（网页流已经验证），paste 和编码 Enter 若 SDK 没暴露，就跟 evo-harness 一样走 CLI escape hatch，钉在专用 socket/pipe 上。这要在装上 rmux 之后用 `rmux list-commands` 回写成本文的实证。

## 确认失败时只补 Enter

`--wait quiet` 超时不等于没发出去。win-rmux 写过：此时重发会排队执行两遍。(win-rmux SKILL drive 节) evo-harness 用 `_prompt_residual` 看输入行或 `queued`，只再发一次 Enter。(`driver.py` L276–278)

Codex 的 `Stop` hook 经常不触发，状态会卡在 `working`。(win-rmux `references/hooks.md`；evo-harness `install_hooks.py` 注释) judge 不能只信 hook，短头和输入行残留是回退。

## 所以

Drive 以 evo-harness `HarnessDriver.drive` 为准：扫框、`paste-buffer -p`、静默、`0d`、确认。[经验: evo-harness driver.py + win-rmux 2026-08-21] 禁止 broadcast-demo 自包 paste，禁止文本和 Enter 同一次发送，禁止对 Codex 预清 `C-c`。[实证: 2026-08-29 poc-drive send_text 与 Enter 分发已绿]
