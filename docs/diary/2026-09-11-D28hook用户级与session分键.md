# 2026-09-11：D28 hook 用户级与 session 分键

> 当日：状态栏跨项目失效根修（D28）落码、文档同步、herdr codex review、v0.5.4 发版。方案与过程见 `docs\proven\P0045`；本篇记流水与自省。

## 做了什么

- **D28 立项到落码当日闭环**：用户诊断四点（状态栏命令用户级、采集 hook 与 shim 项目级、codex 与 claude 注册面不一致、单键互踩）加六点裁定一次给齐；shim/deploy/statusline/hook/doctor/verify 六面迁移到用户级注册 + `~/.oma/state/` session 分键协议；init 迁移退役项目级 ours 注册与 shim。
- **测试与隔离缝**：`OMA_USER_HOME` 与 `OMA_HOME` 双 env 重定向贯穿 init/doctor/verify/statusline/hook，集成测试不再碰真实家目录；146 单测加 23 集成全绿（live verify 四家真机过）。
- **文档**：R002 四行（doctor/hook/init/verify）、R004 三.6 隔离缝、S025 D28 追记、AGENTS 路由与边界、INDEX 代码表、README、COMMAND_MAP/SKILL、P0045 归档。

## 踩坑

- **进程级 env 测试的锁要跨模块共享**：doctor 各自的静态锁锁不住 hook 模块同时 set/remove 的同名 env（OMA_HOME 竞态翻车两条测试）。解：`pathutil::ENV_LOCK` 共享。
- **pwsh 子进程行为测试必须钉 HOME**：状态栏新读序会看真实 `~/.oma/state`，用户手治的活会话 working 态污染 unknown 断言。解：spawn 时 USERPROFILE/HOME 钉 scratch。

## 追记：同日四轮与三轮 review

- 第 2 轮（用户「包括yolo模式和非阻塞模式也要是用户级别」「看看git历史 类似需求我们已经提过无数次」）：yolo 与非阻塞键用户级；历史脉络落档（93ac1f9 POC「不改家目录」起逐面翻正：状态栏 P0027、kimi M058、D22、D28 hook、yolo）。
- 第 3 轮（「所以我们的yolo设置命令要分用户级和项目级」）：--yolo 用户级与 --project-yolo 项目级互斥；doctor 双级接受。
- 第 4 令（「项目级和用户级 冲突要可用诊断和告警出来 形成CTA」）：doctor 双级冲突 warn 加对齐 CTA。
- 状态栏中文目录乱码（用户报修，中文路径显示 GBK 误解码形）：stdin 字节级 UTF-8 解码根修。
- herdr codex 独立 review 三轮：首轮九条（F1 codex 信任键源 = hooks.json 路径，本机 0.149.1 加 0.154.0 无 bypass 活体落盘实证，M061）、次轮六条残留、终局代码面零异议（四条收口项当轮清完：README 安全提示、COMMAND_MAP/SKILL、误入仓脚本、四原语对账）。
- v0.5.4 发版。

## 自省

- 状态通道断裂的本质是「采集面与消费面不同层」：用户第 4 点诊断（shim 写态通但状态栏仍 unknown）指出的正是协议没跟上层迁移，根修必须读写两侧同层同键，半边迁移必留 unknown 显影。
- 用户级注册顺带消灭了跨环境共享项目目录的双侧注册协调（v0.5.x 的 codex 字段所有权复杂度的根源场景），架构上是一次净简化。

## 明日候选

- 验 v0.5.4 对端验收回执；三平台矩阵（WSL 与 lan-mac 各跑一轮）。
- 挂账沿旧：guard 透传白名单化、mac shebang CI 断言（M102）。
