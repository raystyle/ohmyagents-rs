# PLAN：当前目标实施计划

## 当前目标：D06 agent 二进制下装部署五端全量收敛

> 回指 `PRD.md` D06。用户 2026-09-05 裁决三边界：安装行为不动（保持幂等检测安装）、存量原地纳管（不重装不删）、五端全量；配置域（settings、API key、MCP、statusline 等）除外。

### 事实基线（2026-09-05 立项盘点）

- 本机 Windows：`oma agents` 四家 installed=4、全 source=path（原地纳管探测在工作）；`oma agents install` 四家全 skipped（幂等）[实证： 当日实跑]
- P0012 已收口本机三台（Windows / WSL / lan-mac 同机开发位）四家安装全链 [实证： GOAL 历史 2026-09-01]
- 五端真实缺口：lan-win 与 lan-linux 两端的 oma 可达性、agent 盘点与幂等验收均未做 [推断： 待切片 2 盘点证实]
- oma `--version` 不支持（clap 未挂版本参数）；ome catalog 的 oma 条目集成条件为此加发布通道裁决（见 ohmyagents issue #2 #3）[实证： 当日实跑与 ome catalog 注释]
- 存量越界物 `D:\ohmyenv\claude\claude.exe`（ohmyenv.ps1 时代遗产）：按「原地纳管」裁决 oma 仅探测不迁移；EnvRoot 清理归 ome 域另议，本批不发

### 方案骨架（四切片，1 与 2 可并行）

1. **切片 1：oma --version 支持**：clap `version` 挂 Cargo.toml 版本（build.rs 嵌资源口径对齐）；SKILL.md / R002 / README 命令面同步；解开 ome catalog 集成条件之一。
2. **切片 2：lan-win 与 lan-linux 下发加盘点**：oma 二进制下发（sha 对比按需传，sync 脚本固化 `.tools\`，对齐 ome 的 sync-ome-lanwin 模式）；两端 `oma agents` 探测盘点（来源 / 版本 / 路径矩阵落 TODO）。
3. **切片 3：五端幂等验收**：五端各跑 `oma agents` 加 `oma agents install`（存量 skipped 纳管、缺失补装），install 二连跑零变更；mac 加 WSL 复验一次（P0012 后回归）。
4. **切片 4：边界收口与跨仓 ISSUE**：AGENTS 一、边界段加一行（agent 二进制下装部署归 oma，配置域除外口径）；ohmypwsh 发 agent 域退役配合 ISSUE；ohmyagents #2 #3 复核 ome 集成条件进度（--version 已解、发布通道待裁）。

### 验收口径

- 五端 `oma agents` 四家每端 installed（存量纳管或补装后）
- 五端 `oma agents install` 幂等：二连跑输出与退出码一致、零文件变更
- `oma --version` 可用且与 Cargo.toml 一致
- 边界声明落 AGENTS；跨仓 ISSUE 两件发出
- 每提交门禁全绿

### 门禁

`rumdl check .` + `uv run --script .tools\md-ref-scan.py` + `uv run --script .tools\md-heading-scan.py` + `uv run --script .tools\mdcharlint.py`（触碰文件逐个过，存量欠账 G005 排队项管辖不扩批）；src 改动加 cargo test 与 fmt / clippy 零告警。

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
