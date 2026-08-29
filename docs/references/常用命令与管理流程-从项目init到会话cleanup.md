# 常用命令与管理流程：从项目 init 到会话 cleanup

> AGENTS 意图路由的细则。已落地：`oma check`、`oma init --yolo`、`oma doctor`、`oma agents`、`oma hook`。其余命令仍是设计口径。
> 显示名 Oh My Agents；仓库 `ohmyagents`；CLI 二进制 `oma`（对照 ohmypwsh 的 `omp`）。运行时数据目录仍是 `.ohmyagents`。

## 环境

新终端若 PATH 缺新装工具，Windows 先重建：

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

`oma check` 会检测 rmux：版本必须是 pin（`catalog/rmux.toml`，现役 `0.10.0`），校验完整包布局与哈希；没有就按 GitHub 资产安装到本机数据目录。四路 agent 用 `oma agents` 扫 PATH、自定义路径和环境变量，不只看 PATH。版本读取用 `rmux -V`，不要 `--version`。

Windows pane 最小 POC（本机已绿；Linux/mac 后续委托）：

```powershell
cargo run --example poc-endpoint
cargo run --example poc-session
cargo run --example poc-layout
cargo run --example poc-drive
cargo run --example poc-dialogs
```

宿主若在 Job Object 内，`connect_or_start` 会 os error 5；example 自动用 WMI 在 job 外拉起 daemon。结束只 `kill-session`。

## 命令

| 意图 | 命令 | 说明 |
| --- | --- | --- |
| 核对依赖 | `oma check` | 检测 rmux pin 版本与哈希；缺则安装完整包；打印路径/版本/sha256 |
| 只诊断 rmux | `oma check --no-install` | 缺失或哈希/版本不符则非 0，不下载 |
| 无阻塞诊断 | `oma doctor [--project PATH]` | 只读 yolo 键、信任库、已装二进制、`.ohmyagents/state`；不 attach。任一项 `status=block` 则退出 1 |
| 检测已装 agent | `oma agents` | 扫 PATH、`OMA_AGENT_PATH`、`OMA_<AGENT>_BIN`、各家默认目录；打印 `source=env|path|default` 与 version。缺装不退出非 0 |
| hook 写状态 | `oma hook [event]` | 各家 hook 的 `command`。读 stdin JSON（`hook_event_name` / `hookEventName`）或参数。写 `OHMYAGENTS_STATE_FILE`。无该环境变量则 exit 0。不连 rmux |
| 部署项目级 yolo | `oma init --yolo [--project PATH]` | 写 `.claude/settings.json`（`defaultMode=bypassPermissions`）、`.claude/settings.local.json`（顶层 `skipDangerousModePermissionPrompt`）、`.codex/config.toml`（sandbox/approval）、`.kimi-code/config.toml`（`yolo`）。不含 hook/skill |
| 预写信任库 | `oma init --yolo --pretrust [--project PATH]` | 额外写用户家：claude.json trust、codex projects、kimi workspace-trust、grok trusted_folders；grok 的 `permission_mode` 只能写 `~/.grok/config.toml` |
| 权限模式 | `oma init --permission-mode auto\|yolo\|manual` | 覆盖默认 yolo；manual 不写 bypass |
| 开会话 | `oma` | spawn 默认不阻塞 CLI + 打印 URL + REPL；不自动打开浏览器 |
| 无网页 | `oma --no-web` | 不起 HTTP |
| 尝试打开浏览器 | `oma --open` | opener 失败只警告 |
| 本会话 yolo flags | `oma spawn --yolo` | 单次覆盖；有配置落盘时可省略 |
| 委派任务 | `oma run <task> --assign claude,codex` | 写任务到 agent 映射后 drive；一路 blocked 不堵其它路 |
| 一次性发送 | `oma send all\|claude\|codex\|grok\|kimi "..."` | 复用已有 session |
| 状态 | `oma status` | 读项目内 state 与任务指向 |
| 收尾 | `oma cleanup` | 只 `kill-session`，不 `kill-server` |

## REPL

```text
> all <prompt>
> claude|codex|grok|kimi <prompt>
> status
> web
> quit
```

`quit` 只 detach。拆会话用 `cleanup`。

## 文档检查

```powershell
rumdl check .
```

研究与测试文档的事实性断言还要标六态（实证 / 推断 / 经验 / 记忆 / 假设 / 直觉），标准见 `docs\research\guide.md`。`rumdl` 不检查六态，写作者自查。

## 提交

一次一件事：`feat:` / `docs:` / `fix:` / `chore:` + 中文。未经指示不推远端。
