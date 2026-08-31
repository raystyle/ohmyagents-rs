# 常用命令与管理流程：从项目 init 到会话 cleanup

> AGENTS 意图路由的细则。已落地：`oma check`、`oma init --yolo`、`oma doctor`、`oma agents`、`oma hook`、`oma spawn`、`oma status`、`oma send`、`oma cleanup`（P0006 最小闭环，2026-08-31 实测）。REPL、HTTP 网页、`oma run` 仍是设计口径。
> 显示名 Oh My Agents；仓库 `ohmyagents`；CLI 二进制 `oma`（对照 ohmypwsh 的 `omp`）。运行时数据目录仍是 `.ohmyagents`。

## 环境

新终端若 PATH 缺新装工具，Windows 先重建：

```powershell
$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User')
```

`oma check` 会检测 rmux：版本必须是 pin（`catalog/rmux.toml`，现役 `0.10.0`），校验完整包布局与哈希；没有就按 GitHub 资产安装到本机数据目录。四路 agent 用 `oma agents` 扫 PATH、自定义路径和环境变量，不只看 PATH。版本读取用 `rmux -V`，不要 `--version`。

Windows pane 最小 POC 十二件（本机全表绿；Linux/mac 后续委托）：

```powershell
cargo run --example poc-endpoint      # 专用 pipe 与 WMI 退路
cargo run --example poc-session       # CreateOnly / ReuseOnly / 只杀本 session
cargo run --example poc-layout        # 2x2 split_with + argv
cargo run --example poc-drive         # send_text 与 Enter 两段式
cargo run --example poc-dialogs       # hook 写 blocked + sendkeys 点掉
cargo run --example poc-paste         # 全 CLI -L 三段式中文粘贴
cargo run --example poc-locate        # pid 反查进程名，错位 throw
cargo run --example poc-stream        # output_stream Oldest 回放 Now 直播
cargo run --example poc-state         # terminal_state 分类，Quiet 不当 idle
cargo run --example poc-init          # 四家 hook/skill 项目级部署
cargo run --example poc-negatives     # C-c Codex 守卫与 daemon-wide kill 负检
```

宿主若在 Job Object 内，`connect_or_start` 会 os error 5；example 与产品命令自动用 WMI 在 job 外拉起 daemon。结束只 `kill-session`。

## 命令

| 意图 | 命令 | 说明 |
| --- | --- | --- |
| 核对依赖 | `oma check` | 检测 rmux pin 版本与哈希；缺则安装完整包；打印路径/版本/sha256 |
| 只诊断 rmux | `oma check --no-install` | 缺失或哈希/版本不符则非 0，不下载 |
| 无阻塞诊断 | `oma doctor [--project PATH]` | 只读 yolo 键、信任库、已装二进制、`.ohmyagents/state`；不 attach。任一项 `status=block` 则退出 1 |
| 检测已装 agent | `oma agents` | 扫 PATH、`OMA_AGENT_PATH`、`OMA_<AGENT>_BIN`、各家默认目录；打印 `source=env|path|default` 与 version。缺装不退出非 0 |
| hook 写状态 | `oma hook [event]` | 各家 hook 的 `command`。读 stdin JSON（`hook_event_name` / `hookEventName`）或参数。写 `OHMYAGENTS_STATE_FILE`。无该环境变量则 exit 0。不连 rmux |
| 部署项目全套 | `oma init [--project PATH]` | yolo 键加 hook/skill 部署：`.claude/settings.json`（yolo 加 hooks exec form）、`.codex/hooks.json` 加 `config.toml`（yolo 加 features.hooks）、`.grok/hooks/ohmyagents-state.json`、四家 skill 目录、AGENTS/CLAUDE.md（仅缺失时）。幂等合并保留外条目，不改家目录 |
| 部署项目级 yolo | `oma init --yolo [--project PATH]` | 仅无阻塞键：`.claude/settings.json`（`defaultMode=bypassPermissions`）、`.claude/settings.local.json`（顶层 `skipDangerousModePermissionPrompt`）、`.codex/config.toml`（sandbox/approval）、`.kimi-code/config.toml`（`yolo`）。不部署 hook/skill |
| 预写信任库 | `oma init --yolo --pretrust [--project PATH]` | 额外写用户家：claude.json trust、codex projects、kimi workspace-trust、grok trusted_folders；grok 的 `permission_mode` 只能写 `~/.grok/config.toml` |
| 权限模式 | `oma init --permission-mode auto\|yolo\|manual` | 覆盖默认 yolo；manual 不写 bypass（设计口径） |
| 拉起会话 | `oma spawn [--agents a,b] [--stub] [--project PATH]` | 项目专属会话（`oma-<slug>`）里按布局拉 1-4 路 agent，缺省取已装交集；注入 `OHMYAGENTS_PROJECT/AGENT/STATE_FILE`；不阻塞返回；已存在则拒绝叠格 |
| 桩会话 | `oma spawn --stub [--agents a,b]` | 用 shell 桩替代真实 agent（验收与调试） |
| 看状态 | `oma status [--project PATH]` | 只读列各路 pid、进程名（locate）、终端态（1b 分类）、hook 态（层 2，沉默标 silent）；不 attach |
| 发任务 | `oma send <agent> "<text>" [--confirm MARKER] [--project PATH]` | 守卫链（键策略、locate 进程名）后：单行走 SDK `send_text` 与 Enter 两段式；多行（含换行）走三段式粘贴（临时文件 + CLI `load-buffer` + `paste-buffer -p -t %<pane_id>`，Enter 仍单独发，中文可用）；`--confirm` 等短头可见 |
| 收尾 | `oma cleanup [--project PATH]` | 只杀本项目会话并清 manifest；不 kill-server，daemon 随末 session 自然退 |
| 自愈信任 | `oma settle [--wait N] [--project PATH]` | 轮询各路画面（SDK snapshot），白名单匹配信任/审查框自动确认（claude 工作区信任 Enter、codex 审查 Trust all）；密码类永不自动 |
| 开会话（REPL） | `oma` | spawn 默认不阻塞 CLI + 打印 URL + REPL；不自动打开浏览器（设计口径） |
| 无网页 | `oma --no-web` | 不起 HTTP（设计口径） |
| 尝试打开浏览器 | `oma --open` | opener 失败只警告（设计口径） |
| 委派任务 | `oma run "<文本>" [--assign a,b] [--confirm MARKER] [--project PATH]` | 状态门分派（层 2 有则用，沉默走 1b，仅 idle 过）：一路 blocked/busy 跳过并报告不堵其它路；发出路写 `.ohmyagents\tasks\tNNN.json`（id 递增，assigned 记实际发出路与时间戳）；多行文本走三段式；全拦退出 1 |

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

研究与测试文档的事实性断言还要标六态（实证 / 推断 / 经验 / 记忆 / 假设 / 直觉），标准见 `docs\guide\G002-研究标准细则-结构与六态标记.md`。`rumdl` 不检查六态，写作者自查。

## 提交

一次一件事：`feat:` / `docs:` / `fix:` / `chore:` + 中文。未经指示不推远端。
