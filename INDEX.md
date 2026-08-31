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
| `src\main.rs` | CLI 入口与子命令分发（check / init / doctor / agents / hook） |
| `src\lib.rs` | 模块声明 |
| `src\catalog.rs` | `catalog\rmux.toml` pin 读取（版本与哈希） |
| `src\rmux.rs` | `oma check`：布局探测、归档下载安装、哈希校验 |
| `src\rmuxpoc.rs` | POC 共用层：专用端点、闸门、Job Object WMI 退路、桩 argv |
| `src\hook.rs` | `oma hook`：事件到四态映射与 state 落盘 |
| `src\agents.rs` | `oma agents`：PATH / 环境变量 / 默认目录探测 |
| `src\doctor.rs` | `oma doctor`：只读诊断（yolo / 信任 / 二进制 / state） |
| `src\yolo.rs` | `oma init --yolo`：四家配置落盘与 pretrust |
| `src\pathutil.rs` | 路径工具 |
| `examples\poc-*.rs` | 十个部件 POC（见下） |
| `catalog\rmux.toml` | rmux tag 与各平台 SHA256（信任锚） |

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
  examples\
    poc-yolo-doctor.rs  poc-endpoint.rs  poc-session.rs
    poc-layout.rs  poc-drive.rs  poc-dialogs.rs  poc-paste.rs
    poc-locate.rs  poc-stream.rs  poc-state.rs
  docs\
    proven\      P 编号，已完成 plan 归档
    diary\       一天一篇总结自省
    research\    S 编号，研究原型过程（六态）
    references\  R 编号，开发测试参考
    guide\       G 编号，元规范；template.md
    mistakes\    M1xx 分类文件，行级 M0xx
```

## 三、方案归档（`docs\proven\`）

| 编号 | 文件 | 主题 |
| --- | --- | --- |
| P0001 | `P0001-四路会话工具-CLI控制面与网页观察面.md` | 首期切面 |
| P0002 | `P0002-项目重新定位-通用多Agents自动配置和任务编排器.md` | 上一版定位 |
| P0003 | `P0003-rmux检测版本哈希与全平台安装.md` | `oma check` |
| P0004 | `P0004-项目重新定位-通用智能体多路复用任务编排器.md` | 现役定位 |
| P0005 | `P0005-各功能部件POC验证原型.md` | 现役目标 |

## 四、项目日记（`docs\diary\`，一天一篇总结自省）

- `2026-08-29-对照ohmypwsh建立文档骨架.md`
- `2026-08-31-研究体系建设与paste验证.md`

## 五、研究文档（`docs\research\`，S 编号）

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

## 六、开发测试参考（`docs\references\`，R 编号）

| 编号 | 文件 | 用途 |
| --- | --- | --- |
| R001 | `R001-项目定位-通用智能体多路复用任务编排器.md` | 现役定位展开 |
| R002 | `R002-常用命令与管理流程-从项目init到会话cleanup.md` | oma 命令手册 |
| R004 | `R004-测试标准细则-分层断言与门禁流程.md` | 测试分层、断言、闸门 |
| R005 | `R005-选型研究细则-cratesio与github双通道.md` | 选库检索双通道 |
| R006 | `R006-rmux开发参考-连接会话布局与驱动.md` | 写 rmux 相关代码时查 |
| R007 | `R007-agent信任与无阻塞参考-四家配置与检测.md` | 写 init/doctor/hook 时查 |

（R003 退役：原全量清单并入本索引，编号不复用。）

## 七、元规范（`docs\guide\`，G 编号）

| 编号 | 文件 | 用途 |
| --- | --- | --- |
| G001 | `G001-文档标准细则-命名写作规范与rumdl检查.md` | 命名与编号、写作、rumdl |
| G002 | `G002-研究标准细则-结构与六态标记.md` | 研究结构与六态 |
| G003 | `G003-工作流标准细则-从登记到归档五步.md` | 五步工作流与优先级 |
| — | `template.md` | 方案模板（不编号） |

## 八、错误速查（`docs\mistakes\`，文件 M1xx；行级 M0xx）

| 编号 | 分类文件 | 覆盖关键词 | 行级编号段 |
| --- | --- | --- | --- |
| M101 | `M101-drive与paste错误.md` | send-keys、Enter、`C-c`、bracketed paste、marker 假阳性 | M001、M008、M027 |
| M102 | `M102-信任与hook配置错误.md` | 信任框、trust、pretrust、init、yolo | M002、M009-M011 |
| M103 | `M103-文档与命名错误.md` | 命名、显示名、CLI 名、六态、diary | M003-M005、M013-M014 |
| M104 | `M104-rmux安装与CLI调用错误.md` | 安装、`-V`、`-S`、`-L`、`cmd()` | M006-M007、M016、M020 |
| M105 | `M105-agent检测与状态判断错误.md` | PATH、which、idle、Quiet、CPU | M012、M018-M019 |
| M106 | `M106-Windows进程与daemon启动错误.md` | os error 5、Job Object、WMI、exit-empty | M015、M017、M021-M022 |
| M107 | `M107-工具链与脚本错误.md` | sed、grep、PowerShell、中文路径 | M023-M026、M028 |

迭代规则：踩坑按当前最大号接编 MNNN 进对应分类文件（M0xx 行级、新分类用 M1xx 接编）；一行一事；反复踩落 `docs\research\`；改「正确处理」不删历史行；新分类文件登记本节。

## 九、阶段与版本

- `ROADMAP.md`：阶段路线
- `CHANGELOG.md`：版本里程碑

## 十、代码与 pin

- 代码文件位置见第二节表；`catalog\rmux.toml` 是 `oma check` 的信任锚
- `examples` 十个部件 POC 对应方案 P0005 的部件表（yolo-doctor / endpoint / session / layout / drive / dialogs / paste / locate / stream / state）
