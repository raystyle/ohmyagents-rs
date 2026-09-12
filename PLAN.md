# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：D29 oma 更名 HST / hst / v0.6.0 破坏性一次做完

> 依据：用户方案全文（PRD D29 第 0 轮，2026-09-12）；D28/P0045 用户级注册与 session 分键协议不变（本迁移只换标识与根）；M060 guard 白名单语义随迁；ohmycloud 侧批切对齐（镜像开 hst/ 段、oma/ 兼容期保留）。七阶段：

1. **crate 与标识**：Cargo.toml `hst-cli`（bin 与 lib 名 `hst`）、main.rs `oma::` 改 `hst::`、fmtio 信封 tool/app 字段改 hst、版本 0.6.0。禁止 crate 仍叫 oma 只改 README。
2. **命令面**：`hst hook init|status|verify`（hook 面 init / 原 stdin 状态入口 status / agents verify 的 hook 层）；`hst statusline` 一级化（`agents statusline` hidden alias 一个小版本后删）；`hst trace` 六视图、`hst agents [list|verify]`、`hst init [--yolo|--project-yolo|--pretrust|--project]`、doctor/diagnose/self/skill/completions 原面。
3. **路径与 env**：默认根 `~/.hst` 加 `HST_ROOT` 覆盖；启动探测迁移（`~/.oma` 在且 `~/.hst` 缺：同卷 rename，跨卷 copy 加校验删）；`hst init` 并入 heal（四家托管条目 `~/.oma` 改 `~/.hst`、`oma` 改 `hst`，is_ours 签名只动自家，用户手写不碰；doctor 残留 warn）；`HST_MIRROR/HST_GATEWAY_URL/HST_GATEWAY_KEY` 新读加旧 `OMA_*` 一个版本内读并 stderr 提示；测试缝 `HST_HOME/HST_USER_HOME/HST_TRACE_HOME`；win bin 目录 `%USERPROFILE%\.hst\bin`。
4. **资产与自更新**：资产 `hst-<target>`；release 同挂 `oma-*`（stub bin：警告后转调 hst）；mirror job 双推 `oma/` 与 `hst/` 段一个版本；update.rs 资产名与仓库指向 hst-rs。
5. **文档**：README 首句全称加与 Hipo/hst history picker 的 PATH 共存注（装用户目录不覆盖 /usr/bin/hst）、AGENTS/R001/R002/R011/INDEX/SKILL/COMMAND_MAP/skillgen 全换、P0046 归档、CHANGELOG 0.6.0。
6. **测试**：assert_cmd bin 名、hook 落盘快照路径、迁移三用例（有旧无新 / 两者都有 / 只新根）、八条验收清单、三平台矩阵。
7. **发版**：repo 改名 hst-rs（ark 先例）加 remote 更新、tag v0.6.0、CI 资产、digest、herdr 知会对齐。
