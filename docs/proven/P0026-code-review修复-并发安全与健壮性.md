# code review 修复：并发安全与健壮性

- 状态：进行中（2026-09-01 立项）
- 日期：2026-09-01
- 关联：codex review 任务产出（2026-08-31 经 `oma send` 委派，trace 收取）；用户定调「高+中全修」
- 输入：15 条发现（高 5 / 中 7 / 低 3），高严重度 5 条经本仓逐条核实（4 真 1 部分真）

## 背景与问题

用户用 oma 委派 codex 对 `src\orch.rs`、`api.rs`、`server.rs` 做 code review。codex 产出 15 条发现；本仓抽查高严重度全 5 条确认基本属实，集中在四类：跨进程并发与写入原子性、恢复路径僵局、同步阻塞占 tokio worker、无鉴权本地 HTTP 面的 Host 注入。用户定调高+中全修（12 条），低 3 条记档不做（理由见边界）。

## 方案：三切片

### 切片 1：安全与僵局

- **高5 Host 校验**：`server.rs` home/share 端点不再信任请求 Host 拼 frontend_url——校验只允许 `127.0.0.1:<port>` / `localhost:<port>` / `[::1]:<port>`，其余 400（堵 DNS rebinding 偷 share token）。
- **高2 cleanup 僵局**：`orch.rs` `connect(root, false)` 在 manifest 缺失/损坏时不再直接报错，改走既有 label 兜底（label 活则 `label_socket_path` 建 link）；仅 label 也死才报「run oma spawn」。`read_manifest` 区分「文件不在」与「解析失败」（后者带错误上下文）。
- **高3 陈旧 pane**：`reconcile`/`respawn` 死路 split 前先 kill 旧 pane（pane 仍在而 pid 死/进程错位时）；kill 失败视为错误，pane 已不存在则放行。
- **高1a manifest 原子写**：`write_manifest` 改 temp 文件 + rename（同目录原子替换），杜绝半写文件。
- 顺手低13：serve 里重复的 `ensure_web_assets_at` 复用首次结果。

### 切片 2：并发与语义

- **高1b task id 撞号**：`next_task_id` 弃 scan-then-increment；改目录扫描只在命名冲突时重试，或时间戳基。同号并发写不再互覆。
- **高4 阻塞包装**：`run_cli`/`run_cli_checked`/`process_names` 等同步子进程与 fs 段包 `tokio::task::spawn_blocking`；`ensure_label_daemon` 的 `thread::sleep` 改异步等待。handler 不再占死 worker。
- **中8 reconcile stub 语义**：判活用本次 `plan.stub`（非 `m.stub`）；manifest 回写同步计划值；明确「补缺不移除已有路」语义进文档（`--agents` 子集时保留多余路是设计行为）。

### 切片 3：健壮性批

- **中6 send baseline**：发送前快照当前屏，`expect_visible_text`/`--confirm` 等待「新出现」而非「存在」，消残留误判。
- **中7 slug 加固**：hash 输入加长（sha256 前 16 hex）；小写折叠仅 Windows；canonicalize best-effort。
- **中9 web_share 解析锚点**：URL/PIN/expires 用行首锚点正则；解析失败显式报错而非静默 `-`。
- **中10 status 吞错**：`process_names` 失败进 `meta.warning`，不伪装成 `process: null` 正常态。
- **中11 SSE 错误形态**：`/stream`、`/screen` 启动失败改发 SSE `error` event（不再 JSON 200）；首帧拿不到时带错误标记帧。
- **中12 settle 收紧**：marker 匹配从全屏子串收紧到候选菜单行（含已知菜单结构上下文），降误触。

## 边界：不做与理由

- 低14 Query 反序列化绕信封：axum extractor 层拒绝，自定义 extractor 成本高于收益，记档。
- 低15 manifest version 字段：schema 尚无第二版，等真演进再做（YAGNI）。
- 跨进程文件锁（fs2）：原子写 + 唯一 id 消掉绝大部分竞态后，锁收益变小；若切片 2 后仍有撞号实据再加。

## 验收标准

- 切片逐个 `cargo test`（隔离 target）全绿 + 门禁三件套；每切片独立提交。
- 高2：手工构造「session 在 + manifest 删」→ `oma cleanup` 能清（不再要求先 spawn）。[实证]
- 高5：Host 为非本机值时 home 返回 400；正常 127.0.0.1 访问不受影响。[实证]
- 高3：杀一路 pane 内进程后 `oma spawn`，窗格数不增长（旧 pane 被清）。[实证]
- 其余条目按各自单测/集成测断言（期望值来自独立来源，遵守 R004）。

## 实施过程与经验

### 切片 1 完成

- **验收实测全过 [实证]**：高2——stub 会话删 manifest 后 `oma cleanup` 直接清成功（旧代码死局）；高3——杀 claude 路 pwsh 后 `oma spawn`，`reconcile.claude=respawned pane=5` 且 session pane 集合 {%2,%3,%4,%5}（旧死格 %1 已清，四路四格不堆积）；高5——evil Host 400 / localhost 200；看板只读——home 起镜像改 spectator=true，rmux 输出前缀佐证。测试基线 75+10 / 78+10 全绿。
- **计划外发现（本切片最值钱的一条）**：serve daemon 改 CREATE_NO_WINDOW。DETACHED_PROCESS 零控制台下，daemon 再 spawn 的 rmux CLI（TUI，初始化碰 console）卡死——GET / 起镜像时整个 serve 无响应（连 /api/status 都 000）；前台同代码同 manifest 秒回，对照定位。Windows 后台进程要「有隐藏控制台」不要「零控制台」。
- read_manifest 签名改 `Result<Option<Manifest>, String>`：缺失（NotFound）与损坏（corrupt 带路径上下文）分家，恢复路径才对症。无会话项目的报错从 "no session manifest" 变为更准确的 "session daemon is gone; run `oma spawn`"（cli 集成测试断言随契约更新）。
- taskkill 在 Git Bash 里 `/PID /F` 被 MSYS 路径转换吃掉参数且吞输出后无声失败——杀进程验证用 PowerShell `Stop-Process` 并回读确认。
