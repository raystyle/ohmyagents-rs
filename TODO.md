# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

下目标待立项。D10 已达成归档 P0031。排队 D11 至 D13。

### 任务进度清单

无。

## 前目标清单

> D10 G005 存量字符清理（2026-09-07 归档 P0031）：SKIP_DIRS 外四类禁字清零，封闭清单删除。
> D08 doctor 检查面补形态（2026-09-07 归档 P0030）：Grok 状态栏 command 三态、JSON hook args 形态；用户实证栏正常。同日 D09：oma 不管种子。
> D07 oma 收窄配合迁册（2026-09-07 归档 P0029）：agents install/update deprecated、agents.toml 冻结历史锚、doctor 四类归 agents 域；当日热修 M044 至 M048（Codex 信任空表、Codex 状态栏 argv、Grok 字形、Grok hook ParserError、Grok 状态栏 os error 123）。
> 文档体系重构（D01 至 D05，2026-09-03）：全链已完成，PRD 引入（b948d67）、R002 扩容（61a3035）、AGENTS 重写（4147eff）、INDEX 收敛（03a6271）、TODO 清退（2e14434）、PLAN 与 GOAL 切目标（df362f5）、CHANGELOG 与 ROADMAP 补史（fd45180）、G002 CR 修复（8be0e4d）、R 系列六态整改（0bf7a5f）、豁免清单退出（2c0e67e）、标题修正（0e044bb）。
> D06 agent 二进制下装部署五端全量收敛（2026-09-05）：当日闭环（--version 三端、lan 两端下发盘点、五端幂等验收、AGENTS 边界与跨仓 issue）后同日方向反转（D07），五端成果转过渡态；切片与验收明细见 GOAL 历史 2026-09-05 与提交 ad11a79。

## 队列目标

> 历届目标残表已清退：过程与经验见 `docs\proven\` 对应方案与 `GOAL.md` 历史节；本清单只留当前目标与队列。排队项启动时先入 `PRD.md` 走澄清。

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| D11 状态栏工具链段扩展 zig/golang/cpp | 排队 | 已澄清；projKind 加 build.zig / go.mod / CMakeLists（或 meson），图标先 cmap 实证 |
| D12 根下 `.ohmyagents/t006/` 孤儿目录 | 排队 | 已澄清；孤儿 output 实为 t008 第二轮 review 误落，与 `tasks/t006/` 第一轮产物不同 |
| D13 mac `--version` 一致性 | 排队 | 已澄清；源码已有 clap version；门槛推 main 出 CI 资产。推远端仍待用户指示 |
