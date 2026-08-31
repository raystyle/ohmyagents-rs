# M105-agent检测与状态判断错误

> 关键词：PATH、which、二进制、idle、Quiet、CPU、terminal_state、实证滥用。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M012 | 只用 PATH/`which` 判断 agent 未装 | 官方安装常在 `~\.local\bin` 或 Codex junction，当前进程 PATH 未刷新 | 同时扫 PATH、`OMA_AGENT_PATH`、`OMA_*_BIN`、各家默认目录 | 2026-08-29 |
| M018 | hook 不报就把 Quiet 或 CPU 当 idle | Codex Stop / Claude 无 PermissionRequest 时文件或画面会骗人 | 兜底用等待原语 + `terminal_state`（ready/running/confirm/password）；Quiet 只给 Drive 同步 | 2026-08-29 |
| M019 | 把 YouMind 对 clum 的源码分析当成已核实 | 未打开 `tddh/clum` 就把路径与注释当实证 | 浅克隆目标 commit 再标 `[实证]`；注释与现码冲突以现码为准（`wait_exit` 5s vs facade 30s） | 2026-08-29 |
