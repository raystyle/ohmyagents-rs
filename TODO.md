# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

D06 agent 二进制下装部署五端全量收敛（吸收合并下载、安装、部署，配置域除外；方案见 `PLAN.md`，2026-09-05 立项）。

### 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 切片 1：oma --version 支持 | 待办 | clap version 挂 Cargo.toml 版本；SKILL.md / R002 / README 同步；解 ome 集成条件之一 | |
| 切片 2：lan-win 与 lan-linux 下发加盘点 | 待办 | oma 二进制 sha 对比下发（sync 脚本固化 .tools）；两端 agent 探测矩阵 | |
| 切片 3：五端幂等验收 | 待办 | 五端 oma agents 加 install（存量纳管 / 缺失补装），install 二连跑零变更；mac 加 WSL 复验 | |
| 切片 4：边界收口与跨仓 ISSUE | 待办 | AGENTS 边界段一行（agent 二进制下装部署归 oma，配置域除外）；ohmypwsh 退役配合 ISSUE；ohmyagents #2 #3 复核 | |
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
