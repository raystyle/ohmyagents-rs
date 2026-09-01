# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**——基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：P0012 平台接管收口

> 三阶段当日齐（2026-09-01）：WSL Linux 第一棒（构建/基线/daemon/分类器）→ mac 接管收口（四项）→ 回 WSL 补尾（安装/真身四路）。三平台真机全链绿，无剩余切片；待用户定向归档（proven 回填后重写本文件）。

### 1. 剩余切片

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| mac 环境搭建与构建 | 已完成 | 基线 79+10 / 82+10 全绿；build --features server,mcp 过（2026-09-01） |
| rmux mac 资产验收 | 已完成 | `oma check` arm64 全绿；daemon 拉起 Unix 分支 stub 全链复验绿（2026-09-01） |
| 四家 agent mac 安装 | 已完成 | 检测与 `--force` darwin 四家全链绿；grok 双 CDN 补 macos-aarch64 pin（47b1ddd）（2026-09-01） |
| 真身四路 + settle 真机（mac） | 已完成 | 信任屏措辞漂移三路修复加 codex 数字菜单形态（7211d41）；settle 窗口四路全收、真任务闭环、doctor 全绿（2026-09-01） |

### 2. 挂起

> 已清空：WSL Linux 补尾棒 2026-09-01 完成。

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| 四家 agent Linux 安装 | 已完成 | `--force` 真下载四家全链绿（claude 2.1.251 / codex 0.151.0 嵌套 bin / grok 1.0.13 CDN 裸二进制 / kimi 0.39.1 zip），探针全过 |
| 真身四路 + settle 真机（Linux） | 已完成 | settle Linux 实拍命中（codex 数字菜单、kimi don't trust）；grok 家目录阻塞 `oma init --pretrust` 清零；`doctor.blocked=false`；`oma task` t026 产物精确；hook 流通；cleanup 零残留 |

### 3. 接续口径

- 立项：新目标按 G003 五步（登记、研究、方案、实施、归档）走，先搜 `INDEX.md` 防重复造规则。
- 命令面：已落地 17 命令 + REPL（清单见 AGENTS 意图路由与 README）；`oma serve start/stop/status` 守护化，看板默认 spectator 只读（可写镜像用 `oma web`）。
- Unix 已验路径（WSL Linux 与 mac 双验）：daemon 拉起 `boot_new_session`、分类器 Unix 提示符、pid 守卫、serve 进程组隔离；stub 全链绿；mac 另验四家 darwin 安装与真身四路 + settle（2026-09-01）。
- 门禁：`rumdl check .` + `md-ref-scan.py` + `md-heading-scan.py` 裸跑；提交前含 `cargo test`（隔离 target）；提交精确 add（M036）。

## 完成的定义

> 本目标验收口径。

- mac 真机：达成（2026-09-01）——构建与测试基线全绿、rmux 资产验收、四家 agent 安装、真身四路 + settle 全链绿（含真任务与 hook 流）。
- WSL Linux：达成（2026-09-01）——第一棒（构建/基线/daemon/分类器/stub 全链）加补尾棒（四家 `--force` 安装探针全绿、真身四路 + settle、`oma task` 真任务产物精确、doctor 零阻塞、cleanup 零残留）。
