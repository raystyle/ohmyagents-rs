# grok 权威日志升级

- 状态：已完成（2026-08-31 当日达成，验收全过）
- 日期：2026-08-31
- 关联：研究 `S020`（method 分类学与四要素定位）；前置 `S019`（定下升级路径）与 `P0013`（v1 loader）；从 TODO 队列接续（P0012 Linux/mac 待环境切换，本项先行）

## 背景与问题

P0013 的 grok loader 读 chat_history.jsonl：派生缓存（compaction 触发整体重建，历史可被改写）加行无时间戳（同会话所有事件共享 uuid v7 起点近似）。S019 已核实权威日志是 updates.jsonl，留了升级路径（信封 `{timestamp,method,params}`，缺 method 分类学）。

## 目标与非目标

- 目标：grok loader 主源切 updates.jsonl（真实逐事件时间、compaction 免疫）；缺 updates 的旧会话退 chat_history 兜底；四要素抽取与 v1 等价或更好。
- 非目标：不做遥测流（`_x.ai/`）的 hook/turn 检索；不动其它三家 loader。

## 方案

- 会话发现层选源：目录内有 `updates.jsonl` 即权威（started_at 用首行信封秒），否则退 `chat_history.jsonl`（uuid v7 近似）。
- 事件层按文件名分发：`grok_events_from_updates`（新）与 `grok_events_from_chat_history`（v1 保留）。
- updates 抽取状态机（S020 分类学）：只读 `session/update` 流；`user_message_chunk` 过 `hideFromScrollback` 合成闸门、连续拼接、重置 op_intent；`agent_message_chunk` 连续拼接为操作意图；`tool_call` 用 `_meta` 的 `x.ai/tool.kind`（write/edit）判写族（`_meta` 缺失退名字清单），`rawInput` 是现成对象取 file_path/path/target_file 与 new_string/content，ts 用信封秒 ×1000。
- `tool_call_update` 跳过（编辑载荷在 tool_call 已完整）；遥测流整体跳过（无重复载荷，无需去重）。

## 验收标准与结果

- 单测：fixtures 双源各一——updates fixture（遥测行、隐藏分片、read kind、edit kind、tool_call_update、write）全路径断言：选 updates、started_at 首行秒、写族两条、read 不产事件、隐藏分片不污染意图、逐事件真实 ms（1787999900 秒 = 2026-08-29T10:38:20Z 独立来源核对）。过。[实证]
- v1 回归：chat_history fixture 测试原样通过（兜底路径无损）。过。[实证]
- 活体：本仓 grok 历史时间线 8-29 事件 ts 逐秒散开（14:00:40/41/41/42…，不再是同会话共点），双意图与文件路径如旧。过。[实证]
- 基线：62 单测 + 8 集成（无 feature）、65 + 8（server,mcp）两配置全绿。过。[实证]

## 实施过程与经验

- 分类学先行再动刀：S020 用本仓会话 8086 行采样钉死两流职责与四要素位置，实现一次成形——研究先行的又一次兑现。
- 踩坑两枚（同日已在不同形态踩过，同根因聚合）：
  - heredoc 落 fixture 被转义层吃掉 `\\`：Python 源里 `\a` 成 BEL，fixture 路径变 `src\u{7}pp.rs`——与 M034「heredoc 吃引号层」同族，**含转义序列的载荷一律用 Write 工具落文件，不走 shell heredoc**；同日第二次撞 heredoc，教训升格为「fixture 与源码补丁的默认通道是文件工具」。
  - 手写 JSON 括号没配平（首版 fixture 信封少一个闭括号，解析静默失败走了 uuid 兜底）——jsonl fixture 生成后要逐行 parse 自检（Python 生成时顺手做了，第二次就干净了）。
- `relativize` 返回保留原大小写的原始尾段，分隔符形态跟输入走：loader 侧先 `normalize_file` 再 relativize（v1 就这么做，新 loader 保持同序）。
