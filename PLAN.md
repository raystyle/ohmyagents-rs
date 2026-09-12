# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：D31 至 D33 ohmycloud 协调批（v0.6.2）

> 依据：PRD D31 / D32 / D33（ohmycloud 外部协调来函 2026-09-13，来函即澄清）。五段：

1. **D31 域名清扫**：四处明文摘除（`src\diagnose.rs` 注释、README、R002 2.5 节、P0041 背景），表述统一「api 缓存回归测试端点（配置注入）」；代码端点本就 env 加 agent 配置注入（`HST_GATEWAY_URL` 覆盖序），无硬编码不改逻辑；全仓 grep 复扫归零。
2. **D32 旗标连字符化**：clap `pretrust` 参数显式 `long = "pre-trust"` 加隐藏 `alias = "pretrust"`（兼容窗口到 v0.7，与 OMA_* env 同批清）；kv 标记 `init.pretrust.*` 不动（机器面冻结）；全仓 md 的 `--pretrust` 批量更正（含历史档案：旧拼写仍为可用别名，命令逐字可复跑无失真，更正口径记 diary）。
3. **D33 yolo 分级**：`yolo.rs` 加 `YoloLevel`（full/partial/off，clap ValueEnum）；两级旗标改取值式 `--yolo[=<级别>]` / `--project-yolo[=<级别>]`（缺省 full，裸旗标兼容）；写入矩阵：claude full=bypassPermissions 加 skip 加 enableAll、partial=acceptEdits 加 skip（MCP 归 trust 面不写）、codex full=danger-full-access/never、partial=workspace-write/on-request、kimi yolo/auto、grok always-approve/auto（取值证据 grok-build `permissions.rs` canonical 值集）；off=按 ours 等值摘除（新 `retire_user_yolo_with`，项目面 `retire_project_yolo` 扩认 partial 值），空文件删除、用户自设值保留；doctor 判据分级接受（partial 不误报，双级冲突 warn 沿用）；COMMAND_MAP init 行更新。
4. **测试与门禁**：yolo.rs 单测（partial 形状、full 转 off 退役、用户自设值幸存）加 doctor partial 判据用例加 cli 集成（`--yolo=partial` 标记与落盘、`--yolo=off` 退役、`--project-yolo=off`、`--pre-trust` 新拼写与旧别名双跑、非法级别退出 2）；全量测试加 fmt/clippy 加 rumdl 加 .tools 三扫描；dogfood `hst init` 重生 SKILL 加 `hst skill --write`。
5. **发版与归档**：版本 0.6.2、CHANGELOG 里程碑、R002/R007/R001/AGENTS/INDEX/README/S007 追记随批；推 main CI 绿后 tag v0.6.2、镜像 hst/stable 段到货核验、herdr 知会 ohmycloud；P0048 归档加 diary。
