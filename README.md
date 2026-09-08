# Oh My Agents

## 项目介绍

**Agent 全平台 token、hook 与状态栏部署配置工具**（D15，2026-09-08 起）：按目录为终端智能体（当前适配 Claude / Codex / Grok / Kimi 四家）部署 hook、skill、状态栏与 yolo 键，管理 token（secrets 一钥两密文加四 shell 懒注入），并提供只读诊断。oma 不做编排：不拉起会话、不发任务；编排面与 rmux 后端已移除（历史见 `docs\proven\` P0001 至 P0034）。

- 显示名 Oh My Agents；仓库 `ohmyagents-rs`（更名自 OhMyAgents，2026-09-02）；CLI `oma`；远端 <https://github.com/raystyle/ohmyagents-rs>
- **部署配置**：`oma init` 按各家规则落项目级 hook / skill / 状态栏 / yolo 键，幂等合并保留外条目，不写用户家目录默认
- **token 主面**：`oma agents secrets` 一钥两密文存储（app.key / identity.enc / secrets.yaml，SOPS 制）加四 shell 懒注入，明文不常驻；`oma agents providers` 提供商别名簿（zhipu / deepseek 等端点形态）
- **状态栏**：`oma agents statusline` 四家写入面幂等；hook 状态落盘供 `agent:state` 机读标记消费
- **诊断**：`oma doctor` 只读体检（yolo / 信任 / 二进制 / 登录态 / hook 形态 / 状态栏 / CPU 指令集），warn 与 block 分层，block 才退出 1
- **密钥拦截**：`oma hook` 内置密钥闸，PreToolUse / UserPromptSubmit 命中 block 级密钥 exit 2 拒调用（八层防误报）
- **全平台**：Windows / macOS / Linux（含 WSL）同一命令面，四环境自适应部署
- **agent 二进制下装归 ome**（D07 迁册）：`ome install <agent>`；本仓 `oma agents install` / `oma agents update` deprecated 保留兼容

## 如何安装部署

前置：Rust 工具链（rustc/cargo）。oma 自管数据根 `~/.oma`，不动家目录注册。

```powershell
git clone https://github.com/raystyle/ohmyagents-rs
cd ohmyagents-rs
cargo build            # release: cargo build --release
```

检测 agent（缺的用 ome 装）：

```powershell
.\target\debug\oma.exe agents            # 四家已装情况（source=path|env|oma|default + version）
ome install claude                       # 缺的 agent 用 ome 装（D07 起 agent 下装归 ome）
```

进目标项目初始化：

```powershell
oma init --project D:\my\proj            # hook + skill + yolo 键（幂等，不动家目录）
oma agents statusline                    # 配置四家状态栏（幂等）
oma doctor --project D:\my\proj          # 只读诊断部署形态
```

## 完整命令示例

### 典型用法

> `--project PATH` 缺省即当前目录：进项目后全程不用带它。全部命令支持 `--format kv|json|jsonl` 与 `--json` 信封。

```powershell
cd D:\my\proj

oma init                              # 一次性：部署 hook/skill/yolo 键（幂等，重复跑安全）
oma agents statusline                 # 状态栏四家写入面
oma agents secrets init               # token 金库初始化（一钥两密文）
oma agents secrets set ANTHROPIC_API_KEY   # 写入 token（走 stdin，不进 argv）
oma agents secrets inject             # 四 shell profile 写懒注入块（交互 shell 现场解密）
oma doctor                            # 部署形态体检：block 才退出 1
oma agents login grok                 # 设备码登录引导（URL 加 code 跨机完成）
```

### 安装与诊断

```powershell
oma doctor                             # 只读诊断：yolo/信任/二进制/hook 形态/状态栏/登录态 + CPU 指令集段
oma agents                             # 列四家检测（source=path|env|oma|default + version）
oma agents install                     # 已 deprecated（D07 迁册 ome，保留兼容）：自适应装缺
oma agents update                      # 已 deprecated（D07 迁册 ome，保留兼容）：全部升到最新
oma agents providers --example         # 提供商别名簿样例（providers.toml）
oma agents statusline codex            # 只配一家状态栏
```

### 项目初始化

```powershell
oma init                               # 全套：yolo 键 + 四家 hook/skill（SKILL.md 命令图生成）
oma init --yolo                        # 仅无阻塞键
oma init --yolo --pretrust             # 额外预写家目录信任库（四家）
oma init --permission-mode manual      # 覆盖默认 yolo（manual 不写 bypass）
oma hook                               # agent hook 入口（读 stdin JSON 写 .oma/state；密钥拦截闸）
```

### 自维护

```powershell
oma self update                        # 自更新（缺省 dev 滚动源，按资产 sha256 判新）
oma self update --stable               # 走正式版
oma completions powershell             # 补全脚本（bash/zsh/fish/powershell）
```

## 更多文档

- `PRD.md`：需求清单（四原语之首，D 编号与生命周期）
- `AGENTS.md`：协作规则（定位/工作规则/意图路由/资源索引）
- `INDEX.md`：全量索引（D/P/S/R/G/M 编号定位）
- `docs\references\R002-常用命令与管理流程-从项目init到部署诊断.md`：命令手册细则
- 命令设计、研究过程与经验见 `docs\` 各分册

## 环境前提

验收机 2026-09-01：Windows 11 + pwsh 7（rustc/cargo 1.97.1；claude 2.1.246、codex 0.149.1、grok 1.0.13、kimi 0.39.1；CPU 实测 x86_64 avx=true avx2=true avx512f=false）；macOS arm64（Darwin 25.5.0；rustc/cargo 1.97.0）；WSL Linux 同口径收口。编排面验收史见归档 P0012 / P0019。

yolo 启动旗标会关掉审批和沙箱，只在自己信任的项目目录用。
