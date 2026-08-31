# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

settle 自愈信任（P0010）。**2026-08-31 达成：codex 路全通（双机制互兜、三个注册形态坑修复、hook 真实落地、信任持久化后免框）。** 剩余：grok hook 触发诊断、send 两段式间隔产品化、B 机制 hash 偏差对照（用 codex 写回值逆推）、REPL/HTTP 观察面、incurs CLI 经验研究（用户指定）。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 立项 0010 | 已完成 | 双机制方案（B 直写信任库 + A 自动点框互兜） | 2026-08-31 |
| 机制 B 预置 | 已完成 | deploy 直写 `[hooks.state]`（key/hash 源码级复现，canonical JSON+sha256；真实索引遍历）+ 4 单测 | 2026-08-31 |
| 机制 A settle | 已完成 | `orch::settle` + `oma settle`（SDK snapshot 尾行白名单匹配，点框循环） | 2026-08-31 |
| codex 全链验收 | 已完成 | 修三坑（绝对路径、PS 调用操作符 `&`、带 hook 参数）+ deploy 原地替换；任务执行、state 真实落地、Trust all 持久化后免框 | 2026-08-31 |
| 诊断工具 | 已完成 | `examples/poc-dump.rs`（SDK snapshot 看备屏 TUI） | 2026-08-31 |

（P0006 至 P0010 已完成；过程与经验在对应 proven 方案。）
