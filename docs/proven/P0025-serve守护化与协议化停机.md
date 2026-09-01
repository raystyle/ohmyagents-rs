# serve 守护化与协议化停机

- 状态：已完成（2026-09-01 当日达成，全生命周期实测绿）
- 日期：2026-09-01
- 关联：`S023`（rmux 进程模型：协议化自杀）；用户定调「serve 类命令应直接退到后台 daemon，后续 serve start/stop 管理」＋「stop 按 pid 杀？学习 rmux 没有更优雅的 daemon 方式？」

## 背景与问题

`oma serve` 是阻塞前台命令——用户要的形态是 `oma serve start` 即调即退、`stop`/`status` 管理后台。且 stop 不该粗暴 taskkill：rmux 的 daemon 停机走 IPC 协议化自杀（`kill-server` 请求 → handler 置 AtomicBool → 主循环轮询 → 优雅排空 lifecycle hook → 自退出），不靠外部杀。

## 方案

- **守护化**（`src\servectl.rs`）：`serve_start` DETACHED 孤儿化拉起 `oma serve-daemon`（隐藏子命令，Windows `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP`，日志重定向 `~/.ohmyagents/serve/<slug>.log`），drop(child) 故意不 wait（rmux 同款），轮询端口就绪（10s）后返回地址；状态记 `~/.ohmyagents/serve/<slug>.json`（pid/port/project/started_at）。
- **协议化停机**（server.rs）：`DELETE /shutdown` 置 `ShutdownFlag`（Arc<AtomicBool>），`axum::serve().with_graceful_shutdown()` 轮询到后排空在途请求退出——**与 rmux kill-server 同构**（IPC 面 → flag → 主循环自杀）。`serve_stop` 先发 HTTP shutdown 等退出，超时/不可达才降级 taskkill 兜底。
- **命令面**：`oma serve start [--port N] [--project]`（已活直接返回地址秒回）/ `oma serve stop` / `oma serve status`（pid/port/live）；裸 `oma serve` 保留前台（调试）；REPL 内嵌 serve 不变。
- **pid 探活**：Windows FFI `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION) + GetExitCodeProcess`（`STILL_ACTIVE=259`）——**不能 tasklist**（见坑）。

## 验收标准与结果

- start 即退（<10s 内返回 addr）；再 start 已活秒回同地址；status 报 pid/port/live=true；页面 200。[实证]
- `DELETE /shutdown` → 日志见 `draining` → 端口关闭；`oma serve stop` 全链（HTTP 优先 + 兜底）；stop 后 live=false、端口 closed、记录清除。[实证]
- 测试基线：75+10 / 78+10 全绿（隔离 target）。[实证]

## 次轮纠偏补记

- 首版 `serve_stop` 实际只有 taskkill 直杀，本节「HTTP 优先 + 兜底」当时属超写（`DELETE /shutdown` 端点已实现但 CLI 未接线）。次轮补齐：`serve_stop` 先发 `DELETE /shutdown`（ureq，本就是装机链非可选依赖，featureless 构建同样优雅）、轮询 pid 退出（5s）、超时才降级强杀。实测 stop 后日志尾 `serve: shutdown requested; draining`、live=false，协议化路径真走通。[实证]
- 教训：验收节标 `[实证]` 前必须核对**命令面**真消费了该路径——端点存在不等于命令接线（G002「没验证写成已验证」的变体：部件验证过、链路没验证）。

## 实施过程与经验

- **tasklist 死锁坑（本日最硬）**：DETACHED 子进程 spawn 后，等待循环里用 `tasklist /FI ... .output()` 探活——宿主在 Claude Code 的 Job Object 内，**子进程的 stdout 管道在 Job 内被塞住**，`.output()` 永远等不回来 → serve start 阻塞不动。换 FFI OpenProcess 直查（零子进程、零管道）即解。教训：**Job Object 环境下一切 `.output()` 式子进程等待都是雷**，探活类需求优先 FFI。
- rmux 的停机值得抄的是**语义**不是代码：IPC 面 → 标志 → 主循环自杀 → 排空。oma 的 IPC 面是 HTTP，`DELETE /shutdown` 就是 oma 的 kill-server；S023 的进程模型研究直接兑现到产品。
- `drop(child)` 孤儿化在 Windows 上依赖 DETACHED flag（不设则子进程挂控制台，父退子亡）；日志必须重定向文件否则 DETACHED 下 stdout 无处去。
