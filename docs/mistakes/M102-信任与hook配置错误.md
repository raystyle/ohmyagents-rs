# M102-信任与hook配置错误

> 关键词：信任框、trust、pretrust、init、yolo、codex projects、skill 门、MCP。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M002 | 项目级 Claude hook 弹信任框挡 launch | 命令变更触发 hook 信任 | spawn 前预写信任库；hook 注册仍只写项目目录 | 2026-08-25 |
| M009 | 在仓库 cwd 跑 `oma init --yolo` | init 默认写当前目录的 `.claude` / `.codex` / `.kimi-code` | POC 与演示用 `--project` 临时目录；`--pretrust` 才写家目录 | 2026-08-29 |
| M010 | 把项目 `.codex/config.toml` 的 `[projects]` 当成已信任 | Codex 先看用户 store，未信任则跳过项目层 | doctor 的 codex trust 只读 `~/.codex/config.toml` | 2026-08-29 |
| M011 | 把普通项目 skill 标成 n/a、只把 plugin 形态当门 | 官方 `allowed-tools` 不被 trust 挡，误当成交互也不会堵 | 2026-08-31 裁决反转：**只对 skills-dir plugin 形态报 `trust.skill`**（skill 目录带 `.claude-plugin/plugin.json`）；普通 `.claude/skills` 不报，归 `trust.project`。MCP 审批是另一扇门，`--yolo` 不够、要 `--pretrust` | 2026-08-29 |
| M042 | 共享项目目录跨环境（Windows 与 WSL）时 hook 注册弹 ENOENT、codex 信任哈希失效弹屏 | hook 命令嵌 `current_exe()` 绝对路径，WSL init 覆掉 Windows 字段（含 codex `commandWindows`），谁后跑 init 谁生效 | PATH bare 形态（claude/grok，粘性不降级）加 codex 字段所有权（各侧只写本侧字段）；状态栏 ps1 强制 UTF-8 输出。矩阵见 S024 | 2026-09-02 |
