# oma-run委派与任务映射

- 状态：已完成（2026-08-31 stub 全链路验收过：双路分派、置忙跳过、任务文件）
- 日期：2026-08-31
- 关联：前置 P0006/P0007（spawn/status/send 全就位）；研究 `S009`（分层权威链与委派顺序）、`S005`（发送铁律）；参考 `R002`（设计口径）

## 背景与问题

`oma send` 一次发一路且不看状态。编排器的本义是「一条任务分派给多路、一路 blocked 不堵其它路」（R002 设计口径）。状态判断（S009 四层）、守卫链、两形态发送都已产品化，缺的只是把它们组装成委派命令加层 3 任务映射。

## 目标与非目标

- 目标：
  - `oma run "<任务文本>" [--assign a,b] [--confirm MARKER] [--project PATH]`：把任务分派给会话内 agent，写层 3 任务文件 `.ohmyagents\tasks\<id>.json`
  - 委派前逐路状态门（S009 顺序）：hook 态有则用（idle 过、blocked/working 拦），沉默走 1b 终端态（idle 过、其余拦）；被拦的路跳过并报告，不堵其它路
  - assign 缺省 = 会话全路；未在会话的路报错
  - 多行文本自然走三段式（复用 send）
- 非目标：
  - 不自动 spawn（无会话报引导，保持显式）
  - 不做 `--force` 忽略状态强发（重发全文是 S005 禁区）
  - 不做任务生命周期（完成回调、结果聚合）——层 3 v1 只记指派

## 方案

### 状态门

对每路取 `orch::status` 的 `hook_state` 与 `terminal`：

| 判定 | 条件 | 动作 |
| --- | --- | --- |
| 可发 | hook=idle；或 hook 沉默且 terminal=idle | 走 send |
| blocked | hook=blocked 或 terminal=blocked | 跳过报 blocked |
| 忙 | hook=working 或 terminal=working/unknown | 跳过报 busy（不重发全文） |

### 任务映射

> 层 3。

- 目录 `<project>\.ohmyagents\tasks\`；文件 `<id>.json`：`{ "id", "text", "created", "assigned": { "<agent>": <ts> } }`
- id：`t` 加三位递增（扫现有 max+1，例 `t007`）；原子写（tmp+rename）
- run 结束打印 `run.task.id`、`run.sent`、`run.skipped=<agent:reason>`

### 流程

1. connect（自愈）→ 读 manifest → 解析 assign（缺省全路）
2. status 一轮 → 逐路状态门
3. 可发路逐个 send（守卫链内含）；confirm 透传
4. 写任务文件（只含实际发出的路；全被拦则不写并退出码 1）
5. 退出码：至少一路发出为 0；全拦为 1

## 备选方案

| 做法 | 取舍 |
| --- | --- |
| 无会话自动 spawn | 否：半启动违背显式原则，spawn 还要选 agents |
| working 路排队等 idle | 否：v1 保持即发即报；等待调度属后续 |
| 任务文件含每路结果 | 否：发送成功不等于任务完成，层 3 v1 只记指派 |

## 实施步骤

1. 立方案、切 TODO/GOAL（本步）
2. `orch` 加任务文件读写与 `run` 函数（状态门 + 分派 + 层 3）
3. `main.rs` 接 `run` 子命令
4. 单测（状态门判定、任务 id 递增、文件往返）+ `tests/cli.rs` 无会话快败
5. 本机 stub 验收：两路 idle 发双路、人工置忙跳过、blocked 跳过、全拦退出 1
6. 文档回填（R002/AGENTS/INDEX）与提交

## 风险与回滚

- 1b 分类对 TUI 备屏可能 unknown → 全拦退出 1 引导诊断，不误发
- 回滚：run 是新增命令，不影响既有四命令

## 验收标准

- stub 两路：run 双路收到（confirm marker 可见）、任务文件 assigned 含两路
- 置忙一路：`run.skipped` 含原因、另一路照发、退出 0
- 无会话：报引导退出非 0
- `cargo test` 全绿；三件套文档检查过；R002/AGENTS/INDEX/TODO 同步

## 实施过程与经验

### 全链路

> 2026-08-31 Windows stub 绿。

- 状态门就是 S009 权威链的直接落地：`gate(hook_state, terminal)`——层 2 说话则赢（idle 过、blocked/working/unknown 拦），沉默走 1b（仅 idle 过）。单测锁死十种组合。[实证: gate_layer2_wins 单测]
- 实证「一路忙不堵其它路」：置忙 claude（长 Sleep 使 1b 判 unknown）后 run，`run.skipped=claude:busy`、codex 照发且 confirm 可见，t002 的 assigned 只含 codex；退出 0（至少一路发出）。[实证: 本机 stub 链路]
- 层 3 任务文件：`.ohmyagents\tasks\t00N.json`（扫目录 max+1 递增），`assigned` 只记实际发出的路与时间戳；全拦则不写文件且退出 1。原子写 tmp+rename 对齐 `oma hook` 的 state 写法。[实证: t001 双路 / t002 单路]
- send 失败（守卫链 throw）按「跳过并报原因」处理而不是中断整轮——与 blocked/busy 同一出口。
- 小坑两笔：`Box::leak` 换 `'static` 是内存泄漏坏味道（skipped 原因改 `String` 即可，勿为类型方便泄漏）；`entry.path().file_stem().and_then(to_str)` 临时值悬垂要 `.to_string()` 落绑。
- 全套 `cargo test` 41 过（lib 35 加 cli 6）。
