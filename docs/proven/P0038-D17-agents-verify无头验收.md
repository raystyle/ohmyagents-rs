# D17 oma agents verify 无头验收子命令

- 状态：已完成（本机四家实跑全绿，125 单元加 21 集成全绿）
- 日期：2026-09-08
- 关联：PRD D17；S033（文档加源码双层取证）；R004 测试分层；M045 / M053

## 背景与问题

用户提出 hook 和状态栏要做全平台无头 agent 运行测试验收。D40 三端验收覆盖了现有命令面，但「agent 真跑时 hook 事件流与状态栏是否生效」缺一个可重复的无头验收器。S033 两轮取证（文档层加三家开源源码级）钉死判据底座：状态栏四家全部 TUI-only 无头不可验、判据只能两层分离；hook 无头可验但有各家门禁（codex 信任闸静默跳过、grok folder trust、kimi SessionEnd print 不触发）。

## 方案

第 3 轮裁定（用户「继续吧」采纳建议）：形态 = `oma agents verify [名...] [--timeout N]` 子命令；hook 押 state 落盘（SessionStart 先于模型调用，不依赖 token 有效性）；状态栏押脚本直跑 mock JSON 断言；平台先本机 Windows 加 WSL；token 最小消耗（极短 prompt，模型应答不作判据）。

## 落地与验收

- `src\verify.rs`（726 行）：层 1 状态栏 `pwsh -NoProfile -File oma-statusline.ps1 <agent>` 喂 `{}` 断言首行 `<agent>:` 标记；codex 断言 `[tui] status_line` 内置项标 `builtin`。层 2 临时 git 项目 in-process 部署 hook 后拉无头（claude `-p --dangerously-skip-permissions`、codex `exec --dangerously-bypass-hook-trust`、grok `-p --always-approve --cwd`、kimi `-p`），断言 `.oma/state/<agent>.json` 落盘加四态合法。binary 不在记 skip 不计败；fail 退出 1。
- 本机四家实跑 [实证： 2026-09-08]：claude/codex/grok/kimi 的 statusline 与 hook 全 ok（hook.state 观测 idle 与 working）。
- 测试 [实证]：单元 116 升 125（argv 形状、state 解析、marker 断言、codex 内置项、退出码汇总等 9 件）；集成 18 升 21（dies_verify_unknown_agent、全 skip 退 0、live 闸门验收四家真跑）；四门禁绿。

## 坑与处理

1. grok folder trust 门禁：临时目录未信任时项目源 hooks 静默不发；verify 种子 `~/.grok/trusted_folders.toml` 条目加 Drop 摘除（已信任目录不动） [实证： 本机实跑]。
2. kimi 无 oma 项目级注册面（deploy_kimi 只落 skill）：verify 往临时项目 `.kimi-code/config.toml` 追加 `[[hooks]]`，现版 print 模式正常触发 [实证]。
3. codex 项目目录信任闸：部署侧已有 `[features].hooks` 加 trusted_hash 预种，加 bypass 旗标即通 [实证]。
4. Git Bash 构建环境坑（GNU link shadow 加 MSYS2 LIB 转换）记 M053，与本需求无关但挡复验。

## 经验

- 无头验收的判据要押「先于模型调用的事件」：SessionStart 在 session create 时即发，token 无效、网络失败都不影响，验收器自身永远不烧有效 token 也能判 hook 链路 [实证： 本轮四家]。
- 「状态栏是否经 TUI 渲染」与「无头验收」互斥：四家状态栏都是 TUI footer 构件（S033 源码坐实），无头面只能验脚本本体；TUI 渲染归人工冒烟。
- 子代理实现加父代理复验的拆分本轮顺畅：brief 带齐取证事实（命令行、门禁、判据）后零返工。
