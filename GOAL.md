# GOAL：任务目标管理

> 角色：**工作任务管理**，四个部分——**起点**、**锚点**、**进程**、**历史**。随工作实时更新。
> 与其它文档分工：`ROADMAP.md`=阶段路线；`CHANGELOG.md`=版本成果；`docs\diary\YYYY-MM-DD-*.md`=项目日记（当天做了什么）；`docs\proven\NNNN-*.md`=方案与过程经验；`TODO.md`=进度清单；`PLAN.md`=实施指导。

## 起点

> 当前目标的起点：何时发起、为什么发起、要解决什么问题。

- **日期**：2026-08-31。
- **起点**：用户四条定调连发：①「研究学习 ohmypwsh，我们的命令把 windows、linux 和 mac 的这几个 agent 的安装、配置也全部实现」②「自适应系统，管理本机的 agent 安装部署」③「oma 也参考 ohmypwsh 接管自适应本机系统本地 rmux 的 linux、mac 和 windows 的安装」④「增加研究 D:\aitrace，实现指定项目下各 Agent 意图操作块及编辑文件轨迹的检索功能」。①②③合成本目标（本机自适应安装部署：rmux 加四家 agent，三平台）；④立下 queued 目标（aitrace 检索，P0013）；P0011（三传输编排面）实现切片未动，挂回队列。

## 锚点

> 当前锚定的目标 + 推进时间线。

- **锚定的目标**：P0025 serve 守护化于 2026-09-01 达成后，Windows 侧产品形态全量收口（P0017 至 P0025 连续九案闭环）。当前无现役切片，待用户定向。queued：P0012 的 Linux/mac 环境切换接管（用户定调排后）。

### 推进时间线

> 倒序：最新进展在最上。

| 日期 | 进展 |
| --- | --- |
| 2026-09-01 | P0012 阶段切换：WSL Linux 第一棒收口（「目前只到 wsl linux 就可以了」），mac 接管开发启动（「准备让 mac 接管开发」）；变更推远程供 mac 侧拉取接续 |
| 2026-09-01 | P0012 第一棒（WSL）：hook 路径报错修复（oma init 幂等改写）；测试基线 5 败清零（M041 记档 pid_alive 的 kill -0 双语义 + 4 处测试平台假设）；R010 六欠账清四（daemon 拉起、分类器、serve 进程组、pid 守卫）；stub 全链与 serve/doctor/HTTP 验收绿；基线 78+10、81+10 全绿零警告 |
| 2026-09-01 | Windows 侧总收口：四 agent 轮询 review 接力工作流（7 棒收敛至功能性缺陷清零，60+ 修复含 6 件修复自身回归）；oma task 带产物等待的任务目录协议（端到端双验）；任务开始确认与阻塞告警；精确集合与布局自适应；oma key 守卫；agents statusline（claude/codex 幂等）；R010 交接清单落档、P0012 Linux 接管启动 |
| 2026-09-01 | P0026 切片 1 达成：codex review 结果 trace 收取（15 条，高 5 核实 4 真 1 部分真）后立项三切片；看板默认 spectator 只读（用户定调）＋Host 回环校验（高5）＋cleanup 僵局解除（高2）＋死路杀旧 pane 不堆积（高3）＋manifest 原子写（高1a）；计划外抓到并修 serve daemon DETACHED 零控制台下 rmux CLI 卡死（改 CREATE_NO_WINDOW） |
| 2026-09-01 | serve stop 协议化补齐：核对三原语时发现 `serve_stop` 实际只有 taskkill（P0025/R002 口径超写）——补 `DELETE /shutdown` 优先（ureq 复用）加轮询退出加超时强杀兜底，实测日志见 draining；顺手清三处未用导入；README/AGENTS 对齐 start/stop/status 形态 |
| 2026-09-01 | 规则体系收口：G004 经验沉淀细则（proven/references 双链、mistakes 当场记加二犯升格）挂 AGENTS 工作节奏强规则位；M035 记档（python 替换吃 `\r` 劈行，修复过程又踩同型两次）；README 重写为介绍/安装部署/完整命令示例三段 |
| 2026-09-01 | P0025 达成：serve 守护化——`serve start` 即调即退（DETACHED 孤儿化、端口就绪等待、状态文件）、协议化停机端点（DELETE /shutdown → AtomicBool → 优雅排空，rmux kill-server 同构）、FFI OpenProcess 探活（tasklist 在 Job Object 内管道死锁） |
| 2026-09-01 | P0024 达成：agent 实例和解式编排——spawn 三态（新开/附加/死路重开，`attached`/`respawned` marker）＋ `oma respawn` 强制单路重开（kill-pane 单窗格）；命令面只见 agent 实例，六级原语绑在背后；S023 实测纠偏三处（internal-daemon 形态、conhost 兄弟、pane 无 shell 层） |
| 2026-09-01 | P0023 达成：看板资源包化——build.rs 打 tar.gz 嵌二进制、首启释放 `~/.ohmyagents/web/<指纹>/`（一次一份），serve 从释放位托管；单 exe 自带看板 JS 资源，产品化收口 |
| 2026-08-31 | P0022 达成：web 镜像本地化与主页化——前端源码仓发现（rmux-web-share/rmux-typescript）并 npm 构建本地托管（四挫四根因：尾斜杠、e 参数、WASM、ACAO）；session 镜像缺省加免 PIN；`oma serve` 主页即 web-mirror-server（打开即四路窗格），dashboard 删除、编排回归 CLI/API/MCP |
| 2026-08-31 | P0021 达成：官方 web 镜像集成——`oma web` 三面接管 rmux web-share（operator 真 attach、PIN、TTL、断开管理）；自建 xterm 桥下线（用户两次纠偏：要 TUI 镜像、用平台原生 webshare） |
| 2026-08-31 | P0019 达成：产品完备收口——SSE 终端镜像（render_stream 加首帧）、README/CHANGELOG/ROADMAP 对齐、**四家真 agent 全链验收全绿**（claude hook 流加编辑 trace、codex settle Skip、grok 干净项目直通、kimi Up+Enter 信任）；揪修四缺陷：status 逐路降级、spawn 清 CHILD_SESSION、settle 全屏三态、镜像首帧 |
| 2026-08-31 | P0018 达成：Windows 侧指令集检测落地（用户反问触发）——caps 模块（std 检测加退出码分类）进 doctor CPU 段与 agents 探针失败路径；本机实测 avx=true avx2=true avx512f=false；S021 追记 |
| 2026-08-31 | S021 落档：linux 预备检测研究——指令集 SIGILL 问题类（Bun 踩 AVX/AVX2、Rust 原生踩 AVX-512，两案核实到 issue 级）、四级检测阶梯、oma 探针落点 |
| 2026-08-31 | P0017 达成：Windows 全量收口——send 间隔产品化（等回显再 Enter，S005 铁律进产品路径）、HTTP trace 三端点加网页面板（三传输对齐）、SKILL.md 命令图生成（S016 末件）、grok 无头实跑（S007 回填，联邦 trace 同场检出）、`oma mcp --print-config` |
| 2026-08-31 | P0016 达成：REPL 落地——裸 `oma` 重连或拉起会话、编排面内嵌（7900 顺延 7909、--no-web/--open）、行循环分派（all/agent/status/web/quit）；stdin 线程喂 mpsc 保 serve 同活；顺手删 mcp 冗余 tool_router 字段并回归冒烟 |
| 2026-08-31 | P0015 达成：S016 吸收件收口——api::envelope 上提三传输共用、六会话命令 `--json`、status TTY 对齐表（非 TTY 恒 marker 保测试契约）、`oma completions`（clap_complete）、R002 输出规范节 |
| 2026-08-31 | P0014 达成：grok loader 主源切 updates.jsonl（S020 分类学先行——两流职责、hideFromScrollback 闸门、kind 判写族、信封秒逐事件真实时间）；chat_history 留旧会话兜底；本仓 8-29 历史 ts 逐秒散开验收 |
| 2026-08-31 | P0011 达成：三传输编排面当日闭环——切片 3 `oma mcp` stdio（六操作 + trace 三 tools、信封同形、orch 进度迁 stderr 保 stdout 纯协议）+ 切片 4 三通道共测（同 stub 项目 CLI/HTTP/MCP 各走 spawn→status→send→cleanup 全绿） |
| 2026-08-31 | P0011 切片 2 完成：网页可视化单页直出（状态卡、委派、SSE 画面）+ `/stream/{agent}` SSE 桥（tokio-stream 组合不自写 poll）；oldest 回放与未知路负例验收过 |
| 2026-08-31 | P0011 切片 1 完成：HTTP 编排面落地——api 传输无关层 + axum server（feature 隔离）、六操作 JSON 信封、会话锁串行；stub curl 全绿（含 400 与 ok:false 负例）；选型核实订正 rmcp 为 stable 3.1.4 |
| 2026-08-31 | P0013 达成：四家联邦检索全落地——grok/kimi loader 接完（源码核实纠三处偏）、codex 升 FileChange 双源、时间 epoch ms 归一；grok/kimi 真实历史检索命中；S019 落档；M034 记档 |
| 2026-08-31 | P0013 架构定案查询时联邦并首落 claude/codex：S019 本地实证四家会话库全破、`src\trace.rs` + `oma trace sessions\|timeline\|search` 活体验证（双意图自证、历史轮次回溯）；S007 无头缺口由 ohmypwsh 同机实测回填 |
| 2026-08-31 | 目标切到 P0013：S018 aitrace 研究落档（operation_id 归组、双意图、补账、裁决表八坑，七条断言抽查全中）；P0013 立项（五切片，补 agent 过滤与项目路径两缺口） |
| 2026-08-31 | P0012 达成：oma 自适应安装部署——catalog 两层 pin（出厂锚 + `~/.ohmyagents` 用户本地层写回）、渠道序 github 主 CDN 兜底、四家 Windows 装机全绿、update 取证闭环；S017 落档（含四家官方安装脚本逐家实证的渠道反转） |
| 2026-08-31 | S016 incurs 双层源码研究落档（吸收裁决表，三传输模式升核心）；P0011 立项（三通道编排加网页可视化，axum/rmcp 可选 feature 选型） |

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

- 当前目标：P0012 平台接管 mac 阶段（2026-09-01 用户三连定调：「准备去 wsl linux 下开发」→ 第一棒后「目前只到 wsl linux 就可以了」→「准备让 mac 接管开发」）——WSL Linux 第一棒收口（构建/基线/daemon/分类器/stub 全链绿），剩余两项挂起待回环境续做；mac 侧从远程拉取接续（交接读本 `docs\references\R010`）。

## 历史

> 所有已完成目标的轨迹，按日期倒序。

| 日期 | 目标 | 结果 |
| --- | --- | --- |
| 2026-09-01 | code review 修复（P0026） | 达成：codex review 高 5 中 7 全修；计划外修 serve 零控制台卡死、task id 撞号、canonicalize 时序双身份；看板默认只读 |
| 2026-09-01 | serve 守护化与协议化停机（P0025） | 达成：serve start 即调即退、stop 协议化优先（次轮补齐实测）；FFI 探活避 Job Object 管道死锁 |
| 2026-09-01 | agent 实例和解式编排（P0024） | 达成：spawn 三态和解、oma respawn 单路强制重开；S023 进程原语实测三纠偏 |
| 2026-08-31 | web 镜像本地化与主页化（P0022） | 达成：源码构建本地托管、session 免 PIN、主页即镜像、dashboard 下线；命名 web-mirror-server |
| 2026-08-31 | 官方 web 镜像集成（P0021） | 达成：oma web 三面接管 rmux web-share；自建 xterm 桥下线 |
| 2026-08-31 | 产品完备收口与四家真路验收（P0019） | 达成：SSE 终端镜像、门面文档对齐、四家真路全链全绿；修 status 降级、CHILD_SESSION、settle 三态 |
| 2026-08-31 | Windows 侧指令集检测落地（P0018） | 达成：caps 检测进 doctor、退出分类进 agents 探针与装机；本机 avx512f=false |
| 2026-08-31 | Windows 全量收口（P0017） | 达成：send 回显间隔、HTTP trace 三端点、SKILL 命令图、grok 无头、mcp 配置打印 |
| 2026-08-31 | REPL 与编排面内嵌（P0016） | 达成：裸 oma 进 REPL；内嵌编排面端口顺延；stub 管道驱动验收 |
| 2026-08-31 | S016 吸收件收口（P0015） | 达成：--json 信封三传输同形、TTY 表格、completions、R002 输出规范节 |
| 2026-08-31 | grok 权威日志升级（P0014） | 达成：updates 主源加 chat_history 兜底；逐事件真实时间；S020 分类学落档 |
| 2026-08-31 | 三传输编排面（P0011） | 达成：api 传输无关层一份核心三消费；`oma serve`（六操作 + 网页 + SSE）、`oma mcp`（九 tools）；同 stub 项目三通道共测全绿 |
| 2026-08-31 | agent 意图操作块与编辑轨迹检索（P0013） | 达成：四家联邦检索（oma trace 六视图）、双意图与 operation_id、真实历史与无头双验收 |
| 2026-08-31 | 自适应本机安装部署（P0012） | 达成：oma agents install/update、两层 pin 自维护、渠道序 github 主 CDN 兜底；Windows 四家装机全绿 |
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
