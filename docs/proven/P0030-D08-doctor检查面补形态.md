# P0030：D08 doctor 检查面补形态

> 2026-09-07 当日闭环。缘起：用户裁定「ome只管软件部署；oma 要对 hook、状态栏和 agent 配置的检查」（D08）。触发项 Grok 状态栏显示错误。热修 M046 至 M048 后用户实证栏正常；盘点 `oma doctor` 把「写了配置」当 ok，漏掉不可 spawn 的 command 串与 command+args。同日 D09 钉 oma 不管种子。

## 方案

### 1. Grok 状态栏 command 三态

> `src\doctor.rs` `grok_statusline_state`。

- 形态：`CmdPath`（路径以 `oma-statusline-grok.cmd` 结尾且不含 `pwsh`）/ `PwshFile`（含 `pwsh` 与 `-File`）/ `Missing`。
- Windows 只认 CmdPath 为 ok；PwshFile 标 warn，指向 `oma agents statusline grok`。Unix 相反。
- 依据：grok-build `command.rs` 先 `Command::new(整串)`，只 `NotFound`（Unix 另加 ENOEXEC）才回落 shell。`pwsh -File "..."` 含引号，Windows 报 ERROR_INVALID_NAME 123，栏上画出启动失败（M048）。

### 2. JSON hook args 形态

> `json_hooks_form`。

- ours 且 `args` 非空数组则形态 `args`，warn「command+args is PowerShell ParserError under Grok（M047）；oma init」。
- Grok 加载项目 `.claude/settings.json`；`command=oma` 加 `args=["hook",...]` 经 PowerShell 变成 `"oma" hook` 即 ParserError。整行 `oma hook --agent <名>` 仍判 bare。

### 3. 非 doctor 面

- Nerd 字形（M046）走脚本 ASCII 路径，doctor 不扫 TUI。
- API key / MCP 新面不扩（既有 login / yolo / trust.mcp 不动）。

## 验收

- 单测 `grok_statusline_state_classifies_cmd_pwsh_missing` 与 `json_hooks_form` args 例绿；`cargo test --lib` 141 过。
- 本机 `oma doctor`：`grok statusline ok`（配置已是 CmdPath）、`hooks.form=bare`、`doctor.blocked=false`。
- 用户实证 M048 修复后 Grok 状态栏正常（须重启会话读配置）。
- R002 doctor 行补两形态；四原语同步。

## 实施过程与经验

- 热修顺序是 Codex 信任空表（M044）、Codex 状态栏 argv（M045）、Grok 字形（M046）、Grok hook ParserError（M047）、Grok 状态栏 123（M048）。doctor 只在 M044 / M045 当场补了形态；M047 / M048 先修写入面，检查面滞后，用户说「继续 正常了」才立项 D08 补漏检。
- 同型坑：写了配置不等于能跑。Codex argv、Grok 壳行、hook args 都是「marker 在、运行时废」。doctor 要按各家 runner 的 spawn 规则分态，不能只搜 `oma-statusline` 或 `is_ours`。
- D09 同日裁定不管种子：检查面补形态不等于把 ohmycloud 镜像种子拉进本仓 TODO。

## 钩子

- 命令面：`oma doctor`（Grok statusline 三态、hooks.form 增 args）。
- 关联：M047、M048；写入面 `src\statusline.rs` Windows `.cmd` 单路径；D09 边界。
- 遗留：Kimi 状态栏 300ms 超时可能导致 command 回退内置布局，doctor 仍报 configured（未做超时探活）。
