# D19 trace 六视图全量恢复

- 状态：已完成（116 单元加 18 集成全绿，本仓真数据六视图冒烟实证）
- 日期：2026-09-08
- 关联：PRD D19；原方案 P0013 / P0014；研究 S018 / S019 / S020；删除侧归档 P0035（D15）

## 背景与问题

用户问「oma 应该有针对项目的 agent 对话历史 trace 功能，为什么不在了」。答：D15 去编排时 trace 被归类为编排观察面连坐删除（`eeead00` 整删 `src\trace.rs` 1387 行），但 trace 直读四家 agent 原生会话库，与 rmux 运行时零耦合，删除是分类裁定不是技术连坐。用户裁「全量恢复」。

## 方案

三点裁定（PRD D19 第 1 轮）：六视图全量（sessions / timeline / blocks / agent / file / search）；定位口径为只读检索面回归，定位文补「对话历史检索」一域；依赖面自 `eeead00^` 原样带回，`glob` crate 重接，适配 D15 后 main.rs 结构。

## 落地与验收

- 代码：`src\trace.rs` 自 `eeead00^` 原样带回（S018 归一化、S019 联邦设计、S020 grok updates 主源全部修正在内）；`lib.rs` 加回 `pub mod trace;`；`Cargo.toml` 重接 `glob = "0.3"`（regex 与 serde_json 仍在）；`main.rs` 重接 `TraceCmd` 六子命令枚举、`cmd_trace` 与 `print_block_timeline`。
- 测试：`tests\cli.rs` 带回空项目冒烟（sessions 加 timeline 零计数，退出码断言）；`help_lists_the_deploy_surface` 命令面契约扩 trace。
- 验证 [实证： 2026-09-08 本机]：116 单元（trace.rs 内 14 件随恢复回归）加 18 集成全绿，四门禁绿；本仓真数据冒烟六视图全活（sessions=6 含本会话、timeline 带双意图、search 命中 STATUSLINE_PS1、file 追 src\main.rs 修改史）。
- 文档同步（命令面变化四处加定位）：deploy.rs COMMAND_MAP 加行、`oma init` 重生 SKILL、R002 加六行与全表行修正、AGENTS 本质节加只读检索域加意图路由加行、INDEX 代码表加 trace.rs、R001 本质节加「对话历史检索」条。

## 经验

- 归类连坐是删除面裁定的固有风险：trace 服务于编排场景但与编排后端零耦合，D15 按「场景归属」删、D19 按「依赖归属」恢复。删除清单评审要多问一句「它真的依赖被删的东西吗」 [实证： 本轮恢复零适配成本，仅 main.rs 接线]。
- 原样带回优于重写：git 历史里的最终版已含全部坑修（uuidv7 近似、append-only 容错、字符截断防 panic），重写等于把 S018/S019/S020 重踩一遍。
- `--project` 传相对路径 `.` 会因 slug 失配查不到会话（claude slug 规则是完整路径串替换）；传绝对路径即正常。此为既有行为非回归 [实证： 本轮冒烟对比]。
