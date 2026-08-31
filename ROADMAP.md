# ROADMAP

项目全局路线：**大里程碑**。状态：`未开始` / `进行中` / `已完成` / `排后`。细碎轨迹见 `docs\diary\YYYY-MM-DD-*.md`。方案详情见 `docs\proven\NNNN-*.md`。

## 阶段总览

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| 0 | 项目基础设施：对照 ohmypwsh 的文档与目录；定位为通用智能体多路复用任务编排器 | 已完成 |
| 1 | 各功能部件 POC 验证原型 + rmux pin | 已完成 |
| 2 | 项目级自动配置（init）+ 多路 session + CLI 编排 | 已完成 |
| 3 | 编排三传输：HTTP 编排面加可视化网页、MCP、REPL | 已完成 |
| 4 | Windows 全量收口：安装接管、联邦检索、输出与易用、指令集检测 | 已完成 |
| 5 | Linux/mac 端点接管与验收 | 排后 |

## 阶段 0：项目基础设施

AGENTS 四段职责、GOAL/TODO/PLAN 三原语、方案 0001/0002/0004、research、references、rumdl 加自研扫描门禁。现役定位见 P0004。

## 阶段 1：rmux 与脚手架

> 方案 P0005。

十二件 `examples/poc-*` Windows 范围全绿；`oma check` 按 `catalog/rmux.toml` 装 pin 的 rmux 0.10.0。

## 阶段 2：会话与编排

> 方案 P0006 至 P0010。

`init` 全套（hook/skill/yolo 键，幂等不动家目录）；项目专属会话 spawn/status/send/cleanup；三段式粘贴；`oma run` 状态门分派；真四路拉通与 settle 自愈信任。

## 阶段 3：编排三传输

> 方案 P0011 与 P0016。

api 传输无关层一份核心三消费：`oma serve`（六操作 RESTish、JSON 信封、可视化网页、SSE 渲染行画面、trace 端点）、`oma mcp`（stdio 九 tools）、REPL（裸 `oma` 内嵌编排面）。

## 阶段 4：Windows 全量收口

> 方案 P0012、P0013、P0014、P0017、P0018。

rmux 与四家 agent 的自适应安装接管（两层 pin、渠道序、装后探针）；联邦轨迹检索六视图（grok updates 权威日志）；send 回显间隔、SKILL 命令图、grok 无头、`--json` 信封、TTY 表格、completions、`oma mcp --print-config`；CPU 指令集能力检测与探针异常退出分类。

## 阶段 5：跨平台

> 排后（用户定调 2026-08-31：先把 Windows 全量开发好）。

用户定调（2026-08-31）：先把 Windows 全量开发好。资产与代码路径就绪（Windows named pipe 与 POSIX unix socket 双形态已在端点层分流）；指令集 SIGILL 预备检测研究已备（S021 检测阶梯）；待环境切换后按 P0012 验收。
