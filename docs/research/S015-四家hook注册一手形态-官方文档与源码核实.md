# S015-四家hook注册一手形态-官方文档与源码核实

> 2026-08-31。用户定调研究法：四家 agent 除 Claude Code 外全有官方源码，用 gh 定位仓库浅克隆读源码；Claude Code 用官方文档站 llms.txt 定位参考页。本文全部一手（官方文档原文或官方仓源码），替代 S008 里二手转述的注册形态。oma init（P0005 init 部件）部署 hook 的直接依据。

## 需求

- 核实：四家 hook 的注册文件落点、JSON/TOML 形态、事件全集、stdin/exit 语义、信任门——为 `poc-init` 与 `oma init` 提供权威 schema。
- 仓库定位（gh 检索，R005 双通道之 GitHub 法）：
  - Codex：`openai/codex`（120k stars，2026-08-31 仍有 push）[实证: gh repo view]
  - Grok：`xai-org/grok-build`——xAI 官方 "coding agent harness and TUI"（本仓四路里的 grok 即它）[实证: gh repo list xai-org]
  - Kimi：`MoonshotAI/kimi-code`（TS 本尊；`MoonshotAI/kimi-cli` 是 Python 版，用户点名不采用）[实证: gh search + 用户定调]
  - Claude Code：闭源；官方 `code.claude.com/docs/llms.txt` 定位到 `docs/en/hooks.md` 参考页 [实证: 2026-08-31 抓取]

## 关键结论

### 1. Claude（官方 hooks reference）

[实证: code.claude.com/docs/en/hooks.md，2026-08-31]

注册三层嵌套（`settings.json` 家族）：

```json
{"hooks": {"<Event>": [{"matcher": "*", "hooks": [
  {"type": "command", "command": "<exe>", "args": ["hook"], "timeout": 10}
]}]}}
```

- **exec form（command+args）最稳**：args 存在时 command 按 PATH 解析为可执行文件直接 spawn，无 shell、无引号问题；路径占位符 `${CLAUDE_PROJECT_DIR}` 明文替换进 command 与每个 args。Windows 下 command 必须是真 exe（`.cmd`/`.bat` shim 不行）——`oma.exe 绝对路径 + args:["hook"]` 是理想形态。shell form 才有 `shell: "powershell"`。
- 事件 30+（参考页全集）。oma 关心的：SessionStart、SessionEnd、UserPromptSubmit、PreToolUse、PostToolUse、Stop、Notification、SubagentStart/Stop、PreCompact、**PermissionRequest**。
- **重大订正 S009 旧口径**：Claude 现在有标准 `PermissionRequest` 事件（"about to ask you for permission"），且支持 hook 程序化裁决 `hookSpecificOutput.decision.behavior: allow|deny`（allow 还可带 updatedInput/updatedPermissions）。旧结论「Claude 无 PermissionRequest、Notification 顶替归 unknown」过时。
- Notification 有 matcher 细分（`permission_prompt`、`idle_prompt` 等）；`permission_prompt` 在提示等待约 6 秒后才发，即时信号用 PermissionRequest。
- stdin snake_case（hook_event_name/session_id/cwd/tool_name/tool_input/…）；exit 2 block；JSON 输出控制；超时默认 600s（UserPromptSubmit 30s、SessionEnd 共享 1.5s 预算）。
- 项目级 `.claude/settings.json` 要 workspace trust 先行（与 S006 一致）；`-p`/SDK 会话视目录为已信任。
- matcher 语义：纯 exact 字符集按精确串（`|`/`,` 分隔），含其它字符按非锚定 JS 正则。

### 2. Codex（openai/codex 源码）

[实证: codex-rs/config/src/hook_config.rs、codex-rs/hooks/src/schema.rs、hooks/src/engine/discovery.rs，浅克隆 2026-08-31]

- 注册**双源同构**：`hooks.json`（`HooksFile{description?, hooks}`）与 `config.toml` `[hooks]` 表（`HooksToml{events..., [hooks.state."<key>"]}`）。同层两者都非空时警告 prefer single representation。
- 形态与 Claude 同构（camelCase 事件键）：

```json
{"hooks": {"UserPromptSubmit": [{"matcher": null, "hooks": [
  {"type": "command", "command": "oma", "commandWindows": "oma.exe",
   "timeout": 10, "async": false, "statusMessage": "…"}
]}]}}
```

- handler 四型：command（含 **commandWindows** 平台专用命令字段——oma 部署可直接用）、mcp_tool、prompt、agent。
- 层序（discovery）：managed requirements → config layers low-to-high（用户 `~/.codex`、项目 `.codex`，每层先读层目录 hooks.json 再读 config.toml `[hooks]`）→ plugin 源。插件 env 注入 `PLUGIN_ROOT`/`CLAUDE_PLUGIN_ROOT`/`PLUGIN_DATA`/`CLAUDE_PLUGIN_DATA`（对 Claude 插件生态 OOTB 兼容）。
- 信任持久化：`[hooks.state."<source>:<event>[i].hooks[j]"] {enabled, trusted_hash}`（S006 的 trusted_hash 口径一手确认）；`bypass_hook_trust` 旗标跳过。
- 事件 12 个：PreToolUse、PermissionRequest、PostToolUse、PreCompact、PostCompact、SessionStart、SessionEnd、UserPromptSubmit、SubagentStart、SubagentStop、Stop、**Interrupt**（无 Notification）。stdin snake_case 加 Codex 扩展 `turn_id`、`permission_mode`；输出 wire camelCase，注释明言兼容 Claude 语义（"Claude requires reason when decision is block"）。

### 3. Grok（xai-org/grok-build 源码）

[实证: crates/codegen/xai-grok-hooks/src/{config,event,discovery}.rs、xai-grok-workspace/src/{project_config,folder_trust}.rs，浅克隆 2026-08-31]

- 注册：**Claude 同构 JSON**（单测名就叫 `parse_claude_format_single_hook`，且直接解析真实 Claude settings.json 单测 `realistic_claude_settings_file`）：

```json
{"hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": [
  {"type": "command", "command": "…", "timeout": 10, "env": {"K": "V"}}
]}]}}
```

- 源两种（workspace config.rs `HookSourceConfig`）：`SettingsFile`（可直接指 `~/.claude/settings.json`——grok 兼容读 Claude 配置）与 `Directory`（`~/.grok/hooks/*.json`、项目 `<project>/.grok/hooks/*.json`）。TOML 层 `[[hooks.<Event>]]` 写 `.grok/config.toml`（项目层从 cwd 向上走到 git root 逐层叠加）。
- handler 两型：command、http（无 prompt/agent/mcp_tool）；`env` map 注入 hook 进程；command/url 支持 `$VAR` 环境展开（matcher 不展开）。
- 事件集（event.rs 宏）：SessionStart、PreToolUse、PostToolUse、PostToolUseFailure、SessionEnd、Stop、StopFailure、StopCancelled、Notification、UserPromptSubmit、**PermissionDenied**、SubagentStart、SubagentStop、SubagentEnd、PreCompact、PostCompact，另有 legacy 别名（beforeShellExecution→PreToolUse 等）。**无 PermissionRequest**：Claude settings 里的 PermissionRequest 段被 lenient skip（未知事件跳过不报错）。oma 的 grok 路 blocked 信号只能靠 PermissionDenied（已拒，非等待）与 Notification，等待审批态走 1b 画面兜底。
- matcher 是正则（invalid regex 报错）；Stop 等 MatcherPolicy::Ignored 事件带 matcher 只警告不生效。超时默认 5s，Stop 门 600s，prompt 门 30s。
- runner 恒注入 env：`GROK_HOOK_EVENT`、`GROK_HOOK_NAME`、`GROK_SESSION_ID`、`GROK_WORKSPACE_ROOT`、`CLAUDE_PROJECT_DIR`（oma hook 可直接读这些，不依赖 stdin 也行）。
- 名字前缀分层：`global/<stem>`、`project/<stem>`、`plugin/`、`agent:`；层间 additive、同命令 dedup 保高层。
- folder-trust 门（folder_trust.rs）：repo 内 `.grok/hooks/`、`.claude/settings.json`、`.cursor/…` 等都被信任检查覆盖（与 S006/S008 口径一致，现为源码级确认）。

### 4. Kimi（MoonshotAI/kimi-code TS 源码与官方文档）

[实证: packages/agent-core/src/session/hooks/types.ts、src/config/schema.ts HookDefSchema、agent-core-v2 projectLocalConfigService.ts、docs/zh/customization/hooks.md，浅克隆 2026-08-31]

- 注册：**仅用户级** `~/.kimi-code/config.toml`（`KIMI_CODE_HOME` 可迁移数据目录）`[[hooks]]` 扁平数组：

```toml
[[hooks]]
event = "PreToolUse"
matcher = "Bash"        # 正则；不填匹配全部
command = "oma hook"
timeout = 10            # 1–600，默认 30
```

- schema `.strict()`：只许这四字段，多写一个字段整个 config 加载失败。`HookDef` 接口另有 cwd/env 但配置 schema 不收（程序内构造用）。
- **项目级裁决（S008 悬案关闭）**：项目内只有 `.kimi-code/local.toml`，其 schema 仅 `workspace.additional_dir` 一项——**项目级 hook 注册不存在**。oma 对 kimi 的退路：hook 命令自带环境守卫（无 `OHMYAGENTS_STATE_FILE` 即 exit 0），或经用户同意写用户级。
- 事件约 20 个（文档表）：UserPromptSubmit、UserPromptQueued、PreToolUse、Stop、TurnStarted、PostToolUse、PostToolUseFailure、PermissionRequest、PermissionResult、SessionStart、SessionEnd、SessionHeartbeat、SubagentStart/Stop、TaskStarted、StopFailure、Interrupt、Pre/PostCompact、Notification。**可阻断仅 PreToolUse/Stop/UserPromptSubmit**，其余观察型（返回值不影响主流程）。
- stdin snake_case：hook_event_name/session_id/session_title/client_type/cwd 加事件字段；退出码 0 放行（stdout 可附加上下文）、2 阻断（stderr 回 LLM）、其他 fail-open；JSON `hookSpecificOutput.permissionDecision: deny` 同 Claude。
- 同事件多 hook 并行、同 command 去重；Stop 防循环（stop_hook_active 只再触发一次）。
- 区分：`MoonshotAI/kimi-cli`（11k stars）是 Python 旧版，本仓不采用；TS 本尊是 `kimi-code`。

### 5. 交叉对照（oma init 部署矩阵）

[实证: 上述源码与文档汇总]

| 家 | 项目级注册落点 | 形态 | Windows 专用 | blocked 等待事件 | 信任门 |
| --- | --- | --- | --- | --- | --- |
| Claude | `.claude/settings.json` | `hooks.<Event>[].{matcher,hooks[]}` | exec form + `shell:"powershell"` | PermissionRequest（可程序化裁决） | workspace trust |
| Codex | `.codex/hooks.json` 或 `.codex/config.toml [hooks]` | Claude 同构 + `commandWindows` | `commandWindows` | PermissionRequest | `hooks.state.trusted_hash` |
| Grok | `.grok/hooks/*.json` 或 `.grok/config.toml [[hooks.<E>]]` | Claude 同构（command/http） | 无 | 无（PermissionDenied 是已拒） | folder-trust |
| Kimi | **无项目级**（仅 `~/.kimi-code/config.toml [[hooks]]`） | 扁平四字段 strict | 无 | PermissionRequest（观察型） | 用户级自带 |

共同点（oma hook 单一二进制吃四家的基础）：stdin JSON 都含 `hook_event_name` 与 `cwd`（Grok 双形态 hookEventName/hook_event_name 都收）；exit 2 全家都是 block 语义；事件名 PascalCase 全家一致（Grok 收 legacy 别名）。

## 踩坑沉淀

| 坑 | 正解 |
| --- | --- |
| 把 win-rmux/evo 的二手注册形态当权威 | 官方源码/文档一手优先；二手只当线索 |
| rg 误用 `-rln`（`-r ln` 是替换标志） | 想要 `-l -n` 就分开写；`-r` 会把匹配文本替换成给定串 |
| Claude 旧口径「无 PermissionRequest」 | 官方参考页已有该事件且可 allow/deny 裁决 |
| 给 Kimi 项目级写 `[[hooks]]` | local.toml schema 只认 workspace.additional_dir，写了无效 |

## 待办

- `poc-init` 按本矩阵实现项目级部署（Kimi 路只落 skill 与守卫说明）
- Claude PermissionRequest 程序化裁决并入 dialogs 部件设计（可替代 sendkeys 点框的路径）
- Codex `commandWindows` 与 Grok http handler 是否采纳，poc-init 定
- kimi-code 版本对齐：本机安装版本与仓 HEAD 的差异核对

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| web | code.claude.com/docs/llms.txt → en/hooks.md | 2026-08-31 | Claude hook 事件/schema/裁决/超时/信任 |
| git | openai/codex 浅克隆 | 2026-08-31 | hook_config.rs、schema.rs、discovery.rs |
| git | xai-org/grok-build 浅克隆 | 2026-08-31 | xai-grok-hooks 全套、folder_trust、project_config |
| git | MoonshotAI/kimi-code 浅克隆 | 2026-08-31 | HookDefSchema、types.ts、projectLocalConfigService、官方 hooks 文档 |
