# S018：aitrace 意图轨迹机制研究与 oma 检索映射

- 日期：2026-08-31
- 关联：方案 `P0013`（agent 意图操作块与编辑轨迹检索）；蓝本仓 `D:\aitrace`（fork 自 omeedcs/vibetracer，0.7.0 起无头 daemon + CLI + MCP，约 13.9k 行 Rust）；oma 侧挂 `S009`（状态判断）、`S015`（hook 矩阵）
- 研究法：子代理深读全仓源码 + 七条载荷性断言回源码抽查（EditEvent 字段、operation_id 构造、MAX_INTENT_CHARS、HOOK_GRACE、MCP 工具表、真实 edits.jsonl、project_path 空串——全部命中）

## 一、为什么研究

用户定调（2026-08-31）：「增加研究 D:\aitrace，实现指定项目下的各 Agent 意图操作块及编辑文件轨迹的检索功能」。oma 编排四路 agent 改同一项目，缺一个回答「哪个 agent、基于什么意图、何时、改了什么文件」的检索面。aitrace 已把「文件编辑真相 + hook 元数据 + transcript 意图」三源关联做通，是现成蓝本。

## 二、aitrace 是什么

- 定位：AI 编程会话的可观测性驾驶舱——记录项目目录每一次文件编辑（真相源是磁盘），关联 Claude Code hook 元数据与 transcript 意图，经 MCP 把时间线喂回 agent 自纠。[实证: README.md:3]
- agent 支持面：**只有 Claude Code 是一等集成**（PostToolUse hook + transcript 解析）；Cursor/Codex 只剩上游遗留的 `.agent-trace/` 目录导入且自称未验证。[实证: src\import\detect.rs:21-27、README.md:22]
- 形态：无头 daemon（项目内 UDS `daemon.sock`，Windows 走 `uds_windows`）+ CLI + MCP（stdio JSON-RPC）；无 sqlite、无索引，检索全部查询时线性扫 JSONL。[实证: 源码结构与 mcp 模块]

## 三、数据源与链路

| 源 | 机制 | 证据 |
| --- | --- | --- |
| 文件 watcher | notify + debouncer 递归监听，只取 Create/Modify；组件级 ignore（精确 + glob，如 `*.tmp.*` 滤编辑器原子写） | src\watcher\fs_watcher.rs:66-100 |
| Claude PostToolUse hook | matcher `Write\|Edit` → `aitrace hook-send` → UDS 换行分隔 JSON；payload 改写为 `agent_id`（=session_id）、`operation_id`、`tool_name`、`file`、`transcript_path`、`is_error` | src\hook\send.rs:46-95 |
| transcript jsonl | `~/.claude/projects/<路径>/<session-uuid>.jsonl` 增量 tail（记 byte offset，半行留给下轮） | src\daemon\intent_index.rs:104-156 |
| 自有存储 | 追加式 `edits.jsonl` + 内容寻址快照库 + 每会话 meta.json，全在 `<project>/.aitrace/sessions/<id>/` | 源码 |

**hook↔watcher 竞态**：PostToolUse 在落盘之后触发，watcher 事件先到——daemon 用 `HOOK_GRACE = 250ms` 忙等收 hook 元数据再归组。[实证: src\daemon\mod.rs:506、:339]

**路径归一化**：hook 报绝对路径、watcher 报相对路径，关联键统一「相对化 + 正斜杠 + Windows 小写」再进 FIFO；macOS FSEvents 前缀用 canonicalize 兜底。[实证: src\daemon\correlation.rs:13-39]

## 四、意图操作块的机制

> aitrace 没有独立「块」实体——是 `EditEvent` 上的一组归组字段，查询时按 `operation_id` 聚合。

### EditEvent 骨架

```rust
pub struct EditEvent {
    id: u64,                    // 会话内 1-based 帧号
    ts: i64,                    // epoch ms
    file: String, kind: EditKind, patch: String,   // unified diff 全文
    before_hash/after_hash,     // SHA-256
    intent: Option<String>,           // 用户请求（user intent）
    operation_id: Option<String>,     // "session_id:tool_use_id"
    operation_intent: Option<String>, // assistant 声明的操作意图
    agent_id/agent_label/tool_name/restore_id,
}
```

[实证: src\event.rs:39-53 及全结构；真实数据 `.aitrace/sessions/20260828-072234-773273/edits.jsonl` 尾行含 `"intent":"继续"` 与完整 operation_id]

**`operation_id = "session_id:tool_use_id"` 一根线串起「hook 元数据 ↔ transcript 意图 ↔ 编辑帧」——这是该项目最值钱的设计。** 无 tool_use_id 时退回 session_id。[实证: src\hook\send.rs:70-74]

### 意图切分算法（查询时活走父链）

1. `operation_id` 用 `rsplit_once(':')` 取回 tool_use_id，transcript 里 `tool_use` 块登记 `tool_use_id → entry uuid`。
2. 沿 `parentUuid` 父链上溯：**最近一个 assistant text 块胜出；链上无 text 取最近 thinking**；ToolUse/UserText/Other 是游走边界。批量并行工具调用共享同一前置文本。[实证: intent_index.rs:268-290 与测试 :393-414]
3. 截断 `MAX_INTENT_CHARS = 200` 按字符不按字节（按字节切中文会 panic 在 log 宏里杀死 daemon）。[实证: :23、:302-305；panic 坑有回归测试]
4. 用户意图 `intent` 双源：`last-prompt` 标记 + 真实 user text entry，**文件里更靠后的赢**（标记滞后一轮）。[实证: :100-102、:158-194]
5. **transcript 懒写坑**：父文本可能落在 tool_use 行之后数秒——所以查询时活走父链而非 absorb 时一次解析，配**补账队列**（intent 缺失的编辑进 backfill，每轮重试，跨重启持久化 `backfill.json`，上限 32 条）；补账写成**同 id 追加记录**，read_all 按 id 去重保最后一条且维持首见顺序，且必须写回 originating 会话（id 会话内唯一、跨会话撞号）。[实证: 模块头注释 :1-16、daemon\mod.rs:76-120/267-305、edit_log.rs:41-67]
6. 无父链可走时 `operation_intent` 为 null——意图是尽力而为不是保证。[实证: 真实数据可见 null]

## 五、编辑轨迹与帧重建

- **事件 + 内容双轨**：内容真变才记（新旧相同跳过）→ similar 算 unified diff 与 ±行数 → 新内容按 SHA-256 进内容寻址库（`snapshots/<前2字符>/<后62字符>`，仿 Git 对象布局，天然去重）→ 事件追加 edits.jsonl。删除按空串读记 kind=Delete。[实证: recorder\mod.rs:140-170、store.rs:8-49]
- **帧 = 编辑 id**：`get_frame` 重建任意时刻状态——每文件取 ≤frame_id 的最后一条编辑按 after_hash 从快照库取内容。[实证: mcp\handlers.rs:174-205]
- **跨会话基线继承**：daemon 启动找最近真有 edits.jsonl 的前会话，继承 `file → after_hash` 表并复用其快照库作 diff 旧内容源——否则重启后所有文件都变 create。[实证: session.rs:139-149、recorder\mod.rs:71-107]
- 恢复动作预登记 `restore_id` 且优先于 hook 富化，避免把自己的恢复记成 agent 编辑。[实证: correlation.rs:128-147]
- 会话目录名 `YYYYMMDD-HHMMSS-6f微秒` 单调即创建序；meta.json 含 agents[]（label 自增 `claude-code-N`、edit_count、failed_attempts 单独计 is_error 不进时间线）。[实证: session.rs:54-64、event.rs:92-104]

## 六、检索面

CLI：`sessions` / `replay`（文本表 + 每行 `op:`/`ask:` 双意图）/ `restore` / `export` / `daemon start|stop|status|reap`。MCP 七工具（stdio）：

| 工具 | 关键参数 | 备注 |
| --- | --- | --- |
| `list_sessions` | limit/offset | 每条带 `buildHash`（git 短哈希 + -dirty，build.rs 注入） |
| `get_timeline` | session_id、file_filter（glob）、分页 | **无 agent 过滤** |
| `get_frame` / `diff_frames` | frame_id | 重建 / 对比任意时刻 |
| `search_edits` | query（regex，非法退回字面子串） | 匹配域 = patch + file + intent 三字段 |
| `get_regression_window` | file、start/end_frame | 回归窗口 |
| `subscribe_edits` | session_id | edit_notification 转 MCP 通知（阻塞转发） |

分页 clamp：DEFAULT_LIMIT=100、MAX_LIMIT=1000，流式逐行读不进内存。[实证: mcp\tools.rs:10-181、pagination.rs:7-8]

## 七、裁决表

### 吸收

| 件 | 裁决 | 理由 |
| --- | --- | --- |
| `operation_id = session:tool_use` 归组键 | 吸收（核心） | 一根线串三源，oma 检索「意图操作块」的骨架 |
| 双意图字段（operation_intent / intent） | 吸收 | 不把 assistant 的话冒充用户需求 |
| 查询时活走父链 + 同 id 追加补账 | 吸收 | append-only 天然支持「后到的真相」，不改历史 |
| 关联键归一化（相对化+正斜杠+小写） | 吸收 | **写库时就归一化**，别学它两条路（见坑 3） |
| HOOK_GRACE 忙等竞态解法 | 吸收 | hook 后置触发是普适时序 |
| buildHash 随响应 | 吸收 | 长连 MCP/HTTP 进程不随部署升级的机检（P0011 直接抄） |
| 项目级 hook 注册幂等不改家目录 | 对照 | 与 oma init 同构，对照校验 |
| MCP 工具参数形状（session + glob + regex + 分页 clamp） | 吸收 | 检索面对标；**补 agent 过滤与项目路径显式落库两缺口** |

### 不吸收 / 暂缓

| 件 | 裁决 | 理由 |
| --- | --- | --- |
| 内容寻址快照库 | 暂缓 | oma v1 只检索不恢复，只存 diff + hash 体积降一个量级；要恢复再上 |
| 无头 daemon 常驻 | 暂缓 | oma 已有 rmux 会话与 hook 通道，v1 走「hook 事件 + 查询时 transcript 回溯」，不自建录制 daemon |
| 自建 watcher | 切片内定 | 编辑真相源方案在 P0013 切片 3 决（notify 引入与否） |
| Cursor/Codex `.agent-trace/` 导入 | 不吸收 | 上游遗留未验证 |

### 坑清单（oma 直接设防）

1. patch 全文内联 JSONL 体积（最大 583KB/会话）＋空会话目录堆积（51 个目录约一半只有 127 字节 meta）——oma：daemon/采集启动即建会话目录改为**首事件才建**。[实证: 磁盘实测]
2. 查询全量线性扫——oma：v1 JSONL 可接受，量大再 sqlite（R005 选型）。
3. 写库路径反斜杠未归一化（`tests\\integration\\...`）而关联键是正斜杠——两条路必踩，oma 统一写库即归一化。[实证: recorder\mod.rs:134-137 vs correlation.rs:24]
4. 非 UTF-8 读失败当空串——oma 按 lossy 或跳过并记标记。
5. **并发按文件路径单 FIFO——两个 agent 同改一文件会张冠李戴**；oma 多路场景必须按 agent/operation 维度排队。[实证: correlation.rs 结构]
6. 会话 meta 的 `project_path` 写死空串没人填（`project_path: String::new()`）——oma 要「指定项目检索」必须显式落库。[实证: src\session.rs:87、真实 meta.json]
7. 部署耦合：daemon 从 target\debug 跑锁链接器（os error 5）；exe 被锁则改名 `.old`；workspace_root 上跳找 Cargo.toml 且不能 canonicalize（`\\?\` 前缀会断 UDS）。[实证: project.rs:13-22]
8. 中文截断按字节 panic——按字符截。[实证: daemon\mod.rs:63-70 回归测试]

## 八、oma 映射

> P0013 的依据。

- oma 的「意图操作块」= operation_id 归组 + 双意图字段；oma 已有 hook 通道（`oma hook` 带四态）与 agent 身份（`OHMYAGENTS_AGENT`、层 2 状态文件），比 aitrace 多了**多路 agent 维度**——检索面天然要按 agent 过滤（aitrace 的缺口正是 oma 的主场）。
- oma 比 aitrace 难的点：四家 transcript 格式各异（Claude 已知 jsonl 结构；codex/grok/kimi 待研），一等集成不能只做 Claude。
- oma 比 aitrace 易的点：agent 是 oma 自己拉起的（pane 清单在 manifest），身份关联不用猜。
- v1 形态：项目级 `.ohmyagents\trace\`（边界内）；只检索不恢复（快照暂缓）；CLI 检索子命令 + 将来挂 P0011 的 MCP 面。
