# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**——基于 `docs
esearch\`（为什么）与 `docs
eferences\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：agent doctor 部署诊断

> 用户核心轴「agent 部署、管理、验收与诊断」队列顺位项（2026-09-02 起）。一次性核查四家：安装态、yolo、信任、hook 形态、状态栏、登录态、会话健康。

### 方案骨架

- 基础：现有 `oma doctor` 已覆盖 yolo、trust.*、binary、caps、state——在其上聚合而非另起炉灶（依据 R002 命令细则、S026 登录态判据）。
- 新增检查项：登录态（S026 纯文件判据：grok `~/.grok/auth.json` scope 键未过期、kimi `~/.kimi-code/credentials/kimi-code.json` access_token 非空）；状态栏形态（S025 四家落位文件存在性与 oma 段标记）；hook 形态标记（`init.hooks.form=` 同型口径）。
- 会话健康：无 rmux 会话时不误报（部署诊断先于会话存在）。
- 输出：按 agent 分组一行一检查（status=ok|warn|block 同现有 doctor 行协议），`doctor.blocked=` 汇总不变。
- `oma agents login [名]`（S026 待办）：pane 内起 `grok login --device-code` / `kimi login`，扫屏转发 URL+code，完成扫 `✓ Signed in` / `Logged in` 确认——独立切片，本目标先落检测不落引导。

### 验收口径

- 本机 Windows 与 WSL 双侧四家全绿（或带明确 warn 而非误报 block）；lan-mac / lan-win 远程验收通道可用（mac `ssh ray@lan-mac`、Windows `ssh ray@lan-win`）。
- 测试：新增检测各有单测（判据来自 S026 事实源黄金样例，不镜像实现）；rmux 依赖项带闸门。

### 门禁

`cargo fmt --all -- --check` + `cargo clippy` 存量告警不新增 + `cargo test`（隔离 target）+ `rumdl check .` + `md-ref-scan.py` + `md-heading-scan.py`；提交精确 add（M036）。

## 完成的定义

> 本目标验收口径。

- mac 真机：达成（2026-09-01）——构建与测试基线全绿、rmux 资产验收、四家 agent 安装、真身四路 + settle 全链绿（含真任务与 hook 流）。
- WSL Linux：达成（2026-09-01）——第一棒（构建/基线/daemon/分类器/stub 全链）加补尾棒（四家 `--force` 安装探针全绿、真身四路 + settle、`oma task` 真任务产物精确、doctor 零阻塞、cleanup 零残留）。
