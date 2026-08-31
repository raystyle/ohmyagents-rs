# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**——基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：各功能部件 POC 验证原型

> 对应 `GOAL.md`，方案 `docs\proven\P0005`，登记日 2026-08-29。

### 1. 闸门

`cargo run -- check` 退出 0（rmux pin 已装）。命令口径：`docs\references\R002-常用命令与管理流程-从项目init到会话cleanup.md`。

### 2. 已绿件与依据

| 件 | 状态 | 依据 |
| --- | --- | --- |
| `oma init --yolo` / `oma doctor` / `oma agents` / `oma hook` | 已绿 | `docs\references\R007-agent信任与无阻塞参考-四家配置与检测.md`；研究 `S007-yolo与无阻塞启动-配置落盘与无头分路`、`S006-信任阻塞门-四家种类与官方口径` |
| endpoint / session / layout / drive / dialogs | 已绿（Windows） | `docs\references\R006-rmux开发参考-连接会话布局与驱动.md`；研究 `S003-rmux-sdk最佳开发实践与验证poc` |
| paste（全 CLI `-L` label 三段式） | 已绿（Windows） | 同上参考一、三节；研究 S003 篇 paste 节（SDK `cmd()` Windows 不可用的实证） |

Linux/mac 委托后续环境仓库（`docs\proven\P0005` 口径）。

### 3. 接续件与依据

| 件 | 要做什么 | 依据 |
| --- | --- | --- |
| locate | pid 反查进程名；错位 throw | 参考「四、状态与等待」红线（不信 `pane_current_command`）；研究 S003 篇最佳实践 8 |
| stream | `output_stream` 收到字节 | 参考「四、状态与等待」观察条；研究 S003 篇 POC-7 |
| state | Quiet 不当 idle；hook 沉默走 `terminal_state` / `wait_for_text` | 研究 `S009-agent状态判断-通道与分层`（四层含 1b）、`S010-clum等待原语作为hook兜底状态` |
| init | 临时目录落 hook/skill，不改家目录 | 研究 `S008-项目级hook与skill`；参考 `agent信任与无阻塞参考` 四节 |
| negatives | 禁 C-c Codex、禁 kill-server 进主路径 | 参考「六、禁止清单」；研究 `S005-drive铁律与三段式粘贴`、`S004-win-rmux既有rmux研究吸收`（hardened-guard 八条） |

一次一件，按 `TODO.md` 顺序。

### 4. 每件验收

`cargo run --example poc-<名>` 退出 0；失败当场记 `INDEX.md（mistakes 节）`。验收通用口径见工作流细则第四节。

### 5. 边界

全表绿之前不写产品子命令 `spawn` / `send` / `cleanup`；四路真实 agent 不进本目标（桩进程 `pwsh -NoProfile -Command`）。[依据: `docs\proven\P0005` 非目标节]

## 完成的定义（本目标验收）

- TODO 表除「四路真实 agent」外全部已完成或明确跳过（跳过须写 MISTAKES 原因）
- 每个 POC 不调用 `kill-server`
- `rumdl check .` 尽量零告警；0005 与全量清单已登记
- 研究与测试文档的关键结论标六态（AGENTS 写研究与测试文档规则）
