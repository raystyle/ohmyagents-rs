# ROADMAP

项目全局路线：**大里程碑**。状态：`未开始` / `进行中` / `已完成` / `挂起`。细碎轨迹见 `docs\diary\YYYY-MM-DD-*.md`。方案详情见 `docs\proven\NNNN-*.md`。

## 阶段总览

| 阶段 | 目标 | 状态 |
| --- | --- | --- |
| 0 | 项目基础设施：对照 ohmypwsh 的文档与目录；定位为通用智能体多路复用任务编排器 | 已完成 |
| 1 | 各功能部件 POC 验证原型 + rmux pin | 进行中 |
| 2 | 项目级自动配置（init）+ 多路 session + CLI 编排 | 未开始 |
| 3 | 可选网页镜像；默认不弹窗；`--no-web` | 未开始 |
| 4 | Linux/mac/WSL 端点与无浏览器验收 | 未开始 |

## 阶段 0：项目基础设施

AGENTS 四段职责、GOAL/TODO/PLAN 三原语、history 方案 0001/0002/0004、research、references、rumdl。现役定位见 0004。

## 阶段 1：rmux 与脚手架

`oma check` 已能装 pin 的 rmux 0.10。项目级 `init --yolo` 与 `doctor` 已作为 POC 薄 CLI 落地。其余按方案 0005 做 `examples/poc-*`；全绿后再写产品 `spawn` / `send` / `cleanup`。

## 阶段 2：会话与 Drive

`init` 只写项目目录；2x2 spawn；三段式 drive；状态 JSON。

## 阶段 3：观察面

每 pane 一条 WS 二进制流；控制面仍是 CLI。

## 阶段 4：跨平台

Windows named pipe 与 POSIX unix socket；无 DISPLAY 时 CLI 完整可用。
