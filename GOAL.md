# GOAL：任务目标管理

> 角色：**工作任务管理**，四个部分——**起点**、**锚点**、**进程**、**历史**。随工作实时更新。
> 与其它文档分工：`ROADMAP.md`=阶段路线；`CHANGELOG.md`=版本成果；`docs\diary\YYYY-MM-DD-*.md`=项目日记（当天做了什么）；`docs\proven\NNNN-*.md`=方案与过程经验；`TODO.md`=进度清单；`PLAN.md`=实施指导。

## 起点

> 当前目标的起点：何时发起、为什么发起、要解决什么问题。

- **日期**：2026-08-31。
- **起点**：P0006/P0007 把 spawn/status/send/cleanup/init 全部产品化，状态判断与守卫链就位；缺编排器本义——一条任务分派多路、一路 blocked 不堵其它路（S009 层 3 任务映射一并落地）。

## 锚点

> 当前锚定的目标 + 推进时间线。

- **锚定的目标**：oma run 委派与任务映射（方案 P0008）。

### 推进时间线

> 倒序：最新进展在最上。

| 日期 | 进展 |
| --- | --- |
| 2026-08-31 | P0009 达成：真四路拉通——claude 路全通（hook 事件流实时迁移、真任务执行）；spawn cwd 缺陷修复（M031）；三路保守拦截符合设计 |
| 2026-08-31 | P0008 达成：oma run 状态门分派（一路忙/blocked 跳过不堵其它路）加层 3 任务文件；cargo test 41 过。附 init 接 deploy 层收尾 |
| 2026-08-31 | P0007 达成：label 端点融合（CLI 起 daemon、`#{socket_path}` 桥 SDK）、send 多行三段式粘贴（中文验收）、stale pipe 自愈；boot 前缀坑记 M029 |
| 2026-08-31 | label-bridge 实证绿：CLI 起 label daemon 后 `#{socket_path}` 桥出实际 pipe，SDK 直连同一 daemon（poc-label-bridge）；P0007 立项 |
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

- 当前目标（收尾中）：真四路拉通验收。claude 路全通、cwd 缺陷已修（2026-08-31）；剩余候选见 TODO（REPL/HTTP 观察面、三路信任流打通）。

## 历史

> 所有已完成目标的轨迹，按日期倒序。

| 日期 | 目标 | 结果 |
| --- | --- | --- |
| 2026-08-31 | 真四路拉通验收（P0009） | 达成：claude 路全通；spawn cwd 缺陷修复；三路保守拦截符合设计 |
| 2026-08-31 | oma run 委派与任务映射（P0008） | 达成：状态门分派加层 3 任务文件；stub 验收过 |
| 2026-08-31 | send 多行粘贴与 label 端点融合（P0007） | 达成：一个 daemon 双传输面；多行中文三段式；自愈与引导 |
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
