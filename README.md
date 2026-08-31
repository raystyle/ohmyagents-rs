# Oh My Agents

一句话定位：**通用智能体多路复用任务编排器**。在 rmux 上把多路终端智能体编进一个项目会话，按目录部署 hook 与 skill，用 `oma` 下发任务。当前默认适配 Claude / Codex / Grok / Kimi。显示名 Oh My Agents，仓库 `ohmyagents`，CLI `oma`。远端 <https://github.com/raystyle/OhMyAgents>。`oma check` 负责 rmux 的检测、版本/哈希校验与安装。

## 快速开始

文档、`oma check`、项目级 `init --yolo` / `doctor` 已开。当前目标是各功能部件 POC（方案 0005）；`spawn` / `send` / `cleanup` 尚未落地。先读：

```text
AGENTS.md          最高约束
GOAL.md            当前目标
docs\PLAN.md       怎么做
docs\TODO.md       做到哪
docs\history\0001-四路会话工具-CLI控制面与网页观察面.md
```

先装 Rust 工具链，再让 `oma` 自己装 rmux（pin 在 `catalog/rmux.toml`，现役 0.10.0）：

```powershell
cargo build
cargo run -- check
cargo run --example poc-yolo-doctor
cargo run --example poc-endpoint
cargo run --example poc-session
cargo run --example poc-layout
cargo run --example poc-drive
cargo run --example poc-dialogs
cargo run -- init --yolo --project $env:TEMP\oma-demo
cargo run -- doctor --project $env:TEMP\oma-demo
```

不要在本仓库根目录跑 `oma init`：会写入 `.claude` / `.codex` / `.kimi-code`。信任库用 `--pretrust`，默认不写家目录。

## 目录结构

核心布局（明细见 `docs\references\文档全量清单-方案与研究目录的完整索引.md`）：

```text
ohmyagents/
  catalog\           rmux 版本与 SHA256 pin
  src\               oma CLI
  examples\          部件 POC
  docs\history\      方案 NNNN
  docs\diary\        项目日记
  docs\research\     研究（文件名即标题）
  docs\guide\        指南与细则（怎么写、怎么测、怎么用）
  docs\references\   定位与全量清单
```

## 核心概念

- **自动配置**：`init` 把 hook、skill 写进启动的项目目录
- **任务编排**：CLI 下发、看状态、收尾；网页只镜像，默认不弹窗
- **可扩展 agent 表**：当前默认四家，缺已启用的一家则 check 失败
- **六态标记**：研究与测试文档的事实性断言标实证 / 推断 / 经验 / 记忆 / 假设 / 直觉，标准见 `docs\guide\研究标准细则-结构与六态标记.md`

## 常用命令

已落地：`check`、`init --yolo`、`doctor`。其余命令见设计文档。文档检查：

```powershell
rumdl check .
```

命令设计见 `docs\guide\常用命令与管理流程-从项目init到会话cleanup.md`。

## 文档导航

- `AGENTS.md`：定位 / 操作规则 / 意图路由 / 资源索引
- `docs\guide\文档标准细则-命名写作规范与rumdl检查.md`：命名与写作
- `docs\guide\研究标准细则-结构与六态标记.md`：研究规范与六态标记
- `docs\guide\测试标准细则-分层断言与门禁流程.md`：测试分层与门禁
- `docs\references\文档全量清单-方案与研究目录的完整索引.md`：全量索引
- `docs\references\项目定位-通用智能体多路复用任务编排器.md`：现役定位
- `docs\research\四路会话的控制面与观察面.md`：先读的研究

## 环境前提

本机 2026-08-29：

- Windows，pwsh
- rustc / cargo 1.97.1
- claude 2.1.246、codex 0.149.1、grok 1.0.13、kimi 0.38.0
- **rmux 不在 PATH**，阶段 1 才装

yolo 启动旗标会关掉审批和沙箱，只在自己信任的项目目录用。
