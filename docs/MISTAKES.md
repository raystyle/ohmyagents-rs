# MISTAKES：错误沉淀指南

> 角色：**错误沉淀**——错误现象、根因、正确处理。主题深挖见 `docs\research\`，本文件是速查。

## 错误清单

| 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- |
| 本仓误跟 CoreSkills project skill 元文件 | 把 evo `project-docs` 当成 ohmypwsh 结构 | 文档骨架对照 `D:\ohmypwsh`（四段 AGENTS、三原语、history/research/references） | 2026-08-29 |
| CLI 曾写成 `ohmyagents` / `omg`，数据目录一度被改成 `.oma` | 项目名与二进制名混用 | 仓库 `ohmyagents`；命令 `oma`；数据目录 `.ohmyagents` | 2026-08-29 |
| 显示名写成 `ohMyAgents` | 把仓库目录名的驼峰当产品名 | 显示名是 `Oh My Agents` | 2026-08-29 |
| `rmux --version` 非 0 且打印 usage | tmux 风格，版本在 `-V` | 读版本用 `rmux -V`（`rmux 0.10.0`） | 2026-08-29 |
| 只拷一个 `rmux.exe` | tiny CLI 找不到 libexec helper | 必须保留 `rmux` + `libexec/rmux/rmux` + `rmux-daemon` | 2026-08-29 |
| broadcast-demo 自包 `\x1b[200~` | 演示竞速与生产 drive 不同 | 用 evo-harness `paste-buffer -p`，发送侧不自包壳 | 2026-08-29 |
| 对 Codex 预清输入发 `C-c` | `--no-alt-screen` 下 Ctrl-C 退出进程 | 禁止对 Codex 发 `C-c`；超时只补 Enter | 2026-08-21 |
| 项目级 Claude hook 弹信任框挡 launch | 命令变更触发 hook 信任 | spawn 前预写信任库；hook 注册仍只写项目目录 | 2026-08-25 |
| 在仓库 cwd 跑 `oma init --yolo` | init 默认写当前目录的 `.claude` / `.codex` / `.kimi-code` | POC 与演示用 `--project` 临时目录；`--pretrust` 才写家目录 | 2026-08-29 |
| 把项目 `.codex/config.toml` 的 `[projects]` 当成已信任 | Codex 先看用户 store，未信任则跳过项目层 | doctor 的 codex trust 只读 `~/.codex/config.toml` | 2026-08-29 |
| 把普通项目 skill 标成 n/a、只把 plugin 形态当门 | 官方 `allowed-tools` 不被 trust 挡，误当成交互也不会堵 | 有 `.claude/skills` / `commands` 就报 `trust.skill`；MCP 审批是另一扇门，`--yolo` 不够、要 `--pretrust` | 2026-08-29 |
| 只用 PATH/`which` 判断 agent 未装 | 官方安装常在 `~\.local\bin` 或 Codex junction，当前进程 PATH 未刷新 | 同时扫 PATH、`OMA_AGENT_PATH`、`OMA_*_BIN`、各家默认目录 | 2026-08-29 |
| 文档骨架曾去掉六态章节 | 当时用户说不需要 | ohmypwsh 2026-08-29 把六态升为 AGENTS 规则 10；研究与测试文档的结论断言必须标 | 2026-08-29 |
| 项目日记和方案混在 `docs\history\` | 初版对照时 ohmypwsh 尚未拆 diary | 方案只放 `docs\history\NNNN-*.md`；当天流水账放 `docs\diary\YYYY-MM-DD-*.md` | 2026-08-29 |
| `connect_or_start` 报 os error 5 / 拒绝访问 | 宿主在 Job Object 里，SDK 拒绝在 job 内起独立 daemon | 专用 pipe 上用 WMI `Win32_Process.Create` 在 job 外拉起 `libexec\rmux\rmux.exe --__internal-daemon`，再 `connect()`；不 kill-server，不把 wt 当默认 | 2026-08-29 |
| Windows `-S` 拒自定义 pipe 名 | CLI 只要 `\\.\pipe\rmux-...` 前缀 | 专用端点用 `\\.\pipe\rmux-omapoc-<pid>-<tag>`，仍非 Default 发现管道 | 2026-08-29 |
| `session.kill()` 偶发 `daemon closed the transport` | 末会话被杀后 daemon 先关连接再回包 | 关连接视为 kill 成功；先确认 keeper 仍在，再杀目标会话。禁止因此改杀 server | 2026-08-29 |

## 迭代规则

1. **当场追加**：每踩一个坑立即补/改一行。
2. **一行一事**：根因和正确处理各一句话。
3. **主题深挖分流**：反复踩或需分析时落 `docs\research\`，本表只留速查。
4. **过期更新**：找到更好做法时改「正确处理」，不删历史行。
