# Claude官方文档-阻塞门与编排对齐

> 2026-08-29。用户给出权威源 [https://code.claude.com/docs/](https://code.claude.com/docs/)。旧泄漏仓库 [codeaashu/claude-code](https://github.com/codeaashu/claude-code) 只当历史；**新 Claude Code 以本页文档为准**。对照本仓 `oma init --yolo [--pretrust]` / `oma doctor` 的设置、绕过、检测。

## 需求

- 研究：官方文档里，编排会碰到的阻塞门有哪些（目录 / hooks / skills / MCP / plugin / YOLO 确认 / 无头 `-p` / onboarding）；每种的落盘位置、会话类型差异、版本分界；对本仓要怎么设、绕、检。
- 核查：泄漏源与上一篇 `信任阻塞种类-目录hook-skill-MCP.md` 里可真可假的主张（`hasTrustDialogAccepted`、独立 hooks 键、`skipDangerousModePermissionPrompt` 范围、committed MCP 审批、skill 是否单独成门、`-p` 是否等于 YOLO）。
- 意图：混合。官方文档是权威；泄漏与 GitHub issue 只作对照和缺口。

## 结论

### 研究：官方把门拆开了，不是「一个信任框包打天下」

官方 [permissions.md](https://code.claude.com/docs/en/permissions.md) 的表 **What runs before you trust a folder** 把仓库内容按种类列行，按会话类型列列。同一仓库里，交互会话和 `claude -p` / SDK **不是同一套门**。[实证: 2026-08-29 对照 code.claude.com/docs permissions / hooks / permission-modes]

| 仓库内容 | 只信任了父目录（交互） | `claude -p` / SDK、本夹从未信任 |
| --- | --- | --- |
| settings 里的 hooks、`env`、`apiKeyHelper`；项目 skill 的 hooks 和 `allowed-tools` | 用 | 用。**workspace trust 从不挡 skill 的 `allowed-tools`** |
| `.claude/settings.json` 的 `permissions.allow` 和 `additionalDirectories` | 不生效，会再弹信任框列出它们 | 不生效。stderr 打 `this workspace has not been trusted` |
| 项目 subagent 的 frontmatter hooks、项目 `@skills-dir` plugin、`extraKnownMarketplaces` | 不用，也不弹框 | 不用 |
| subagent frontmatter 里的 inline `mcpServers`（v2.1.238+） | 不用，也不弹框 | 不用 |
| `.mcp.json` 服务器（含仓库自己写的审批） | **会问你**；仓库自己的审批不算 | **不问就连**。SDK 还要 `settingSources` 含 project。同夹 `claude mcp list` 仍显示 pending |
| `.mcp.json` 上的 `headersHelper`（v2.1.238+） | 等本夹信任框；此前只用静态 `headers` | **不跑**。stderr `headersHelper not run` |

手写信任的官方路径：**`~/.claude.json` 的 `projects["<path>"].hasTrustDialogAccepted = true`**。`<path>` 是 git 仓库根；仓库外是启动目录。家目录接受的信任**当次会话有效、不落盘**。

[hooks.md](https://code.claude.com/docs/en/hooks.md) 补了一刀交互 vs `-p`：

- **交互**：所有 settings 文件里的 hooks（**含你自己的 `~/.claude/settings.json`**）都先卡住，直到本夹或可覆盖的父目录信任框被接受。
- **`-p` / SDK**：不弹框，把夹子当 trusted，仓库 `.claude/settings.json` 里的 hooks **会跑**。

所以 TUI pane（交互启动）和将来脚本 `-p` 必须分路诊断。不能把「`-p` 不弹信任框」理解成「交互也不弹」。

### 研究：YOLO 是 `bypassPermissions`，不是 `-p`，也不是 auto

[permission-modes.md](https://code.claude.com/docs/en/permission-modes.md)：

- 模式：`default`（UI 叫 Manual）、`acceptEdits`、`plan`、`auto`、`dontAsk`、`bypassPermissions`。
- Pro / Max / Team 交互的内置起点从 v2.1.228（Windows native v2.1.233）起是 **`auto`**。`claude -p` 和 SDK 的内置起点仍是 **`default`（Manual）**。
- **`auto` 不能从项目或 local settings 生效**，只能写用户或 managed。仓库不能把自己授予 auto。
- 本仓说的 YOLO = `bypassPermissions` / `--dangerously-skip-permissions`。官方只建议容器 / VM。deny 规则在此模式仍生效。ask 规则、`requiresUserInteraction` MCP、组织标成 ask 的 connector、关键路径 `rm`/`rmdir`、跨会话消息防护 **任何模式都不自动批**。
- 交互第一次进 bypass：责任确认框；接受后写 **用户 settings** 的 `skipDangerousModePermissionPrompt`，只问一次。拒绝则退出。`-p` **不弹此框**。`--bg` 在你没在交互里接受过之前 **直接拒绝启动**。
- Linux / macOS：root / sudo 拒绝 bypass（识别到的 sandbox 除外）。v2.1.248+ `--restricted` 也拒绝。
- `--allow-dangerously-skip-permissions` 只把 bypass **加入 Shift+Tab 循环，不激活**。
- Web / cloud **忽略** 仓库里的 `defaultMode: "bypassPermissions"` / `"dontAsk"`。

[settings-reference.md](https://code.claude.com/docs/en/settings-reference.md) 对 `skipDangerousModePermissionPrompt`：

- **Scope：User, local, or managed。项目 `.claude/settings.json` 里写了不算**，避免未信任仓库替你跳过确认框。
- 类型 boolean；接受对话框时 Claude 把它写成用户 settings 的 `true`。

本仓 `apply_project_yolo` 写到 `.claude/settings.local.json` 符合官方 scope。不要写进共享 `settings.json`。

### 研究：MCP 审批和目录信任正交，v2.1.196+ 仓库不能自己批自己

[mcp.md](https://code.claude.com/docs/en/mcp.md) **Project server approvals and workspace trust**：

- v2.1.196+：`enableAllProjectMcpServers` / `enabledMcpjsonServers` **提交进项目 `.claude/settings.json` 的，在未信任夹子里被忽略**。`claude mcp list` 停在 `⏸ Pending approval`。
- 未信任夹子里仍然生效的审批来源：用户 `~/.claude/settings.json`、managed、`--settings`。
- 未跟踪的 `.claude/settings.local.json`：要等信任框（配置家目录例外）。v2.1.207 之前会在从未信任的夹子里也套用 local 审批。
- `disabledMcpjsonServers` 任何文件都能拒绝，优先于 enable-all。
- 交互会弹 per-server 审批。**`-p` / SDK / cloud 不问就加载项目服务器**。`bypassPermissions` 且设了 `skipDangerousModePermissionPrompt` 也跳过 MCP 审批框。
- 不要进项目 MCP：`disabledMcpjsonServers`、`--setting-sources` 去掉 project、`--strict-mcp-config`。
- `claude mcp reset-project-choices` 清审批选择。

官方手写信任也适用于 `headersHelper`：同一键 `hasTrustDialogAccepted`。v2.1.238 之前 `-p` 会在未信任夹子里跑 helper。

### 研究：skill 默认不是信任门；做成 plugin 才是

[skills.md](https://code.claude.com/docs/en/skills.md)：

- 普通项目 skill：`.claude/skills/<name>/SKILL.md`。`allowed-tools` 走权限流，**官方写明 workspace trust 从不挡它**。
- 给 skill 目录加 `.claude-plugin/plugin.json` 会变成 plugin（`<name>@skills-dir`），**项目 `.claude/skills/` 里这种形态要先接受 workspace trust**。
- `extraKnownMarketplaces`、项目 `@skills-dir` plugin：未信任本夹则不加载，`-p` 也不加载。

doctor 把「有 `.claude/skills`」一律绑 `trust.project` 过严。只应对 **plugin 形态** 报 `trust.skill`。

### 研究：无头 `-p` 的安全面，不是 YOLO 面

[headless.md](https://code.claude.com/docs/en/headless.md)：

- `-p` 不弹 workspace trust，也不弹 per-server MCP 审批。
- 没有 `--bare` 时，`-p` 会跑项目 `.claude/settings.json` 的 hooks、连 `.mcp.json`，即使夹子从未信任。
- `--bare` 跳过 hooks / skills / commands / subagents / plugins / MCP / auto memory / CLAUDE.md。官方推荐脚本和 SDK，并写明 **将来会变成 `-p` 默认**。
- `-p` 起点权限模式是 Manual。要免审批必须另加 `--permission-mode` / `--allowedTools` / `--dangerously-skip-permissions`。
- `--bare` 仍读项目 settings 的 `env` 和部分 helper；`apiKeyHelper` 只从 `--settings` 读。

GitHub [#10409](https://github.com/anthropics/claude-code/issues/10409)（2.0.27，2025-10）：交互 `--dangerously-skip-permissions` **跳过信任检查但不会把夹子标成 trusted**，hooks 被 debug 打成 skipped。这和 `-p`「当 trusted」相反。编排不能用 YOLO flag 代替 `hasTrustDialogAccepted`。

### 研究：对本仓的设置 / 绕过 / 检测

**设置（交互 TUI pane）**

1. Folder：`~/.claude.json` `projects["<git-root>"].hasTrustDialogAccepted=true`。Windows 路径拼写会裂成多条（见缺口）。家目录不要指望落盘。
2. YOLO 模式：项目 `.claude/settings.json` `permissions.defaultMode: "bypassPermissions"`（交互读项目；VS Code 扩展**不读项目**作起点模式）。
3. YOLO 确认框：用户或 local `skipDangerousModePermissionPrompt: true`。不要写项目共享文件。
4. MCP：先写 folder 信任，再写 **local** `enableAllProjectMcpServers`；若要在未信任夹子里也批，写 **用户** settings。不要指望 committed 项目 settings 的 enable-all。
5. Hooks：没有第二把官方 persist 键。交互能否跑 hooks = folder 是否被接受（或 `-p`）。双写 `hasTrustDialogHooksAccepted` 官方未记载，无害但不构成检测依据。

**绕过（按会话类型）**

| 目的 | 交互 pane | 脚本 `-p` |
| --- | --- | --- |
| 不弹目录框 | 必须落盘 `hasTrustDialogAccepted` | 默认就不弹；**不等于** allow 规则生效 |
| 不弹 YOLO 确认 | skip 键或先交互接受一次 | 不弹；`--bg` 仍要先接受过 |
| 不弹工具审批 | `bypassPermissions` + skip 键 | `--permission-mode` / `--allowedTools`；`-p` 单独不够 |
| 不弹 MCP 审批 | folder 信任 + local/user enable-all；或 bypass+skip | 默认加载项目 MCP |
| 不跑别人仓库的 hooks/MCP | 不信任 / `disableAllHooks` | `--bare` 或 `--settings '{"disableAllHooks": true}'`（必须 `--settings`，用户文件会被项目 settings 盖掉） |

**检测（doctor）**

- `yolo`：项目 `permissions.defaultMode=bypassPermissions`。
- `skip_prompt`：用户或 local 的 `skipDangerousModePermissionPrompt`。项目共享文件里的值应视为无效。
- `trust.project`：`hasTrustDialogAccepted`。父路径覆盖子目录；嵌套 git 仓库单独弹。
- `trust.hooks`：交互路径下 **covered_by folder**。不要把泄漏键当硬条件。
- `trust.mcp`：有项目 MCP 时是**独立门**。交互会弹 `MCPServerApprovalDialog`；v2.1.196+ 还要 folder 信任后 committed 审批才算。用户 / managed / `--settings` 的 enable-all 在未信任夹子里仍生效。只写项目 `settings.json` 的 enable-all **不够**。
- `trust.skill`：有 `.claude/skills` 或 `.claude/commands` 时也是门。`allowed-tools` 本身不被 trust 挡，但 skills-dir plugin 不加载、泄漏 TrustDialog 会扫 skills-bash、交互首次仍要目录信任。doctor 在未写 `hasTrustDialogAccepted` 时报 block。`--yolo` 清不掉；`--pretrust` 才写 folder 键。
- `onboarding`：`hasCompletedOnboarding` 仍在 `~/.claude.json`；官方 settings 索引未列此键，当全局状态而非 settings.json。
- 交互 vs `-p`：doctor 当前按 **交互 pane** 口径。若将来加 `-p` 委派，hooks/MCP 的 block 语义要反过来（`-p` 会跑未信任仓库的 hooks 和 MCP）。

### 核查：上一篇主张

| 主张 | 结论 | 说明 |
| --- | --- | --- |
| `hasTrustDialogAccepted` 是 folder 落盘键 | **成立** | 官方 permissions.md / mcp.md 明文，2026-08 文档仍用此键 |
| 独立 `hasTrustDialogHooksAccepted` 是现役官方键 | **已过时 / 官方未记载** | 官方只讲 folder 键；交互 hooks 跟 folder 走 |
| TrustDialog 扫描 hooks/MCP/skill-bash 但只写 folder 键 | **说法不一** | 泄漏行为；新文档改成「按内容种类、按会话类型」分表，不再描述同一框的扫描字段 |
| `skipDangerousModePermissionPrompt` 不能写项目共享 settings | **成立** | 官方 scope：User, local, or managed |
| 该键在 local 顶层而非嵌套 permissions | **成立** | 示例就是顶层 boolean |
| committed `enableAllProjectMcpServers` 在未信任夹子生效 | **不成立**（v2.1.196+） | 仓库不能自己批自己的 `.mcp.json` |
| 项目 `.claude/skills` 一律要 folder 信任 | **不成立** | 普通 skill 的 `allowed-tools` 不被 trust 挡；plugin 形态才要 |
| `-p` 关掉 trust verification | **成立，但要加限定** | 不弹框；hooks 当 trusted 跑；allow 规则仍不生效；MCP 不问就连；headersHelper 反而不跑 |
| `-p` 等于 YOLO | **不成立** | `-p` 起点 Manual；YOLO 是 bypass |
| `--allow-dangerously-skip-permissions` 管目录/MCP | **不成立** | 只加循环，不激活 bypass，不管 trust |
| `--dangerously-skip-permissions` 会把夹子标 trusted | **不成立** | issue 10409；官方要手写 `hasTrustDialogAccepted` |
| deny 在 bypass 仍生效 | **成立** | 官方 permission-modes |
| onboarding 与 folder 不是同一键 | **成立** | `hasCompletedOnboarding` 在 `~/.claude.json` 顶层 |

## 事实源

| 类型 | 定位 | 日期 | 对应需求 | 提供了什么 |
| --- | --- | --- | --- | --- |
| web | [https://code.claude.com/docs/llms.txt](https://code.claude.com/docs/llms.txt) | 拉正文当日 | 研究：文档地图 | 官方索引；权限 / MCP / skills / hooks / headless / settings-reference 入口 |
| web | [https://code.claude.com/docs/en/permissions.md](https://code.claude.com/docs/en/permissions.md) | 2026-08-29 索引页标注 | 研究：folder 门；核查 persist 键 | 「What runs before you trust a folder」全表；`hasTrustDialogAccepted` 手写路径；allow 规则等信任；local settings 何时要信任 |
| web | [https://code.claude.com/docs/en/security.md](https://code.claude.com/docs/en/security.md) | 同上 | 研究：trust verification；核查 `-p` | 首次跑仓库和 new MCP 要信任；**`-p` 关掉 trust verification**；家目录不落盘 |
| web | [https://code.claude.com/docs/en/permission-modes.md](https://code.claude.com/docs/en/permission-modes.md) | 页内 2026-08-28 | 研究：YOLO / auto / `-p` 起点 | bypass 确认框写用户 settings；root/sudo；`--restricted`；auto 默认；`-p` 为 Manual |
| web | [https://code.claude.com/docs/en/settings-reference.md](https://code.claude.com/docs/en/settings-reference.md) | 拉正文当日 | 核查 skip 键 scope；MCP 键 | `skipDangerousModePermissionPrompt` = User/local/managed；`enableAllProjectMcpServers` Any file 但未信任时忽略项目共享文件 |
| web | [https://code.claude.com/docs/en/mcp.md](https://code.claude.com/docs/en/mcp.md) | 同上 | 研究：MCP 门；核查 v2.1.196+ | 仓库不能自己批自己；`-p` 不问就加载；bypass+skip 也跳过 MCP 框；headersHelper 信任 |
| web | [https://code.claude.com/docs/en/skills.md](https://code.claude.com/docs/en/skills.md) | 同上 | 研究：skill 门 | 普通 skill 不被 trust 挡；skill-as-plugin 要 workspace trust |
| web | [https://code.claude.com/docs/en/hooks-guide.md](https://code.claude.com/docs/en/hooks-guide.md) / [hooks.md](https://code.claude.com/docs/en/hooks.md) | 同上 | 研究：hooks 门 | `/hooks` 只读；交互卡住全部 settings hooks；`-p` 当 trusted |
| web | [https://code.claude.com/docs/en/headless.md](https://code.claude.com/docs/en/headless.md) | 同上 | 研究：`-p` vs YOLO | `--bare`；`-p` 加载 hooks/MCP；起点 Manual |
| web | [https://code.claude.com/docs/en/settings.md](https://code.claude.com/docs/en/settings.md) | 同上 | 研究：scope / Windows local 文件 | 优先级；committed 键等信任；Windows 上 local 文件不跟仓库根走 |
| web | [https://code.claude.com/docs/en/claude-directory.md](https://code.claude.com/docs/en/claude-directory.md) | 同上 | 研究：`~/.claude.json` | `projects` 记 trust-dialog acceptance；权限规则去 settings.local |
| github | [anthropics/claude-code#10409](https://github.com/anthropics/claude-code/issues/10409) | 2025-10-27，Claude 2.0.27 | 核查 YOLO flag 是否标 trusted | `--dangerously-skip-permissions` 跳过检查但不写信任；hooks 被 skip |
| github | [anthropics/claude-code#90220](https://github.com/anthropics/claude-code/issues/90220) | 2026-08-27，2.1.241 | 核查检测键是否可靠 | `hasTrustDialogAccepted: false` 仍可能跑 hooks；无稳定 programmatic 信号 |
| github | [anthropics/claude-code#88418](https://github.com/anthropics/claude-code/issues/88418) | 2026-08-22 | 缺口：路径拼写 | 同一目录最多三条 path 键，信任/MCP 状态分裂 |
| github | [anthropics/claude-code#84402](https://github.com/anthropics/claude-code/issues/84402) | 2026-08-17 | 研究：marketplace | 祖先 walk 授信，`extraKnownMarketplaces` 要精确匹配 |
| x | [@bukati](https://x.com/bukati/status/2035060459616022718) 回复 [@levelsio](https://x.com/levelsio/status/2035050535133290607) | 2026-03-20 | 核查 persist 键 | 社区当时已指向 `~/.claude.json` `projects.hasTrustDialogAccepted`（家目录反复弹框场景） |
| x | [@lydiahallie](https://x.com/lydiahallie/status/2021012074160324633) | 2026-02-10 | 研究：Desktop YOLO | Anthropic 职员：Desktop 支持 `--dangerously-skip-permissions` |
| x | [@4Ndr3w10000](https://x.com/4Ndr3w10000/status/2086747885924925775) | 2026-08-10 | 研究：`claude agents` | v2.1.225 给 `claude agents` 补了与主 CLI 相同的 workspace trust |

## 缺口

- **`hasTrustDialogAccepted` 不是可靠运行时信号**（issue 90220，2.1.241）。doctor 只能报「键在不在」，不能保证 hooks 实际会跑。官方没有 `claude trust status`。
- **路径拼写**（issue 88418）：Windows / 大小写 / 斜杠会导致信任写到另一条 `projects` 键。本仓 `keys_match` 已做 native/forward 对照，未覆盖「最多三条拼写并存」。
- **onboarding**：官方 settings 索引未列 `hasCompletedOnboarding`；键仍出现在 `~/.claude.json`。未能从 settings-reference 正文核对写入时机。
- **泄漏键 `hasTrustDialogHooksAccepted`**：官方 2026-08 文档零命中。继续双写无官方依据。
- **VS Code 扩展**：不读项目 settings 作起点权限模式；有 issue 称扩展里信任框不弹、项目插件被静默跳过（#67319）。本仓 TUI pane 走 CLI，不覆盖扩展。
- **本轮未再复跑** 本机 `claude 2.1.246` 实证（只拉官方文档 + issue）。与泄漏行为冲突处以文档为准。
- **X**：无 2026-08 官方账号发文解释 v2.1.196 MCP 信任变更；社区帖作讨论不当既成事实。
- Codex / Grok / Kimi 不在本篇范围；仍以 `信任阻塞种类-目录hook-skill-MCP.md` 的源码核实为准。
