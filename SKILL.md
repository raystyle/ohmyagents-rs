---
name: ohmyagents
description: 通用智能体多路复用任务编排器。在 rmux 上把多路终端 agent 编进一个项目会话：拉起、状态、委派、带产物等待的任务、自愈信任、轨迹检索。任务目录协议给 agent 提供读提示词写产物的稳定契约。
---

# Oh My Agents

Rust 单二进制 CLI（命令 `oma`）。把项目目录变成多路 agent 会话：claude / codex / grok / kimi 各占一路窗格，oma 管拉起与和解（活路附加、死路重开）、状态与阻塞告警、任务委派与产物等待。运行时后端 rmux；编排三通道 CLI / HTTP / MCP；`oma serve` 主页即多路窗格看板。

## 何时使用

- 要并行委派多路 agent 做同一项目的事，并看各路状态与阻塞告警。
- 要**带产物等待**的任务：`oma task` 发出后 oma 阻塞等产物文件出现，产物即命令输出。
- 要检索某 agent 在本项目改了什么、基于什么意图（`oma trace` 四家联邦读）。

## 命令

```text
oma spawn [--agents a,b] [--stub]     拉起或重连会话（精确集合：给几路就几路）
oma status                            各路 pid/进程/终端态/hook 态
oma send <agent> "<文本>"             单路委派（任务开始确认，阻塞打 send.alert）
oma task <agent> "<文本>" [--timeout N]  带产物等待的任务（见任务目录协议）
oma task list | show <id>             任务清单与产物查看
oma run "<文本>" [--assign a,b]       状态门分派（闲路才发）
oma settle [--wait N]                 自愈信任/审查框
oma respawn <agent>                   强制重开一路
oma key <agent> <KEY>                 发单键（守卫：codex 拒 C-c）
oma trace sessions|timeline|blocks|agent|file|search   轨迹检索
oma serve start|stop|status           看板守护化（127.0.0.1，打开即多路窗格）
oma web [agent]                       web 镜像链接（官方域走 PIN 与警示）
oma mcp                               MCP server（stdio）
oma cleanup                           只杀本会话
```

六会话命令加 `--json` 出 `{ok, data|error, meta}` 信封（与 HTTP/MCP 同形）。

## 任务目录协议

`oma task <agent> "<文本>"` 在 `<project>\.ohmyagents\tasks\<id>\` 建任务目录并阻塞等待。收到委派的 agent 按此协议操作：

1. **读**：提示词全文在 `prompt.md`（send 文本只带尾注，文件才是权威）。
2. **写**：产物写到同目录 `output.md`，先写完整内容。
3. **完成标记**：最后创建空文件 `DONE`——oma 只认 DONE 不认 output 存在（防半写误判），顺序不能反。

oma 等 DONE 出现后打印 `output.md` 全文退出；超时（缺省 600s，0 无限）任务目录保留，产物晚到可用 `oma task show <id>` 收取。

### 收件人模式：等另一个 agent 的任务产物

不要前台死等（会占住会话）。挂后台 watcher，DONE 出现即收：

```bash
while [ ! -f ".ohmyagents/tasks/<id>/DONE" ]; do sleep 15; done
cat ".ohmyagents/tasks/<id>/output.md"      # 产物到手，继续处理或报告
```

要点：只等 DONE 不等 output.md（半写不算完成）；间隔 10-15s 足够（产物不赶秒级）；中途随时 `oma task show <id>` 查进度。

## 输出契约

- marker 行：`命令.键=值`（如 `spawn.attached=claude,codex`、`task.done=t001`），稳定可解析。
- 告警行：`send.alert=` / `spawn.alert=` / `settle.<agent>.stalled=` 走 stderr——任务未启动、阻塞框、顽固屏需要人工跟进。
- 退出码：0 成功；1 业务失败（含 task 等待超时、search 无命中语义）；2 参数错。

## 示例

```bash
oma spawn --agents codex              # 单路 codex 全屏
oma task codex "review src/orch.rs 并把结论写产物"   # 阻塞等产物
oma task codex "..." --timeout 0 &    # 后台无限等
oma task list                         # 看哪些任务完成了
oma run "跑一遍构建"                   # 闲路才发
```
