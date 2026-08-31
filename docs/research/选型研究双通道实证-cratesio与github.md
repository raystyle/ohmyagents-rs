# 选型研究双通道实证-cratesio与github

> 2026-08-31。编码经验研究（`ponytail懒人阶梯与oma编码经验.md`）的两个配套落地方法合并沉淀：crates.io 通道（选 crate 看稳度）与 GitHub 通道（选项目看质量、深读源码、搜文档）。规则性结论已固化进 `docs\guide\选型研究细则-cratesio与github双通道.md`（操作手册），本篇保留两法的完整证据链与淘汰现场。素材来自用户综述与 investigator skill 速查表，全部本机实证。

## 背景

AGENTS 写 Rust 规则要求「先查最流行、最稳定的库」。两通道各答一半：crates.io 答「哪个 crate 稳」，GitHub 答「哪个项目值得学、官方文档怎么讲、源码怎么写」。

## 关键结论

### crates.io 通道

1. **三级通道按成本递增**：内置 CLI（零安装）到第三方 CLI（增强）到 HTTP API（脚本）。日常选型内置 CLI 加 cratesinfo 即覆盖。[经验: 用户综述；本机实证可用]
2. **本机镜像环境订正**：cargo 源被 rsproxy 替换后，`cargo search` 与 `cargo info` 必须加 `--registry crates-io`。[实证: 2026-08-31 本机两命令复现]
3. **稳度四信号**：`max_stable_version` 非空（进过 1.x 更稳）、`recent_downloads` 高、`updated_at` 近 6 到 12 个月、`reverse_dependencies` 不低。启发式不是门禁：窄领域新库会被误杀，人工核 repository 再定（rmux-sdk 即例：0.x 但两周节奏，配 pin `=x.y.z`）。[经验: 用户综述阈值；四字段本机可取实证]
4. **排序口径**：`sort=recent-downloads` / `recent-updates` 比 relevance 与总下载量更接近「还在用还在维护」；crates.io 无质量分，要「最稳」用 API 自合成规则。[经验: 综述；sort 本机生效]
5. **反向依赖返回结构**：`.dependencies[].crate_id` 是被查 crate 自己；依赖方在 `.versions[].crate`，总数在 `.meta.total`。[实证: 2026-08-31 本机 jq 解包 rmux-sdk]
6. **第三方 CLI 四者实测，留 cratesinfo 卸其余**：cratesinfo 的 search 一屏列版本、双下载量、updated、`cargo add` 行，info 另有 created、docs、repository，versions 列历史，全通。落选：cargo-crates 0.1.5 编译失败；cargo-seek 纯 TUI 无 TTY 零输出（`-s` 只预填）；get-blessed 0.2.1 解析上游 panic（缺 `recommendations` 字段）。[实证: 2026-08-31 安装实测与卸载]
7. **限流礼节**：API 限 1 req/s 必带可识别 UA；大批量走 sparse index 或每日 dump。crates_io_api 未本机编译（oma 无程序化需求）。[经验: 综述；本机实测约 1.2s/req 未被拒]

### GitHub 通道

8. **`gh search repos` 是流行与活跃的一屏分辨器**：`--sort=stars --json fullName,stargazersCount,pushedAt,description` 一次拿齐；stars 高但 pushedAt 旧（tab-rs 684 星 2023 停更）与 stars 次但活跃（limux）立刻分开。[实证: 2026-08-31 搜 terminal multiplexer]
9. **新秀筛法**：关键词加 `created:>YYYY-MM-DD` 内联加 `--sort=stars`；实测筛 2026 年 Rust terminal 类头名 40873 星、herdr 33710 星在列（与本仓研究交叉验证）。[实证: 2026-08-31]
10. **单仓质量信号**：`gh repo view --json stargazerCount,pushedAt,licenseInfo,isArchived,issues,latestRelease,repositoryTopics`。rmux 实测 2606 星、latest v0.10.0；`license: null` 与 crates.io 标的 MIT OR Apache 不一致——仓库缺可识别 LICENSE 文件，定型前人工核。[实证: 2026-08-31]
11. **发布节奏**：`gh api repos/<o>/<r>/releases`，rmux 实测 0.8 到 0.10 约两周一版。[实证: 2026-08-31]
12. **code search 找真实用法**：对 rmux-sdk 搜签名片段返回空——新库无第三方公开用法，生态薄的采用风险信号。[实证: 2026-08-31；机制由 investigator CS 用例背书 经验]
13. **文档搜索**：gh 2.97.0 无 docs 子命令；正解 `gh search code "<词> extension:md"` 内联（实测命中 fzf CHANGELOG 等），`--filename "*.md"` 通配空不可靠；官方文档定点 `--repo github/docs`（webhooks 命中 content 源文件；空结果 = 官方没写）。[实证: 2026-08-31 三组对照]
14. **clone 两档**：`--depth 1` 拿当前代码（rust-guidelines、clum 两次旧实证）；`--filter=blob:none --no-checkout` 拿全历史不拿内容（1.9MB 可 `git log`）。clone 后接 investigator 六步（rg / ast-grep / git）。[实证: 2026-08-31 blob:none 对照；depth 1 旧实证]

## 两通道分工

| 问题 | crates.io 通道 | GitHub 通道 |
| --- | --- | --- |
| 哪个 crate 稳 | recent_downloads、max_stable、updated | stars、pushedAt、releases 节奏 |
| 生态厚不厚 | reverse_dependencies | code search 用法命中数 |
| 源码值不值得学 | 只到元数据 | clone 深读、blob:none 看演进 |
| license 可信度 | crates 元数据 | 仓库 LICENSE 文件（两处不一致时人工核） |

操作手册（五步与四步工作流、坑速查十行）见 `docs\guide\选型研究细则-cratesio与github双通道.md`，本篇不重复。

## 踩坑沉淀

| 坑 | 正解 |
| --- | --- |
| 镜像源劫持 cargo search / info | 加 `--registry crates-io` |
| 第三方 CLI 素材与实况脱节 | 装前实测再留；本仓只留 cratesinfo |
| 反向依赖取错字段 | 依赖方 `.versions[].crate`，总数 `.meta.total` |
| 星数当唯一标准 | stars 与 pushedAt 并看；新秀另走 created 筛选 |
| qualifier 混独立 flag | created 等限定词与关键词同在引号内，语言用 `--language=Rust` |
| search 与 view 字段名不同 | `stargazersCount` 对 `stargazerCount`；open 数取 `issues.totalCount` |
| `--filename` 通配 | 扩展名用内联 `extension:md` |
| 全量 clone 浪费 | `--filter=blob:none --no-checkout` 先行 |
| gh 搜索大小写敏感 | 搜词与源码一致；rg 加 `-i`，结构匹配 ast-grep |
| code_search 403 | 手动等 reset 或降频 |

## 待办

1. ast-grep 本机未装：clone 深读需要结构搜索时再装并补实证。
2. cratesinfo 无 JSON 输出：纯管道场景走 curl 加 jq；oma 要程序化查询再引 crates_io_api。

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| web | crates.io API（search / crates / reverse_dependencies） | 2026-08-31 curl 实测 | 稳度字段、排序、反向依赖结构 |
| 本地 | cargo search / info / add；cratesinfo、cargo-seek、get-blessed、cargo-crates | 2026-08-31 | 镜像订正、四 CLI 淘汰现场 |
| github | gh 2.97.0 search repos/code、repo view、api releases、clone | 2026-08-31 | 流行活跃分辨、质量信号、文档搜索、两档克隆 |
| 本地 | `D:\sourcecode\CoreSkills\skills\investigator\` 速查表 | 2026-08-31 | gh 陷阱表（字段名、qualifier、通配）背书 |
