# REPL 与编排面内嵌

- 状态：已完成（2026-08-31 当日达成，stub 验收全过）
- 日期：2026-08-31
- 关联：前置 `P0011`（serve/api 就位，REPL 是它们的内嵌消费者）；R002 的 REPL 设计口径（`oma` 裸调用）

## 背景与问题

R002 立项日起的口径：裸 `oma` 进 REPL——spawn 默认不阻塞、打印 URL、不自动开浏览器。P0011 落了 serve 与 api 后，REPL 从「要造的壳」变成「把已有件拼进一个循环」。

## 方案

- `oma` 裸调用（无子命令）进 `src\repl.rs`：顶层 flag `--stub`、`--agents a,b`、`--no-web`、`--open`。
- 会话：manifest 已在则重连（不叠格），否则 `api::spawn` 缺省拉起。
- 编排面内嵌：`server::serve_in_background`（路由表从 serve() 抽出共用，tokio::spawn 后台跑）；端口 7900 被占则顺延试到 7909，全占则警告降级（CLI 通道不受影响）；`--no-web` 关；`--open` 才开浏览器（`cmd /c start` / `xdg-open`，失败只警告）。REPL 退出进程即止，编排面不独立存活。
- 行循环：`all <文本>` 走 `api::run` 状态门分派；`claude|codex|grok|kimi <文本>` 走 `api::send`；`status` 表格（`render_status_table` 从 main 收编 repl.rs，CLI 两处共用）；`web` 打印 URL；`quit`/`exit`/EOF 只 detach。
- stdin 阻塞读放独立线程喂 tokio mpsc：REPL 循环在 `recv().await` 上等输入，每个 await 都给后台 serve 任务让路（current_thread runtime 下必需）。
- 解析器 `parse_repl_line` 纯函数，单测锁行为（路由四形、残缺行与未知命令 None）。

## 验收标准与结果

- 单测：解析器正反例、表格对齐两例（从 main.rs 收编）。过。[实证]
- stub 活体（管道驱动）：banner（session spawned、web URL）、status 表格两路、`all` 双路分派（t001 sent=2）、单路 send、`web` 复述 URL、`quit` detach 退出 0、进程退出后编排面随之收、`cleanup` 收会话。过。[实证]
- MCP 回归：顺手删掉 mcp.rs 冗余 `tool_router` 字段（宏走 `Self::tool_router()` 函数，字段 never-read 警告），stdio 冒烟重跑 9 tools 与 trace 调用仍绿——删前先怀疑、删后必回归。过。[实证]
- 基线：67+10（无 feature）、70+10（server,mcp）全绿，零 warning。过。[实证]

## 实施过程与经验

- REPL 的真正难点是「阻塞 stdin 与异步 serve 同活」：std thread 加 mpsc 一层就解，不必引 crossterm 或 dialoguer——交互面 v1 用普通行读，方向键历史留给未来。
- rmcp 3.1.4 的 `tool_router` 字段是 0.x 时代残留姿势（其自带 calculator 测试仍带着）：路由真实入口是 `#[tool_router]` 生成的关联函数，字段 never-read 警告即证据，删之而冒烟必须重跑。
- 编排面端口顺延（7900-7909）而非失败即弃：多项目同时开 REPL 时第一个占 7900 第二个自动 7901，用户体验对齐「弹不出浏览器不是错误」的宽边界。
