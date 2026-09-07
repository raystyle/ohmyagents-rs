# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

D07 oma 收窄配合迁册（2026-09-05 立项、2026-09-07 达成归档 P0029；方案见 `PLAN.md`，需求见 `PRD.md` D07）。下目标待立项，走 `PRD.md` 追问链。

### 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 迁册批：agents install/update deprecated（D07） | 已完成 | 入口 stderr 打 `oma.deprecated` 指向 `ome install`（不删命令、stdout kv 与 json 面 R011 不动；update 注明通道语义由 ome 裁决）；clap 帮助与 COMMAND_MAP 同步；集成测试两条；R002 两行 deprecated 注、AGENTS 路由两行、INDEX 两行。原门槛「等 ome 仓 D07 切片 1 与 3 落地」，2026-09-07 用户裁定不等切片 3 先清自身面 | 2026-09-07 |
| `catalog\agents.toml` 头注记数据权威转 ome | 已完成 | 文件头注记冻结历史锚（数据权威 ome `catalog\tools.toml` agent 四节，pin 与 sha 不再随上游滚动）；INDEX catalog 行同步 | 2026-09-07 |
| doctor 四类检查归 agents 域 | 已完成 | R002 doctor 行重排（agents 域四类：登录态 / hook 形态 / 状态栏 / 会话健康；二进制在位与版本、token 诊断归 ome doctor）；AGENTS doctor 路由行同步；代码不动（ohmyagents#5「维持」口径，binary 在位探查保留作 spawn 前置） | 2026-09-07 |
| 跨仓交底与回填 | 已完成 | ohmyenv-rs 发 issue：本机 ome 部署位与 catalog 停 2026-09-02 旧版、`ome install` 报未知工具，请部署含 D07 新版并实证四家幂等跳过；ohmyagents#5 回填迁册批落地 | 2026-09-07 |
| 根 SKILL.md 命令图对账（队列转入顺带收口） | 已完成 | COMMAND_MAP 改 deprecated 文案并重跑 `oma init`：四端 SKILL 全部再生（marker 在位即覆写为生成版，与 COMMAND_MAP 同构，2026-09-03 登记的落后欠账随再生消除） | 2026-09-07 |

## 前目标清单

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
