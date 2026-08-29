# YouMind无头模式与YOLO免审批对照-Claude-Grok-Kimi

> 2026-08-29。学习研究公开文档 [https://youmind.com/d/QO6XcZnIybiBU1](https://youmind.com/d/QO6XcZnIybiBU1)（标题《编码智能体 CLI 无头模式与 YOLO 免审批完全对照：Claude Code · Grok Build · Kimi Code》，YouMind 文件 id `01a04d75-2b3b-74ba-9771-cd2c87219447`，`updated_at` 2026-08-29T12:19:11Z）。对照官方文档、本机 `--help`、GitHub。

## 需求

- 研究：这篇 YouMind 文的核心结论、三家对照表、失败模式与选型；和本仓已有《yolo与无阻塞启动》的关系；对 Oh My Agents（rmux pane TUI，不是 CI `-p`）意味着什么。
- 核查：无头 ≠ YOLO；Claude `-p` 静默拒绝；Grok `-p` 可能卡审批；Kimi `-p` 默认 auto 且禁止叠 `--yolo`；各家 YOLO 开关与配置键；权限 ≠ 沙箱。
- 意图：混合（先读原文，再核官方与本机）。

## 背景

YouMind 这篇是 2026-08 的三家 CLI **无头（`-p`）** 与 **免审批（YOLO）** 对照，不覆盖 Codex。本仓刚落地的是 **TUI pane** 的项目级 yolo 落盘与 doctor。两者容易被当成同一件事：都叫 yolo，但一条是「去掉 TUI 的一次性进程」，一条是「pane 里不弹审批框」。

## 结论

### 1. 原文主结论成立：无头 ≠ YOLO，三家架构不同

| 家 | 无头开关 | 无头默认权限 | 要全自动还要 | 无头遇审批 |
| --- | --- | --- | --- | --- |
| Claude Code | `claude -p` / `--print` | Manual（`default`），逐项审批 | `--dangerously-skip-permissions` 或 `--permission-mode bypassPermissions` | 无 UI 则该次调用被拒；流程仍可标成功 |
| Grok Build | `grok -p` / `--single` | Ask；`-p` 不含免审批 | `--always-approve`（官方 CLI 参考写明 alias `--yolo`） | 官方未写死卡死；社区/Hermes 写「可能 stall 等审批」；企业文档另给 `dontAsk`（无 allow 则静默拒绝） |
| Kimi Code | `kimi -p` / `--prompt` | **就是 `auto`**：常规工具自动批，deny 仍生效 | 不要叠 `--yolo`：官方 **禁止** `-p` 与 `--yolo`/`--auto`/`--plan` 同用 | 审批问题被设计消掉；风险改成限流时静默挂起 |

一句话：从 Claude/Grok 脚本抄到 Kimi 时，最容易踩的是「再加一遍 `--yolo`」——Kimi 会直接拒绝这个组合。[实证: 2026-08-29 对照官方 headless / permission-modes / kimi-command；本机 claude 2.1.246、grok 1.0.13、kimi 0.38.0 `--help`]

### 2. 核查结果（对照官方 + 本机 2026-08-29）

本机：`claude 2.1.246`、`grok 1.0.13`、`kimi 0.38.0`。

| 主张 | 判定 | 依据 |
| --- | --- | --- |
| Claude `-p` 不改变权限门控 | **成立** | 官方 headless：`-p` 常与 `--allowedTools` / `--permission-mode` 叠用；permission-modes：`claude -p` 的内置起始模式是 `default`（Manual），即使 Pro/Max/Team 交互默认已是 `auto` |
| Claude 无头假绿（exit 0、没干活） | **方向成立，细节社区化** | 官方：无 prompt UI 时待审批调用被拒；`-p` + `--dangerously-skip-permissions` 下「仍会提示的少数调用改为拒绝」。`subtype=success` / `is_error=false` 出自社区文，本轮未复跑 JSON 样例 |
| Grok `-p` 与 `--always-approve` 正交 | **成立** | 官方 Headless 表把二者并列；CLI 参考：`--always-approve` auto-approve，alias `--yolo`；`--dangerously-skip-permissions` 也是兼容别名 |
| Grok 无头默认会卡住 | **说法不一** | Hermes（2026-08-29）写 stall；官方企业节用 `dontAsk` + `--allow` 做 CI，说明无头命运取决于 mode，不是只有卡死一种 |
| Grok `permission_mode` 取代 `yolo`/`approval_mode` | **成立，但旧键仍可用** | 官方 Permissions：用户 `~/.grok/config.toml` 的 `[ui] permission_mode = "ask" \| "auto" \| "always-approve"`；**不能写项目** `.grok/config.toml`；`approval_mode` 与 `yolo = true` 仍工作，`permission_mode` 优先 |
| 权限 ≠ 沙箱 | **成立** | Grok 官方第一句就把 Permissions 与 Sandbox 拆开；Claude 官方同样拆 permission mode 与 Bash sandbox |
| Kimi `-p` 默认 auto，且不能叠 `--yolo` | **成立** | 官方 kimi-command：`-p` 不请求人工审批，按 auto；冲突规则：`--prompt` 不能与 `--yolo`/`--auto`/`--plan` 同用。本机 `kimi --help` 有 `-p`/`-y`/`--auto`/`--plan` |
| `--yes` / `--auto-approve` 是 `--yolo` 隐藏别名 | **成立** | 官方 kimi-command 原文 |
| Plan 退出审批不被 `--yolo` 绕过 | **成立** | 官方 warning |
| kimi-cli #2072「yolo 混同非交互」已修复 | **主线未核实** | issue 仍 **OPEN**；但本机 kimi-code 0.38.0 help 已写 “the agent may still ask questions”，产品语义已解耦 |
| Claude auto mode 分类器 + 无头连拒终止 | **部分过时** | Anthropic 工程博文（2026-03-25）写无头累计拒绝则 **terminate**；当前官方 permission-modes：`-p` 无 `--permission-prompt-tool` 时达阈值 **不终止进程，拒绝该动作后继续** |

本机 `grok --help` 额外出现 Claude 同构的 `--permission-mode`：`default, acceptEdits, auto, dontAsk, bypassPermissions, plan`。YouMind 表未写这一层；官方 Permissions 页仍用 ask/auto/always-approve 叙事。以本机 1.0.13 为准：CI 也可以走 `dontAsk` + `--allow`，不必只有 `--always-approve`。

### 3. 对本仓 Oh My Agents 的含义

当前 `oma` **不是** `claude -p` 编排器：四路活在 rmux pane 的 **TUI** 里。YouMind 的「静默假绿 / 卡审批 / `-p` 即 auto」主要打在 **将来的无头一次性任务**，不是现在的 `init --yolo` / `doctor`。

对现役 TUI 路径：

1. 项目级持久化仍然正确。Claude `permissions.defaultMode=bypassPermissions`、Grok 用户 `[ui] permission_mode=always-approve`、Kimi `default_permission_mode=yolo`，与原文「YOLO 配置键」一致。Grok **官方再次确认不能写项目** `.grok/config.toml` 的 permission_mode。
2. pane 里的阻塞是信任框 + 审批框，不是 `-p` 的 silent deny。`oma doctor` 扫 yolo/trust/binary/state 仍然对口。
3. 不要把 `kimi -p` 的默认 auto 当成交互 TUI 的默认。无 flag 的 `kimi` 仍是逐项审批；本仓写项目 `yolo`/`auto` 才是 pane 无阻塞。
4. 若以后加 `oma run --headless` / CI：三家必须分配方。Claude：`-p` + `--allowedTools` 或 `--permission-mode dontAsk|acceptEdits`，不要只看 exit 0。Grok：`-p` + `--always-approve` 或 `dontAsk`+allow，并加 `--no-auto-update`。Kimi：只 `-p`，禁止再加 `--yolo`。
5. YouMind **没有 Codex**。本仓第四路仍以 `.codex/config.toml` 的 `sandbox_mode` + `approval_policy=never` 为准（见已有 yolo 研究）。
6. 原文推荐「精确预授权优于全局 YOLO」。产品 spawn 可以以后加 `--allowedTools` 中间档；POC 的 `--yolo` 仍是受信项目的全局免审批。

### 4. 与已有 yolo 研究的差集

已有研究管 **TUI 启动不弹框、doctor 不 attach**。这篇补的是 **无头进程的权限默认值**：

- 无头和 YOLO 正交（Claude/Grok）或无头即 auto（Kimi）。
- 失败模式：Claude 假绿、Grok 可能 stall、Kimi hang。
- 校验产物/diff，不校验退出码。

不替代、不推翻 `init --yolo` 落盘表。

## 事实源

| 类型 | 定位 | 日期 | 对应需求 | 提供什么 |
| --- | --- | --- | --- | --- |
| web | [youmind.com/d/QO6XcZnIybiBU1](https://youmind.com/d/QO6XcZnIybiBU1) 经公开 `GET /api/v1/noAuthShare/getSharedEntity?shortId=QO6XcZnIybiBU1` 取 `content.plain` | 2026-08-29 | 研究原文 | 全文结论、对照表、速查卡 |
| web | [code.claude.com/docs/en/headless](https://code.claude.com/docs/en/headless) | 拉正文当日 | 核查 Claude `-p` | `-p` 与 `--allowedTools` / `--permission-mode` 组合；不暗示 YOLO |
| web | [code.claude.com/docs/en/permission-modes](https://code.claude.com/docs/en/permission-modes) | 拉正文当日 | 核查 Claude 起始模式与假绿 | `-p` 内置起始 `default`；无头达拒绝阈值不杀进程 |
| web | [anthropic.com/engineering/claude-code-auto-mode](https://www.anthropic.com/engineering/claude-code-auto-mode) | 2026-03-25 | 核查 auto mode | 分类器；当时写无头 terminate（已被现文档改） |
| web | [docs.x.ai/build/cli/headless-scripting](https://docs.x.ai/build/cli/headless-scripting) | 拉正文当日 | 核查 Grok `-p` | `-p` 与 `--always-approve` 分列；`--no-auto-update` |
| web | [x.ai/docs/build/features/permissions](https://x.ai/docs/build/features/permissions) | 拉正文当日 | 核查 Grok 配置 | 用户级 `permission_mode`；旧键仍可用；权限≠沙箱 |
| web | [x.ai/docs/build/cli/reference](https://x.ai/docs/build/cli/reference) | 拉正文当日 | 核查别名 | `--always-approve` alias `--yolo`；兼容 `--dangerously-skip-permissions` |
| web | [x.ai/docs/build/enterprise](https://x.ai/docs/build/enterprise) | 拉正文当日 | 核查无头卡死 | CI 示例用 `dontAsk` + `--allow` |
| web | [moonshotai.github.io 对应 kimi-command.md](https://github.com/MoonshotAI/kimi-code/blob/main/docs/en/reference/kimi-command.md)（raw blob `6d7c19e`） | 拉正文当日 | 核查 Kimi | `-p`=auto；冲突规则；隐藏别名；Plan 不被 yolo 绕过 |
| github | [MoonshotAI/kimi-cli#2072](https://github.com/MoonshotAI/kimi-cli/issues/2072) | 仍 OPEN | 核查 yolo/非交互解耦 | 主张成立但未关 issue |
| web | [hermes-agent … grok](https://hermes-agent.nousresearch.com/docs/user-guide/skills/optional/autonomous-ai-agents/autonomous-ai-agents-grok) | 2026-08-29 | 核查 Grok stall | 「无 `--always-approve` 时 headless 可能 stall」 |
| 本机 | `claude --help` / `grok --help` / `kimi --help` | 2026-08-29 | 核查 flag | 版本 2.1.246 / 1.0.13 / 0.38.0；grok 另有 Claude 式 `--permission-mode` |

## 缺口

- 未在本机对三家真实跑一遍 `-p` 写文件任务，因此 Claude JSON `subtype=success` 假绿、Grok stall vs dontAsk，都没有本机时序证据。
- YouMind 页是 SPA，正文不在 HTML；本轮用无鉴权 share API 取 plain。未装 `youmind` CLI、无 `YOUMIND_API_KEY`。
- X：无对齐到该短链或该标题的一手帖；领域讨论未采入。
- Codex 不在原文范围内，本轮不扩展 Codex `-p`/`--yolo`。
- Claude 受保护路径在 bypass 下是否仍提示：原文自己标「随版本反复」，本轮未核 2.1.246。
- `grok --help` 的 `--permission-mode bypassPermissions` 与文档 `[ui] permission_mode=always-approve` 的取值是否一一对应，未做 `grok inspect` 对照。
