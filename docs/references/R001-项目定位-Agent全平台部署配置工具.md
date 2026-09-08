# 项目定位：Agent 全平台部署配置工具

> AGENTS「项目定位」的展开。现役定位以本文件为准。演进：方案 0001 是首期切面，方案 0002 与 0004 是前两版编排定位，方案 0035 定调本定位（D15，2026-09-08 用户裁定：去编排，退化为纯部署配置工具）。

## 本质

Oh My Agents（仓库名 `ohmyagents-rs`，更名自 OhMyAgents，2026-09-02；CLI 二进制 `oma`）是 **Agent 全平台 token、hook 与状态栏部署配置工具**。

- **部署配置**：按各家规则在启动的项目目录部署 hook、skill、状态栏、yolo 键与信任预写，幂等合并，不把配置写成用户家目录全局默认。
- **token 管理**：`oma agents secrets` 一钥两密文存储加四 shell 懒注入（S031），是 token 部署配置主面；`oma agents providers` 别名簿承载提供商端点配置（S027）。
- **对话历史检索**：`oma trace` 六视图联邦读四家原生会话库（P0013/P0014），只读、与 rmux 零耦合；D15 曾连坐删除，D19（2026-09-08 用户裁定）全量恢复为只读检索面。
- **诊断**：`oma doctor` 只读体检（yolo / 信任 / 二进制 / state / 登录态 / hook 形态 / 状态栏 / CPU 能力），warn 与 block 分层。
- **全平台**：Windows / macOS / Linux（含 WSL）同一命令面；四环境自适应（P0027，矩阵见 S024）。
- **通用**：智能体集合可扩展。当前默认适配 Claude、Codex、Grok、Kimi，不是产品上限。

## 边界

- 配置钉在启动的那个项目目录，不替代 ohmypwsh 的五端环境总台。
- 不替代 claude / codex / grok / kimi 本体，只配置它们、诊断它们的部署形态。
- 不做编排（D15）：不拉起会话、不发任务、不做 web 镜像与传输面；rmux 不再是运行时后端。编排时代定位见归档 P0004 与 P0001 至 P0034。

## 四仓生态

> 2026-09-02 用户定调。四仓互相发 issue 协作（`https://github.com/raystyle/<仓>/issues`）；本仓是 agent 侧。

| 仓库 | 本地 | CLI | 职责 |
| --- | --- | --- | --- |
| ohmyenv-rs | `D:\ohmyenv-rs` | `ome` | 本机部署、管理、验收；agent 二进制下装与版本、工具与运行时依赖 |
| ohmyagents-rs | `D:\ohmyagents-rs` | `oma` | 诊断、配置、hook、状态栏与 token（本仓）；不管种子、不管 agent 二进制下装（D09）、不管编排（D15） |
| ohmypwsh | `D:\ohmypwsh` | 无单一 CLI | Windows 元主机操作台：初始主机密钥配置、兄弟仓 git 密钥扫描安全、集成 ome 与 oma 做本地与远程的主机操作、部署、检查、诊断 |
| ohmycloud | `D:\ohmycloud` | `omcf` | 云端基础设施：工具、运行时、agent 各版本二进制的 S3 存储与 web 分发（专用域名加 Cloudflare）；镜像种子归此仓 |

- 分工裁决：agent（claude / codex / grok / kimi）二进制下装归 ome（D07，ohmyagents#5）；镜像种子归 ohmycloud（`env.ohmygh.com`，D36 / P0014）；oma 不管种子，只管诊断、配置、hook、状态栏与 token（D09 加 D15，2026-09-07 与 2026-09-08 用户裁定）。旧裁决「安装归本仓、不进 ome 名录」（2026-08-31）已被 D07 反转。
- 分发通道：官方源与 `env.ohmygh.com` 镜像由 ome `download_asset_with_mirror` 消费；oma 不维护渠道序、不参与种子。`oma agents install` / `update` 仅兼容入口，指向 `ome install`。
- 密钥：本仓 providers.toml 走 sops 托管标准；密钥体系主权与跨仓密钥扫描安全归 ohmypwsh，集成而非自建。

## 约束在一个项目

启动 cwd 或 `--project` 就是唯一工作区。`init` 把各家项目级文件写进该目录。hook 状态在 `<project>\.oma\state\`（供状态栏 `agent:state` 机读标记消费，S025）。信任库（各 agent 记在用户家）可以预写；hook 注册不写家目录。
