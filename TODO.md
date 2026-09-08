# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

无。D16 当日闭环。

## 前目标清单

> D16 oma self update 镜像通道（2026-09-08 归档 P0036）：OMA_MIRROR dev 段边车判新、sha256 强制校验、网络失败回落 GitHub。
> D15 oma 去编排收窄为纯部署配置工具（2026-09-08 归档 P0035）：编排命令与 rmux 后端移除；保留 init / doctor / agents / hook / self update / completions；文档八面同步。
> D14 数据目录改名为 `.oma`（2026-09-07 归档 P0034）：项目与家目录两根同改；旧 `.ohmyagents` 仅旧在则迁；不是 `.omc`。
> D12 任务目录孤儿（2026-09-07 归档 P0033）：`.ohmyagents/t006/` 是 t008 第二轮草稿误落，已删；协议尾注路径显式 `tasks/`。
> D11 状态栏 zig/go/cpp（2026-09-07 归档 P0032）：projKind 扩展；本机临时目录实跑；Grok 仍跳过工具链。
> D10 G005 存量字符清理（2026-09-07 归档 P0031）：SKIP_DIRS 外四类禁字清零，封闭清单删除。
> D08 doctor 检查面补形态（2026-09-07 归档 P0030）：Grok 状态栏 command 三态、JSON hook args 形态；用户实证栏正常。同日 D09：oma 不管种子。
> D07 oma 收窄配合迁册（2026-09-07 归档 P0029）：agents install/update deprecated、agents.toml 冻结历史锚、doctor 四类归 agents 域；当日热修 M044 至 M048。
> 文档体系重构（D01 至 D05，2026-09-03）：全链已完成。
> D06 agent 二进制下装部署五端全量收敛（2026-09-05）：当日闭环后同日方向反转（D07）。

## 队列目标

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| D13 mac `--version` 一致性 | 排队 | CI 资产已出（2026-09-08 04:41Z 三平台）；待 mac 上 `oma self update` 实收（可走 OMA_MIRROR 镜像通道，D16） |
