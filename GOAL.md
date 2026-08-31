# GOAL：任务目标管理

> 角色：**工作任务管理**，四个部分——**起点**、**锚点**、**进程**、**历史**。随工作实时更新。
> 与其它文档分工：`ROADMAP.md`=阶段路线；`CHANGELOG.md`=版本成果；`docs\diary\YYYY-MM-DD-*.md`=项目日记（当天做了什么）；`docs\proven\NNNN-*.md`=方案与过程经验；`TODO.md`=进度清单；`PLAN.md`=实施指导。

## 起点

> 当前目标的起点：何时发起、为什么发起、要解决什么问题。

- **日期**：2026-08-31。
- **起点**：P0005 十二部件 POC 在 Windows 全表绿（12 个 example 退出 0），共用层就位未组装。按其实施步骤 4 开产品命令最小闭环。

## 锚点

> 当前锚定的目标 + 推进时间线。

- **锚定的目标**：`oma spawn` / `status` / `send` / `cleanup` 最小产品闭环（方案 P0006），含 `tests/cli.rs` 集成起步。

### 推进时间线（倒序）

| 日期 | 进展 |
| --- | --- |
| 2026-08-31 | P0006 达成：spawn/status/send/cleanup 全链路绿（slug 会话 + pane 清单 + 四层 status + 守卫链 send + session 级 cleanup）；tests/cli.rs 5 例起步，cargo test 38 过 |
| 2026-08-31 | POC negatives 绿，P0005 Windows 范围全表绿（12 个 example）：C-c Codex 守卫 throw、只杀本 session、src 无 daemon-wide kill |
| 2026-08-31 | POC init 绿（Windows）：按 S015 一手矩阵部署四家 hook/skill；幂等合并保留外条目；家目录指纹零变化 |
| 2026-08-31 | S015 四家 hook 注册一手形态：Claude 官方 hooks reference + codex/grok-build/kimi-code 三仓源码；订正 Claude PermissionRequest 旧口径、关闭 Kimi 项目级悬案 |
| 2026-08-31 | POC state 绿（Windows）：最小 detect_terminal_state 分类器落地；Quiet 静止不映射 idle；confirm/password 阻塞可判可点掉 |
| 2026-08-31 | POC stream 绿（Windows）：output_stream Oldest 回放 backlog、Now 只收新字节；11ms 见 marker；marker 子串坑记 M027 |
| 2026-08-31 | POC locate 绿（Windows）：pid 经 CIM 批量反查进程名；死 pid 与错位守卫 throw，置于 send_key 前 |
| 2026-08-31 | 研究体系建设：测试三源与编码经验落地为规则；知识库四目录分职加全库 P/S/R/G/M 编号；INDEX 唯一索引与 rg/mq/ast-grep 搜索链 |
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

- 当前目标（收尾中）：产品命令最小闭环。四命令全链路验收过（2026-08-31）；REPL、HTTP 网页、`oma run`、多行粘贴留待后续方案。

## 历史

> 所有已完成目标的轨迹，按日期倒序。

| 日期 | 目标 | 结果 |
| --- | --- | --- |
| 2026-08-31 | 产品命令最小闭环（P0006） | 达成：spawn/status/send/cleanup 全链路绿；tests/cli.rs 起步；cargo test 38 过 |
| 2026-08-29 | 各功能部件 POC 验证原型（P0005） | 达成（Windows 范围）：12 个 example 全表绿；locate/stream/state/init/negatives 五件与 paste 同日收官；Linux/mac 委托后续仓库 |
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
