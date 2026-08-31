# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

自适应本机安装部署：oma 参照 ohmypwsh 接管本机 rmux 加四家 agent（claude、codex、grok、kimi）在 Windows、Linux、macOS 的安装与配置（对应 `GOAL.md`，方案 P0012，登记日 2026-08-31；用户四条定调 2026-08-31）。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| ohmypwsh 安装面研究 | 已完成 | 双路深读 + 六条载荷性断言回源码抽查；半截现状三坑（kimi 残条目、grok 缺 single 分支、New-ToolDef 双源漂移） | 2026-08-31 |
| ohmypwsh 配置面研究 | 已完成 | 四家配置落点、合并三流派、密钥边界（划归 ohmypwsh 不吸收） | 2026-08-31 |
| 四家官方渠道取证 | 已完成 | oma catalog 目标版本与官方校验和：claude v2.1.251（SHASUMS256.txt）、codex rust-v0.151.0（codex-package_SHA256SUMS）、kimi @0.39.1（.sha256 边车）、grok 1.0.13（x.ai/cli/stable 通道，仓无 release/tag） | 2026-08-31 |
| aitrace 研究 | 进行中 | D:\aitrace 深读在跑（queued 目标 P0013：意图操作块与编辑文件轨迹检索） | 2026-08-31 |
| S017 研究落档 | 已完成 | `docs\research\S017-ohmypwsh安装配置机制与四家agent渠道取证.md`（含追记：四家官方安装脚本逐家实证与渠道反转） | 2026-08-31 |
| 立项 0012 | 已完成 | `docs\proven\P0012-自适应本机安装部署-rmux与四家agent接管.md`（Windows 保证、Linux/mac 环境切换接管） | 2026-08-31 |
| catalog/agents 泛化 | 已完成 | `catalog\agents.toml`：四家、渠道序（github 主 CDN 兜底）、资产按 source 绑定（kimi 双渠道制品不同的实证）、加载期 schema 校验（残条目/未实现 kind/坏 sha 全拦） | 2026-08-31 |
| install 实现与验收 | 已完成 | `src\install.rs` + `oma agents install/update`；Windows 隔离 OMA_HOME 四家装机全绿（sha 锚全过、探针全活、grok 承接 hash 自证）；update 写回用户本地 pin 闭环；54 测过 | 2026-08-31 |

## 队列目标

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| P0011 三传输编排面 | 挂起待续 | HTTP API、MCP、网页可视化四切片全待办（方案已立） |
| P0013 aitrace 检索 | 排队 | 指定项目下各 Agent 意图操作块及编辑文件轨迹检索；D:\aitrace 研究进行中 |

（P0006 至 P0010 已完成；过程与经验在对应 proven 方案。）
