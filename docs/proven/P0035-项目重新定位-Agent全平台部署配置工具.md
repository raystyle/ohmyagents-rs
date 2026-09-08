# 项目重新定位：Agent 全平台部署配置工具

- 状态：已完成（定位与命令面已改，验收全绿）
- 日期：2026-09-08
- 关联：PRD D15；`docs\references\R001-项目定位-Agent全平台部署配置工具.md`；上一版定位为方案 0004

## 背景与问题

P0004 把 oma 定位为「通用智能体多路复用任务编排器」，运行时后端是 rmux。2026-09-08 用户三裁：「还是 oma」「oma 退化为纯部署配置工具」「和 HOOK 和状态栏工具」（PRD D15 第 2 轮）。编排面整体移除，oma 收窄为 Agent 全平台 token、hook 与状态栏部署配置工具。

## 裁定与方案

- 不改名：CLI 仍 `oma`，仓库仍 `ohmyagents-rs`。
- 移除面：编排命令 check / spawn / respawn / status / send / key / run / task / settle / cleanup / REPL / web / serve / mcp / trace；rmux 运行时后端与 `rmux-sdk` 依赖；可选 feature server / mcp；`build.rs` 资源包打包与 `catalog\rmux.toml`；十四件 `examples\poc-*.rs`；编排时代工具 `review-round.py` 与 `share-view-probe.py`。
- 保留面：`oma init`（hook/skill/yolo 部署全套）、`oma doctor`（摘会话健康类）、`oma agents`（检测 / login / providers / statusline / secrets，install 与 update 仍 deprecated 指向 ome）、`oma hook`（状态落盘加密钥拦截）、`oma self update`、`oma completions`；`--format` 信封冻结面（R011）不动。
- token 定位：`oma agents secrets` 一钥两密文加四 shell 懒注入为 token 部署配置主面；providers.toml 别名簿保留为配置面（注入消费面 spawn 已移除）。四仓边界不动，密钥体系主权仍归 ohmypwsh。
- 技术处置：rmux.rs 中与 rmux 无关的通用归档工具剥离为 `src\archive.rs`（install / update 继续用）；api.rs 的 JSON 信封函数迁入 `src\fmtio.rs`（形状冻结不变）；`src\deploy.rs` COMMAND_MAP 与 SKILL 模板改部署配置面，重跑 `oma init` 四家 SKILL 再生。

## 过程与验收

- 切片 1 代码面收窄：十一个编排模块整删，main.rs 重写为六命令分发，Cargo.toml 摘五个编排依赖加两个 feature 加 build-dependencies；`cargo build` 绿（含 `--locked` 与 release）。
- 切片 2 保留面回归：doctor 摘会话健康；tests/cli.rs 删编排用例保部署配置面；97 单元加 17 集成全绿 [实证： 2026-09-08 本机 cargo test]。
- 切片 3 文档对齐：AGENTS 定位与意图路由改写；R001 / R002 改名重写；INDEX 代码表与退役注记；README 三段重写；SKILL 重跑 init 再生；CHANGELOG / ROADMAP 里程碑。门禁：rumdl 加断链加标题括号加 mdcharlint 全绿 [实证： 2026-09-08 本机四门禁]。

## 经验

- 收窄式重构先立「保留面清单」再删：凡不在清单内的命令、模块、依赖、资产、工具一次划清，避免删一半留半截引用。
- 生成物链路要跟着改：SKILL.md 由 COMMAND_MAP 生成，命令面收窄后必须改源头常量并重跑 `oma init`，只删命令分发会留下旧命令图的生成物 [实证： 本轮初跑 init 再生出旧编排命令图，改 COMMAND_MAP 后才正确]。
