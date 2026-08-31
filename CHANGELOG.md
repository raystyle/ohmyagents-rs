# Changelog

本文件只记录**大版本里程碑**：定位变更、发布、阶段完成、核心能力整体落地。细碎条目由 `docs\diary\YYYY-MM-DD-*.md` 与 git 历史承载。

## [Unreleased]

### 里程碑

> 2026-08-29 至 2026-08-31：从空仓库到 Windows 全量可用。方案 P0001 至 P0019，过程与经验在各 `docs\proven\` 文档。

- **项目定位**（P0004）：Oh My Agents，通用智能体多路复用任务编排器。首期切面见 P0001。
- **部件 POC 全绿**（P0005）：Windows 范围十二件 example（端点、会话、布局、驱动、对话框、粘贴、定位、流、状态、部署、负例、label 桥）。
- **产品命令闭环**（P0006 至 P0010）：spawn/status/send/cleanup 全链路、send 多行三段式粘贴（中文验收）、oma run 状态门分派、真四路拉通（claude 路全通）、settle 自愈信任（codex hash 复现与白名单点框互兜）。
- **自适应本机安装部署**（P0012）：oma 接管 rmux 与四家 agent 的安装——catalog 两层 pin（出厂锚 + `~/.ohmyagents` 用户本地层写回）、渠道序 github 主 CDN 兜底、sha256 信任锚、装后探针；`oma agents install/update`。Windows 四家装机全绿。
- **联邦轨迹检索**（P0013/P0014）：`oma trace` 六视图查询时直读四家原生会话库；双意图、operation_id 归组、epoch ms 归一；grok 主源升级 updates.jsonl 权威日志（逐事件真实时间，S020）。
- **三传输编排面**（P0011/P0016）：api 传输无关层一份核心三消费——`oma serve`（六操作 RESTish、JSON 信封、可视化网页、SSE 渲染行画面、trace 端点）、`oma mcp`（stdio 九 tools）、REPL（裸 `oma`，编排面内嵌）；三通道共测全绿。
- **输出与易用**（P0015）：六会话命令 `--json` 信封、`oma status` TTY 表格、`oma completions`、R002 输出规范节。
- **Windows 全量收口**（P0017）：send 回显间隔产品化（S005 铁律）、SKILL.md 命令图生成（S016 末件）、grok 无头实跑（S007 回填）、`oma mcp --print-config`。
- **指令集检测**（P0018）：`oma doctor` CPU 能力段（avx/avx2/avx512f）与探针异常退出分类（illegal-instruction 带缓解 hint），S021 问题类的 Windows 落地。
- **文档地基**：AGENTS 四段、三原语、P/S/R/G/M 编号体系、六态标记、rumdl 加两件自研扫描进门禁、`.tools` 脚本归档。

### 排后

- Linux/mac 接管（P0012 跨平台面）：资产与代码路径就绪；指令集 SIGILL 预备检测研究已备（S021）。
