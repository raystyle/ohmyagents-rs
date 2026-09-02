# S030-密钥hook安全拦截接管与同类Rust实现

> 2026-09-02。用户定调：oma 未接管 ohmypwsh 的密钥 hook 安全拦截（secret-guard）；gh 搜索同类 Rust 程序取证实现方式。四仓分工下密钥体系主权在 ohmypwsh（R001），oma 接管的是**会话内拦截面**：agent 的工具调用与提示词在出口处的防泄露闸。

## ohmypwsh 原实现语义

[实证: D:\ohmypwsh\scripts\hooks\secret-guard.py 逐行读]

- **信封兼容四 CLI**：Claude Code / Kimi（`hook_event_name` + snake_case）、Codex（snake_case 无事件名，从 payload 推断）、Reasonix（`event` + camelCase）。
- **阻断语义**：PreToolUse / UserPromptSubmit → 命中即 `exit 2` 阻断；PostToolUse → 只观察（Codex 路替换输出）；异常 **fail-open** `exit 0`（防护挂了不挡活）。
- **双层正则**：provider 前缀类大小写敏感（`sk-ant-`、`ghp_`、`AKIA`、`xox`、`eyJ` JWT、PEM 私钥头、mongodb/pg/mysql/redis 带密码 URI）；通用赋值类忽略大小写（`api_key=`、`token=`、`bearer `）；**bare password 只 warn 不阻断**——659 次命中绝大多数是大段 JSON/文档的误报教训（P0 降级）。
- **配套**：SECRET_ENV_NAMES 环境变量名名单；自扫豁免（guard 源码自身与豁免研究文档）；URI 正则字符串拼接构造避免源码自身触发扫描。
- **测试**：14 例冒烟（payload → 期望 exit code），覆盖四 CLI 信封 × 命中/干净。

## 同类 Rust 实现

[实证: gh search + repo 逐个核]

| 项目 | 星 | 形态 | 对 oma 的参考点 |
| --- | --- | --- | --- |
| mongodb/kingfisher | 1220 | 泄露检测引擎：crates 拆 core / rules / scanner；规则**数据驱动**（`data/default` 规则库）；`betterleaks_filter.rs` 兼容 gitleaks 规则；泄漏凭据**活性验证**（blast radius） | 规则与引擎分离（规则可独立演进）；检测后再验活是下一步方向 |
| 0sec-labs/foxguard | 290 | 通用代码安全扫描器（含 secret 扫描）batteries included | 扫描器形态参考 |
| rtk-ai/rtk | - | **LLM agent hook 生命周期层**（Rust）：install/uninstall/SHA-256 完整性校验/审计/信任管理，覆盖 6 家 agent + TOML filter trust；重写逻辑在 registry 层 | 与 oma 域最近：hook 部署 + 过滤信任已是 oma init 地盘（S015/S024），拦截闸挂同一骨架顺理成章 |
| openai/codex（codex-rs/hooks） | - | hook 引擎内部：output_parser、pre_tool_use 事件模型 | oma S015/S024 已用过其源码；hook 事件模型一手参照 |
| MohamedAbdallah-14/awesome-claude-hooks | 3 | hook 集合含 security guards（非 Rust） | 语义参考非实现参考 |

共同模式 [推断: 由上表归纳]：**规则表数据驱动 + 引擎薄壳 + fail-open + 分级（阻断/warn）**——没有谁把正则硬编码在拦截路径里。

## oma 接管落点

- **零新依赖**：`regex` 与 `serde_json` 已在依赖面（R005 口径：组合不自写）。
- **通道已就绪**：`oma hook`（src\hook.rs）已解析四家信封写状态——拦截闸是同一条入口的第二职责：PreToolUse / UserPromptSubmit 命中密钥 → 状态照写 + `exit 2` 带原因；PostToolUse → warn 记状态不阻断。
- **规则表**：独立 `src/secretguard.rs` 模块，模式表静态常量（对齐 secret-guard.py 清单与降级教训：bare password warn_only）；SECRET_ENV_NAMES 同表；自扫豁免（oma 自身源码与本文档）。
- **注册面**：hook 注册已带 `--agent <名>` 参数与 bare 形态（P0027），guard 语义不需要新注册——同一 `oma hook` 调用内分流。
- **测试**：ohmypwsh 14 例冒烟语料直接移植为黄金用例（独立 oracle：期望值来自其测试契约非 oma 实现镜像，R004）。
- **边界**：age/SOPS 金库与密钥部署（整块加密、备份脚本）**不接管**——那是 ohmypwsh 主权域（R001 四仓分工）；oma 只做会话出口拦截。

## 待办

- `src/secretguard.rs` 实现 + hook.rs 分流接线 + 14 例黄金测试（独立切片）。
- 规则活性验证（kingfisher 式 blast radius）不进首期。
