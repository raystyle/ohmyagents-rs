# 自适应本机安装部署：rmux 与四家 agent 接管

- 状态：已完成（2026-08-31：Windows 四家装机全绿、update 写回 pin 闭环、54 测过；Linux/mac 资产与代码路径就绪，运行验收待环境切换）
- 日期：2026-08-31
- 关联：研究 `S017`（ohmypwsh 安装配置机制与四家渠道取证）、`S014`（检测路径表）；前置 `P0003`（rmux 安装信任锚）、`P0005`（部件 POC）；用户定调 2026-08-31：「研究学习 ohmypwsh，我们的命令把 windows、linux 和 mac 的这几个 agent 的安装、配置也全部实现」「自适应系统，管理本机的 agent 安装部署」「oma 也参考 ohmypwsh 接管自适应本机系统本地 rmux 的 linux、mac 和 windows 的安装」「现在保证 windows 的，linux 和 mac 等我们切换环境接管开发」

## 背景与问题

oma 当前能**检测**四家 agent（`oma agents`：PATH / 环境变量 / 默认目录）但**不能安装**——缺哪路只能人工去装。ohmypwsh 已有完整安装体系（catalog 唯一 pin、绿色部署、五端 agent 四件套标准），但其 Windows 侧 agent 绿色安装是半截（kimi 残条目、grok 缺 single 分支、New-ToolDef 第二源漂移）。用户定调 oma 成为**自适应系统**：本机缺什么就按 catalog pin 补什么，rmux 与四家 agent、三平台同一套机制；Windows 现在保证，Linux/mac 保 catalog 与代码路径就绪，切换环境再接管验收。

## 目标与非目标

- 目标：
  - catalog 泛化：`catalog/agents.toml` 四家 pin（tag / repo / url 模板 / per-OS+arch 资产 + sha256 / 布局线索），沿用「信任锚是本文件」原则；加载期 schema 校验（S017 反面教材三条全拦）
  - 安装子系统：泛化 `rmux.rs` 的下载-校验-解压-落盘-验证管线为 `install.rs`，支持 GitHub release 与 CDN（grok）两种来源、zip / tar_gz / single 三种形态、leaf 名递归找二进制、装后 `--version` 探针（重试容忍杀软延迟）
  - `oma agents install [names...]`：无参装全部缺失、已装（任何来源）跳过并报告、`--force` 重装、`--root PATH` 自定义安装根（缺省 `%LOCALAPPDATA%\ohmyagents`）
  - 探测闭环：oma 自管根挂进 `agents::Probe` 的 extra_dirs——oma 装的 oma 自己找得到，不写用户 PATH 不动家目录
  - rmux 三平台口径：现有 `install_pinned` 本就 os 中立，linux/mac 布局（`bin/`+`libexec/`）已按官方 tar 实测核对（S017 第四节），运行面验收留待环境切换
- 非目标：
  - 不做密钥与模型用户级配置（ohmypwsh 五端总台职责；AGENTS 边界原文）；grok `~/.grok/config.toml` 不预写，首启自注册
  - 不写用户 PATH、不改家目录 hook 注册（oma 自管安装的消费者是 oma 自己）
  - Linux / mac 的实测验收不在本期（环境切换后接管）；brew / apt / msi / SFX 分支不做（agent 与 rmux 全是绿色资产）
  - 不做版本强一致：oma 不因已装版本≠pin 而重装（rmux 除外，它保持现有 pin 强一致的 `oma check` 语义）

## 方案

### catalog/agents.toml 形态

```text
[[agents]] 逐家一块：
name（claude|codex|grok|kimi） / repo / tag / version / url_kind（github|cdn）
[[agents.assets]] per OS+arch：name / sha256 / kind（zip|tar_gz|single）
binary = 解包后二进制名（claude|codex|grok|kimi[.exe]）——leaf 名递归查找的靶
```

pin 值取 S017 第四节本机取证：claude v2.1.251、codex rust-v0.151.0、kimi @moonshot-ai/kimi-code@0.39.1、grok 1.0.13（CDN）。

### 安装管线

> 落 `src\install.rs`。

resolve(os,arch) → download（GitHub release 直链或 CDN 模板；GH_TOKEN 可选，复用现有 ureq 通道）→ sha256 对照 pin（不符即删即败）→ extract（zip / tar_gz / single 直拷，unix chmod +x）→ find binary（leaf 名递归查找，深度有界）→ 落 `root/agents/<name>/<version>/`（临时目录原子切换）→ manifest（tag/version/asset/archive_sha256/binary_sha256）→ `--version` 探针重试 5×500ms。

### 自适应语义

- `oma agents install`：对每家先跑现有探测（env / PATH / 默认目录 / oma 自管根）——**已装即跳过**（打印 source），只对 missing 走安装
- `oma agents install grok --force`：指定重装
- 探测集成：`Probe::from_env` 的 extra_dirs 追加 oma 自管根下各 agent 的 `bin` 目录集合（现排名在 PATH 之后，正合「用户已装的优先、oma 兜底」）

### rmux 接管

`oma check` 现有逻辑不动（已三平台 pin）；把 `install.rs` 的共享件（下载、校验、解压、leaf 查找）从 rmux.rs 抽出共用，rmux 保持专属布局函数。linux/mac 运行面（daemon 起 socket、WMI 换 systemd/user launch）留待环境切换。

## 实施步骤

1. catalog/agents.toml + catalog.rs 多条目解析与加载期校验（单测：四家 windows 资产齐、sha 长 64、grok 是 cdn、unknown kind 拒绝）
2. src/install.rs 管线 + main.rs `oma agents install` 子命令
3. agents.rs 探测集成（oma 自管根）+ `oma agents` 输出标注 source=oma
4. Windows 实测验收（见下）
5. 文档回填（R002 命令手册、S014 检测表补 oma 根、INDEX/TODO/diary）与提交

## 风险与回滚

- 渠道漂移：GitHub release 资产更名 / grok CDN 改版——catalog pin 是快照，漂移时 `asset_for` 报错而不是猜；更新 pin 走取证流程（S017 第四节方法）
- 杀软扫描锁文件：装后版本探针重试 + 明确报错路径
- claude win32 zip 布局与 linux tar 不同（zip 内层结构未逐一开箱）——leaf 名递归查找兜底，验收即实测
- 回滚：`install` 是新增子命令与新增 catalog 文件，现有命令零改动；`--root` 指向临时目录可整目录删除

## 验收标准

- `oma agents install` 在干净 `--root` 下装齐四家（Windows 本机实测，走真下载真校验）；输出含每家 version 探针结果
- `oma agents` 能以 source 报出 oma 自管安装；`oma spawn --agents <刚装的>` 可拉起（stub 或真实）
- 已装任一家时重跑 `oma agents install` 报 skip 不重装
- `cargo test` 全绿；文档三件套过；R002 / S014 / INDEX / TODO / diary 同步
- Linux / mac：catalog 资产与 `asset_for` 路径就绪（单测覆盖非 windows 组合），运行验收标注「待环境切换」

## 实施过程与经验

### 渠道研究的三次反转

> 2026-08-31，用户连发定调下当日闭环。

- **「默认 github、CDN 兜底」倒逼渠道一手化**：最初只核了 GitHub release 面与 grok 的 x.ai CDN。用户点破「四家都有官方安装脚本可实证渠道」后逐家抓脚本：kimi 的 install.sh（WebFetch 服务端抓，本机 DNS 对 code.kimi.com 间歇超时）实证 CDN 三件套——版本通道 `{base}/latest`、平台清单 `{base}/binaries/<ver>/manifest.json`、二进制 `{base}/binaries/<ver>/<filename>`；codex 的 install.sh 实证**纯走 GitHub**（README 里的 `releases.openai.com/codex` 实为 404 死桶），且 release JSON 的 `assets[].digest` 自带 sha256 逐资产哈希；claude 的 GCS 分发桶存在（`stable` 返回 2.1.236，滞后于 GitHub 的 2.1.251）但资产 URL 三探 404、install.ps1 域名不通——留槽不填。[实证: 脚本全文与探测记录见 S017 第四节]
- **kimi 双渠道制品不同**：CDN manifest 是裸单二进制（`kimi-code-win32-x64.exe`，checksum 与 GitHub zip 完全不同源）。这推翻了「资产矩阵 per-agent 一份」的初设计，schema 改为**资产按 source 绑定**（github=zip 家族、cdn=single 家族），`[[agents.sources.assets]]` 挂渠道下。[实证: manifest.json 与边车 checksum 对照]
- **oma 自管根定在 `~/.ohmyagents`**（用户定调「oma 在用户 home 下建立维护自己的应用数据」）：`OMA_HOME` 环境变量可覆盖（隔离验收全靠它）；rmux 的 managed_root 同步迁入并保留旧 `%LOCALAPPDATA%\ohmyagents` 兼容探测（本机 `oma check` 回归绿）。

### pin 自维护闭环

- catalog 两层：仓内 `catalog\agents.toml` 是出厂锚（include_str 编译嵌入），用户本地层 `~/.ohmyagents\catalog\agents.toml` 由 `oma agents update` 写回（`render_catalog` 手写渲染，round-trip 单测锁死）。resolve 顺序用户层优先；**用户层存在但损坏是硬错误**（提示删文件重置），不静默降级。
- update 取证阶梯：github 家 `assets[].digest` 优先（一次 API 拿全）→ SUMS 清单 / `.sha256` 边车兜底；kimi CDN manifest 一次拿六平台 filename+checksum；grok 无清单就下载自算（GCS 兜底源也下了 linux 资产算哈希）。**任一资产取证失败整体报错保旧 pin**——不产出半新半旧的 pin。
- 版本与更名规则四形全覆盖：`version_from_tag` 取最后一段 `@` 之后首个数字起（v2.1.251、rust-v0.151.0、@moonshot-ai/kimi-code@0.39.1、1.0.13）；资产名含旧版本才替换（codex/grok），不含则原名（claude/kimi zip 家族）。

### Windows 验收实录

- 隔离 `OMA_HOME` 下 `oma agents install --force` 四家全绿：claude 2.1.251、codex 0.151.0（`bin\codex.exe` 布局由 leaf 递归查找命中）、grok 1.0.13、kimi 0.39.1；sha 锚全过（不匹配即败），装后探针全部真实输出版本号。[实证: install.* marker 输出]
- **grok 承接 ohmypwsh 的自算 hash 被装机自证**：x.ai CDN 下载实测 sha 与 pin 一致，跨仓信任转移成立。
- 自适应跳过：不 `--force` 时四家按真实安装位（`~/.local/bin`、codex standalone releases、`~/.grok/bin`、`~/.kimi-code/bin`）报 skip，零下载。[实证]
- `oma agents` 探测集成：oma 自管位以 `extra=` 行出现，source 排名 path 优先于 oma——**用户自装优先、oma 兜底**，不抢已装。[实证]
- update 闭环：`update grok` 报 uptodate（版本通道解析）；`--force` 走 GCS 兜底取证 + 装机 + 写回用户本地 pin 层（渲染 schema 正确）。[实证]
- 测试：47 单测（catalog 校验四例、find_binary 布局两形、URL 三渠道、pin round-trip、探测 oma 排序）+ 7 集成（unknown agent 快败不触网）。`oma check` 旧根兼容回归绿。

### 遗留

- claude GCS 兜底槽、codex npm 制品兜底槽：URL 形状未实证，不填（S017 留档取证方法）。
- Linux / mac 运行验收（含 unix chmod +x 真跑）待环境切换接管；catalog 资产与 `asset_for` 路径已就绪并有单测覆盖。
- grok 装后首启会自写 `~/.grok/config.toml`（marketplace 注册），oma 按边界不预写（P0012 非目标）。
