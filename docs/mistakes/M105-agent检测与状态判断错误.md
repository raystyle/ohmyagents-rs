# M105-agent检测与状态判断错误

> 关键词：PATH、which、二进制、idle、Quiet、CPU、terminal_state、实证滥用。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M012 | 只用 PATH/`which` 判断 agent 未装 | 官方安装常在 `~\.local\bin` 或 Codex junction，当前进程 PATH 未刷新 | 同时扫 PATH、`OMA_AGENT_PATH`、`OMA_*_BIN`、各家默认目录 | 2026-08-29 |
| M018 | hook 不报就把 Quiet 或 CPU 当 idle | Codex Stop / Claude 无 PermissionRequest 时文件或画面会骗人 | 兜底用等待原语 + `terminal_state`（ready/running/confirm/password）；Quiet 只给 Drive 同步 | 2026-08-29 |
| M019 | 把 YouMind 对 clum 的源码分析当成已核实 | 未打开 `tddh/clum` 就把路径与注释当实证 | 浅克隆目标 commit 再标 `[实证]`；注释与现码冲突以现码为准（`wait_exit` 5s vs facade 30s） | 2026-08-29 |
| M040 | 单路换单路（grok→kimi）spawn 报 "session daemon is gone"，任务没发出去 | 精确集合「先收后补」：收掉唯一活路 → 会话空 → rmux daemon 随末 session 自然退 → 补路时已无 daemon（grok 三轮复核警告「不相交可毁会话」当日实炸） | 收放类操作一律**先补后收**：计划路全部就位后才收多余路，任何时刻会话不空；复核警告过的事故模式要在下次同类改动前修，不「记档知悉」了事 | 2026-09-01 |
