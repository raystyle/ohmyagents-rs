# R011-Agent友好IO契约-format三态信封退出码与冻结面

> issue #1 总台集成契约的落地件（2026-09-02，与 ome S003 同构取齐；两处分道见下节）。ohmypwsh 总台按本文件消费 oma 输出；字段变更需 breaking 标注（对齐 ohmyenv-rs #4 冻结口径）。

## 三态输出

全局 `--format kv|json|jsonl`，`--json` 为 json 简写（两者互斥，clap 用法错 exit 2），子命令后可用（`oma doctor --json`）。

| 态 | 形态 | 适用 |
| --- | --- | --- |
| kv（缺省） | marker 行（`key=value` 每行一条） | 人读与既有消费者 |
| json | `{ok, data\|error, meta:{command,project}}` 信封单对象 | 机器面（信封形状沿用 P0015 三传输同形；HTTP/MCP 传输面已随 D15 移除） |
| jsonl | 列表逐行对象，无信封 | 列表型（agents 行、doctor finding 行）；非列表命令 jsonl 视同 json |

值一律字符串（ome 同款）；**字段序与 kv 行序一致**（serde_json 开 `preserve_order`；ome S003 实证教训：默认字母序会打乱）。

## 与 ome 的两处分道

> ome 已于 2026-09-12 更名 ark（Agent Runtime Kit，仓 ark-rs）；本节所引 ome S003 为其时实证，口径不变。

> 记档理由。

1. **信封保留**：ome 裁决裸数据；oma 的 `{ok,data}` 信封是既有契约（P0015 三传输同形；传输面已随 D15 移除，形状冻结不变），复用优先于两仓完全同构。
2. **业务失败信封仍进 stdout 且退出非 0**：机器读者从 stdout 信封拿 `ok:false`，结构化模式 stderr 另出单行 `{"code":"error","message":...}` 供人称与日志通道：双通道而非 ome 的 stdout 纯数据。

## 退出码

| 码 | 语义 |
| --- | --- |
| 0 | 成功 |
| 1 | 业务失败 / doctor blocked / hook guard 未涉 |
| 2 | clap 用法错（含 `--json` 与 `--format` 互斥）与 secretguard 阻断（hook 面，stderr 掩码原因） |

## 冻结面

> 本批机器可读命令。D15 收窄后仅余两行（`oma check` / `oma status` / 七会话命令随编排面移除）；self update 的 marker 键只增不改（D16 增 `update.mirror` / `update.source` / `update.fallback`）；statusline 的 marker 键同理只增（D18 增 `statusline.custom` / `statusline.script`，自备脚本在场或刚部署时才打）。

| 命令 | json 数据形 | jsonl 行形 |
| --- | --- | --- |
| `oma doctor` | `{blocked, findings[]}` | 逐 finding：`{agent,check,status,path,detail}` |
| `oma agents` | `{installed,missing,agents[]}` | 逐 agent：`{agent,status,source,path,version,extras[]}`（missing 行带 hint） |

hook / completions 不进 format 面（协议通道各有自己的 stdout 纪律）。

## 验收

集成五测（tests\cli.rs）：信封解析加 blocked 退出 1、jsonl 逐行可解析加首键 agent（字段序契约）、子命令后 `--json` 简写、互斥 exit 2、双通道错误（stdout 信封 ok:false 加 stderr 单行 JSON）。

## 来源

issue #1；ome S003（Agent 友好 IO 研究与重构，含 preserve_order 与值一律字符串两条实证教训）；P0015 信封三传输同形。
