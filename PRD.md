# PRD：需求清单管理

> 角色：**需求清单**，四原语之首：需求驱动目标。GOAL 是理解 PRD 后定下的目标和达成标准，GOAL 的每个目标应能回指本清单条目；条目经「追问链」人机交互逐条澄清后登记，禁止静默假设。
> 与其它原语分工：`PRD.md` = 需求清单（要什么）；`GOAL.md` = 目标轨迹（要达成什么）；`PLAN.md` = 当前目标方案（怎么做）；`TODO.md` = 进度清单（做到哪）。

## 生命周期

```text
新需求 到 待澄清 到 已澄清 到 已采纳 到 已交付
拒绝路径：任一状态 到 已拒绝（记原因防复问）
```

## 需求清单

> 一条需求一行：编号接编 `D` 加两位，全局唯一不复用，断号留注记；「派生去向」回指 GOAL 锚点 / PLAN 切片 / P / S / G 编号。
> 历史需求不回填：本清单自 D01 起只记今后新需求；引入前的需求轨迹见 `GOAL.md` 起点 / 锚点 / 历史节与 `docs\proven\` 归档，队列中的历史排队项启动时再入本清单走澄清。

| 编号 | 需求 | 状态 | 澄清轮次 | 派生去向 |
| --- | --- | --- | --- | --- |
| D01 | 学习 reader_rs 引入 PRD 需求清单为第四原语，需求驱动目标 | 已交付 | 第 0 轮（用户 2026-09-03 拍板） | 本文件；AGENTS / G001 / G003 / INDEX / GOAL / PLAN / TODO / README 同步（b948d67） |
| D02 | AGENTS 意图路由细节全下沉 R002，R002 成为命令面唯一权威 | 已交付 | 第 0 轮（用户拍板） | R002 扩容（61a3035）；AGENTS 重写（4147eff） |
| D03 | 根级重写五文件（AGENTS / INDEX / TODO / CHANGELOG / README）禁字合规并退出 `md-char-allow.txt` 豁免 | 已交付 | 第 0 轮 | 五文件重写；豁免清单删五行（2c0e67e） |
| D04 | INDEX 收敛九节并修复登记缺陷；docs\web 登记为资源包输入区 | 已交付 | 第 0 轮 | INDEX 重写（03a6271）；M103 记 M043 |
| D05 | TODO 残表清退；CHANGELOG 与 ROADMAP 补 2026-09-01 至 09-02 里程碑 | 已交付 | 第 0 轮 | TODO 瘦身（2e14434）；CHANGELOG / ROADMAP 补史（fd45180） |
| D06 | 吸收合并 agent 二进制的下载、安装、部署入 oma（配置域除外）：安装行为保持幂等检测不动，存量原地纳管，五端全量验收 | 已交付（方向反转） | 第 1 轮（2026-09-05 用户三裁：不动 oma 的保持幂等检测安装、存量原地纳管、五端全量） | 当日五端验收闭环；同日追问链再裁安装域回归 ome（见 D07），五端成果转过渡态 |
| D07 | oma 收窄为配置 agent、hook、编排（用户裁定 2026-09-05「oma 以后只管配置 agent 和 hook 和编排，不管 agent 的升级和安装」）：agents install/update 加 deprecated 指向 ome，`catalog\agents.toml` 数据权威注记转 ome；登录态 / hook 形态 / 状态栏 / 会话健康四类检查归 agents 域 | 已交付 | 第 3 轮（随 ome 仓 D07 三轮六裁，跨仓同轮）；2026-09-07 用户裁定不等 ome 切片 3 先清自身面 | 迁册批落地 2026-09-07（deprecated 提示加两条集成测试、agents.toml 冻结历史锚、doctor 四类 R002 与 AGENTS 归 agents 域、根 SKILL 再生）；ome 侧部署滞后以 issue 交底 ohmyenv-rs，其仓 D07 切片 3 与 4 自跟；归档 P0029 |
| D08 | ome只管软件部署；oma 要对 hook、状态栏和 agent 配置的检查 | 已交付 | 第 1 轮（2026-09-07 用户裁定，纠正「ome doctor 调用 oma」的误读）：检查面留 oma doctor，不收进 ome；触发项 Grok 状态栏显示错误。2026-09-07 用户实证 M048 修复后栏正常，立项补检查面缺口 | doctor 补 Grok 状态栏 command 三态（M048）与 hook args（M047）；归档 P0030 |
| D09 | oma不管种子 只管诊断、配置、hook、状态栏和编排 | 已交付 | 第 1 轮（2026-09-07 用户裁定，纠正把 ohmycloud 镜像种子算进本仓 TODO）：种子归 ohmycloud，本机安装归 ome | AGENTS 边界、R001 四仓职责与分发通道；不立项新功能 |
| D10 | G005 存量字符清理 | 已交付 | 第 0 轮（用户 2026-09-02 量化并定调清零后 mdcharlint 零容忍；2026-09-07 点名启动） | SKIP_DIRS 外清零并删封闭清单；归档 P0031 |
| D11 | 状态栏工具链段扩展 zig/golang/cpp | 已交付 | 第 1 轮（2026-09-02「以后」；2026-09-07 点名启动）：projKind 加 build.zig / go.mod / CMakeLists（或 meson），图标先 cmap 实证 | 共享 ps1 探测加三枚码位；本机临时目录实跑；归档 P0032 |
| D12 | 根下 `.ohmyagents/t006/` 孤儿目录收敛 | 已交付 | 第 1 轮（diary 09-03 待接；2026-09-07 点名启动）：动前核对与 `tasks/t006/` 产物归属 | 孤儿是 t008 第二轮草稿；已删；协议路径显式 tasks/；归档 P0033 |
| D13 | mac `--version` 一致性 | 已交付 | 第 1 轮（D06 余量；2026-09-07 点名）：源码已有 clap `version`，mac 部署位为旧版；门槛为推 main 出 CI 资产后 `oma self update`。2026-09-07 用户指示推，`main` 已到 origin `7211df0` | 已交付（2026-09-09）：lan-mac 部署位原为编排纪元化石（无 self/无 --version），经镜像 stable 段直装 v0.3.0（oma/stable 边车 sha256 校验过，digest 52631254 与 ohmycloud 对账一致；mirror 第二端点消费实证）；实收 `oma --version` = oma 0.3.0，agents 四家检测与 statusline --example 冒烟绿；旧件备份 ~/.oma/oma-fossil-backup，此后该端可走 oma self update |
| D14 | .ohmyagents 改名为 .oma 目录 | 已交付 | 第 2 轮（2026-09-07）：用户纠错「失误 我们的目录应该是oma」。不是 `.omc`（ohmycloud CLI `omc` / `~/.omc` 金库，其仓 D24）。两根同改：项目 `<cwd>/.oma` 与家目录 `~/.oma`。旧 `.ohmyagents` 在、新目录不在则改名迁过去。技能名 / hook 文件名 `ohmyagents*` 不动。M004 数据目录行被本裁定覆盖 | 归档 P0034 |
| D15 | omg去掉编排，专注于Agent的全平台token、hook和状态栏部署配置（用户原话 2026-09-08） | 已交付 | 第 2 轮（2026-09-08 用户两裁）：1)「还是oma」不改名；2)「oma 退化为纯部署配置工具」编排面全删（spawn/respawn/status/send/key/run/task/settle/cleanup/REPL/web/serve/mcp/trace 与 rmux 依赖），3)「和HOOK和状态栏工具」确认保留面为 token（secrets 域）、hook、状态栏部署配置加诊断（doctor/agents/login/providers/init/self/completions）；四仓边界不动（token 即现有 secrets 域主面，ohmypwsh 密钥安全不冲突） | 归档 P0035；AGENTS / R001 / R002 / INDEX / README / CHANGELOG / ROADMAP / SKILL 同步；97 单元加 17 集成与四门禁全绿 |
| D16 | oma self update 镜像通道（ohmycloud 对账五点引出，用户 2026-09-08「要做」）：OMA_MIRROR 环境变量指向镜像基址（env.ohmygh.com），dev 滚动段按 `<基址>/oma/dev/<资产>` 加 `.sha256` 边车判新下载，校验后走原安装路径 | 已交付 | 第 1 轮（2026-09-08 用户「要做」裁定采纳提案四点）：1) 开关形态 `OMA_MIRROR=<基址>` 对齐 ome OME_MIRROR；2) 只覆盖 dev 通道（镜像无 manifest，stable 段留 GitHub 直连）；3) 镜像判新靠 sha256 边车与安装记录比对（免 manifest），边车裸哈希归一为 `sha256:<hex>` 与 GitHub digest 同形互认；4) 镜像网络失败回落 GitHub 并打 warning | 归档 P0036；R002 / AGENTS / R011 / S028 追记同步；102 单元加 17 集成与四门禁全绿；镜像三分支端到端 2026-09-08 Windows 实证（ohmycloud 播种后） |
| D17 | hook 和状态栏全平台无头 agent 运行测试验收（用户 2026-09-08 提出）：四家 agent 无头模式真跑，验收 oma init 部署的 hook 事件流与状态栏在各平台实际生效 | 已交付 | 第 1 轮发起（2026-09-08），四问待裁：1) 形态：oma 子命令（如 `oma agents verify`）还是 CI matrix 工作流还是两者；2) 四家无头入口与 hook 事件覆盖需先研究（无头模式 PreToolUse 是否触发、状态栏是否渲染）；3) 验收判据：hook 押 state 落盘、状态栏押直调写入命令脚本断言输出，还是必须经 agent TUI；4) 平台范围与 token 供给（五端还是 CI 三平台，验收用 key 走 secrets 还是环境变量）。第 2 轮输入（2026-09-08 用户口述）：claude 与 grok 的状态栏似乎可以渲染，codex 与 kimi 不清楚 [记忆： 用户观感，待 S 研究实证]；D40 三端验收已按现有命令面覆盖 hook 落盘与脚本直调面。第 2 轮研究结案（S033，2026-09-08）：无头入口四家齐（claude -p / codex exec / grok -p / kimi -p）；hook 无头触发 claude、grok、kimi 可验，codex 须 bypass 信任闸加钉版本；状态栏四家均无头不可验（TUI footer 构件），判据只能脚本直跑加 TUI 冒烟两层分离，四问之 2、3 已有实证底座。第 3 轮裁定（2026-09-08 用户「继续吧」采纳建议口径）：1) 形态 = `oma agents verify` 子命令；2) 判据 = hook 押 state 落盘（SessionStart 先于模型调用，不依赖 token 有效性）、状态栏押脚本直跑 mock JSON 断言（S033 两层分离）；3) 平台 = 先本机 Windows 加 WSL 两端；4) token = 最小消耗（每家一条极短 prompt，SessionStart 落盘即判，模型应答不作判据） | 归档 P0038；本机四家实跑全绿；125 单元加 21 集成与四门禁全绿 |
| D18 | 状态栏用户级定制（用户 2026-09-08「能定制状态栏么」引出）：`oma-statusline.ps1` 唯一权威是 oma 二进制内嵌 `STATUSLINE_PS1`，每次 `oma agents statusline` 整文件重释放，手改落盘必被重盖；定制能力只能在 oma 生成时做 | 已交付 | 第 1 轮发起（2026-09-08），三问待裁：1) 定制粒度：段落开关（`~/.oma/statusline.toml` 控段落显隐与顺序）/ 模板变量（格式串加图标映射）/ 整脚本替换（`--script <路径>` 部署用户自备脚本并跳过内嵌覆盖）；2) 重跑语义：用户配置与内嵌默认的合并优先级，缺省键回落内嵌还是报错；3) codex 面：codex 只有内置项 ID 数组（M045）无外部命令面，定制是否豁免 codex 只管 claude/kimi/grok 三家脚本路。第 2 轮裁定（2026-09-09 用户三裁并裁立项开工）：1) 定制粒度 = 三层全要（段落开关加整脚本替换加模板变量）；2) 合并语义 = 缺省键回落内嵌默认（键级宽容合并；段落清单键写下即全量序）；3) codex 面 = 一并支持（定制面为内置项 ID 子集选择） | 已交付：归档 P0039；拆段拼装加生成时烘焙（三层键加 --script/--builtin/--example）；R002 定制行、COMMAND_MAP / SKILL 重生、R011、AGENTS、INDEX、CHANGELOG、README 同步；145 单元加 24 集成与四门禁全绿 |
| D19 | trace 六视图全量恢复（用户 2026-09-08「oma应该有针对项目的agent对话历史trace功能为什么不在了」引出）：D15 去编排时 trace 被归类观察面连坐删除（`eeead00`），但其直读四家原生会话库、与 rmux 零耦合 | 已交付 | 第 1 轮（2026-09-08 用户「全量恢复」裁定）：1) 范围六视图全量（sessions/timeline/blocks/agent/file/search）；2) 定位口径：trace 以只读检索面回归，定位文补「对话历史检索」一域；3) 依赖面：代码自 `eeead00^` 原样带回，`glob` crate 重接，适配现 main.rs 结构 | 归档 P0037；116 单元加 18 集成与四门禁全绿；本仓真数据六视图冒烟实证；文档六处同步 |
| D41 | 双仓自维护 S3 种子（ohmycloud 交付单，用户裁你仓承 A 路线）：release 工作流加 mirror job，rclone 直传 R2（env 桶），dev 滚动产物传 oma/dev、正式 tag 产物传 oma/stable，「我滚你播」接力退役 | 已交付 | 第 1 轮（2026-09-08 ohmycloud D41 交付单加用户裁定采纳）：1) 路线 A 自产产物镜像，rclone provider=Cloudflare 加 endpoint 走 Secrets；2) 必带 NO_CHECK_BUCKET=true（受限 token 无建桶权）；3) 可选 Cache-Control max-age=60（消费侧已有 ?v=/?t= 击穿双保险）；边车 sha256sum 原生格式（hex 空两格加资产名） | 实现面闭环：mirror job 落地（`44def44` 起三笔），oma/dev 段实证精确六件自动到货；stable 段随下次 v* tag 自动；待 ohmycloud 双段验收关账转已交付。2026-09-09 ohmycloud dev 段验收 PASS（三方对账：域面实算 sha = 边车 = GitHub digest 逐字一致；回执 ohmycloud#6 issuecomment-5598606699，经 herdr 跨仓通知当日闭环）；余 stable 段等下一版 v* tag 首次到货补验后整行关账。2026-09-09 v0.3.0 封版触发 stable 段首供，ohmycloud 补验 PASS（三方对账同口径，回执 ohmycloud#6 issuecomment-5599216966），D41 整行关账 |
| D21 | oma 诊断功能扩展（ohmycloud 跨仓需求 ohmyagents-rs#8，D45 配套，用户裁定 2026-09-09）：`oma diagnose cache`（网关别名逐模型双连探测缓存命中矩阵，claude 线 /v1/messages 读 cache_creation/read_input_tokens、codex 线 /v1/responses 读 usage.input_tokens_details.cached_tokens、ds 特判自动前缀不可见不算无缓存）与 `oma diagnose agents`（claude/codex 配置指向检测、别名在册核对 /v1/models、key 活性一发探测、thinking 上限对照） | 已交付 | 第 1 轮（2026-09-09 用户裁「还是 2 吧」）：落法 = 独立 `oma diagnose` 族，与 doctor 的零网络零 token 只读契约分家；凭据通道 = 读 agent 侧原生配置（D45 模板下发的 base_url 与 key），不新建凭据存储（D20 口径延续）；ds 缓存结论修正记档 = 15 显式加 1 隐式，全 16 模型实际都有缓存能力；codex 默认别名 = zy-gpt56sol-codex（astra 因上游无缓存透传让位）。矩阵与实测方法论见 ohmycloud D45 与 issue #8 附件 | 归档 P0041：`oma diagnose cache\|agents` 落地（网关发现 env 覆盖大于 claude env 大于 codex provider、双连至多三连取优、ds 特判、thinking 上限表 fable5 与 opus48 65536）；R002 2.5 活性诊断节、COMMAND_MAP 与 SKILL 再生、AGENTS 路由、INDEX、README、CHANGELOG 同步；131 单元加 21 集成与四门禁全绿；真网关实收（codex 线 hit 1792、ds 线 auto-prefix、本机未 D45 渲染态如实上报）；#8 回执 issuecomment-5601988015 |
| D22 | oma 自己的 SKILL 整理（用户 2026-09-09「要整理出来oma自己的SKILL，要按SKILL标准来」加「oma可以命令自适应输出生成SKILL」）：按 Agent Skills 标准落一份 oma 用户级技能，且内容从活命令树自适应生成，不再手维护命令表 | 已交付 | 第 1 轮（2026-09-09 用户两句即裁定）：落法 = `oma skill` 子命令，从 clap 命令树渲染 SKILL.md（stdout；`--write` 落用户级 `~/.claude/skills/ohmyagents/`），frontmatter 按 SKILL 标准（name 加 description 含何时用）；项目级 init 生成物（COMMAND_MAP 加 marker 语义）不动，双层记档 | 归档 P0042：src\skillgen.rs 渲染器加 `oma skill [--write]`；位置参数 value_name 汉化；about 回落 Cargo 描述单一源；用户级技能已用 `oma skill --write` 刷新（旧版挂着 D20 已删面） |
| D23 | secretguard 实值比对误报修复（ohmyagents-rs#9，2026-09-10）：模型别名被子串匹配误拦（值是更长连字符标识符的真子串即触发），要求收窄误报但保留真实拦截 | 已交付 | 第 1 轮（2026-09-10 issue 三点期望）：1) 完整 token 匹配替代裸子串（边界字符类 [A-Za-z0-9_.-] 之外才算命中，杀超集误报族；根因层 providers.toml 源已随 D20 删除，v0.3.0 在位端仍踩、随新版解）；2) 真实拦截保留（真钥泄露完整 token 形态仍 block，子串形态理论上降权为不拦，换零误报，用户裁「real privacy blocking must be retained」以完整值语义满足）；3) 告警 masked 带脱敏前缀（名[头4字符…长度]）供定位触发词 | 归档 P0043：contains_complete_token 边界匹配加 masked 脱敏前缀；135 单元全绿；#9 回执随 v0.4.0 发 |
| D24 | oma 与 omc / ome 集成的功能优先级裁定（用户裁 2026-09-10，ohmycloud 共识补充提案）：首要诊断与检测，其次恢复与治愈，最后才是安装部署配置；oma 侧按此序排 roadmap | 已交付 | 第 1 轮（用户裁定）：doctor / detect 面先行（与 omc agent doctor 分工照 2026-09-10 前条共识）；heal / recover 面次之（未开始，方向为诊断结论到本机修复动作）；install / deploy / config 最后（oma 自身安装面已随 D20 收窄，集成以与 ome / omc 协作面为主） | 裁定类不立项新功能：ROADMAP 阶段 8 承接优先级序；R001 分工裁决节补记；ohmycloud#6 回执确认（commit 111d49b） |
| D25 | 三仓里程碑对齐封版（用户裁 2026-09-10 跨仓协调）：omc 0.3.0 / ome 0.2.0 / oma v0.5.0 同日封版对齐；重叠功能由 ome 侧削减（ome doctor agent 层收窄为依赖健康、agent 装态对账归 omc） | 已采纳 | 第 1 轮（2026-09-10 协调三问，oma 侧意见经 herdr 口径在本会话提交，等三仓定稿）：1) 版本与日期：同意 v0.5.0 同日封版（三仓版本握手需 oma 有可指认号），日期建议 2026-09-11 或以 omc/ome 就绪为准，v0.5.0 定性为里程碑对齐版（无排定切片、增量预期零或仅热修，CHANGELOG 标注对齐语义不造功能）；2) 重叠自检：oma 无 install/deploy/config 面与 D24 冲突（D20 删尽；self update 属自身维护），提交三处文件级协调点待裁：状态栏写入面与 omc 模板渲染同文件（建议 omc 渲染保留 oma 状态栏四段或入模板）、yolo 键双源（实测两台含托管端用户级 approval/sandbox 与 oma --yolo 项目级并存，建议托管端以 omc 模板为准）、pretrust 信任预写归属（建议托管端归 omc/ome）；3) 回执通道：按用户更正走 herdr 会话同步不发 issue（AGENTS 义务表与 R001 已同步改口径）。第 2 轮裁定（2026-09-10 omc 经 herdr 回执三点裁加日期定稿）：a) 状态栏同文件：omc 渲染器读改写语义天然保留 statusLine 与 [tui] 段（tests 五断言钉死），omc 模板永不碰状态栏面，oma 段归 oma；b) yolo 双源：托管端用户级以 omc 为准（patchCodexConfig 固化并测试锁定），oma --yolo 面向项目级与非托管端，codex 项目级覆盖用户级的优先级 oma 侧已记档 R002；c) pretrust：归属维持 oma init --pretrust，omc / ome 不双写；minor：omc 模板不管 ~/.claude/skills。日期：三仓同日封版定 2026-09-12（ome doctor 削减需落码窗，其 09-12 不及则顺延 09-13，omc 已接受）；oma v0.5.0 里程碑对齐版定性不变，封版流程与验收窗口相应顺延，封版后经 herdr 知会锚更新 | 已交付：用户急令提前，v0.5.0 于 2026-09-10 当日封版发布（M056 根修加共识落档为主增量；六资产齐、oma/stable 滚动、herdr 知会版本与资产 sha）；D24 优先级序与三点裁定均已落档 R001/R002/ROADMAP |
| D20 | oma 不再管理 agent token 的环境变量注入，删除这个功能；oma 专注于 agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置这 5 个功能（用户原话 2026-09-09） | 已交付 | 第 1 轮裁定（2026-09-09 用户四裁）：1) providers 别名簿一并删除（env 注入形态记录面，消费面已随 D15 移除）；2) `oma agents login` 设备码引导删除（doctor 登录态诊断属可用性诊断，保留）；3) deprecated 的 `agents install/update` 兼容层顺带删除（D07 迁册 ome 已两个周期）；4) 本地安装的先不动、只改程序代码、不要安装（shell profile 懒注入块与各端部署位不动，不部署新二进制；vault 文件不动）。默认保留面（发起时声明未被推翻）：`oma agents verify` 为 hook 与状态栏验收面、`oma hook` 密钥拦截闸属 hook 域、`self update` 与 `completions` 为工具自维护 | 归档 P0040：五面删除净删约三千行（secrets.rs / providers.rs / login.rs / catalog.rs / catalog\agents.toml 加 install 机器与命令面）；secretguard 实值比对只看环境变量；AGENTS / R001 / R002 / R011 / INDEX / README / CHANGELOG / COMMAND_MAP 与 SKILL 同步；124 单元加 21 集成与四门禁全绿 |

## 状态机定义

| 状态 | 进入条件 | 出口 | 记录义务 |
| --- | --- | --- | --- |
| 新需求 | 首次登记 | 待澄清 / 已拒绝 | 措辞用用户原话；重建措辞按 G002 标六态 |
| 待澄清 | 追问链发起 | 已澄清 / 已拒绝 | 澄清轮次计数（未发起 / 第 N 轮） |
| 已澄清 | 澄清裁定完成 | 已采纳 / 已拒绝 | 记裁定结论与依据（S 编号或用户点名） |
| 已采纳 | 立项（GOAL 起点锚点 + PLAN 方案 + TODO 清单） | 已交付 / 已拒绝 | 派生去向回填 |
| 已交付 | 验收口径全过 | 终态 | 派生去向补 P 编号或文件 |
| 已拒绝 | 裁定不做 | 终态 | 记原因防复问；实证依据挂 S 编号 |

## 与五步闭环的衔接

> 五步闭环见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`；登记步的权威落位自本清单起由 TODO 前移至 PRD。

| G003 五步 | PRD 动作 | 其它原语 |
| --- | --- | --- |
| 1 登记 | 新需求入 PRD（状态新需求，走追问链，禁止静默假设） | TODO 不建行（澄清采纳后才立） |
| （澄清，登记步内） | 待澄清转已澄清，记轮次与裁定 | |
| 2 立项 | 已澄清转已采纳，派生去向回填 | GOAL 起点锚点回指 D 编号；PLAN 方案；TODO 清单 |
| 3 执行 | 状态不变 | TODO 进度 |
| 4 验收 | 已采纳转已交付 | TODO 清空、proven 归档、GOAL 历史行 |
| 5 归档 | 不动 | diary 钩子、CHANGELOG 版本级 |

## 维护规则

- **登记**：新需求先入本清单（状态新需求），再走追问链澄清；澄清完转已澄清，采纳立项转已采纳，交付验收转已交付。
- **回指**：GOAL 锚点与 PLAN 立项须注明所服务的需求编号（如 D02）。
- **拒绝**：裁定不做的需求转已拒绝并记原因，防复问；实证依据挂 S 编号。
- **追溯**：历史需求补登记时措辞属重建，须按 G002 标六态。
- **纪律**：状态列不许把未验收写成已交付；各条目在承载它的提交内流转，收口提交全表复查。
