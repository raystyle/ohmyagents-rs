# R006-rmux开发参考-连接会话布局与驱动

> 角色：写 `oma` 的 rmux 相关代码时查的**做法参考**（要做什么怎么做）。从 `docs\research\` 的 rmux-sdk、win-rmux、drive、clum 四篇研究与 POC 实证浓缩；出错排查见 `INDEX.md（mistakes 节）`，证据链见研究原文。全部条目可溯源六态。

## 一、连接与端点

1. **SDK 路线用显式专用端点**：`RmuxEndpoint::WindowsPipe(r"\\.\pipe\rmux-oma-<hash>")` / `UnixSocket(<dir>/socket)`，不用 `Default`（会连上用户已有 daemon）。`Rmux::builder().endpoint(ep).connect_or_start()`。[实证: 2026-08-29 poc-endpoint]
2. **Job Object 退路**：宿主在 job 内（agent / CI）`connect_or_start` 报 os error 5 时，WMI `Win32_Process.Create` 在 job 外拉 `libexec\rmux\rmux.exe --__internal-daemon <pipe>`，再 `connect()`。见 `src\rmuxpoc.rs`。[实证: 2026-08-29 WMI 路线绿]
3. **CLI 路线用 `-L <label>` 专用端点**：Windows CLI `-S` 无条件拒绝一切形态；`-L <pid+tag>` 的 pipe 名为 `\\.\pipe\rmux-S-<SID>-il-medium-<label>`。daemon 启动命令必须直接 `new-session -d`（`start-server` 会因 exit-empty 立即退出）；WMI 起 daemon 后轮询 `list-sessions` 等 ready。[实证: 2026-08-31 poc-paste]
4. **SDK `cmd()` 在 Windows 不可用**（注入 `-S` 必被拒）；SDK 无公开裸 Request 口（`transport()` 为 `pub(crate)`）。paste 等命令自 spawn CLI。[实证: 2026-08-31 读 0.10.0 源码与实测]

## 二、会话与布局

5. **spawn 用 `EnsureSession::named(..).create_only().detached(true)`**：撞名报错不静默叠窗格；`send`/`status` 用 `reuse_only()`。禁止默认 CreateOrReuse。[实证: 2026-08-29 poc-session]
6. **2x2 布局**：`ensure_session` 出 0.0；`0.0.split_with(Right)` 出 0.1；`0.0.split_with(Down)` 出 0.2；`0.1.split_with(Down)` 出 0.3。每步带该格 `ProcessSpec`（argv），不先分空壳再 spawn。[实证: 2026-08-29 poc-layout]
7. **argv 直 spawn 不包壳**：`[pwsh, -NoProfile, -NoExit]` / agent 原生 exe；`.bat`/`.cmd` 被拒。keep-alive 防止 agent 崩了格消失（`remain-on-exit` 默认 off）。[实证: poc-layout；经验: win-rmux]
8. **进程模型**：安装保留 `rmux.exe` + `libexec\rmux\rmux.exe` + `rmux-daemon.exe` 三件；进程内设 `RMUX_DISABLE_TINY_CLI=1`。[经验: win-rmux environment.md；实证: oma check 布局]

## 三、驱动

> drive

9. **短 ASCII**：`pane.send_text(text)` 后单独 `pane.send_key("Enter")`。`send_text` 字面 UTF-8、不隐式换行、不解析键名。文本和 Enter 禁止同发。[实证: 2026-08-29 poc-drive]
10. **长文与中文**：全 CLI 三段式——payload 写临时文件（UTF-8 无换行无 ESC）→ `load-buffer -b <name> <file>` → `paste-buffer -p -b <name> -t <session>:0.0` → `send-keys Enter` → `capture-pane -p` 轮询验证。发送侧永不自包 `\x1b[200~`。[实证: 2026-08-31 poc-paste 中文绿]
11. **发前扫框**：驱动前检查画面有无确认框（`expect_visible_text` / DIALOGS 模式），有则先点掉（`y` + Enter）。[经验: evo-harness `_sweep_dialogs`]
12. **超时只补 Enter**：quiet 超时不重发全文；禁止对 Codex 发 `C-c`（单次即退出）。[经验: evo-harness + win-rmux 2026-08-21]

## 四、状态与等待

13. **分层判断**（详细见 `research\S009-agent状态判断-通道与分层.md`）：0 存活（pid）→ 2 语义（hook 文件，可选加速）→ 1b 终端语义兜底（`terminal_state` / `wait_for_text`）→ 3 任务。Quiet 只给 Drive 同步，不当 idle。[推断: 分层对照；实证: poc-dialogs]
14. **SDK 等待**：`pane.expect_visible_text().to_contain(..).timeout(..)`；per-op 超时用 `.timeout(Duration)`（V1 默认 5s）。[实证: poc-drive；clum 源码核实]
15. **观察**：网页镜像走 `output_stream_starting_at(Oldest)` 字节流；结论写文件不写屏幕（备屏 capture 常空）。[经验: web-claude-demo + win-rmux]

## 五、清理与环境

16. **cleanup 只 `kill-session -t <本会话>`**：不 `kill-server`；`-L` label daemon 随末 session 自动退。临时 buffer `delete-buffer`、payload 文件删除。[实证: 各 POC 收尾]
17. **环境**：spawn 前清 `NO_COLOR`、设 `TERM=xterm-256color`；PATH 注入 dispatcher 目录。[实证: rmuxpoc::prepare_env]

## 六、禁止清单

> 写码时红线

- 文本和 Enter 同发；对 Codex `C-c`；自包 bracketed paste
- `CreateOrReuse` 默认；先分空壳再 spawn；`.bat` agent
- `kill-server` 进主路径；查询失败杀 server
- 把 `wait-pane Quiet` 当 idle；CPU 当主判据
- 依赖 `pane_current_command` 做 locate（用 pid 反查）
