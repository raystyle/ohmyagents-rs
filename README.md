# Oh My Agents

## 项目介绍

**通用智能体多路复用任务编排器**：在 rmux 上把多路终端智能体（当前适配 Claude / Codex / Grok / Kimi 四家）编进一个项目会话，按目录自动部署 hook 与 skill，用 `oma` 下发任务、看状态、检轨迹。

- 显示名 Oh My Agents；仓库 `ohmyagents-rs`（更名自 OhMyAgents，2026-09-02）；CLI `oma`；远端 <https://github.com/raystyle/ohmyagents-rs>
- **三通道编排**：CLI、HTTP API（`oma serve`，主页即可视化看板）、MCP（`oma mcp` stdio），一份编排核心三消费
- **agent 实例优先**：命令面只见 agent；服务、会话、窗口、窗格、PTY 作为复杂性绑在 agent 背后：初始检测互斥、操作绑定已开实例，绝不重复开已活原语（新开/附加/重开三态和解）；**精确集合**：`--agents` 给几路就几路，多余路自动收掉；**布局按路数自适应**：1 路全屏、2/3 路左右列分、4 路 2x2，收放后自动重排
- **带产物等待的任务**：`oma task` 建任务目录（`prompt.md` 提示词全文）、send 带协议尾注、阻塞等 `DONE` 标记（agent 写 `output.md` 后最后创建），委派即产物、后台收件人模式即闭环
- **任务开始确认与阻塞告警**：send/run/task 发出后等该路真开始（working/画面变化双信号）；blocked（确认/密码框）、未启动、死路、孤儿窗格全部打 `*.alert=` 告警，信任框类由 settle 白名单自动处理
- **单键守卫**：`oma key` 发单键（codex 拒 `C-c`：一个 C-c 杀进程；打断 codex 用 Esc）
- **可视化看板**：`oma serve` 主页即 web 镜像：打开就是多路窗格实时画面（fit-fill 字号自适应铺满），可打字可拖窗格（本地 operator）；资源包随二进制走，首启释放 oma 数据根
- **联邦轨迹检索**：`oma trace` 查询时直读四家原生会话库，双意图（用户请求与 assistant 声明）加 operation_id 归组，可回溯 oma 出现之前的历史
- **自适应安装**：`oma check` 装 rmux（pin + sha256 信任锚）；`oma agents install` 装缺的 agent（github 主 CDN 兜底）；`oma agents update` 取证升级并写回用户本地 pin
- **安全面**：serve 只绑 127.0.0.1 + 全局 Host 回环闸（防 DNS rebinding）；公网中继镜像（`oma web` 官方域）缺省 PIN，免 PIN 组合打显著警示
- **自举工作流**：oma 编排的 agent 给 oma 自身做 review（`.tools\review-round.py` 轮询接力，FINDINGS 契约 + 已拍板不修清单收敛）

## 如何安装部署

前置：Rust 工具链（rustc/cargo）。oma 自管数据根 `~/.ohmyagents`，不动家目录注册。

```powershell
git clone https://github.com/raystyle/ohmyagents-rs
cd ohmyagents-rs
cargo build --features server,mcp      # release: cargo build --release --features server,mcp
```

装运行时与 agent（全部自适应：已装即跳过）：

```powershell
.\target\debug\oma.exe check           # 装/校验 rmux（pin 在 catalog/rmux.toml，现役 0.10.0）
.\target\debug\oma.exe agents          # 检测四家已装情况（缺装行带 hint）
.\target\debug\oma.exe agents install  # 缺的按 catalog 装（oma 自管根 ~/.ohmyagents/agents）
```

进目标项目初始化并开会话（注意：不要在本仓库根跑 `init`：会写 `.claude` / `.codex` / `.kimi-code` 进项目）：

```powershell
oma init --project D:\my\proj          # hook + skill + yolo 键（幂等，不动家目录）
oma spawn --project D:\my\proj         # 和解式拉起：缺省已装交集，1-4 路
oma serve start --project D:\my\proj   # 后台起编排面（即调即退）；浏览器开 http://127.0.0.1:7900/ 即看板
```

把 `oma` 加进 PATH 的两种方式（任选）：

```powershell
cargo install --path D:\OhMyAgents --features server,mcp   # 装进 ~/.cargo/bin（已在 PATH）
# 或调试期直接用构建产物 + alias：
Add-Content $PROFILE "Set-Alias oma D:\OhMyAgents\target\debug\oma.exe"
```

日常入口任选：

```powershell
oma                                    # REPL（和解起会话 + 内嵌编排面 + 行循环）
oma serve start                        # 后台编排面（即调即退，主页即看板）
oma serve stop                         # 停掉（协议化排空优雅退出）
oma status                             # 纯 CLI
```

## 完整命令示例

### 典型用法

> `--project PATH` 缺省即当前目录：`cd` 进项目后全程不用带它。六会话命令与 respawn 均支持 `--json`（与 HTTP/MCP 同形信封 `{ok, data|error, meta}`）。

```powershell
cd D:\my\proj                        # 进目标项目（不加 --project 即用此目录）

oma init                              # 一次性：部署 hook/skill/yolo 键（幂等，重复跑安全）
oma spawn                             # 开会话：缺省已装交集 1-4 路（如 claude,codex,grok,kimi）
oma serve start                        # 后台起看板（即调即退），浏览器开 http://127.0.0.1:7900/
#   看板里：四路窗格实时画面、直接打字对话、拖动分隔线调布局

oma run "给四家都总结一下当前架构"       # 日常委派：状态门分派，忙路自动跳过
oma send claude "看看 src/main.rs"     # 单路直发
oma task codex "review 并把结论写产物"  # 带产物等待：阻塞到 agent 写 output.md 并建 DONE
oma task list                         # 任务清单与完成态
oma status                            # 看各路状态（pid/进程/终端态/hook 态）
oma respawn codex                     # 某路死了或卡住：只重开这一路

oma trace timeline --limit 10         # 查轨迹：谁改了什么、基于什么意图
oma cleanup                           # 收工：只杀本项目会话
```

每天回来接着干（会话跨命令可重连）：

```powershell
cd D:\my\proj
oma spawn                             # 和解：活路附加（不重开）、死路自动重开
#   spawn.attached=claude,codex  spawn.respawned=grok  spawn.mode=reconcile
oma serve start                        # 看板照常（已活直接返回地址，不重复开）
```

只想要一个交互入口：

```powershell
cd D:\my\proj
oma                                   # REPL：和解起会话 + 内嵌看板 + 行循环
> all 跑一遍构建
> claude 修一下编译错误
> status
> quit                                 # 只 detach；明天回来会话还在
```

### 安装与诊断

```powershell
oma check                              # 核对 rmux pin（版本+sha256+布局）；缺则装
oma check --no-install                 # 只诊断不下载（不符则退出非 0）
oma doctor                             # 只读诊断：yolo/信任/二进制/hook 形态/状态栏/登录态/会话健康 + CPU 指令集段
oma agents                             # 列四家检测（source=path|env|oma|default + version）
oma agents install                     # 自适应装缺（已装任何来源即跳过）
oma agents install claude grok --force # 指定重装 oma 自管根
oma agents update                      # 全部升到最新（取证 sha 后写回用户本地 pin）
oma agents update kimi                 # 只升一家
oma agents statusline                  # 配置 claude/codex 状态栏（幂等；脚本释放 oma 数据根）
oma agents statusline codex            # 只配一家
```

### 项目初始化

```powershell
oma init                               # 全套：yolo 键 + 四家 hook/skill（SKILL.md 命令图生成）
oma init --yolo                        # 仅无阻塞键
oma init --yolo --pretrust             # 额外预写家目录信任库（四家）
oma hook                               # agent hook 入口（读 stdin JSON 写 .ohmyagents/state）
```

### 编排

> 和解式三态：会话不在新开、在则活路附加、死路重开；精确集合：给几路就几路。

```powershell
oma spawn                              # 新开：缺省已装交集，1-4 路
oma spawn --agents claude,codex        # 指定路（多余路自动收掉出 removed；1/2/3/4 路布局自适应）
oma spawn --stub                       # shell 桩（验收与调试）
oma spawn                              # 会话已在：活路附加、死路重开
#   输出：spawn.attached=claude  spawn.respawned=codex  spawn.removed=grok  spawn.mode=reconcile

oma respawn codex                      # 强制重开一路（先分后杀保会话；不动其它路）
oma status                             # 各路 pid/进程/终端态/hook 态（TTY 对齐表格；死路报 dead）
oma status --json                      # JSON 信封（机器面；进程名查询失败带 warning）
```

### 发任务与分派

```powershell
oma send claude "修复 src/main.rs 的编译错误"          # 单路单行（等回显再 Enter；阻塞框打 send.alert）
oma send claude "多行
任务
文本"                                                # 多行自动三段式粘贴
oma send claude "跑测试" --confirm "test result: ok"   # 等画面出现确认短头
oma run "给四家都总结一下当前架构"                     # 状态门分派：忙路跳过不堵其它路；全拦退出非 0
oma run "重构登录模块" --assign claude,codex           # 只分派指定路（重复名自动去重）
oma settle --wait 30                    # 自愈信任框（claude Enter / kimi 上移+Enter / codex 升级屏与 hooks 审查屏）
oma key codex Esc                       # 发单键（守卫：codex 拒 C-c；打断用它）
oma cleanup                             # 只杀本项目会话（不动 daemon 与其它会话）
```

### 带产物等待的任务

> 任务目录协议：`.ohmyagents\tasks\<id>\` 下 agent 读 `prompt.md`、写 `output.md`、最后创建空文件 `DONE`；oma 只认 DONE（防半写）。SKILL 已部署协议，agent 知道怎么做。

```powershell
oma task codex "review src/ 并把结论写产物"             # 建目录 + 发送 + 阻塞等 DONE → 打印产物退出
oma task codex "..." --timeout 0                       # 无限等（上限 86400）
oma task codex "..." &                                 # 后台阻塞（产物落盘后进程自退）
oma task list                                          # 任务清单与完成态
oma task show t001                                     # 元数据 + 产物收取（超时后晚到也能收）
```

等另一个 agent 的产物用**收件人模式**（不占会话前台）：

```powershell
while (-not (Test-Path .ohmyagents\tasks\t001\DONE)) { Start-Sleep 15 }
Get-Content .ohmyagents\tasks\t001\output.md
```

### 轨迹检索

> 查询时联邦读四家原生会话库。

```powershell
oma trace sessions                      # 项目内各 agent 会话
oma trace timeline --limit 20           # 编辑事件（operation_id、双意图、真实时间戳）
oma trace timeline --agent grok --file "src/*.rs"
oma trace blocks                        # 操作块视图（一次工具调用一块，可能多文件）
oma trace agent claude                  # 某家 agent 的块时间线
oma trace file src/orch.rs              # 单文件被谁何时基于什么意图改过
oma trace search "登录|auth"            # 正则检索 patch/file/双意图四域
```

### 传输与镜像

```powershell
oma serve start                         # 后台守护化（即调即退；已活秒回地址）
oma serve start --port 8080 --project D:\my\proj
oma serve status                        # pid / 端口 / 是否活
oma serve stop                          # 协议化停机（DELETE /shutdown 优雅排空，兜底强杀）
oma serve                               # 裸形态保留前台跑（调试用）
#   浏览器开 http://127.0.0.1:7900/ 即看板（整会话镜像、免 PIN、可打字可分屏）
#   RESTish：POST /spawn | GET /status | POST /send | POST /run | POST /settle | DELETE /session
#            POST /share/{agent} | GET /share | DELETE /share/{id}/stop
#            GET /trace/sessions|timeline|search    （全部 JSON 信封）

oma mcp                                 # MCP server（stdio 九 tools：六操作 + trace 三件）
oma mcp --project D:\my\proj
oma mcp --print-config                  # 打印 Claude Code / codex / 通用 mcpServers 注册片段

oma web                                 # 起 web 镜像链接（缺省整会话、官方域、PIN 防外发）
oma web claude                          # 单路镜像
oma web --spectator --ttl 600           # 只读旁观 10 分钟
oma web --no-pin                        # 免 PIN 直连（本地场景）
```

### REPL 与其他

```powershell
oma                                     # REPL：和解起会话 + 内嵌编排面（7900 顺延 7909）
oma --agents claude,codex --no-web      # 指定路、不起 HTTP
oma --open                              # 打印 URL 后尝试开浏览器（失败只警告）
oma completions powershell              # 补全脚本（bash/zsh/fish/powershell）
```

REPL 会话内：

```text
> all 给四家都跑一遍构建
> claude 看看 src/orch.rs 的 spawn 逻辑
> status
> web
> quit          （只 detach；拆会话用 oma cleanup）
```

## 更多文档

- `PRD.md`：需求清单（四原语之首，D 编号与生命周期）
- `AGENTS.md`：协作规则（定位/工作规则/意图路由/资源索引）
- `INDEX.md`：全量索引（D/P/S/R/G/M 编号定位）
- `docs\references\R002-常用命令与管理流程-从项目init到会话cleanup.md`：命令手册细则
- 命令设计、研究过程与经验见 `docs\` 各分册

## 环境前提

验收机 2026-09-01：Windows 11 + pwsh 7（rustc/cargo 1.97.1；claude 2.1.246、codex 0.149.1、grok 1.0.13、kimi 0.39.1；rmux 0.10.0；CPU 实测 x86_64 avx=true avx2=true avx512f=false）；macOS arm64（Darwin 25.5.0；rustc/cargo 1.97.0；四家 agent 与 rmux 0.10.0 同版；`oma check`/stub 全链/四家 darwin 安装/真身四路 + settle 全绿，P0012）。

yolo 启动旗标会关掉审批和沙箱，只在自己信任的项目目录用。三平台真机全验：Windows 与 macOS 四家安装加真身四路全链绿；WSL Linux 同口径收口（2026-09-01，四家 `--force` 安装探针全绿、真身四路 + settle、`oma task` 真任务产物精确）。
