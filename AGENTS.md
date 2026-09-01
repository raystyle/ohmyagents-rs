# AGENTS.md

本文件是协作规则的**最高约束**，四段职责依次为：**项目定位**、**操作规则**、**意图路由**、**资源索引**。

## 一、项目定位

> 本项目的本质与边界。根为定位，下分本质、边界、管理对象、方案索引。

1. **本质**
   - Oh My Agents 是通用智能体多路复用任务编排器：在 rmux 上把多路终端智能体编进一个项目会话，按目录自动配置并编排任务。

2. **边界**
   - 编排钉在启动的项目目录；不替代 ohmypwsh 五端环境总台，不替代各 agent 本体。
   - 编排操作三通道：CLI、HTTP API、MCP 接口（P0011）；网页做可视化编排。弹不出浏览器不是错误。
   - 运行时后端是 rmux，不引入 herdr 当宿主。
   - hook、skill、状态文件只落启动目录；oma 自管应用数据根是 `~/.ohmyagents`（agent 安装与本地 pin，P0012），默认不改用户家目录 hook 注册。

3. **管理对象**
   - 可注册的终端 agent（当前默认 claude / codex / grok / kimi，可扩展）。
   - 目标项目目录（cwd 或 `--project`）。
   - rmux 任务会话（专用 pipe 或 unix socket）+ 可选 HTTP 镜像。

4. **方案索引**
   - 定位：`docs\references\R001-项目定位-通用智能体多路复用任务编排器.md`
   - 定位变更：`docs\proven\P0004-项目重新定位-通用智能体多路复用任务编排器.md`；上一版 `docs\proven\P0002-项目重新定位-通用多Agents自动配置和任务编排器.md`
   - 首期切面：`docs\proven\P0001-四路会话工具-CLI控制面与网页观察面.md`
   - 研究：`docs\research\`（文件名即标题，按关键词搜）

## 二、操作规则

> 两类场景：**工作节奏**（何时做什么）与**写作编码**（写什么按什么标准）。每条下分可以与禁止。产品与 rmux 的行为约束不在此层，见意图路由与 `docs\research\`。

### 工作节奏

1. **每轮对话**
   - 可以：先核对三原语 `GOAL.md`、`TODO.md`、`PLAN.md`；实质推进当场更新 todo 与 plan。
   - 禁止：不核对三原语就干活；偏离当前目标；推进了不更新 todo/plan。

2. **踩坑时**
   - 可以：当场按当前最大号接编 MNNN，落 `docs\mistakes\` 对应分类文件一行（文件名即错误主题，分类表见 `INDEX.md`）；同根因或同型坑合并聚合进已有条目（保留最早编号与首踩日期），不必每踩必新增；主题深挖落 `docs\research\`。
   - 禁止：只留在对话里反复试错。

3. **发现问题时**
   - 可以：任何问题都参照现有文档逻辑、结构与 `INDEX.md` 自修正，走五步闭环（循环自迭代）——**定位**：先搜 INDEX 与相关文档，确认是否已有规则、研究或参考覆盖；**归类**：文档错修文档、规则缺补规则（AGENTS 或对应细则）、知识缺落研究（六态）、出错模式记 mistakes、验证过的做法沉淀 references；**修正**：改在源头，下游引用、索引与三原语同步；**验证**：`rumdl check .` 加 `.tools` 两个 md 扫描（`md-ref-scan.py` 断链、`md-heading-scan.py` 标题括号），涉及结构再对账 INDEX 与磁盘；**提交**：一事一提交，diary 记钩子。
   - 禁止：跳过定位直接改（重复造已有规则）；只修表象不回写体系；问题只留在对话或记忆里；修完不跑验证；把临时补丁当最终方案不归档。

4. **交付变更时**
   - 可以：改代码同步对应文档，改文档同步索引与 `docs\diary\`；遵守命名标准；技术文档按文档标准细则写。
   - 禁止：只改代码不落文档；改了文档不更新索引。

5. **经验沉淀时（强规则，G004）**
   - 可以：成功的 plan 沉淀归 `docs\proven\`（方案与过程）；研究被实证后的做法与多次错误后沉淀成的正确工作流进 `docs\references\` 并挂意图路由或 R002；
   - 错误经验踩坑当场记 `docs\mistakes\`（同根因聚合）；同型坑二犯以上把正确处理升格成 references 工作流并互指。
   - 禁止：`[经验]` 断言只留在研究文档不落 references（检索不到等于没沉淀）；错误只记现象不记根因与处理；`[推断]`/`[假设]` 跳级进 references；一条知识两个权威落位互相重复。

6. **提交时**
   - 可以：`feat:` / `docs:` / `fix:` / `chore:` 前缀加中文描述；一次提交只做一件事。
   - 禁止：多事混一提交；未经指示推远端。

### 写作编码

7. **执行命令与写文件时**
   - 可以：Windows 命令用 PowerShell 7（`pwsh`），Linux / macOS / WSL 用该平台常规 shell；Markdown / Rust 源码 UTF-8；Windows 上需兼容 5.1 的脚本用 UTF-8 BOM。
   - 禁止：Windows 上默认用 `powershell.exe` 5.1；无 BOM 的中文 ps1 给 5.1 读。

8. **写 Rust 时**
   - 可以：先查 crates.io / docs.rs / GitHub 上是否已有最流行、最稳定、或已经覆盖本需求的库，检索走双通道细则 `docs\references\R005-选型研究细则-cratesio与github双通道.md`（crates.io 稳度四信号 + gh 流行活跃分辨，结论附证据）；选定后用最少代码接上，优先组合而不是自写协议、解压、HTTP、哈希、CLI 解析。
   - 禁止：在现成库已能稳定完成的前提下从零实现；为风格引入冷门或实验 crate；一次拉一堆用不上的依赖。

9. **写文档时**
   - 可以：遵守 `docs\guide\G001-文档标准细则-命名写作规范与rumdl检查.md`（树形、标题干净、无 emoji 与箭头等装饰符号、文件名即标题、rumdl）。
   - 禁止：标题带括号、口号或破折号（解释放标题下一行引用 `>`）；整段混杂不成树。

10. **写研究与测试文档时**
    - 可以：事实性断言必须标六态之一——`[实证]`（本机实测）、`[推断]`（逻辑推出）、`[经验]`（历史惯例）、`[记忆]`（待复核）、`[假设]`（待验证）、`[直觉]`（主观倾向）；标准见 `docs\guide\G002-研究标准细则-结构与六态标记.md`；研究与测试的结论断言不标六态即视为未完成。
    - 禁止：把「没验证」写成「已验证」（实证滥用）；断言不标六态；用猜测冒充结论。

11. **写测试时**
    - 可以：遵守 `docs\references\R004-测试标准细则-分层断言与门禁流程.md`。分层按官方三层（单元 `#[cfg(test)]`、集成 `tests\*.rs`、doctest），集成优先于单元；意图对应方法（冒烟断退出码、回归用黄金文件、验收对照 oracle）；测试名写成可读规格，负例带 `dies_` 前缀；期望值必须来自独立来源（规范、黄金文件、属性），断言只写稳定字段（标记行、退出码），放过 pid 与时间戳；测试体用 `TestResult` 加 `?` 传播错误；rmux 依赖的测试按闸门 skip；测试设施收 `test-util` feature。
    - 禁止：重言式断言（期望值来自被测同款逻辑或镜像实现分支，AI 生成测试高发）；公开 API 测试塞 `mod tests{}` 不进 `tests\`；默认 mock（oma 拿真 daemon）；计时进断言；测试设施无 feature gate 进生产构建；只测 happy path。

12. **写临时脚本时**
    - 可以：按需自定义的 ps1 / py / Rust 工具，有复用价值即归档 `.tools\`（规则与清单见 `.tools\README.md`；Python 带 PEP 723 头，用 `uv run --script` 运行，py 选库走 `docs\references\R008-项目工具Python库选型细则-pypi与uv.md`；PowerShell 选模块走 `docs\references\R009`）；文档结构大改（改名、编号、移目录）后跑 `uv run --script .tools\md-ref-scan.py` 做断链回归。
    - 禁止：可复用脚本散落仓库根或只留在对话里；把 `pypi.org/search` 或抓网页当可编程选型接口；用 sed 批改中文与反斜杠路径（用 `md-replace.py`，见 M023）；归档不带自述与用法。

## 三、意图路由

> 需求意图与操作方法的映射。命令细则见 `docs\references\R002-常用命令与管理流程-从项目init到会话cleanup.md`。
> 显示名 Oh My Agents；仓库 `ohmyagents`；CLI 二进制 `oma`。数据目录仍是 `.ohmyagents`。

- **核对照**：`oma check`（rmux 版本 + 哈希 + 完整布局；缺则按 `catalog/rmux.toml` 安装。`--no-install` 只诊断）
- **无阻塞诊断**：`oma doctor`（进程存活 + hook 语义 + 任务指向 + yolo；不把 wait-pane Quiet 当 idle）
- **检测已装 agent**：`oma agents`（PATH、`OMA_AGENT_PATH`、`OMA_*_BIN`、oma 自管根 `~/.ohmyagents\agents`、各家默认安装目录；Windows / Linux / macOS；缺装行带 hint）
- **安装缺失 agent**：`oma agents install [名] [--force] [--root PATH]`（自适应：已装任何来源即跳过；catalog pin 加渠道序 github 主 CDN 兜底加 sha256 信任锚加 leaf 找二进制加装后探针；Windows 实测四家全绿，Linux / mac 资产与路径就绪待环境切换验收，P0012）
- **升级与 pin 维护**：`oma agents update [名] [--force]`（最新版解析加 sha 取证加写回用户本地 pin 层 `~/.ohmyagents\catalog\agents.toml`；取证不全整体失败保旧 pin）
- **hook 写状态**：`oma hook`（agent lifecycle hook 调用；stdin JSON 或参数；写 `OHMYAGENTS_STATE_FILE`；缺环境则静默。不连 rmux 管道）
- **Windows 最小 pane POC**：`cargo run --example poc-endpoint|poc-session|poc-layout|poc-drive|poc-dialogs|poc-paste|poc-locate|poc-stream|poc-state|poc-init|poc-negatives`（专用 pipe、CreateOnly、2x2、send_text+Enter、hook blocked + sendkeys、load-buffer+paste-buffer -p 中文、pid 反查进程名错位 throw、output_stream Oldest 回放 Now 直播、terminal_state 分类 Quiet 不当 idle、hook/skill 项目级部署幂等不改家目录、C-c Codex 与 daemon-wide kill 负例守卫）。Windows 范围全表绿；Linux/mac 委托后续仓库
- **部署项目级 hook/skill/yolo**：`oma init [--project PATH]` 全套（yolo 键加 hook/skill，S015 矩阵，幂等不改家目录）；`--yolo` 仅键；`--pretrust` 追加家目录信任库
- **和解拉起**：`oma spawn [--agents a,b] [--stub] [--project PATH]`（P0024：会话不在新开，在则活路附加、死路重开；命令面只见 agent 实例，窗格复杂性绑在背后；不阻塞）；阻塞用 `doctor`/`status` 诊断，不在主命令里长时间 `wait_ready` 卡住委派
- **开会话（REPL）**：`oma [--stub] [--agents a,b] [--no-web] [--open]`（P0016 已落地）：会话已在则重连不叠格；默认内嵌编排面（端口 7900 顺延 7909）打印 URL；`--open` 才开浏览器（失败只警告）。行命令 `all|<agent> <文本>|status|web|quit`，quit 只 detach
- **委派**：`oma send <agent> "<文本>"`（单行两段式、多行三段式粘贴均实测可用；`--confirm MARKER` 等短头确认）；`oma run "<文本>" [--assign a,b]` 状态门分派多路（一路 blocked/busy 跳过不堵其它路，写层 3 任务文件）。Drive 遵守三段式铁律（发前扫框、`paste-buffer -p`、Enter 单独发**且与文本间隔**，细则见 `docs\research\S005-drive铁律与三段式粘贴.md`）：禁止文本和 Enter 同发、对 Codex 发 `C-c`、发送侧自包 `\x1b[200~`
- **查轨迹**：`oma trace sessions|timeline|blocks|agent|file|search`（六视图，`--project` 挂叶子）：查询时联邦读四家原生会话库（grok 主源 updates.jsonl 权威日志，chat_history 兜底，S020）
- **自愈信任**：`oma settle [--wait N]`（自检测信任/审查框并自动确认默认应选项，各家自己持久化信任；密码类永不自动）。codex 的 hook 注册形态见 `src\deploy.rs`（绝对路径加 PowerShell 调用操作符 `&`）
- **看状态**：`oma status`（层 0 pid + locate 进程名 + 1b 终端态 + 层 2 hook 态）
- **重开一路**：`oma respawn <agent>`（强制关闭再打开该 agent 实例；kill-pane 单窗格，不动会话与其它路）
- **收尾**：`oma cleanup`（只杀本 session）
- **起 HTTP 编排面**：`oma serve [--port N] [--project PATH]`（P0011 已落地，需 `--features server` 构建）：六操作 RESTish 加 JSON 信封加 SSE 画面；**主页即 web 镜像**（打开就是多路窗格，P0022；前端资源包嵌二进制首启释放 oma 数据根，P0023；配置 dashboard 已删，编排走 CLI/API/MCP）；只绑 127.0.0.1；curl 全绿口径见 `docs\references\R002`
- **起 web 镜像**：`oma web [agent] [--spectator] [--ttl N] [--no-pin]`（P0021/P0022 已落地）：缺省整会话镜像（全窗格可编辑带分屏），给 agent 单 pane；`oma serve` 的 `GET /` 即 web-mirror-server 主页（自动起镜像免 PIN，打开即多路窗格）；HTTP `POST /share`（会话）/`POST /share/{agent}`（单路）/`GET /share`/`DELETE /share/{id}/stop`
- **起 MCP server**：`oma mcp [--project PATH] [--print-config]`（P0011 已落地，需 `--features mcp` 构建）：stdio 九 tools（六操作加 trace 检索），信封与 HTTP 同形；`--print-config` 打印各客户端注册片段（任何构建可用）；三通道共测口径见 `docs\references\R002`
- **查文档**：先搜 `INDEX.md` 定位编号，再读文件；rg / mq / ast-grep 全套搜索方法见四、资源索引
- **项目工具**：`.tools\`（自定义脚本归档；Python 用 `uv run --script .tools\<名>.py`，清单见 `.tools\README.md`；py 选库细则 `docs\references\R008`）；文档验证三件套：断链回归 `md-ref-scan.py`、标题括号 `md-heading-scan.py`、`rumdl check .`
- **JSON 信封**：六会话命令加 `--json`（spawn/status/send/run/settle/cleanup）出 `{ok, data|error, meta}` 信封，与 HTTP/MCP 同形（P0015 已落地）
- **生成补全**：`oma completions <shell>`（clap_complete，bash/zsh/fish/powershell 等）

已落地：`check`、`init`（全套）、`doctor`、`agents`、`agents install`、`agents update`、`hook`、`spawn`、`status`（TTY 表格）、`send`、`cleanup`、`run`、`settle`、`trace` 六视图、`serve`（HTTP 编排面加网页可视化）、`mcp`（stdio 九 tools）、REPL（裸 `oma`，内嵌编排面）、`respawn`（强制重开一路）、`web`（web 镜像，整会话缺省）、`completions`、六会话命令 `--json` 信封。设计命令全部落地；新想法走 G003 五步再立项，禁止把未验收口径写成已可跑。

## 四、资源索引

> 定位看 `INDEX.md`（项目根目录，唯一索引：编号表、目录结构、代码文件位置）。本节是**配合 INDEX 的搜索与分析方法**。

**速记**：前缀定位 `P`（proven 归档）/ `S`（research 研究）/ `R`（references 开发测试参考）/ `G`（guide 元规范）/ `M`（mistakes 错误；文件 M1xx、行级 M0xx）；根目录三原语 `GOAL` / `PLAN` / `TODO`。

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
