# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

P0012 平台接管三阶段当日齐（用户定调 2026-09-01：WSL Linux 第一棒 → mac 接管收口 → 回 WSL 补尾）：Windows / mac / WSL Linux 三平台真机全链绿。P0012 全部任务项完成，待用户定向归档（proven 方案回填）。

## 任务进度清单

> mac 阶段清单。

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| mac 环境搭建 | 已完成 | 基线 79+10 / 82+10 全绿（比 Linux 各多 1 例系 a4160d6 Unix 提示符单测）；build --features server,mcp 25.7s | 2026-09-01 |
| rmux mac 资产验收 | 已完成 | `oma check` arm64 全绿：PATH 发现 rmux 0.10.0、darwin 资产 sha 对锚、dispatcher/helper/daemon 哈希齐、自管根布局 | 2026-09-01 |
| 四家 agent mac 安装 | 已完成 | 检测四家全绿（PATH）；`--force` darwin 四家全链验收；抓到 grok 双 CDN 无 macos pin，实测推翻 S017 假设并自算 sha 补 pin（47b1ddd） | 2026-09-01 |
| 真身四路 + settle 真机 | 已完成 | 抓到三路信任屏措辞漂移 + codex hooks 屏数字菜单新形态，补 marker 加黄金行回归（7211d41）；全新项目 settle 窗口四路全收零手工、`oma task` 真任务产物精确、hook 流通、doctor.blocked=false | 2026-09-01 |

## WSL Linux 阶段清单

> 两棒收口：第一棒（构建/基线/daemon/分类器）加补尾棒（安装/真身四路），2026-09-01 当日齐。

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| WSL 环境搭建 | 已完成 | 全量构建 43s 过；测试基线首跑 5 败（1 真 bug M041 + 4 处测试 Windows 假设）全修复；78+10 与 81+10 全绿零警告；重跑 `oma init` 修复 hook 的 Windows 路径报错 | 2026-09-01 |
| rmux Linux 资产验收 | 已完成 | `oma check` 全绿：PATH 发现 rmux 0.10.0，asset/dispatcher/helper/daemon 四层 sha256 对 pin 全过；unix socket 链路 stub 实测通 | 2026-09-01 |
| daemon 启动路径 | 已完成 | `boot_new_session` Unix 分支（无 Job Object，裸 spawn + stdio 置空 + 独立进程组）；分类器 Unix 提示符判据与 serve `process_group(0)` 同批落地；stub 全链验收（spawn/status/send/respawn/cleanup/serve/doctor/HTTP 信封） | 2026-09-01 |
| 拉取后基线复验 | 已完成 | mac 侧 marker 与 catalog 改动拉回后 80+10 / 83+10 全绿（黄金行回归计入） | 2026-09-01 |
| 四家 agent Linux 安装 | 已完成 | `--force` 真下载四家全链绿：claude 2.1.251 / codex 0.151.0（嵌套 bin 布局）/ grok 1.0.13（CDN 裸二进制）/ kimi 0.39.1（zip），自管根落位、探针全过、双源检测 `extra=` 行正常 | 2026-09-01 |
| 真身四路 + settle 真机 | 已完成 | 四 pane 真身拉起；settle Linux 实拍命中（codex 数字菜单双 marker、kimi don't trust Up+Enter）；grok 家目录阻塞（yolo+信任）用 `oma init --pretrust` 清零、`doctor.blocked=false`；`oma task` t026 真任务产物精确（斐波那契前 10 项）；claude/codex hook 流在写状态；cleanup 零残留 | 2026-09-01 |

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 切片 1 安全与僵局 | 已完成 | 看板默认 spectator 只读、Host 校验（高5）、connect label 兜底解 cleanup 僵局（高2）、死路杀旧 pane（高3）、manifest 原子写（高1a）；计划外修 serve daemon 零控制台卡死（CREATE_NO_WINDOW） | 2026-09-01 |
| 切片 2 并发与语义 | 已完成 | alloc_task_id 原子占位（高1b）、三秒级同步段进 spawn_blocking（高4 分档）、reconcile 判活用 plan.stub 并回写（中8，语义=补缺不移除） | 2026-09-01 |
| 切片 3 健壮性批 | 已完成 | send baseline（中6）、slug 词法归一 16hex（中7，实踩 canonicalize 时序双身份）、web_share 行锚点（中9）、status warning（中10）、SSE error event（中11）、settle 行级短行（中12） | 2026-09-01 |

## 前目标 0024 残表

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| reconcile 三态 | 已完成 | 会话不在新开；在则活路附加、死路重开（split 回一路回写 manifest） | 2026-09-01 |
| oma respawn | 已完成 | 强制关闭打开一路（kill-pane 单窗格）；api/CLI/HTTP 三面 | 2026-09-01 |
| 四连验收 | 已完成 | 新开/附加/杀路重开/强制重开（pane-only）stub 全绿 | 2026-09-01 |
| P0025 serve 守护化 | 已完成 | serve start 即调即退（DETACHED 孤儿化）；stop 协议化停机次轮补齐（DELETE /shutdown 优先，超时降级强杀），实测日志见 draining | 2026-09-01 |
| G004 经验沉淀细则 | 已完成 | 成功经验 proven/references 双链、错误经验 mistakes 当场记加二犯升格；挂 AGENTS 工作节奏第 5 条强规则位 | 2026-09-01 |
| M035 记档 | 已完成 | python str.replace 吃路径 \r 转义劈行；修复过程又踩同型两次（记档本身的教训） | 2026-09-01 |
| README 三段重写 | 已完成 | 项目介绍/安装部署/完整命令示例（含典型用法）；serve start 形态次轮补齐 | 2026-09-01 |

## 更早前目标 0017 与 0018 残表

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 立项 0017 | 已完成 | 五切片定界：send 间隔、HTTP trace、SKILL 命令图、grok 无头、mcp 配置打印 | 2026-08-31 |
| send 间隔产品化 | 已完成 | expect_visible_text 等末行短头可见再 Enter；超时降级照发留痕；stub 验证 visible | 2026-08-31 |
| HTTP trace 端点 | 已完成 | 三端点挂 api 现成函数；/api 11 端点；网页轨迹面板；本仓真数据全绿 | 2026-08-31 |
| SKILL 命令图生成 | 已完成 | COMMAND_MAP 生成；标记覆写加旧版升级加用户内容跳过三态；活体验证 | 2026-08-31 |
| grok 无头实跑 | 已完成 | `--always-approve -p` 写文件 exit 0 产物精确；联邦 trace 同场检出；S007 回填 | 2026-08-31 |
| mcp 配置打印 | 已完成 | `--print-config` 三形态片段；featureless 构建也可用 | 2026-08-31 |
| 0018 指令集检测 | 已完成 | 用户反问触发：caps 检测进 doctor、退出分类进 agents 与装机；本机 avx512f=false 实测 | 2026-08-31 |

## 队列目标

（无现役——P0012 三平台（Windows / mac / WSL Linux）全部完成，待归档后起新目标。）

（P0006 至 P0026 已完成；过程与经验在对应 proven 方案。）
