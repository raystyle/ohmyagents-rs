# M101-drive与paste错误

> 关键词：send-keys、Enter、C-c、bracketed paste、200~、paste-buffer、超时、重发。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M001 | 对 Codex 预清输入发 `C-c` | `--no-alt-screen` 下 Ctrl-C 退出进程 | 禁止对 Codex 发 `C-c`；超时只补 Enter | 2026-08-21 |
| M008 | broadcast-demo 自包 `\x1b[200~` | 演示竞速与生产 drive 不同 | 用 evo-harness `paste-buffer -p`，发送侧不自包壳 | 2026-08-29 |
| M027 | poc-stream 的 no-replay 断言把 Now 流误判为回放旧行 | live marker `OMA-POC-STREAM-NOW` 包含 backlog marker `OMA-POC-STREAM` 子串，子串搜索自命中 | 多 marker 同流断言时，marker 互不为子串（`OMA-STREAM-BACKLOG` / `OMA-STREAM-LIVE`）；与 paste POC 拼 marker 防回显假阳性（P0005 经验）同族 | 2026-08-31 |
| M038 | settle 对 codex 升级屏按了两轮 `2+Enter`：第二轮时屏已关，「2」落进输入框被提交成任务（codex 反问「你只发了 2」） | 按完 keys 立即进下一轮快照，快照还是旧帧（marker 仍在）就再按一轮；没有「按后确认屏变化」也没有警告事件 | 按后确认：等 marker 从屏上消失（3s 上限）才算成功；顽固不消失**不重按**（防重复提交），打 `settle.<agent>.stalled=` 事件人工接手 | 2026-09-01 |
| M039 | 人工打断 codex 直接 `rmux send-keys C-c`——实杀进程（pane dead），S005 铁律只在文档没有程序入口 | 守卫 `check_send_key` 一直在（codex 拒 C-c），但 oma 缺发单键命令，按键只能裸 rmux CLI 绕过守卫 | 用 `oma key <agent> <KEY>`（守卫入口：codex 的 C-c 被拒并给替代建议）；打断 codex 用 Esc（esc to interrupt），别用 C-c | 2026-09-01 |
