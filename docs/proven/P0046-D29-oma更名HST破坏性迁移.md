# P0046：D29 oma 更名 HST / hst / v0.6.0 破坏性迁移

> 2026-09-12。用户定夺全文（PRD D29 第 0 轮），先回迁移计划（七阶段）再执行，当日一次做完。职责不变：hook 落盘、statusline、只读 trace、doctor、yolo；不编排、不装二进制（归 ark）。

## 方案七阶段与落点

1. **crate 与标识**：Cargo.toml `hst-cli`（bin `hst` 加 lib `hst` 加过渡 stub bin `oma`）；main.rs `hst::`；fmtio kv 前缀 `hst:`；0.6.0。
2. **命令面**：`hst hook init|status|verify`（hook 面 init / stdin 状态入口 / agents verify 的 hook 层过滤）；`hst statusline` 一级化，`agents statusline` 隐藏别名一个小版本；`agents [list|verify]`；其余原面。
3. **路径与 env**：`pathutil::DIR = ".hst"`，legacy 链 `.oma` 加 `.ohmyagents` 同卷 rename（HST_ROOT 覆盖时不自动迁）；`hst init` heal 即重部署（托管注册收敛到 `~/.hst/hooks/hst-state.*`，is_ours 保留 oma 历史形态识别）；doctor 加 `hooks.migrate` warn（ours 注册仍带 oma-state / .oma 形态）；`HST_MIRROR/HST_GATEWAY_URL/HST_GATEWAY_KEY` 新读加旧 `OMA_*` 一个版本 stderr 提示；测试缝 `HST_ROOT/HST_USER_HOME/HST_TRACE_HOME`。
4. **资产与自更新**：`hst-<target>` 资产；release 同挂 `oma-<target>` stub（警告后转调 hst，姊妹 hst 优先）；mirror job `hst/` 主段 sync 加 `oma/` 兼容段推 stub（对齐 ark 策略，ohmycloud 批切对齐）；update.rs 资产名与 UA 与 DEFAULT_REPO（raystyle/hst-rs）。
5. **文档**：README 全换（HST 全称首句、`.hst\bin`、Hipo/hst PATH 共存注、stub 说明）、AGENTS 定位与路由、R002 D29 更名映射注记、R011/R004/INDEX 命令名 sweep、CHANGELOG 0.6.0、本件、diary。
6. **测试**：迁移三用例（旧无新 rename / 两者都有用新不动旧 / 只新根）钉 pathutil；guard 白名单 fake hst 三态；bin 名 `hst` 全换；SKILL 渲染断言 hst。
7. **发版**：repo 改名 hst-rs、tag v0.6.0、CI 双资产、digest、herdr 知会。

## 关键机理记档

- **heal 即重部署**：托管注册的收敛逻辑（is_ours 陈旧判据 = 不等于本次命令）天然覆盖跨名迁移：旧 oma-state 形态被识别为 ours 陈旧态，重部署收敛到 hst-state 新形态；无需独立改写器。is_ours 同时保留 `hst`/`hst-` 与 `oma`/`oma-` 形态匹配。
- **guard 白名单前缀随迁**：`oma secretguard:` 改 `hst secretguard:`（main.rs 输出侧与 shim 白名单 grep 侧同换）；旧 shim 调 stub 的破损路径（argv 形态不符）落在白名单外，fail-open 放行不炸会话。
- **双根并存实况**（活体验证）：迁移窗口内旧会话 shim 重建 `.oma/state` 与新根 `.hst` 并存，data_dir 语义「新在用新不动旧」保证幂等；doctor 双根状态面都可读。
- **shim 实体缺失 heal 覆盖**（ome 会话知识转递建议采纳核验）：doctor `hooks.form` 的 shim-dead 探针探实体文件在位性（F8 加固）；`hst init` 与 `hst hook init` 必经 deploy_shims 重落实体，注册在位而实体缺失的场景一次 init 即愈。
- **oma stub**：独立 bin `src/bin/oma-stub.rs`，警告更名后按「同目录姊妹 hst > PATH hst」转调，都缺则打安装指路 exit 1；release 与镜像 oma/ 兼容段挂 stub，让旧 `oma self update` 升到 stub 被引导换名而不是断链 404。

## 验收：用户八条清单

1. `hst --version` = hst 0.6.0 新名 过；2. `hst doctor` 无指向 `~/.oma` 的托管 hook（heal 后 `hooks.migrate` 无 warn）过 本机活体；3. `hst hook status` shim 在 `~/.hst/hooks` 过（hst-state 三件）；4. `hst statusline --example` 写 `~/.hst/statusline.toml` 语义（example 打印路径注）过；5. `hst trace sessions` 与旧一致（trace 零改动只换名）过；6. `HST_MIRROR` 新变量生效（旧 OMA_MIRROR 读并提示）过 单测；7. 旧用户装 0.6 跑一次 `hst init` 后 doctor 无 block 过 本机 dogfood；8. 兼容别名 `agents statusline` 隐藏转发 过。

## 经验

- 更名迁移的最大风险面是「写死路径的注册串」而非二进制本身：heal 复用重部署的陈旧收敛，比独立改写器少一套状态机。
- 破坏性更名的兼容预算花在两处最值：旧命令 stub（不断链）加旧 env 读提示（不静默失效），其余面一次切净。
