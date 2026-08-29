# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\history\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

各功能部件需求的 POC 验证原型（对应 `GOAL.md`，方案 0005，登记日 2026-08-29）。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 立项 0005 | 已完成 | `docs\history\0005-各功能部件POC验证原型.md` | 2026-08-29 |
| POC check | 已完成 | `oma check`：版本 0.10.0、归档哈希、完整布局（Windows） | 2026-08-29 |
| POC yolo | 已完成 | `oma init --yolo` 写项目级无阻塞键；`--pretrust` 才写家目录；example 用临时目录 | 2026-08-29 |
| POC doctor | 已完成 | `oma doctor` 只读 yolo/trust/binary/state；不 attach；block 则退出 1 | 2026-08-29 |
| 检测已装 agent | 已完成 | `oma agents`：PATH / `OMA_AGENT_PATH` / `OMA_*_BIN` / 默认目录；Win/Linux/mac | 2026-08-29 |
| hook 写状态子命令 | 已完成 | `oma hook`：stdin 事件 → `.ohmyagents/state`；无管道 | 2026-08-29 |
| POC endpoint | 已完成（Windows） | `examples/poc-endpoint.rs`：专用 `WindowsPipe`，非 Default。Job Object 下走 WMI 拉起 daemon。Linux/mac 委托后续仓库 | 2026-08-29 |
| POC session | 已完成（Windows） | CreateOnly 撞名失败；ReuseOnly 可接；kill-session 后 keeper 仍在；不 kill-server | 2026-08-29 |
| POC layout | 已完成（Windows） | 2x2 `split_with` + pwsh 桩 argv；四格独立 pid | 2026-08-29 |
| POC drive | 已完成（Windows） | `send_text` 与 Enter 分发；短头 `OMA-POC-DRIVE` 可见 | 2026-08-29 |
| POC dialogs | 已完成（Windows） | 假 Allow 框：`oma hook` 写 blocked（state 文件），sendkeys `y`+Enter 点掉 | 2026-08-29 |
| POC paste | 待办 | LoadBuffer + PasteBuffer 中文 | 2026-08-29 |
| POC locate | 待办 | pid 反查；错位 throw | 2026-08-29 |
| POC stream | 待办 | `output_stream` 收到字节 | 2026-08-29 |
| POC state | 待办 | Quiet 不当 idle；`terminal_state` / `wait_for_text` 作 hook 沉默兜底（见 clum 等待原语文） | 2026-08-29 |
| POC init | 待办 | 临时目录落 hook/skill，不改家目录 | 2026-08-29 |
| POC negatives | 待办 | 禁止 C-c Codex、禁止 kill-server 进主路径 | 2026-08-29 |
