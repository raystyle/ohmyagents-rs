# R007-agent信任与无阻塞参考-四家配置与检测

> 角色：写 `oma init` / `doctor` / `hook` 相关代码时查的**做法参考**（要做什么怎么做）。从 `docs\research\` 的信任阻塞门、yolo 与无阻塞、项目级 hook、agent 状态判断四篇浓缩；出错排查见 `INDEX.md（mistakes 节）`。全部条目可溯源六态。

## 一、四家 yolo 落盘

> `init --yolo` 写什么

| agent | 权限/沙箱 | 信任库（`--pretrust` 才写用户家） | 注意 |
| --- | --- | --- | --- |
| claude | 项目 `.claude/settings.json` `permissions.defaultMode=bypassPermissions`；`.claude/settings.local.json` 顶层 `skipDangerousModePermissionPrompt=true` | `~/.claude.json` `projects.<abs>.hasTrustDialogAccepted`；`hasCompletedOnboarding` | skip 键 scope 限 User/local/managed，**不写共享项目文件、不进 permissions 嵌套** [实证: 官方 settings-reference + poc-yolo-doctor] |
| codex | 项目 `.codex/config.toml`：`sandbox_mode=danger-full-access`、`approval_policy=never` | 用户 `~/.codex/config.toml` `[projects."<abs>"] trust_level=trusted` | 信任只认用户层；项目 `[projects]` 无效 [实证: poc-yolo-doctor] |
| kimi | 项目 `.kimi-code/config.toml` `default_permission_mode=auto|yolo` | `~/.kimi-code/workspace-trust/wd_*` | 无 hook-trust 框 [经验: kimi docs] |
| grok | **仅用户** `~/.grok/config.toml` `[ui] permission_mode=always-approve` | `~/.grok/trusted_folders.toml` | 官方明文不能写项目；always-approve 下 deny 与 PreToolUse hook 仍生效（secret-guard 有用）[实证: 官方 permissions 页] |

flags（`--yolo`、`--always-approve` 等）只作单次覆盖，配置落盘优先。[经验: ohmypwsh 0017]

## 二、信任门检测

> doctor 拆类

- `trust.project`：folder 键在不在（父路径覆盖子目录；Windows 路径可能裂多条键）。
- `trust.hooks`：Claude 交互下 covered_by folder；Codex 看 `[hooks.state] trusted_hash`；Grok 未信任且有 hooks 报 block（无头静默跳过）。
- `trust.mcp`：独立门。Claude committed 项目审批不算数（v2.1.196+），生效来源：用户 / managed / `--settings` / 未跟踪 local。
- `trust.skill`：**只对 skills-dir plugin 形态报**（skill 目录带 `.claude-plugin/plugin.json`）；普通 `.claude/skills` 归 `trust.project`。（依据 `docs
esearch\S006-信任阻塞门-四家种类与官方口径.md` 2026-08-31 裁决节）
- `yolo` / `skip_prompt` / `onboarding` 分立检查。
- doctor 退出码：缺二进制、缺 yolo 键、缺信任、blocked 过期则非 0；全程只读磁盘与进程表，不 attach。[实证: poc-yolo-doctor]

## 三、无头分路

> 将来 `oma run --headless`

| 家 | 无头开关 | 默认权限 | 全自动要 |
| --- | --- | --- | --- |
| claude | `-p` | Manual | `--allowedTools` 或 `--permission-mode dontAsk\|acceptEdits`；校验产物不看 exit 0 |
| grok | `-p` | Ask | `--always-approve` 或 `dontAsk`+`--allow`（加 `--no-auto-update`） |
| kimi | `-p` | **即 auto** | **禁止**再叠 `--yolo`/`--auto`/`--plan`（`--yes`/`--auto-approve` 是隐藏别名） |
| codex | — | — | 走 `.codex/config.toml`（不在 YouMind 范围） |

[实证: 2026-08-29 官方 headless/permission-modes/kimi-command + 本机三家家 help]

## 四、hook 与状态通道

> `oma hook` 怎么接

1. spawn 注入 `OHMYAGENTS_PROJECT` / `OHMYAGENTS_AGENT` / `OHMYAGENTS_STATE_FILE`；各家项目 hook 的 `command` 调 `oma hook`（stdin 事件 JSON 或 `oma hook blocked`）。[实证: poc-dialogs]
2. hook 缺环境变量或项目对不上 **exit 0**（安全带：用户级误装也不污染别的仓库）；不连 rmux 管道。（依据 `docs
esearch\S008-项目级hook与skill.md` 安全带节）
3. 事件映射四态：idle（SessionStart/Stop/Interrupt/SessionEnd）、working（UserPromptSubmit/Pre/PostToolUse 等）、blocked（PermissionRequest，Codex/Kimi）、unknown（Notification，**不映 idle**）。事件名双形态归一。[经验: evo-harness STATE_MAP]
4. hook 注册项目级：Claude settings `matcher:"*"`；Codex 须先信任再写 `hooks.state.trusted_hash`；Grok 项目 hooks 要 folder-trust；Kimi 暂用户 config 加项目脚本。[经验: 各家官方]
5. hook 沉默不判 idle：走终端语义兜底（`terminal_state` 分类 password/confirm、`wait_for_text` 等执行证据）。Codex `Stop` 常不触发，working 超阈值标 stale。（依据 `docs
esearch\S010-clum等待原语作为hook兜底状态.md`）
6. 报阻塞（hook 写文件）与点阻塞（drive 发键）分路，不合成一条 pipe。（依据 `docsesearch\S009-agent状态判断-通道与分层.md` 分层模型）

## 五、委派前检查清单

按序核：配置优先 flags 单次覆盖；Claude skip 键写 User 或 Local；Codex 先 `[projects]` 再项目 hook；Grok 权限模式只写用户 config；Kimi hook 注册暂走用户 config；Drive 用 `paste-buffer -p`；spawn 立即返回；不把 Quiet 当 idle。[经验: 2026-08-29 复核定调]

## 六、禁止清单

- 把 yolo 当 trust（或反之）；信任绕过长期留 argv
- skip 键写共享项目文件或 permissions 嵌套；Grok permission_mode 写项目
- Kimi `-p` 叠 `--yolo`；把 exit 0 当无头成功
- hook 上报走 `rmux set-environment`；Notification 映 idle
- doctor 里 attach、send-keys、wait_ready
