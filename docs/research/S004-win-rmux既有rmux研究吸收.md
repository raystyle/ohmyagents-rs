# S004-win-rmux既有rmux研究吸收

> 2026-08-29 全量吸收，2026-08-31 压缩重写：约六成结论已由专项研究、MISTAKES 与本机 POC 实测承载，本篇只留 win-rmux **独有**的四块事实（仓库地图、进程模型与独有行为行、hardened-guard 八条、命令核验表）。通用结论请查专项：《S005-drive铁律与三段式粘贴》《S009-agent状态判断-通道与分层》《S003-rmux-sdk最佳开发实践与验证poc》《S007-yolo与无阻塞启动-配置落盘与无头分路》与 `INDEX.md（mistakes 节）`。

## 仓库地图（吸收源）

| 层 | 路径 | 角色 |
| --- | --- | --- |
| 协作规则 | `AGENTS.md` | 实测坑摘要；pwsh 7 |
| 产品入口 | `skills/win-rmux/SKILL.md` | 六原语 + 前置守卫 + yolo 表 |
| 用法研究 | `references/rmux-usage.md` | daemon 模型、launcher、NO_COLOR |
| 坑表 | `references/troubleshooting.md` | launch / drive / 备屏 / agent |
| 命令面 | `commands.md` + `command-verification.md` | `list-commands` 全表；真实 server 实测 |
| 扩展 | `extensions.md` 等 | wait-pane、find-panes、`-H`、paste-buffer `-p` |
| 安全复核 | `.rmux_tasks/skill-review/` + `poc-claude/hardened-guard.ps1` | 2026-08-21 评为 HIGH 的守卫修正 |

[实证: 2026-08-29 读 `D:\sourcecode\win-rmux` SKILL 与 14 篇 references]

## 进程模型与独有行为行

- 进程三层：公开 `rmux.exe` 是 tiny 分发器；完整实现 `libexec\rmux\rmux.exe`；daemon 是 `libexec\rmux\rmux.exe --__internal-daemon <pipe>`。安装不能只拷一个 exe。[实证: win-rmux environment.md + 本仓 check 布局一致]
- `show-environment` 只显示显式 set 的键，**看不见继承的 API key**——不要用它判断密钥是否在。[经验: win-rmux]
- `stream-pane` 持续阻塞不自行退出（核验表 TIMEOUT）；`collect-pane-output` 缺 `--until-pane-exit` 则 ERR。勿前台裸跑。
- `remain-on-exit` 默认 off：进程退即关 pane，spawn 要 keep-alive，agent 崩了格就没了。
- `respawn-pane -k` 对 Codex 实测不稳、布局重排：不要靠 respawn 救 Codex pane。
- `--wait` 本版只支持 `quiet`；`--wait-text` 是独立开关。
- 默认 shell `show-options` 写 `cmd.exe` 但无命令新格实测常是 `pwsh.exe`：始终显式 argv spawn。
- 长 prompt 用 send-keys 会截断：写文件再短指令 `Read <path>`；中文 send-keys 乱码走 paste-buffer UTF-8（本仓 2026-08-31 已实证全 CLI 路线）。
- `rmux claude` teammate 要 Git Bash、内层 socket 随机外层看不见：`oma` 自己 spawn `claude`。

## hardened-guard 八条硬约束

`.rmux_tasks` 安全复核（2026-08-21）把 SKILL 出厂守卫评为多条 HIGH，POC `hardened-guard.ps1` 是修正形状。`oma` 默认对齐 POC：

1. 永不启发式 `kill-server`：区分无进程 / 无 server / 查询失败 / 正常，查询失败不杀。
2. 会话带所有权标记；别人的 session 不碰；自己残留也要显式 `--force` 才杀。
3. hook 安装默认不改用户家；项目级 opt-in。
4. User 环境只 allowlist 密钥，不整表拷贝（`refresh-user-env.ps1` 是反例）。
5. launch 先直接 `new-session -d`；`os error 5` 且交互桌面才退 wt；无头 fail-fast。
6. 固定 sleep 换截止轮询：pane 数 + 进程名齐才算就绪。
7. locate 不匹配硬中止，不 warn-and-continue（会把 prompt 打进裸 pwsh）。
8. 生成的 launcher 写临时目录不写仓库 cwd；unit 名校验 `[A-Za-z0-9_-]+`。

[实证: win-rmux `research-claude.md` + `hardened-guard.ps1` 2026-08-21]

## 命令核验表要点

`command-verification.md`（2026-08-20，rmux 0.10.0，Windows 10.0.26）是 headless 真实 server 实测，不是命令目录。

对 `oma` 有用的 OK：`new-session` / `split-window` / `list-*` / `has-session` / `send-keys` / `capture-pane` / `paste-buffer -p` / `load-buffer` / `set-environment` / `wait-pane` / `find-panes` / `find-sessions` / `pane-snapshot` / `broadcast-keys`。

预期 ERR / TIMEOUT（不是 bug）：无 attach 时 `attach-session` / `display-menu` / `command-prompt`；`stream-pane --lines` TIMEOUT；`collect-pane-output` 缺旗标 ERR；`rmux claude` 无 Git Bash ERR；`setup tmux-shim` 仅 Unix。

扩展命令 `wait-pane` / `find-panes` / `stream-pane` **不在 `list-commands` 里但可用**——不要用 `list-commands` 当扩展命令存在性的唯一判据。

[实证: win-rmux command-verification.md 两轮；本仓 paste 路线 2026-08-31 复证实测]

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本地 | `D:\sourcecode\win-rmux`（skill v0.1.5，标 rmux 0.10.0）：SKILL、references 14 篇、hooks/scripts、`.rmux_tasks` | 2026-08-29 | 全部吸收源 |
| github | `Helvesec/rmux` v0.10.0 | 2026-08-04 | 官方布局与逃生开关对照 |

## 缺口

- `send-keys -H 0d` 核验表无行（裸 send-keys OK）；本仓已定案 SDK 用 `send_key("Enter")`，CLI 侧用键名 `Enter`，`-H` 不再需要。
- win-rmux 标 0.10.0 与本仓 pin 一致；装新版后应重新核，不把核验表当永远快照。
