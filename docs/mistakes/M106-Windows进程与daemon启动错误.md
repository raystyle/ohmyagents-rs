# M106-Windows进程与daemon启动错误

> 关键词：os error 5、Job Object、WMI、daemon、start-server、exit-empty、ready、kill、transport、pane cwd。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M015 | `connect_or_start` 报 os error 5 / 拒绝访问 | 宿主在 Job Object 里，SDK 拒绝在 job 内起独立 daemon | 专用 pipe 上用 WMI `Win32_Process.Create` 在 job 外拉起 `libexec\rmux\rmux.exe --__internal-daemon`，再 `connect()`；不 kill-server，不把 wt 当默认 | 2026-08-29 |
| M017 | `session.kill()` 偶发 `daemon closed the transport` | 末会话被杀后 daemon 先关连接再回包 | 关连接视为 kill 成功；先确认 keeper 仍在，再杀目标会话。禁止因此改杀 server | 2026-08-29 |
| M021 | WMI 起 label daemon 后首条本地命令报哈希形 pipe 连不上 | WMI 返回时 daemon 还没绑 label pipe，client 回落哈希形名后报错 | WMI `new-session` 后轮询 `list-sessions` 直到 exit 0 再继续 | 2026-08-31 |
| M022 | WMI 单跑 `start-server` 后 daemon 立即消失 | exit-empty 语义：无 session 的空 server 自动退出 | WMI 启动命令直接用 `new-session -d`（daemon 加 keeper session 一步到位） | 2026-08-31 |
| M031 | 真 agent 在 `C:\Windows\System32` 打开、hook 全静默 | spawn 未设 pane 进程 cwd，继承 WMI daemon 的 System32；hook 的 `project_allows` 按 cwd 不匹配项目静默（安全带掩盖根因） | `ensure_session().working_directory(<项目根>)` 加 split builder `.cwd()`；`~/.claude.json` 信任键是正斜杠形态，清理误信任别用反斜杠查 | 2026-08-31 |
