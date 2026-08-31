# AGENTS.md

本文件是协作规则的**最高约束**，四段职责依次为：**项目定位**、**操作规则**、**意图路由**、**资源索引**。

## 一、项目定位

> 本项目的本质与边界。根为定位，下分本质、边界、管理对象、方案索引。

1. **本质**
   - Oh My Agents 是通用智能体多路复用任务编排器：在 rmux 上把多路终端智能体编进一个项目会话，按目录自动配置并编排任务。

2. **边界**
   - 编排钉在启动的项目目录；不替代 ohmypwsh 五端环境总台，不替代各 agent 本体。
   - CLI 是编排入口；网页只观察。弹不出浏览器不是错误。
   - 运行时后端是 rmux，不引入 herdr 当宿主。
   - hook、skill、状态文件只落启动目录，默认不改用户家目录 hook 注册。

3. **管理对象**
   - 可注册的终端 agent（当前默认 claude / codex / grok / kimi，可扩展）。
   - 目标项目目录（cwd 或 `--project`）。
   - rmux 任务会话（专用 pipe 或 unix socket）+ 可选 HTTP 镜像。

4. **方案索引**
   - 定位：`docs\references\项目定位-通用智能体多路复用任务编排器.md`
   - 定位变更：`docs\proven\0004-项目重新定位-通用智能体多路复用任务编排器.md`；上一版 `docs\proven\0002-项目重新定位-通用多Agents自动配置和任务编排器.md`
   - 首期切面：`docs\proven\0001-四路会话工具-CLI控制面与网页观察面.md`
   - 研究：`docs\research\`（文件名即标题，按关键词搜）

## 二、操作规则

> 两类场景：**工作节奏**（何时做什么）与**写作编码**（写什么按什么标准）。每条下分可以与禁止。产品与 rmux 的行为约束不在此层，见意图路由与 `docs\research\`。

### 工作节奏

1. **每轮对话**
   - 可以：先核对三原语 `GOAL.md`、`TODO.md`、`PLAN.md`；实质推进当场更新 todo 与 plan。
   - 禁止：不核对三原语就干活；偏离当前目标；推进了不更新 todo/plan。

2. **踩坑时**
   - 可以：当场落 `docs\mistakes\MISTAKES.md` 一行；主题深挖落 `docs\research\`。
   - 禁止：只留在对话里反复试错。

3. **交付变更时**
   - 可以：改代码同步对应文档，改文档同步索引与 `docs\diary\`；遵守命名标准；技术文档按文档标准细则写。
   - 禁止：只改代码不落文档；改了文档不更新索引。

4. **提交时**
   - 可以：`feat:` / `docs:` / `fix:` / `chore:` 前缀加中文描述；一次提交只做一件事。
   - 禁止：多事混一提交；未经指示推远端。

### 写作编码

5. **执行命令与写文件时**
   - 可以：Windows 命令用 PowerShell 7（`pwsh`），Linux / macOS / WSL 用该平台常规 shell；Markdown / Rust 源码 UTF-8；Windows 上需兼容 5.1 的脚本用 UTF-8 BOM。
   - 禁止：Windows 上默认用 `powershell.exe` 5.1；无 BOM 的中文 ps1 给 5.1 读。

6. **写 Rust 时**
   - 可以：先查 crates.io / docs.rs / GitHub 上是否已有最流行、最稳定、或已经覆盖本需求的库，检索走双通道细则 `docs\guide\选型研究细则-cratesio与github双通道.md`（crates.io 稳度四信号 + gh 流行活跃分辨，结论附证据）；选定后用最少代码接上，优先组合而不是自写协议、解压、HTTP、哈希、CLI 解析。
   - 禁止：在现成库已能稳定完成的前提下从零实现；为风格引入冷门或实验 crate；一次拉一堆用不上的依赖。

7. **写文档时**
   - 可以：遵守 `docs\guide\文档标准细则-命名写作规范与rumdl检查.md`（树形、标题干净、无 emoji 与箭头等装饰符号、文件名即标题、rumdl）。
   - 禁止：标题带括号口号或破折号；整段混杂不成树。

8. **写研究与测试文档时**
   - 可以：事实性断言必须标六态之一——`[实证]`（本机实测）、`[推断]`（逻辑推出）、`[经验]`（历史惯例）、`[记忆]`（待复核）、`[假设]`（待验证）、`[直觉]`（主观倾向）；标准见 `docs\guide\研究标准细则-结构与六态标记.md`；研究与测试的结论断言不标六态即视为未完成。
   - 禁止：把「没验证」写成「已验证」（实证滥用）；断言不标六态；用猜测冒充结论。

9. **写测试时**
   - 可以：遵守 `docs\guide\测试标准细则-分层断言与门禁流程.md`。分层按官方三层（单元 `#[cfg(test)]`、集成 `tests\*.rs`、doctest），集成优先于单元；意图对应方法（冒烟断退出码、回归用黄金文件、验收对照 oracle）；测试名写成可读规格，负例带 `dies_` 前缀；期望值必须来自独立来源（规范、黄金文件、属性），断言只写稳定字段（标记行、退出码），放过 pid 与时间戳；测试体用 `TestResult` 加 `?` 传播错误；rmux 依赖的测试按闸门 skip；测试设施收 `test-util` feature。
   - 禁止：重言式断言（期望值来自被测同款逻辑或镜像实现分支，AI 生成测试高发）；公开 API 测试塞 `mod tests{}` 不进 `tests\`；默认 mock（oma 拿真 daemon）；计时进断言；测试设施无 feature gate 进生产构建；只测 happy path。

## 三、意图路由

> 需求意图与操作方法的映射。命令细则见 `docs\guide\常用命令与管理流程-从项目init到会话cleanup.md`。
> 显示名 Oh My Agents；仓库 `ohmyagents`；CLI 二进制 `oma`。数据目录仍是 `.ohmyagents`。

- **核对照**：`oma check`（rmux 版本 + 哈希 + 完整布局；缺则按 `catalog/rmux.toml` 安装。`--no-install` 只诊断）
- **无阻塞诊断**：`oma doctor`（进程存活 + hook 语义 + 任务指向 + yolo；不把 wait-pane Quiet 当 idle）
- **检测已装 agent**：`oma agents`（PATH、`OMA_AGENT_PATH`、`OMA_*_BIN`、各家默认安装目录；Windows / Linux / macOS）
- **hook 写状态**：`oma hook`（agent lifecycle hook 调用；stdin JSON 或参数；写 `OHMYAGENTS_STATE_FILE`；缺环境则静默。不连 rmux 管道）
- **Windows 最小 pane POC**：`cargo run --example poc-endpoint|poc-session|poc-layout|poc-drive|poc-dialogs|poc-paste`（专用 pipe、CreateOnly、2x2、send_text+Enter、hook blocked + sendkeys、load-buffer+paste-buffer -p 中文）。Linux/mac 委托后续仓库
- **部署项目级 hook/skill/yolo**：`oma init [--yolo]`
- **开会话**：`oma`（REPL，spawn 默认不阻塞）；`--no-web` 不起 HTTP；`--open` 才尝试打开浏览器。阻塞用 `doctor`/`status` 诊断，不在主命令里长时间 `wait_ready` 卡住委派
- **委派**：`oma run <task> --assign …` 或 REPL / `send`。Drive 遵守三段式铁律（发前扫框、`paste-buffer -p`、Enter 单独发，细则见 `docs\research\drive铁律与三段式粘贴.md`）：禁止文本和 Enter 同发、对 Codex 发 `C-c`、发送侧自包 `\x1b[200~`
- **看状态**：`oma status`
- **收尾**：`oma cleanup`（只杀本 session）
- **查文档**：文件名即标题，`rg --files docs | rg <关键词>`

已落地：`check`、`init --yolo`、`doctor`、`agents`、`hook`。其余仍是设计口径，禁止假装已经可跑。

## 四、资源索引

### 目录结构

| 类别 | 目录 | 说明 |
| --- | --- | --- |
| 文档 | `docs\`（PLAN/TODO 在根；proven/diary/research/guide/references/mistakes 在 docs）+ 根目录 GOAL/README/AGENTS/CHANGELOG/ROADMAP | 见文档指南 |
| 代码 | `src\` + `catalog\` | Rust CLI `oma`；rmux pin 在 catalog |
| 运行时产物 | 目标项目下 `.ohmyagents\`；本机工具 `%LOCALAPPDATA%\ohmyagents\rmux\<ver>\` | gitignore 项目态；工具前缀不进仓 |

### 文档指南

> 四目录职能：`research` = 研究原型过程（为什么，对齐六态）；`references` = 开发测试参考（要做什么怎么做，六态溯源）；`guide` = 元规范（怎么写字怎么研究）；`mistakes` = 出错怎么纠。`MISTAKES` 与 `references` 是经验教训的两面。

- **目标/怎么做/做什么**：`GOAL.md`（起点/锚点/进程/历史）、`PLAN.md`（当前目标怎么做）、`TODO.md`（当前目标进度清单）
- **方案详情**：`docs\proven\NNNN-*.md`（已完成 plan 的归档；进行中与否见 todo）；项目日记 `docs\diary\YYYY-MM-DD-*.md`（**一天一篇**，写当天总结与自省，不写工作细节）
- **全量清单**：`docs\references\文档全量清单-方案与研究目录的完整索引.md`
- **阶段/版本**：`ROADMAP.md`、`CHANGELOG.md`（只记大里程碑）
- **研究/踩坑**：`docs\research\`，规范见 `docs\guide\研究标准细则-结构与六态标记.md`（六态是强规则，见 AGENTS 写研究与测试文档规则）
- **开发测试参考**：`docs\references\`（写码与测试时查的做法：命令手册、测试标准、选型研究、rmux 开发参考、agent 信任参考）
- **元规范**：`docs\guide\`（文档标准、研究标准）
- **错误速查**：`docs\mistakes\MISTAKES.md`

新文档按类别落位，并登记进全量清单。
