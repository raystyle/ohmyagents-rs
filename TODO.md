# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

产品命令最小闭环：`oma spawn` / `status` / `send` / `cleanup` 加 `tests/cli.rs` 起步（对应 `GOAL.md`，方案 P0006，登记日 2026-08-31）。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 立项 0006 | 已完成 | `docs\proven\P0006-产品命令最小闭环-spawn状态send与cleanup.md`；GOAL/TODO 切换 | 2026-08-31 |
| orch 模块 | 已完成 | `src\orch.rs`：项目 slug 稳定会话、manifest（agent→pane id）、spawn 布局、status 四层、send 守卫链、cleanup；4 单测 | 2026-08-31 |
| 子命令接线 | 已完成 | `main.rs` 加 spawn/status/send/cleanup 四命令（current_thread tokio） | 2026-08-31 |
| 集成测试起步 | 已完成 | `tests/cli.rs` 5 例冒烟（assert_cmd/predicates 进 dev-deps）：check/agents/hook/doctor/send 快败；全套 cargo test 38 过 | 2026-08-31 |
| 本机全链路验收 | 已完成 | `--stub --agents claude,codex`：spawn（pane %0/%1 不阻塞）→ status（pid/pwsh/idle/silent）→ send（两段式 + confirm 短头可见）→ cleanup（killed + manifest 清 + 后续 status 引导 spawn） | 2026-08-31 |

（P0005 任务清单已随目标完成移除；过程与经验在 `docs\proven\P0005-各功能部件POC验证原型.md`，Windows 范围全表绿 2026-08-31。）
