# 三传输编排面：http api 与 mcp 与网页可视化

- 状态：进行中（方案已立，实现切片推进）
- 日期：2026-08-31
- 关联：研究 `S016`（incurs 三传输参照与输出经验）；前置 P0006 至 P0010（orch 编排核心全绿）；定位演进自 `R001` / `P0001`（网页从只观察升级为可视化编排）

## 背景与问题

用户定调（2026-08-31）：oma 的编排操作要有**三个通道**——CLI、HTTP API、MCP 接口；浏览器网页做**可视化编排**。原定位「CLI 是编排入口；网页只观察」升级为「编排核心一份，三传输消费，网页可视化编排」。incurs 的 `command::execute` 单入口喂 CLI/HTTP/MCP 三传输是现成架构参照（S016）。

oma 的有利条件：编排核心已收在 `src\orch.rs`（`Link` 结构 + spawn/status/send/run/settle/cleanup 六函数），完全不感知 CLI——三传输只需薄适配层。

## 目标与非目标

- 目标：
  - HTTP API：RESTish 端点覆盖编排六操作（spawn/status/send/run/settle/cleanup），JSON 信封（S016 吸收：`{ok, data|error, meta}`），绑定项目会话
  - MCP 接口：oma 作为 MCP server 暴露同六操作为 tools（任意 MCP 客户端可编排）
  - 网页可视化编排：无构建链单页（浏览器直开），编排操作面板 + 实时画面（stream）
  - 一份编排核心，三传输零逻辑重复
- 非目标：
  - 不做多用户/鉴权（本机工具，绑定 127.0.0.1）
  - 不做 OpenAPI 命令生成
  - REPL 仍走 CLI 通道（后续）

## 方案

### 分层

```text
orch.rs（编排核心，已有：Link + 六函数）
  ├─ CLI 适配（main.rs，已有：clap 子命令）
  ├─ HTTP 适配（新 server 模块：axum，可选 feature "server"）
  │    POST /spawn   GET /status   POST /send
  │    POST /run     POST /settle  DELETE /session
  │    GET /stream/<agent>（SSE，output_stream 桥）
  └─ MCP 适配（新 mcp 模块：rmcp，可选 feature "mcp"）
       六 tools 同签名
网页：docs\web\ 单页（fetch 调 API + SSE 看画面）
```

### 选型

> R005 双通道裁决。

| 件 | 候选 | 裁决 |
| --- | --- | --- |
| HTTP | axum 0.8（核实：0.8.9，crates.io 4.4 亿下载，稳定线） | 事实标准、tokio 同栈、incurs 同款；可选 feature 隔离依赖 [实证: S016 依赖清单 + crates.io 2026-08-31 核实] |
| MCP | rmcp 3.1.4（官方 Rust SDK，已 stable；2310 万下载，2026-08-20 发版，3.1.x 月更节奏） | 立项时记 3.0.0-beta 已过时，2026-08-31 核实订正为 stable——beta 风险解除；feature 隔离保留（依赖面隔离价值不变）[实证: crates.io API] |
| 网页 | 无构建链单页（原生 JS/HTML） | 符合「弹不出浏览器不是错误」的轻边界；不引 node 工具链 |
| 信封 | 手写 `{ok, data|error, meta}` serde 类型 | S016 吸收，无需 toon |

### 切片

1. HTTP API 最小集（六操作 + JSON 信封 + 127.0.0.1）＋ `oma serve` 子命令
2. 网页最小可视化（会话面板：四路状态卡 + send/run 按钮 + SSE 画面）
3. MCP server（六 tools）＋ `oma mcp` 子命令（stdio）
4. 三传输共测：同一项目 CLI/HTTP/MCP 各走一遍六操作
5. 安全边界：仅本机监听；MCP stdio 无网络面

## 风险与回滚

- rmcp 版本漂移：3.1.4 已 stable（2026-08-31 核实），beta 破坏性变更风险解除；feature 隔离保留，核心无损
- HTTP 会话并发写：oma 单项目会话语义下加串行化（一次一命令）
- 回滚：三传输都是可选 feature，关掉即回 CLI 单通道

## 验收标准

- 同一 stub 项目：CLI、HTTP（curl）、MCP（任意客户端或测试桩）三通道各自完成 spawn→status→send→cleanup
- 网页打开即见四路状态并可点按钮委派（stub 验收）
- `cargo test` 全绿；三件套文档检查过；R001/AGENTS/R002/INDEX 同步

## 实施过程与经验

（进行中）
