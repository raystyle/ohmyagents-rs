# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**——基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：P0012 平台接管 mac 阶段

> 用户定调 2026-09-01：先「准备去 wsl linux 下开发」→ 第一棒完成后「目前只到 wsl linux 就可以了」→「准备让 mac 接管开发」。WSL Linux 第一棒收口，剩余两项挂起待回 WSL 环境续做；mac 阶段启动。交接读本 `docs\references\R010-Windows到Linux交接清单.md`（三节欠账清单对 mac 同样适用，资产形态换 darwin）。

### 1. 剩余切片

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| mac 环境搭建与构建 | 待做 | clone 后 `cargo test` 两口味基线 + `cargo build --features server,mcp`（Linux 第一棒同流程） |
| rmux mac 资产验收 | 待做 | `oma check` 真机（catalog 已 pin darwin 资产 sha）；daemon 拉起走 `boot_new_session` Unix 分支（已平台化） |
| 四家 agent mac 安装 | 待做 | `oma agents install` 真机：darwin 资产名/解包/安装目录/探针待验收 |
| 真身四路 + settle 真机（mac） | 待做 | 信任屏 marker mac 差异；四家 TUI 画面过分类器 |

### 2. 挂起

> WSL Linux 剩余两项，待回环境续做。

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| 四家 agent Linux 安装 | 挂起 | `oma agents install` 真机：Linux 资产名/解包/leaf 找二进制/mark_executable 待验收（catalog pin 已备，S017 渠道序已实证） |
| 真身四路 + settle 真机（Linux） | 挂起 | 信任屏 marker Linux 差异；stub 全链已绿，真身待做 |

### 3. 接续口径

- 立项：新目标按 G003 五步（登记、研究、方案、实施、归档）走，先搜 `INDEX.md` 防重复造规则。
- 命令面：已落地 17 命令 + REPL（清单见 AGENTS 意图路由与 README）；`oma serve start/stop/status` 守护化，看板默认 spectator 只读（可写镜像用 `oma web`）。
- Linux 已验路径：daemon 拉起 `boot_new_session`、分类器 Unix 提示符、pid 守卫、serve 进程组隔离；stub 全链绿（2026-09-01）。
- 门禁：`rumdl check .` + `md-ref-scan.py` + `md-heading-scan.py` 裸跑；提交前含 `cargo test`（隔离 target）；提交精确 add（M036）。

## 完成的定义

> 本目标验收口径。

- mac 真机：构建与测试基线全绿、rmux 资产验收、四家 agent 安装、真身四路 + settle 全链绿。
- WSL Linux 侧：第一棒已收口（构建/基线/daemon/分类器/stub 全链）；挂起两项待回 WSL 环境续做后并档。
