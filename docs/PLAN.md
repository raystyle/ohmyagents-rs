# PLAN：当前目标规划指导

> 角色：**当前目标怎么推进**（步骤/标准/验收），随目标变化更新，不存历史目标。
> 与 `docs\TODO.md` 分工：todo = 做到哪；本文件 = 怎么做。
> 目标完成后把过程与经验回填 `docs\history\` 对应方案，新目标另起；`docs\diary\` 只记当天流水账。

## 当前目标实施计划

> 当前目标「各功能部件需求的 POC 验证原型」（对应 `GOAL.md`，方案 0005，登记日 2026-08-29），任务项见 `docs\TODO.md`。

1. 闸门：`cargo run -- check` 退出 0（rmux pin 已装）。
2. 用户点名先做项目级 yolo 与提示阻塞诊断：`oma init --yolo` / `oma doctor` / `examples/poc-yolo-doctor.rs` 已绿。本机已装 agent 探测：`oma agents`。Windows 最小 pane 原型已绿：`poc-endpoint` / `poc-session` / `poc-layout` / `poc-drive` / `poc-dialogs` / `poc-paste`。paste 走全 CLI `-L` label（SDK cmd() 在 Windows 因 `-S` 注入必败，见 MISTAKES）。Linux/mac 不在本机复跑，后续委托到对应环境仓库。状态兜底改为等待原语 + `terminal_state`（研究 clum，不引入其运行时）。其余 locate / stream / init / negatives 仍按 TODO 一次一件。
3. 每件验收：`cargo run --example poc-<名>` 退出 0；失败当场记 `docs\MISTAKES.md`。
4. 全表绿之前，不写产品子命令 `spawn` / `send` / `cleanup`。`init --yolo` 与 `doctor` 已作为本切片的薄 CLI 落地；hook/skill 仍走 `poc-init`。
5. 四路真实 agent 不进本目标；桩进程用 `pwsh -NoProfile -Command`。

## 一、从待办到完成的五步

1. **登记**：新想法进 `docs\TODO.md`（优先级 + 一句话 + 登记日）
2. **立项**：开工前先在 `docs\history\NNNN-短名.md` 写方案（模板 `docs\history\template.md`）
3. **执行**：按方案实施步骤推进
4. **验收**：按方案验收标准核对；POC 看退出码；文档看清单与断链
5. **归档**：完成后在 `docs\diary\YYYY-MM-DD-*.md` 补当天流水账；方案回填「实施过程与经验」；版本/定位级成果另记 CHANGELOG

## 二、拆步骤的标准

- **一步一件事**：每个 example 只证明一个部件
- **先方案后代码**：部件边界以 0005 为准；API 细节以 `rmux-sdk最佳开发实践与验证poc.md` 为准
- **每步带验收**：`cargo run --example` 退出码；会话结束后 `list-sessions` 无 POC 名
- **依赖明确**：无 rmux（check 失败）不准开 pane POC

## 三、优先级与取舍

| 优先级 | 含义 | 处理 |
| --- | --- | --- |
| 高 | 已完成：check、项目级 yolo、doctor（用户点名提前） | 归档验收，不再重做 |
| 高 | 已完成（Windows）：endpoint / session / layout / drive / dialogs / cleanup=kill-session | Linux/mac 委托 |
| 高 | 已完成（Windows）：paste（全 CLI `-L`） | locate / stream 接续 |
| 中 | state（Quiet 不当 idle） | 高项绿了再做 |
| 低 | init hook/skill / negatives、网页 HTTP | 本目标末尾；HTTP 不进本目标 |

原则：**一次只推进一个任务目标**（见 `GOAL.md` 锚点）。

## 四、完成的定义

- TODO 表除「四路真实 agent」外全部已完成或明确跳过（跳过须写 MISTAKES 原因）
- 每个 POC 不调用 `kill-server`
- `rumdl check .` 尽量零告警；0005 与全量清单已登记
- 研究与测试文档的关键结论标六态（AGENTS 操作规则 10）

## 五、经验指南

> 效率最高的做法，非规则。

- SDK 用 `rmux-sdk = "=0.10.0"`，不要依赖 `rmux-client` / `rmux-server`。
- spawn 用 CreateOnly；端点用显式 pipe/socket。
- Drive：文本和 Enter 分发；paste 不自包 bracketed paste。
- 对照：evo-harness drive；win-rmux 坑表；官方 example 只借 API 形状。
