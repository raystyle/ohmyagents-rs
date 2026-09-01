# agent 实例和解式编排

- 状态：已完成（2026-09-01 当日达成，三态 stub 验收全绿）
- 日期：2026-09-01
- 关联：`S023`（原语模型）；用户两轮定调——「可控：绑定自己的服务、会话、窗口、窗格、shell、PTY，初始检测互斥，操作绑定已开实例不重开」＋「确定 新开、附加、重新打开（关闭打开），oma 的命令不关注窗格只关注 agent 实例，每个 agent 背后绑定这些复杂性」

## 背景与问题

P0019 验收留下的缺口：窗格死了不再绑定（status 报 dead、send 报错，只能 cleanup 全重来）。用户定调的正确形态：**命令面只见 agent 实例**，六级原语（服务/会话/窗口/窗格/shell/PTY）作为复杂性绑在 agent 背后；三态语义——新开、附加、重新打开（关闭打开）。

## 方案

- **`orch::reconcile`**：会话不在→整体新开（原 spawn 路径）；在→逐 agent 判活——**活路附加**（manifest 不动）、**死路/缺路重开**（现有会话主窗格右侧 split 回一路、manifest 回写该条）。取代旧「会话已存在即拒绝」——附加正是防叠格的正确形态（活的绝不重开）。
- **活判据 `agent_alive`**：pane 存在 + `running_pid` 活 + 进程名匹配（stub 记 pwsh、真 agent 记本名——status 的 locate 链同源）。
- **`oma respawn <agent>`**：强制重新打开一路——kill-pane 只打该窗格（不动会话与其它路），split 开新一路回写 manifest；真 agent 沿安装探测路径，stub 会话用 shell 桩。
- **接线三传输**：api::spawn 改和解式（data 带 `attached`/`respawned`）；CLI `spawn` marker 行（`spawn.mode=new|reconcile`）；REPL 起会话同一 api；HTTP `/spawn` 同形；respawn 带 `--json`。
- 窗格 id 只活在 manifest 与诊断输出，命令/UI 全程 agent 名。

## 验收标准与结果

> stub 双路四连测。[实证: 2026-09-01]

1. 新开：`spawn.attached=` `spawn.respawned=claude,codex` `spawn.mode=new`
2. 附加：再 spawn 同——`attached=claude,codex` `respawned=` `mode=reconcile`
3. 死路重开：taskkill 杀 codex 路 pid → spawn——`attached=claude` `respawned=codex`（只重开死路）
4. 强制重开：`oma respawn claude`（活路）——`respawn.pane=4`（新窗格 id）`respawn.scope=pane-only`，终态两路全活
5. 测试基线：74+10 / 77+10 全绿（隔离 target）

## 实施过程与经验

- 「绑定已开实例」的完整链自此闭环：服务（label 探测不重开）→ 会话（reuse_only）→ 窗格（manifest pane_id 复用）→ 死路自愈（reconcile 重开）→ 强制重开（respawn 单路）。P0019 缺口补上。
- `Option<Future>` 不能 `.unwrap_or(false)`（类型不匹配）——条件 await 先 match 出 Future 再 `.await`，或直接分支。
- 重开的 split 基准取会话主窗格（`session.pane(0,0)`）右侧——v1 布局美观度让位于语义正确（活路绝不动，死路从主位补位）。
