# M102-信任与hook配置错误

> 关键词：信任框、trust、pretrust、init、yolo、codex projects、skill 门、MCP。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M002 | 项目级 Claude hook 弹信任框挡 launch | 命令变更触发 hook 信任 | spawn 前预写信任库；hook 注册仍只写项目目录 | 2026-08-25 |
| M009 | 在仓库 cwd 跑 `oma init --yolo` | init 默认写当前目录的 `.claude` / `.codex` / `.kimi-code` | POC 与演示用 `--project` 临时目录；`--pretrust` 才写家目录 | 2026-08-29 |
| M010 | 把项目 `.codex/config.toml` 的 `[projects]` 当成已信任 | Codex 先看用户 store，未信任则跳过项目层 | doctor 的 codex trust 只读 `~/.codex/config.toml` | 2026-08-29 |
| M011 | 把普通项目 skill 标成 n/a、只把 plugin 形态当门 | 官方 `allowed-tools` 不被 trust 挡，误当成交互也不会堵 | 2026-08-31 裁决反转：**只对 skills-dir plugin 形态报 `trust.skill`**（skill 目录带 `.claude-plugin/plugin.json`）；普通 `.claude/skills` 不报，归 `trust.project`。MCP 审批是另一扇门，`--yolo` 不够、要 `--pretrust` | 2026-08-29 |
| M042 | 共享项目目录跨环境（Windows 与 WSL）时 hook 注册弹 ENOENT、codex 信任哈希失效弹屏 | hook 命令嵌 `current_exe()` 绝对路径，WSL init 覆掉 Windows 字段（含 codex `commandWindows`），谁后跑 init 谁生效 | PATH bare 形态（claude/grok，粘性不降级）加 codex 字段所有权（各侧只写本侧字段）；状态栏 ps1 强制 UTF-8 输出。矩阵见 S024 | 2026-09-02 |
| M044 | 纯 Windows 项目 Codex 弹 Hooks need review，`[hooks.state]` 空表；doctor 仍报 trust.hooks ok | P0027 字段所有权后 Windows 只写 `commandWindows`，`codex_trust_entries` 仍只认 `command` 于是跳过播种；doctor 把用户 store 里 `~/.codex/hooks.json` 的 leftover hash OR 进来掩盖空表 | 播种与 merge 同口径：`command` 或 `commandWindows` 任一是 oma 即 ours；hash 按本侧字段取值、缺则回退异侧；doctor 的项目 hook 信任只读项目 `config.toml`，不拿用户 store 顶 | 2026-09-07 |
| M045 | Codex TUI 状态栏空或只剩默认项；doctor 因配置含 `oma-statusline` 仍报 ok | `oma agents statusline` 把 `[tui].status_line` 写成 command argv（`command`/`pwsh`/`-File`/脚本路径）；Codex 只收内置项 ID，无法解析的字符串静默跳过（ohmypwsh S016），整栏被跳空；S025 误写成「可外部命令」 | 写入面改内置项 ID（`run-state` 等，对齐 S016 推荐清单）加 `status_line_use_colors`；doctor 把 command-argv 形态标 warn，内置项才 ok；S025 矩阵行订正 | 2026-09-07 |
| M046 | Grok TUI 状态栏图标显示成替换符，git 超前旗标一串虚线箭头 | 共享 ps1 用 Nerd 私用区字形（shell/机器人/包/工具链）加 U+21E1 重复箭；Grok pager 字体无这些码位，command 形态本身合法 | Grok 走 ASCII 安全路径（无 PUA、ahead/behind 写成 `+N`/`-N`、跳过工具链子进程）；claude/kimi 仍用 Nerd。配置键不变，重放脚本即生效 | 2026-09-07 |
