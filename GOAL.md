# GOAL：任务目标管理

> 角色：**工作任务管理**，四个部分——**起点**、**锚点**、**进程**、**历史**。随工作实时更新。
> 与其它文档分工：`ROADMAP.md`=阶段路线；`CHANGELOG.md`=版本成果；`docs\diary\YYYY-MM-DD-*.md`=项目日记（当天做了什么）；`docs\proven\NNNN-*.md`=方案与过程经验；`TODO.md`=进度清单；`PLAN.md`=实施指导。

## 起点

> 当前目标的起点：何时发起、为什么发起、要解决什么问题。

- **日期**：2026-08-29。
- **起点**：用户定调先做各功能部件需求的 POC 验证原型。文档骨架、定位、`oma check` 已有；产品命令尚未写。要在叠完整 CLI 之前，按部件核 drive / 布局 / 清理等已知坑。

## 锚点

> 当前锚定的目标 + 推进时间线。

- **锚定的目标**：为 Oh My Agents 各功能部件做出可跑的 POC 验证原型（方案 P0005）。

### 推进时间线（倒序）

| 日期 | 进展 |
| --- | --- |
| 2026-08-31 | POC paste 绿（Windows）：全 CLI `-L` label 三段式；实证 SDK cmd() 在 Windows 不可用、`-S` 无条件拒绝 |
| 2026-08-29 | 核实 tddh/clum `68a90e4` 源码：等待族与 `terminal_state` 属实；订正 wait_exit 5s 注释 |
| 2026-08-29 | 研究 clum 等待原语：hook 沉默时用 `terminal_state` / `wait_for_text` 兜底，不引入 clum 运行时 |
| 2026-08-29 | 对齐 ohmypwsh 文档结构：方案在 `docs\proven\`，日记拆到 `docs\diary\` |
| 2026-08-29 | 对齐 ohmypwsh 六态文档规则：AGENTS 六态规则 + `guide.md` + 研究关键结论标记 |
| 2026-08-29 | 立项方案 P0005；GOAL/TODO/PLAN 切到本目标；check 部件已验收 |
| 2026-08-29 | Windows 最小 pane POC 已绿：endpoint / session / layout / drive / dialogs；Linux/mac 委托后续仓库 |
| 2026-08-29 | 用户点名先做项目级 yolo 与提示阻塞诊断；`oma init --yolo` / `oma doctor` / `poc-yolo-doctor` 已绿 |

## 进程

> 当前目标的进程：只记录当前这一个目标的进行状态。

- 当前目标：各功能部件 POC 验证原型。check、yolo、doctor、Windows pane 最小集（endpoint/session/layout/drive/dialogs/paste）已完成；locate/stream/init/negatives 未开；Linux/mac pane 跑测委托后续仓库。

## 历史

> 所有已完成目标的轨迹，按日期倒序。

| 日期 | 目标 | 结果 |
| --- | --- | --- |
| 2026-08-29 | 对照 ohmypwsh 建立项目结构与项目文档 | 达成：四段 AGENTS、三原语大写、定位 0004、`oma check` 装上 rmux 0.10.0 |
| 2026-08-29 | 先写研究、不写 Cargo | 达成：对照博客与吸收报告；后因 check 开了 `src\` |
| 2026-08-29 | 删除 project skill 对本仓的元文件 | 达成：去掉误跟的 AGENTS/CLAUDE/CHANGELOG/ROADMAP/docs 地图 |

## 维护规则

- **起点**：开工时写一句「何时发起 + 为什么发起」。
- **锚点**：每完成一个节点补一行（日期 + 进展）。
- **进程**：只记当前目标；达成后整条移入「历史」。
- **历史**：日期 + 目标 + 结果，倒序。
- **一目标一路径**：起点、锚点、进程、历史同属一个目标轨迹。
- **日记与方案**：当天流水账进 `docs\diary\`；方案与过程经验进 `docs\proven\`。
