# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

下目标待立项，走 `PRD.md` 追问链。D08 已达成归档 P0030；D09 边界同日交付。

### 任务进度清单

无。

## 前目标清单

> D08 doctor 检查面补形态（2026-09-07 归档 P0030）：Grok 状态栏 command 三态、JSON hook args 形态；用户实证栏正常。同日 D09：oma 不管种子。
> D07 oma 收窄配合迁册（2026-09-07 归档 P0029）：agents install/update deprecated、agents.toml 冻结历史锚、doctor 四类归 agents 域；当日热修 M044 至 M048（Codex 信任空表、Codex 状态栏 argv、Grok 字形、Grok hook ParserError、Grok 状态栏 os error 123）。
> 文档体系重构（D01 至 D05，2026-09-03）：全链已完成，PRD 引入（b948d67）、R002 扩容（61a3035）、AGENTS 重写（4147eff）、INDEX 收敛（03a6271）、TODO 清退（2e14434）、PLAN 与 GOAL 切目标（df362f5）、CHANGELOG 与 ROADMAP 补史（fd45180）、G002 CR 修复（8be0e4d）、R 系列六态整改（0bf7a5f）、豁免清单退出（2c0e67e）、标题修正（0e044bb）。
> D06 agent 二进制下装部署五端全量收敛（2026-09-05）：当日闭环（--version 三端、lan 两端下发盘点、五端幂等验收、AGENTS 边界与跨仓 issue）后同日方向反转（D07），五端成果转过渡态；切片与验收明细见 GOAL 历史 2026-09-05 与提交 ad11a79。

## 队列目标

> 历届目标残表已清退：过程与经验见 `docs\proven\` 对应方案与 `GOAL.md` 历史节；本清单只留当前目标与队列。排队项启动时先入 `PRD.md` 走澄清。

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| G005 存量字符清理 | 排队 | 3671 处四类禁字（DASH 2142、ARROW 255、EMOJI 892、FULLWIDTH 382，2026-09-02 量化）：FULLWIDTH 可机械替换，DASH/ARROW 按语义改写；清零后 mdcharlint.py 进验证链零容忍 |
| 状态栏工具链段扩展 zig/golang/cpp | 排队 | 用户定调 2026-09-02「以后」：projKind 探测加 build.zig / go.mod / CMakeLists（或 meson），图标先 cmap 实证；现役 rust/node+ts/python 三态 |
| 根下 `.ohmyagents/t006/` 孤儿目录收敛 | 排队 | 早期 task 布局遗留，与 tasks/t006/ 内容不同；动前先核对两轮产物归属（diary 09-03 待接） |
| mac --version 一致性（D06 余量转入） | 排队 | mac 现 oma 为旧版（无 --version）；待推 main 触发 CI 出新资产后走 oma self update 收口（推远端待用户指示） |
