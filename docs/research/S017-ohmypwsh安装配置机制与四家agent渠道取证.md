# S017：ohmypwsh 安装配置机制与四家 agent 渠道取证

- 日期：2026-08-31
- 关联：方案 `P0012`（自适应本机安装部署）；前置 `S014`（检测路径表）、`S015`（hook 注册一手形态）；蓝本仓 `D:\ohmypwsh`（五端环境控制总台，R009 已对其模块管理对齐过一轮）
- 研究法：ohmypwsh 安装面与配置面分两路深读（catalog.psd1 全文自查 + 两路子代理交叉），载荷性断言逐条回源码抽查；oma 侧渠道取证全部本机实测

## 一、为什么研究

用户定调（2026-08-31 四条连发）：oma 要成为**自适应系统，管理本机的 agent 安装部署**——把 claude / codex / grok / kimi 四家在 Windows、Linux、macOS 的安装、配置全部实现，并且同样接管本机 rmux 的三平台安装。ohmypwsh 已有完整的工具安装配置体系（catalog 唯一 pin、EnvRoot 绿色部署、五端 agent 四件套标准），是现成蓝本；边界上 AGENTS 已定「oma 不替代 ohmypwsh 五端总台」，故本研究只吸收**本机自适应安装**的机制，不吸收五端远控与密钥管理。

## 二、ohmypwsh 安装面机制

### catalog.psd1 唯一 pin 源

> 每工具条目 = `Category` + `Deploy = @{win;linux;mac}` + `Win` 块 + `Pos` 块。Deploy 取值：`envroot`（绿色进 EnvRoot）/ `official`（官方目录）/ `installer` / `standard`（POSIX `~/.local/bin`）/ `apt` / `brew`。[实证: D:\ohmypwsh\scripts\catalog.psd1 全文本机阅读]

字段族（7z 条目是全字段样板，catalog.psd1:3-23）：`Asset`（当前资产名）、`AssetPattern`（选资产 regex）、`AssetShaSuffix`（逐资产 `.sha256` 边车）、`Bin`（PATH 注入目录）、`Dir`（安装目录）、`Exe`（二进制相对路径，含 `%` 即官方目录类）、`Extract`（解压分支：zip / targz / copy / single / msi / 7zsfx / 7z-extra / gsudo / rmux / 7z-archive）、`CdnUrl` / `CdnVersionUrl` / `CdnIndexUrl`（非 GitHub 来源）、`Repo`、`Sha256` / `Sha256Mac`、`SumsAsset` / `SumsPattern`（官方统一校验清单，名称支持 `{version}`/`{tag}` 占位）、`Tag` / `TagPrefix` / `Version`。

四家 agent 的 pin 形态（2026-08-31 时点 ohmypwsh 锁定值）：

| 家 | Repo / 渠道 | Tag 形态 | 资产名（Win / Pos） | 校验源 |
| --- | --- | --- | --- | --- |
| claude | anthropics/claude-code | `v2.1.246` | `claude-win32-x64.zip` / `claude-linux-x64.tar.gz`、`claude-darwin-arm64.tar.gz` | release 附 `SHASUMS256.txt` |
| codex | openai/codex | `rust-v0.149.1`（TagPrefix `rust-v`） | `codex-package-x86_64-pc-windows-msvc.tar.gz` / `-unknown-linux-musl.tar.gz`、`-aarch64-apple-darwin.tar.gz` | release 附 `codex-package_SHA256SUMS` |
| grok | x.ai CDN（`https://x.ai/cli/<asset>`；GCS `storage.googleapis.com/grok-build-public-artifacts/cli` 兜底） | `1.0.13`（版本通道 `https://x.ai/cli/stable` 裸文本） | `grok-<ver>-windows-x86_64.exe` / `grok-<ver>-linux-x86_64`（裸单二进制） | 无官方清单，pin 自算 sha256 |
| kimi | MoonshotAI/kimi-code | `@moonshot-ai/kimi-code@0.38.0`（npm 式 tag） | `kimi-code-win32-x64.zip` / `-linux-x64.zip`、`-darwin-arm64.zip` | 逐资产 `.sha256` 边车 |

[实证: catalog.psd1:142-176（claude）、:177-211（codex）、:345-379（grok）、:533-566（kimi）；grok CDN 与 GCS 兜底另证 set-grok.ps1:36-37；`x.ai/cli/stable` 返回 `1.0.13` 为本机 curl 实测]

### ohmyenv 安装管线

命令面 `query / install / deploy / update / pin / status / daily`；下载**不依赖 gh**，全部 api.github.com REST 直连 + 资产直链（403/限流才切 `gh api` 认证通道）。[实证: ohmyenv.ps1 头注；helpers.ps1 Invoke-GitHubApi 分支]

关键语义（oma 直接可抄的）：

1. **先 pin 后 update**：pin 只写 Tag/Version/Asset，版本真变才清 Sha256；sha 在 install 下载校验成功后回填——catalog 永远描述「已验证过的状态」，不描述「想要的版本」。[实证: helpers.ps1 Set-ToolPin 与 Install-ToolVersion 回填段]
2. **哈希优先级**：pin 的 Sha256 最可信；缺失才动态取官方清单（SumsAsset 或 `.sha256` 边车）；多列 checksums 仅参考不覆盖 pin。[实证: helpers.ps1:1082-1084 注释原文]
3. **缓存命中必须先过 sha 再复用**，不一致删缓存重下——「return 会谎报复用」坑有注释点名。[实证: helpers.ps1:799]
4. **装后读版本重试 5 次 × 500ms** 再比对期望版本（杀软/SFX 瞬态延迟）。[实证: helpers.ps1 Install-ToolVersion 尾段]
5. **PATH 注入写注册表 `HKCU\Environment` 并保 `REG_EXPAND_SZ`**（`SetEnvironmentVariable` 恒写 REG_SZ，重写含 `%USERPROFILE%` 的 PATH 会静默损坏）；新目录前置；比较用展开后形式防重复。[实证: helpers.ps1:1354、:1366]
6. **解包定位用 leaf 文件名递归查找**而非含版本号的完整路径——pin 升版路径会漂移。[实证: download-mac-tools.ps1:87-97 注释]
7. **zip/targz 解压后顶层单目录展平**（内层只有一个目录时搬上来）。[实证: Install-ToolVersion zip/targz 分支]
8. **SFX（GUI 子系统 exe）必须 `Start-Process -Wait`** 拿真实退出码，`&` 调用不等待且 `$LASTEXITCODE` 恒空；msiexec 退出码 3010 放行不算失败。[实证: helpers.ps1 7zsfx 与 msi 分支注释]
9. **EnvRoot 可重定位**：参数 > 环境变量 > 平台默认 > 锁定值；锁定值跨平台失效强制平台默认；`Test-SafeUnderRoot` 防路径逃逸。[实证: helpers.ps1:461-475、:939-947]

### agent 的实际安装通道与半截现状

ohmypwsh 对 agent 定调「官方脚本 + 官方目录」（set-agent-upgrade.ps1:11），四家官方脚本：codex `chatgpt.com/codex/install.{sh,ps1}`、claude `claude.ai/install.{sh,ps1}`、kimi `code.kimi.com/kimi-code/install.{sh,ps1}`、grok `x.ai/cli/install.{sh,ps1}`。[实证: set-agent-upgrade.ps1:37-40]

但 Windows 侧实际是绿色部署与半截条目并存：

- claude/codex 走 ohmyenv 绿色部署进 EnvRoot（`claude\claude.exe`、`codex\bin\codex.exe`），无静默参数无安装器；claude 另有「双位置」补丁——本体在 EnvRoot，再按 Length+LastWriteTimeUtc 幂等同步到官方 native 位 `~/.local/bin/claude.exe` 消 `claude doctor` 的 PATH 警告。[实证: set-claude-config.ps1:17-57]
- **kimi Win 条目是残条目**：无 `Dir` 无 `Extract`，ohmyenv 无法绿色部署，实装靠官方 install.ps1。[实证: catalog.psd1:533-551；推断: Install-ToolVersion 拿空 Dir 会 Join-Path 失败——子代理静态分析，未执行验证]
- **grok 的 `Extract='single'` 没有对应分支**：helpers.ps1 解压分支表无 single，落到 default 抛「未知解压类型」。[实证: helpers.ps1:1241 与 catalog.psd1:119 的 single 声明对照]
- **静态元数据有第二源已漂移**：`New-ToolDef` 的 grok 硬编码 1.0.5，合并语义是 def 覆盖 catalog 静态字段——catalog 1.0.13 与 def 1.0.5 双源漂移。这是「唯一 pin 源」设计的自我违背，oma 的反面教材：**catalog 必须是唯一真相，schema 校验在加载期做**（声明的解压类型必须有实现、Dir/Extract/Bin/Exe 一致性）。[实证: helpers.ps1:22-407 New-ToolDef 与 :447-483 合并语义；grok 漂移为子代理静态分析推断]

POSIX 侧组件脚本（`scripts\wsl\tools\<tool>.sh`、mac 收拢脚本）才是完整的 agent 布局事实：claude/kimi 单二进制 `install -m 0755` 到 `~/.local/bin`；codex 官方 standalone 布局 `~/.codex/packages/standalone/releases/<ver>/` + `current` symlink + `~/.local/bin/codex` 链接；grok 二进制落 `~/.grok/downloads/`、`~/.grok/bin/{grok,agent}` 符号链接；rmux 官方 `install.sh --prefix ~/.local` 保 `bin/`+`libexec/` 布局。[实证: 各组件脚本；oma 侧另证——rmux linux tar 流式列表见 `rmux-0.10.0-linux-x86_64/<省略>/bin/rmux` + `libexec/rmux/rmux`，本机 curl+tar -tz]

## 三、ohmypwsh 配置面机制

### 四家配置落点

| 家 | 用户级配置 | 关键键 |
| --- | --- | --- |
| claude | `~/.claude/settings.json` + `~/.claude.json`（onboarding 与信任） | settings.json 顶层 `env`（全字符串值）放模型键；`permissions.defaultMode='bypassPermissions'`；`~/.claude.json` 的 `hasCompletedOnboarding`、`customApiKeyResponses.rejected=[]`、`projects.<正斜杠路径>.hasTrustDialogAccepted` |
| codex | `~/.codex/config.toml` + `models.json` + `hooks.json` | `model`、`model_provider`、`model_catalog_json="models.json"`（**必须相对 CODEX_HOME**，绝对路径在 drvfs 双端互见时必炸一端）、`[model_providers.<id>] base_url/wire_api/env_key`、`[projects.'<绝对路径>'] trust_level="trusted"`、`[features] hooks=true` |
| kimi | `~/.kimi-code/config.toml` | 顶层键必须在第一个 `[table]` 之前：`default_model`、`default_permission_mode`、`extra_skill_dirs`、`telemetry`；hook 走 `[[hooks]]`；信任在 `~/.kimi-code/workspace-trust/<workspace_id>`（内容 `{"root","trustedAt"}`，id 从 workspaces.json 读不硬编码） |
| grok | `~/.grok/config.toml` | `[cli] installer="internal"` + `channel="stable"`；`[marketplace] official_marketplace_auto_installed` 是 **sticky flag 不是开关**——`true` 表已注册过启动 no-op，`false`/缺键反而触发重新注册重建 cache（「漂回」根因） |

[实证: set-claude-config.ps1、set-wsl-agent-config.ps1、set-kimi-config.ps1、set-grok.ps1、set-grok-config.ps1、S004/S019 研究篇；行号见各脚本]

### 合并三流派

1. JSON 逐键 merge（claude settings.json）：保留全部原有顶层键，只 upsert 标准子键 + 删指定遗留键；写前比对字符串相同即跳过，不同先备份 `.bak-<时间戳>`。
2. 标准集内写、标准集外清 + 保留第三方（remote 配置脚本）：env 块先 purge 非标准键再写标准键；hooks/statusLine 只 upsert 自己的项，`-PurgeThirdParty` 才清第三方。**dry-run 默认、`-Apply` 才落盘**。
3. TOML 行级正则合并（codex/kimi/grok）：`(?m)^\s*key\s*=` 探测，有则整行替换无则插入；**段内操作必须按块界定**（`[table]` 到下一表头），跨段匹配会产生重复键即非法 TOML。

[实证: 对应脚本与行号；oma 的 deploy.rs 已在 codex config.toml 项目级实践同族做法]

### 密钥边界

key 载体一律用户级环境变量（Windows 注册表 User 层），config 文件只写引用名（如 `env_key="DEEPSEEK_API_KEY"`）不写值；SOPS+age 加密备份进仓 `.secrets\`。oma 已有对应原则（settle 的「密码类永不自动」），且 AGENTS 边界把五端密钥标准留给 ohmypwsh——**oma 不做密钥管理**，本研究只记录边界。[实证: set-claude-key.ps1、sops-encrypt-agent-key.ps1]

## 四、oma 侧渠道取证

> 2026-08-31 本机 gh api + curl 实测，oma catalog pin 的直接依据。

| 家 | 最新 tag | 发布时间 | oma 可 pin 资产与官方校验和状态 |
| --- | --- | --- | --- |
| claude | `v2.1.251` | 2026-08-28 | 八资产（win32 x64/arm64 zip、linux x64/arm64 加 musl、darwin x64/arm64 tar.gz）+ `SHASUMS256.txt` 全量到手 |
| codex | `rust-v0.151.0` | 2026-08-29 | `codex-package-*` 六资产（win msvc x64/arm64、linux musl x64/arm64、darwin x64/arm64）+ `codex-package_SHA256SUMS` 到手；资产面混有大量伴生件（bwrap、app-server、symbols 等），选资产必须按 `codex-package-` 前缀过滤 |
| kimi | `@moonshot-ai/kimi-code@0.39.1` | 2026-08-28 | 六平台 zip 各带 `.sha256` 边车，六条校验和全到手 |
| grok | 通道 `stable` = `1.0.13` | — | grok-build 仓**无 release 无 tag**，分发纯走 x.ai CDN；无官方校验清单，sha256 只能自算（ohmypwsh pin 的 hash 即其自算结果） |

包内布局实测（流式 tar 列表）[实证: 本机 curl | tar -tz]：

- codex linux 包平铺：`bin/codex` 在归档根，另有 `codex-path/rg`、`codex-resources/`（zsh 等）边料
- claude linux 包根级单文件 `claude`
- rmux linux 包版本前缀目录 `rmux-0.10.0-linux-x86_64/` 含 `bin/` + `libexec/rmux/rmux` + `install.sh`——oma 现有 `find_package_root` 一层扫描正好吃下

### 追记：官方安装脚本逐家实证与渠道反转

> 2026-08-31 当日补证。用户定调「四家都有官方安装脚本，可确定二进制从 CDN、云还是 GitHub 下载，默认 github、CDN 兜底」后逐家抓脚本（本机 curl 对部分域 DNS 间歇超时 / JS 壳，用服务端抓取绕过）。

- **kimi install.sh 全文实证**：CDN 三件套——版本通道 `https://code.kimi.com/kimi-code/latest`（裸文本）、平台清单 `{base}/binaries/<ver>/manifest.json`（`platforms["<os>-<arch>"].filename/checksum`，平台键 `win32-x64` 形态）、二进制 `{base}/binaries/<ver>/<filename>`；装到 `~/.kimi-code/bin/kimi` 并做 legacy Python kimi-cli shim 迁移（首 个改 `kimi-legacy` 保留兜底、其余清重）。**关键发现：CDN 是裸单二进制，与 GitHub 的 zip 制品不同源**（同名基干但 checksum 完全不同）——oma 的资产矩阵必须按渠道绑定，不能 per-agent 一份。[实证: install.sh 全文 + manifest.json 六平台对照]
- **codex install.sh 全文实证**：**纯走 GitHub**——`api.github.com` 取 release 元数据、`github.com/openai/codex/releases/download/rust-v<ver>/` 下载；README 里的 `releases.openai.com/codex` 实测 404 死桶（对象存储未开公开）。布局 `~/.codex/packages/standalone/releases/<ver>-<target>/` + `current` 链接 + `~/.local/bin/codex`；锁用 flock/lockf/mkdir 三级退化。**附带金矿：release JSON 的 `assets[].digest` 自带 `sha256:` 前缀逐资产哈希**（脚本用它做首选校验，SHA256SUMS 文本只是 package 布局的二级来源）——oma update 的取证首选同此。[实证: install.sh 全文 + releases.openai.com 404]
- **claude GCS 桶**：`storage.googleapis.com/claude-code-dist-<uuid>/claude-code-releases/stable` 裸文本通道存在（返回 2.1.236，滞后 GitHub 的 2.1.251），但资产 URL 三种猜测形状全 404、`claude.ai/install.ps1` 本机 DNS 不通——**资产形状未实证不接线**，oma 的 claude 兜底槽留空。[实证: stable 通道响应；推断: 三探 404 说明形状另有其形，需 install.ps1 原文]
- **grok install.sh**：x.ai 主 + GCS `grok-build-public-artifacts` 兜底，与 ohmypwsh set-grok.ps1 的 BasePrimary/BaseFallback 一致——双 CDN 同名资产。[实证: install.sh URL 提取]

## 五、关键结论

1. **oma 的安装子系统是 rmux 安装器的泛化而非新造**：`src/rmux.rs` 已有下载（ureq 直连 + GH_TOKEN 可选）、sha256、zip/targz 解压、manifest、顶层目录发现全套；catalog 已有 per-OS/arch 资产 schema 与「信任锚是本文件不是现场 SUMS」原则。缺的只是多条目、grok CDN 单文件、二进制 leaf 查找、装后版本探针。[推断: 基于 rmux.rs 与 catalog.rs 现状逻辑]
2. **oma catalog 吸收 ohmypwsh 的字段族但做减法**：oma 不需要 msi/7zsfx/brew/apt 分支（agent 与 rmux 全是绿色资产），`kind` 收 zip / tar_gz / single 三值即可；`url` 模板支持 GitHub release 与 CDN 两种；校验和一律进 pin（oma 原则：本文件即信任锚，连官方 SUMS 都只是取证来源）。[推断]
3. **oma 不写用户 PATH、不预写用户级 agent 配置**：oma 自管安装的消费者是 oma 自己（orch spawn 与 agents 探测），把 managed root 挂进 `Probe::extra_dirs` 即闭环，不必像 ohmypwsh 那样为人类 shell 注册 PATH；grok 的 `~/.grok/config.toml` 两键留给首启自写（缺键只多一次 marketplace 注册，无功能损失），oma 装完打印提示即可。这保住 AGENTS 边界「默认不改用户家目录」。[推断: 边界推导；grok sticky flag 语义为实证]
4. **版本口径**：oma pin 本机取证时点最新（claude 2.1.251 / codex 0.151.0 / kimi 0.39.1 / grok 1.0.13）；本机已装的旧版（如 codex 0.149.1）不冲突——oma 自适应「已装则跳过、只补缺」，managed 安装是兜底不是强制版本。[推断: 设计裁决]
5. **ohmypwsh 的三个坑是 oma 的门禁清单**：唯一 pin 源不得有第二静态源（加载期校验拦截）；声明的解压类型必须有实现分支；残条目（有 pin 无布局）必须在加载期报错而不是运行期 Join-Path 失败。[实证: 上文半截现状三条]
6. **平台矩阵覆盖**：claude 与 kimi 六平台全齐、codex 六资产全齐、grok 仅 win x64 与 linux x64（CDN 资产命名如此）；oma catalog 按实际存在的资产 pin，缺失组合在 `asset_for` 报「no pinned asset」即自适应拒绝，不猜。grok mac 待 x.ai 出资产或另立渠道再补。[实证: 校验和矩阵取证；推断: 矩阵策略]
7. **渠道序裁决（追记后定稿）**：github 默认、CDN 兜底；kimi 双渠道制品不同必须按 source 绑资产；codex github 即全渠道（digest 字段是最佳取证面）；claude 兜底槽空置待证；grok 双 CDN。[实证: 上文追记；推断: 空槽裁决]
