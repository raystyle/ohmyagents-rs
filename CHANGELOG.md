# Changelog

本文件只记录**大版本里程碑**：定位变更、发布、阶段完成、核心能力整体落地。细碎条目由 `docs\diary\YYYY-MM-DD-*.md` 与 git 历史承载。

## [Unreleased]

### 里程碑

> 2026-08-29 至 2026-08-31：从空仓库到 Windows 全量可用。方案 P0001 至 P0019，过程与经验在各 `docs\proven\` 文档。

- **项目定位**（P0004）：Oh My Agents，通用智能体多路复用任务编排器。首期切面见 P0001。
- **部件 POC 全绿**（P0005）：Windows 范围十四件 example（端点、会话、布局、驱动、对话框、粘贴、定位、流、状态、部署、负例、yolo 诊断、label 桥、备屏）。
- **产品命令闭环**（P0006 至 P0010）：spawn/status/send/cleanup 全链路、send 多行三段式粘贴（中文验收）、oma run 状态门分派、真四路拉通（claude 路全通）、settle 自愈信任（codex hash 复现与白名单点框互兜）。
- **自适应本机安装部署**（P0012）：oma 接管 rmux 与四家 agent 的安装：catalog 两层 pin（出厂锚 + `~/.ohmyagents` 用户本地层写回）、渠道序 github 主 CDN 兜底、sha256 信任锚、装后探针；`oma agents install/update`。Windows 四家装机全绿。
- **联邦轨迹检索**（P0013/P0014）：`oma trace` 六视图查询时直读四家原生会话库；双意图、operation_id 归组、epoch ms 归一；grok 主源升级 updates.jsonl 权威日志（逐事件真实时间，S020）。
- **三传输编排面**（P0011/P0016）：api 传输无关层一份核心三消费：`oma serve`（六操作 RESTish、JSON 信封、可视化网页、SSE 渲染行画面、trace 端点）、`oma mcp`（stdio 九 tools）、REPL（裸 `oma`，编排面内嵌）；三通道共测全绿。
- **输出与易用**（P0015）：六会话命令 `--json` 信封、`oma status` TTY 表格、`oma completions`、R002 输出规范节。
- **Windows 全量收口**（P0017）：send 回显间隔产品化（S005 铁律）、SKILL.md 命令图生成（S016 末件）、grok 无头实跑（S007 回填）、`oma mcp --print-config`。
- **指令集检测**（P0018）：`oma doctor` CPU 能力段（avx/avx2/avx512f）与探针异常退出分类（illegal-instruction 带缓解 hint），S021 问题类的 Windows 落地。
- **文档地基**：AGENTS 四段、三原语、P/S/R/G/M 编号体系、六态标记、rumdl 加两件自研扫描进门禁、`.tools` 脚本归档。

### 里程碑 2026-09-01

- **web 镜像与看板**（P0021 至 P0023）：`oma web` 三面接管 rmux web-share（operator、PIN、TTL）；前端源码本地构建托管、serve 主页即 web-mirror-server；看板资源包化（build.rs 打 tar.gz 嵌二进制、首启释放 oma 数据根）。
- **和解式编排**（P0024）：spawn 三态（会话不在新开、在则活路附加、死路重开）；`oma respawn` 单路强制重开；精确集合与布局自适应。
- **serve 守护化**（P0025）：`serve start` 即调即退（CREATE_NO_WINDOW）；`serve stop` 协议化停机（DELETE /shutdown 优雅排空）。
- **code review 修复**（P0026）：并发安全与健壮性三切片（看板默认只读加 Host 校验、cleanup 僵局解除、task id 原子占位等高 5 中 7 全修）。
- **任务与委派**：`oma task` 带产物等待（任务目录协议 prompt.md/output.md/DONE）；send/run/task 任务开始确认与阻塞告警；`oma key` 单键守卫。
- **流程件**：G004 经验沉淀细则；README 三段重写；oma 编排的 agent 轮换接力 review 工作流（`.tools/review-round.py`）。

### 里程碑 2026-09-02

- **四环境部署自适应**（P0027）：PATH 探针 bare 形态与 codex 字段所有权（Windows 与 WSL 双侧并存不互踢）；状态栏重铸（starship 风格、`agent:state` 机读标记、`oma status` 扫屏交叉核对）。
- **agent doctor 部署诊断与登录引导**（P0028）：doctor warn 层四类部署检查（登录态、hook 形态、状态栏、会话健康）；`oma agents login` 跨机设备码引导。
- **密钥与权限面**：spawn claude 路固定 `--dangerously-skip-permissions`（S029）；`oma hook` 密钥拦截闸（S030，八层防误报）；`oma agents secrets` 一钥两密文与四 shell 懒注入（S031）；`oma agents providers` 别名注入（S027）。
- **生态**：仓库更名 ohmyagents-rs；四仓生态定调（ohmyenv-rs / ohmyagents-rs / ohmypwsh / ohmycloud）。
- **三平台验收**：Windows、macOS、WSL Linux 四家 agent 安装与真身四路全链绿（P0012 收口）。
- **流程件（2026-09-03）**：参考 reader_rs 文档体系重构：PRD 四原语引入、AGENTS 工作规则重组加文档对齐义务表、R002 升命令面唯一权威、INDEX 收敛九节修复登记缺陷、TODO 残表清退、根级五文件禁字合规退出豁免清单。

### 里程碑 2026-09-07

- **迁册批**（P0029 / D07）：`oma agents install` / `update` deprecated 指向 `ome install`；`catalog\agents.toml` 冻结历史锚；doctor 四类检查归 agents 域。
- **doctor 检查面**（P0030 / D08）：Grok 状态栏 command 三态（Windows 只认 `.cmd` 单路径）；JSON hook args 数组 warn。热修 M044 至 M048。
- **边界**（D09）：oma 不管种子，只管诊断、配置、hook、状态栏和编排。
- **G005 存量字符**（P0031 / D10）：SKIP_DIRS 外四类禁字清零；封闭清单删除；mdcharlint 零容忍。
- **状态栏工具链**（P0032 / D11）：projKind 扩展 zig / go / cpp（build.zig / go.mod / CMakeLists 或 meson）；图标 cmap 实证 seti-zig E6A9、seti-go E627、seti-cpp E646。
- **任务目录孤儿**（P0033 / D12）：删除误落 `.ohmyagents/t006/`；协议尾注路径显式 `tasks/`。
- **数据目录**（P0034 / D14）：项目与家目录 `.ohmyagents` 改名为 `.oma`；旧目录仅旧在则改名迁过去。不是 `.omc`（ohmycloud）。

### 里程碑 2026-09-08

- **重新定位**（P0035 / D15）：oma 去编排，退化为 Agent 全平台 token、hook 与状态栏部署配置工具。编排命令（check / spawn / respawn / status / send / key / run / task / settle / cleanup / REPL / web / serve / mcp / trace）与 rmux 后端整体移除；保留 init / doctor / agents（检测 / login / providers / statusline / secrets 加 deprecated install / update）/ hook / self update / completions。
- **self update 镜像通道**（P0036 / D16）：`OMA_MIRROR=<基址>` 只覆盖 dev 通道；边车 sha256 判新免 manifest、下载强制校验（不符报错不回落）、网络失败回落 GitHub；尾巴修缓存击穿（边车 `?t=` 时间戳、资产 `?v=` 边车锚，每滚天然新键）。e2e 一次性副本实证三分支全绿。
- **首个正式版 v0.1.0**：整理清理后封版，v* tag 触发正式 release（三平台 zip / tar.gz 加 .sha256 边车，同 dev 形态）；`oma self update --stable` 走 releases/latest 吃到。镜像分发与四端测试验收归 ohmycloud。
- **trace 六视图全量恢复**（P0037 / D19）：D15 连坐删除的对话历史检索以只读检索面回归（与 rmux 零耦合）；代码自 git 历史原样带回，定位文补「对话历史检索」域。
- **无头验收**（P0038 / D17）：`oma agents verify` 四家 agent 无头验收（状态栏 mock 直跑加 hook 无头落盘两层判据，S033 取证底座）；本机四家全绿。
- **v0.2.0 封版**：trace 恢复后第二正式版（v* tag 触发，三平台资产加边车同形态）；`oma self update --stable` 吃 releases/latest 即到此版。

### 里程碑 2026-09-09

- **状态栏用户级定制**（D18）：脚本拆段拼装（HEAD 加 COMMON 加 PROBE 加 14 段块加 TAIL，行为等价迁移）；`~/.oma/statusline.toml` 三层定制（`segments` 段落显隐与序、`[template]` 段内模板加 `[icons]` 图标映射、`[codex] items` 内置项子集），键级缺省回落内嵌默认、坏文件硬错；`oma agents statusline --script <路径>` 整脚本替换（marker 跳过内嵌覆盖）加 `--builtin` 还原；`--example` 模板打印。README 精练为人类面向（三平台预编译安装加使用示例）。
- **去 token 注入收窄**（D20）：oma 收敛为五功能（可用性诊断、hook、状态栏、trace、yolo）；删 `oma agents secrets`（一钥两密文与四 shell 懒注入）、`providers` 别名簿、`login` 设备码引导、`install` / `update` 兼容层与 catalog 安装机器（净删约三千行）；secretguard 实值比对只看环境变量；密钥安全归 ohmypwsh、agent 二进制归 ome。本地部署位与 shell profile 懒注入块不动（只改程序代码）。
- **活性诊断族**（D21，ohmyagents-rs#8 / ohmycloud D45 配套）：`oma diagnose cache`（网关别名双连缓存命中矩阵，ds 线自动前缀不可见特判，三连取优抗网关异区）与 `oma diagnose agents`（配置指向、别名在册核对、key 活性、thinking 上限对照）；与 doctor 的零网络体检契约分家；真网关实收 codex 线 hit、ds 线 auto-prefix。
- **D41 关账**：ohmycloud 双段验收全绿（dev 段与 stable 段三方对账 sha 逐字一致，回执 issuecomment-5598606699 / 5599216966），「我滚你播」接力退役。
- **v0.3.0 封版**：D18 后第三正式版（v* tag 触发，三平台资产加边车同形态）；mirror job 随 tag 首次填充 oma/stable 段；`oma self update --stable` 吃 releases/latest 即到此版。

### 排后

- Linux/mac 接管（P0012 跨平台面）：资产与代码路径就绪；指令集 SIGILL 预备检测研究已备（S021）。
