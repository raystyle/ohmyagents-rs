# INDEX：项目总索引

> 角色：全仓**唯一索引**——只做定位：编号表、目录结构、代码文件位置。搜索与分析方法（rg、mq、ast-grep 怎么配合本索引）见 `AGENTS.md` 四、资源索引。规则权威源见 `AGENTS.md`；命名与编号规则见 `docs\guide\G001-文档标准细则-命名写作规范与rumdl检查.md`。

## 一、编号体系

**前缀定位**：`P`（proven，已完成 plan 归档，4 位）；`S`（research，研究原型过程，3 位）；`R`（references，开发测试参考，3 位）；`G`（guide，元规范，3 位）；`M`（mistakes，分类文件 M1xx、行级错误 M0xx 全局递增不复用）。根目录三原语：`GOAL`（目标轨迹）/ `PLAN`（当前目标方案，基于研究与参考）/ `TODO`（进度清单）。

**目录职能**：`proven` 已完成 plan 归档；`diary` 一天一篇总结与自省；`research` 研究原型过程（为什么，六态对齐，规范见 G002）；`references` 开发测试参考（要做什么怎么做，六态溯源）；`guide` 元规范（含 `template.md`）；`mistakes` 出错怎么纠（与 references 是经验教训的两面）。

新文档按类别落位，编号接当前最大号，登记进本索引对应节。

## 二、目录结构与代码文件位置

| 类别 | 目录 | 说明 |
| --- | --- | --- |
| 文档 | `docs\`（proven/diary/research/guide/references/mistakes）+ 根目录 GOAL/PLAN/TODO/INDEX/AGENTS/README/CHANGELOG/ROADMAP | 见上节职能 |
| 代码 | `src\` + `catalog\` | Rust CLI `oma`；rmux pin 在 catalog |
| 运行时产物 | 目标项目下 `.ohmyagents\`；本机工具 `%LOCALAPPDATA%\ohmyagents\rmux\<ver>\` | gitignore 项目态；工具前缀不进仓 |

**代码文件位置**：

| 文件 | 职责 |
| --- | --- |
| `.tools\` | 项目自定义脚本工具归档（ps1 / py / Rust；`README.md` 含清单与规则；`uv run --script` 载体） |
| `.tools\md-ref-scan.py` | markdown 仓内引用断链扫描（文档大改后回归门禁；豁免清单 `md-ref-allow.txt`） |
| `.tools\md-replace.py` | 中文与反斜杠路径安全的字面批量替换（规避 sed 坑 M023） |
| `src\main.rs` | CLI 入口与全部子命令分发（check/init/doctor/agents/hook/spawn/status/send/cleanup/run/settle/trace/serve/mcp/completions）；`--json` 信封出口与 status TTY 表格 |
| `src\lib.rs` | 模块声明 |
| `src\catalog.rs` | `catalog\rmux.toml` 与 `catalog\agents.toml` pin 读取与加载期校验 |
| `src\rmux.rs` | `oma check`：布局探测、归档下载安装、哈希校验 |
| `src\rmuxpoc.rs` | POC 共用层：专用端点、闸门、Job Object WMI 退路、桩 argv |
| `src\hook.rs` | `oma hook`：事件到四态映射与 state 落盘 |
| `src\agents.rs` | `oma agents`：PATH / 环境变量 / 默认目录探测 |
| `src\doctor.rs` | `oma doctor`：只读诊断（yolo / 信任 / 二进制 / state） |
| `src\yolo.rs` | `oma init --yolo`：四家配置落盘与 pretrust |
| `src\deploy.rs` | `oma init` hook/skill 部署层：按 S015 矩阵落项目文件，幂等合并；SKILL.md 由 COMMAND_MAP 命令图生成（标记覆写三态） |
| `src\orch.rs` | 产品编排层：项目 slug 会话、spawn/status/send/cleanup、pane 清单 |
| `src\install.rs` | 自适应安装层：多渠道下载、sha 信任锚、oma 自管根布局、update 取证与 pin 写回 |
| `src\trace.rs` | 意图轨迹检索层：四家会话发现 + 四家联邦 loader（codex FileChange 主源、grok updates 权威日志加 chat_history 兜底、注入过滤、epoch ms 归一）+ 块聚合与过滤分页检索 |
| `src\api.rs` | 传输无关编排操作层（P0011）：六操作加 trace 检索三件返回结构化 JSON，HTTP 与 MCP 共用 |
| `src\mcp.rs` | MCP 适配层（feature `mcp`，P0011）：rmcp 3.1.4 stdio 九 tools，信封同形，stdout 纯协议 |
| `src\server.rs` | HTTP 适配层（feature `server`，P0011/P0019）：axum 六操作 RESTish + JSON 信封 + 会话写串行化 + 网页直出 + 行日志 SSE + 终端镜像 SSE（render_stream 加首帧）+ trace 三端点；`serve_in_background` 供 REPL 内嵌 |
| `src\repl.rs` | REPL 交互层（P0016）：裸 `oma` 进；stdin 线程喂 mpsc、行命令分派、编排面内嵌、状态表格渲染（CLI 共用） |
| `docs\web\share-src\` | rmux-web-share 前端源码（Astro，`npm run build` 出产物；node_modules 与 dist 不进仓） |
| `docs\web\kanban\` | web-mirror-server 前端构建产物（资源包构建输入；build.rs 打 tar.gz 嵌二进制） |
| `build.rs` | kanban 资源包打包（tar.gz 加 sha256 指纹进 OUT_DIR；rerun-if-changed 挂资产目录） |
| `src\webassets.rs` | 资源包嵌入与首启释放（`~/.ohmyagents/web/<指纹>/`，一次一份，P0023） |
| `tests\cli.rs` | CLI 集成冒烟（assert_cmd；check/agents/hook/doctor/send 快败） |
| `src\caps.rs` | CPU 指令集能力与探针退出形态分类（S021/P0018：is_x86_feature_detected 加 0xC000001D 识别） |
| `src\pathutil.rs` | 路径工具 |
| `examples\poc-*.rs` | 十四个 POC（见下；label-bridge 端点融合、dump 备屏诊断） |
| `catalog\rmux.toml` | rmux tag 与各平台 SHA256（信任锚） |
| `catalog\agents.toml` | 四家 agent pin：渠道序（github 主 CDN 兜底）、per-OS+arch 资产 SHA256、官方校验清单线索（信任锚） |

```text
ohmyagents/
  GOAL.md / PLAN.md / TODO.md / INDEX.md   三原语加总索引
  AGENTS.md / CLAUDE.md / README.md / CHANGELOG.md / ROADMAP.md
  Cargo.toml / LICENSE / .rumdl.toml
  catalog\
    rmux.toml
  .tools\            自定义脚本工具（md-ref-scan / md-replace 等）
  src\
    main.rs  lib.rs  catalog.rs  rmux.rs  rmuxpoc.rs
    hook.rs  agents.rs  doctor.rs  yolo.rs  pathutil.rs
    deploy.rs  orch.rs
  tests\
    cli.rs
  examples\
    poc-yolo-doctor.rs  poc-endpoint.rs  poc-session.rs
    poc-layout.rs  poc-drive.rs  poc-dialogs.rs  poc-paste.rs
    poc-locate.rs  poc-stream.rs  poc-state.rs  poc-init.rs
    poc-negatives.rs  poc-label-bridge.rs  poc-dump.rs
  docs\
    proven\      P 编号，已完成 plan 归档
    diary\       一天一篇总结自省
    research\    S 编号，研究原型过程（六态）
    references\  R 编号，开发测试参考
    guide\       G 编号，元规范；template.md
    mistakes\    M1xx 分类文件，行级 M0xx
```

## 三、方案归档

> 位置 `docs\proven\`。

| 编号 | 文件 | 主题 |
| --- | --- | --- |
| P0001 | `P0001-四路会话工具-CLI控制面与网页观察面.md` | 首期切面 |
| P0002 | `P0002-项目重新定位-通用多Agents自动配置和任务编排器.md` | 上一版定位 |
| P0003 | `P0003-rmux检测版本哈希与全平台安装.md` | `oma check` |
| P0004 | `P0004-项目重新定位-通用智能体多路复用任务编排器.md` | 现役定位 |
| P0005 | `P0005-各功能部件POC验证原型.md` | 已完成（Windows 全表绿） |
| P0006 | `P0006-产品命令最小闭环-spawn状态send与cleanup.md` | 已完成（同日验收） |
| P0007 | `P0007-send多行粘贴与label端点融合.md` | 已完成（同日验收，含自愈） |
| P0008 | `P0008-oma-run委派与任务映射.md` | 已完成（同日 stub 验收） |
| P0009 | `P0009-真四路拉通验收.md` | 已完成（claude 路全通；spawn cwd 缺陷修复 M031） |
| P0010 | `P0010-settle自愈信任-自检测与自动确认.md` | 已完成（codex 路全通；双机制互兜） |
| P0011 | `P0011-三传输编排面-http-api与mcp与网页可视化.md` | 已完成（api 层一份核心三消费；serve 网页 SSE、mcp 九 tools、三通道共测全绿） |
| P0012 | `P0012-自适应本机安装部署-rmux与四家agent接管.md` | 已完成（Windows 四家装机全绿；Linux/mac 待环境切换） |
| P0013 | `P0013-agent意图操作块与编辑轨迹检索.md` | 已完成（四家 loader 活体验证；MCP 挂载归 P0011） |
| P0014 | `P0014-grok权威日志升级.md` | 已完成（updates 主源加 chat_history 兜底；逐事件真实时间） |
| P0015 | `P0015-S016吸收件收口.md` | 已完成（--json 信封、TTY 表格、completions、R002 输出规范） |
| P0016 | `P0016-REPL与编排面内嵌.md` | 已完成（裸 oma 进 REPL；编排面内嵌端口顺延；stub 验收过） |
| P0017 | `P0017-Windows全量收口.md` | 已完成（send 回显间隔、HTTP trace 三端点、SKILL 命令图、grok 无头、mcp 配置打印） |
| P0018 | `P0018-Windows侧指令集检测落地.md` | 已完成（caps 检测进 doctor；探针退出分类进 agents 与装机） |
| P0019 | `P0019-产品完备收口与四家真路验收.md` | 已完成（SSE 终端镜像、门面文档对齐、四家真路全链验收；修 status 降级、CHILD_SESSION、settle 三态） |
| P0021 | `P0021-官方web镜像集成.md` | 已完成（oma web 三面集成 rmux web-share；自建 xterm 桥下线） |
| P0022 | `P0022-web镜像本地化与主页化.md` | 已完成（前端源码构建本地托管、session 镜像免 PIN、主页即镜像、dashboard 下线；命名 web-mirror-server） |
| P0023 | `P0023-看板资源包化.md` | 已完成（build.rs 打 tar.gz 嵌二进制、首启释放 oma 数据根、指纹一次一份） |
| P0024 | `P0024-agent实例和解式编排.md` | 已完成（spawn 三态和解：新开/附加/死路重开；oma respawn 强制单路重开） |

## 四、项目日记

> 位置 `docs\diary\`；一天一篇总结自省。

- `2026-08-29-对照ohmypwsh建立文档骨架.md`
- `2026-08-31-研究体系与POC全绿.md`

## 五、研究文档

> 位置 `docs\research\`；S 编号。

| 编号 | 文件 | 主题 |
| --- | --- | --- |
| S001 | `S001-四路会话的控制面与观察面.md` | CLI 控四路、网页只镜像；三份 demo 拆解 |
| S002 | `S002-跨平台与无浏览器.md` | 端点平台口径、WMI breakaway、默认不弹窗 |
| S003 | `S003-rmux-sdk最佳开发实践与验证poc.md` | SDK 0.10 一手核查；POC-0..8 |
| S004 | `S004-win-rmux既有rmux研究吸收.md` | 进程模型、hardened-guard 八条、命令核验表 |
| S005 | `S005-drive铁律与三段式粘贴.md` | 五铁律与 paste 三段式 |
| S006 | `S006-信任阻塞门-四家种类与官方口径.md` | 四家门总表；Claude 官方分路 |
| S007 | `S007-yolo与无阻塞启动-配置落盘与无头分路.md` | 四家落盘表；无头分路；doctor 判据 |
| S008 | `S008-项目级hook与skill.md` | 四家项目级发现规则；init 部署树 |
| S009 | `S009-agent状态判断-通道与分层.md` | 四层含 1b 模型；文件总线；事件映射 |
| S010 | `S010-clum等待原语作为hook兜底状态.md` | `terminal_state` 分类器；SDK 超时真相 |
| S011 | `S011-Command-LineRust测试方法论与oma测试分层.md` | 三源测试对照（已固化 R004） |
| S012 | `S012-ponytail懒人阶梯与oma编码经验.md` | 实现取舍七档阶梯 |
| S013 | `S013-选型研究双通道实证-cratesio与github.md` | 两落地法证据链（已固化 R005） |
| S014 | `S014-检测已装agent-PATH与默认目录与环境变量.md` | 四家二进制探测路径表 |
| S015 | `S015-四家hook注册一手形态-官方文档与源码核实.md` | 注册落点/schema/事件全集/部署矩阵（poc-init 依据） |
| S016 | `S016-incurs命令输出与帮助经验吸收.md` | 双层源码研究：输出信封/CTA/帮助/三传输参照（P0011 依据） |
| S017 | `S017-ohmypwsh安装配置机制与四家agent渠道取证.md` | ohmypwsh 安装配置蓝本与四家渠道 checksum 取证（P0012 依据） |
| S018 | `S018-aitrace意图轨迹机制研究与oma检索映射.md` | aitrace 三源关联机制、裁决表与坑清单（P0013 依据） |
| S019 | `S019-四家会话日志格式与联邦检索取证.md` | 四家会话库四要素钉死与三仓源码纠偏（P0013 依据） |
| S020 | `S020-grok权威日志updates与method分类学.md` | updates 信封两流分类学与四要素定位（P0014 依据） |
| S021 | `S021-linux预备检测-指令集SIGILL问题类与检测阶梯.md` | AVX-512/AVX2 SIGILL 问题类、四级检测阶梯与 oma 探针落点（P0012 预备） |
| S022 | `S022-rust程序自带资源包的三路线与释放裁决.md` | include_bytes 对 rust-embed 对嵌入归档加释放；指纹目录口径（P0023 依据） |
| S023 | `S023-rmux在windows的进程树与原语实测.md` | 活体进程树加源码核实；三纠偏（internal-daemon 形态、conhost 兄弟、pane 无 shell 层）与原语表 |

## 六、开发测试参考

> 位置 `docs\references\`；R 编号。

| 编号 | 文件 | 用途 |
| --- | --- | --- |
| R001 | `R001-项目定位-通用智能体多路复用任务编排器.md` | 现役定位展开 |
| R002 | `R002-常用命令与管理流程-从项目init到会话cleanup.md` | oma 命令手册 |
| R004 | `R004-测试标准细则-分层断言与门禁流程.md` | 测试分层、断言、闸门 |
| R005 | `R005-选型研究细则-cratesio与github双通道.md` | 选库检索双通道 |
| R006 | `R006-rmux开发参考-连接会话布局与驱动.md` | 写 rmux 相关代码时查 |
| R007 | `R007-agent信任与无阻塞参考-四家配置与检测.md` | 写 init/doctor/hook 时查 |
| R008 | `R008-项目工具Python库选型细则-pypi与uv.md` | py 工具选库与 uv 工作流 |
| R009 | `R009-项目工具PowerShell模块选型细则-psgallery与psresourceget.md` | ps 模块选型与 ohmypwsh 统一管理 |

（R003 退役：原全量清单并入本索引，编号不复用。）

## 七、元规范

> 位置 `docs\guide\`；G 编号。

| 编号 | 文件 | 用途 |
| --- | --- | --- |
| G001 | `G001-文档标准细则-命名写作规范与rumdl检查.md` | 命名与编号、写作、rumdl |
| G002 | `G002-研究标准细则-结构与六态标记.md` | 研究结构与六态 |
| G003 | `G003-工作流标准细则-从登记到归档五步.md` | 五步工作流与优先级 |
| — | `template.md` | 方案模板（不编号） |

## 八、错误速查

> 位置 `docs\mistakes\`；分类文件 M1xx，行级 M0xx。

| 编号 | 分类文件 | 覆盖关键词 | 行级编号段 |
| --- | --- | --- | --- |
| M101 | `M101-drive与paste错误.md` | send-keys、Enter、`C-c`、bracketed paste、marker 假阳性 | M001、M008、M027 |
| M102 | `M102-信任与hook配置错误.md` | 信任框、trust、pretrust、init、yolo | M002、M009-M011 |
| M103 | `M103-文档与命名错误.md` | 命名、显示名、CLI 名、六态、diary、标题规范 | M003-M005、M013-M014、M030 |
| M104 | `M104-rmux安装与CLI调用错误.md` | 安装、`-V`、`-S`、`-L`、`cmd()`、`-t` 前缀匹配 | M006-M007、M016、M020、M029 |
| M105 | `M105-agent检测与状态判断错误.md` | PATH、which、idle、Quiet、CPU | M012、M018-M019 |
| M106 | `M106-Windows进程与daemon启动错误.md` | os error 5、Job Object、WMI、exit-empty、pane cwd | M015、M017、M021-M022、M031 |
| M107 | `M107-工具链与脚本错误.md` | sed、grep、PowerShell、中文路径、测试临时目录 | M023-M026、M028、M032-M034 |

迭代规则：踩坑按当前最大号接编 MNNN 进对应分类文件（M0xx 行级、新分类用 M1xx 接编）；一行一事；同根因或同型坑**可合并聚合**进已有条目（保留最早编号与首踩日期，聚合后的正解写全），避免同型条目无限线性追加；反复踩落 `docs\research\`；改「正确处理」不删历史行；新分类文件登记本节。

## 九、阶段与版本

- `ROADMAP.md`：阶段路线
- `CHANGELOG.md`：版本里程碑

## 十、代码与 pin

- 代码文件位置见第二节表；`catalog\rmux.toml` 是 `oma check` 的信任锚
- `examples` 十二个部件 POC 对应方案 P0005 的部件表（yolo-doctor / endpoint / session / layout / drive / dialogs / paste / locate / stream / state / init / negatives），Windows 范围全表绿（2026-08-31）；`poc-label-bridge` 是 P0007 的 label 端点融合实证
