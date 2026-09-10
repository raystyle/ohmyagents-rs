# D27 hook 与 oma 二进制解耦全平台 shim

- 状态：已完成（140 单元加 22 集成全绿；本机 Windows 加 WSL 双侧实跑实证；随 v0.5.1 发布）
- 日期：2026-09-10
- 关联：PRD D27；M055 / M056 / M057；M047（PS 调用操作符语义）；M048（grok 单路径）；S015（四家注册矩阵）；P0027（codex 字段所有权）

## 背景与问题

用户裁定（2026-09-10 原话）：「项目的 hook 仍然有问题 HOOK 不应该是oma本身 windows系统下agent工作过程中会独占oma进程 不能无痛升级轮换吧」。v0.5.0 及之前项目 hook 注册直接指向 oma 二进制（bare `oma hook --agent X` 或绝对路径），oma 自更新换文件时被 hook 进程占用（Windows 文件锁），且 oma 缺位 / 路径搬迁即 state 通道全盲。

## 方案

- **shim 常量**（`src\shim.rs`）：`oma init` 先落自包含状态写入脚本到 `<project>\.oma\hooks\`，各家注册指向 shim，state 通道零 oma 依赖。
  - `oma-state.cmd`（Windows）：两级形态。部署前探 PATH 上 jq（用户裁：jq 归 ome 部署）：在位落 jq 解析版（字段提取与 state JSON 生成全走 jq `--arg` 传值，程序体零裸双引号绕开 cmd 引号地狱；ts 取 `now|floor` 纪元秒）；缺位落 findstr 回落版（锚定子串替换 `*hook_event_name":=` 剥前缀取首引号段，抗字段序变化不造伪值；grok camelCase 事件名降级 unknown）加 `init.hooks.warn` 指向 `ome install jq`。CRLF 落盘（cmd 对 LF-only 的 goto / label 扫描有历史坑）。
  - `oma-state.sh`（Linux bash / mac zsh 同一语义，shebang 按宿主替换，Unix chmod 755）：sed 提取（POSIX 基线，不引 jq 前置依赖），`date +%s`。
  - `oma-state-grok.cmd`：grok 只认可整串 spawn 的单路径（M048），把 grok 烧进包装再转主 shim。
  - 三份脚本全侧落齐（Windows init 也备 .sh、Unix init 也备 .cmd），共享项目并存；写前比对幂等。
- **注册形态按消费方 shell**：claude / grok（settings.json 经 PowerShell，M047）用调用操作符 `& "cmd 路径" 名`；codex 按字段所有权各侧只写本侧（P0027），commandWindows 为 `& "cmd 路径" codex`（M057 根修，见下），Unix command 为 `"sh 路径" codex`。
- **secretguard fail-open 委托**：shim 先写 state，PreToolUse / UserPromptSubmit 时 `where oma` 在位则 `type payload | oma hook --agent X` 透传退出码（2 = block），不在位放行 exit 0。`oma hook` 保留为手动入口与委托目标。
- **默认落点**：`%~dp0..\state\<agent>.json`（shim 在 `.oma\hooks\` 上一级回 `.oma\state`，与 `oma hook` 回退写盘同位）；`OHMYAGENTS_STATE_FILE` 覆盖优先。
- **陈旧收敛**：merge 的 stale 判据改为「ours 且不等于本次 shim 命令」，老 bare 形态、旧 exe 绝对路径、搬迁后旧 shim 路径、重复条目一律弃后补单条；doctor `hooks.form` 加 shim 读法（bare / absolute 降级为提示升级的 warn）。

## M057 根修随记

M056 的「codex 在 Windows 用 cmd 执行 commandWindows、直引号路径才对」结论是验证通道错误：codex hook 经**会话环境 shell** 执行（openai/codex `core/src/session/mod.rs` 的 `environment.shell.derive_exec_args`，本机 world_state 实证 `shell:"powershell"`，Windows 缺省 PowerShell），直引号路径加参数在 PS 下是 ParserError。同机 codex exec 对照实测：直引号形态三事件全 Failed、`&` 调用操作符形态三事件全 Completed 且 state 落盘带真实 session id。教训：注册形态验收必须真消费方端到端实跑（codex exec / 无头 verify），单 shell 模拟不作数。存量 `~/.codex/hooks.json` 用户级 M056 期直引号 oma 条目仍会 Failed（oma 不动家目录），需手改或删。

## 落地与验收

- 单元：shim 常量标记与形态断言（jq 版零 findstr、回落版 ts=0）、CRLF 规整不双写、写前比对幂等、三文件全侧落齐、deploy 测试改 shim 期望（claude `&` 形态、grok 单路径、codex 双字段、老形态收敛单条、跨侧保留幂等）、doctor hooks.form shim 分类。
- 集成 22 项含 live verify：本机真 codex 无头实跑经 shim + 信任预种落盘绿 [实证： v0.5.1 前夜复测]。
- 本机实跑：jq 版七事件映射（idle/working/blocked/unknown 加 notification 形状区分）、guard 委托真密钥 exit 2、oma 缺位 fail-open、grok 包装、env 覆盖、findstr 回落版字段序翻转用例与缺事件诚实 unknown；WSL bash 跑 .sh 双事件落盘绿 [实证： 2026-09-10 本机]。

## 经验

- 解耦判据先分「短命 spawn」与「常驻」：hook 是百毫克级短命进程，真实痛点不是并发占用而是升级瞬间的文件锁与缺位盲区，自包含脚本一次落盘两侧（脚本本体 + 注册命令）全解。
- cmd 写 JSON 的两条活路：jq `--arg`（程序体零引号）或锚定子串替换；token 位次硬切必造伪值。
- 跨 shell 注册形态没有万能形：按消费方真实验证（PS 面 `&`、cmd 面直引号、单路径面包装），并把验证通道钉在真消费方上（M057 教训）。
