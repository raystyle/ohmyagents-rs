# S008-项目级hook与skill

2026-08-29。2026-08-31 注：各家注册形态已经一手源码核实并细化，见《S015-四家hook注册一手形态-官方文档与源码核实》；本文的落点结论与 S015 一致，注册 JSON/TOML 具体形状以 S015 为准。

这条多路编排钉在**一个项目目录**上：从哪启动，就只在哪加载 hook 和 skill。不要去改用户家里的 `~/.claude`、`~/.codex`。win-rmux 的安装器会写全局配置，evo-harness 后来也退回用户级——两边都有理由，但不是我们要的边界。

(本会话用户定调「都是项目级生效」「hook 只会在启动目录加载」「命令还要部署 agents 和符合各 agents 的 skill」)

## 别人为什么逃到用户级

evo-harness 起初默认「只写 run 工作目录的 `.claude/settings.json`」。后来 `install_claude_project_hooks` 改成写 `~/.claude/settings.json`，注释写得很明白：项目级每次命令变更都会弹 hook 信任确认，launch 探针被框挡死，grok-debut 两轮 `LAUNCH_FAILED`。(`D:\sourcecode\evo-harness\src\evo_harness\install_hooks.py` L1–6 与 L197–204)

win-rmux 的 `install-agent-hooks.ps1` 直接写 `~/.codex/hooks.json`、`~/.claude/settings.json`、`~/.kimi-code/config.toml`，SKILL 前置守卫也警告这会污染以后所有会话。(`scripts/install-agent-hooks.ps1` L169–180；SKILL.md L84–87)

我们接受信任框这个成本，用预信任和 yolo 旗标把它压下去，而不是把 hook 装进家目录。信任**存储**仍然在用户家（agent 自己的设计），那是例外，不是把 hook 注册也一并全球化。

## 各家项目级落点

启动 cwd = 项目根（或 `--project`）。agent 按自己的发现规则从该目录往上找。下面是官方/源码口径，不是本仓库跑出来的。

**Claude** 项目级 settings 是 `<project>/.claude/settings.json`，skill 在 `.claude/skills/`。项目 hook 要过目录信任（交互下所有 settings hooks 卡到 folder 信任被接受，无独立 persist 键，见《S006-信任阻塞门-四家种类与官方口径》）；预写 `~/.claude.json` 的 `hasTrustDialogAccepted` 即可（`hasTrustDialogHooksAccepted` 官方未记载，双写无害不构成检测依据）。

**Codex** 项目配置 `./.codex/config.toml`（向上找到项目根），hook 还可以是 `./.codex/hooks.json`。(OpenAI Codex hooks 文档「User config `~/.codex/config.toml`；Project config `./.codex/config.toml`」) 未信任项目会发现配置但当 disabled layer。预写 `[projects."<abs>"] trust_level = "trusted"`。(`pretrust.py` `pretrust_codex`) Skill：从 CWD 走到 repo 根的 `.agents/skills`。(developers.openai.com/codex/skills)

evo-harness 还写过：用户层声明在 `~/.codex/config.toml` 的 `[hooks.<Event>]`，`~/.codex/hooks.json` 不是用户配置加载点。(`install_hooks.py` L80–83) **该主张已被官方文档推翻（2026-08-29 复核裁决）**：Codex 用户层 `~/.codex/hooks.json` 与 `config.toml` 都加载；项目层 `hooks.json` 与 `config.toml` 并存时「两处都扫描、匹配的都跑」。

**Grok** 项目 hook 在 `<project>/.grok/hooks/*.json`，要 `/hooks-trust` 或启动 `--trust`，决定记在 `~/.grok/trusted_folders.toml`。(x.ai/docs/build/features/hooks) Skill：`.grok/skills/` 向上走到 repo 根。(x.ai skills-plugins-marketplaces) 它还会读 `.claude/settings.json`。(同上 hooks 页；evo-harness `install_grok_hooks` 注释)

**Kimi** skill 项目级是 `.kimi-code/skills/` 和 `.agents/skills/`（项目根 = 向上最近的 `.git`）。(moonshotai kimi-code skills 文档 Project > User) Hook 官方例子写在 `~/.kimi-code/config.toml` 的 `[[hooks]]`。(kimi.com/code docs hooks) **项目级 hook 注册不存在（2026-08-31 源码裁决）**：项目内只有 `.kimi-code/local.toml`，其 schema 仅收 `workspace.additional_dir`，写 `[[hooks]]` 无效。[实证: kimi-code TS 源码 projectLocalConfigService.ts，见 S015]

## 信任框要先写掉

`pretrust.py` 是 spawn 前的机械步骤，不是 hook 本身。(文件头 L1–16，2026-08-24 WSL)

| agent | 预写 | 启动旗标 |
|---|---|---|
| claude | `~/.claude.json` projects[abs] `hasTrustDialogAccepted`（hooks 键不必要，见《信任阻塞门》） | `--allow-dangerously-skip-permissions` 只管工具，不管目录/hook 框 |
| codex | 用户 `config.toml` `[projects."abs"] trust_level`（只认用户层） | `--dangerously-bypass-hook-trust`；官方说 --yolo 不绕过 trust |
| kimi | `workspaces.json` + `workspace-trust/wd_...` | 默认高亮 Don't trust，必须 Up×3 |
| grok | `trusted_folders.toml`（agent 自己记） | `--trust` |

预写改的是用户家的**信任库**，不是把 hook 命令写进家目录。Oh My Agents 可以复用这套幂等预写，但 hook 脚本和注册落在项目里。

## `init` 往项目里部署什么

(据各家发现规则 + 用户「命令部署 agents 和 skill」)

```text
<project>/
  AGENTS.md                         # 四路共用的项目说明（codex/grok/kimi 都读）
  CLAUDE.md                         # 一行 @AGENTS.md
  .agents/skills/<name>/SKILL.md    # Codex + Kimi 的仓级 skill
  .claude/settings.json             # 仅本项目 hook
  .claude/skills/                   # Claude 项目 skill（可由 .agents/skills 同步）
  .codex/config.toml                # features.hooks + 项目 [hooks]
  .grok/hooks/ohmyagents-state.json
  .grok/skills/
  .kimi-code/skills/
  .ohmyagents/state/<agent>.json    # 状态通道，见《S009-agent状态判断-通道与分层》
```

2026-08-31 订正：上树的 `.ohmyagents/hook.py` 已不实施——四端共用入口是 `oma hook` 子命令，注册里写 oma 二进制绝对路径（Claude 用 exec form `command+args:["hook"]`；Codex 可用 `commandWindows`；Grok/Kimi command 单字段）。形态见 S015 部署矩阵。[实证: S015]

`oma init` 幂等：只增不删用户已有 hook 条目，按脚本名去重；JSON 解析失败拒写。skill 以 `.agents/skills` 为源，再按各家目录各放一份或做拷贝——Claude 不扫 `.agents/skills`。(据 Claude 只声明 `.claude/skills`)

Kimi 项目级 hook 不存在（见上节裁决）：退路是 hook 命令自带环境守卫，没有 `OHMYAGENTS_PROJECT` 就立刻退出。即使用户级误装了一条，也不会在别的仓库报状态。(据 evo-harness `agent_state_hook.py` L89–91 `EVO_STATE_FILE` 缺失则 exit 0) 这是安全带，不是允许默认去写 `~/.kimi-code/config.toml`。

## 所以

Hook 和 skill 的**注册与文件**都在启动的那个项目目录。[推断: 各家官方「从启动目录向上找项目配置」] 家目录只允许写信任库（否则 TUI 弹框，会话起不来）。[经验: evo-harness pretrust + win-rmux 信任框] `init` 是部署命令，`serve` 只 spawn、不再改全局 agent 配置。
