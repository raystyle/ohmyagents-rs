# 项目工具PowerShell模块选型细则：psgallery与psresourceget

> AGENTS 工作规则「写临时脚本时」的 PowerShell 侧细则。与 `R005`（Rust 产品依赖）、`R008`（py 工具）三分项目工具选型。素材：2026-08-31 用户提供《PowerShell 模块选择与开发稳定指南》，经本机实证并对照 ohmypwsh `scripts\psmodule.ps1` 与 `modules.psd1`（自研模块管理器，P0006）的实际配置对齐后落定。

## 一、搜索能力对照

> 与 PyPI 相反：Gallery 网站与官方 CLI 搜索都有。[实证: 2026-08-31 本机 pwsh 7.6.5]

| 通道 | 用法 |
| --- | --- |
| 网站关键词 | `powershellgallery.com`，字段语法 `Id:` / `Tags:` / `Author:` |
| CLI 条件搜索 | `Find-PSResource <名>`（7.4+ 首选，PSResourceGet 1.2.0 本机实证）；旧栈 `Find-Module -Name / -Tag / -Filter`（实证 ImportExcel 7.8.10 可查） |
| 已知名字元数据 | `Find-PSResource <名>` 出版本与描述 |

坑与稳妥做法：`Find-PSResource` 裸调偶发「could not be found in any registered repositories」（本机两次一败一成，冷查询抖动）；显式 `-Repository PSGallery` 可复现——ohmypwsh `psmodule.ps1` 即恒带显式仓。[实证: 2026-08-31 两轮对照]

## 二、稳度判据

> 选型看四项，装前必核名字。

| 信号 | 看什么 |
| --- | --- |
| 下载量 | Gallery 页 Downloads |
| 最近更新 | Last Updated |
| 维护者 | Owner（`azure-sdk`、`msgraph-sdk-powershell` 这类官方账号） |
| 项目页 | ProjectUri 有源码有文档 |

名字必须与官方文档一字不差（2026 出现过大规模仿冒）。[经验: 指南] 相对稳妥一类：`Az` / `Microsoft.Graph` / `Pester` / `ImportExcel` / `PSScriptAnalyzer` / `PSReadLine`。[经验: 指南]

## 三、安装与版本管理

> 本仓（及本机）模块**经 ohmypwsh `psmodule.ps1` 统一管理**，不自行 `Install-Module`。

```powershell
# 选型与钉版（ohmypwsh 仓）
pwsh -NoProfile -File D:\ohmypwsh\scripts\psmodule.ps1 pin <模块> -Latest     # 钉最新
pwsh -NoProfile -File D:\ohmypwsh\scripts\psmodule.ps1 install <模块>         # 按 modules.psd1 装到 D:\ohmyenv\modules
pwsh -NoProfile -File D:\ohmypwsh\scripts\psmodule.ps1 update [all]           # 主版本跨版要 -IncludeBreaking
```

对齐说明（ohmypwsh 现状对指南的超集与偏离）：

- **版本锁超出指南**：指南口径「生产钉 `-RequiredVersion`」；ohmypwsh 锁定清单 `modules.psd1` 记 Version 加 nupkg SHA256 双锁，安装前校验哈希。[实证: modules.psd1 Pester 5.7.1 与 Posh-SSH 3.2.7 均带 Sha256]
- **安装走直连 nupkg 而非 Install-PSResource**：`api/v2/package/<名>/<版本>` 下载加哈希校验加手工部署——指南说「日常用 CLI 不必手写 HTTP」，此处是 S028 结论的有意偏离（规避 PSGallery 直连不稳），哈希锁兜住了完整性。[经验: ohmypwsh S028]
- **5.1 的 PowerShellGet/TLS 老坑被架构绕开**：psmodule `#Requires 7.0`，安装由 pwsh7 执行，PS5 只作为部署目标目录、不跑安装；bootstrap 无 TLS 1.2 处理但不需要。[实证: psmodule 与 bootstrap 源码]
- **Trusted 语义**：本机 PSGallery 在两套注册表均为 Untrusted，ohmypwsh 也不设 Trusted——与指南「Trusted 只减确认不代表安全」一致，且哈希校验比 Trusted 更强。[实证: Get-PSRepository / Get-PSResourceRepository]
- **注册表残留已清净**：S028 时点提过的 `OhMyClaude` 本地仓残留，现两表均只剩 PSGallery。[实证: 2026-08-31]

## 四、写与打包

- 临时脚本直接 ps1 归档 `.tools\`（AGENTS 规则 11）；有模块形态价值才 `New-ModuleManifest`（GUID 建后不重建；`FunctionsToExport` 禁 `*`）。[经验: 指南]
- 打包：`psmodule.ps1 pack <目录>`（底层 `Compress-PSResource`）。[实证: ohmypwsh]
- 发布 Gallery：`Publish-PSResource -ApiKey`（本仓暂无发布需求，流程留指南口径）。[经验: 指南]
- PS5 兼容：含中文的 ps1 统一 UTF-8 BOM（AGENTS 规则 6 同源）。

## 五、决策树

| 目标 | 做法 |
| --- | --- |
| 不知道叫什么 | 网站或 `Find-PSResource`（CLI 条件搜索有） |
| 已知名字 | `Find-PSResource <名> -Repository PSGallery` 核版本 |
| 装进本机 | ohmypwsh `psmodule.ps1 pin` 加 `install`（版本哈希双锁） |
| 脚本临时用 | 优先无依赖 ps1；确需模块走 psmodule，不散装 |

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本地 | 用户《PowerShell 模块选择与开发稳定指南》（Downloads） | 2026-08-31 | 三通道对照、稳度判据、LTS 口径骨架 |
| 本地 | ohmypwsh `scripts\psmodule.ps1` 与 `modules.psd1`、S028 | 2026-08-31 | 本机实际模块管理机制与锁定清单 |
| web | PSGallery（Find-PSResource / Find-Module / 两套仓库注册表） | 2026-08-31 | 搜索、元数据、注册表现状实测 |
