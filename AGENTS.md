# AGENTS.md

本文件是协作规则的**最高约束**，四段职责依次为：**项目定位**、**工作规则**、**意图路由**、**资源索引**。

## 一、项目定位

> 本项目的本质与边界。根为定位，下分本质、边界、管理对象、方案索引。

1. **本质**
   - Oh My Agents 是通用智能体多路复用任务编排器：在 rmux 上把多路终端智能体编进一个项目会话，按目录自动配置并编排任务。

2. **边界**
   - 编排钉在启动的项目目录；不替代 ohmypwsh 五端环境总台，不替代各 agent 本体。
   - 四仓分工（2026-09-02 定调，细目见 R001 四仓生态节）：ohmyenv-rs（`ome`）管工具与运行时依赖、本仓（`oma`）管 code agent 部署配置与编排、ohmypwsh 管五端总台与密钥安全、ohmycloud 管云端二进制分发；跨仓协作互相发 issue。
   - 编排操作三通道：CLI、HTTP API、MCP 接口（P0011）；网页做可视化编排。弹不出浏览器不是错误。
   - 运行时后端是 rmux，不引入 herdr 当宿主。
   - hook、skill、状态文件只落启动目录；oma 自管应用数据根是 `~/.ohmyagents`（agent 安装与本地 pin，P0012），默认不改用户家目录 hook 注册。

3. **管理对象**
   - 可注册的终端 agent（当前默认 claude / codex / grok / kimi，可扩展）。
   - 目标项目目录（cwd 或 `--project`）。
   - rmux 任务会话（专用 pipe 或 unix socket）+ 可选 HTTP 镜像。

4. **方案索引**
   - 需求入口：`PRD.md`（新需求先入 PRD 走追问链，禁止静默假设）。
   - 定位：`docs\references\R001-项目定位-通用智能体多路复用任务编排器.md`
   - 定位变更：`docs\proven\P0004-项目重新定位-通用智能体多路复用任务编排器.md`；上一版 `docs\proven\P0002-项目重新定位-通用多Agents自动配置和任务编排器.md`
   - 首期切面：`docs\proven\P0001-四路会话工具-CLI控制面与网页观察面.md`
   - 研究：`docs\research\`（文件名即标题，按关键词搜）

## 二、工作规则

> 四类场景：**对话**（何时做什么）、**操作**（过程纪律）、**编码**（写什么按什么标准）、**文档**（写文档与脚本的标准）。每类下分可以与禁止；节末文档对齐义务表是各动作的对账底线。产品与 rmux 的行为约束不在此层，见意图路由与 `docs\research\`。

### 对话

1. **每轮对话**
   - 可以：先核对四原语 `PRD.md`、`GOAL.md`、`TODO.md`、`PLAN.md`；实质推进当场更新 todo 与 plan。
   - 禁止：不核对四原语就干活；偏离当前目标；推进了不更新 todo/plan。

2. **需求驱动**
   - 可以：新需求先入 `PRD.md`（状态新需求，走追问链澄清，禁止静默假设）；澄清采纳后立项，GOAL 锚点回指 D 编号。
   - 禁止：静默假设需求；GOAL 目标无需求来源。

### 操作

3. **踩坑时**
   - 可以：当场按当前最大号接编 MNNN，落 `docs\mistakes\` 对应分类文件一行（文件名即错误主题，分类表见 `INDEX.md`）；同根因或同型坑合并聚合进已有条目（保留最早编号与首踩日期），不必每踩必新增；主题深挖落 `docs\research\`。
   - 禁止：只留在对话里反复试错。

4. **发现问题时**
   - 可以：任何问题都参照现有文档逻辑、结构与 `INDEX.md` 自修正，走五步闭环（循环自迭代）：**定位**：先搜 INDEX 与相关文档，确认是否已有规则、研究或参考覆盖；**归类**：文档错修文档、规则缺补规则（AGENTS 或对应细则）、知识缺落研究（六态）、出错模式记 mistakes、验证过的做法沉淀 references；**修正**：改在源头，下游引用、索引与四原语同步；**验证**：`rumdl check .` 加 `.tools` 三个 md 扫描（`md-ref-scan.py` 断链、`md-heading-scan.py` 标题括号、`mdcharlint.py` 四类禁用字符封闭清单门禁），涉及结构再对账 INDEX 与磁盘；**提交**：一事一提交，diary 记钩子。
   - 禁止：跳过定位直接改（重复造已有规则）；只修表象不回写体系；问题只留在对话或记忆里；修完不跑验证；把临时补丁当最终方案不归档。

5. **交付变更时**
   - 可以：改代码同步对应文档，改文档同步索引与 `docs\diary\`；遵守命名标准；技术文档按文档标准细则写。
   - 禁止：只改代码不落文档；改了文档不更新索引。

6. **经验沉淀时（强规则，G004）**
   - 可以：成功的 plan 沉淀归 `docs\proven\`（方案与过程）；研究被实证后的做法与多次错误后沉淀成的正确工作流进 `docs\references\` 并挂意图路由或 R002。
   - 错误经验踩坑当场记 `docs\mistakes\`（同根因聚合）；同型坑二犯以上把正确处理升格成 references 工作流并互指。
   - 禁止：`[经验]` 断言只留在研究文档不落 references（检索不到等于没沉淀）；错误只记现象不记根因与处理；`[推断]`/`[假设]` 跳级进 references；一条知识两个权威落位互相重复。

7. **提交时**
   - 可以：`feat:` / `docs:` / `fix:` / `chore:` 前缀加中文描述；一次提交只做一件事。
   - 禁止：多事混一提交；未经指示推远端。

### 编码

8. **执行命令与写文件时**
   - 可以：Windows 命令用 PowerShell 7（`pwsh`），Linux / macOS / WSL 用该平台常规 shell；Markdown / Rust 源码 UTF-8；Windows 上需兼容 5.1 的脚本用 UTF-8 BOM。
   - 禁止：Windows 上默认用 `powershell.exe` 5.1；无 BOM 的中文 ps1 给 5.1 读。

9. **写 Rust 时**
   - 可以：先查 crates.io / docs.rs / GitHub 上是否已有最流行、最稳定、或已经覆盖本需求的库，检索走双通道细则 `docs\references\R005-选型研究细则-cratesio与github双通道.md`（crates.io 稳度四信号加 gh 流行活跃分辨，结论附证据）；选定后用最少代码接上，优先组合而不是自写协议、解压、HTTP、哈希、CLI 解析。
   - 禁止：在现成库已能稳定完成的前提下从零实现；为风格引入冷门或实验 crate；一次拉一堆用不上的依赖。

10. **写测试时**
    - 可以：遵守 `docs\references\R004-测试标准细则-分层断言与门禁流程.md`。分层按官方三层（单元 `#[cfg(test)]`、集成 `tests\*.rs`、doctest），集成优先于单元；意图对应方法（冒烟断退出码、回归用黄金文件、验收对照 oracle）；测试名写成可读规格，负例带 `dies_` 前缀；期望值必须来自独立来源（规范、黄金文件、属性），断言只写稳定字段（标记行、退出码），放过 pid 与时间戳；测试体用 `TestResult` 加 `?` 传播错误；rmux 依赖的测试按闸门 skip；测试设施收 `test-util` feature。
    - 禁止：重言式断言（期望值来自被测同款逻辑或镜像实现分支，AI 生成测试高发）；公开 API 测试塞 `mod tests{}` 不进 `tests\`；默认 mock（oma 拿真 daemon）；计时进断言；测试设施无 feature gate 进生产构建；只测 happy path。

11. **写临时脚本时**
    - 可以：按需自定义的 ps1 / py / Rust 工具，有复用价值即归档 `.tools\`（规则与清单见 `.tools\README.md`；Python 带 PEP 723 头，用 `uv run --script` 运行，py 选库走 `docs\references\R008-项目工具Python库选型细则-pypi与uv.md`；PowerShell 选模块走 `docs\references\R009`）；文档结构大改（改名、编号、移目录）后跑 `uv run --script .tools\md-ref-scan.py` 做断链回归。
    - 禁止：可复用脚本散落仓库根或只留在对话里；把 `pypi.org/search` 或抓网页当可编程选型接口；用 sed 批改中文与反斜杠路径（用 `md-replace.py`，见 M023）；归档不带自述与用法。

### 文档

12. **写文档时**
    - 可以：遵守 `docs\guide\G001-文档标准细则-命名写作规范与rumdl检查.md`（树形、标题干净、文件名即标题、rumdl）加 `docs\guide\G005-中英文技术文档字符与标点硬禁令.md`（四类禁用字符：破折号、箭头、emoji、非法全角；豁免区与替代写法；封闭豁免清单 `md-char-allow.txt` 只减不增，新文件强制合规）。
    - 禁止：标题带括号、口号或破折号（解释放标题下一行引用 `>`）；整段混杂不成树；豁免区外出现 G005 四类禁字。

13. **写研究与测试文档时**
    - 可以：事实性断言必须标六态之一：`[实证]`（本机实测）、`[推断]`（逻辑推出）、`[经验]`（历史惯例）、`[记忆]`（待复核）、`[假设]`（待验证）、`[直觉]`（主观倾向）；标准见 `docs\guide\G002-研究标准细则-结构与六态标记.md`；研究与测试的结论断言不标六态即视为未完成。
    - 禁止：把「没验证」写成「已验证」（实证滥用）；断言不标六态；用猜测冒充结论。

### 文档对齐义务表

> 对齐义务：什么动作后、到什么状态、必须对齐撰写哪些文档。漏对齐即登记债。

| 动作 | 状态时机 | 文档义务 |
| --- | --- | --- |
| 每轮对话 | 每轮开工前 | 核对四原语（PRD / GOAL / PLAN / TODO）；实质推进当场更新 TODO 与 PLAN |
| 新需求提出 | 提出时 | PRD 登记新需求行（状态新需求，走追问链） |
| 追问链澄清 | 澄清完成 | PRD 状态流转加澄清轮次与裁定 |
| 目标立项 | 开工前 | GOAL 起点与锚点（回指 D 编号）、PLAN 方案、TODO 清单 |
| 选型与调研 | 研究完成 | S 文档（六态）加 INDEX 研究节 |
| 交付变更 | 改动完成 | 改代码同步对应文档；命令面变化四处同步（COMMAND_MAP 加行、重跑 `oma init` 重生 SKILL、R002 加行、本文件三节加行）；INDEX 代码表同步 |
| 写测试 | 新层或新面 | R004 落点同步、INDEX 代码表 tests 行 |
| 写脚本 | 归档时 | `.tools\README.md` 清单行、INDEX 代码表 .tools 行 |
| 踩坑 | 当场 | `docs\mistakes\` 对应分类文件接编一行；INDEX 错误速查节行级编号段当轮同步延长 |
| 发现问题 | 当场 | 五步闭环（定位 / 归类 / 修正 / 验证 / 提交），改在源头、下游与四原语同步 |
| 经验沉淀（强规则，G004） | 验收后 | proven 归档（P 编号）；`[实证]` 做法升 references 并挂意图路由或 R002；mistakes 同型二犯升格 |
| 方案达成 | 验收全绿 | proven 归档、GOAL 历史行、INDEX 归档节、TODO 起新清单 |
| 每次提交 | 提交后 | diary 当天记钩子 |
| 版本级成果 / 文档结构变更 | 阶段完成 / 改名移目录后 | CHANGELOG 里程碑与 ROADMAP 阶段状态；INDEX 同步且 `md-ref-scan.py` 断链回归必跑 |

## 三、意图路由

> 需求意图到命令的映射（摘要层）。每条命令的行为细则、机理出处、marker 行、退出码与落地状态的唯一权威见 `docs\references\R002-常用命令与管理流程-从项目init到会话cleanup.md`。

- **核对依赖**：`oma check`（rmux pin 版本加哈希；缺则安装）
- **只诊断**：`oma check --no-install`（缺失或不符非 0，不下载）
- **无阻塞诊断**：`oma doctor`（七面只读体检；warn 与 block 分层，block 才退出 1）
- **检测已装 agent**：`oma agents`（PATH / 环境变量 / oma 自管根 / 默认目录四源）
- **安装缺失 agent**：`oma agents install [名] [--force]`（自适应只补缺，pin 加 sha256 信任锚）
- **提供商别名注入**：`oma agents providers [--example]`（别名簿 providers.toml；`spawn --agents claude@zhipu` 注入 env/argv）
- **升级与 pin 维护**：`oma agents update [名]`（最新版解析加取证写回用户本地 pin）
- **设备码登录引导**：`oma agents login <grok|kimi>`（URL 加 code 干净输出跨机完成，落盘凭据为判据）
- **配置状态栏**：`oma agents statusline [名]`（四家写入面幂等）
- **密钥管理**：`oma agents secrets init|set|env|inject|status`（一钥两密文存储加四 shell 懒注入）
- **hook 写状态加密钥拦截**：`oma hook`（状态落盘；block 级密钥 exit 2 拒调用）
- **部署项目全套**：`oma init [--project PATH]`（yolo 加 hook/skill，四环境自适应，幂等）
- **部署项目级 yolo**：`oma init --yolo`（仅无阻塞键）；`--pretrust` 追加家目录信任库
- **和解拉起**：`oma spawn [--agents a,b] [--stub]`（不在新开、在则附加、死路重开；精确集合与布局自适应）
- **重开一路**：`oma respawn <agent>`（kill-pane 单窗格，不动会话与其它路）
- **看状态**：`oma status`（pid / 终端态 / hook 态 / 扫屏四层，marker 与 TTY 双读者）
- **发任务**：`oma send <agent> "<文本>"`（单行两段式、多行三段式；`--confirm` 短头确认）
- **发单键**：`oma key <agent> <KEY>`（守卫入口；codex 拒 C-c）
- **委派任务**：`oma run "<文本>" [--assign a,b]`（状态门分派多路）
- **带产物等待的任务**：`oma task <agent> "<文本>" [--timeout N]`（建任务目录、阻塞等 DONE、打产物）
- **自愈信任**：`oma settle [--wait N]`（信任/审查框自动确认，密码类永不）
- **收尾**：`oma cleanup`（只杀本 session）
- **开会话（REPL）**：裸 `oma`（重连或拉起，内嵌编排面，行命令 all/agent/status/web/quit）
- **起 web 镜像**：`oma web [agent]`（缺省整会话镜像；官方域中继 E2EE 加 PIN）
- **起 HTTP 编排面**：`oma serve start|stop|status`（即调即退守护；主页即 web 镜像；需 `--features server`）
- **起 MCP server**：`oma mcp`（stdio 九 tools，信封同形；`--print-config` 出注册片段；需 `--features mcp`）
- **oma 自更新**：`oma self update [--stable] [--git]`（缺省 dev 滚动源，Windows rename 舞步）
- **检索轨迹**：`oma trace sessions|timeline|blocks|agent|file|search`（六视图，四家联邦读）
- **生成补全**：`oma completions <shell>`
- **输出格式契约**：全局 `--format kv|json|jsonl` 加 `--json` 简写（信封三传输同形，冻结面见 R011）

设计命令全部落地（2026-08-31）；新想法走 G003 五步再立项，禁止把未验收口径写成已可跑。

## 四、资源索引

> 定位看 `INDEX.md`（项目根目录，唯一索引：编号表、目录结构、代码文件位置）。本节是**配合 INDEX 的搜索与分析方法**。

**速记**：前缀定位 `D`（PRD 需求）/ `P`（proven 归档）/ `S`（research 研究）/ `R`（references 开发测试参考）/ `G`（guide 元规范）/ `M`（mistakes 错误；文件 M1xx、行级 M0xx）；根目录四原语 `PRD` / `GOAL` / `PLAN` / `TODO`。

**外部数据源**：`gh` / `git`（源码、issue 与同类仓检索取证）；`browser-harness`（浏览器搜索与任意网站抓取，ohmyenv-rs 安装）；`reader`（本地文档与电子书读取，ohmyenv-rs 安装）。

**查文档**：先搜 `INDEX.md` 定位编号，再读文件。

**项目工具**：`.tools\`（自定义脚本归档；Python 用 `uv run --script .tools\<名>.py`，清单见 `.tools\README.md`；py 选库细则 `docs\references\R008`）；文档验证三件套：断链回归 `md-ref-scan.py`、标题括号 `md-heading-scan.py`、`rumdl check .`。

**搜索方法（文档）**：

```powershell
rg -n "关键词" INDEX.md                        # 1 先搜总索引，定位编号或文件
rg --files docs | rg 关键词                     # 2 按文件名搜文档
rg -n "关键词" docs\research docs\references    # 3 全文搜研究参考
rg -n "关键词" docs\mistakes\                   # 4 搜错误处理

# mq（markdown 结构查询，D:\ohmyenv\mq\mq.exe，jq 风格；section 模块必须 -A）
mq -F grep '.h2' docs\research\*.md             # 跨文件按节标题定位（文件:行号:标题）
mq -A 'section::section(., "关键结论")' 文档     # 抽整节内容（含正文）
mq -A -F json '.h1' 文档                        # 结构化 JSON（类型/深度/位置）
```

**搜索方法（代码）**：

```powershell
ast-grep outline -l rs --json src\              # 模块符号表（INDEX 代码表配符号清单）
ast-grep run -p 'pub fn name($$$) $$$' -l rs    # 按名定位定义，免疫注释与调用行
ast-grep run -p 'fn $NAME($$$) -> Result<$RET, String> $$$' -l rs --json  # 签名表
```

坑速查：mq 的 `.h.1` 是层级值不是文本（节点用 `.h`/`.h1`）；无 `.s` 选择器（用 section 模块）；ast-grep 的 fn 模式必须带 body 通配 `$$$`、可见性要写进模式、JSON 变量取 `metaVariables.single.<VAR>.text`。详见 M107。

**分析路径**：改产品行为先读 `docs\references\R006/R007`（怎么做）再回 `docs\research\S00x`（为什么）；踩坑查 `docs\mistakes\M1xx`；写码选库走 R005；测试规范 R004；新想法走 G003 五步；定位代码先 INDEX 模块表再 ast-grep 符号；抽文档节用 mq section。
