# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

agent doctor 部署诊断（队列顺位接续，用户核心轴「agent 部署、管理、验收与诊断」）：一次性核查四家安装态、yolo、信任、hook 形态、状态栏、登录态（S026 落地）、会话健康。方案见 PLAN。

## 任务进度清单

> agent doctor 切片。

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| oma agents login 引导 | 已完成 | `src\login.rs`：子进程捕获（纯 stderr 无 TTY，源码实证推翻 S026 pane 扫屏原推断）、URL/code 机读标记转发、超时杀进程、成功判据 = 退出 0 且 doctor 落盘凭据过；5 黄金例（源码取证样例）加负例全绿。WSL 实机**双半程闭环**：失败路径（TLS 断连诊断转述）加成功路径（跨机 UX 定调后重跑：用户另一台机器完成授权，`login.ok=true`，doctor 登录态翻绿） | 2026-09-02 |
| 余项 | 排队 | lan-win / lan-mac 远程验收通道（doctor 与 login 同批走）；kimi 侧真登录待需时验 | 2026-09-02 |

## 前目标 P0027 清单

> 已归档；方案与过程在 `docs\proven\P0027-四环境部署自适应-hook形态与状态栏.md`，当日闭环。

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 全链 | 已完成 | 取证（S024）、hook 自适应、状态通道、状态栏重铸（S025）、双环境收敛、测试门禁——详表见 proven | 2026-09-02 |

## 前目标 P0012 清单

> mac 阶段。

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

> 用户定调 2026-09-02「agent 部署、管理、验收与诊断」核心轴，五端视角（本机 / WSL / lan-win / lan-linux / lan-mac）。

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| statusline 覆盖 kimi 与 grok | 已完成 | merge_kimi/merge_grok 幂等合并落位（2026-09-02 当日） |
| S026 grok/kimi OAuth 登录研究 | 已完成 | 双流取证落档（grok loopback+设备码双流、kimi 仅设备码；凭据落盘与纯文件登录态判据）；`oma agents login` 与 doctor 登录态行归 agent doctor 切片落地 |
| agent doctor 部署诊断 | 排队 | 一次性核查四家安装态/yolo/信任/hook 形态/状态栏/登录态/会话健康 |
| agent 密钥管理 age 加 sops | 排队 | 参考 remotex_rs（age 身份自管副本 + sops 加密 + 注入 agent 配置）；密钥体系主权与跨仓密钥扫描归 ohmypwsh，集成而非自建（2026-09-02 四仓定调） |
| 提供商与模型变量注入 | 排队 | claude/codex 用变量配 provider url/key/model（参考 D:\sourcecode\core model_lns 模式），spawn env 指派 |
| rmux 编排扫屏消费 agent:state | 排队 | status/doctor 交叉核对 hook 态与扫屏态 |
| 会话层 bypassPermissions 未生效排查 | 排队 | 用户实测本会话 /permissions 非 bypass（Allow 规则堆积即审批实锤）；配置层 doctor 全绿；2.1.24x 模式优先级与网关限制研究结论待收（claude-code-guide 代理）；oma 侧候选修法：spawn 的 claude 路固定 `--dangerously-skip-permissions` argv |
| 仓库与目录更名 ohmyagents-rs | 已完成 | 五步收口：GitHub 更名（用户）、remote set-url、目录 D:\ohmyagents-rs、双环境重跑 init 加 --pretrust（双侧 doctor.blocked=false）；残留清扫 Cargo.toml/README/main.rs 帮助文案/S028（diary 存档不改） |

（P0006 至 P0026 已完成；过程与经验在对应 proven 方案。）
