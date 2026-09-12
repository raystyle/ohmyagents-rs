# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`；需求的取舍与状态见 `PRD.md`。

## 当前目标

D30 0.6.1 更名收尾批（2026-09-12 立项，D29 残面一次清完）：

- [x] 阶段 1 四回归修（data_dir bin 先建根、退役环旧名死码、SKILL marker 断再生、技能身份回 ohmyagents）
- [x] 阶段 2 串面清扫（README 安装块与四处 URL、doctor/verify/deploy/main/statusline 用户可见串、UA 两处、源注释顺手批、R002 与 AGENTS 同步、ome 改 ark 注释尾巴随批清）
- [x] 阶段 3 测试与门禁（新增两用例 156 单测加 24 集成全绿、fmt/clippy、rumdl 加 .tools 三扫描；本仓 dogfood init 重部署与 pretrust 全绿）
- [ ] 阶段 4 发版（0.6.1、CHANGELOG、diary、tag、digest 双源核对、herdr 知会）
- [ ] 阶段 5 本机老用户迁移实证（stub 指路、装 hst、rename 迁移加 heal、doctor 绿、转调、散件合并回）
- [ ] 阶段 6 归档（P0047、四原语收口）

D29 前目标（2026-09-12 当日闭环归档 P0046，v0.6.0 发版；repo 改名 hst-rs 用户已办）清单见 git 历史：

- [x] 阶段 1 至 7 全落（154 单测加 24 集成加三平台矩阵、八条验收、十二资产 digest 双源核对）

D28 前目标（2026-09-11 四轮裁定当日闭环归档 P0045，v0.5.4 发版对端验锚全绿）清单见 git 历史：

- [x] shim 用户级加 session 分键双写与 SessionEnd GC（src\shim.rs）
- [x] 注册四家迁用户级（claude/codex/grok/kimi，src\deploy.rs）
- [x] 状态栏 oma 段读序重排（src\statusline.rs）
- [x] oma hook 用户级分键写与陈旧清理（src\hook.rs）
- [x] init 迁移清理项目 ours 注册与 shim 退役（src\deploy.rs）
- [x] doctor hooks.form 与 state 检查面迁用户级（src\doctor.rs）
- [x] verify 临时用户级注册泛化与判据隔离（src\verify.rs）
- [x] 测试与四门禁全绿；herdr codex review 三轮对齐（151 单测加 24 集成；F1 信任键源根修加 R2 六条清零）
- [x] 文档同步（R002/R004/AGENTS/INDEX/CHANGELOG/COMMAND_MAP/SKILL/S025 追记）加 P0045 归档
- [x] v0.5.4 发版（digest 对端验锚加 herdr 知会 ohmycloud）

挂账：无（M060 两笔 2026-09-11 清讫：guard 透传白名单化加 shebang 注入缝）。

## 前目标清单

> D21 oma diagnose 活性诊断族（2026-09-09 归档 P0041）：cache 双连探测加三连取优加 ds 特判、agents 配置指向加在册加 key 活性加 thinking 对照；131 单元加 21 集成与四门禁全绿；真网关实收。

> D20 去 token 注入收窄（2026-09-09 归档 P0040）：五功能收敛；secrets / providers / login / install / update 五面删除，净删约三千行；124 单元加 21 集成与四门禁全绿。

> D18 状态栏用户级定制（2026-09-09 归档 P0039）：拆段拼装加生成时烘焙，segments / template / icons / codex items 三层键加 --script 整替换；145 单元加 24 集成与四门禁全绿。

> D17 oma agents verify 无头验收（2026-09-08 归档 P0038）：状态栏 mock 直跑加 hook 无头落盘两层判据；本机四家全绿。

> D19 trace 六视图全量恢复（2026-09-08 归档 P0037）：只读检索面回归；原样带回零适配；116 单元加 18 集成全绿。

> D16 oma self update 镜像通道（2026-09-08 归档 P0036）：OMA_MIRROR dev 段边车判新、sha256 强制校验、网络失败回落 GitHub。
> D15 oma 去编排收窄为纯部署配置工具（2026-09-08 归档 P0035）：编排命令与 rmux 后端移除；保留 init / doctor / agents / hook / self update / completions；文档八面同步。
> D14 数据目录改名为 `.oma`（2026-09-07 归档 P0034）：项目与家目录两根同改；旧 `.ohmyagents` 仅旧在则迁；不是 `.omc`。
> D12 任务目录孤儿（2026-09-07 归档 P0033）：`.ohmyagents/t006/` 是 t008 第二轮草稿误落，已删；协议尾注路径显式 `tasks/`。
> D11 状态栏 zig/go/cpp（2026-09-07 归档 P0032）：projKind 扩展；本机临时目录实跑；Grok 仍跳过工具链。
> D10 G005 存量字符清理（2026-09-07 归档 P0031）：SKIP_DIRS 外四类禁字清零，封闭清单删除。
> D08 doctor 检查面补形态（2026-09-07 归档 P0030）：Grok 状态栏 command 三态、JSON hook args 形态；用户实证栏正常。同日 D09：oma 不管种子。
> D07 oma 收窄配合迁册（2026-09-07 归档 P0029）：agents install/update deprecated、agents.toml 冻结历史锚、doctor 四类归 agents 域；当日热修 M044 至 M048。
> 文档体系重构（D01 至 D05，2026-09-03）：全链已完成。
> D06 agent 二进制下装部署五端全量收敛（2026-09-05）：当日闭环后同日方向反转（D07）。

## 队列目标

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| M060 两笔加 ome 更名尾巴随下个功能版发 | 排队 | 已落 main 滚动（oma/dev 段随 CI 可取）；用户裁 2026-09-11 攒着，随下个功能版打 tag 发出；同批换代码内 ome hint 串为 ark（ome 更名 Ark 周知 2026-09-12，文档面已换） |
| D13 mac `--version` 一致性 | 完成 | 2026-09-09 lan-mac 经镜像 stable 段直装 v0.3.0 实收：边车 sha256 校验过（digest 与 ohmycloud 对账一致，mirror 第二端点消费实证）、`--version` = oma 0.3.0、agents 四家检测与 `statusline --example` 冒烟绿；旧件备份 `~/.oma/oma-fossil-backup` |
