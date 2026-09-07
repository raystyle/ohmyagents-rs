# P0029：D07 迁册批，agents 下装归 ome

> 2026-09-07 当日闭环。缘起：D06（2026-09-05）oma 当日收编 agent 二进制五端闭环后，同日用户追问链三轮六裁**方向反转**：「oma 以后只管配置 agent 和 hook 和编排，不管 agent 的升级和安装」。agent 二进制下装部署回归 ohmyenv-rs（ome）承载，oma 做迁册配合。执行日用户再裁定：不等 ome 仓 D07 切片 3，先清自身面，ome 侧以 issue 交底。请求清单见 ohmyagents#5。

## 方案

### 1. install 与 update deprecated 提示

> src\main.rs。

- **入口提示**：`cmd_agents_install` / `cmd_agents_update` 函数头 stderr 打 `oma.deprecated=... moved to ome (D07); use: ome install <agent>; this command still works`。命令不删、行为保留（幂等安装与取证升级链路原样），后续新版 agent 的下装走 ome。
- **提示走 stderr 的理由**：stdout 是 kv / json 输出面（R011 冻结契约），`oma.deprecated` 属带外告警；与 `spawn.alert`、`run.skipped` 同通道。update 提示额外注明升级通道语义由 ome 裁决（ohmyagents#2 余项）。
- **帮助面同步**：clap 两子命令 doc comment 加 deprecated 注（`--help` 可见）；COMMAND_MAP 的 install 行改 deprecated 文案。
- **测试**：tests\cli.rs 两条集成测试，未知名（`nope`）加临时 `--root` 触发触网前快败，断言 stderr 含 `oma.deprecated` 与 `ome install` 且退出非 0（R004 冒烟断退出码与标记行）。

### 2. catalog agents.toml 冻结历史锚

> catalog\agents.toml。

- 文件头注记：数据权威是 ome `catalog\tools.toml` agent 四节（pin 与 sha 已迁，ohmyenv-rs 461a178），本文件冻结为历史锚、不再随上游滚动；既有取证注释保留作迁册时点版本基线。文件仍被 oma 加载（catalog.rs 信任锚与用户本地层机制不动），只是停止演进。

### 3. doctor 四类检查归 agents 域

> docs\references\R002、AGENTS.md。文档层重排，代码不动。

- 登录态 / hook 形态 / 状态栏 / 会话健康四类检查在 R002 与 AGENTS 归类为 agents 域（oma 本域）；二进制在位与版本、token 可用性诊断归 ome doctor agent 层（ohmyagents#5「维持」口径）。
- oma doctor 代码不动：binary 在位探查保留（spawn 前置依赖），不与 ome doctor 的版本与 token 五字段重复。

### 4. 命令面与四原语同步

- R002：doctor 行重排归类、install 与 update 两行加 deprecated 注（原行为细则保留作兼容期参考）。
- AGENTS：边界段 D06 行改写为 D07 分工（ome 承载下装、oma 收窄配置 hook 编排）、四仓分工行补 D07 修正、doctor 与 install / update 三条路由行标注、`~/.ohmyagents` 行注记。
- INDEX：`catalog\agents.toml` 与 `src\install.rs` 两行同步。
- 根 SKILL：重跑 `oma init` 四端再生（marker 在位即覆写，与 COMMAND_MAP 同构；2026-09-03 登记的根 SKILL 对账欠账随再生消除）。
- 四原语：PRD D07 流转已交付、GOAL 起点锚点时间线历史切 D07（修正 D07 立项时起点锚点滞留 D06 的欠账）、PLAN 重写为 D07、TODO 起新清单。

## 验收

- 新增两条集成测试绿；全量 cargo test、fmt / clippy 零告警；md 三件套与 rumdl 触碰文件过。
- `oma agents install nope --root <tmp>` 实跑：stderr 先出 `oma.deprecated` 再报未知 agent，退出非 0，stdout kv 面无提示混入。
- 跨仓：ohmyenv-rs 发 issue 交底（本机 ome 部署位与 catalog 停 2026-09-02 旧版，`ome install claude` 报未知工具，请部署含 D07 新版并实证四家幂等跳过）；ohmyagents#5 回填。

## 钩子

- 命令面：`oma agents install` / `oma agents update`（R002 两行 deprecated 注）、`oma doctor`（R002 agents 域四类归类）。
- 关联：ohmyagents#5（迁册请求）、ohmyagents#2（升级通道语义余项）、ohmypwsh#9（catalog agent 节冻结退役，承接方 ome）；ohmyenv-rs D07 切片 3 与 4 其仓自跟。
- 遗留：oma deprecated 提示指向的 `ome install` 在本机尚不可用（部署滞后，issue 交底）；oma 五端部署位的 deprecated 版本更新随下次下发节奏。
