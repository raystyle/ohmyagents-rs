# PLAN：当前目标实施计划

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
> 排队：D13 mac `--version` 实收（CI 资产已出，可走 OMA_MIRROR 镜像通道）。

## 当前目标：D18 状态栏用户级定制

> PRD D18 第 2 轮三裁（2026-09-09）：粒度三层全要（段落开关加整脚本替换加模板变量）、缺省键回落内嵌、codex 一并支持（内置项 ID 子集）。

### 总体架构

- **生成时烘焙**（不是运行时读配置）：`STATUSLINE_PS1` 唯一权威仍在 oma 二进制；`oma agents statusline` 每次运行先读 `~/.oma/statusline.toml`，按配置拼装定制版脚本再落盘。依据：PRD D18 登记前提「定制能力只能在 oma 生成时做」；现状 deploy 幂等整文件重释放。
- **合并语义（键级）**：用户配置只覆盖写下的键，没写的键用内嵌默认；`segments` 段落清单键写下即全量（顺序加显隐，合并两个顺序无意义）。坏文件（解析失败）硬错退出 1，缺键宽容回落。
- **配置落位**：`~/.oma/statusline.toml`（用户级，D14 根）；`--example` 打印带注释全量示例（对齐 `oma agents providers --example` 先例）。

### 切片 1：重构拆段

> 行为等价改造：默认段序拼装产物与拆段前脚本 mock 直跑输出逐字节一致。

- `src\statusline.rs` 的 `STATUSLINE_PS1` 单体 const 拆为：`PS1_HEAD`（param / UTF-8 / JSON 解析 / `$nerd` / Seg 与 FmtTok / FmtDur / `$parts`）、`PS1_COMMON`（`$dir` 与 `$root` 发现、projKind 探测从包版本段上提，工具链段与包版本段共享 `$projKind`）、14 个段块 const（shell / dir / oma / model / context / duration / git / package / python / rust / node / zig / go / cpp）、`PS1_TAIL`（join 输出）。
- 段内私有依赖上提或内收：进程祖先链（shell 段专用）收进 shell 块；`$projKind` 探测进 COMMON（隐藏 package 段时工具链段仍需判型）。
- 拼装函数按段序 concat；默认段序等于现脚本顺序。等价判据：`oma agents verify` mock 直跑（D17 判据面）输出与改造前一致；既有 const 内容断言测试改为对拼装产物断言。
- 依据：R004（黄金文件与回归）；P0038（verify 判据面）。

### 切片 2：段落开关

- `statusline.toml` 顶层 `segments = [...]` 键：段 id 数组即全量序（没列的段不显示）；键缺省回落内嵌默认 14 段全量序。
- 段 id 即切片 1 的块名（shell / dir / oma / model / context / duration / git / package / python / rust / node / zig / go / cpp）；未知 id 硬错（拼装无法命中段块）。
- toml 解析复用现有 `toml = "0.8"` 依赖（R005：不引新 crate）。

### 切片 3：模板变量与图标映射

- `[template]` 表：每段一条格式串（如 `oma = "{agent}:{state}"`）；ps1 内加占位替换助手（hashtable 逐 `{key}` 替换，不引模板库）。各段变量集：dir `{path}`；shell `{icon}` `{name}`；oma `{agent}` `{state}`；model `{model}`；context `{pct}` `{used}` `{window}`；duration `{duration}`；git `{branch}` `{flags}`；package 与七工具链段 `{icon}` `{version}`。
- `[icons]` 表：图标键映射（oma / model / context / duration / package / python / rust / node / ts / zig / go / cpp；shell 双图标 pwsh 与 generic）；Grok ASCII 路径不受图标键影响（M046：grok 无 Nerd 字形）。
- 两表键级缺省回落内嵌默认；grok 部署脚本与其余三家同源拼装（`$nerd` 分支维持）。

### 切片 4：整脚本替换

- `oma agents statusline [名] --script <路径>`：用户脚本拷到部署位（`script_path` 同名落盘，agent 配置命令行不动，claude / kimi / grok 三家调用约定不变）；写边车标记 `<script>.custom` 记源路径，此后无 `--script` 的重跑跳过内嵌覆盖（打印 custom 状态）。
- `--builtin` 还原内嵌（删标记重释放）；`--script` 对 codex 无脚本面（M045），带 codex 名时只应用 `[codex] items`。
- 自备脚本调用契约进 R002：首参 agent 名、stdin 喂 agent JSON、stdout 单行状态栏、300ms 约束（kimi S025）。`oma agents verify` 直跑部署位脚本，天然覆盖自备脚本回归。

### 切片 5：codex 内置项子集

- `statusline.toml` `[codex] items = [...]`：用户清单替换 `CODEX_STATUS_LINE_ITEMS` 渲染 `[tui].status_line`；键缺省回落内嵌推荐八项（S016）。codex 对未知 id 静默跳过（源码实证），oma 侧不校验清单合法性、原样透传。

### 收尾

- 测试：拼装器单测（隐藏 / 重排 / 未知 id 硬错 / 缺省回落）、toml 解析合并单测、`--script` 与 `--builtin` 集成测试（标记跳过与还原）、codex items 写入集成；期望值来自配置文件与内嵌常量（R004 独立来源）。
- 文档：R002 命令面（`--script` / `--builtin` / `--example` 与调用契约）、COMMAND_MAP 加行、重跑 `oma init` 再生 SKILL、R011 输出面（新 kv 行）、AGENTS 意图路由行更新、INDEX 代码表、CHANGELOG、diary。
