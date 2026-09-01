# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**——基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：code review 修复：并发安全与健壮性

> 对应 `GOAL.md`，方案 `docs\proven\P0026`，登记日 2026-09-01。输入为 codex review（经 oma 委派、trace 收取）15 条发现，高 5 全核实（4 真 1 部分真）；用户定调高+中全修并追加「web 看板默认只读」。

### 1. 闸门

`cargo test`（隔离 target）全绿；门禁三件套裸跑。涉及安全（Host 校验、只读缺省）与恢复路径（cleanup 僵局）的条目必须手工实测，不只靠单测。

### 2. 切片与依据

| 切片 | 条目 | 落点 | 依据 |
| --- | --- | --- | --- |
| 1 安全与僵局 | 看板默认 spectator（用户定调）、Host 校验（高5）、connect label 兜底（高2）、死路杀旧 pane（高3）、manifest 原子写（高1a）、复用 kanban（低13） | `server.rs` home/share 端点、`orch.rs` connect/reconcile/respawn/write_manifest | P0026 方案切片 1；S023 进程模型（pane 语义） |
| 2 并发与语义 | task id 撞号（高1b）、阻塞包 spawn_blocking（高4）、reconcile stub 语义（中8） | `orch.rs` next_task_id/run_cli/process_names/reconcile | P0026 方案切片 2；R004（期望值独立来源） |
| 3 健壮性批 | send baseline（中6）、slug 加固（中7）、web_share 解析（中9）、status 吞错（中10）、SSE 错误（中11）、settle 收紧（中12） | `orch.rs` send/project_slug/settle、`api.rs` web_share、`server.rs` stream/screen | P0026 方案切片 3；S005（发送铁律） |

### 3. 不做与理由

- 低14 Query 绕信封：extractor 层拒绝，自定义成本高于收益。
- 低15 manifest version：schema 无第二版（YAGNI）。
- 跨进程文件锁：原子写 + 唯一 id 后收益小，有撞号实据再加。

### 4. 每片验收

`cargo test` 全绿 + 门禁过 + 独立提交；高2/高3/高5/看板只读按 P0026 验收节手工实测。

## 完成的定义

> 本目标验收口径。

- P0026 十二条（高 5 + 中 7）全部落地或记档理由；每切片一笔提交
- 高2 僵局实测解除；高5 Host 非本机 400；高3 窗格不堆积；看板默认只读（spectator）
- P0026 回填实施过程与经验；TODO/GOAL/INDEX 同步
