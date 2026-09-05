# 常用命令与管理流程：从项目 init 到会话 cleanup

> 角色：AGENTS 意图路由的细则载体，**命令面唯一权威**：行为细节、机理出处、marker 行、退出码、落地状态。
> 边界：协作规则在 AGENTS 二；文件与模块定位在 INDEX；规范禁令在 `docs\guide\`；输出冻结面见 `docs\references\R011-Agent友好IO契约-format三态信封退出码与冻结面.md`。
> 显示名 Oh My Agents；仓库 `ohmyagents-rs`；CLI 二进制 `oma`。运行时数据目录仍是 `.ohmyagents`。

已落地命令全表（设计命令全部落地 2026-08-31，增量至 2026-09-02）：`oma check`、`oma init`（全套）、`oma doctor`、`oma agents`（检测 / install / update / login / statusline / providers / secrets）、`oma self update`、`oma hook`（状态落盘加密钥拦截闸）、`oma spawn`、`oma respawn`、`oma status`、`oma send`、`oma key`、`oma run`、`oma task`（含 list / show）、`oma settle`、`oma trace` 六视图、`oma serve`（start / stop / status，HTTP 编排面）、`oma web`、`oma mcp`、REPL（裸 `oma`）、`oma completions`、六会话命令 `--json`。

## 一、环境与依赖

新终端若 PATH 缺新装工具，Windows 先重建：

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

`oma check` 会检测 rmux：版本必须是 pin（`catalog/rmux.toml`，现役 `0.10.0`），校验完整包布局与哈希；没有就按 GitHub 资产安装到本机数据目录。四路 agent 用 `oma agents` 扫 PATH、自定义路径和环境变量，不只看 PATH。版本读取用 `rmux -V`，不要 `--version`。

Windows pane 最小 POC 十四件（本机全表绿；Linux / mac 委托后续仓库）：

```powershell
cargo run --example poc-endpoint      # 专用 pipe 与 WMI 退路
cargo run --example poc-session       # CreateOnly / ReuseOnly / 只杀本 session
cargo run --example poc-layout        # 2x2 split_with + argv
cargo run --example poc-drive         # send_text 与 Enter 两段式
cargo run --example poc-dialogs       # hook 写 blocked + sendkeys 点掉
cargo run --example poc-paste         # 全 CLI -L 三段式中文粘贴
cargo run --example poc-locate        # pid 反查进程名，错位 throw
cargo run --example poc-stream        # output_stream Oldest 回放 Now 直播
cargo run --example poc-state         # terminal_state 分类，Quiet 不当 idle
cargo run --example poc-init          # 四家 hook/skill 项目级部署
cargo run --example poc-negatives     # C-c Codex 守卫与 daemon-wide kill 负检
cargo run --example poc-yolo-doctor   # 项目级 yolo 键与只读诊断
cargo run --example poc-label-bridge  # CLI 起 label daemon 后 #{socket_path} 桥
cargo run --example poc-dump          # 备屏诊断
```

宿主若在 Job Object 内，`connect_or_start` 会 os error 5；example 与产品命令自动用 WMI 在 job 外拉起 daemon。结束只 `kill-session`。

## 二、命令族

### 2.1 自举与诊断

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 核对依赖 | `oma check` | 检测 rmux pin 版本与哈希；缺则安装完整包；打印路径 / 版本 / sha256 |
| 只诊断 rmux | `oma check --no-install` | 缺失或哈希 / 版本不符则非 0，不下载 |
| 无阻塞诊断 | `oma doctor [--project PATH]` | 只读体检：yolo 键、信任库、已装二进制、`.ohmyagents/state`、CPU 能力段（avx / avx2 / avx512f 三布尔，S021）、部署面四类（2026-09-02）：登录态（grok `~/.grok/auth.json` RFC3339 加 create_time 加 30 天兜底加 300s 提前量；kimi credentials `hasToken` 加空串墓碑，S026）、hook 形态（`hooks.form` bare / absolute；codex per-OS 字段 command / commandWindows）、状态栏（四家 oma bar 配置标记加脚本在位加 pwsh 咨询，S025）、会话健康（manifest 在才探 daemon 活性，无 manifest 不误报）。不 attach；`status=warn` 是部署缺口不计数，任一项 `status=block` 才退出 1 |

### 2.2 agent 安装与登录

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 检测已装 agent | `oma agents` | 扫 PATH、`OMA_AGENT_PATH`、`OMA_<AGENT>_BIN`、oma 自管根 `~/.ohmyagents/agents`、各家默认目录（Windows / Linux / macOS）；打印 `source=env\|path\|oma\|default` 与 version。缺装不退出非 0，缺装行带 `hint=oma agents install <名>` |
| 安装缺失 agent | `oma agents install [名…] [--force] [--root PATH]` | 自适应：已装（任何来源）跳过只补缺；`--force` 重装。按 catalog pin 走渠道序（github 主 CDN 兜底）下载并 sha256 校验，解包落 oma 自管根（缺省 `~/.ohmyagents/agents/<名>/<版本>/`，`OMA_HOME` 或 `--root` 覆盖），leaf 名找二进制、写 manifest、装后 `--version` 探针。pin 源 `catalog/agents.toml`。三平台实测四家全绿（Windows / mac / WSL Linux；mac 侧 grok 双 CDN 补 macos-aarch64 pin，Linux 侧 codex 嵌套 bin 布局） |
| 设备码登录引导 | `oma agents login <grok\|kimi> [--timeout N]` | claude / codex 走各自原生登录。起 `grok login --device-code` / `kimi login` 子进程：只抽 `login.url=` / `login.code=` 机读标记干净输出，不转发原始 stderr（设备码流天生跨机：URL 加 code 拿到任何机器完成，用户定调 2026-09-02）；等浏览器侧完成（缺省 600s、0 不限时，超时杀进程）；成功判据 = 退出 0 且 doctor 登录态判据过（落盘凭据为准，不单信成功标记）；失败带 `login_state=` 与尾部诊断行 |
| 升级与 pin 维护 | `oma agents update [名…] [--force] [--root PATH]` | 解析最新版（github `releases/latest`、grok `x.ai/cli/stable`、kimi CDN `latest`），取证新 sha（github `assets[].digest` 优先、SUMS 清单与边车兜底、kimi CDN manifest、grok 下载自算），升级 oma 自管安装并把 pin 写回用户本地层 `~/.ohmyagents/catalog/agents.toml`（删该文件重置出厂锚）。已最新报 uptodate；取证不全则整体失败保旧 pin |
| 提供商别名注入 | `oma agents providers [--example]` | 别名簿 `~/.ohmyagents/providers.toml`（标准 sops 托管可加密）。`oma spawn --agents claude@zhipu,codex@deepseek` 按 `agent@alias` 注入该路 env / argv：claude 走 `ANTHROPIC_*` env、codex 走 `-c` 运行时覆写；别名沿 manifest 进 respawn / 和解。官方四格矩阵见 S027 |
| 配置状态栏 | `oma agents statusline [名]` | 四家写入面（S025 矩阵）：claude settings.json 幂等合并 statusLine 块；codex config.toml `[tui] status_line` 整段替换；kimi tui.toml `[status_line].command`；grok config.toml `[ui.status_line] type=command`。pwsh 脚本释放 oma 数据根 statusline/，脚本内强制 UTF-8 输出防 CP936 下图标变问号；pwsh 未检出打 `statusline.pwsh=missing` 警告 |

### 2.3 密钥与 hook

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 密钥管理 | `oma agents secrets init\|set <KEY>\|env --shell <pwsh\|bash\|zsh\|nu>\|inject\|status` | 一钥两密文加四 shell 懒注入（S031，对齐 ohmycloud D20 与 ohmypwsh 懒注入）。oma 自管根落 `app.key`（32B、0600 原子写）、`identity.enc`（AES-256-GCM 包裹 SOPS 标准 age 身份，`oma:v1:` 标记）、`secrets.yaml`（SOPS 制密文，sops 二进制加工、值 base64）。解密链全程内存 app.key 到 identity.enc 到 SOPS_AGE_KEY 到 vault；`inject` 向四 shell profile 写标志行包裹的懒注入块（幂等），交互 shell 启动现场解密只写会话 env，明文不常驻注册表。秘密不进 argv（set 走 stdin）、输出 redacted |
| hook 写状态加密钥拦截 | `oma hook [event]` | 双职责。其一状态落盘：各家 hook 的 `command`，读 stdin JSON（`hook_event_name` / `hookEventName`）或参数，写 `OHMYAGENTS_STATE_FILE`；缺该环境变量则 exit 0；不连 rmux。其二密钥拦截闸（S030）：PreToolUse / UserPromptSubmit 命中 block 级密钥则 exit 2 拒工具调用（stderr 掩码原因回给模型），PostToolUse 只观察。八层防误报：精确前缀、实值比对（env 加 providers.toml 明文）、熵值门、stopwords、语料拼接豁免、password warn-only、日志掩码、fail-open |

### 2.4 项目部署

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 部署项目全套 | `oma init [--project PATH]` | yolo 键加 hook / skill 部署：`.claude/settings.json`（yolo 加 hooks exec form）、`.codex/hooks.json` 加 `config.toml`（yolo 加 features.hooks）、`.grok/hooks/ohmyagents-state.json`、四家 skill 目录、AGENTS / CLAUDE.md（仅缺失时）。幂等合并保留外条目，不改家目录。四环境自适应（P0027，矩阵见 S024）：claude / grok 探针命中 PATH 写 bare `oma`（粘性不降级）；codex 按字段所有权各侧只写本侧（`command` / `commandWindows`）；共享项目目录 Windows 与 WSL 双侧并存不互踢；输出 `init.hooks.form=` 标记形态 |
| 部署项目级 yolo | `oma init --yolo [--project PATH]` | 仅无阻塞键：`.claude/settings.json`（`defaultMode=bypassPermissions`）、`.claude/settings.local.json`（顶层 `skipDangerousModePermissionPrompt`）、`.codex/config.toml`（sandbox / approval）、`.kimi-code/config.toml`（`yolo`）。不部署 hook / skill |
| 预写信任库 | `oma init --yolo --pretrust [--project PATH]` | 额外写用户家：claude.json trust、codex projects、kimi workspace-trust、grok trusted_folders；grok 的 `permission_mode` 只能写 `~/.grok/config.toml` |
| 权限模式 | `oma init --permission-mode auto\|yolo\|manual` | 覆盖默认 yolo；manual 不写 bypass |

### 2.5 会话编排

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 和解拉起 | `oma spawn [--agents a,b] [--stub] [--project PATH] [--json]` | 和解式（P0024）：会话不在新开；在则逐 agent 判活，活路附加、死路重开（`spawn.attached=` / `spawn.respawned=` / `spawn.mode=new\|reconcile`）；先补后收防空会话：给几路就几路，多余路自动收掉出 `removed`；布局按路数自适应（1 全屏、2 / 3 左右列分、4 路 tiled 2x2）；命令面只见 agent 实例，服务 / 会话 / 窗格复杂性绑在背后。claude 路清 `CLAUDE_CODE_CHILD_SESSION` 并强开 session 持久化（P0019）、固定 `--dangerously-skip-permissions`（S029：flag 优先于 settings 各层，2.1.257 起项目层 bypass 被忽略）。拉起后自动跑一轮信任框 settle（白名单只碰信任 / 升级屏，任务级确认永不自动按）；新拉路等待就绪（idle 稳定 / working / 画面变化任一，20s），有阻塞或死路打 `spawn.alert=`。不阻塞返回；已存在则拒绝叠格 |
| 桩会话 | `oma spawn --stub [--agents a,b]` | 用 shell 桩替代真实 agent（验收与调试） |
| 重开一路 | `oma respawn <agent> [--project PATH] [--json]` | 强制关闭再打开该 agent 实例（kill-pane 只打该窗格，不动会话与其它路）；manifest 回写新 pane id；按 manifest 别名与 argv 复读（respawn 自动吃到 spawn 时注入的提供商配置） |
| 看状态 | `oma status [--project PATH]` | 层 0 pid 加 locate 进程名、层 1b 终端态、层 2 hook 态、扫屏层：状态栏 `agent:state` 机读标记加 hook 交叉核对 `check=match\|mismatch\|-`（S025 消费面）。双读者：TTY 打对齐表格（AGENT / PID / PROCESS / TERMINAL / HOOK / SCREEN / CHECK），管道与测试打 marker 行；一路 pane 消失该路报 `terminal=dead` 其余照报（P0019） |
| 自愈信任 | `oma settle [--wait N] [--project PATH]` | 轮询各路画面（SDK snapshot 全屏匹配，P0019），白名单按 (marker, key 序列) 三态自动确认：claude 工作区信任 Enter、kimi 文件夹信任 Up+Enter（默认焦点在不信任项）、codex 升级提示 2+Enter（Skip）；密码类与用户级 hook 审查永不自动 |
| 收尾 | `oma cleanup [--project PATH]` | 只杀本项目会话并清 manifest；不 kill-server，daemon 随末 session 自然退 |

### 2.6 委派与任务

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 发任务 | `oma send <agent> "<text>" [--confirm MARKER] [--project PATH]` | 守卫链（键策略、locate 进程名）后：单行走 SDK `send_text` 与 Enter 两段式；多行（含换行）走三段式粘贴（临时文件加 CLI `load-buffer` 加 `paste-buffer -p -t %<pane_id>`，Enter 仍单独发，中文可用）；`--confirm` 等短头可见。任务开始确认：Enter 后等该路真开始（working / 画面变化双信号，15s），blocked 或未启动打 `send.alert=` |
| 发单键 | `oma key <agent> <KEY>` | 单键受守卫入口：codex 拒 `C-c`（一个 C-c 杀进程，打断 codex 用 Esc） |
| 委派任务 | `oma run "<文本>" [--assign a,b] [--confirm MARKER] [--project PATH]` | 状态门分派（层 2 有则用，沉默走 1b，仅 idle 过）：一路 blocked / busy 跳过并报告不堵其它路；发出路写 `.ohmyagents/tasks/tNNN.json`（id 递增，assigned 记实际发出路与时间戳）；多行文本走三段式；全拦退出 1 |
| 带产物等待的任务 | `oma task <agent> "<文本>" [--timeout N]` | 学 reader_rs 形态（2026-09-01）：建 `.ohmyagents/tasks/<id>/`（prompt.md 提示词全文），send 带协议尾注，随后阻塞等 DONE 标记（agent 写 output.md 后最后创建 DONE，只认 DONE 防半写；缺省 600s、0 无限）后打产物退出。`oma task list` 与 `oma task show <id>` 查清单与产物。SKILL 部署带任务目录协议（`oma init` 重跑同步）；仓根 `SKILL.md` 为 agent 技能文档 |

Drive 铁律（S005，三段式粘贴）：发前扫框、`paste-buffer -p`、Enter 单独发且与文本间隔；禁止文本和 Enter 同发、对 Codex 发 `C-c`、发送侧自包 `\x1b[200~`。

### 2.7 轨迹检索

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 检索会话 | `oma trace sessions [--project PATH]` | 查询时联邦读四家原生会话库（claude projects 目录、codex rollout、grok sessions、kimi session_index），列项目内各 agent 会话 |
| 检索编辑轨迹 | `oma trace timeline [--agent A] [--file GLOB] [--limit N] [--project PATH]` | 意图操作块元素视图：每条编辑事件带 operation_id（session:call）、kind、tool、ts 与双意图（intent=用户请求、op_intent=assistant 声明）；分页 clamp 1-1000。四家全量：claude（Edit / Write）、codex（FileChange 主源加 apply_patch 兜底）、grok（updates.jsonl 权威主源加 chat_history 兜底，S020，逐事件真实时间）、kimi（loop tool.call） |
| 检索操作块 | `oma trace blocks [--agent A] [--limit N] [--project PATH]` | 一个 operation_id 一块（一次工具调用可能多文件），时间正序取最新 N 块，聚合 edits / files / kinds / 双意图 |
| 检索 agent 轨迹 | `oma trace agent <名> [--limit N] [--project PATH]` | 某家 agent 的操作块时间线（名不在四家内退出非 0） |
| 检索单文件轨迹 | `oma trace file <相对路径\|glob> [--agent A] [--limit N] [--project PATH]` | 文件维度：该文件被哪些 agent、何时、基于什么意图改过（创建 / 修改 / 删除），时间正序 |
| 检索关键词 | `oma trace search <query> [--agent A] [--limit N] [--project PATH]` | 正则匹配 patch、file、双意图四域，非法正则退字面子串；先全量匹配后截断；输出元素命中数与匹配块数两个粒度 |

### 2.8 传输面与自维护

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 开会话（REPL） | `oma [--stub] [--agents a,b] [--no-web] [--open]` | 裸调用进 REPL（P0016）：会话已在则重连（不叠格），否则缺省拉起（不阻塞）；默认内嵌 HTTP 编排面（7900 被占顺延到 7909）并打印 URL；`--no-web` 不起、`--open` 才开浏览器（失败只警告）。行命令：`all <prompt>`（状态门分派）、`<agent> <prompt>`（单路发送，多行走三段式）、`status`（表格）、`web`（复述 URL）、`quit` / `exit` / EOF（只 detach，拆会话用 cleanup）；空行忽略，裸 `all` / 裸 agent 名与未知命令给提示不崩 |
| 起 web 镜像 | `oma web [agent] [--spectator] [--ttl N] [--no-pin] [--project PATH]` | rmux web-share 接管（P0021 / P0022）：缺省整会话镜像（一个 URL 全窗格、operator 可编辑、带分屏控制）；给 agent 则单 pane；官方域前端走公网中继（E2EE 加密、PIN 防外发），`--no-pin` 显式关且与官方域叠加时打 `web.warning=PUBLIC-RELAY-NO-PIN` 显著警示；serve 起的走本地前端（127.0.0.1）自动免 PIN。URL / PIN / 过期打 marker 行（PIN 等同键盘权限勿外传） |
| 后台起编排面 | `oma serve start [--port 7900] [--project PATH]` | 即调即退后台守护（CREATE_NO_WINDOW，已活秒回地址；P0025）。六操作 RESTish：`POST /spawn`、`GET /status`、`POST /send`、`POST /run`、`POST /settle`、`DELETE /session`；`GET /` 即 web-mirror-server 主页：自动起整会话镜像（operator、本地免 PIN、12h）注入 token，打开就是多路窗格可打字可分屏（P0022）；前端资源包随二进制走（build.rs 打 tar.gz 嵌入、首启释放 `~/.ohmyagents/web/<指纹>/`，P0023）；`GET /api` 端点自述；`GET /stream/{agent}?from=oldest\|now` pane 行日志 SSE；`GET /screen/{agent}` 终端镜像 SSE（render_stream surface 投影，全屏纯文本无 ANSI，含首帧，P0019）；`GET /trace/sessions\|timeline\|search` 轨迹三端点；`POST /share` / `POST /share/{agent}` / `GET /share` / `DELETE /share/{id}/stop` web 镜像管理。JSON 信封；业务失败 200 加 `ok:false`，坏 JSON 400；只绑 127.0.0.1，写操作会话锁串行；Ctrl-C 只停 serve 不清会话。需 `--features server` 构建，缺 feature 报错退出 1 |
| 停 / 查编排面 | `oma serve stop [--project PATH]` / `oma serve status [--project PATH]` | stop 走协议化停机（`DELETE /shutdown` 优雅排空，超时 taskkill 兜底）；status 报 pid / port / live。裸 `oma serve` 保留前台调试 |
| 起 MCP server | `oma mcp [--project PATH] [--print-config]` | stdio 传输（无网络面，需 `--features mcp`），九 tools：六操作（oma_spawn / oma_status / oma_send / oma_run / oma_settle / oma_cleanup）加 trace 三件（oma_trace_sessions / oma_trace_timeline / oma_trace_search）。返回信封与 HTTP 同形；stdout 是 JSON-RPC 通道，进度只进 stderr。`--print-config` 打印各客户端注册片段（Claude Code、codex、通用 mcpServers 三形态；任何构建可用） |
| oma 自更新 | `oma self update [--stable] [--repo owner/name] [--git] [--force]` | 缺省 dev 滚动源：CI 每推 main 构建测试后覆盖发布的 prerelease，按资产 sha256 判新；`--stable` 走正式版（v* tag 触发构建）；Windows rename 舞步自替换；无 release 体面降级 `--git` 源码安装。机制见 S028 |
| 生成补全 | `oma completions <shell>` | clap_complete 出 bash / zsh / fish / powershell 等补全脚本到 stdout（如 `oma completions powershell >> $PROFILE` 用法自取） |

## 三、输出契约

> S016 吸收裁决表的落点。双读者三轨，错误一律带下一步。

- **marker 行（机器面，默认）**：`命令.键=值` 行式输出进 stdout，管道与测试消费；断言只押 marker 行与退出码。
- **TTY 表格（人读面）**：stdout 是 TTY 时 `oma status` 打对齐表格（手写 formatter，无 toon 依赖）；同一份数据两副面孔，测试跑在非 TTY 下天然走 marker。
- **`--format kv|json|jsonl` 与 `--json` 简写（互斥）**：kv 是 marker 行缺省；json 出 `{ok, data\|error, meta:{command, project}}` 信封与 HTTP / MCP 三传输同形（P0015，api 层一份信封三消费）；jsonl 列表逐行对象。值一律字符串、字段序与 kv 行序一致（preserve_order）；结构化错误 stderr 单行 JSON、业务失败信封仍进 stdout 退出非 0，机器读者拿信封、人类拿 stderr 错误行。机器面冻结命令与退出码表见 `docs\references\R011-Agent友好IO契约-format三态信封退出码与冻结面.md`（issue #1）。
- **错误 CTA**：用户可见错误自带下一步（如 `no session manifest; run oma spawn first`）；新增错误路径保持同款，禁止裸报错。
- **`--version`**：oma 自身版本（clap 标准，源 `Cargo.toml`；也是 ome catalog 集成本仓条目的探测条件之一，D06 切片 1 复核）。与 agent 装后探针的 `--version`（各家 agent 二进制）是两回事。

## 四、维护规则

- **新增命令同步链**：`src/deploy.rs` 的 COMMAND_MAP 加行，加 `cargo test deploy`，加重跑 `oma init` 重生 SKILL，加本文件对应族加行，加 AGENTS 三节加行；缺一即登记债。
- **落地状态标注**：`[实证: 日期 + 验收面]`；设计口径必须写明「设计口径」并挂 S 编号，禁止把未验收写成已落地。
- **六态标注口径**：行内合法形态 `[实证: 来源]` 与 `依据 docs\research\S0NN`；`[推断]` / `[假设]` 只许出现在 research（G004），本文件不许出现。
- **权威边界**：一句话进 AGENTS 三节，行为细节进本文件，文件定位进 INDEX，规则进 G 系；协作规则与文档检查、提交规范不在本文件重复（权威在 AGENTS 二与 G001）。
