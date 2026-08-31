# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

send 多行粘贴与 label 端点融合（对应 `GOAL.md`，方案 P0007，登记日 2026-08-31）。**2026-08-31 达成。** 同日收尾件：`oma init` 接 deploy 层（无 flag 全套部署 yolo 加 hook/skill，`--yolo` 仅键）。下一刀候选：`oma run` 委派与任务映射、REPL + HTTP 观察面、真四路拉通（需装齐）。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 立项 0007 | 已完成 | 方案落位；label-bridge 实证（`examples/poc-label-bridge.rs` 绿） | 2026-08-31 |
| CLI 助手进 rmuxpoc | 已完成 | `run_cli(_checked)`、`label_alive`、`label_socket_path`（`#{socket_path}`/`#{pid}`）、`wmi_new_session`、`ensure_label_daemon` | 2026-08-31 |
| orch 端点迁移与自愈 | 已完成 | label=`oma-<slug>`；boot keeper `oma-boot-<slug>`（前缀坑 M029）；manifest 加 label/pipe；connect 失败按 label 重查回写 | 2026-08-31 |
| send 多行分支 | 已完成 | payload 临时文件（无 ESC）+ `load-buffer` + `paste-buffer -p -t %<pane_id>` + Enter 单独；buffer/文件用后即清 | 2026-08-31 |
| 测试与验收 | 已完成 | cargo test 38 过；本机：中文两行 paste（第二行 confirm 可见）、单行回归、假 pipe 自愈回写、daemon 亡引导、cleanup | 2026-08-31 |
| init 接 deploy 层 | 已完成 | 无 flag 全套（yolo 加 S015 矩阵部署），`--yolo` 仅键；cli.rs init 冒烟两例；cargo test 39 过 | 2026-08-31 |

（P0006/P0007 任务清单随目标完成合并如上；过程与经验在对应 proven 方案。）
