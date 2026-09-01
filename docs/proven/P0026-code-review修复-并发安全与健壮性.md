# code review 修复：并发安全与健壮性

- 状态：已完成（2026-09-01 三切片当日闭环）
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

### 切片 2 完成

- **高1b**：`next_task_id` 改 `alloc_task_id`——scan 出初值后 `create_new` 原子占位、撞号自增重试；占位只在确有派发时发生（全忙跳过不留空文件）。[实证：单测覆盖占位残留后继续分配不回退]
- **高4（务实分档）**：三个秒级同步段进 `tokio::task::spawn_blocking`——status 的 pwsh+CIM 批查、send/agent_alive 的单 pid 反查、web_share 系列的 rmux web-share 子进程（`web_share_cli` 改按值入参）。**不做**：run_cli 全链 async 化（label_alive 等毫秒级 CLI，10+ 调用点改签名收益不成比例）；`ensure_label_daemon` 的 sleep 循环（仅冷启动低频路径）。记档于此。
- **中8**：reconcile 判活改用本次 `plan.stub`（非 `m.stub`）、manifest 的 stub 随计划回写——stub 会话后跑真 spawn 不再把 stub pane 误判为活路。**语义明确**：reconcile 是「补缺不移除」，`--agents` 子集时已有路保留（设计行为，多余路用 cleanup 或 respawn 管理）。
- 验收：75+10 / 78+10 全绿；daemon 冒烟 home=200、run 端点 200、stop 干净。[实证]
- 小坑：闭包 move 后 `id` 再用于返回值触发 use-after-move——echo 副本先行 clone；target\debug\oma.exe 被残留 serve daemon 锁住构建报「拒绝访问」——杀进程即解（M036 关联：测试完的 daemon 要 stop）。

### 切片 3 完成

- **中6**：send 发送前 snap baseline；`await_new_text` 两路——baseline 不含目标走 rmux 原生静默等待，已含（上轮残留）改轮询快照等「内容变化后目标仍在」；echo 超时降级照发 Enter 留痕，confirm 残留同样不再误报。
- **中7**：slug 16 hex + 平台化小写 + 归一。**实踩两坑**：① canonicalize best-effort 回退在「目录创建前后」算出不同 slug（rm 后 spawn：label 时不存在回退原样、session 时已建又归一，同进程两个身份）——改**纯词法归一**（相对挂 cwd、清 `.`/`..`，零 IO 确定）；② slug 加长使旧 8 位会话失联——已无活会话，一次性清理（kill 旧 label daemon + 删旧 manifest）。实测相对 `.t1` 与绝对 `D:\ohmyagents\.t1` 同 slug、label==session、真看板 home 含 astro。[实证]
- **中9**：web_share 解析行锚点——URL 只认 spectator/operator 行或行首 http 的 token，pin/expires 行首锚定。**次轮实踩自纠**：官方域输出形态是 `rmux:   https://share.rmux.io/#t=`（stderr 前缀行），行首角色锚把官方域 URL 全滤掉（`oma web` 报「没有 URL」）——终版锚改为「URL token 必含 `#t=`」（share token 挂 hash 是 P0021/P0022 实证过的稳定不变量，两种形态都命中）。
- **中10**：status 返回 `(panes, warning)`，进程名批查失败进 `data.warning`/`status.warning=`/TTY 首行告警，不伪装 process=null。
- **中11**：`/screen`、`/stream` 启动失败改 `sse_error_reply`（text/event-stream + error event）；screen 首帧拿不到发 `error` event 不静默空屏。
- **中12**：settle 匹配收紧「行级短行」——marker 须命中单行且 trimmed ≤ 80 列（P0019 三态实测均为短行），正文长行同词不再误触按键。**后续补第四态**：codex hooks 审查屏（`7 hooks need review` / `Press t to trust all`）——oma init 部署的项目级 hooks 首启需 review，白名单加 `("hooks need review", &["t", "Esc"])`（trust all 后 Esc 关面板回工作区），spawn 自动 settle 窗口 6s→10s（该屏在 config 扫描后才出现）。实拍验收：`settle.pane.codex=dismissed=hooks need review:t`。[实证]
- 验收：75+10 / 78+10 全绿；GET /status 带 warning 字段（None=查询成功）。[实证]

## 整案收口

- codex review 高 5 + 中 7 共 12 条全部落地；低 3 条记档不做（低14 extractor 层、低15 YAGNI、低13 顺手并入切片 1）。
- 计划外抓到三个真雷：serve daemon 零控制台卡死（DETACHED→CREATE_NO_WINDOW）、task id 撞号实修（原子占位）、canonicalize 时序双身份（改词法归一）——review 之外的收获大于 review 本身。
- 经验：AI review 的发现要**逐条核实再修**（15 条里高 5 有 1 条部分真）；修的过程中实测冒烟比单测先抓出两个计划外缺陷。
- **当日追加三件**（用户验收看板连发）：①布局自愈——respawn/死路重开的 kill+split 留不规则网格（实测 kimi 独占半屏），reconcile（有重开时）与 respawn 尾部 `select-layout tiled` 一键重排均匀网格，幂等失败只警告；实测弄乱 1x4 横条后 respawn 即恢复 2x2。[实证] ②`oma key <agent> <KEY>` 单键守卫入口（codex 拒 C-c，M039：裸 rmux 绕守卫实杀过一路）。③settle 按后确认（M038：升级屏关掉后旧帧触发重按，「2」落输入框提交成任务）——等 marker 消失（3s）才认成功，顽固不消失不重按、打 `settle.<agent>.stalled` 事件。
- **再追加两件（用户连发定调）**：④**布局按路数适配**——1 路全屏、2/3 路左右列分（even-horizontal）、4 路 2x2（tiled）；⑤**精确集合**（推翻中8「补缺不移除」）：`--agents codex` 就是一路——reconcile 收掉不在计划里的多余路（kill-pane + manifest 移除 + `removed` marker/json 字段），命令面即真身。stub 三形态实测：4 路 2x2 → `--agents claude,codex` 收 grok/kimi 成左右全高两列 → `--agents claude` 收 codex 成全屏。[实证]

## 二轮复核：codex 对修复本身的复核

codex 报告：**无高危**，中 5 / 低 4；验证通过项（cargo test 全绿、无空白错、可编译）。修复批当日闭环：

- **中1 settle 等待语义**（真缺陷）：原「每路首扫未命中即 break」让 wait_secs 只对命中后超时生效，config 扫描后才出现的 hooks 屏永远等不到——重构为**窗口内外层循环**：每轮快扫全部路、命中的当场处理（含按后确认），全空稍歇再扫，窗口全局共享不被一路吃光。
- **中2 CLI 漏 settle**（真）：cmd_spawn 直连 orch 无 settle——补 `orch::settle(&link, &root, 10)`（失败不挡 spawn），三通道口径一致。
- **中3 settle 在 gate 锁内**（真）：HTTP /spawn 持会话锁做 settle 会堵 send/run/cleanup——settle 移出 `api::spawn`，server 侧 gate 释放后独立调（CLI 同步独立调）。
- **中4 tmp 固定名**（真）：write_manifest 的 tmp 加 pid 后缀，并发写不再共用。
- **中5 收路吞 kill 失败**（真）：kill_pane 返回 Err（pane 仍在）时不再移出 manifest——留 `remove-failed` 留痕，避免孤儿 pane 与 relayout 路数错位。
- 低项顺手修：host_is_local 补裸 IPv6 与大小写；write_task 失败清零字节占位。expires token 形态与 relayout 同步 CLI 记档不修（口径一致）。
- 验证：75+10 / 78+10 全绿。[实证]
- 二轮复核的价值实证：一轮修复（settle 自动化）引入或掩盖的问题（等待语义、锁内等待）被同 agent 的二轮抓出——**修复本身也要过复核**。

### 当日验收期追加修复

- **留观条目闭环（二犯当日定位修复）**：看板「画面缩成一团、不自适应」二犯，截屏实锤——**不是偶发**，rmux web-share 前端桌面分支 `scale = Math.min(1, ...)` 只缩不放，120x32 会话按自然字符尺寸（约 960px）渲染，宽屏必然缩在左上角；官方域线上版同样形态（对照实验排除本地包回归）。首次「正常了」属误判（当时窗口/接受度差异）。修复：share-src 桌面分支加 **fit-fill**——字号按容器自适应放大（13→32 封顶，xterm 原生渲染清晰，非 CSS 位图拉伸），残余比例差仍走 transform 缩小；npm 重建资源包（指纹更新，首启自动释放新包）。截屏验收四路铺满视口。[实证]
