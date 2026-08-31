# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

agent 意图操作块与编辑轨迹检索（对应 `GOAL.md`，方案 P0013，登记日 2026-08-31；S018 aitrace 研究已备）。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| S018 aitrace 研究 | 已完成 | `docs\research\S018-aitrace意图轨迹机制研究与oma检索映射.md`（operation_id 归组、双意图、补账队列、裁决表与八坑；七条载荷性断言回源码抽查全中） | 2026-08-31 |
| 立项 0013 | 已完成 | `docs\proven\P0013-agent意图操作块与编辑轨迹检索.md`（五切片；agent 过滤与项目路径显式落库补 aitrace 两缺口） | 2026-08-31 |
| S019 四家会话日志格式 | 待办 | Claude transcript 已知；codex rollout、grok、kimi 会话文件源码定位 | — |
| trace 存储层 | 待办 | `src\trace.rs`：edits.jsonl 追加 + id 去重 + meta 显式落 project_path | — |
| 采集 v1 | 待办 | hook 扩 PostToolUse（Claude 先行）+ 编辑真相源定案 | — |
| 检索面 | 待办 | `oma trace sessions\|timeline\|search`（agent/glob/regex/分页） | — |
| 验收 | 待办 | 真实 Claude 路检索可见 + 双路 stub 同文件不张冠李戴 | — |

## 队列目标

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| P0011 三传输编排面 | 挂起待续 | HTTP API、MCP、网页可视化四切片全待办（方案已立；检索面将来挂 MCP） |
| P0012 Linux/mac 接管 | 环境切换待续 | 资产与代码路径就绪，运行验收待切换环境 |

（P0006 至 P0012 已完成；过程与经验在对应 proven 方案。）
