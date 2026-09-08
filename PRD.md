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
| D13 | mac `--version` 一致性 | 已澄清 | 第 1 轮（D06 余量；2026-09-07 点名）：源码已有 clap `version`，mac 部署位为旧版；门槛为推 main 出 CI 资产后 `oma self update`。2026-09-07 用户指示推，`main` 已到 origin `7211df0` | 排队；待 CI 出资产后 mac `oma self update` |
| D14 | .ohmyagents 改名为 .oma 目录 | 已交付 | 第 2 轮（2026-09-07）：用户纠错「失误 我们的目录应该是oma」。不是 `.omc`（ohmycloud CLI `omc` / `~/.omc` 金库，其仓 D24）。两根同改：项目 `<cwd>/.oma` 与家目录 `~/.oma`。旧 `.ohmyagents` 在、新目录不在则改名迁过去。技能名 / hook 文件名 `ohmyagents*` 不动。M004 数据目录行被本裁定覆盖 | 归档 P0034 |
| D15 | omg去掉编排，专注于Agent的全平台token、hook和状态栏部署配置（用户原话 2026-09-08） | 已交付 | 第 2 轮（2026-09-08 用户两裁）：1)「还是oma」不改名；2)「oma 退化为纯部署配置工具」编排面全删（spawn/respawn/status/send/key/run/task/settle/cleanup/REPL/web/serve/mcp/trace 与 rmux 依赖），3)「和HOOK和状态栏工具」确认保留面为 token（secrets 域）、hook、状态栏部署配置加诊断（doctor/agents/login/providers/init/self/completions）；四仓边界不动（token 即现有 secrets 域主面，ohmypwsh 密钥安全不冲突） | 归档 P0035；AGENTS / R001 / R002 / INDEX / README / CHANGELOG / ROADMAP / SKILL 同步；97 单元加 17 集成与四门禁全绿 |
| D16 | oma self update 镜像通道（ohmycloud 对账五点引出，用户 2026-09-08「要做」）：OMA_MIRROR 环境变量指向镜像基址（env.ohmygh.com），dev 滚动段按 `<基址>/oma/dev/<资产>` 加 `.sha256` 边车判新下载，校验后走原安装路径 | 已交付 | 第 1 轮（2026-09-08 用户「要做」裁定采纳提案四点）：1) 开关形态 `OMA_MIRROR=<基址>` 对齐 ome OME_MIRROR；2) 只覆盖 dev 通道（镜像无 manifest，stable 段留 GitHub 直连）；3) 镜像判新靠 sha256 边车与安装记录比对（免 manifest），边车裸哈希归一为 `sha256:<hex>` 与 GitHub digest 同形互认；4) 镜像网络失败回落 GitHub 并打 warning | 归档 P0036；R002 / AGENTS / R011 / S028 追记同步；102 单元加 17 集成与四门禁全绿；镜像三分支端到端 2026-09-08 Windows 实证（ohmycloud 播种后） |
| D17 | hook 和状态栏全平台无头 agent 运行测试验收（用户 2026-09-08 提出）：四家 agent 无头模式真跑，验收 oma init 部署的 hook 事件流与状态栏在各平台实际生效 | 待澄清 | 第 1 轮发起（2026-09-08），四问待裁：1) 形态：oma 子命令（如 `oma agents verify`）还是 CI matrix 工作流还是两者；2) 四家无头入口与 hook 事件覆盖需先研究（无头模式 PreToolUse 是否触发、状态栏是否渲染）；3) 验收判据：hook 押 state 落盘、状态栏押直调写入命令脚本断言输出，还是必须经 agent TUI；4) 平台范围与 token 供给（五端还是 CI 三平台，验收用 key 走 secrets 还是环境变量）。第 2 轮输入（2026-09-08 用户口述）：claude 与 grok 的状态栏似乎可以渲染，codex 与 kimi 不清楚 [记忆： 用户观感，待 S 研究实证]；D40 三端验收已按现有命令面覆盖 hook 落盘与脚本直调面。第 2 轮研究结案（S033，2026-09-08）：无头入口四家齐（claude -p / codex exec / grok -p / kimi -p）；hook 无头触发 claude、grok、kimi 可验，codex 须 bypass 信任闸加钉版本；状态栏四家均无头不可验（TUI footer 构件），判据只能脚本直跑加 TUI 冒烟两层分离，四问之 2、3 已有实证底座 | 待定 |
| D18 | 状态栏用户级定制（用户 2026-09-08「能定制状态栏么」引出）：`oma-statusline.ps1` 唯一权威是 oma 二进制内嵌 `STATUSLINE_PS1`，每次 `oma agents statusline` 整文件重释放，手改落盘必被重盖；定制能力只能在 oma 生成时做 | 待澄清 | 第 1 轮发起（2026-09-08），三问待裁：1) 定制粒度：段落开关（`~/.oma/statusline.toml` 控段落显隐与顺序）/ 模板变量（格式串加图标映射）/ 整脚本替换（`--script <路径>` 部署用户自备脚本并跳过内嵌覆盖）；2) 重跑语义：用户配置与内嵌默认的合并优先级，缺省键回落内嵌还是报错；3) codex 面：codex 只有内置项 ID 数组（M045）无外部命令面，定制是否豁免 codex 只管 claude/kimi/grok 三家脚本路 | 待定 |
| D19 | trace 六视图全量恢复（用户 2026-09-08「oma应该有针对项目的agent对话历史trace功能为什么不在了」引出）：D15 去编排时 trace 被归类观察面连坐删除（`eeead00`），但其直读四家原生会话库、与 rmux 零耦合 | 已交付 | 第 1 轮（2026-09-08 用户「全量恢复」裁定）：1) 范围六视图全量（sessions/timeline/blocks/agent/file/search）；2) 定位口径：trace 以只读检索面回归，定位文补「对话历史检索」一域；3) 依赖面：代码自 `eeead00^` 原样带回，`glob` crate 重接，适配现 main.rs 结构 | 归档 P0037；116 单元加 18 集成与四门禁全绿；本仓真数据六视图冒烟实证；文档六处同步 |
| D41 | 双仓自维护 S3 种子（ohmycloud 交付单，用户裁你仓承 A 路线）：release 工作流加 mirror job，rclone 直传 R2（env 桶），dev 滚动产物传 oma/dev、正式 tag 产物传 oma/stable，「我滚你播」接力退役 | 已采纳 | 第 1 轮（2026-09-08 ohmycloud D41 交付单加用户裁定采纳）：1) 路线 A 自产产物镜像，rclone provider=Cloudflare 加 endpoint 走 Secrets；2) 必带 NO_CHECK_BUCKET=true（受限 token 无建桶权）；3) 可选 Cache-Control max-age=60（消费侧已有 ?v=/?t= 击穿双保险）；边车 sha256sum 原生格式（hex 空两格加资产名） | 实现面闭环：mirror job 落地（`44def44` 起三笔），oma/dev 段实证精确六件自动到货；stable 段随下次 v* tag 自动；待 ohmycloud 双段验收关账转已交付 |

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
