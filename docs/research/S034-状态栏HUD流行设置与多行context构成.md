# S034：状态栏 HUD 流行设置与多行 context 构成

> D36（用户 2026-09-13 连续四令）研究先行件：1) 不显示 `claude:working` 形 agent 态段；2) 显示当前 MCP 与 tools 数量加对话 context 构成比例；3) 状态栏支持多行；4) 盘点当下最流行的 Claude Code / Codex 状态栏 HUD 设置。结论供追问链澄清设计用。

## 一、流行 HUD 工具盘点（2026-09 检索）

| 工具 | 形态 | 流行点 | 对 D36 的启示 |
| --- | --- | --- | --- |
| [ccstatusline](https://www.npmjs.com/package/ccstatusline)（npm，2.2.x 滚动） | 交互式 TUI 配置出 statusline 脚本 | 「类目领导者：最易读的通用形态」（[对比评测](https://yigitkonur.com/research/claude-code-statuslines-compared)） | 段落可配置加预设主题是主流预期；hst 已有 segments 三层键同向 |
| [CCometixLine](https://github.com/Haleclipse/CCometixLine)（Rust 单二进制） | 18 加原子段拼装、TUI 配置 | 高性能（Rust）、内置 usage 轨迹 | 「context token 红绿灯」分级提示是高频卖点；多行布局（双行）受欢迎 |
| claude-powerline | vim powerline 风格分段 | 轻量、无遥测 | powerline 分隔符美学（箭头形分段）是审美主流之一 |
| claude-statusline（TOML 主题） | TOML 配置加内置主题 | cost 追踪加 context 百分比 | `context 百分比` 是标配段；TOML 定制与 hst statusline.toml 同型 |

共同标配段：模型名、context 用量（百分比或红绿灯）、git 分支加脏态、session 时长、cost（API 计费场景）。共同少见：MCP / tools 计数（无现成工具做，因官方 payload 不给）。

## 二、Claude Code statusline 输入契约（官方口径）

- 输入：每 tick 向脚本 stdin 喂 JSON 会话数据（[官方文档](https://code.claude.com/docs/en/statusline)）[实证: 2026-09-13 官方页]。
- 字段：`model`（display_name 与 id）、`workspace`（current_dir 等）、`session_id`、cost / usage 族、`context_window` 族（used / total / remaining / 百分比；2.1.6 起给实际窗口值而非会话累计，[社区讨论](https://www.reddit.com/r/ClaudeAI/comments/1qbmrc7/claude_status_line_can_now_show_actual_context/) 与 [issue #13783](https://github.com/anthropics/claude-code/issues/13783) 记过新旧口径差）[实证: 2026-09-13]。
- **多行**：官方支持脚本输出多行（status bar 渲染多行内容）[实证: 2026-09-13 官方页]；D36 第 3 令可行。
- **MCP 与 tools 计数不在 payload 里**：要显示需自取数据源。候选：项目 `.mcp.json` 的 `mcpServers` 键数（hst doctor 已读同源）[实证: doctor.rs project_mcp_configured]；tools 数无稳定外部源（`claude mcp list` 是子进程调用，statusline 预算内不宜）[推断]。
- context 构成比例（system / tools / messages / free 切分）：payload 只给 used / total 两级，**细分构成拿不到**；近似只能 `used / total` 一维百分比或进度条 [推断: 官方字段集推导]。

## 三、四家写入面约束（S025 矩阵回照）

- codex：`[tui] status_line` 只吃内置项 ID 数组，无外部命令面（M045）：多行、MCP 计数、context 比例在 codex 侧做不了，保持内置项形态。
- kimi：`tui.toml` `[status_line].command` 单行输出加 300ms 预算；多行未验证 [假设: 待澄清轮实证]。
- grok：`[ui.status_line] type=command`，Windows 侧 `.cmd` 单路径直 spawn（M048）；多行未验证 [假设: 待澄清轮实证]。
- claude：多行官方支持，是 D36 第 3 令的主战场。

## 四、agent 态段取舍（D36 第 1 令）

现状：`oma` 段吃 `~/.hst/state/<agent>-<session>.json` 渲染 `agent:state`（S025 机读标记）。用户令「不显示 `claude:working` 形免得影响 herdr 的 hook」的理解候选：a) 状态栏该段与 herdr 自身状态显示重复且挤占宽度；b) 担心状态栏频繁触发 hook 读写干扰。落设计前需澄清：是「segments 默认序列去掉 oma 段」还是「完全移除该段能力」[待澄清]。机读标记消费面（`agent:state` grep）若仍有人吃，删段要留配置开关。

## 五、设计候选与缺口

> 供追问链澄清用。

1. 多行布局：claude 双行（上行模型加 git 加 context 百分比、下行 MCP 加 tools 加 agent 态）与单行可配；缺 kimi / grok 多行实证。
2. context 构成比例：官方只给一维；「构成比例」若指细分（tools 占多少）则超出 payload 能力，需降级口径或读 transcript（重、慢，不宜 statusline）[推断]。
3. MCP / tools 计数：MCP 数可从 `.mcp.json` 键数稳定取；tools 数无稳定源，候选拿 session JSONL 里 tool_use 名集合（重）或省略 [推断]。
4. 段开关沿用 `~/.hst/statusline.toml` segments 三层键扩段 id（`mcp` / `context_pct` / `multiline`），不引入新配置面 [直觉: 与 D18 架构同向]。

## 六态

本篇检索与官方页核对 2026-09-13 落；流行度判断来自检索面（npm 版本滚动、GitHub 仓、对比评测、社区讨论），未做下载量拉表 [记忆: 待复核]。四家多行与 payload 细分字段的实证留追问链后补。
