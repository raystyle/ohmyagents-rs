# agent 意图操作块与编辑轨迹检索

- 状态：进行中（claude/codex 全链验收通过——无头双家产编辑、五视图检索可见；grok/kimi 事件 loader 待 S019 源码核实）
- 日期：2026-08-31
- 关联：研究 `S018`（aitrace 机制与 oma 映射）、`S009`（状态四层）、`S015`（hook 矩阵）；前置 P0006-P0012（编排与安装底座）；用户定调 2026-08-31：「增加研究 D:\aitrace，实现指定项目下的各 Agent 意图操作块及编辑文件轨迹的检索功能」

## 背景与问题

oma 编排四路 agent 改同一项目，事后无法回答「哪个 agent、基于什么意图、何时、改了什么文件」。aitrace（S018）已把「文件编辑真相 + hook 元数据 + transcript 意图」三源关联做通但只支持 Claude Code 一家、且无 agent 过滤、会话不记项目路径。oma 的主场恰是多路 agent：pane 清单与 hook 身份都是 oma 自己发的。

## 目标与非目标

- 目标：
  - 采集：项目级编辑事件落 `.ohmyagents\trace\`（edits.jsonl + meta.json，meta 显式落 project_path 与 agents——补 aitrace 两缺口）
  - 归组：`operation_id`（agent 会话标识加 tool 调用标识）串 hook 元数据与会话日志意图，双意图字段（operation_intent / intent）分开
  - 检索：`oma trace` 子命令族（sessions / timeline / search / file），支持**按 agent、按项目、按文件 glob、按 regex** 过滤加分页 clamp
  - 四家逐步接入：Claude 先行（transcript 结构已知），codex/grok/kimi 逐家研究会话日志格式后接
- 非目标：
  - 不做恢复/回滚（restore）；内容寻址快照暂缓，只存 diff + 哈希
  - 不自建常驻录制 daemon（v1 用 oma hook 事件 + 查询时会话日志回溯；watcher 引入与否在切片 3 决）
  - 不改各 agent 用户级配置；trace 数据只落项目目录（AGENTS 边界）

## 方案

> 2026-08-31 架构定案（S019 本地实证后）：**查询时联邦**取代采集落盘——四家原生会话库
> （claude transcript / codex rollout / grok chat_history / kimi wire.jsonl）各自携带项目归属、
> 工具调用身份、双意图与编辑内容，oma 在查询时读取并归一化。零采集设施（无 watcher、无 daemon）、
> 可回溯 oma 部署前的全部历史。原「.ohmyagents\trace 落盘」方案降为远期缓存层。

### 数据形状

归一化事件（loader 公共产出）：

```text
TraceEvent { agent, session_id, call_id, tool, file(项目相对+正斜杠), kind(create|modify|delete),
             user_intent, op_intent, ts, patch }
operation_id = session_id:call_id   （S018 核心设计，一根线串会话与工具调用）
```

四家会话发现（本地实证）：

| 家 | 位置 | 项目归属 |
| --- | --- | --- |
| claude | `~/.claude/projects/<slug>/*.jsonl`（slug=非字母数字换 `-`） | 目录 slug |
| codex | `~/.codex/sessions/*/*/rollout-*.jsonl` | 首行 `session_meta.cwd` |
| grok | `~/.grok/sessions/<百分号编码项目路径>/<uuid>/chat_history.jsonl` | 目录名解码 |
| kimi | `~/.kimi-code/session_index.jsonl` 的 `workDir` | 索引行 |

### 意图回溯

顺序文件近似（查询时）：tool 调用前最近的 assistant text 是操作意图、最近的真实 user text 是用户意图；codex 跳过环境注入的 user message（`# AGENTS.md`/`<environment_context>` 族）；截断按字符 200（S018 坑 8）。

### 检索面设计原则

> 用户定调（2026-08-31）：**轨迹本身就是各实体时间线**——文件、agent、会话、操作块四个实体维度各一条时间线，search 跨实体。五个视图：

| 视图 | 命令 | 实体 |
| --- | --- | --- |
| 编辑轨迹 | `oma trace timeline` | 编辑事件（元素），时间正序取最新 N |
| agent 轨迹 | `oma trace agent <名>` | 某家的操作块时间线 |
| 意图操作块 | `oma trace blocks` | operation_id 块（一次工具调用，可能多文件） |
| 操作块时间线 | 同 blocks | 块的时间正序视图（最新 N 块，与 timeline 语义一致） |
| 文件轨迹 | `oma trace file <路径>` | 单文件的修改史（谁、何时、何意图） |
| 块与元素搜索 | `oma trace search <query>` | 跨实体正则，输出元素命中数与匹配块数两个粒度 |

### 切片

1. **S019 四家会话日志格式研究**：本地实证已完成；源码核实进行中（grok 编辑类 tool 形状、kimi tool.result、codex 行序保证）
2. **联邦检索层**：`src\trace.rs`——四家会话发现 + claude/codex 事件 loader（已落地）+ grok/kimi loader（待源码核实）
3. **CLI 检索面**：`oma trace sessions|timeline|blocks|agent|file|search`（分页 clamp 1-1000；非法正则退字面子串；先匹配后截断）——已落地
4. **验收**：无头通道（ohmypwsh S010 实测基准：claude -p / codex exec / kimi -p）在临时项目产真实编辑后检索可见
5. **P0011 联动**：检索面同签名挂 MCP tool；输出带 buildHash 风格版本行

## 风险与回滚

- 四家 transcript 格式漂移（无官方契约）：解析按「尽力而为 + null 兜底」，格式变化不崩检索
- edits.jsonl 线性扫性能：v1 接受，量大再 sqlite（R005）
- 回滚：trace 是新增目录与新增子命令，关掉即无痕

## 验收标准

- 指定项目跑一路真实 agent（Claude 先行）改文件后：`oma trace timeline` 能按 operation_id 出块、双意图可见、`oma trace search <词>` 命中 patch/intent/file 三域
- 按 agent 过滤在多路会话下各归各（stub 双路互改同文件不张冠李戴）
- meta.json 的 project_path 非空；空跑不建会话目录
- `cargo test` 全绿；文档三件套过；R002/INDEX/TODO/GOAL/diary 同步

## 验收实录

> 2026-08-31 无头通道（ohmypwsh S010 基准法）。

- 临时项目双家无头各写一文件（`claude -p --permission-mode acceptEdits`、`codex exec --skip-git-repo-check -s danger-full-access`），`oma trace sessions` 两家各命中一会话（claude 走目录 slug、codex 走 `session_meta.cwd`）；`timeline` 两条编辑事件各带完整 `intent`（prompt 原文）与 `operation_id`；`trace file hi.md` 单文件轨迹命中 codex 的 create。[实证: 全程输出留存]
- 无头一次性任务的 `op_intent` 为 `-`：模型直奔工具没有前置文本——意图是尽力而为不是保证（S018 同结论）。
- claude `Write` 无显式 create/delete 信息，v1 kind 一律 modify（文档化局限；磁盘真相归远期缓存层）。
- 验收顺带踩了 M031 同族坑：没切 cwd 两家把文件写进本仓——先清误产物再 `Set-Location` 重跑。[实证]

## 实施过程与经验

### 联邦检索首落 claude 与 codex

> 2026-08-31。

- **claude loader**：user 行（跳过 isMeta）更用户意图、assistant text 块更操作意图、Edit/Write/MultiEdit/NotebookEdit 的 tool_use 出事件（call_id 即 `tool_use.id`，patch 取 `new_string`/`content`）；claude 的 kind 无显式 create/delete 信息，v1 一律 modify（文档化）。[实证: 本仓 `oma trace timeline` 输出]
- **codex loader**：`response_item` 顺序扫——assistant message 更操作意图、真实 user message 更用户意图（`is_codex_injected_context` 跳过 `# AGENTS.md`/`<environment_context>`/`<user_instructions>`/`<turn_context>` 注入）；`custom_tool_call(apply_patch)` 解析 `*** Add/Update/Delete File:` 头出事件，**一次调用多文件操作出多事件共享 call_id**（本地实证的补丁形状）。[实证: 本机 rollout 采样 + 夹具单测]
- **双意图活体自证**：对本仓自跑 `oma trace timeline`，输出 `intent=继续 op_intent=clap 层级问题……对齐` 挂在每次 Edit 上——用户话与 assistant 话各自归位；`oma trace search` 直接挖出**历史轮次**的研究编辑（联邦架构回溯威力的实证——那些编辑发生在 oma trace 存在之前）。[实证]
- **两个实现坑当场修**：其一 search 先截断后匹配把候选池截没（改先匹配后 truncate）；其二文件存绝对路径导致相对 glob 失配（装载时 `relativize`——项目内路径相对化，对齐 aitrace 存储形状）。[实证]
- **夹具**：`tests\fixtures\trace\{claude,codex}` 用真实形状行（含 apply_patch 双文件操作、环境注入 user message 干扰项）锁 loader 断言（R004 黄金文件法）。
- grok/kimi：会话发现已通（grok 百分号解码目录命中本仓、kimi session_index 过滤待接），事件 loader 等源码核实报告（grok 编辑类 tool_type 名称、kimi tool.call 的编辑 args 键位）后补。
