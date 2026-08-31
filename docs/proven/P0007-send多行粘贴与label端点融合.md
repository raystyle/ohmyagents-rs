# send多行粘贴与label端点融合

- 状态：已完成（2026-08-31 全链路验收过，含中文多行与自愈）
- 日期：2026-08-31
- 关联：前置 P0006（产品命令闭环）；研究 `S005`（三段式铁律）、`S003`（`-S` 拒绝与 label 命名空间）；实证 `examples/poc-paste.rs`、`examples/poc-label-bridge.rs`

## 背景与问题

P0006 的 `oma send` 只收单行文本：多行任务（给 agent 的长提示词）被拒。三段式粘贴（`load-buffer` + `paste-buffer -p` + Enter 单独发）在 poc-paste 已实证，但走的是独立 label daemon；产品会话用的是 SDK 自造 pipe 名端点，而 Windows CLI 拒绝一切 `-S` 形态、label pipe 名带随机 salt SDK 无法预连接——两条传输面在产品里没打通。

2026-08-31 实证（`examples/poc-label-bridge.rs`）：CLI `-L label display-message -p '#{socket_path}'` 能打出 label daemon 的实际 pipe 全名，SDK `RmuxEndpoint::WindowsPipe(该名)` 可直连同一 daemon（pane 活、locate 到 pwsh.exe）。桥成立。

## 目标与非目标

- 目标：
  - 产品会话端点迁到 label 命名空间（label = `oma-<slug>` 稳定），daemon 由 CLI 语义拉起（Job Object 下 WMI），SDK 经 `#{socket_path}` 查询到的实际 pipe 连接
  - manifest 记录 label 与实际 pipe；connect 失败时按 label 重查 pipe 自愈（daemon 仍在但 salt 名变了的场景）并回写
  - `oma send` 支持多行：含换行即走三段式（临时 payload 文件 + CLI load/paste-buffer -p + Enter 单独发），单行保持 SDK 两段式
  - 中文多行验收：粘贴执行行 marker 可见
- 非目标：
  - 不做 REPL / HTTP / `oma run`（后续方案）
  - 不改 hook/skill 部署（deploy 接 CLI 另行切片）
  - Linux/mac 委托后续仓库

## 方案

### 端点融合（spawn 侧）

1. label = `oma-<slug>`（沿用 P0006 的项目 slug）
2. 启动序列：CLI `-L label list-sessions` 试活 → 失败则 WMI 起 `new-session -d` 的 boot keeper 会话（名 `oma-<slug>-boot`，桩 shell）→ 轮询 CLI 就绪 → `#{socket_path}` 取实际 pipe → SDK `connect(pipe)`
3. SDK 建产品会话（CreateOnly + ProcessSpec env，照 P0006）与布局；产品会话建好后 kill boot 会话（daemon 因产品会话在场不会 exit-empty）
4. manifest 增 `label` 与 `pipe` 两字段

### 连接自愈（status/send/cleanup 侧）

- 直连 manifest.pipe；失败且 label 探测活着 → 重查 `#{socket_path}` → 重连 → 回写 manifest
- label 探测也死 → 报引导 `oma spawn`（不自动重建会话，避免半启动）

### send 多行分支

- 文本含 `\n` 或 `\r`：写临时 payload 文件（UTF-8、无 ESC、不含自包 bracketed 壳）→ CLI `-L label load-buffer -b <buf> <file>` → `paste-buffer -p -b <buf> -t <target>` → Enter 单独 `send-keys`（或 SDK send_key）→ 删临时文件与 buffer
- target 用 `session:%<pane_id>`（与 manifest 的稳定 pane id 同源）；若 rmux CLI 不认该形态，退 `session:0.<spawn 序>` 并在经验节记录
- `--confirm` 语义不变（SDK `expect_visible_text`）
- 单行分支不动（SDK 两段式）

### 测试

- 单测：多行判定、payload 文件写入（无 ESC、UTF-8）、target 构造、manifest pipe 字段往返
- 集成：`tests/cli.rs` 增 send 多行无会话快败（不新增 daemon 依赖）
- 本机验收：stub 会话两格 → 多行中文 send（marker 拼接式防回显假阳性）→ 单行回归 → cleanup

## 备选方案

| 做法 | 取舍 |
| --- | --- |
| 产品全走 CLI（丢 SDK 面） | 否：snapshot/分类、expect 等待、output_stream 都是 SDK 独有，status 与 confirm 依赖它们 |
| 双 daemon（SDK 会话 + CLI paste 会话） | 否：paste 必须贴进产品 pane，临时会话无意义 |
| 逐行 send_text 模拟多行 | 否：无 bracketed-paste 语义，TUI 会把每行当独立提交，正是三段式要避开的坑 |

## 实施步骤

1. 立方案、切 TODO/GOAL（本步）
2. `rmuxpoc` 补 CLI 助手（run_cli、label 探测、socket_path 查询、WMI new-session 泛化）
3. `orch` 端点迁移与 manifest 扩展、connect 自愈、send 多行分支
4. 单测 + cli.rs 快败例
5. 本机全链路验收（含中文多行）
6. 文档回填与提交

## 风险与回滚

- CLI 不认 `%N` target：退序号形态，经验节记录差异
- label daemon 被外杀：自愈探测给明确引导，不自动重建
- 回滚：P0006 单行能力不受影响（多行是新增分支）；端点迁移若不稳，回退自造 pipe 端点只需还原 orch::endpoint

## 验收标准

- stub 会话：多行中文 send 后执行输出 marker 可见（拼接式 marker）；单行 send 回归绿
- 跨命令重连与自愈：手动 kill daemon 后 status 给引导文案；不复活半启动
- `cargo test` 全绿；rumdl 与断链检查过
- TODO/GOAL/INDEX/R002 与磁盘一致；diary 记钩子

## 实施过程与经验

### 全链路（2026-08-31，Windows 绿）

- **桥的形状**：产品端点从「SDK 自造 pipe 名」迁到 CLI label 命名空间（label=`oma-<slug>` 稳定）。daemon 由 CLI 语义拉起（Job Object 下 WMI `new-session -d` boot keeper 会话），就绪后 `display-message -p '#{socket_path}'` 取实际 pipe 全名（含用户 SID 与随机 salt，只能查询不能推导），SDK 以该名直连同一 daemon。此后一个 daemon 双传输面：SDK 做 snapshot/expect/输入，CLI 做 `load-buffer`/`paste-buffer`（Windows 拒一切 `-S`，CLI 只认 `-L`）。[实证: `examples/poc-label-bridge.rs` 退出 0 + 本机产品链路]
- **多行 send**：文本含换行即三段式——payload 写临时文件（UTF-8、无 ESC、发送侧绝不自包 bracketed 壳）→ CLI `load-buffer -b` → `paste-buffer -p -b -t %<pane_id>` → Enter 仍单独发（SDK `send_key`）；buffer 与临时文件用后即清。target 用 `%N` 稳定 pane id 直达（与 manifest 同源），无需窗口坐标。[实证: 中文两行 payload，`send.split=paste-buffer-p+Enter`，第二行执行输出 confirm 可见]
- **自愈**：manifest 直连 pipe 失败且 label 探测活着时，重查 `#{socket_path}` 重连并回写 manifest；label 也死则报「run `oma spawn`」引导，不自动重建避免半启动。salt 每次起 daemon 都变，自愈让 manifest 不必永远新鲜。[实证: 手改 manifest.pipe 为假名后 status 正常并回写真名]
- **坑（新）**：boot keeper 会话名起初取 `oma-<slug>-boot`，产品名恰是其前缀——rmux `-t` 是前缀匹配，`reuse_only(oma-<slug>)` 命中 boot 会话误报「已存在」。boot 名改为 `oma-boot-<slug>`（前缀关系反转）解决；spawn 成功后 kill boot 会话，daemon 由产品会话保活。[实证]
- 单行 send 与 cleanup 行为不变（回归绿）；`cargo test` 38 过。
