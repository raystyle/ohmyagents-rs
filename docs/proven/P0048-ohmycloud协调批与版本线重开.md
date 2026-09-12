# P0048：D31 至 D35 ohmycloud 协调批 v1.0.0

> 2026-09-13。ohmycloud WSL 总台外部协调三件（域名清扫、旗标连字符化、yolo 分级）开工，日内并入 D34 版本线重开与 D35 README 与仓库描述两裁；herdr codex 三轮独立评审达成一致后发 v1.0.0（HST 线开山版）。

## 方案与落点

1. **D31 私有网关域名清扫**：四处明文摘除（`src\diagnose.rs` 模块注释、README 活性诊断节、R002 2.5 节、P0041 背景段），统一表述「api 缓存回归测试端点（配置注入）」；端点解析本就是 env 覆盖加 agent 侧配置三源注入（`HST_GATEWAY_URL` > claude settings env > codex config），无硬编码，逻辑不动 [实证： 全仓 rg 复扫归零]。
2. **D32 旗标连字符化**：`#[arg(long = "pre-trust", alias = "pretrust")]`，canonical 拼写 `--pre-trust`，旧拼写隐藏别名（1.1.0 兼容窗与 OMA_* env 同批清）；kv 标记 `init.pretrust.*` 逐字不动（机器面冻结）；全仓 md 28 处批量更正含历史档案（旧拼写仍可用，历史命令逐字可复跑无失真）。
3. **D33 yolo 分级**：`YoloLevel`（full/partial/off，clap ValueEnum）加取值式旗标 `--yolo[=<级别>]` / `--project-yolo[=<级别>]`（`num_args = 0..=1` 加 `default_missing_value = "full"`，裸旗标兼容、两级互斥保留）。写入矩阵 full = 现行键组；partial = claude `acceptEdits` 加 skip、codex `workspace-write`/`on-request`、kimi `auto`、grok `auto`；off = ours 等值摘除（新增 `retire_user_yolo_with` 用户级退役面；`retire_project_yolo` ours 值集扩两代）。doctor 判据分级接受（partial 不误报 block），双级冲突 warn CTA 带 `=<level>`。marker 只增 `init.yolo.level` 与 `init.retired`。
4. **D34 版本线重开**：Cargo 与 CHANGELOG 版本线重开，v1.0.0 起算（与 ark 1.0.0 同语义），v0.6.x 及 oma 0.x 各节转更名过渡期记录；活文档 v0.7 兼容窗口径改 1.1.0。
5. **D35 README 与仓库描述**：README 精简为安装（三平台加镜像直链加源码）与使用（部署与信任、诊断验收、活性诊断、状态栏、trace）；GitHub 描述一句话（`gh repo edit raystyle/hst_rs --description`）。

## 评审闸门（herdr codex 三轮）

- 首轮（区间 349233a..eaaae84）：D31/D32/D34 零异议；D33 F1 = full 降 partial 时 ours 落的 `enableAllProjectMcpServers` 不摘，与「MCP 审批保留确认」语义矛盾。裁 (a)：partial 分支按 ours 等值摘除（两级同纪律），补降级回归测试；F2 测试注释版本号、F3 R001 命名与口径统一随批；根 SKILL.md 为 D15 编排遗物删除。
- 二轮（eaaae84..38b1e35 复核加 README）：F1 修闭环；新 A = `enabledMcpjsonServers` 名单里 hst 落值仍在（doctor 视非空名单为已批准）。裁改口径：名单混有 agent 原生用户审批（Claude Code 交互确认也写此键），无法归因落写者，按用户自设值保留纪律一律不动，彻底恢复 MCP 确认需手动清名单；pretrust 组合口径 = 确定性后写者赢（同批或重跑 `--pre-trust` 会再开 MCP 直通）。B trace 分页表述（sessions 无 `--offset`）、C 示例改可直接复跑形、D canonical 仓名 `hst_rs`（hst-rs 是改名重定向别名；README 四链、Cargo repository、DEFAULT_REPO、stub 指路、main help、R001/R002 别名注）、E clap 帮助 partial 注释补边界。
- 三轮终局（38b1e35..ed94f94）：五处逐项核实（含四个 release 直链与镜像平铺链实测 200）、门禁复扫全绿、两处纯装饰残留（R001 四仓表行拼写、update.rs 注释）明示不阻塞 tag；达成一致。

## 关键机理记档

- **grok 配置层取值证据链**：`[ui] permission_mode` canonical 值集 = always-approve|auto|ask（`parse_permission_mode_canonical` 未知串回落 Ask 安全向；旧键 `approval_mode`/`yolo` 另有窄解析），CLI `--permission-mode` 是另一套 Claude 同构六值且 0.2.14 起才正确覆盖 config；两层不能混用同一值集 [实证: grok-build permissions.rs 源码 2026-09-13 加本机 --help]。
- **ours 等值摘除的归因边界**：布尔键（enableAll、skip）ours 判定可靠（true 即 ours 落值）；名单键（enabledMcpjsonServers）混 agent 原生用户审批，无法归因，一律保留。摘除策略按键型分流，不是一刀切 [经验： F1 两轮裁定的收敛点]。
- **评审闸门工作法**：每轮把裁定口径写回文档（源码注释加 R002/R007）再交复核，三轮收敛（首轮 1 中 2 轻、二轮 1 中 4 轻、三轮零新发现）；只改代码不落口径会引来同题反复。

## 验收

1. 160 单测（新增 yolo 分级三件与降级回归、pretrust 双拼写）加 27 集成全绿；fmt 加 clippy（仅存量告警）加 rumdl 加 .tools 三扫描绿。
2. 本仓 dogfood：init 裸跑（`init.yolo.level=full` 标记）四处 SKILL 再生；`hst skill --write` 用户级技能刷新；`hst --version` = hst 1.0.0。
3. 发版 v1.0.0：tag 打在评审终批 `ed94f94`；CI 绿；十二资产（hst 本体六件加 oma stub 六件）加 sha256 边车齐；镜像 hst/stable 与 oma/stable 段到货核验（资产与边车 HEAD 200 加 digest 对账）；herdr 知会 ohmycloud（版本加资产 sha256 全量）。
4. D31 四处与 D35 README 均无私有域名；GitHub 仓库描述一句话已生效。

## 观察面

- 旧 OMA_* env 读取、`--pretrust` 别名、oma stub 资产、旧路径转发 shim 的兼容窗统一到 1.1.0（原 v0.7 口径随 D34 改号）。
- D36 状态栏 HUD 簇已登记（隐 agent 态段、MCP 与 tools 计数加 context 构成比例、多行支持、流行 HUD 研究先行），研究落 `docs\research\` 后走 G003 五步，不随本批。

## 经验

- 降级路径要独立测试点：fresh-only 断言盖不住 full 到 partial 的键残留（F1 由评审抓出）；分级面每条迁移路径（升、降、关）各至少一测。
- 版本号与仓库名这类全局串，改一处要全仓 rg 复扫三类面：发布直链、代码常量（DEFAULT_REPO、repository）、文档口径（canonical 加别名注）。
- 外部协调批里「安全清扫」与「行为功能」分开提交分开验收：前者 grep 归零即闭环，后者要评审闸门。
