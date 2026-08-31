# M101-drive与paste错误

> 关键词：send-keys、Enter、C-c、bracketed paste、200~、paste-buffer、超时、重发。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M001 | 对 Codex 预清输入发 `C-c` | `--no-alt-screen` 下 Ctrl-C 退出进程 | 禁止对 Codex 发 `C-c`；超时只补 Enter | 2026-08-21 |
| M008 | broadcast-demo 自包 `\x1b[200~` | 演示竞速与生产 drive 不同 | 用 evo-harness `paste-buffer -p`，发送侧不自包壳 | 2026-08-29 |
| M027 | poc-stream 的 no-replay 断言把 Now 流误判为回放旧行 | live marker `OMA-POC-STREAM-NOW` 包含 backlog marker `OMA-POC-STREAM` 子串，子串搜索自命中 | 多 marker 同流断言时，marker 互不为子串（`OMA-STREAM-BACKLOG` / `OMA-STREAM-LIVE`）；与 paste POC 拼 marker 防回显假阳性（P0005 经验）同族 | 2026-08-31 |
