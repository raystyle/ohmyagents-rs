# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

web 镜像本地化与主页化（对应 `GOAL.md`，方案 P0022，2026-08-31 用户连发定调：本地化、找源码仓、不要 wrangler、命名 web-mirror-server、主页即可视化、dashboard 删除）。当日完成并经用户验收。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 本地化攻坚 | 已完成 | 四连挫四根因（尾斜杠、e 参数误判、缺 WASM、缺 ACAO）；本地前端全通 | 2026-08-31 |
| 源码仓与构建 | 已完成 | rmux-web-share 源码 clone 进仓；autocrlf 坑；npm build 全绿（SRI 自动重打） | 2026-08-31 |
| serve 目录托管 | 已完成 | /share-fe/* 与 /_astro/* 读盘 + 防穿越 + MIME + ACAO；rebuild 免重编 | 2026-08-31 |
| session 镜像与免 PIN | 已完成 | agent 参数 Option 化（None=整会话）；本地自动 --no-pin | 2026-08-31 |
| 主页即镜像 | 已完成 | GET / 自动起会话镜像注入 token（shim 方案）；dashboard 删除；编排回归 CLI/API/MCP | 2026-08-31 |

## 前目标 0017 与 0018 残表

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

| 目标 | 状态 | 说明 |
| --- | --- | --- |
| P0012 Linux/mac 接管 | 排后 | 用户定调（2026-08-31）：先把 Windows 全量开发好；资产与代码路径就绪；预备检测研究已备（S021 指令集 SIGILL 检测阶梯） |

（P0006 至 P0016 已完成；过程与经验在对应 proven 方案。）
