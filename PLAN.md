# PLAN：当前目标实施计划

## 当前目标：D07 oma 收窄配合迁册

> 已达成 2026-09-07，归档 P0029。回指 `PRD.md` D07。用户 2026-09-05 追问链三轮六裁方向反转：agent 二进制下装部署回归 ohmyenv-rs（ome），oma 收窄为配置 agent、hook、编排。2026-09-07 用户裁定不等 ome 仓 D07 切片 3 先清自身面，ome 侧以 issue 交底。下目标立项时本文件重写。

### 事实基线

> 2026-09-07 迁册批开工盘点。

- ome 仓源码已含 D07 切片 1 与 2：四家入册 `catalog\tools.toml`、`ome install` 幂等（PATH 在位即跳过，与 oma 判定同口径）、doctor 三层 [实证： ohmyenv-rs 461a178 / 9179893 与 ohmyagents#5]
- 本机 ome 部署位（`%LOCALAPPDATA%\Programs\ome`）与 catalog 同步层停 2026-09-02 旧版，`ome install claude` 报「未知工具」[实证： 2026-09-07 本机实跑]
- oma 侧 agents install / update 行为面稳定：P0012 三平台全链与 D06 五端验收基线 [实证： GOAL 历史 2026-09-01 / 2026-09-05]

### 方案骨架

> 迁册批三件（ohmyagents#5 请求），oma 自身面。

1. **deprecated 提示**：`cmd_agents_install` / `cmd_agents_update` 入口 stderr 打 `oma.deprecated` 指向 `ome install`（不删命令、stdout kv 与 json 面 R011 不动）；clap 帮助与 COMMAND_MAP 同步；集成测试两条钉 stderr 提示与退出码。
2. **`catalog\agents.toml` 头注记**：数据权威转 ome `catalog\tools.toml` agent 四节，本文件冻结历史锚（pin 与 sha 不再随上游滚动）。
3. **doctor 四类检查归 agents 域**：R002 与 AGENTS 归类重排（登录态 / hook 形态 / 状态栏 / 会话健康归 agents 域；二进制在位与版本、token 诊断归 ome doctor）；oma doctor 代码不动（binary 在位探查保留作 spawn 前置，ohmyagents#5「维持」口径）。

### 验收口径

- 两条新集成测试绿（stderr 提示含 `oma.deprecated` 与 `ome install`，未知名快败不触网）
- 既有测试全绿、fmt / clippy 零告警、md 三件套过（触碰文件）
- 四原语与 AGENTS / R002 / INDEX 同步；根 SKILL 重跑 `oma init` 再生
- 跨仓：ohmyenv-rs issue 交底部署滞后；ohmyagents#5 回填

### 门禁

`rumdl check .` + `uv run --script .tools\md-ref-scan.py` + `uv run --script .tools\md-heading-scan.py` + `uv run --script .tools\mdcharlint.py`（触碰文件逐个过，存量欠账 G005 排队项管辖不扩批）；src 改动加 cargo test 与 fmt / clippy 零告警。

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
