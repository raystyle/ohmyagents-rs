# S010-clum等待原语作为hook兜底状态

> 2026-08-29。学习 [YouMind d/vNdu7MLj7FPssJ](https://youmind.com/d/vNdu7MLj7FPssJ)（本地稿 `clum 等待原语实现分析报告.md`）。分析对象 [tddh/clum](https://github.com/tddh/clum) `68a90e4ab10dd571c03b834ca2a1a9a2d31feae9`（2026-08-28）。**首稿只读了 YouMind，未打开 clum 源码。** 同日浅克隆该 commit，对照本机 `rmux-sdk` 0.10.0 逐条核实。用户定调：等待原语是比 hook 更好的兜底状态原语。本仓不引入 clum 运行时。

## 需求

- 研究：clum 如何把「等一个终端」做成可观测、可恢复的状态迁移；`terminal_state` 八态与本仓 idle/working/blocked 怎么对齐。
- 核查：能不能替代 hook 当**兜底**（hook 不报、Stop 不触发、PermissionRequest 不存在时仍能判）。
- 意图：混合。本仓不引入 clum MCP / QUIC / bridge 运行时。

## 背景

本仓层 2 曾把各家 lifecycle hook 写成 `.ohmyagents/state` 当语义权威。已踩的洞：Codex `Stop` 常不触发、Claude 没有标准 `PermissionRequest`、hook 信任框、Windows 专用 pipe 对不上就静默丢。[经验: win-rmux hooks.md；本仓 `oma hook` 仍是可选上报口]

层 1 的 `Quiet` / `wait_until_stable_for` 只表示画面不变，**不是** agent idle。深思会假绿。[经验: 《S009-agent状态判断-通道与分层》]

用户要的兜底：设置和 flags 清不掉、hook 又报不出来时，编排器仍能从 PTY 读出「能发 / 在干活 / 卡确认或密码」，再决定 sendkeys 或跳过该路。

## 关键结论

clum 的价值不在远程 MCP，而在 **rmux-sdk 等待族 + 超时快照 + `terminal_state` 分类器**。对本仓：hook 降为可选加速通道；**hook 沉默时的状态权威改走等待原语**。[推断: 用户定调「替代 hook 的兜底」；clum 源码不提 Oh My Agents]

YouMind 对文件路径与主流程的描述，在 `68a90e4` 上成立；有一处注释过时，见下节「核实」。[实证: 2026-08-29 浅克隆 tddh/clum @ 68a90e4]

| 本仓原层次 | 旧做法 | 对齐 clum 之后 |
| --- | --- | --- |
| 0 存活 | pane pid / exists | 不变。`wait_exit` 只判进程死，不判忙闲 |
| 1 Drive 同步 | `Quiet` / `wait_until_stable_for` | 仍只给发键前静默。`stable_ms=0` 无意义，须拒绝 |
| **1b 终端语义兜底** | 无；曾误用 Quiet 或 CPU | **新建**：`expect_visible_text` / `wait_for` + `snapshot` + `terminal_state`。ready 才可发；confirm/password 当 blocked；running 当 working |
| 2 hook 文件 | 默认权威 | **可选**。事件到了就写 state 文件；没到不以 unknown 当 idle |
| 3 任务 | `.ohmyagents/tasks` | 不变 |

映射（启发式，不是 hook 事件）：

| `terminal_state` | 兜底四态 | 编排 |
| --- | --- | --- |
| ready | idle | 可 `send` |
| running | working | 不重复全文 |
| confirm / password | blocked | 停该路；password **禁止**往输入打密钥；confirm 才 sendkeys |
| repl / editor / pager | working 或 unknown | 不往 agent 输入框塞任务 |
| unknown | unknown | 不催、不猜 |

不引入 clum 进程、不引入 QUIC、不把 MCP 当控制面。本仓已有 SDK：`expect_visible_text().timeout()`、`wait_until_stable_for`、`output_stream`、`snapshot`。[实证: 2026-08-29 poc-drive / poc-dialogs 已用可见文本等待；rmuxpoc `default_timeout` 20s]

## 源码核实

> 下列断言均对照 `68a90e4` 文件，不再引用 YouMind 当实证。

MCP 层确是薄转发。`clum-mcp/src/tools/output.rs` 的 `wait_for_text` / `wait_exit`：缺 `host` 报错；`session_name` 默认 `"clum"`；`timeout_ms` 默认 `DEFAULT_WAIT_TIMEOUT_MS = 30_000`（`clum-core/src/lib.rs` L24–25）；`send_json_frame` 后 `recv_json_frame`，再 `enrich_pane_response` 与 audit。无等待循环。[实证: output.rs L9–68]

Bridge 等待在 `rmux-bridge/src/protocol/output.rs`。`ProtocolProxy` 两套连接：`rmux` 的 `default_timeout(30s)`，`rmux_long` 的 `default_timeout(Duration::MAX)`（`protocol/mod.rs` L35–44）。[实证: 该文件]

| 原语 | 核实结果 |
| --- | --- |
| `wait_for_text` | `expect_visible_text().to_contain(text).timeout(from_millis(timeout_ms))`。成功带 `terminal_state`+cursor；`RmuxError::WaitTimeout` 再 `snapshot()` 填 `partial_output`。注释写要避开 SDK 5s 假超时。L149–216 |
| `wait_stable` | `stable_ms==0` 直接拒绝。`wait_until_stable_for(stable_ms).timeout(timeout_ms)`。L348–376 |
| `wait_exit` | **未调用** `pane.wait_exit()`。100ms 轮询 `info()`；无 pane 返回 `exited:false`；有 `exit_state` 才 `exited:true`；`Exited` 无 exit_state 则 `exited:false`。L287–345 |
| `wait_for_bytes` | 走 `rmux_long`；参数名 `_timeout_ms` 未使用。`only_new` 时 `has_capability("sdk.waits.armed")` 则 `wait_for_next` 否则 `wait_for`。TOOLS.md 与 schema 均写超时未强制。L219–284 |
| `collect_until_exit` | 死窗格快路径；否则 `tokio::spawn` 收集 + 外层 `timeout`；超时 `abort.abort()` 再 snapshot。L415–530 |
| `stream_pane` | MCP `stream.rs`：魔数 `0x02`，缓冲 10000，满则 **丢新留旧**，超时文案 `stream_pane again to continue`。空闲 300s / keepalive 30s |

`detect_terminal_state`（`terminal_state.rs` L29–166）：去尾空行后最后 12 行；光标不可见先 editor/pager 否则 Running；password 关键词只扫 **tail**；confirm 次之；shell 提示符再加 col==0 则 Running。P0 单测：`[sudo] password` 且 col=0 仍是 Password（L235–239），因 password 规则在 shell 的 col 检查之前。英文关键词表属实，无中文「是否继续」。

**订正 YouMind / clum 注释。** `handle_wait_exit` 写「`pane.wait_exit()` 受 SDK 默认 5s（`V1_DEFAULT_TIMEOUT`）限制」。本机 `rmux-sdk` 0.10.0：`V1_DEFAULT_TIMEOUT` 确是 5 秒（`discovery.rs` L29）；`Pane::wait_exit` **没有** per-op `.timeout()`，只用 facade 的 `configured_default_timeout`（`wait.rs` L180–187）。但 clum 的 `self.rmux` 已经是 **30s** 默认，不是 5s。轮询的真实理由是：调用方 `timeout_ms` 可以大于 facade 30s，而 `wait_exit()` 吃不到这次请求的超时。[实证: protocol/mod.rs L35–38；rmux-sdk wait.rs；clum output.rs L307–309 注释过时]

本仓 `rmuxpoc` builder `default_timeout(20s)` 同样盖过 5s；单次等待仍应再写 `.timeout()`，与 clum 的 per-op 覆写同策略。[实证: `src/rmuxpoc.rs`]

## 原文在说什么

一条等待：AI 客户端 JSON-RPC 到 clum-mcp（只解析、路由、审计）到 rmux-bridge（真等待）到 rmux-sdk 到本机 daemon。MCP 层零等待语义。[实证: output.rs 如上]

六个原语：

| 原语 | SDK 落点 | 本仓用途 |
| --- | --- | --- |
| `wait_for_text` | `expect_visible_text().to_contain().timeout(ms)` | 等短头离开、等 Allow 文案 |
| `wait_stable` | `wait_until_stable_for(stable_ms).timeout(ms)` | Drive 前静默 |
| `wait_exit` | 绕开 `pane.wait_exit()`，100ms 轮询 `info()` | 桩进程/pane 死 |
| `wait_for_bytes` | `wait_for` / `wait_for_next`（capability `sdk.waits.armed`） | 网页镜像之外的字节门；报告写明超时参数未强制 |
| `collect_until_exit` | spawn 收集 + 外层 timeout；超时 abort 丢已收集字节 | 非 TUI 命令输出；coding agent 不适用 |
| `stream_pane` | 长连字节流 | 观察面，已规划 `poc-stream` |

工程三条（应抄契约，不抄仓库）：

1. **SDK 无 per-op 超时的调用会撞 facade 默认。** 未设 builder 时确是 5 秒 `V1_DEFAULT_TIMEOUT`。clum 对 text/stable 用链式 `.timeout(timeout_ms)`；对 bytes/collect 用 `rmux_long`（`Duration::MAX`）；对 exit 自轮询。注释里的「wait_exit 被 5s 卡住」与现码 30s facade 不完全一致，见上节订正。[实证: rmux-sdk discovery.rs；clum protocol/mod.rs]
2. **超时即信息。** 失败分支立刻 `snapshot()`，带回 `partial_output` + `terminal_state` + `cursor`。禁止超时后空着手再 attach 猜。
3. **文档与代码对齐缺陷。** `wait_for_bytes` 的 `_timeout_ms` 未用、属无限等；`collect_until_exit` 超时丢缓冲。本仓若用这两条必须自加看门狗。

`terminal_state` 输入：可见文本、光标列、光标是否可见。窗口取最后 12 行。优先级：password > confirm > editor > pager > REPL > shell 提示符。P0：`[sudo] password` 且 col=0 仍是 Password。[实证: terminal_state.rs L29–166、L235–239]

局限：规则按英文终端（`Password:`、`Are you sure`、`-- INSERT --`）。中文「是否继续？」「密码：」不在表里，会掉进 unknown 或误分类。[实证: password_keywords / confirm_patterns 无中文] 本仓假对话框用的是英文 `Allow this action?`，不能当成中文 TUI 已覆盖。[实证: poc-dialogs]

## 为什么这比 hook 更适合兜底

hook 是 agent 自愿上报。不上报时文件停在 working/unknown，编排器无法区分「还在深思」和「卡在确认框」。[经验: Codex Stop；Claude 无 PermissionRequest]

等待原语读的是 **已经画在 PTY 上的东西**，不依赖各家 hook 事件表、不依赖 hook 信任、不依赖 hook 进程找得到 `oma` 或 named pipe。残差对话框：`wait_for_text` 或 `terminal_state=confirm` 之后走已有 sendkeys，与《S009-agent状态判断-通道与分层》「报阻塞 / 点掉阻塞分路」仍成立，只是「报」可以从 snapshot 分类来，不必等 hook 写文件。

hook 仍有用：备屏 TUI 上确认文案可能不进 snapshot（win-rmux observe：claude/kimi capture 常空）。那时候 hook 或扫到的可见残行才是信号。顺序建议：

1. 有新的 state 文件且未 stale 用 hook
2. 否则 `snapshot` + `terminal_state`（及必要的 `wait_for_text`）
3. Quiet 只给 Drive 同步，永不映射 idle
4. 仍 unknown 则不委派、不重发全文

禁止把 clum 的 MCP 同步占连、QUIC 多跳、audit.db 带进 `oma`。本仓控制面已经是进程内 SDK。[推断: AGENTS 最少依赖；编排器自己就是等待调用方]

## 对本仓 POC 的落点

- `poc-state` 应证明：Quiet 超时 ≠ idle；`terminal_state=ready` 或可见提示符才算可发。不必先装 hook。**2026-08-31 已绿**：Quiet 静止成立（revision 不变）而 verdict 非 idle；confirm/password 从画面判出并分别以 y+Enter、裸 Enter 点掉。[实证: examples/poc-state.rs 退出 0]
- `poc-dialogs` 已是 `wait_for_text("Allow this action?")` + sendkeys；下一步是分类器而不是再加 hook 管道。**分类器已落 rmuxpoc `detect_terminal_state`（2026-08-31）。[实证]**
- `poc-stream` 继续走 `output_stream`，对照 clum `stream_pane` 的「超时文案引导续传」，不要把流当忙闲权威。**2026-08-31 已绿。[实证]**
- per-op `.timeout()` 必须显式，避免 SDK 5s 假超时。
- 中文确认框：关键词表要可扩展，英文规则不能当生产默认。

## 待办

- ~~本仓实现最小 `detect_terminal_state`（ready / running / confirm / password / unknown 即可；editor/pager/repl 可后补）~~ 2026-08-31 完成，rmuxpoc 共用层。[实证: 7 单测过]
- 单测覆盖：列 0 的 password（已覆盖）、行中的 `➜`（未做，本仓桩是 pwsh `PS>` 提示符，无生产默认价值）、中文「是否继续」（已覆盖，纯中文行落 Unknown 不误判 idle；注意 `是否继续？(y/n)` 含 ASCII 标记会正确命中 Confirm，测缺口必须纯中文）
- editor / pager / repl 三态后补；中文关键词表生产前再扩
- 备屏 TUI 上 snapshot 为空时，兜底失效，仍要 hook 或禁止乱发
- 不把 `tddh/clum` 加进 Cargo.toml
