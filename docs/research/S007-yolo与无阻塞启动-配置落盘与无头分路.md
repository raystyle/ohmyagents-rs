# S007-yolo与无阻塞启动-配置落盘与无头分路

> 2026-08-31。由《yolo与无阻塞启动-参数或配置》（2026-08-29，TUI pane 配置落盘）与《YouMind无头模式与YOLO免审批对照》（2026-08-29，三家无头分路）合并，并回写《现有研究复核与扩展》的 Grok 订正与委派前检查清单。两条线一件事的两面：**TUI pane 靠配置落盘不弹框；将来无头任务按家分配方**。

## 背景

编排器要把 agent 指到任务上：启动不能卡在信任框、权限框、登录框；诊断不能堵主命令。ohmypwsh 0017 定调「权限和信任写进配置，不靠每次 flags」。无头（`-p`）是另一条路：一次性进程、无 TUI，与 YOLO 正交（Claude/Grok）或无头即 auto（Kimi）——两者不可混。

## 关键结论

### TUI pane：配置落盘

1. **启动无阻塞靠配置，不靠现场按键。** `init` 把 yolo 与项目信任写进该项目能加载的文件；spawn 时 TUI 不应再弹框。evo-harness 的 `DIALOGS` 按键只当兜底。[经验: ohmypwsh 0017 + evo-harness pretrust]
2. **诊断无阻塞、不 attach。** `doctor`/`status` 只读磁盘：yolo 键、信任库、`.ohmyagents\state\`、任务映射。不 `send-keys`、不等 `wait_ready`。[实证: 2026-08-29 poc-yolo-doctor]
3. **任务指向与启动分离。** spawn 立即返回；agent 与 task-id 写项目内 JSON；主 CLI 委派只查「该 agent 对应 task 且非 blocked」。[假设: 产品 spawn 未落地]
4. **参数与配置并存。** 默认走配置；`--yolo` 等只覆盖单次会话，受控编排不把信任绕过长期留在 argv。[经验: ohmypwsh 0017 定调]

### 四家落盘表

> 2026-08-31 订正版

表内各键为各家官方文档与 poc-yolo-doctor 实测口径；Claude 列订正自 S006 核查表。[实证: 2026-08-29 poc-yolo-doctor + 官方 settings/config 文档]

| agent | 权限/沙箱（配置） | 项目信任（用户家，`--pretrust` 才写） | 订正注 |
| --- | --- | --- | --- |
| claude | 项目 `.claude/settings.json`：`permissions.defaultMode=bypassPermissions`；`skipDangerousModePermissionPrompt=true` 写 local **顶层**（scope 限 User/local/managed，不进共享项目文件、不进 permissions 嵌套） | `~/.claude.json` `projects.<abs>.hasTrustDialogAccepted`；`hasCompletedOnboarding` | ~~`hasTrustDialogHooksAccepted`~~ 不列为必要键：官方零记载，双写无害但不构成检测依据（见《信任阻塞门》） |
| codex | 项目 `.codex/config.toml`：`sandbox_mode=danger-full-access`、`approval_policy=never` | 用户 `~/.codex/config.toml` `[projects."<abs>"] trust_level=trusted`（项目级 `[projects]` 不能清信任框） | 有项目 hook 时另写 `[hooks.state] trusted_hash` 替代 `--dangerously-bypass-hook-trust` |
| kimi | 项目 `.kimi-code/config.toml`：`default_permission_mode=auto|yolo` | `~/.kimi-code/workspace-trust/wd_*` | 无 hook-trust 框 |
| grok | **仅用户** `~/.grok/config.toml`：`[ui] permission_mode=always-approve`（官方明文不能写项目 `.grok/config.toml`；旧键 `approval_mode`/`yolo` 仍工作但 `permission_mode` 优先） | `~/.grok/trusted_folders.toml` 替代 `--trust` | always-approve 下 deny 规则与 PreToolUse hook **仍生效**（secret-guard 类 hook 在 yolo 里仍有用）[经验: 2026-08-29 官方复核] |

### 无头分路：三家对照

> YouMind 核查

5. **无头 ≠ YOLO**：Claude `-p` 起点 Manual 逐项审批、无 UI 时该次调用被拒（流程可假绿）；Grok `-p` 默认 Ask、要全自动另加 `--always-approve`（alias `--yolo`）或 `dontAsk`+`--allow`；Kimi `-p` **就是 auto**、官方禁止与 `--yolo`/`--auto`/`--plan` 同用（`--yes`/`--auto-approve` 是隐藏别名）。从 Claude/Grok 脚本抄到 Kimi 最容易踩「再加一遍 --yolo」。[实证: 2026-08-29 官方 headless/permission-modes/kimi-command + 本机三家家 `--help`]
6. 本机 `grok 1.0.13 --help` 另有 Claude 同构 `--permission-mode`（default/acceptEdits/auto/dontAsk/bypassPermissions/plan），官方 Permissions 页未写此层；CI 可走 `dontAsk`+`--allow`。[实证: 本机 help]
7. 无头失败模式：校验产物/diff，不校验退出码（Claude 假绿、Grok 可能 stall、Kimi 限流静默挂起）。[经验: YouMind 对照文]
8. 对现役 TUI 路径：`oma doctor` 扫 yolo/trust/binary/state 仍对口；不要把 Kimi `-p` 即 auto 当交互默认。若加 `oma run --headless`：Claude `-p`+`--allowedTools` 或 `--permission-mode dontAsk|acceptEdits`；Grok `-p`+`--always-approve`（加 `--no-auto-update`）；Kimi 只 `-p` 禁叠 `--yolo`；Codex 不在 YouMind 范围，仍走 `.codex/config.toml`。[推断: 三家口径推导]

### doctor 与无阻塞判据

三问全文件只读：配好 yolo 了吗（配置键）；闲着还是卡住（state JSON）；在跟哪条任务（tasks JSON）。

spawn 后立刻允许委派，当且仅当：doctor 全绿；pane 进程活着（pid）；state 非 `blocked`（unknown 允许先发，Drive 走三段式兜底）。禁止主命令 `wait_ready` 数分钟；禁止对 Codex 发 `C-c` 清框。

`oma doctor` 退出码：缺二进制、缺 yolo 键、缺项目信任、blocked 过期则非 0。[实证: 2026-08-29 poc-yolo-doctor]

### 委派前检查清单

> 回写自复核篇

实现 `init`/`spawn`/`run` 时按序核：配置优先 flags 单次覆盖；Claude skip 键写 User 或 Local；Codex 先预写 `[projects]` 再写项目 hook；Grok 权限模式写用户 config、项目只放 hooks/skills/deny；Kimi hook 注册暂走用户 config、脚本放项目；Drive 用 `paste-buffer -p`；spawn 立即返回；不把 wait-pane Quiet 当 idle。[经验: 2026-08-29 复核定调]

## 踩坑沉淀

| 坑 | 正解 |
| --- | --- |
| WSL 脚本把 skip 键写进 `permissions` 对象 | 顶层布尔；scope User/local/managed |
| Grok permission_mode 写项目 `.grok/config.toml` | 官方明文仅用户 config 生效 |
| 把 `-p` 当 YOLO 用 | `-p` 起点Manual；免审批另加旗标 |
| Kimi `-p` 再叠 `--yolo` | 官方禁止该组合 |
| 信任绕过长期留在 argv | 配置落盘，flags 只单次 |
| Codex 项目级 `[projects]` 当信任 | 只认用户 `~/.codex/config.toml` |

## 待办

- 项目级 `.codex/config.toml` 的 `[hooks.state]` 能否替代用户级 trusted_hash。
- hook/skill 的 `poc-init` 另做。

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本地 | ohmypwsh 0017、`set-*-config.ps1`、`verify-five-ends.ps1`；evo-harness flags | 2026-08-25 | 配置持久化定调；落盘形状 |
| 本地 | `poc-yolo-doctor`（本仓） | 2026-08-29 | init/doctor POC 实证 |
| web | youmind.com/d/QO6XcZnIybiBU1（share API 取 plain） | 2026-08-29 | 三家无头对照原文 |
| web | code.claude.com headless / permission-modes；docs.x.ai headless-scripting / permissions / cli-reference / enterprise；MoonshotAI kimi-command.md | 2026-08-29 | 无头默认权限、别名、dontAsk 路线、Kimi 冲突规则 |
| 本机 | `claude --help` 2.1.246 / `grok --help` 1.0.13 / `kimi --help` 0.38.0 | 2026-08-29 | flag 面；grok 的 Claude 式 `--permission-mode` |
| github | MoonshotAI/kimi-cli#2072 OPEN | 2026-08-29 | yolo/非交互混同议题 |

## 缺口

- 未本机实跑三家 `-p` 写文件任务（假绿/stall 无时序证据）。
- Claude 受保护路径在 bypass 下是否仍提示（原文标「随版本反复」）未核 2.1.246。
- grok `--permission-mode` 与 `[ui] permission_mode` 取值对应未做 inspect 对照。
