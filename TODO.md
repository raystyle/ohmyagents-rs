# TODO：当前目标任务进度清单

> 角色：**当前目标的任务进度清单**。当前目标完成后，过程与经验回填到 `docs\proven\` 对应方案，并起新清单；当天做的事只记 `docs\diary\`。

## 当前目标

官方 web 镜像集成（对应 `GOAL.md`，方案 P0021，2026-08-31 用户定调「和官方的 demo 一样」并点破 rmux 自带 web-share）。当日完成。

## 任务进度清单

| 任务项 | 进度 | 说明 | 日期 |
| --- | --- | --- | --- |
| 立项 0021 | 已完成 | 集成定界：oma 接管 rmux web-share，自建 xterm 桥下线 | 2026-08-31 |
| api 与 HTTP 面 | 已完成 | web_share/shares/stop 三函数（stderr 合并解析）；POST/GET/DELETE 三端点 | 2026-08-31 |
| CLI 与网页 | 已完成 | `oma web [agent]`；状态卡「官方镜像」按钮 | 2026-08-31 |
| 三面验收 | 已完成 | CLI 打 URL/PIN、HTTP list/create/stop、用户浏览器 operator 直操 | 2026-08-31 |

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
