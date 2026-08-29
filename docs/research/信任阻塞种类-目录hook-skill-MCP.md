# 信任阻塞种类-目录hook-skill-MCP

> 2026-08-29。用户指出 trust block 分很多种（skill、项目目录、hook 等），编排要能**设置、绕过、检测**。源码锚点：Claude 旧版 [codeaashu/claude-code](https://github.com/codeaashu/claude-code)（2026-03-31 npm map 泄漏）；Codex `openai/codex`；Grok `xai-org/grok-build`；Kimi `MoonshotAI/kimi-code`。新 Claude 只对照官方文档。本机 `claude 2.1.246` / `codex 0.149.1` / `grok 1.0.13` / `kimi 0.38.0`。

## 需求

- 研究：各家会弹出或静默跳过的信任门有哪些种类；每种的落盘键、flag 绕过、只读检测点；对本仓 `init --pretrust` / `doctor` 应拆成哪些 check。
- 核查：folder 与 hook 是否同一对话框；skill 是否另有门；MCP/plugin 是否独立；Grok `--trust` 是否覆盖 hooks+MCP；Codex `trust_level` 与 `trusted_hash` 是否正交；Kimi 除 workspace-trust 外还有没有 hook/skill 门。
- 意图：混合。

## 结论

### 总表：编排要处理的门

工具审批（yolo）不是 trust。Trust 是「这段仓库代码能不能在你机器上跑」。种类如下。[推断: 泄漏源 codeaashu/claude-code 2026-03-31 + 各家现役配置键；新 Claude 只对文档]

| 种类 | Claude | Codex | Grok | Kimi |
| --- | --- | --- | --- | --- |
| 项目目录 / folder | 独立对话框 `TrustDialog`；键 `projects.<abs>.hasTrustDialogAccepted`；父目录信任覆盖子目录；家目录只记会话、不落盘；**`-p` 关掉信任校验** | `[projects."<abs>"] trust_level=trusted`；未信任则整层项目 `.codex/` 不加载 | 统一 store `~/.grok/trusted_folders.toml`；无仓库敏感配置则**不提示**；无头有配置则 **Untrusted（静默跳过项目代码）** | `~/.kimi-code/workspace-trust/wd_*`；默认 Don't trust |
| hook | 泄漏版（2026-03-31）：hook **出现在同一 folder 对话框**（扫描 `.claude/settings*.json` 的 hooks），接受时只写 `hasTrustDialogAccepted`。evo-harness 另写 `hasTrustDialogHooksAccepted`（泄漏 `ProjectConfig` **没有**此字段）。新文档未再命名第二框 | **独立**：`HookTrustStatus` = `untrusted \| trusted \| modified`；`[hooks.state.<源>] trusted_hash`；启动 interstitial；`--dangerously-bypass-hook-trust`；`--yolo` **不**绕过 | **并入 folder**：`/hooks-trust` 与 `--trust` 写同一 store，一次授权 **MCP+LSP+hooks**（及 plugins/agents/…） | 无独立 hook-trust 框；hook 示例在用户 `config.toml` |
| skill | **无独立 persist 键**。项目/plugin skill 若带 `allowedTools` Bash，只作为 folder 对话框的 `hasSkillsBash` 信号 | 跟项目层走：未信任则项目 skill 目录不加载 | **不在** folder-trust 扫描种类里（种类有 mcp/plugins/permission/lsp/envrc/claude/hooks/agents/roles/personas/workflows，**没有 skills**）。skill 按路径发现 | 项目 `.kimi-code/skills` / `.agents/skills` 无额外 trust 框 |
| MCP | **独立** `MCPServerApprovalDialog`；`enableAllProjectMcpServers` / `enabledMcpjsonServers`（迁到 local settings） | 项目 MCP 依赖项目信任层 | 并入 folder（`.mcp.json`、`[mcp_servers]`、`.cursor/mcp.json`、`~/.claude.json` projects.mcpServers） | 未在本轮源码中核到独立 MCP trust 框 |
| plugin | 安装警告 `PluginTrustWarning`（文案，不是 folder 那种 persist） | plugin hook 走 hook trust | 项目 `.grok/plugins` 与 `[plugins].paths` **会触发** folder 门；`grok plugin install --trust` 是另一条 | — |
| YOLO 确认框 | `BypassPermissionsModeDialog` → `skipDangerousModePermissionPrompt`（用户 settings） | 沙箱/审批是 yolo，不是 trust | always-approve 是权限，不是 folder | 启动 `StartPermissionPrompt`：auto/yolo/manual，**不是**目录信任 |
| 其它 | `hasCompletedOnboarding`；`hasClaudeMdExternalIncludesApproved` | — | 家/`$HOME` 等 over-broad 根 **拒绝落盘、直接当 trusted**；本地未 stamp 的 grok **整套 folder-trust inert** | — |

### Claude（泄漏源 + 新文档）

泄漏仓库是 2026-03-31 npm `.map` 拉出的 `src/`，[codeaashu/claude-code](https://github.com/codeaashu/claude-code)。新 CLI 2.1.246 只能对文档。

**设置**

- Folder：写 `~/.claude.json` `projects.<abs>.hasTrustDialogAccepted=true`（接受对话框时 `saveCurrentProjectConfig` 只设这一键）。
- YOLO 确认：用户 `settings.json` 顶层 `skipDangerousModePermissionPrompt=true`（从旧全局 `bypassPermissionsModeAccepted` 迁过来）。
- MCP：local settings `enableAllProjectMcpServers` 或把服务器名写入 `enabledMcpjsonServers`。
- 泄漏版 **没有** `hasTrustDialogHooksAccepted` 类型字段。evo-harness 仍写它——可能是更早/并行口径。编排继续双写无害；doctor 应**分开报**：有项目 hooks 时若只有 folder 键、没有 hooks 键，标 `trust.hooks` 为未知或沿用 folder（泄漏语义下同一框）。

**绕过**

- Folder：官方 security：**`-p` 禁用 trust verification**。家目录无法持久化，只能当次会话。
- YOLO 框：skip 键或非交互 `-p`（该框是 bypass 模式专用）。
- MCP：预写 enable-all；不能靠 `--dangerously-skip-permissions`。
- `--allow-dangerously-skip-permissions` **不管** 目录/hook/MCP 框。

**检测**

- `trust.project`：`hasTrustDialogAccepted`（含父路径）。
- `trust.hooks`：项目 settings 是否有 hooks；键 `hasTrustDialogHooksAccepted` 若存在则单独看。
- `trust.mcp`：项目 MCP 是否 pending（无 enable-all / 未在 enabled 列表）。
- `trust.skill`：有 `.claude/skills` 或 `.claude/commands` 且 folder 未接受 → 交互会堵（plugin 不加载；TrustDialog 会扫 skills-bash）。无独立 persist 键，绕过靠 `hasTrustDialogAccepted`。
- `skip_prompt`：已有。
- `onboarding`：`hasCompletedOnboarding`，不要和 folder 绑死。

### Grok（`xai-org/grok-build` `folder_trust.rs`）

一条 store，多种**触发物**。`--trust` / `/hooks-trust` 是同一 grant。

**设置**：`trusted_folders.toml` `folders.<abs>.trusted=true`。

**绕过**：`--trust`；`GROK_FOLDER_TRUST=0` 或 `[folder_trust] enabled=false`（整门关掉，项目 hook/MCP 不再 gated）；无头默认不提示、直接 Untrusted。

**检测**：store 是否 trusted；再按磁盘有无 `.grok/hooks`、`.mcp.json`、`.grok/plugins` 等标种类。空仓无敏感配置 → 不会弹，doctor 的 `trust.project` 仍可报未登记（但运行时不会挡）。

Skill **不是**这扇门的触发物。不要把「没 skill-trust 键」当成 skill 会弹框。

### Codex（`openai/codex` `hook_config.rs` + `HookTrustStatus`）

两扇门：

1. **项目**：`trust_level=trusted` 才加载项目 `.codex/`。
2. **hook**：每条源 `trusted_hash`；状态 `untrusted/trusted/modified`；命令变更会变 `modified` 再弹。bypass flag：`--dangerously-bypass-hook-trust`。官方/issue：`--yolo` 不绕过 hook trust。

Skill 走项目层，没有第三扇 skill 对话框（本轮未在源码树看到独立 SkillTrust 类型）。

### Kimi（`MoonshotAI/kimi-code`）

- 目录：`workspace-trust` 文件（evo-harness 实证形状仍准）。
- 启动权限框 `StartPermissionPrompt`：`auto | yolo | manual | cancel` — 这是 **permission mode**，应归 yolo，不要标成 `trust.skill`。
- 源码树有大量 `permissionGate` / `permissionMode`，没有与 Claude `TrustDialog` 对等的 folder+hooks 第二框。

### 对本仓：设置 / 绕过 / 检测

| 动作 | 现在 | 应改 |
| --- | --- | --- |
| 设置 | `apply_pretrust` 揉成一次写 folder（+ Claude 两个 hasTrust* + Codex 项目 trust + Kimi wd + Grok folders） | 保持一次预写，但 **按种类落键**：Claude folder 与 hooks 键分开写；Codex 另写 `hooks.state.trusted_hash`（有项目 hook 时）；MCP enable-all 有项目 `.mcp.json` 时才写 |
| 绕过 | 产品 spawn 尚未加 argv | 文档化：Claude `-p` 跳过 folder（我们 TUI **不能**靠这个）；Codex hook 用 hash 持久化而不是每次 `--dangerously-bypass-hook-trust`；Grok `--trust` ≡ 写 store |
| 检测 | doctor 每家一个 `trust` | 拆 `trust.project` / `trust.hooks` / `trust.mcp` / `trust.skill`（无独立门则 `ok` 并注明 covered_by / n/a） |

TUI pane 路径：folder 未绿就会在启动卡住（Claude/Kimi/Grok 交互）。Grok 无头是静默跳过项目 hook，看起来像「没 hook」而不是弹框——doctor 必须把「未信任 + 有 hooks」报成 `trust.hooks=block`。

## 事实源

| 类型 | 定位 | 日期 | 对应需求 | 提供什么 |
| --- | --- | --- | --- | --- |
| github | [codeaashu/claude-code](https://github.com/codeaashu/claude-code) `src/components/TrustDialog/TrustDialog.tsx` `utils.ts` | 泄漏 2026-03-31 | Claude folder/hooks/skill 是否同框 | 一框扫描 MCP/hooks/bash/skills-bash；接受只写 `hasTrustDialogAccepted` |
| github | 同仓库 `src/utils/config.ts` `ProjectConfig` | 同上 | persist 键 | `hasTrustDialogAccepted`、父路径覆盖、家目录 session-only；**无** `hasTrustDialogHooksAccepted` |
| github | 同仓库 `BypassPermissionsModeDialog.tsx` `migrateBypassPermissionsAcceptedToSettings.ts` | 同上 | YOLO 确认 | 写入用户 `skipDangerousModePermissionPrompt` |
| github | 同仓库 `mcpServerApproval.tsx` `migrateEnableAllProjectMcpServersToSettings.ts` | 同上 | MCP 独立门 | pending 项目 MCP 另开对话框 |
| web | [code.claude.com/docs/en/security](https://code.claude.com/docs/en/security) | 拉正文当日 | 新版绕过 | 首次 codebase 与新 MCP 要 trust；`-p` 关闭 trust verification；家目录不持久化 |
| github | `xai-org/grok-build` `crates/codegen/xai-grok-workspace/src/folder_trust.rs` | 拉正文当日 | Grok 统一门 | 优先级、无头 Untrusted、触发种类不含 skills |
| 本地 | `C:\Users\ray\.grok\docs\user-guide\10-hooks.md` | grok 1.0.13 随包装 | Grok hook=folder | `--trust` / `/hooks-trust` 同一 store，覆盖 MCP/LSP/hooks |
| github | `openai/codex` `codex-rs/config/src/hook_config.rs` `HookTrustStatus.ts` | 拉正文当日 | Codex hook 独立 | `trusted_hash`；状态 untrusted/trusted/modified |
| github | `MoonshotAI/kimi-code` `start-permission-prompt.ts` | 拉正文当日 | Kimi 启动框 | auto/yolo/manual，非目录信任 |
| 本地 | `D:\sourcecode\evo-harness\src\evo_harness\pretrust.py` | 2026-08-24 实证 | 落盘形状 | Claude 双 hasTrust*；Codex projects；Kimi wd_* |

## 缺口

- GitHub **code search 403 限流**；Claude 泄漏靠 git tree + raw 文件，未全库 grep `hasTrustDialogHooksAccepted`（类型定义里没有）。
- 未读 Claude 2.1.246 运行时二进制；新文档只确认 folder + MCP + `-p` 跳过，未点名 hooks 第二键。
- Codex 项目 `trust_level` 加载器本轮只核 hook_config / HookTrustStatus，未再读 `config/loader` 全文。
- Kimi `workspace-trust` 写入路径本轮树检索未直接打到文件名（仍以 evo-harness 实证 + 本机目录为准）。
- X 无贡献（未采讨论）。
- 未在本机对四家空仓/有 hook 仓各弹一次框做时序录屏。
