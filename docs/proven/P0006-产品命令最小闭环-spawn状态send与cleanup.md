# 产品命令最小闭环-spawn状态send与cleanup

- 状态：已完成（2026-08-31 本机全链路验收过；REPL/网页/run 留待后续方案）
- 日期：2026-08-31
- 关联：`GOAL.md` 当前目标；前置方案 `P0005-各功能部件POC验证原型.md`（Windows 全表绿）；研究 `S003`（API 与 POC 结论）、`S005`（Drive 铁律）、`S009/S010`（状态判断）、`S015`（hook 注册形态）；参考 `R002`（命令手册口径）、`R006/R007`

## 背景与问题

P0005 十二个部件 POC 已在 Windows 全部跑绿，共用层（专用端点、WMI 退路、CreateOnly、布局、守卫、分类器、deploy）就位但未组装成产品命令。用户指令：继续，即按 P0005 实施步骤 4 开产品命令。

## 目标与非目标

- 目标：
  - `oma spawn`：项目目录上起多路 agent 会话（稳定命名可重连），按布局分格，注入 `OHMYAGENTS_*` 环境，不阻塞
  - `oma status`：只读列出本会话各 pane 的 pid、进程名（层 0 加 locate）、终端语义状态（层 1b）、state 文件（层 2），不 attach
  - `oma send <agent> <text>`：守卫链（locate 进程名、`check_send_key`）加两段式分发（文本、Enter 分发），出短头确认为收
  - `oma cleanup`：只杀本 session，daemon 随 exit-empty 自然退；不 kill-server
  - `tests/cli.rs` 集成测试起步（R004 第一段）：`check` / `agents` / `hook` / `doctor` 只读命令冒烟
- 非目标：
  - 不做 REPL、HTTP 网页观察面（后续方案）
  - 不做多行文本粘贴（三段式 paste-buffer 属下一切片）
  - 不做四路真实 agent 的拉通验收（本机未装齐时 `--stub` 桩替代；真四路待装齐后验）
  - Linux/mac 委托后续仓库

## 方案

### 会话标识与端点（可重连是产品与 POC 的分界）

- 项目根（`--project` 或 cwd）规范化后取 SHA256 前 8 位得 `slug`；会话名 `oma-<slug>`，端点 WindowsPipe `\\.\pipe\rmux-oma-<slug>`（Unix socket 在 temp 下按 slug 建目录）。同一项目反复 spawn/status/send/cleanup 命中同一会话；不同项目互不干扰。
- 连接语义照 rmuxpoc：connect_or_start，Job Object 下 WMI 在 job 外起 daemon 再轮询连接。

### spawn

1. `oma check` 布局闸门（gate）
2. `oma agents` 探测：`--agents a,b` 用指定集；缺省取四家已装交集；一个没有且未 `--stub` 则报错并列出可装
3. `--stub` 用 pwsh 交互桩替代（验收与调试用）
4. 布局：1 路单格、2 路左右、3-4 路 2x2（多出报错）
5. 每格 argv 为该 agent 二进制（无参数进各自 TUI）；env 注入 `OHMYAGENTS_PROJECT` / `OHMYAGENTS_AGENT` / `OHMYAGENTS_STATE_FILE`（指向 `<project>\.ohmyagents\state\<agent>.json`，目录先建）
6. 会话已存在则报「已存在，用 status/send/cleanup」不叠窗格（CreateOnly 语义）
7. 返回前不等待 agent 就绪（无阻塞启动，`doctor`/`status` 事后诊断）

### status

- 层 0：pane 存在与 pid；locate：批量 CIM 反查进程名
- 层 1b：`classify_snapshot` 四态映射
- 层 2：state 文件存在则读出 state 与 ts，缺省标 silent
- 输出 marker 行（`status.pane.<agent>.…`）加人读表

### send

1. 守卫链：会话在、pane 活、pid 反查进程名含期望 agent 名（或桩 pwsh）、`check_send_key`
2. 文本单行约束（v1）：含换行即拒绝并提示后续切片
3. 分发：`send_text` 与 `send_key("Enter")` 两次（铁律：禁同发）
4. 可选 `--confirm <marker>`：短头可见才算送达（`expect_visible_text` 带超时）

### cleanup

- kill 本会话（transport 断开按已死处理）；不 kill-server；`--project` 决定杀哪个 slug

### 测试（R004 细则）

- 单元：slug 稳定性（同路径同 slug、异路径异 slug）、argv/env 构造、send 单行守卫
- 集成 `tests/cli.rs`：assert_cmd 冒烟 `check --no-install`、`agents`、`hook`（无 env 静默退出 0）、`doctor`（临时目录）；断言只写退出码与 stdout marker 行
- 选型：assert_cmd 与 predicates 为 clap 官方生态标准件（crates.io 高下载、持续维护），走 R005 最少代码接入

## 备选方案

| 做法 | 取舍 |
| --- | --- |
| spawn 后 wait_ready 再返回 | 否：P0005 结论无阻塞启动，阻塞误判伤委派 |
| 会话名用 pid 隔离（同 POC） | 否：产品要跨命令重连，pid 每次变 |
| send 全量走 paste-buffer 三段式 | 推后：Windows 上 CLI 端点链路属下一切片，v1 单行两段式已覆盖主要委派 |

## 实施步骤

1. 立方案、切 GOAL/TODO/INDEX（本步）
2. `src/orch.rs`：slug 与端点、spawn/status/send/cleanup 四函数
3. `main.rs` 子命令接线（spawn/status/send/cleanup）
4. 单测 + `tests/cli.rs` 集成
5. 本机验收：`--stub` 全链路（spawn 到 status 到 send 到 cleanup）；`oma check` / `doctor` 不回归
6. 文档回填：R002 命令手册从设计口径改实测口径、diary 记钩子

## 风险与回滚

- 会话已存在与残留：CreateOnly 报错文案引导 cleanup；不动用户其它 rmux 会话
- agent 未装即 spawn：探测先行报错，不半启动
- 回滚：删除会话名 `oma-<slug>` 即恢复；产品命令独立于已绿的 POC 层

## 验收标准

- `oma spawn --stub --project <tmp>` 退出 0 且不阻塞；`status` 列出桩格 pid、`pwsh.exe`、idle 或 unknown、state=silent
- `oma send claude echo OMA-SEND-OK --stub --confirm OMA-SEND-OK`（桩格）短头可见
- `oma cleanup` 后 `status` 报会话不存在；不 kill-server
- `cargo test` 全绿；`tests/cli.rs` 四命令冒烟过
- R002、INDEX、TODO、GOAL 与磁盘一致；diary 记钩子

## 实施过程与经验

### 全链路（2026-08-31，Windows 绿）

- **可重连身份是产品与 POC 的分界，落法是双稳定锚**：会话名/端点按项目路径 SHA256 前 8 位派生（`oma-<slug>`、`\\.\pipe\rmux-oma-<slug>`），跨命令重连；pane 定位不靠窗口坐标（`session.pane(row,col)` 语义随布局漂移），spawn 时记 **daemon 稳定 pane id** 进 `.ohmyagents\session.json` manifest（agent→pane_id，daemon 生命周期内稳定），status/send 用 `pane_by_id` 重验证。[实证: spawn 两次调用间 status/send/cleanup 均命中同一会话]
- spawn：缺省取 `oma agents` 已装交集；`--agents` 显式指定，未装先报错不半启动；`--stub` 用交互 shell 桩。第一格走 `ensure_session` 的 `ProcessSpec`（env 用 `environment: Vec<"K=V">`），后续格走 split builder `.spawn(argv).env(k,v).title(agent)`——split 的 env 是逐键链式，与 ProcessSpec 的整体 Vec 是两套写法。[实证: 源码 split_builder.rs + 本机 spawn --stub 两格 %0/%1]
- env 注入三键落地：`OHMYAGENTS_PROJECT` / `OHMYAGENTS_AGENT` / `OHMYAGENTS_STATE_FILE`（指向 `.ohmyagents\state\<agent>.json`），`oma hook` 的沉默安全带由此生效。
- status 四层合流：pid（层 0）、批量 CIM 进程名（locate）、`classify_snapshot` 四态（1b）、state 文件（层 2，沉默标 silent）——S009 权威链的产品化首秀。[实证: status 输出 terminal=idle hook=silent]
- send 守卫链：`check_send_key`（键策略）→ manifest 定位 → `expect_process`（locate，桩期望 pwsh、真 agent 期望自身名）→ 两段式；多行文本 v1 直接拒绝。`--confirm` 用 `expect_visible_text` 等短头。[实证: send.confirm=OMA-SEND-OK]
- cleanup：`reuse_only` 拿会话后 kill，transport 断开按已死处理；manifest 随删；cleanup 后 status 的报错文案引导 `oma spawn`。[实证: cleanup.killed=true，后续 status 报 no session manifest]
- 集成测试起步（R004 第一段）：assert_cmd/predicates（clap 官方生态标准件，R005）5 例——check/agents/hook 静默/doctor 空目录退出 1/send 无 manifest 快败；断言只写退出码与 marker 行。全套 `cargo test` 38 过。
- 教训两笔：先写实现后核 API 会臆造方法名（`spawn_argv`/`panes()`/`find_pane_by_text` 全是没核实的），`PaneId` 也不是 u64——写 SDK 调用前先 rg 源码签名，与 M024/M025 同族；CLI 子命令用 current_thread runtime 就够（无并发 future），不必 rt-multi-thread。
- 偶发未定位：全量 `cargo test` 首轮 1 例 lib 失败（截断未见名），连跑两次 33+5 全绿；疑为并行测试临时目录毫秒碰撞，留观复发再修。
