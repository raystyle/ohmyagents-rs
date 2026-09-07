# PLAN：当前目标实施计划

## 当前目标：D08 doctor 检查面补形态

> 回指 `PRD.md` D08。用户 2026-09-07 裁定检查面留 oma doctor。触发项 Grok 状态栏，热修 M046 至 M048 后用户实证栏正常。本目标只补「写了配置仍报 ok」的漏检，不扩到 API key / MCP 新面（既有 login / yolo / trust.mcp 不动）。

### 事实基线

> 2026-09-07 盘点（本机 `oma doctor` 全绿，`doctor.blocked=false`）。

- 已有检查：登录态 / hook 形态（bare/absolute/none）/ 状态栏（含 marker）/ 会话健康；Codex argv 已 warn（M045）；Codex 项目 hook 信任只读项目表（M044）
- 漏检：Grok `[ui.status_line].command` 只要含 `oma-statusline` 即 ok，`pwsh -File "..."` 壳行 Windows 123 仍绿（M048）[实证： grok-build `command.rs` 只 NotFound 回落 shell]
- 漏检：JSON hook `command=oma` 加 `args` 数组仍判 bare，Grok 加载 Claude settings 时 ParserError（M047）[实证： 当日 hook 热修]
- 非 doctor 面：Nerd 字形（M046）走脚本 ASCII 路径，doctor 不扫 TUI

### 方案骨架

1. **Grok 状态栏三态**：`CmdPath`（`oma-statusline-grok.cmd` 单路径）/ `PwshFile`（`pwsh -File` 壳行）/ `Missing`。Windows 只认 CmdPath 为 ok，PwshFile 标 warn 指向 `oma agents statusline grok`；Unix 相反。
2. **JSON hook args**：ours 且 `args` 非空数组则形态 `args`，warn「command+args is PowerShell ParserError under Grok（M047）；oma init」。Grok 自己的整行命令不受影响。
3. **测试**：`grok_statusline_state` 三态；`json_hooks_form` 加 args 例。期望来自 grok-build spawn 规则与 M047 复现形态，不断本机路径。

### 验收口径

- 两条新单测绿；既有 doctor 单测绿
- 本机 `oma doctor` grok statusline 仍 ok（当前配置已是 CmdPath）
- R002 doctor 行补两形态；四原语同步

### 门禁

触碰文件：`rumdl check` 加 md 三件套；`cargo test --lib`。存量 clippy 告警不扩批。

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
