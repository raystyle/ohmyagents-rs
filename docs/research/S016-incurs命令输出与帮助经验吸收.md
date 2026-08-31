# S016-incurs命令输出与帮助经验吸收

> 2026-08-31。用户两次点名（docs.rs 页 + GitHub 仓 + 「深度研究 Rust 实现」）。方法纠偏记录：首轮只抓了 docs.rs 模块页即被用户打断——本仓方法是一手源码优先（S015 先例），已浅克隆 `douglance/incurs` 补齐双层源码研究：TS 原型（`src/*.ts`，Rust 注释自证 Ported from）加 Rust 实现（`crates/incurs/src/`，crates.io 0.5.3 同仓发布）。回答：oma 的命令、输入输出、帮助该吸收什么。

## 需求

- 研究：incurs 作为「为人类与 AI agent 双读者设计的 CLI 框架」，其输出信封、格式化、帮助生成、命令组织的模块经验。
- 意图：吸收模式进 oma（命令/IO/help），非引依赖。

## 稳度与选型裁决

[实证: crates.io API 2026-08-31] incurs 0.5.3（2026-08-17 发布），下载 845 / 近期 743，单维护者（douglance），Rust 移植自 wevm/incur（TS）。MIT。

**裁决：不引依赖，吸收模式。** 理由：一人早期项目量级（R005 稳度信号不足）；框架级整体替换本仓 clap 栈不现实；重可选依赖（axum/rmcp/tiktoken/oas3）与 oma 编排器物种不同。其**设计模式**（双读者输出、错误 CTA、命令即 skill）才是可迁移资产。[推断]

## 关键结论

### 1. 输出三层分离

> 源码：`crates/incurs/src/output.rs`（191 行全文）加 `formatter.rs`（792）加 `cli.rs` 的 write 路径（TS `Cli.ts` 1380-1460 同构）。

| 层 | 类型 | 职责 |
| --- | --- | --- |
| 结果 | `CommandResult::{Ok{data,cta,exit_code}, Error{code,message,retryable,exit_code,cta}, Stream}` | 命令产出（数据或错误四件套） |
| 信封 | `OutputEnvelope{result, meta{command,duration,cta,next_offset}}` | 包装：谁跑的、多久、下一步 |
| 呈现 | `Format::{Toon,Json,Yaml,Markdown,Jsonl,Table,Csv}` | 同一数据多面渲染，默认 Toon |

要点：**错误是一等公民**——`ExecuteError{code, message, retryable, field_errors}`（校验错带 path/expected/received），错误也能带 CTA；成功也可带 `exit_code`（包装子进程状态）。[实证: output.rs]

### 2. CTA 块：下一步建议的结构化形态

`CtaBlock{commands:[{command,description}], description}`——每条输出（成功或失败）都可附「建议命令」，人类面渲染成 `Suggested commands:` 列表，agent 面进 meta。命令自动补 CLI 名前缀、折叠 args/options。oma 已有雏形（「no session manifest; run `oma spawn` first」）但未成规范。[实证: output.rs + internal/cta.ts]

### 3. OutputPolicy 与双读者

`OutputPolicy::{All, AgentOnly}`——按命令粒度声明谁能看；`renderOutput = !(human && !formatExplicit && agent-only)`：人类交互且未显式要格式时，agent-only 输出整体不渲染。 oma 的对应问题是反过来的：marker 行是 agent 面，人读面缺。[实证: cli.rs / Cli.ts 1384-1387]

### 4. token 预算与分页

`truncate`（estimateTokenCount + sliceByTokens）与 `meta.next_offset`——agent 读长输出的**续读协议**：`--token-limit N --token-offset M`，截断时附 `[truncated: showing tokens X–Y of Z]`，nextOffset 进 meta 永远可见。另有 `--token-count`（只报数）。oma 输出短，暂不需要；网页观察面做长流时可回看。[实证: Cli.ts truncate]

### 5. 帮助生成

`help.rs`（695 行）手写格式化（非 clap 派生）：header（`name@version — desc`）、Usage、Aliases、Commands（**名字对齐两空格 + 描述**，取 max_len 对齐）、参数/选项/示例/hint/env 分节；root 与 leaf 两形态。价值点：帮助是**人读文档**独立于 clap 结构——oma 用 clap derive 时帮助已够用，吸收点在「命令摘要对齐」观感。[实证: help.rs 81-119]

### 6. 命令即 skill

`sync_skills`：从命令图（含参数 schema）自动生成 SKILL.md 装进各家 agent 的 skill 目录——**CLI 的用法对 agent 可发现**。oma init 现在写死一份 SKILL.md；进化方向是 `oma init` 从 oma 自身命令图生成（spawn/status/send/run/settle 用法表），agent 在会话里就知道怎么查状态、怎么被委派。[实证: 模块 sync_skills]

### 7. 其它模块速览

`command::execute` 一个入口喂 CLI/HTTP/MCP 三传输（oma 的观察面走 rmux stream，不需要）；`filter`（输出路径过滤，jq 风格选择/切片）；`completions`（bash/zsh/fish/nushell，oma 可用 clap_complete 低成本补）；`pager`（人读分页）。[实证: 模块清单]

## oma 吸收裁决表

| 经验 | 裁决 | 落点 |
| --- | --- | --- |
| marker 行（机器面） | 已有，保留默认 | 不动（tests 稳定契约） |
| 错误 CTA 规范 | **吸收**：每个用户可见错误带下一步建议 | R002 输出规范节 + 各错误路径补 `hint:` 行 |
| `--json` 信封 | **吸收（后续切片）**：读命令输出 `{ok,data,meta}` | status/spawn 等加 `--json` |
| 人读表格 | **吸收**：TTY 时对齐表，管道保持 marker | `oma status` 表格化（formatter 手写对齐即可，无 toon 依赖） |
| OutputPolicy | 暂缓：oma 双读者靠「marker+表格」双轨已覆盖 | — |
| 命令即 skill | **吸收（后续切片）**：SKILL.md 由命令图生成 | oma init 进化 |
| token 分页 | 暂缓：oma 输出短 | 网页观察面时回看 |
| completions | **吸收（低成本）**：clap_complete | `oma completions <shell>` |
| 统一执行入口喂多传输 | **核心吸收（2026-08-31 用户定调升级：oma 要 CLI、HTTP API、MCP 三通道编排加网页可视化编排）**：oma 的编排核心已收在 `orch.rs`（Link 结构加六函数），三传输即给它包三个适配前端；incurs 的 `command::execute` 单入口三消费（CLI/HTTP/MCP）是现成参照 | 方案 P0011 |

**裁决升级注记**：本表初稿把 MCP/HTTP 判为「不吸收（物种不同）」，当日被用户定调推翻——oma 不只观察面，要做三传输编排器。incurs 的价值从「输出经验」升为「三传输架构参照」：编排逻辑与传输解耦（oma 已天然如此，orch 不感知 CLI），传输层薄适配。[推断: 架构对照]

## 对本仓的落点

- 本轮实装：`oma status` TTY 人读表格（管道/测试仍 marker 行）+ 错误 hint 规范入 R002。
- 后续切片：`--json` 信封、命令图生成 SKILL.md、`oma completions`。

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| git | `douglance/incurs` 浅克隆（TS `src/` 加 Rust `crates/incurs/src/`） | 2026-08-31 | output/help/formatter/cli 源码 |
| web | crates.io API（845 下载、0.5.3、单维护者） | 2026-08-31 | 稳度四信号 |
| web | docs.rs 模块地图 | 2026-08-31 | 模块清单与结构索引 |
