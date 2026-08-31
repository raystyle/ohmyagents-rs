# 常用命令与管理流程：从项目 init 到会话 cleanup

> AGENTS 意图路由的细则。已落地：`oma check`、`oma init`（全套）、`oma doctor`、`oma agents`（含 install/update）、`oma hook`、`oma spawn`、`oma status`、`oma send`、`oma cleanup`、`oma run`、`oma settle`、`oma trace` 六视图、`oma serve`（HTTP 编排面，2026-08-31 实测）。REPL 与网页可视化仍是设计口径。
> 显示名 Oh My Agents；仓库 `ohmyagents`；CLI 二进制 `oma`（对照 ohmypwsh 的 `omp`）。运行时数据目录仍是 `.ohmyagents`。

## 环境

新终端若 PATH 缺新装工具，Windows 先重建：

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

`oma check` 会检测 rmux：版本必须是 pin（`catalog/rmux.toml`，现役 `0.10.0`），校验完整包布局与哈希；没有就按 GitHub 资产安装到本机数据目录。四路 agent 用 `oma agents` 扫 PATH、自定义路径和环境变量，不只看 PATH。版本读取用 `rmux -V`，不要 `--version`。

Windows pane 最小 POC 十二件（本机全表绿；Linux/mac 后续委托）：

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
```

宿主若在 Job Object 内，`connect_or_start` 会 os error 5；example 与产品命令自动用 WMI 在 job 外拉起 daemon。结束只 `kill-session`。

## 命令

| 意图 | 命令 | 说明 |
| --- | --- | --- |
| 核对依赖 | `oma check` | 检测 rmux pin 版本与哈希；缺则安装完整包；打印路径/版本/sha256 |
| 只诊断 rmux | `oma check --no-install` | 缺失或哈希/版本不符则非 0，不下载 |
| 无阻塞诊断 | `oma doctor [--project PATH]` | 只读 yolo 键、信任库、已装二进制、`.ohmyagents/state`；不 attach。任一项 `status=block` 则退出 1 |
| 检测已装 agent | `oma agents` | 扫 PATH、`OMA_AGENT_PATH`、`OMA_<AGENT>_BIN`、oma 自管根、各家默认目录；打印 `source=env|path|oma|default` 与 version。缺装不退出非 0，缺装行带 `hint=oma agents install <名>` |
| 安装缺失 agent | `oma agents install [名…] [--force] [--root PATH]` | 自适应：已装（任何来源）跳过，只补缺；`--force` 重装。按 catalog pin 走渠道序（github 默认、CDN 兜底）下载并 sha256 校验，解包落 oma 自管根（缺省 `~/.ohmyagents/agents/<名>/<版本>/`，`OMA_HOME` 或 `--root` 覆盖），leaf 名找二进制、写 manifest、装后 `--version` 探针。pin 源 `catalog\agents.toml`（信任锚是文件哈希） |
| 升级与 pin 维护 | `oma agents update [名…] [--force] [--root PATH]` | 解析最新版（github `releases/latest`、grok `x.ai/cli/stable`、kimi CDN `latest`），取证新 sha（github `assets[].digest` 优先、SUMS 清单与边车兜底、kimi CDN manifest、grok 下载自算），升级 oma 自管安装并把 pin **写回用户本地层** `~/.ohmyagents\catalog\agents.toml`（删该文件重置出厂锚）。已最新报 uptodate；取证不全则整体失败保旧 pin |
| hook 写状态 | `oma hook [event]` | 各家 hook 的 `command`。读 stdin JSON（`hook_event_name` / `hookEventName`）或参数。写 `OHMYAGENTS_STATE_FILE`。无该环境变量则 exit 0。不连 rmux |
| 部署项目全套 | `oma init [--project PATH]` | yolo 键加 hook/skill 部署：`.claude/settings.json`（yolo 加 hooks exec form）、`.codex/hooks.json` 加 `config.toml`（yolo 加 features.hooks）、`.grok/hooks/ohmyagents-state.json`、四家 skill 目录、AGENTS/CLAUDE.md（仅缺失时）。幂等合并保留外条目，不改家目录 |
| 部署项目级 yolo | `oma init --yolo [--project PATH]` | 仅无阻塞键：`.claude/settings.json`（`defaultMode=bypassPermissions`）、`.claude/settings.local.json`（顶层 `skipDangerousModePermissionPrompt`）、`.codex/config.toml`（sandbox/approval）、`.kimi-code/config.toml`（`yolo`）。不部署 hook/skill |
| 预写信任库 | `oma init --yolo --pretrust [--project PATH]` | 额外写用户家：claude.json trust、codex projects、kimi workspace-trust、grok trusted_folders；grok 的 `permission_mode` 只能写 `~/.grok/config.toml` |
| 权限模式 | `oma init --permission-mode auto\|yolo\|manual` | 覆盖默认 yolo；manual 不写 bypass（设计口径） |
| 拉起会话 | `oma spawn [--agents a,b] [--stub] [--project PATH]` | 项目专属会话（`oma-<slug>`）里按布局拉 1-4 路 agent，缺省取已装交集；注入 `OHMYAGENTS_PROJECT/AGENT/STATE_FILE`；不阻塞返回；已存在则拒绝叠格 |
| 桩会话 | `oma spawn --stub [--agents a,b]` | 用 shell 桩替代真实 agent（验收与调试） |
| 看状态 | `oma status [--project PATH]` | 只读列各路 pid、进程名（locate）、终端态（1b 分类）、hook 态（层 2，沉默标 silent）；不 attach |
| 发任务 | `oma send <agent> "<text>" [--confirm MARKER] [--project PATH]` | 守卫链（键策略、locate 进程名）后：单行走 SDK `send_text` 与 Enter 两段式；多行（含换行）走三段式粘贴（临时文件 + CLI `load-buffer` + `paste-buffer -p -t %<pane_id>`，Enter 仍单独发，中文可用）；`--confirm` 等短头可见 |
| 收尾 | `oma cleanup [--project PATH]` | 只杀本项目会话并清 manifest；不 kill-server，daemon 随末 session 自然退 |
| 自愈信任 | `oma settle [--wait N] [--project PATH]` | 轮询各路画面（SDK snapshot），白名单匹配信任/审查框自动确认（claude 工作区信任 Enter、codex 审查 Trust all）；密码类永不自动 |
| 开会话（REPL） | `oma` | spawn 默认不阻塞 CLI + 打印 URL + REPL；不自动打开浏览器（设计口径） |
| 无网页 | `oma --no-web` | 不起 HTTP（设计口径） |
| 尝试打开浏览器 | `oma --open` | opener 失败只警告（设计口径） |
| 委派任务 | `oma run "<文本>" [--assign a,b] [--confirm MARKER] [--project PATH]` | 状态门分派（层 2 有则用，沉默走 1b，仅 idle 过）：一路 blocked/busy 跳过并报告不堵其它路；发出路写 `.ohmyagents\tasks\tNNN.json`（id 递增，assigned 记实际发出路与时间戳）；多行文本走三段式；全拦退出 1 |
| 检索会话 | `oma trace sessions [--project PATH]` | 查询时联邦读四家原生会话库（claude projects 目录、codex rollout、grok sessions、kimi session_index），列项目内各 agent 会话 |
| 检索编辑轨迹 | `oma trace timeline [--agent A] [--file GLOB] [--limit N] [--project PATH]` | 意图操作块（元素视图）：每条编辑事件带 operation_id（session:call）、kind、tool、ts 与双意图（intent=用户请求、op_intent=assistant 声明）；分页 clamp 1-1000。四家全量：claude（Edit/Write）、codex（FileChange 主源加 apply_patch 兜底）、grok（tool_calls 写文件族）、kimi（loop tool.call） |
| 检索操作块 | `oma trace blocks [--agent A] [--limit N] [--project PATH]` | 操作块时间线：一个 operation_id 一块（一次工具调用可能多文件），时间正序取最新 N 块，聚合 edits/files/kinds/双意图 |
| 检索 agent 轨迹 | `oma trace agent <名> [--limit N] [--project PATH]` | 某家 agent 的操作块时间线（名不在四家内退出非 0） |
| 检索单文件轨迹 | `oma trace file <相对路径\|glob> [--agent A] [--limit N] [--project PATH]` | 文件维度：该文件被哪些 agent、何时、基于什么意图改过（创建/修改/删除），时间正序 |
| 检索关键词 | `oma trace search <query> [--agent A] [--limit N] [--project PATH]` | 正则匹配 patch、file、双意图四域，非法正则退字面子串；先全量匹配后截断；输出元素命中数与匹配块数两个粒度 |
| 起 HTTP 编排面 | `oma serve [--port 7900] [--project PATH]` | 六操作 RESTish：`POST /spawn`（body `{"agents":["a"],"stub":false}`）、`GET /status`、`POST /send`、`POST /run`、`POST /settle`、`DELETE /session`；`GET /` 直出可视化网页（`docs\web\index.html` 单页：状态卡、委派按钮、SSE 画面）；`GET /api` 端点自述；`GET /stream/{agent}?from=oldest\|now` pane 输出 SSE（`open` 事件带 pane_id，回放积压用 oldest）。JSON 信封 `{ok, data\|error, meta:{command, project}}`；业务失败 200 加 `ok:false`，坏 JSON 400；只绑 127.0.0.1，写操作会话锁串行（一次一命令）；Ctrl-C 只停 serve 不清会话。需 `--features server` 构建，缺 feature 报错退出 1 |
| 起 MCP server | `oma mcp [--project PATH]` | stdio 传输（无网络面），九 tools：六操作（oma_spawn/oma_status/oma_send/oma_run/oma_settle/oma_cleanup）加 trace 检索（oma_trace_sessions/oma_trace_timeline/oma_trace_search）。返回信封与 HTTP 同形（`structured`/`structured_error`，业务失败 caller 可见）；stdout 是 JSON-RPC 通道，进度只进 stderr。需 `--features mcp` 构建。MCP 客户端配置示例：`oma mcp --project D:\path\to\proj` |

## REPL

```text
> all <prompt>
> claude|codex|grok|kimi <prompt>
> status
> web
> quit
```

`quit` 只 detach。拆会话用 `cleanup`。

## 文档检查

```powershell
rumdl check .
```

研究与测试文档的事实性断言还要标六态（实证 / 推断 / 经验 / 记忆 / 假设 / 直觉），标准见 `docs\guide\G002-研究标准细则-结构与六态标记.md`。`rumdl` 不检查六态，写作者自查。

## 提交

一次一件事：`feat:` / `docs:` / `fix:` / `chore:` + 中文。未经指示不推远端。
