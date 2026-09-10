# 常用命令与管理流程：从项目 init 到部署诊断

> 角色：AGENTS 意图路由的细则载体，**命令面唯一权威**：行为细节、机理出处、marker 行、退出码、落地状态。
> 边界：协作规则在 AGENTS 二；文件与模块定位在 INDEX；规范禁令在 `docs\guide\`；输出冻结面见 `docs\references\R011-Agent友好IO契约-format三态信封退出码与冻结面.md`。
> 显示名 Oh My Agents；仓库 `ohmyagents-rs`；CLI 二进制 `oma`。运行时数据目录是 `.oma`（D14；旧名 `.ohmyagents` 仅旧在则改名迁过去）。

已落地命令全表（D20 收窄 2026-09-09：五功能 = 可用性诊断、hook、状态栏、trace、yolo）：`oma init`（全套 / --yolo）、`oma doctor`、`oma agents`（检测 / statusline / verify）、`oma hook`（状态落盘加密钥拦截闸）、`oma self update`、`oma completions`、`oma trace` 六视图（只读检索，与 rmux 无耦合）、`oma diagnose cache\|agents`（D21 活性诊断，打真网关）、全局 `--format` 加 `--json`。编排命令已随 D15 移除、token 注入面（secrets / providers / login）与 install / update 兼容层已随 D20 移除，历史口径见归档 P0001 至 P0034、P0040 与 git 历史。

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

### 2.2 agent 检测

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 检测已装 agent | `oma agents` | 扫 PATH、`OMA_AGENT_PATH`、`OMA_<AGENT>_BIN`、oma 自管根 `~/.oma/agents`、各家默认目录（Windows / Linux / macOS）；打印 `source=env\|path\|oma\|default` 与 version。缺装不退出非 0，缺装行带 `hint=ome install <名>`（D20 起 hint 改指 ome） |
| 配置状态栏 | `oma agents statusline [名] [--example] [--script 路径] [--builtin]` | 四家写入面（S025 矩阵）：claude settings.json 幂等合并 statusLine 块；codex config.toml `[tui] status_line` 写成内置项 ID 数组（无外部命令面，command argv 会被静默跳空，M045）；kimi tui.toml `[status_line].command`；grok config.toml `[ui.status_line] type=command`。Windows Grok 的 command 必须是可直接 spawn 的 `.cmd` 单路径（整串 `Command::new` 遇引号报 os error 123，且不回落 shell，M048）；Unix 仍写 `pwsh -File`。pwsh 脚本释放 oma 数据根 statusline/（claude/kimi/grok），脚本内强制 UTF-8 输出防 CP936 下图标变问号；pwsh 未检出打 `statusline.pwsh=missing` 警告。共享脚本 projKind 先到先得：Cargo.toml / package.json / pyproject 优先，其后 `build.zig` / `go.mod` / `CMakeLists.txt` 或 `meson.build`（D11 / P0032）；图标 seti-zig E6A9、seti-go E627、seti-cpp E646；Grok ASCII 路径跳过工具链子进程（M046） |
| 状态栏用户级定制 | `~/.oma/statusline.toml`（D18） | 生成时烘焙：`oma agents statusline` 每次运行读本文件重拼脚本后落盘（脚本唯一权威仍是二进制内嵌段块，手改落盘必被重盖，定制只能走本文件）。键级缺省回落：没写的键用内嵌默认；坏文件硬错退出 1（错误带文件出处）。三层键：`segments`（段 id 数组即全量序，可用 shell / dir / oma / model / context / duration / git / package / python / rust / node / zig / go / cpp；未知与重复 id 硬错）、`[template]`（段格式串，占位符 `{icon}` `{name}` `{path}` `{agent}` `{state}` `{model}` `{pct}` `{used}` `{window}` `{duration}` `{branch}` `{flags}` `{version}`；`<段>-ascii` 键给 grok 的 ASCII 形，缺省同用 nerd 模板图标恒空）、`[icons]`（图标映射，键含 `ts` 与 `shell-pwsh` 子项；oma 机器人宽字形默认跟两空格）、`[codex] items`（codex 内置项 ID 子集原样透传，未知 id codex 侧静默跳过；键缺省回落内嵌推荐八项）。模板与图标值经单引号转义烘焙，用户串无法越出 ps1 字面量（注入不成立）。COMMON 与 PROBE 按消费方门控拼入（段序无 dir / oma 不跑 rev-parse；无 package 与工具链段不跑 projKind 探测，省 kimi 300ms 预算）。`--example` 打印带注释全量模板（对齐 providers 先例）。整脚本替换：`--script <路径>` 把自备脚本拷到部署位（agent 配置命令行不动）并写 `<脚本>.custom` 标记记源路径，此后无 `--script` 的重跑跳过内嵌覆盖（kv 打 `statusline.custom=true` 与 `statusline.script=<源>`）；`--builtin` 删标记还原内嵌。自备脚本调用契约：首参 agent 名、stdin 喂 agent JSON、stdout 单行状态栏、kimi 侧 300ms 超时约束（S025）。`oma agents verify` 直跑部署位脚本，天然覆盖自备脚本回归 |

> D20（2026-09-09）移除面历史口径：`oma agents install` / `update`（D07 迁册 ome 后保留兼容，D20 删）、`oma agents login`（设备码引导，S026）、`oma agents providers`（别名簿，S027）、`oma agents secrets`（一钥两密文加四 shell 懒注入，S031）。行为细则与机理见 git 历史与 P0029 / P0036 / P0040；密钥安全归 ohmypwsh，agent 二进制归 ome。

### 2.3 hook

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| hook 写状态加密钥拦截 | `oma hook [event]` | 双职责。其一状态落盘：各家 hook 的 `command`，读 stdin JSON（`hook_event_name` / `hookEventName`）或参数，写 `OHMYAGENTS_STATE_FILE`；缺该环境变量则回退写 `<payload cwd 的 .git 根>/.oma/state/<agent>.json`，回退要求 agent 名非空（`OHMYAGENTS_AGENT` 或 `--agent`，注册命令已烧入）且 cwd 向上 8 层内有 `.git`，两条件缺一则静默 exit 0 不落盘（D40 四端验收红根因：无头直测漏 `--agent`，实现无缺陷）。状态供状态栏 `agent:state` 机读标记消费（S025）。其二密钥拦截闸（S030）：PreToolUse / UserPromptSubmit 命中 block 级密钥则 exit 2 拒工具调用（stderr 掩码原因回给模型），PostToolUse 只观察。八层防误报：精确前缀、实值比对（D20 后只看环境变量；D23 起完整 token 边界匹配：值须两侧无 [A-Za-z0-9_.-] 类字符才算命中，杀模型别名连字符超集误报 #9，真实密钥完整值仍 block；masked 形如 `名[头4字符…长度]` 可定位不泄密）、熵值门、stopwords、语料拼接豁免、password warn-only、日志掩码、fail-open |

### 2.4 项目部署

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 部署项目全套 | `oma init [--project PATH]` | yolo 键加 hook / skill 部署：`.claude/settings.json`（yolo 加 hooks exec form）、`.codex/hooks.json` 加 `config.toml`（yolo 加 features.hooks）、`.grok/hooks/ohmyagents-state.json`、四家 skill 目录、AGENTS / CLAUDE.md（仅缺失时）。幂等合并保留外条目，不改家目录。四环境自适应（P0027，矩阵见 S024）：claude / grok 探针命中 PATH 写 bare `oma`（粘性不降级）；codex 按字段所有权各侧只写本侧（`command` / `commandWindows`；`command` 为 codex schema 必填，Windows 侧新部署无保留值时落 bare `oma hook --agent codex` 兜底，M055；commandWindows 为 cmd 形态直接引号路径（codex 在 Windows 用 cmd 执行该字段，PowerShell 调用操作符 `&` 必炸，M056）；config.toml `[hooks]` 只留 state 信任键，非 state 定义键部署时清扫（单一来源 = hooks.json，消 codex 双 representation 警告）；共享项目目录 Windows 与 WSL 双侧并存不互踢；输出 `init.hooks.form=` 标记形态 |
| 部署项目级 yolo | `oma init --yolo [--project PATH]` | 仅无阻塞键：`.claude/settings.json`（`defaultMode=bypassPermissions`）、`.claude/settings.local.json`（顶层 `skipDangerousModePermissionPrompt`）、`.codex/config.toml`（sandbox / approval）、`.kimi-code/config.toml`（`yolo`）。不部署 hook / skill。层级语义（跨仓共识 2026-09-10）：托管端用户级 yolo 键以 omc 模板为准（patchCodexConfig 固化 approval_policy=never 加 sandbox_mode=danger-full-access）；oma --yolo 面向项目级与非托管端，codex 项目级配置覆盖用户级 |
| 预写信任库 | `oma init --yolo --pretrust [--project PATH]` | 额外写用户家：claude.json trust、codex projects、kimi workspace-trust、grok trusted_folders；grok 的 `permission_mode` 只能写 `~/.grok/config.toml` |
| 权限模式 | `oma init --permission-mode auto\|yolo\|manual` | 覆盖默认 yolo；manual 不写 bypass |

### 2.5 活性诊断

> D21（2026-09-09，ohmyagents-rs#8 / ohmycloud D45 配套）。与 doctor 的契约分家：这里打真网关（llm.d3fend.cn）、烧最小 token（每别名至多三条极短 prompt）、有网络延迟；doctor 保持零网络零 token。凭据只读 agent 侧原生配置（D45 模板下发形态），`OMA_GATEWAY_URL` / `OMA_GATEWAY_KEY` 环境覆盖（联调与测试通道）；不新建存储（D20 口径）。单机语义边界（2026-09-10 跨仓共识）：本族只做本机运行时诊断；多端装态、配置对账、网关舰队探活归 omc agent doctor（配置真源在 omc 金库），oma 不做多端配置一致性检测。

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| 缓存探测 | `oma diagnose cache [别名...]` | 网关发现序：env 覆盖 > claude `~/.claude/settings.json` env（ANTHROPIC_BASE_URL 加 AUTH_TOKEN）> codex `~/.codex/config.toml` 激活 provider 的 base_url 加 auth.json OPENAI_API_KEY；别名缺省 = GET /v1/models 全量。逐别名判线：`-codex` 尾走 /v1/responses，其余走 /v1/messages。双连同 payload（至多三连取优，网关两连异区只写不读的实测形态）：claude 线 system 带 `cache_control: ephemeral` 长文本（约 2000 token，稳过 1024 门槛）读 usage 的 cache_creation/read_input_tokens；codex 线 instructions 读 `usage.input_tokens_details.cached_tokens` 与 cache_write_tokens。verdict：hit（读到）/ write-only（只写不断连）/ auto-prefix（ds claude 线特判：官方端点全自动前缀匹配、usage 不透传字段，判不可见而非无缓存）/ none / error。kv 输出 `cache.<别名>.line= verdict=` 加 summary；任一 error 退出 1 |
| agent 配置检测 | `oma diagnose agents` | claude 面：base_url 是否网关指向（ok / missing / warn-not-gateway）、ANTHROPIC_MODEL 加 DEFAULT 三键对 /v1/models 在册核对（warn-not-on-record，防改名漂移）、MAX_THINKING_TOKENS 对实测上限表（zy-claudefable5 与 zy-claudeopus48 = 65536；未知上限报 info）。codex 面：激活 provider base_url 指向、model 在册、auth.json key 单独打一发 /v1/models 判 alive/dead。kv 输出 `agents.<家>.<项> state=`；探测完成即 ok=true（warn 是发现不是失败） |

### 2.6 自维护

| 意图 | 命令 | 行为细则 |
| --- | --- | --- |
| oma 自更新 | `oma self update [--stable] [--repo owner/name] [--git] [--force]` | 缺省 dev 滚动源：CI 每推 main 构建测试后覆盖发布的 prerelease，按资产 sha256 判新；`--stable` 走正式版（v* tag 触发构建）；Windows rename 舞步自替换；无 release 体面降级 `--git` 源码安装。机制见 S028。镜像通道（D16）：设 `OMA_MIRROR=<基址>`（如 `https://env.ohmygh.com`）后 dev 通道改走 `<基址>/oma/dev/<资产>`，sha256 边车判新（免 manifest，边车裸哈希与 GitHub digest 归一互认）、下载后强制 sha256 校验（不符报错不回落）、网络类失败打 warning 回落 GitHub；stable 通道不吃镜像 |
| 生成补全 | `oma completions <shell>` | clap_complete 出 bash / zsh / fish / powershell 等补全脚本到 stdout（如 `oma completions powershell >> $PROFILE` 用法自取） |
| 生成 oma 技能 | `oma skill [--write]` | D22：从 clap 活命令树自适应渲染 SKILL.md（frontmatter 按 Agent Skills 标准；命令表含子命令与旗标，新命令自动出现不需手维护）；缺省打印 stdout，`--write` 落用户级 `~/.claude/skills/ohmyagents/SKILL.md`（幂等覆写，升级后重跑同步）。项目级 init 生成物（COMMAND_MAP 加 marker 覆写语义）是另一层，双层各管各的面 |
| 无头验收 agent | `oma agents verify [名...] [--timeout N]` | 两家一层四行（D17，S033 两层判据）。状态栏层：mock JSON `{}` 直跑 `~/.oma/statusline/oma-statusline.ps1` 断言首行 `<agent>:` 机读标记（codex 无外部命令面，断言 `[tui] status_line` 含内置项 ID 标 builtin）。hook 层：临时 git 项目 in-process 部署 hook 后拉起无头（claude `-p` 加 `--dangerously-skip-permissions`、codex `exec` 加 `--dangerously-bypass-hook-trust`、grok `-p` 加 `--always-approve`、kimi `-p`），断言 `.oma/state/<agent>.json` 落盘加 state 四态合法；SessionStart 先于模型调用故不依赖 token 有效性；kimi SessionEnd print 不触发不作判据。grok 临时目录须先种子 trusted_folders（verify 自动种子加 Drop 摘除）。binary 不在记 skip 不计败；任一非跳过项 fail 退出 1 |
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
