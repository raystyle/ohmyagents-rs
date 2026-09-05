# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

D06 agent 二进制下装部署五端全量收敛（吸收合并下载、安装、部署，配置域除外；方案见 `PLAN.md`，2026-09-05 立项）。

### 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 切片 1：oma --version 支持 | 已完成 | 源码 clap version 属性已在（部署位旧构建遮蔽）；CI 同口径 --features server,mcp 三处构建实测 oma 0.1.0（本机、WSL、lan-linux）；部署位更新（本机与 lan-win 的 D:\ohmyenv\cargo\bin、lan-linux 的 ~/.local/bin）；R002 补 --version 全局参数登记 | 2026-09-05 |
| 切片 2：lan-win 与 lan-linux 下发加盘点 | 已完成 | lan-win oma.exe scp 下发（4/4 installed source=path，codex 为 EnvRoot 越界物 D:\ohmyenv\codex\bin）；lan-linux linux-gnu 构建经 WSL（target-linux 持久目录）下发 ~/.local/bin；盘点修正：lan-linux 三家缺（claude/codex/grok）、kimi 在 default 位 0.38.0 | 2026-09-05 |
| 切片 3：五端幂等验收 | 已完成 | 本机 4/4 全 skipped；WSL 4/4 全 skipped；lan-win 4/4 全 skipped；lan-linux 首装三家（各带版本探针）加二连跑全 skipped；mac 4/4 全 skipped（旧版 oma，agents 子命令语义未变） | 2026-09-05 |
| 切片 4：边界收口与跨仓 ISSUE | 已完成 | AGENTS 边界段加 agent 二进制域归本仓行（幂等检测安装唯一权威通道、存量原地纳管、配置域除外）；ohmypwsh#9 发出（catalog.psd1 四家 agent 节冻结退役配合）；ohmyagents #2 回填进展（--version 已解、装位三裁覆盖、发布通道待裁） | 2026-09-05 |
| 余量：mac --version 一致性 | 待办 | mac 现 oma 为旧版（无 --version）；待推 main 触发 CI 出新资产后走 oma self update 收口（推远端待用户指示） | |
| 立项登记（D06） | 已完成 | PRD D06 第 1 轮三裁、GOAL 起点锚点切换、PLAN 四切片、TODO 建行；基线盘点（本机 4/4 幂等纳管实证、--version 缺口实证、五端缺口 lan-win 加 lan-linux） | 2026-09-05 |

## 前目标清单

> 文档体系重构（D01 至 D05，2026-09-03）：全链已完成，PRD 引入（b948d67）、R002 扩容（61a3035）、AGENTS 重写（4147eff）、INDEX 收敛（03a6271）、TODO 清退（2e14434）、PLAN 与 GOAL 切目标（df362f5）、CHANGELOG 与 ROADMAP 补史（fd45180）、G002 CR 修复（8be704d）、R 系列六态整改（0bf7a5f）、豁免清单退出（2c0e67e）、标题修正（0e044bb）。

## 队列目标

> 历届目标残表已清退：过程与经验见 `docs\proven\` 对应方案与 `GOAL.md` 历史节；本清单只留当前目标与队列。排队项启动时先入 `PRD.md` 走澄清。

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| G005 存量字符清理 | 排队 | 3671 处四类禁字（DASH 2142、ARROW 255、EMOJI 892、FULLWIDTH 382，2026-09-02 量化）：FULLWIDTH 可机械替换，DASH/ARROW 按语义改写；清零后 mdcharlint.py 进验证链零容忍 |
| 状态栏工具链段扩展 zig/golang/cpp | 排队 | 用户定调 2026-09-02「以后」：projKind 探测加 build.zig / go.mod / CMakeLists（或 meson），图标先 cmap 实证；现役 rust/node+ts/python 三态 |
| 根下 `.ohmyagents/t006/` 孤儿目录收敛 | 排队 | 早期 task 布局遗留，与 tasks/t006/ 内容不同；动前先核对两轮产物归属（diary 09-03 待接） |
| 根 SKILL.md 命令图对账 | 排队 | 手写源落后 `src/deploy.rs` COMMAND_MAP 约 2 条新命令（2026-09-03 重构核对发现）；按 R002 四节命令同步链补齐 |
