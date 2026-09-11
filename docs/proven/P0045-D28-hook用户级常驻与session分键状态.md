# P0045：D28 hook 用户级常驻与 session 分键状态

> 2026-09-11。用户裁定「hook 应用户全局」+ 随修发小版本（v0.5.4）。需求链：PRD D28；研究底座：S015（四家注册面一手矩阵）、S025（状态栏矩阵与 session 标识）、S028 追记无需（发版面不变）。

## 背景与根因

现象：不同项目状态栏 `claude:unknown`。用户诊断四点 [实证: 用户 2026-09-11 本机]：

1. 状态栏命令在用户级（`~/.claude/settings.json` 指 `~/.oma/statusline/`）。
2. 状态采集 hook 与 shim 落项目级（各项目 `.oma/hooks/oma-state.cmd`，TARGET 项目相对 `.oma/state/`），未 init 项目零数据。
3. codex 的 hook 用户级（`~/.codex/hooks.json`）而 claude 项目级，两 agent 不一致。
4. 本机临时治理（hook 与 shim 提用户级全局单文件）shim 写态通，但状态栏仍 unknown：状态栏按项目路径找状态文件的映射协议没跟上。

根因总结：采集面与消费面跨用户级/项目级两半分裂；且状态按 agent 单键，herdr 多 claude 会话并发互踩。

## 方案

1. **shim 用户级加 session 分键**（`src\shim.rs`）：shim 常驻 `~/.oma/hooks/`（deploy_shims 收 oma 根）；默认双写 `~/.oma/state/<agent>.json`（agent 最新）加 `<agent>-<session>.json`（session 键）；session 三源（payload `session_id` / `sessionId` / grok 的 `GROK_SESSION_ID` env，S015 runner 注入实证）；`OHMYAGENTS_STATE_FILE` 覆盖互斥单写（verify 与测试）；SessionEnd 删本 session 键（GC）。
2. **注册四家迁用户级**（`src\deploy.rs`）：claude `~/.claude/settings.json`（settings 家族用户层全项目生效，S015）、codex `~/.codex/hooks.json` 加 `~/.codex/config.toml`（features.hooks 加 trusted_hash 预种，key source = 用户 config 路径）、grok `~/.grok/hooks/ohmyagents-state.json`（global 层）、kimi `~/.kimi-code/config.toml` `[[hooks]]`（kimi 仅用户级，S015；M058 全局触发面转正为产品面）。命令形态沿用 M059 无引号正斜杠（路径含空格 warn 扩展到家目录）、M048 grok 单路径、M058 kimi 裸命令行；陈旧判据覆盖 D27 项目级 shim 路径（用户手治的同型注册自动收敛）。
3. **状态栏读序**（`src\statusline.rs` SEG_OMA）：env 覆盖互斥，次用户级 session 键（`session_id`/`sessionId`），次用户级 agent 最新，末项目级旧协议；候选序内会话闸不符**续找**（旧实现闸后 unknown 断头）。grok 状态栏 payload 无 session 标识（S025），靠 agent 最新键消费。
4. **oma hook 同协议**（`src\hook.rs`）：缺省双写用户级键对，SessionEnd GC，写侧顺带清扫同 agent 超 7 天陈旧键（崩溃残留）；旧项目 cwd/.git 回退删除（读侧项目旧协议兼容保留在读序末位）。
5. **init 迁移退役**（`src\deploy.rs` retire）：项目级 ours 注册摘除（外来保留，只剩 ours 的文件整删），`.oma/hooks/`（含 `.ohmyagents` 旧名）oma 生成 shim 删除（带生成标记才动）；skill 与 AGENTS/CLAUDE.md 仍项目级。yolo 面不动（D25：项目级）。
6. **doctor / verify 跟随**：doctor hooks.form 四家全量读用户级文件，项目 ours 残留打 `hooks.retired` warn；codex trust.hooks 迁用户 store；状态面双层（用户级 blocked 恒 warn 不归因本项目，项目级旧文件 blocked 仍 block）。verify 判据 env 隔离（子进程带 `OHMYAGENTS_STATE_FILE` 指进临时目录，回落扫用户级状态目录本轮窗口新文件），注册走真实用户级面（byte 备份五件、deploy、Drop 还原；M058 kimi guard 泛化）。

测试隔离缝：`OMA_USER_HOME`（四家用户配置根）与 `OMA_HOME`（oma 自管根）双 env 重定向，init/doctor/verify/statusline/hook 全链认；进程级 env 测试共享 `pathutil::ENV_LOCK`。

## 过程

- 测试迁移主收成：146 单测加 23 集成全绿（Windows 本机）；clippy 对齐基线。集成 init 测试改 `OMA_USER_HOME`/`OMA_HOME` 隔离后不再碰真实家目录。
- 踩坑两笔（当轮内修）：doctor 测试各模块各持 env 锁锁不住彼此（OMA_HOME/OMA_USER_HOME 竞态翻车），升共享锁进 pathutil；pwsh 行为测试未隔离子进程 HOME，真实 `~/.oma/state` 活会话状态污染判据（用户手治的 working 态读进 unknown 断言）。
- live verify 四家真机全绿（本机 Windows：用户级注册 byte 备份 deploy 还原全链过）。

## 验收

- 单测加集成全绿；`rumdl check .` 加 `.tools` 三件 md 扫描绿；`oma init` 对 v0.5.3 形项目一次收敛（注册迁移、shim 退役、外来保留）单测加集成双钉。
- v0.5.4 发版，资产 digest 交对端验锚（herdr codex review 前置）。

## 追记：codex review 九条与第 2 轮扩面

herdr codex 独立 review（对齐 v0.5.3 范式，报告 `.oma/review-v054.md`）九条主张：

- **F1（高，阻塞，已根修）**：codex 信任预种键源错位。oma 一直用 config.toml 路径作 `[hooks.state]` 键前缀，codex 的键源是**定义该 hook 的 hooks.json 路径**（discovery.rs 的 load_hooks_json 返回 source_path；本机 config.toml 里 codex 自写键即 `hooks.json:` 前缀一手实证）。修：`codex_trust_entries` 键源改 hooks.json 路径；活体验证 = 本机 codex 0.149.1 与 0.154.0 双版本 `codex exec` 无 bypass 均触发用户级 hook 并落盘 `~/.oma/state/codex.json`。P0010 期「hash 复现未全中」旧账由此闭案（记 M061）。旧 `config.toml:` 前缀键留作惰性残留（codex 自写同前缀键属用户合法信任，不可归因清理）。
- **F2（中，已修）**：doctor codex trust.hooks 判据硬化为逐键对账（期望键源 = hooks.json、期望哈希现算；「有任一 trusted_hash 即 ok」会假绿），oracle 用真 deploy 行为（种真断 Ok、篡改断 Block）。
- **F3（中，已修）**：项目根即家目录时 init 不再自删用户级 shim（oma 根短路守卫）。
- **F4（中，已修）**：用户级 codex config.toml `[hooks]` 非 state 定义键只清**全 ours** 的（外来定义保留，用户级主配置不做静默删除）。
- **F5（低，已修）**：ENV_LOCK 单一权威归 `testenv`（pathutil 重导出），跨模块 env 测试真互斥。
- **F6（低，已修）**：verify 的真实家直取集中 `verify_real_home` 并记档（live 面刻意不认隔离缝；「全链认缝」口径修准为 init/doctor/statusline/hook 认缝、verify 用户级面直取真实家）。
- **F7（低，已修）**：UserHooksGuard 先构造后 deploy，失败路径也随 Drop byte 还原。
- **F8（低，已修）**：doctor kimi hooks.form 补 shim-dead 在位探针（对齐其它家辨形）。
- **F9（低，已修）**：cmd findstr 回落形态的 Notification blocked 判定锚定 notification 行（此前整包 findstr，任意位置出现 permission 即误判）。

**第 2 轮扩面（用户同日两令）**：yolo 与非阻塞键全量用户级（四家用户配置；项目级旧键 init 等值退役；doctor 判据随迁；覆盖 D25「oma --yolo 面向项目级」口径，托管端协调收敛为同文件幂等合并共识）。历史脉络（用户点名 git 历史佐证）：POC 期 93ac1f9「四家 hook 项目级部署幂等不改家目录」是当时的保守安全属性，此后用户级需求逐面翻正：状态栏 P0027（天然用户级）、kimi M058（只有全局）、D22 skill（用户级）、D28 hook（本件）、D28r2 yolo。结论：oma 配置面全量用户级，项目面只余 skill 与 AGENTS/CLAUDE.md（项目内语义）。

**乱码修（用户同日报修）**：中文目录在状态栏显示「缁跨洘」形 GBK 误解码。根因 = 状态栏脚本只钉了输出侧 UTF-8（S024），stdin 输入侧 `[Console]::In` 随控制台码页（CP936）解码重定向输入。修：stdin 字节级读取（`OpenStandardInput` 加 `MemoryStream`）显式 UTF-8 解码；行为测试喂中文 cwd JSON 断言原样输出。

**第 3 轮（用户追裁「yolo 设置命令要分用户级和项目级」）**：`--yolo` 用户级、`--project-yolo` 项目级（互斥，clap conflicts_with 退出 2）；项目级旧口径（claude 项目双件、codex 项目 config 加项目信任预种、kimi 项目 config）收编为显式旗标；doctor yolo 判据双级接受，项目键在场时按 agent 分层规则遮蔽用户键（detail 标层级）。同批清完 codex review R2 六条低阶残留（R2-1 键标签尾部取、R2-2 findstr 键值段提取与 delims 补空格、R2-3 死绑定、R2-4 R002/R004 口径、R2-5 CLI 帮助、R2-6 四条回归钉 F3/F4/F8/F9）。

**第 4 令（用户「项目级和用户级冲突要可用诊断和告警出来形成 CTA」）**：doctor 三家 yolo 判据双级并读，双级并存且值不同打 warn 加对齐 CTA（`oma init --project-yolo` 对齐或撤一面；项目遮蔽用户是 agent 分层事实，报告项目文件）；同值双级不告警。冲突用例测试钉（三家 warn 加 CTA 在场、对齐后翻 ok）。

## 经验

- 采集面与消费面必须同层：状态栏（用户级）配项目级采集是慢性断裂，任何「半边迁移」都会以 unknown 形式显影（用户诊断第 4 点即此）。
- 用户级注册让「未 init 项目」从根上消失，同时消灭跨环境共享项目目录的双侧注册协调问题（各 OS 家目录自管）。
- session 分键是多会话并发的正解，但无 session 标识的消费面（grok 状态栏）必须留 agent 最新键通道，双写是两全。
