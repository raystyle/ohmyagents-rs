# rmux-sdk最佳开发实践与验证poc

> 2026-08-29。研究 `rmux-sdk` 0.10.0 的官方用法，对照本仓 `oma` 原语，准备可逐步跑通的验证 POC。闸门 `oma check` 本机 Windows 已装 0.10.0。现役实施见方案 0005，代码落 `examples/poc-*`。

## 需求

- 研究：Rust `rmux-sdk` 官方推荐怎么连 daemon、建会话、分窗、发键、等待、收流、清理；哪些是一等 API，哪些必须走协议/CLI 逃生舱；怎样和本仓 launch / locate / drive / observe / judge / cleanup 对齐。
- 核查：
  1. `rmux-sdk` 最新 crates.io 是否就是 0.10.0，是否必须与 daemon 同版本。
  2. SDK 是否提供一等 `load-buffer` / `paste-buffer -p` / `send-keys -H`。
  3. 官方是否要求 `connect_or_start` + `EnsureSession`，以及 CreateOrReuse 会不会静默叠窗格。
  4. Windows 端点是否用 `RmuxEndpoint::WindowsPipe`，默认端点是否会和别人的 daemon 撞车。
  5. `wait_until_stable_for` / Quiet 是否等于 agent idle。
  6. Drop `Pane` 会不会杀进程；cleanup 该调什么。
- 产物：一套按 `oma` 操作排序的验证 POC 清单与最小 Rust 草稿（先文档，后代码）。

意图：混合。锚点：`Helvesec/rmux` v0.10.0、`docs/scripting-sdk.md`、docs.rs `rmux-sdk` 0.10.0、`Helvesec/rmux-demos`、本仓 drive / win-rmux 吸收 / 跨平台文。

## 结论

### 1. 官方分层：代码用 SDK，交互用 CLI

`docs/scripting-sdk.md` 写明：SDK 通过 typed IPC 跟本地 daemon 说话，**不是** CLI 解析器，也不是 tmux control-mode 包装。交互式 tmux 兼容工作流用 CLI；**代码是用户**时用 `rmux-sdk`。[实证: 2026-08-29 对照 rmux-sdk 0.10.0 源码 + Windows POC-1..4]

本仓 `oma` 是编排器，代码是用户。默认路径是 SDK。只有 SDK 没暴露的命令（paste-buffer、load-buffer）才走逃生舱。不要像 win-rmux 那样全程 `rmux.exe` 子进程。

crate 约束（`crates/rmux-sdk/src/lib.rs`）：`rmux-sdk` 是 public peer，**禁止**把 `rmux-client` / `rmux-core` / `rmux-server` / `rmux-pty` 当普通依赖。身份类型从 `rmux-proto` 再导出，调用方只 import `rmux_sdk`。

依赖钉死：

```toml
rmux-sdk = "=0.10.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

0.10.0 与 daemon 0.10.0 同发。release note：0.10 **不**与 0.9.x daemon 线兼容。SDK crates.io 发布日 2026-08-05，比 GitHub tag 晚一天。本机 rustc 1.97.1 覆盖 MSRV 区间。

### 2. 最佳实践清单（给 `oma`）

1. **专用端点，不要 Default。** `RmuxEndpoint` 是 `non_exhaustive`：`Default` / `UnixSocket(PathBuf)` / `WindowsPipe(String)`。Default 走平台发现，会连上用户已有 daemon。本仓按项目 hash 独立 pipe / socket（见《跨平台与无浏览器》）。
2. **`connect_or_start` 可以，但 Job Object 要退路。** 官方 quickstart 用 `Rmux::builder().connect_or_start()`，没有 TTY 也能起 hidden daemon。win-rmux 实测宿主在 Job Object 里 `new-session -d` 报 os error 5。POC 先直接连；失败再 `--via-wt`。
3. **会话策略按命令分。** `EnsureSessionPolicy`：`CreateOnly`（重名当错误）、`CreateOrReuse`（`new-session -A`）、`ReuseOnly`。官方例子用 CreateOrReuse。本仓 **spawn 用 CreateOnly**，撞名拒绝或 `--force` 后 `kill-session`；`send` / `status` 用 ReuseOnly。禁止默认 CreateOrReuse（会静默贴到别人的会话）。
4. **argv spawn，不要壳。** `Pane::split_with(dir).process(ProcessSpec)` 把命令和 split 一次送出，避免中间默认 shell。`spawn(argv)` 不给交互壳追加换行。Windows 上 `.bat`/`.cmd` 被拒，agent 必须是原生 exe。
5. **文本和 Enter 分发。** 官方 `sdk_demo_send_keys.rs`：`send_text("uname -s")` 然后 `send_key("Enter")`。`send_text` 明确：字面 UTF-8、不解析键名、**不隐式换行**。键名用 `Enter`，不用 `C-m`。
6. **Paste 不是一等 Pane API，Request 逃生舱也不通。** 见核查 2。0.10.0 源码核实：`RmuxCommandKind::Request` 是纯 DTO（`into_request` 只转换、不发 IPC），唯一的发送口 `Pane::transport()` 是 `pub(crate)`。Windows 上 `Rmux::cmd()` 又注入 `-S <pipe>`，而 CLI 对一切 `-S` 形态无条件拒绝（含 `\\.\pipe\rmux-...`）。因此长文 / 中文粘贴走自 spawn CLI：`-L <专用label>` + `load-buffer` + `paste-buffer -p`，label pipe 名为 `\\.\pipe\rmux-S-<SID>-il-medium-<label>`（label 明文后缀）。发送侧不自包 `\x1b[200~`。[实证: 2026-08-31 poc-paste Windows 绿；`-S` 拒绝用 pinned 0.10.0 二进制多形态实测]
7. **等待分层。** `wait_until_stable_for` / `expect_stable` = Quiet（画面不变），官方写明不推断 prompt。`wait_for_text` 是客户端轮询 snapshot。agent 忙闲仍读 `.ohmyagents/state`。Drive 同步可用 Quiet；judge 不能用 Quiet。
8. **定位用稳定 id + pid。** `pane.id()` 得 `%N`；`info()` 含 running pid。`foreground_state` 官方是 best-effort，Windows 报 ConPTY 根进程，**不分类 agent 名**。locate 仍按 win-rmux：pid 反查进程名。
9. **镜像走字节流，不走 capture。** `output_stream` / `output_stream_starting_at(Oldest)` 是 web-claude-demo 路径。`snapshot()` 是 live grid，备屏 TUI 仍可能空。观察面用 stream；结论写文件。
10. **Drop 是 inert。** `Pane::close` 才杀格；`detach` / drop 不碰 daemon。cleanup = 关本 session，不 `kill-server`。
11. **能力协商。** `Rmux::capabilities()` 或 `rmux capabilities --json`。流恢复要 `Pane::recover_output` / `surface_stream`（0.10 新，capability-gated）。
12. **环境。** spawn 前清 `NO_COLOR`，设 `TERM=xterm-256color`；`RMUX_DISABLE_TINY_CLI=1` 留给 CLI 逃生舱。SDK 本身不走 tiny CLI。

### 3. 对照本仓原语：SDK 调什么

| `oma` 操作 | SDK | 不要 |
| --- | --- | --- |
| `check` | 本机有 `rmux`；`Rmux::capabilities()`；四路 agent 在 PATH | 假装 SDK 能代替二进制安装 |
| `spawn` | 专用 `WindowsPipe`/`UnixSocket` + `connect_or_start`；`EnsureSession` CreateOnly + detached；`split_with` 三次成 2x2；`ProcessSpec` argv + `-e` 身份 | CreateOrReuse；上 2 下 1 的第三次 `-f -v`；默认壳 |
| `doctor` / locate | `pane.exists` / `id` / `info` pid；读 `.ohmyagents/state` | `foreground_state` 当 agent 名；Quiet 当 idle |
| `send` / `run` drive | 短 ASCII：`send_text` + `send_key("Enter")`；长/中文：Request LoadBuffer + PasteBuffer，再 `send_key("Enter")`；超时只补 Enter | 文本和 Enter 同发；对 Codex `send_key("C-c")`；自包 bracketed paste；`PaneSet::broadcast` 当生产 drive |
| `status` | ReuseOnly 连已有 session；读任务 JSON + state 文件 | attach；capture 猜 TUI |
| 观察网页 | `output_stream_starting_at(Oldest)` 二进制进 WS | `web-share` 当默认；`capture_pane -p` |
| `cleanup` | session close / kill-session | `kill-server`；drop handle 当清理 |

四路 2x2 布局（相对 win-rmux 上 2 下 1）：

1. `ensure_session` 出 pane 0.0
2. `0.0.split_with(Right)` 出 0.1
3. `0.0.split_with(Down)` 出 0.2
4. `0.1.split_with(Down)` 出 0.3

每步 `split_with` 带该格 agent 的 `ProcessSpec`，不要先分空壳再 spawn。

### 4. 核查结果

| 主张 | 结论 |
| --- | --- |
| crate 0.10.0 必须配 daemon 0.10.0 | 成立。crates.io `newest_version=0.10.0`（2026-08-05）；GitHub release 2026-08-04 写明不与 0.9.x 线兼容 |
| SDK 有一等 paste-buffer / load-buffer / send-keys -H | **不成立**（paste/load）。`Pane` 只有 `send_text` / `send_key`；协议层 `rmux_proto::Request` 有 `LoadBuffer` / `PasteBuffer`。**且 Request 逃生舱不可达**：`RmuxCommandKind::Request` 是惰性 DTO，`transport()` 为 `pub(crate)`，外部无法发裸 Request。[实证: 2026-08-31 读 0.10.0 源码] `Rmux::cmd()` 注入的 `-S <pipe>` 在 Windows 被无条件拒绝，CLI 逃生舱同样经 SDK 不可用。[实证: 2026-08-31] 实际通道是自 spawn CLI `-L <label>`。`-H` 是 CLI 旗标，SDK 用 `send_key("Enter")` 即可，不必 hex |
| 官方 quickstart 是 connect_or_start + EnsureSession | 成立。但例子用 CreateOrReuse；本仓 spawn 改 CreateOnly |
| Default 端点会撞别人的 daemon | 成立（发现策略）。本仓必须显式 pipe/socket |
| Quiet 等于 idle | **不成立**。`wait_for_load_state(Quiet)` 文档：画面稳定，不推断 prompt。与《rmux状态判断与hook补充》一致 |
| Drop Pane 杀进程 | **不成立**。`close` 才杀；drop/detach inert |

### 5. 不要从 demo 抄进 POC 的

| 来源 | 不要 |
| --- | --- |
| `scripting-sdk.md` 例子 | `send_text("printf ...\\n")` 把换行塞进文本；本仓 Enter 单独 `send_key` |
| broadcast-demo | 自包 bracketed paste；缺 CLI 注水凑格 |
| demo-orchestration | Claude 当编排器；`C-m` |
| web-claude-demo | 默认 attach 占终端；听 `0.0.0.0` |
| win-rmux | 全局 hook、启发式 kill-server、整表 User env、固定名 `execution-unit` |

### 6. 验证 POC 清单（装上 rmux 后按序跑）

闸门：`oma check` 非 0 则整组 skip。与方案 0005 同一条件。

会话名建议 `oma-poc-<yyyyMMdd>-<n>`，pipe `ohmyagents-poc-<hash>`。每个 POC 结束必须 `kill-session`，禁止 `kill-server`。

#### POC-0 闸门

验收：PATH 有完整包（`rmux.exe` + `libexec\rmux\rmux.exe`）；`rmux capabilities --json` 可解析；`claude`/`codex`/`grok`/`kimi` 可先不做（本步只核 rmux）。

#### POC-1 专用端点连接

验证：`Rmux::builder()` 显式 `WindowsPipe`（或 Unix socket）+ `connect_or_start`；`capabilities()` 成功；`endpoint()` 不是 Default。

```rust
use rmux_sdk::{Rmux, RmuxEndpoint};

#[tokio::main]
async fn main() -> rmux_sdk::Result<()> {
    let endpoint = if cfg!(windows) {
        RmuxEndpoint::WindowsPipe(r"\\.\pipe\ohmyagents-poc".into())
    } else {
        RmuxEndpoint::UnixSocket("/tmp/ohmyagents-poc/socket".into())
    };
    let rmux = Rmux::builder()
        .endpoint(endpoint) // 方法名以 0.10.0 docs 为准，装上后核
        .connect_or_start()
        .await?;
    let _caps = rmux.capabilities().await?;
    Ok(())
}
```

`builder().endpoint(...)` 的准确方法名以 docs.rs `Rmux` builder 为准（本轮该页 404，见缺口）。连接失败（os error 5）记 mistakes，不要立刻 kill-server。

#### POC-2 CreateOnly 会话 + cleanup

验证：`EnsureSessionPolicy::CreateOnly` 建 `oma-poc-2`；第二次同名必须失败；`ReuseOnly` 能拿到第一次；cleanup 后 `list-sessions` 无此名；其它 rmux 会话仍在。

#### POC-3 2x2 split_with + argv

验证：四格都是 `pwsh -NoProfile -Command 'echo PANE<n>'`（先不用四路 agent）；`pane(0,n).info()` pid 非空；进程名含 pwsh。禁止先 split 空壳再 spawn。

#### POC-4 drive 两段式

验证：对 0.0 `send_text("echo OMA-POC-4")` 再 `send_key("Enter")`；`wait_for_text("OMA-POC-4")` 或 `expect_visible_text().to_contain(...)`。禁止同一次带换行。超时只再 `send_key("Enter")`。

#### POC-5 paste 逃生舱

验证：经 `RmuxCommandKind::Request` 发 LoadBuffer + PasteBuffer（`-p` 语义）；中文 payload 出现在 pane；发送侧字符串不含 `\x1b[200~`。若 Request 构造过重，退 CLI：`RMUX_DISABLE_TINY_CLI=1` + `load-buffer` + `paste-buffer -p` + `send_key("Enter")`。

**2026-08-31 结果（Windows 绿）**：Request 逃生舱不可达（见核查 2），SDK `cmd()` 在 Windows 因 `-S` 注入必败。实际路线是全 CLI：`-L <pid+tag label>` + `new-session -d`（WMI 在 job 外起 daemon 与 keeper session）+ `load-buffer -b` + `paste-buffer -p -b -t session:0.0` + `send-keys Enter` + `capture-pane -p` 轮询中文 marker。三段式与「发送侧无 ESC 壳」均达成；daemon 随末 session 退出（exit-empty），无需显式 kill-server。[实证: `examples/poc-paste.rs` 退出 0]

#### POC-6 locate pid

验证：四格 `info()` pid 与 `Get-CimInstance Win32_Process`（或 `/proc`）进程名一致；故意错位 `send_key` 前必须 throw，不能 warn-and-continue。

#### POC-7 字节流观察

验证：`output_stream_starting_at(Oldest)` 收到非空字节；POC-4 的 echo 能在流里看到。这是以后网页镜像的最小核。不在 POC 里开 HTTP。

#### POC-8 负例（必须失败）

- 对「假 Codex」pane（本步可用 pwsh 代替）发 `send_key("C-c")` 的代码路径在 review 中禁止合入（真 Codex 等四路 POC）。
- Quiet 超时不得映射成 idle。
- cleanup 后 `Get-Process rmux` 仍可因其它会话存在；只断言本 session 名消失。

四路真实 agent 的 POC 放在 POC-1..7 全绿之后，另开 `oma spawn` 实现，不塞进第一批。

### 7. 落地顺序

1. 安装 rmux 0.10 完整包，写回 overview 安装路径（`D:\ohmyenv\rmux` 已过时）。
2. 本仓 `Cargo.toml`：bin `oma` + `[[example]]` 对应 POC-1..7。
3. 环境：进程内 `RMUX_DISABLE_TINY_CLI=1`，清 `NO_COLOR`。
4. 每个 example 退出前 kill-session；`cargo test` 不默认起 daemon。
5. POC 全绿再写 `oma check` / `cleanup` 产品命令。

## 事实源

| 类型 | 定位 | 日期 | 对应 | 提供了什么 |
| --- | --- | --- | --- | --- |
| github | `Helvesec/rmux` `docs/scripting-sdk.md` | 0.10.0 | 研究 1、核查 3 | CLI vs SDK；capabilities；crate 例子列表 |
| github | `crates/rmux-sdk/src/lib.rs` | main / 0.10 | 研究 1 | 禁止依赖内部 crate；quickstart |
| github | `crates/rmux-sdk/examples/sdk_demo_send_keys.rs` | main | 研究 2、核查 2 | `send_text` 与 `send_key("Enter")` 分发 |
| web | <https://docs.rs/rmux-sdk/0.10.0/rmux_sdk/handles/struct.Pane.html> | 2026-08-05 | 研究 2、核查 5-6 | send_text 无隐式换行；split_with；close vs drop；Quiet；stream；foreground 不分类 agent |
| web | <https://docs.rs/rmux-sdk/0.10.0/rmux_sdk/command/enum.RmuxCommandKind.html> | 2026-08-05 | 核查 2 | 无 paste/load 变体；有 Request 逃生舱 |
| web | <https://docs.rs/rmux-sdk/0.10.0/rmux_sdk/types/enum.RmuxEndpoint.html> | 2026-08-05 | 核查 4 | Default / UnixSocket / WindowsPipe |
| web | <https://docs.rs/rmux-sdk/0.10.0/rmux_sdk/ensure/enum.EnsureSessionPolicy.html> | 2026-08-05 | 核查 3 | CreateOnly / CreateOrReuse / ReuseOnly |
| web | crates.io `rmux-sdk` API | 2026-08-05 | 核查 1 | 0.10.0 newest；依赖 rmux-proto 0.10.0 |
| github | `Helvesec/rmux` release v0.10.0 | 2026-08-04 | 核查 1 | 与 0.9.x 不线兼容；recover_output |
| github | `rmux-proto` Request PasteBuffer / LoadBuffer | main | 核查 2 | 协议有、SDK Pane 无一等封装 |
| 本地 | 本仓 drive / win-rmux吸收 / 跨平台 / 状态通道 | 2026-08-29 | 研究 1、POC | 原语与硬约束 |
| x | keyword `rmux-sdk` | 2026-08-29 | 全部 | 仅项目介绍转发，无 SDK 实践增量 |

## 缺口

- docs.rs `rmux_sdk::Rmux` / builder 页本轮 404；POC 已核 `Rmux::builder().endpoint(RmuxEndpoint).connect_or_start()` / `.connect()`。
- `CommandRun` 本轮 code search 空；逃生舱以 `RmuxCommandKind::Request` 为准，CLI 子进程是第二退路。
- `send-keys -H` 在 SDK 中无对应 API；Enter 用 `send_key("Enter")`。装上 rmux 后再决定 CLI 逃生舱要不要 `-H 0d`。
- Windows POC-1..4 + dialogs + paste 已绿（2026-08-29/31）。Job Object 下 `connect_or_start` 报 os error 5，改 WMI 拉起 daemon 再 `connect()`。**订正**：专用 pipe 的 `\\.\pipe\rmux-...` 前缀只对 SDK `WindowsPipe` 端点与 `--__internal-daemon` 成立；CLI `-S` 对一切形态无条件拒绝（2026-08-31 实测推翻前句「只要前缀」的说法），CLI 侧专用端点只能 `-L <label>`。Linux/mac 未跑，委托后续仓库。
- locate / stream / Quiet-idle 负例尚未跑。paste 的 SDK 通道（Request 逃生舱）在 0.10.0 不可达，等上游暴露公开发送口再回补。
- X 对本题无贡献。
- 未 diff `Helvesec/rmux-demos` 当前树与本仓引用的 web-claude-demo API 是否仍叫 `output_stream_starting_at`。
