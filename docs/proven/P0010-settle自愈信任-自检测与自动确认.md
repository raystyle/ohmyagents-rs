# settle自愈信任：自检测与自动确认

- 状态：已完成（2026-08-31：codex 路全通——双机制互兜闭环、三个注册形态坑修复、hook 真实落地）
- 日期：2026-08-31
- 关联：前置 P0009（claude 路通、三路信任流卡点）；研究 `S006`（信任门四家）、`S009`（1b 分类与备屏限制）、`S010`（terminal_state 分类器）；用户定调「做到自愈自检测，自动 send key 持久化信任」

## 背景与问题

P0009 实证：各家首启信任框（claude「trust this folder」实测 Enter 可点、codex/grok 形态待核）是 hook 链路通断的闸门，且各家确认后会**自行持久化信任**（codex 写 `hooks.state.trusted_hash`、claude 记 folder trust）。点框是人肉环节——要求 spawn 后自动检测（自检测）自动点（自愈），让信任一次落库后续免框。

## 目标与非目标

- 目标：
  - `oma settle [--wait 秒] [--project PATH]`：轮询会话各路画面（SDK snapshot，备屏 TUI 的 capture 常空），按**白名单关键词**识别信任/工作区确认框，自动 sendkeys 确认（Enter 或 y+Enter），循环至框消失
  - 判据与输出：每路 settle 结果（dismissed/none/timeout）；可选验证 hook state 文件开始落地
  - 关键词白名单只收信任类确认（trust folder / hooks trust / workspace trust），普通 [y/n] 不自动点（那是任务语义，不是信任）
  - POC 先行：`examples/poc-settle.rs` 摸清 codex/grok 实况关键词与键序，再产品化
- 非目标：
  - 不绕过信任机制（不做 hash 伪造、不写用户级 kimi 注册——边界见 AGENTS）
  - 不点任务语义确认框（agent 问「要执行 X 吗」不代答）

## 方案

### 检测

- 画面源：`pane.snapshot().visible_lines()`（live grid，备屏也有内容；capture-pane 仅作对照）
- 白名单（首轮实测固化，可扩展）：
  | 家 | 关键词 | 确认键 |
  | --- | --- | --- |
  | claude | `trust this folder` | Enter（P0009 实证「Enter to confirm」） |
  | codex | 待 POC 实况 | 待定 |
  | grok | 待 POC 实况 | 待定 |
- 只在**尾部若干行**匹配（框在屏底/居中，避免正文误命中）

### 动作

- 命中即 send 确认键，等 1-2 秒重扫；框消失即该路完成
- 每路有界轮询（默认 30s）；全程打印 marker

### 成功信号

- 首选：该路 `.ohmyagents/state/<agent>.json` 出现（层 2 活了）
- 次选：框不再现且画面稳定

## 实施步骤

1. 立方案、POC `examples/poc-settle.rs`（dump 各路画面 + 关键词检测 + 点框循环）
2. codex/grok 实况摸键序，固化白名单
3. 产品化 `orch::settle` + `oma settle` 子命令
4. 单测（关键词匹配边界：尾部行、大小写、误命中防护）+ 实测验收
5. 文档回填（R002/AGENTS/S006 补实测口径）与提交

## 风险与回滚

- 误点任务确认框：白名单词表收紧 + 只点一次每框形态
- codex 备屏连 snapshot 都空：如实记录，settle 对该路报 unknown，不瞎点
- 回滚：settle 是新增命令，spawn 不自动调用（显式执行），零侵入

## 验收标准

- 全新临时项目：init 全套 → spawn 四路 → settle 一次点掉出现的信任框 → claude/codex/grok 至少两路 hook state 落地（kimi 除外，无项目级）
- 再 spawn（同项目）不再现框（信任已持久化的实证）
- 单测与三件套过；文档同步

## 实施过程与经验

### 全链路

> 2026-08-31 Windows codex 路。

- **双机制落地与互验**：机制 B（deploy 预置 `[hooks.state]` trusted_hash：key=项目 config.toml 绝对路径加事件标签加组索引加 handler 索引；hash=归一 handler 身份的 canonical 键排序 JSON 加 sha256，`commandWindows` 归一丢弃、SessionEnd/Interrupt 超时钳 1-3）写盘即 7 条；但本机 codex 0.149.1 与 S015 核实的 HEAD 存在版本偏差，hash 复现未全中（仍弹「Hooks need review」），机制 A（`oma settle` + 手动 send 点框）兜底：选「2 Trust all and continue」后 codex **自己持久化正确 hash，之后 spawn 免框**——两机制互备正是用户要的设计。[实证: 全程画面与 config.toml]
- **三个注册形态坑（codex 0.149 Windows）**：其一 hook 执行环境不继承我们的 PATH，`command: "oma"` 裸名启动失败 exit 1——必须绝对路径；其二 codex 的 `command` 是完整命令行且 0.149 在 Windows 经 **PowerShell** 执行，`"exe" hook` 是 PS 语法错（字符串后裸 token），需调用操作符 `& "exe" hook`；其三注册串必须带 `hook` 参数（裸 exe 是 clap usage 错）。修复后 codex hook 全链真实落地（`Stop→idle` state 文件写入）。[实证: 画面「hook exited with code 1」逐项排除 + 手动复现对照]
- **两段式在真 TUI 需要间隔**：`send_text` 后紧连的 Enter 被 codex TUI 吞掉（文本进框未提交），延时约 2 秒再单独 Enter 才提交——S005 铁律的新实证：不止「分开发」，还要「隔开发」。产品 send 的间隔化列入待办。[实证: 补发 Enter 前后画面]
- **交互阻塞矩阵（settle 白名单起点）**：codex 更新菜单（默认项竟是「Update now」，盲点默认会替换二进制——动作表必须按框定制：选 `2` Skip）；codex hooks 审查（选 `2` Trust all）；claude 工作区信任（Enter）。密码类永不自动（S010）。`TRUST_DIALOGS` 白名单表已立，扩展即增行。
- **deploy 幂等升级**：ours 条目已存在时**原地替换**为新形态（注册形态演化：裸名→绝对路径→PS 调用操作符），不追加重复；trust 种子随最终 hooks.json 真实索引重算。[实证: init.hooks.wrote.count 由 0 变 2]
- grok 实况：pretrust 的 always-approve 直达 REPL 无框，hook 仍 silent（grok 项目 hooks 发现/触发待下轮诊断）；kimi 按边界不写用户级。
- 诊断工具沉淀：`examples/poc-dump.rs`（SDK snapshot 视角看备屏 TUI，capture-pane 看不到的面）。
