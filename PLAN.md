# PLAN：当前目标实施计划

> 角色：**当前目标怎么推进**（步骤与验收），随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流方法论见 `docs\guide\工作流标准细则-从登记到归档五步.md`。
> 目标完成后过程与经验回填 `docs\proven\` 对应方案，新目标另起。

## 当前目标：各功能部件 POC 验证原型

> 对应 `GOAL.md`，方案 `docs\proven\0005`，登记日 2026-08-29；任务项见 `TODO.md`。

1. 闸门：`cargo run -- check` 退出 0（rmux pin 已装）。
2. 已绿：`oma init --yolo` / `oma doctor` / `oma agents` / `oma hook`；Windows pane 原型 `poc-endpoint` / `poc-session` / `poc-layout` / `poc-drive` / `poc-dialogs` / `poc-paste`（paste 走全 CLI `-L` label，SDK `cmd()` 在 Windows 因 `-S` 注入必败，见 MISTAKES）。Linux/mac 委托后续环境仓库。
3. 接续件：locate / stream / state / init / negatives 按 `TODO.md` 一次一件；状态兜底用等待原语加 `terminal_state`（clum 研究，不引入其运行时）。
4. 每件验收：`cargo run --example poc-<名>` 退出 0；失败当场记 `docs\mistakes\MISTAKES.md`。
5. 全表绿之前不写产品子命令 `spawn` / `send` / `cleanup`；四路真实 agent 不进本目标（桩进程 `pwsh -NoProfile -Command`）。

## 完成的定义（本目标验收）

- TODO 表除「四路真实 agent」外全部已完成或明确跳过（跳过须写 MISTAKES 原因）
- 每个 POC 不调用 `kill-server`
- `rumdl check .` 尽量零告警；0005 与全量清单已登记
- 研究与测试文档的关键结论标六态（AGENTS 写研究与测试文档规则）

## 开发口径

> 写码时的具体做法见 `docs\references\rmux开发参考-连接会话布局与驱动.md` 与 `docs\references\agent信任与无阻塞参考-四家配置与检测.md`；本节只留目标级提示。

- SDK 钉 `rmux-sdk = "=0.10.0"`；对照 evo-harness drive、win-rmux 坑表；官方 example 只借 API 形状。
