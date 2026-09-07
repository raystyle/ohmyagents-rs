# PLAN：当前目标实施计划

## 当前目标：D12 根下 `.ohmyagents/t006/` 孤儿目录收敛

> 回指 `PRD.md` D12。`.ohmyagents/` 已被 gitignore，属项目本地数据。

### 事实基线

- `tasks/t006/` 是第一轮 review：FINDINGS=13，基线 HEAD=064ca87，有 DONE [实证: 目录与 task.json]
- `.ohmyagents/t006/output.md` 正文是 t008 第二轮 review（FINDINGS=12，HEAD=3652180），与 `tasks/t008/output.md` 只差两处措辞 [实证: git diff --no-index 4 行]
- 现行 `task.rs` 协议尾注用 `id = format!("tasks/{id}")`，发出路径已是 `.ohmyagents/tasks/<id>/`，不是缺 `tasks/` 的产品回归 [实证: src\task.rs]

### 方案骨架

1. 以 `tasks/t008/output.md` 为第二轮正本，不把孤儿覆盖回去。
2. 删除 `.ohmyagents/t006/`（仅 output.md 一份误落）。
3. 单测钉协议尾注含 `.ohmyagents/tasks/`，防止以后再写成 `.ohmyagents/{id}/`。

### 验收口径

- `.ohmyagents/t006/` 不在。
- `tasks/t006/` 与 `tasks/t008/` 原产物不动。
- 协议路径单测绿。

### 门禁

`cargo test --lib task`；触碰 md 过 rumdl 与 mdcharlint。

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
