# S029-bypassPermissions会话层失效与命令面注入

> 2026-09-02。用户报修：oma 拉起的 claude 会话 `/permissions` 非 bypass、Allow 规则堆积即审批实锤；配置层 `oma doctor` 全绿。用户定调：bypassPermissions 是 **agent 的命令与配置**——oma init 写的配置对**已存在会话不实时生效**（已活会话不重读 settings），命令面注入才是可靠通道。

## 现象取证

- [实证: 本机 transcript 与 settings] oma pane 会话审批照弹：项目 `.claude/settings.local.json` 堆积 **119 条 Allow**（mtime 停在 13:46 后不再长——旧会话堆积，非当日在涨）；用户手拉的会话（带 flag 起）transcript 77 条 `permissionMode:"bypassPermissions"` 全程 bypass。
- [实证: 无头探针] `claude -p` 令跑 `echo probe-$RANDOM` 回读：带 `--dangerously-skip-permissions` 与不带（靠 user 层 defaultMode）**都真执行**（随机值回读命中）——两种通道在无头下都活。
- [实证: 排除假线索] 曾在 transcript 抓到 `"bypassPermissions is not available when ANTHROPIC_BASE_URL is set"`——时间戳核对全是**自己 grep 的回显**（transcript 自指陷阱）；官方限制清单（见下）无 BASE_URL 条目，排除网关假设。

## 机制

> claude-code-guide 代理取证，官方文档背书（链接在条目内）。

- **模式取值顺序**：`--permission-mode` / `--dangerously-skip-permissions` flag > settings `permissions.defaultMode` > 内置默认（docs permission-modes "Which mode a session starts in"）。
- **settings 栈**：managed > `--settings` > 项目 `settings.local.json` > 项目 `settings.json` > 用户 `~/.claude/settings.json`；`defaultMode` 是标量按栈取最高层，`allow/deny` 列表跨层合并不覆盖（docs settings "Settings precedence"）。
- **v2.1.257 起：项目层与 local 层的 `defaultMode:"bypassPermissions"` 被忽略**（与 `auto` 同等待遇），changelog 原文要求写到 user/managed 或传 `--permission-mode`——oma pane 会话正是「项目层 bypass + 无 flag」形态，命中此条。
- **flag 与 defaultMode 等效且 per-session 优先**；唯一反制是 managed `permissions.disableBypassPermissionsMode:"disable"`（会直接拒绝 flag）与 `--restricted`。
- **Allow 规则堆积 = 从未进过 bypass 的反证**（bypass 下 allow 规则无效、审批不弹）。
- 交互式首启有一次性 dangerous-mode 接受对话框（`skipDangerousModePermissionPrompt: true` 只跳框不是启用前置）；`--resume` 沿用保存时的模式。

## oma 落点

- [实证: 已落地] `plan_agents`（src\orch.rs）claude 路 argv 固定追加 `--dangerously-skip-permissions`——命令面强制通道，免疫 settings 层级与 2.1.257 项目层忽略；别名 argv 追加其后不冲突；`oma respawn` 走同一 plan 重建自动吃到。
- 已存在会话不吃新 argv（用户定调：init 配置面不实时）——**重开该路即生效**：`oma respawn claude`。
- 测试：`claude_argv_carries_bypass_flag_others_do_not`（其它家不沾 flag、stub 路不受影响）；`profile_alias_injects_env_and_argv` 契约更新为 `[bin, flag]`。

## 遗留

- 新机器首启仍会弹一次 dangerous-mode 接受框（user 层 skipPrompt 需首启后才有）——settle 白名单可覆盖。
- codex/grok/kimi 的 yolo 走各自配置面（S007），不进 argv。
