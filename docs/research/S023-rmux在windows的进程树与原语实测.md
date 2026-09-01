# S023：rmux 在 windows 的进程树与原语实测

- 日期：2026-09-01
- 关联：`S004`（进程模型旧口径）、`S022`；用户给定问题框架（进程树含 conhost 隔层、rmux-daemon.exe 守护、pane 恒有 shell）——本研究以本机活会话实测与 rmux 源码核实，**纠偏三处**
- 研究法：活体进程树（`Get-CimInstance Win32_Process` 的 PPID 链 + 命令行）+ rmux 主仓源码（`crates/rmux-client/src/auto_start.rs`、`crates/rmux-pty/src/{child.rs,backend/windows/flags.rs}`、`crates/rmux-os`）双向对照

## 一、实测现场

> 样本：oma 项目会话 daemon（pid 8572，oma 装的 0.10.0）四路 agent + 用户手动 daemon（pid 22580）两 pane。[实证: 2026-09-01 本机]

```text
（父进程已退出，daemon 孤儿化常驻）
rmux.exe --__internal-daemon \\.\pipe\rmux-S-1-5-21-...     pid 8572   ← daemon 本体
  ├─ conhost.exe --headless --width 120 --height 32 ...     ← ConPTY 宿主（per pane，尺寸=pane 尺寸）
  ├─ conhost.exe --headless --width 59  --height 32 ...
  ├─ conhost.exe --headless --width 60  --height 15 ...
  ├─ conhost.exe --headless --width 59  --height 15 ...
  ├─ claude.exe                                             ← pane 进程，直挂 daemon
  ├─ codex.exe
  ├─ grok.exe
  └─ kimi.exe

rmux.exe --__internal-daemon ...（另一个 -L daemon）        pid 22580
  ├─ conhost.exe --headless --width 80 --height 24 ...      ×2
  ├─ pwsh.exe                                               ← 该 daemon 的 pane 是 shell
  │    └─ kimi.exe / node.exe / ...                         ← shell 的子进程（用户命令）
```

## 二、对用户给定框架的三处纠偏

1. **「rmux-daemon.exe 守护进程」不成立（运行形态）**：包里确有 `rmux-daemon.exe`（源码 `src/daemon_main.rs` 独立 bin target），但实际运行的 daemon 全部是 **`libexec\rmux\rmux.exe --__internal-daemon <pipe>`**——客户端 auto-start 走 re-exec 同一二进制（`auto_start.rs` 的 `INTERNAL_DAEMON_FLAG` 与 `rmux_binary_path()`，优先解析当前 exe）。[实证: 进程命令行 + 源码]
2. **conhost 与 pane 进程是兄弟，不是隔层**：ConPTY 宿主（`conhost.exe --headless`）与 pane 子进程都直挂 daemon 名下（`child.rs` 的 `Command::new(program).spawn()` 由 daemon 发起；ConPTY 由 `CreatePseudoConsole` 创建，其 conhost 也是 daemon 子）。用户框架图把 shell 画在 conhost 之下——实测 PPID 全部指向 daemon。[实证]
3. **pane 不必有 shell 层**：pane 的「进程原语」就是 daemon 直接 CreateProcess 的任意程序——oma 的真 agent（claude/codex/grok/kimi）**无中间 shell 直挂 daemon**；shell（pwsh/cmd）只是 pane 程序的一种选择（stub 会话与用户手动 daemon 形态），shell 下再挂用户命令子进程。[实证]

用户框架中被证实的部分：conhost per ConPTY（每 pane 一个、headless、尺寸随 pane）；daemon 无控制台；多 `-L` daemon 各自独立进程树互不干扰；CLI 客户端即调即退。[实证]

## 三、源码核实的机制链

- **daemon 生命周期**：客户端 auto-start → `spawn_hidden_daemon`（`rmux_os::daemon::spawn_hidden_daemon_command_requiring_job_breakaway`）→ **drop(child) 故意不 wait**——daemon 必须比短命客户端活得久（孤儿化的来源）；Windows 侧用 `StartupReadyEvent` 同步就绪（2s 超时）。[实证: 源码]
- **job breakaway**：daemon 启动要求 Job Object breakaway——宿主在 Job 内且不许 breakaway 即 os error 5（oma 的 WMI 退路正是绕这个，两端同源）。[实证: 源码 + P0005 实战]
- **ConPTY flags**：`PSEUDOCONSOLE_RESIZE_QUIRK | WIN32_INPUT_MODE`（按需加 `PASSTHROUGH`）——解释 resize 行为与 win32 输入模式（oma 发键用的正是这条通路）。[实证: 源码 flags.rs]
- **控制台信号**：Ctrl+C 走 conhost 的进程组广播，非 Linux 的 process group + TTY 驱动——oma 禁对 codex 发 C-c 的守卫在此机制层。[经验: S005 旧口径，本次未重测]

## 四、windows 原语表

| 概念 | Linux | rmux on Windows（实测） |
| --- | --- | --- |
| 服务（daemon） | fork 后脱离会话的常驻进程，unix socket | `rmux.exe --__internal-daemon` re-exec 孤儿进程（libexec 路径），named pipe；job breakaway 必需 |
| 会话 session | daemon 内对象 | 同；oma 按项目 slug 命名（`oma-<slug>`），跨命令重连靠 manifest |
| 窗口 window | daemon 内布局对象 | 同（oma v1 每会话单窗口） |
| 窗格 pane | PTY 对 + 直接子进程 | **ConPTY（headless conhost）+ daemon 直接 CreateProcess 的子进程**；pane 尺寸即 conhost 启动参数 |
| pane 程序 | 恒为 shell | 任意程序（oma：agent 直挂；shell 是选项） |
| 伪终端 | forkpty 一步 | CreatePseudoConsole + EXTENDED_STARTUPINFO/PSEUDOCONSOLE attribute 附子进程 |
| 客户端 | rmux 命令临时 attach | 同（CLI/SDK 即连即退；SDK 是常连接，CLI 一次性） |
| 信号 | process group + TTY | conhost 进程组广播；resize 走 ConPTY resize API |

## 五、关键结论

1. Windows 侧真实开销是「每 pane 一个 headless conhost」而非「多一层进程嵌套」——兄弟进程、数量线性于 pane 数。[实证]
2. oma 的全部产品行为与该进程模型吻合：label per project = 多 daemon、cleanup 只 kill-session（不动 daemon 与其它树）、spawn 的 working_directory 直传 CreateProcess（M031 的根因层）、status 的 pid+进程名 locate 正是「pane 进程=daemon 直接子」的可观测面。[实证]
3. `rmux-daemon.exe` 在包里但 auto-start 不用——排查进程时按 `--__internal-daemon` 参数找，别按进程名找（oma doctor/status 若将来加 daemon 诊断需用此口径）。[实证]
