# Oh My Agents

一句话定位：**通用智能体多路复用任务编排器**。在 rmux 上把多路终端智能体编进一个项目会话，按目录自动部署 hook 与 skill，用 `oma` 下发任务、看状态、检轨迹。当前适配 Claude / Codex / Grok / Kimi 四家。显示名 Oh My Agents，仓库 `ohmyagents`，CLI `oma`。远端 <https://github.com/raystyle/OhMyAgents>。

编排操作三通道：CLI、HTTP API（`oma serve`，自带可视化网页）、MCP（`oma mcp` stdio）；一份编排核心三消费。

## 快速开始

先装 Rust 工具链，然后让 oma 自己装运行时与缺的 agent（rmux pin 在 `catalog/rmux.toml`，agent pin 在 `catalog/agents.toml`；oma 自管数据根 `~/.ohmyagents` 不动家目录注册）：

```powershell
cargo build --features server,mcp
.\target\debug\oma.exe check           # 装/校验 rmux（现役 0.10.0）
.\target\debug\oma.exe agents          # 检测四家已装情况
.\target\debug\oma.exe agents install  # 缺的按 catalog 装（github 主 CDN 兜底）
```

进项目目录初始化并开会话（不要在本仓库根跑 `init`：会写 `.claude` / `.codex` / `.kimi-code`）：

```powershell
oma init --project D:\my\proj    # hook + skill + yolo 键，幂等，不动家目录
oma spawn --project D:\my\proj   # 拉起多路 agent 会话（缺省已装交集）
oma                              # 或裸 oma 进 REPL（内嵌网页，打印 URL）
```

REPL 行命令：`all <文本>`（状态门分派）、`claude|codex|grok|kimi <文本>`（单路）、`status`、`web`、`quit`（只 detach）。

三传输等价：

```powershell
oma serve --port 7900 --project D:\my\proj   # HTTP：GET / 网页、六操作 RESTish、SSE 画面、trace 端点
oma mcp --project D:\my\proj                 # MCP stdio：九 tools
oma mcp --print-config                       # 各客户端注册片段
```

六会话命令（spawn/status/send/run/settle/cleanup）都支持 `--json`（与 HTTP/MCP 同形信封）；`oma completions powershell` 出补全脚本。

## 目录结构

核心布局（明细见 `INDEX.md`）：

```text
ohmyagents/
  INDEX.md           文档总索引（P/S/R/G/M 编号定位）
  GOAL / PLAN / TODO 三原语
  catalog\           rmux 与四家 agent 的版本 pin（信任锚是文件哈希）
  src\               oma CLI（orch 编排核心 + api 传输无关层 + 三适配前端）
  examples\          部件 POC（Windows 范围全绿）
  docs\web\          可视化单页（无构建链，serve 直出）
  docs\proven\       P 编号，已完成方案归档
  docs\diary\        项目日记（一天一篇总结自省）
  docs\research\     S 编号，研究原型过程（六态）
  docs\guide\        G 编号，元规范
  docs\references\   R 编号，开发测试参考
  docs\mistakes\     M 编号，错误速查
```

## 核心概念

- **编排核心一份三消费**：`orch`（Link 加六函数）不感知传输；CLI 行式、HTTP 信封、MCP structured 信封都是薄适配（api 层共用）
- **自动配置**：`init` 把 hook、skill（命令图生成）、yolo 键写进项目目录，幂等合并不动家目录
- **状态门**：`run` 分派前查各路 hook 态与终端态，一路忙/阻塞跳过不堵其它路
- **自愈信任**：`settle` 自检测并自动确认信任/审查框（密码类永不自动）
- **联邦轨迹检索**：`oma trace` 查询时直读四家原生会话库（claude projects、codex rollout、grok updates 权威日志、kimi wire），双意图（用户请求与 assistant 声明）加 operation_id 归组，可回溯 oma 出现之前的历史
- **六态标记**：研究与测试文档的事实性断言标实证 / 推断 / 经验 / 记忆 / 假设 / 直觉，标准见 `docs\guide\G002-研究标准细则-结构与六态标记.md`

## 常用命令速查

| 意图 | 命令 |
| --- | --- |
| 核对运行时 | `oma check` |
| 检测/安装/升级 agent | `oma agents` / `agents install` / `agents update` |
| 初始化项目 | `oma init` |
| 无阻塞诊断 | `oma doctor`（含 CPU 指令集能力段） |
| 拉会话/看状态/收尾 | `oma spawn` / `oma status` / `oma cleanup` |
| 发任务/分派 | `oma send <agent> "<文本>"` / `oma run "<文本>"` |
| 自愈信任 | `oma settle` |
| 查轨迹 | `oma trace sessions\|timeline\|blocks\|agent\|file\|search` |
| 网页/MCP | `oma serve` / `oma mcp` |
| REPL | 裸 `oma` |

文档检查：`rumdl check .`。命令设计见 `docs\references\R002-常用命令与管理流程-从项目init到会话cleanup.md`。

## 文档导航

- `AGENTS.md`：定位 / 操作规则 / 意图路由 / 资源索引
- `INDEX.md`：全量索引
- `docs\references\R001-项目定位-通用智能体多路复用任务编排器.md`：现役定位
- `docs\references\R002-常用命令与管理流程-从项目init到会话cleanup.md`：命令手册
- `docs\guide\G001-文档标准细则-命名写作规范与rumdl检查.md`：命名与写作
- `docs\guide\G002-研究标准细则-结构与六态标记.md`：研究规范与六态标记
- `docs\references\R004-测试标准细则-分层断言与门禁流程.md`：测试分层与门禁

## 环境前提

本机 2026-08-31：

- Windows 11，pwsh 7；rustc / cargo 1.97.1
- claude 2.1.246、codex 0.149.1、grok 1.0.13、kimi 0.39.1
- rmux 0.10.0（oma 自管根，`oma check` 安装）
- CPU 能力（`oma doctor` 实测）：x86_64，avx=true avx2=true avx512f=false

yolo 启动旗标会关掉审批和沙箱，只在自己信任的项目目录用。Linux/mac 接管已排后（资产与代码路径就绪，预备检测见 `docs\research\S021`）。
