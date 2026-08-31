# S014-检测已装agent-PATH与默认目录与环境变量

> 2026-08-29。用户要求检测系统装了哪些 agent，覆盖 Windows / Linux / macOS；扫描 PATH、默认位置、自定义位置（环境变量）。

## 背景

只跑 `which claude` 会漏掉未进 PATH 的官方安装（尤其 Windows 的 `~\.local\bin` 和 Codex 的 `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin`）。编排要知道本机有哪些可 spawn 的二进制，不能把「不在当前进程 PATH」当成没装。

## 关键结论

扫描顺序是 env > PATH / `OMA_AGENT_PATH` > 各家默认目录。只跑 `which` 会把未进当前进程 PATH 的官方安装当成没装。[实证: 2026-08-29 `oma agents` 本机四路均命中；claude 在 `~\.local\bin`]

## 扫描顺序

同一 agent 多处命中时取最高优先级，其余打 `extra=`。

1. **显式环境变量**（`source=env`）：`OMA_CLAUDE_BIN` / `OMA_CODEX_BIN` / `OMA_GROK_BIN` / `OMA_KIMI_BIN`，以及 `CLAUDE_BIN`、`CODEX_BIN`、`GROK_BIN`、`KIMI_BIN`、`KIMI_CODE_BIN`。
2. **PATH 与自定义路径列表**（`source=path`）：进程 `PATH`；额外 `OMA_AGENT_PATH`（分隔符与 PATH 相同）。Windows 再按 `PATHEXT` 试 `.exe` / `.cmd` / `.bat`。`CODEX_HOME` 下的 `packages/standalone/current` 也当路径根扫。
3. **默认安装位置**（`source=default`），即使不在 PATH：

| agent | Windows | POSIX |
| --- | --- | --- |
| claude | `~\.local\bin\claude.exe`；`~\.claude\bin`；WinGet `%\LOCALAPPDATA%\Programs\ClaudeCode`；npm `%APPDATA%\npm` | `~/.local/bin/claude`；`~/.claude/bin`；`~/.claude/local`；Homebrew `/opt/homebrew/bin`、`/usr/local/bin` |
| codex | `%LOCALAPPDATA%\Programs\OpenAI\Codex\bin`；`~\.codex\packages\standalone\current` | `~/.local/bin/codex`；`~/.codex/packages/standalone/current`；`~/.cargo/bin` |
| grok | `~\.grok\bin\grok.exe`；`~\.local\bin` | `~/.grok/bin/grok`；`~/.local/bin` |
| kimi | `~\.kimi-code\kimi.exe`（及 `kimi-code.exe`、`bin\`） | `~/.kimi-code/kimi`；`~/.local/bin/kimi` |

对照 ohmypwsh《四家 agent 官方安装位置研究》。

## 命令

```text
oma agents
```

缺装打印 `status=missing`，退出码仍为 0。`oma doctor` 的 `check=binary` 走同一套探测。
