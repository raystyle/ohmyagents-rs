# PLAN：当前目标实施计划

## 当前目标：会话层 bypassPermissions 未生效排查

> 用户报修 2026-09-02：oma 拉起的 claude 会话 `/permissions` 非 bypass，Allow 规则堆积（审批仍在弹）即实锤；`oma doctor` 配置层全绿（`permissions.defaultMode=bypassPermissions` 落在项目 `.claude/settings.json`）——问题在会话层不在配置层。

### 方案骨架

1. **研究**（claude-code-guide 代理）：claude 2.1.24x 的权限模式解析优先级——`--permission-mode`/`--dangerously-skip-permissions` CLI flag、`CLAUDE_CODE_*` env、user `~/.claude/settings.json`、项目 `.claude/settings.json` 与 `settings.local.json` 各层 defaultMode 的胜负关系；`bypassPermissions` 被静默降级的已知情形（managed settings、gateway/自定义 ANTHROPIC_BASE_URL 限制、版本行为变化）；`--dangerously-skip-permissions` 与 defaultMode 的差异。
2. **本机取证**：三层 settings 的 defaultMode 实值；oma spawn claude 路的实际 argv 与 env（`orch.rs`）；oma 会话与用户手拉会话的差异面。
3. **结论落 S029**（六态标注），按结论定 oma 修法；候选（若 defaultMode 在会话层确被忽略）：spawn 的 claude 路固定 `--dangerously-skip-permissions` argv。

### 验收口径

oma 拉起的 claude 会话内 `/permissions` 显示 bypassPermissions（或等效不再弹审批）；`oma doctor` 语义不破；测试守卫不倒退。

### 门禁

`cargo fmt --all -- --check` + `cargo clippy` 存量告警不新增 + `cargo test`（隔离 target）+ `rumdl check .` + `md-ref-scan.py` + `md-heading-scan.py`；提交精确 add（M036）。

> 角色：**当前目标方案文档**——基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
