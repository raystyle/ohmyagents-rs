# P0027-四环境部署自适应-hook形态与状态栏

> 2026-09-02 当日闭环。缘起：Windows 侧会话每 prompt 弹 hook ENOENT、状态栏 `??`、codex 信任屏复燃（用户报修三连），用户定调根因——oma 缺四系统环境（Windows / WSL 共享目录 / Linux / macOS）自适应部署管理。方案矩阵见 `docs\research\S024-四环境部署自适应矩阵.md`（为什么）与 S025（状态栏四家矩阵与机读标记）；本文记方案与过程。

## 方案

### 1. hook 注册跨环境

> src\deploy.rs。

- **PATH 探针加形态阶梯**：`find_on_path("oma")`（pathutil.rs 包 `which` crate，Windows 自动 PATHEXT）命中 → claude/grok 注册 **bare** 形态（`command:"oma"` + `args` / `"oma hook --agent <名>"`），各消费 OS 用自己的 PATH 解析——一份注册全环境可用；未命中 → 绝对路径（单环境项目行为不变）。
- **粘性**：已有 bare 不降级（`settings_has_bare_oma` + `choose_form`），防一侧有 PATH 一侧没有时 see-saw 复辟；降级风险时 init 打 `init.hooks.warn=`。
- **codex 字段所有权**：`merge_codex_hook_event` 按 `OsSide` 只重写本侧字段（Unix `command` / Windows `commandWindows`），异侧字段逐字节保留、永不 stale-drop oma 条目。信任键天然按各 OS 的 config.toml 路径分族共存，每侧重播本侧哈希即自洽。
- 注册参数带 `--agent <名>`：用户手拉会话（无 env）也能回退写项目状态文件。
- 输出 `init.hooks.form=bare|absolute` 机读标记。

### 2. hook 状态通道全覆盖

> src\hook.rs。

- `oma hook --agent <名>`：env（oma 会话）优先；回退 = payload `cwd` 沿 `.git` 上溯到项目根写 `.ohmyagents/state/<agent>.json`。
- 状态记录新增 `session` 字段（payload session_id/sessionId）。

### 3. 状态栏

> src\statusline.rs。

- ps1 首行强制 `[Console]::OutputEncoding=UTF8`（CP936 吃 emoji 的根修，实测 `3f 3f` → `f0 9f a7 bd` 级修复）。
- 段序（用户多轮定调）：` shell | 完整目录 | 󰚩 agent:state | ✦ model | 󰍛 context | 󰅐 时长 |  branch [旗标] | 󰏗 包版本 | 󱘗 rust`；Catppuccin 系 256 色；图标按用户字体 cmap 实证（fontTools 按名搜），shell 段在最前。
- **oma 段机读标记**：`agent:state` 四态（`:` 单字节分隔，扫屏子串可匹配）；**会话闸**——状态记录 session 与 payload session_id 不符按 unknown（死会话遗留态不冒充）。
- shell 段：祖先链跳过 agent 本体找宿主 shell；Windows `Get-Process .Parent`、Linux `/proc`、macOS `ps -o ppid=` 兜底。
- 部署：claude/codex 命令串带 agent 名参数；pwsh 探测告警 `statusline.pwsh=missing`（advisory）。

## 过程

1. 取证：S015 矩阵 + claude 官方文档（exec form PATH 解析、无 per-OS 字段、statusline shell 路由）+ 本机盘上破坏面（`.codex/hooks.json` 双字段皆 `/mnt/d`）+ Node spawn 无扩展名补 `.exe` 实测。
2. Plan 子代理设计合并语义（`probe_hit`/`OsSide` 注入保测试宿主无关）；claude/grok `merge_hook_event` 零语义改动（stale 谓词天然要求含分隔符，测试钉住）。
3. 实机收敛：Windows 与 WSL 各 `cargo install --path` + `oma init`，双侧第二轮起三注册文件字节不变（see-saw 消除实证）；`oma init --pretrust` 清 Windows kimi 家目录信任残留，双侧 `doctor.blocked=false`。
4. 状态通道活体验证：本会话（用户手拉、无 env）PreToolUse 实时写 working，会话闸拦下旧会话遗留 idle。
5. 用户多轮定调收敛渲染：去成本段（网关计价不准）、oma 段 agent:state（先项目名后改）、完整目录路径、starship 风格旗标与版本段、Catppuccin 配色、pwsh 图标（md-terminal_powershell EBC7，cmap 实证）。
6. 全仓 rustfmt 漂移就地清零（用户定调「发现就要解决」），fmt 进门禁。

## 验收

- 测试 88+12 全绿零警告（新增 codex 字段保留、bare 粘性、foreign 自愈、hook 回退写、字节幂等等 8 例）；`cargo fmt --check` 干净。
- Windows/WSL 双环境 init 字节稳定、form=bare、doctor 全绿。
- 状态栏渲染与状态实时性本机目验；旧会话遗留态被会话闸拦截目验。

## 已知限制与后续

- 另外项目首次接入需在该项目跑一次 `oma init`（注册升级到 `--agent` 形态，claude 会话热加载）。
- `oma agents statusline` 覆盖 kimi（tui.toml）与 grok（config.toml [ui.status_line]）——S025 矩阵已一手，归下一目标。
- rmux 编排扫屏消费 `agent:state` 标记（status/doctor 交叉核对）——S025 待办。
- 远程验收通道（ssh ray@lan-win / ray@lan-mac）待跑（用户 2026-09-02 提供）。
