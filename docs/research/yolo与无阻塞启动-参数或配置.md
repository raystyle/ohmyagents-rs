# yolo与无阻塞启动：参数或配置

> 2026-08-29。对照 `D:\ohmypwsh` 方案 0017 节 1.1b、`set-*-config.ps1`、`verify-five-ends.ps1`，以及 evo-harness 启动 flags。为编排器「先无阻塞拉起、再委派任务」提供命令与配置口径。

## 背景

编排器要能把各 agent 指到具体任务上：启动不能卡在信任框、权限框、登录框；诊断不能堵主命令。ohmypwsh 把这件事叫 **yolo / 非阻塞参数化持久化**（2026-08-25 定调，方案 `0017`，追踪 `raystyle/evo-harness#2`）：权限和信任写进配置文件，不靠每次启动传 flags。本仓定位是通用多 Agents 自动配置和任务编排器，命令必须同时支持**配置落盘**和**单次参数覆盖**。

## 关键结论

1. **启动无阻塞靠配置，不靠现场按键。** `init` 把 yolo 与项目信任写进该项目能加载的文件；spawn 时 TUI 不应再弹框。evo-harness 的 `DIALOGS` 按键只当兜底。[经验: ohmypwsh 0017 + evo-harness pretrust]
2. **诊断无阻塞、不 attach。** `doctor` / `status` 只读磁盘：yolo 键、信任库、`.ohmyagents\state\`、任务到 agent 的映射。不 `send-keys`、不等 `wait_ready`。[实证: 2026-08-29 `poc-yolo-doctor` 不 attach]
3. **任务指向与启动分离。** spawn 立即返回；agent 与 task-id 写在项目内 JSON。主 CLI 委派任务时只要求「该 agent 对应该 task 且状态不是 blocked」，不必盯着 TUI。[假设: 产品 `spawn` 尚未落地]
4. **参数与配置并存。** 默认走配置（对齐 ohmypwsh）；`--yolo` / `--permission-mode` 只覆盖这一次会话。受控编排不应把信任绕过长期留在 argv 里。

## 无阻塞诊断：agent 指向任务

编排器要回答三句，且都不阻塞：

| 问 | 读什么 | 不做什么 |
| --- | --- | --- |
| 这个 agent 配好 yolo 了吗 | 项目与用户配置里的权限/信任键 | 不启动 TUI |
| 这个 agent 现在闲着还是卡住 | `<project>\.ohmyagents\state\<agent>.json` | 不 capture-pane、不轮询 CPU |
| 这个 agent 在跟哪条任务 | `<project>\.ohmyagents\tasks\<task-id>.json` | 不 round-trip 进 pane |

任务文件建议形状：

```json
{
  "id": "t-20260829-1",
  "goal": "…",
  "agents": { "claude": "assigned", "codex": "assigned" },
  "cwd": "D:\\foo"
}
```

`oma doctor` 退出码：缺二进制、缺 yolo 键、缺项目信任、state 为 blocked 且过期，则非 0。全程文件与进程表，不进交互。ohmypwsh 的 `verify-five-ends.ps1` 已用同样思路扫 `skipDangerousModePermissionPrompt`、grok folder trust、codex `hooks.state` 残留（脚本内 `claudeSkip=` / `grokFT=` / `codexClean=`）。

## yolo 与非阻塞写哪里

ohmypwsh 0017 表是权威对照。本仓默认写**项目级**能加载的文件；信任库仍在用户家（agent 自己的存储，见《项目级hook与skill》）。

| agent | 权限/沙箱（项目或用户配置） | 项目信任（用户家，init 预写） | 额外非阻塞键 |
| --- | --- | --- | --- |
| claude | `.claude/settings.json`：`permissions.defaultMode=bypassPermissions` | `~/.claude.json` projects 绝对路径：`hasTrustDialogAccepted`、`hasTrustDialogHooksAccepted`；`hasCompletedOnboarding=true` | `skipDangerousModePermissionPrompt=true`（ohmypwsh 本机已证实生效；WSL 脚本曾写进 `permissions` 对象，以 0017 顶层键为准再核一次） |
| codex | `.codex/config.toml`：`sandbox_mode=danger-full-access`、`approval_policy=never` | `[projects."<abs>"] trust_level=trusted` | `[hooks.state."<源>"] trusted_hash` 替代 `--dangerously-bypass-hook-trust` |
| kimi | `.kimi-code/config.toml` 或用户配置：`default_permission_mode=auto` 或 `yolo`（`set-kimi-config.ps1 -PermissionMode`） | `~/.kimi-code/workspace-trust/wd_*` | 无 hook-trust 框 |
| grok | 项目或 `~/.grok/config.toml`：`[ui] permission_mode=always-approve` | `~/.grok/trusted_folders.toml` 替代 `--trust` | `--fullscreen` 仍是启动形态，不是信任绕过 |

ohmypwsh 明确：evo-harness 的 `--dangerously-bypass-hook-trust` / `--trust` / `--allow-dangerously-skip-permissions` 属于「未参数化的疏漏」，应对齐成上表落盘。权限类 flags（`--auto`、`--always-approve`、`--dangerously-skip-permissions`）与配置等价，编排器可作**显式申明**，不要当作唯一开关。

## 命令参数

建议（实现前的口径）：

```text
oma init [--yolo|--permission-mode auto|yolo|manual]
oma doctor              # 无阻塞：yolo 键 + 信任 + 任务映射 + state
oma spawn [--yolo] [--no-block]
oma run <task> --assign claude,codex
```

- `init --yolo`（默认）：写上表持久化键，并预写当前项目绝对路径的信任库。
- `init --permission-mode manual`：不写 bypass / never / always-approve，供人盯着 TUI 用。
- `spawn --no-block`（默认）：拉起 pane 后立即返回；就绪由 `doctor`/`status` 看，不把 CLI 卡住。
- `spawn --yolo`：本会话 argv 再带等价 flags，覆盖尚未落盘的环境；有配置时 flags 可省略。
- `run --assign`：把 task-id 写进任务 JSON，再 drive。agent 未 idle 则记录 pending，不阻塞其它路。

REPL 里 `status` 打任务指向，不 attach。

## 无阻塞初始启动怎么才算过

spawn 之后立刻允许委派，当且仅当：

1. `doctor` 对该 agent 的 yolo 与信任全绿。
2. pane 进程活着（list-panes / pid），不必等输入框文案。
3. state 文件不是 `blocked`。`unknown` 或空表示 hook 还没上报，允许先发；Drive 仍走三段式，框还在才用 DIALOGS 兜底。

禁止：主命令里 `wait_ready` 数分钟；禁止为了清输入框对 Codex 发 `C-c`。

## 和已有研究的关系

- 项目级文件落点：《项目级hook与skill》
- 委派正文：《drive铁律与三段式粘贴》
- 运行时状态：《agent状态通道》
- 本篇只管：启动前把路铺平、启动后诊断不堵、任务和 agent 的指向可查

## 落地（POC，2026-08-29）

- `oma init --yolo --project <dir>`：只写项目文件。`skipDangerousModePermissionPrompt` 写在 `.claude/settings.local.json` **顶层**（不进 `permissions`）。
- `oma init --yolo --pretrust`：才写用户家信任库。Grok `permission_mode` 仍只能写 `~/.grok/config.toml`。
- `oma doctor --project <dir>`：只读，打印 `agent= check= status=ok|block`；`doctor.blocked=true` 时退出 1。
- Codex 信任只认用户 `~/.codex/config.toml` 的 `[projects]`；项目级 `[projects]` 不能清信任框。
- 验收：`cargo run --example poc-yolo-doctor`（临时目录，默认不 `--pretrust`）。

## 待办

- 项目级 `.codex/config.toml` 的 `[hooks.state]` 能否替代用户级 trusted_hash
- hook/skill 的 `poc-init` 另做，不并进本切片
- 无头 `-p` 与 TUI pane 是两条路，见 `YouMind无头模式与YOLO免审批对照-Claude-Grok-Kimi.md`；将来若加 headless 一次性任务再按三家分配方
