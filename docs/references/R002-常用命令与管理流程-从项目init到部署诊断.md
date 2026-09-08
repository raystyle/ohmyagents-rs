# 常用命令与管理流程：从项目 init 到部署诊断

> 角色：AGENTS 意图路由的细则载体，**命令面唯一权威**：行为细节、机理出处、marker 行、退出码、落地状态。
> 边界：协作规则在 AGENTS 二；文件与模块定位在 INDEX；规范禁令在 `docs\guide\`；输出冻结面见 `docs\references\R011-Agent友好IO契约-format三态信封退出码与冻结面.md`。
> 显示名 Oh My Agents；仓库 `ohmyagents-rs`；CLI 二进制 `oma`。运行时数据目录是 `.oma`（D14；旧名 `.ohmyagents` 仅旧在则改名迁过去）。

已落地命令全表（D15 收窄 2026-09-08，D19 恢复 trace）：`oma init`（全套）、`oma doctor`、`oma agents`（检测 / install / update / login / statusline / providers / secrets）、`oma hook`（状态落盘加密钥拦截闸）、`oma self update`、`oma completions`、`oma trace` 六视图（只读检索，与 rmux 无耦合）、全局 `--format` 加 `--json`。编排命令（`oma check` / `spawn` / `respawn` / `status` / `send` / `key` / `run` / `task` / `settle` / `cleanup` / REPL / `web` / `serve` / `mcp`）已随 D15 移除，历史口径见归档 P0001 至 P0034 与 git 历史。

## 一、环境与依赖

新终端若 PATH 缺新装工具，Windows 先重建：

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

四路 agent 用 `oma agents` 扫 PATH、自定义路径和环境变量，不只看 PATH。oma 无外部运行时依赖（D15 起 rmux 后端移除，`oma check` 与 `catalog\rmux.toml` 同步删除；rmux 历史 pin 口径见 P0003 与 git 历史）。

## 二、命令族

### 2.1 诊断

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 无阻塞诊断 | `oma doctor [--project PATH]` | 只读体检：yolo 键、信任库、已装二进制、`.oma/state`、CPU 能力段（avx / avx2 / avx512f 三布尔，S021）、agents 域三类（D07 归类 2026-09-07，原部署面四类 2026-09-02；会话健康类随 D15 移除）：登录态（grok `~/.grok/auth.json` RFC3339 加 create_time 加 30 天兜底加 300s 提前量；kimi credentials `hasToken` 加空串墓碑，S026）、hook 形态（`hooks.form` bare / absolute / args；args 为 command+args 数组，Grok 经 PowerShell 会 ParserError，M047；codex per-OS 字段 command / commandWindows）、codex `trust.hooks` 读项目 `.codex/config.toml` 的 `[hooks.state]`（不拿用户 store 里 `~/.codex/hooks.json` leftover 顶空表，M044）、状态栏（四家 oma bar 配置标记加脚本在位加 pwsh 咨询，S025；Codex 内置项 ID 才 ok、command-argv 标 warn，M045；Grok Windows 只认 `oma-statusline-grok.cmd` 单路径，`pwsh -File` 壳行标 warn，M048）。二进制在位与版本、token 可用性诊断归 ome doctor agent 层（D07 分工，ohmyagents#5）；oma doctor 保留 binary 在位探查作部署判据。`status=warn` 是部署缺口不计数，任一项 `status=block` 才退出 1 |

### 2.2 agent 安装与登录

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 检测已装 agent | `oma agents` | 扫 PATH、`OMA_AGENT_PATH`、`OMA_<AGENT>_BIN`、oma 自管根 `~/.oma/agents`、各家默认目录（Windows / Linux / macOS）；打印 `source=env\|path\|oma\|default` 与 version。缺装不退出非 0，缺装行带 `hint=oma agents install <名>` |
| 安装缺失 agent | `oma agents install [名…] [--force] [--root PATH]` | 【deprecated（D07 迁册 2026-09-07，ohmyagents#5）】二进制下装部署已迁 ohmyenv-rs：请用 `ome install <agent>`（数据权威 ome `catalog\tools.toml` agent 四节）；本命令保留兼容，入口先打 stderr 提示 `oma.deprecated`。原行为：自适应：已装（任何来源）跳过只补缺；`--force` 重装。按 catalog pin 走渠道序（github 主 CDN 兜底）下载并 sha256 校验，解包落 oma 自管根（缺省 `~/.oma/agents/<名>/<版本>/`，`OMA_HOME` 或 `--root` 覆盖），leaf 名找二进制、写 manifest、装后 `--version` 探针。pin 源 `catalog/agents.toml`（已冻结历史锚，不再随上游滚动）。三平台实测四家全绿（Windows / mac / WSL Linux；mac 侧 grok 双 CDN 补 macos-aarch64 pin，Linux 侧 codex 嵌套 bin 布局） |
| 设备码登录引导 | `oma agents login <grok\|kimi> [--timeout N]` | claude / codex 走各自原生登录。起 `grok login --device-code` / `kimi login` 子进程：只抽 `login.url=` / `login.code=` 机读标记干净输出，不转发原始 stderr（设备码流天生跨机：URL 加 code 拿到任何机器完成，用户定调 2026-09-02）；等浏览器侧完成（缺省 600s、0 不限时，超时杀进程）；成功判据 = 退出 0 且 doctor 登录态判据过（落盘凭据为准，不单信成功标记）；失败带 `login_state=` 与尾部诊断行 |
| 升级与 pin 维护 | `oma agents update [名…] [--force] [--root PATH]` | 【deprecated（D07 迁册 2026-09-07）】agent 升级归 ome（升级通道语义由 ome 裁决，ohmyagents#2 余项）；本命令保留兼容，入口先打 stderr 提示 `oma.deprecated`。原行为：解析最新版（github `releases/latest`、grok `x.ai/cli/stable`、kimi CDN `latest`），取证新 sha（github `assets[].digest` 优先、SUMS 清单与边车兜底、kimi CDN manifest、grok 下载自算），升级 oma 自管安装并把 pin 写回用户本地层 `~/.oma/catalog/agents.toml`（删该文件重置出厂锚）。已最新报 uptodate；取证不全则整体失败保旧 pin |
| 提供商别名注入 | `oma agents providers [--example]` | 别名簿 `~/.oma/providers.toml`（标准 sops 托管可加密）。按 `agent@alias` 记该路 env / argv 注入形态：claude 走 `ANTHROPIC_*` env、codex 走 `-c` 运行时覆写；官方四格矩阵见 S027。注入消费面（`oma spawn`）已随 D15 移除，别名簿保留为配置面，供外部拉起方读取 |
| 配置状态栏 | `oma agents statusline [名]` | 四家写入面（S025 矩阵）：claude settings.json 幂等合并 statusLine 块；codex config.toml `[tui] status_line` 写成内置项 ID 数组（无外部命令面，command argv 会被静默跳空，M045）；kimi tui.toml `[status_line].command`；grok config.toml `[ui.status_line] type=command`。Windows Grok 的 command 必须是可直接 spawn 的 `.cmd` 单路径（整串 `Command::new` 遇引号报 os error 123，且不回落 shell，M048）；Unix 仍写 `pwsh -File`。pwsh 脚本释放 oma 数据根 statusline/（claude/kimi/grok），脚本内强制 UTF-8 输出防 CP936 下图标变问号；pwsh 未检出打 `statusline.pwsh=missing` 警告。共享脚本 projKind 先到先得：Cargo.toml / package.json / pyproject 优先，其后 `build.zig` / `go.mod` / `CMakeLists.txt` 或 `meson.build`（D11 / P0032）；图标 seti-zig E6A9、seti-go E627、seti-cpp E646；Grok ASCII 路径跳过工具链子进程（M046） |

### 2.3 密钥与 hook

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 密钥管理 | `oma agents secrets init\|set <KEY>\|env --shell <pwsh\|bash\|zsh\|nu>\|inject\|status` | 一钥两密文加四 shell 懒注入（S031，对齐 ohmycloud D20 与 ohmypwsh 懒注入）。oma 自管根落 `app.key`（32B、0600 原子写）、`identity.enc`（AES-256-GCM 包裹 SOPS 标准 age 身份，`oma:v1:` 标记）、`secrets.yaml`（SOPS 制密文，sops 二进制加工、值 base64）。解密链全程内存 app.key 到 identity.enc 到 SOPS_AGE_KEY 到 vault；`inject` 向四 shell profile 写标志行包裹的懒注入块（幂等；Windows 上 zsh/bash 另写 WSL `$HOME` 对应文件，块内无 `oma` 则回退 `oma.exe`，M050），交互 shell 启动现场解密只写会话 env，明文不常驻注册表。秘密不进 argv（set 走 stdin）、输出 redacted。`env` 金库空时出 noop 注释（pwsh 空串会让 IEX 炸）。token 部署配置主面（D15） |
| hook 写状态加密钥拦截 | `oma hook [event]` | 双职责。其一状态落盘：各家 hook 的 `command`，读 stdin JSON（`hook_event_name` / `hookEventName`）或参数，写 `OHMYAGENTS_STATE_FILE`；缺该环境变量则 exit 0。状态供状态栏 `agent:state` 机读标记消费（S025）。其二密钥拦截闸（S030）：PreToolUse / UserPromptSubmit 命中 block 级密钥则 exit 2 拒工具调用（stderr 掩码原因回给模型），PostToolUse 只观察。八层防误报：精确前缀、实值比对（env 加 providers.toml 明文）、熵值门、stopwords、语料拼接豁免、password warn-only、日志掩码、fail-open |

### 2.4 项目部署

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 部署项目全套 | `oma init [--project PATH]` | yolo 键加 hook / skill 部署：`.claude/settings.json`（yolo 加 hooks exec form）、`.codex/hooks.json` 加 `config.toml`（yolo 加 features.hooks）、`.grok/hooks/ohmyagents-state.json`、四家 skill 目录、AGENTS / CLAUDE.md（仅缺失时）。幂等合并保留外条目，不改家目录。四环境自适应（P0027，矩阵见 S024）：claude / grok 探针命中 PATH 写 bare `oma`（粘性不降级）；codex 按字段所有权各侧只写本侧（`command` / `commandWindows`）；共享项目目录 Windows 与 WSL 双侧并存不互踢；输出 `init.hooks.form=` 标记形态 |
| 部署项目级 yolo | `oma init --yolo [--project PATH]` | 仅无阻塞键：`.claude/settings.json`（`defaultMode=bypassPermissions`）、`.claude/settings.local.json`（顶层 `skipDangerousModePermissionPrompt`）、`.codex/config.toml`（sandbox / approval）、`.kimi-code/config.toml`（`yolo`）。不部署 hook / skill |
| 预写信任库 | `oma init --yolo --pretrust [--project PATH]` | 额外写用户家：claude.json trust、codex projects、kimi workspace-trust、grok trusted_folders；grok 的 `permission_mode` 只能写 `~/.grok/config.toml` |
| 权限模式 | `oma init --permission-mode auto\|yolo\|manual` | 覆盖默认 yolo；manual 不写 bypass |

### 2.5 自维护

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| oma 自更新 | `oma self update [--stable] [--repo owner/name] [--git] [--force]` | 缺省 dev 滚动源：CI 每推 main 构建测试后覆盖发布的 prerelease，按资产 sha256 判新；`--stable` 走正式版（v* tag 触发构建）；Windows rename 舞步自替换；无 release 体面降级 `--git` 源码安装。机制见 S028。镜像通道（D16）：设 `OMA_MIRROR=<基址>`（如 `https://env.ohmygh.com`）后 dev 通道改走 `<基址>/oma/dev/<资产>`，sha256 边车判新（免 manifest，边车裸哈希与 GitHub digest 归一互认）、下载后强制 sha256 校验（不符报错不回落）、网络类失败打 warning 回落 GitHub；stable 通道不吃镜像 |
| 生成补全 | `oma completions <shell>` | clap_complete 出 bash / zsh / fish / powershell 等补全脚本到 stdout（如 `oma completions powershell >> $PROFILE` 用法自取） |
| 检索会话 | `oma trace sessions [--project PATH]` | 查询时联邦读四家原生会话库（claude projects 目录、codex rollout、grok sessions、kimi session_index），列项目内各 agent 会话。只读，与 rmux 无耦合（D19 恢复） |
| 检索编辑轨迹 | `oma trace timeline [--agent A] [--file GLOB] [--limit N] [--project PATH]` | 意图操作块元素视图：每条编辑事件带 operation_id（session:call）、kind、tool、ts 与双意图（intent=用户请求、op_intent=assistant 声明）；分页 clamp 1-1000。四家全量：claude（Edit / Write）、codex（FileChange 主源加 apply_patch 兜底）、grok（updates.jsonl 权威主源加 chat_history 兜底，S020，逐事件真实时间）、kimi（loop tool.call） |
| 检索操作块 | `oma trace blocks [--agent A] [--limit N] [--project PATH]` | 一个 operation_id 一块（一次工具调用可能多文件），时间正序取最新 N 块，聚合 edits / files / kinds / 双意图 |
| 检索 agent 轨迹 | `oma trace agent <名> [--limit N] [--project PATH]` | 某家 agent 的操作块时间线（名不在四家内退出非 0） |
| 检索单文件轨迹 | `oma trace file <相对路径\|glob> [--agent A] [--limit N] [--project PATH]` | 文件维度：该文件被哪些 agent、何时、基于什么意图改过（创建 / 修改 / 删除），时间正序 |
| 检索关键词 | `oma trace search <query> [--agent A] [--limit N] [--project PATH]` | 正则匹配 patch、file、双意图四域，非法正则退字面子串；先全量匹配后截断；输出元素命中数与匹配块数两个粒度 |

## 三、输出契约

> S016 吸收裁决表的落点。双读者三轨，错误一律带下一步。

- **marker 行（机器面，默认）**：`命令.键=值` 行式输出进 stdout，管道与测试消费；断言只押 marker 行与退出码。
- **`--format kv|json|jsonl` 与 `--json` 简写（互斥）**：kv 是 marker 行缺省；json 出 `{ok, data\|error, meta:{command, project}}` 信封（P0015；HTTP / MCP 传输面已随 D15 移除，信封形状冻结不变）；jsonl 列表逐行对象。值一律字符串、字段序与 kv 行序一致（preserve_order）；结构化错误 stderr 单行 JSON、业务失败信封仍进 stdout 退出非 0，机器读者拿信封、人类拿 stderr 错误行。机器面冻结命令与退出码表见 `docs\references\R011-Agent友好IO契约-format三态信封退出码与冻结面.md`（issue #1）。
- **错误 CTA**：用户可见错误自带下一步；新增错误路径保持同款，禁止裸报错。
- **`--version`**：oma 自身版本（clap 标准，源 `Cargo.toml`；也是 ome catalog 集成本仓条目的探测条件之一，D06 切片 1 复核）。与 agent 装后探针的 `--version`（各家 agent 二进制）是两回事。

## 四、维护规则

- **新增命令同步链**：`src/deploy.rs` 的 COMMAND_MAP 加行，加 `cargo test deploy`，加重跑 `oma init` 重生 SKILL，加本文件对应族加行，加 AGENTS 三节加行；缺一即登记债。
- **落地状态标注**：`[实证: 日期 + 验收面]`；设计口径必须写明「设计口径」并挂 S 编号，禁止把未验收写成已落地。
- **六态标注口径**：行内合法形态 `[实证: 来源]` 与 `依据 docs\research\S0NN`；`[推断]` / `[假设]` 只许出现在 research（G004），本文件不许出现。
- **权威边界**：一句话进 AGENTS 三节，行为细节进本文件，文件定位进 INDEX，规则进 G 系；协作规则与文档检查、提交规范不在本文件重复（权威在 AGENTS 二与 G001）。
