# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**——基于 `docs
esearch\`（为什么）与 `docs
eferences\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。

## 当前目标：P0027 四环境部署自适应

> 已完成，待归档。

> 用户定调 2026-09-02。方案与过程回填 `docs\proven\P0027-四环境部署自适应-hook形态与状态栏.md`；依据 S024（部署自适应矩阵）、S025（四家状态栏矩阵与机读标记）。

### 接续口径

- 下一目标候选（S025 待办升格）：`oma agents statusline` 扩展 kimi（`~/.kimi-code/tui.toml [status_line].command`，300ms/1s 节流约束）与 grok（`~/.grok/config.toml [ui.status_line] type=command`，仅用户级）；rmux 编排扫屏消费 `agent:state` 标记（status/doctor 交叉核对）。
- 远程验收通道（用户 2026-09-02 提供）：mac `ssh ray@lan-mac`、Windows `ssh ray@lan-win`、本机 WSL、本机——四端可验收。
- 门禁：`cargo fmt --all -- --check`（2026-09-02 起进门禁）+ `cargo clippy` 存量告警不新增 + `cargo test`（隔离 target）+ `rumdl check .` + `md-ref-scan.py` + `md-heading-scan.py`；提交精确 add（M036）。

## 完成的定义

> 本目标验收口径。

- mac 真机：达成（2026-09-01）——构建与测试基线全绿、rmux 资产验收、四家 agent 安装、真身四路 + settle 全链绿（含真任务与 hook 流）。
- WSL Linux：达成（2026-09-01）——第一棒（构建/基线/daemon/分类器/stub 全链）加补尾棒（四家 `--force` 安装探针全绿、真身四路 + settle、`oma task` 真任务产物精确、doctor 零阻塞、cleanup 零残留）。
