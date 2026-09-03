# PLAN：当前目标实施计划

## 当前目标：文档体系重构（PRD D01 至 D05）

> 用户定调 2026-09-03「参考 D:\reader_rs 重构项目文档」。四项拍板：引入 PRD 四原语；AGENTS 意图路由细节全下沉 R002；根级五文件禁字合规退出豁免清单（存量 3671 处另跑）；docs\web 不动只登记角色。需求清单见 `PRD.md`。

### 方案骨架

按提交序列推进（依赖有序，8/9 可并行）：

1. **PRD 引入**：新建 `PRD.md`（D 编号与生命周期状态机、与 G003 五步衔接）；G001/G003/INDEX/GOAL/PLAN/TODO/README 头部同步四原语。
2. **R002 扩容**：重排四节八命令族，补七缺命令面（providers/statusline/secrets/self update/key/task/hook 拦截闸）；先于 AGENTS 瘦身防细节在途丢失。
3. **AGENTS 重写**：二节四类场景（对话/操作/编码/文档）加文档对齐义务表 14 行；三节每命令一行摘要，权威指向 R002；8 处下游段名引用同提交同步（操作规则改工作规则、规则号改名字引用）。
4. **INDEX 收敛**：十节收敛九节（删目录树与十节）；P 表去状态化；登记缺陷修复（diary 篇、.tools 四件、src 五件、M105/M107 编号段、P0020 注记）；docs\web 登记资源包输入区；M043 记档。
5. **TODO 清退**：残表删除留指针，只留当前目标与队列三行。
6. **PLAN 与 GOAL 切目标**：本文件换新目标；GOAL 锚点进程切换、断表合并不丢行。
7. **CHANGELOG 与 ROADMAP 补史**：09-01（P0021 至 P0026）与 09-02（P0027/P0028 加密钥权限面加更名）两段里程碑。
8. **G002 CR 修复**：L80 `docs\references\` 路径被字面 CR 劈断，恢复并去 0x0D。
9. **R 系列六态整改**：R004/R006/R007/R009 共八行 `[推断]` 越级，按升实证、改引用式（依据 S 编号）、移研究或删三出口处理；只改标注不改断言语义。
10. **豁免清单退出**：最后执行；五根文件（ROADMAP 验过后六件）从 `md-char-allow.txt` 删行，全仓 mdcharlint 复验零违规。
11. **收口**：diary 新篇、GOAL 历史行、PRD D02 至 D05 转已交付全表复查、TODO 清单收口。

### 验收口径

- PRD D01 至 D05 全部已交付，无残留已采纳
- R002 补齐七缺命令面；AGENTS 意图路由字符占比两成量级
- INDEX 九节与磁盘对账零差异（数量与文件名逐字一致）
- 五根文件 mdcharlint 零违规并退出豁免清单；`rg 三原语|操作规则|工作节奏`（排除 diary/proven/web 与历史叙述）零残留
- 每提交四件验证全绿

### 门禁

`rumdl check .` + `uv run --script .tools\md-ref-scan.py` + `uv run --script .tools\md-heading-scan.py` + `uv run --script .tools\mdcharlint.py`（触碰文件逐个过）；cargo test 不需要（零 src 改动，COMMAND_MAP 只读不动）；SKILL.md 生成物不动不手改。

> 角色：**当前目标方案文档**——基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
