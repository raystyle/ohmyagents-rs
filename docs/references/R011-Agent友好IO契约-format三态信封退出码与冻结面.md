# R011-Agent友好IO契约-format三态信封退出码与冻结面

> issue #1 总台集成契约的落地件（2026-09-02，与 ome S003 同构取齐；两处分道见下节）。ohmypwsh 总台按本文件消费 oma 输出；字段变更需 breaking 标注（对齐 ohmyenv-rs #4 冻结口径）。

## 三态输出

全局 `--format kv|json|jsonl`，`--json` 为 json 简写（两者互斥，clap 用法错 exit 2），子命令后可用（`oma doctor --json`）。

| 态 | 形态 | 适用 |
| --- | --- | --- |
| kv（缺省） | marker 行（`key=value` 每行一条） | 人读与既有消费者 |
| json | `{ok, data\|error, meta:{command,project}}` 信封单对象 | 机器面（与 HTTP/MCP 三传输同形，P0015） |
| jsonl | 列表逐行对象，无信封 | 列表型（agents 行、status pane 行、doctor finding 行）；非列表命令 jsonl 视同 json |

值一律字符串（ome 同款）；**字段序与 kv 行序一致**（serde_json 开 `preserve_order`；ome S003 实证教训：默认字母序会打乱）。

## 与 ome 的两处分道

> 记档理由。

1. **信封保留**：ome 裁决裸数据；oma 的 `{ok,data}` 信封是 CLI/HTTP/MCP 三传输同形的既有契约（P0015），复用优先于两仓完全同构。
2. **业务失败信封仍进 stdout 且退出非 0**：机器读者从 stdout 信封拿 `ok:false`，结构化模式 stderr 另出单行 `{"code":"error","message":...}` 供人称与日志通道：双通道而非 ome 的 stdout 纯数据。

## 退出码

| 码 | 语义 |
| --- | --- |
| 0 | 成功 |
| 1 | 业务失败 / doctor blocked / hook guard 未涉 |
| 2 | clap 用法错（含 `--json` 与 `--format` 互斥）与 secretguard 阻断（hook 面，stderr 掩码原因） |

## 冻结面

> 本批机器可读命令。

| 命令 | json 数据形 | jsonl 行形 |
| --- | --- | --- |
| `oma doctor` | `{blocked, findings[]}` | 逐 finding：`{agent,check,status,path,detail}` |
| `oma agents` | `{installed,missing,agents[]}` | 逐 agent：`{agent,status,source,path,version,extras[]}`（missing 行带 hint） |
| `oma check` | 单对象报告（source/path/version/pin/sha 族） | 单行退化 |
| `oma status` | api::status（panes[] 加 screen/check 扫屏层） | 逐 pane 行加 warning 行 |
| 七会话命令 | 信封（P0015 既有，`--json` 兼容不变） | 视同 json |

hook / mcp / serve / completions 不进 format 面（协议通道各有自己的 stdout 纪律）。`agents install` 结构化排下批。

## 验收

集成五测（tests\cli.rs）：信封解析加 blocked 退出 1、jsonl 逐行可解析加首键 agent（字段序契约）、子命令后 `--json` 简写、互斥 exit 2、双通道错误（stdout 信封 ok:false 加 stderr 单行 JSON）。

## 来源

issue #1；ome S003（Agent 友好 IO 研究与重构，含 preserve_order 与值一律字符串两条实证教训）；P0015 信封三传输同形。
