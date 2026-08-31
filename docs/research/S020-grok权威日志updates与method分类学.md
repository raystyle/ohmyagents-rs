# S020：grok 权威日志 updates 与 method 分类学

- 日期：2026-08-31
- 关联：方案 `P0014`（grok loader 升级）；前置 `S019`（第二节定下「chat_history 是派生缓存、权威在 updates.jsonl」与升级路径）；用户指令「继续todo」从队列接续
- 研究法：本地实证（本机 23 个 grok 会话直接采样，重点本仓 8-29 会话 8086 行 updates）加源码结论承接（S019 已核实 rebuild_chat_history 机制），载荷性断言全部来自本机实物

## 一、为什么研究

S019 留下缺口：oma v1 读 grok 的 chat_history.jsonl 是派生缓存（compaction/rewind 触发整体重建，历史可被改写），且行无时间戳（ts 靠会话 uuid v7 起点近似，同会话所有事件共享一个时间）。升级路径是 updates.jsonl，但当时只知信封形 `{timestamp,method,params}`，缺 method 分类学——本研究把它钉死。

## 二、信封与两流

> 全部本机实测。[实证: 本仓会话采样]

```json
{"timestamp":1787999890,"method":"session/update","params":{"sessionId":"...","update":{"sessionUpdate":"...","...":"..."}}}
```

- `timestamp` 是 **epoch 秒**（非 ms；1787999894 = 2026-08-29T10:38:14Z）。
- method 两流，载荷同形职责不同：

| method 流 | 职责 | 出现的 sessionUpdate |
| --- | --- | --- |
| `session/update` | 内容（oma 检索只读这流） | tool_call_update、tool_call、agent_thought_chunk、agent_message_chunk、user_message_chunk、plan、current_mode_update |
| `_x.ai/session/update` | 遥测（hook/turn/compaction） | hook_execution、turn_completed、retry_state、auto_compact_started/completed、compaction_checkpoint、task_backgrounded/completed |

两流无重复载荷：tool_call 只在内容流，hook 只在遥测流——不需要跨流去重。

## 三、四要素定位

| 要素 | 位置 | 细节 |
| --- | --- | --- |
| 用户意图 | `user_message_chunk.content.text` | 合成闸门是 `_meta.hideFromScrollback == true`（实测样本是 `<system-reminder>` 后台任务通知）——等价 claude isMeta / grok chat_history 的 synthetic_reason / kimi origin.kind；`_meta.promptIndex` 是 prompt 分片归组键（本机样本 1:1，代码仍按连续拼接防御） |
| 操作意图 | `agent_message_chunk.content.text` | 流式分片，连续拼接（本机样本最长单 run 1 片，防御性拼接保留） |
| 编辑记录 | `tool_call`（注意不是 tool_call_update） | `toolCallId` 是 call 身份；`_meta` 下键名含斜杠的 `x.ai/tool` 带 `kind: write/edit/read`——**写族判定用 kind 免名字硬编码**（名字清单留作 `_meta` 缺失时的兜底）；`rawInput` 是**现成 JSON 对象**（chat_history 里是字符串还要二次解析）：`file_path`（或 path/target_file）加 `content`（write）或 `old_string/new_string`（search_replace） |
| 时间 | 信封 `timestamp`（秒 ×1000） | **每事件真实时间**——v1 同会话共享一个近似时间的局限就此解除 |

`tool_call_update` 是状态/展示更新（kind、title、locations），编辑载荷在 tool_call 已完整，跳过即可。kind 无 create/delete 信息，EditKind 维持 Modify（与 v1 一致）。

## 四、覆盖与兼容

- 本机 23 会话：19 有 updates.jsonl，4 旧会话只有 chat_history——**双源必须共存**：会话发现优先 updates，缺则退 chat_history（v1 loader 保留为兜底）。[实证]
- 会话起点：updates 首行 timestamp（真实）优先，退 uuid v7 近似。[实证]
- started_at 与事件 ts 都来自信封后，grok 的 uuid v7 解析只剩兜底职责。

## 五、关键结论

1. method 分类学成立：两流职责切分干净，内容检索只读 `session/update` 流即可，无跨流去重负担。[实证]
2. grok 的合成闸门在 updates 里换了形态：`hideFromScrollback`（chat_history 里是 `synthetic_reason`）——四家闸门第四种形状，同构不同名。[实证]
3. `_meta["x.ai/tool"].kind` 是写族判定的权威信号，比名字清单稳（新工具名出现只要 kind 标 write/edit 就能收）。[推断: 机制外推，本机未见反例]
4. 升级收益三件：真实逐事件时间戳（时间线不再同会话共点）、compaction 改写免疫（权威日志 append-only）、rawInput 免二次解析。[实证]
