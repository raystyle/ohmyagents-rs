# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：D28 状态栏跨项目失效根修

> 依据：S015（四家注册面：claude settings 家族用户级生效、codex 用户层 `~/.codex`、grok 全局 `~/.grok/hooks/*.json`、kimi 仅用户级 `[[hooks]]`）、S025（claude/kimi 状态栏 stdin 带 session 标识、grok 无）、D27/P0044（shim 形态与 M059 注册命令形态）。方案切片：

1. **shim 用户级加 session 分键**（shim.rs）：shim 落 `~/.oma/hooks/`（deploy_shims 收 oma_home）；默认写 `~/.oma/state/<agent>-<session>.json` 加 `<agent>.json` 双写（session 缺省只写后者；grok 用 GROK_SESSION_ID 补）；OHMYAGENTS_STATE_FILE 覆盖优先不动（单文件语义）；SessionEnd 删本 session 键文件（GC）。
2. **注册四家迁用户级**（deploy.rs）：claude `~/.claude/settings.json` hooks 合并；codex `~/.codex/hooks.json` 加 `~/.codex/config.toml` features 与 trusted_hash 预种（key source = 用户 config 路径）；grok `~/.grok/hooks/ohmyagents-state.json`；kimi `~/.kimi-code/config.toml [[hooks]]` 扁平四字段 strict 合并。命令形态沿用 M059 无引号正斜杠与 M048 grok 单路径。
3. **状态栏读序**（statusline.rs SEG_OMA）：env 覆盖优先，次用户级 session 键文件（`session_id`/`sessionId`），次用户级 `<agent>.json`（带会话闸），末项目级旧协议（带会话闸，兼容未迁移端）。
4. **init 迁移清理**（deploy.rs）：`oma init` 部署用户级注册；项目级 ours 注册摘除（claude/codex/grok），`.oma/hooks/` 三件 ours shim 删除；skill/AGENTS 项目级部署不动。
5. **doctor/verify 跟随**：doctor hooks.form 与 state 检查面迁用户级（项目残留提示重跑 init）；verify 改临时用户级注册加 Drop 还原（kimi guard 泛化），判据文件经 OHMYAGENTS_STATE_FILE 隔离到临时目录。
6. **发版**：测试门禁全绿加 herdr codex review 后 v0.5.4，digest 交对端验锚。
