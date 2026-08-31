# github搜索研究法-gh命令与仓库深读实证

> 2026-08-31。编码经验研究的第二个配套落地方法：通过 GitHub 搜库评估与 clone 深读研究代码。素材来自用户口径（最流行的库、排名靠前的库、新发布但质量高的库、clone 深入学习），命令细节参照 `D:\sourcecode\CoreSkills\skills\investigator\` 的 gh 速查表与陷阱表，本机逐条实证后标六态。与 investigator skill 的分工：本方法是选型研究级（先于采用）；investigator 是代码符号级调查（采用后深挖），clone 落地后即衔接其六步链。

## 背景

方法一（`rust库搜索研发分析法-CLI与API选型实证.md`）回答「crates.io 上哪个 crate 稳」；本方法回答「GitHub 上哪个项目值得学」——流行度（stars）、活跃度（pushedAt）、发布节奏（releases）、真实用法（code search）、源码质量（clone 深读）。两法合起来才是写 Rust 规则「先查最流行最稳的库」的完整操作面。

## 关键结论

1. **`gh search repos` 是流行与活跃的一屏分辨器**：`--sort=stars --json fullName,stargazersCount,pushedAt,description` 一次拿齐，stars 高但 pushedAt 旧的（tab-rs 684 星 2023 停更）与 stars 次但昨天还在推的（limux）立刻分开。[实证: 2026-08-31 本机搜 terminal multiplexer]
2. **新发布高质量库的筛法**：关键词加 `created:>YYYY-MM-DD` 内联 qualifier 加 `--sort=stars`，新库按星排。实证筛 2026 年 Rust terminal 类，头名 40873 星，herdr 33710 星在列——与本仓既有研究交叉验证。[实证: 2026-08-31 本机]
3. **单仓质量信号一次取齐**：`gh repo view <o/r> --json stargazerCount,pushedAt,licenseInfo,isArchived,issues,latestRelease,repositoryTopics`。rmux 实测：2606 星、pushed 2026-08-09、未归档、10 个 open issues、latest v0.10.0、topics 齐；`license: null` 与 crates.io 标的 MIT OR Apache 不一致——GitHub 仓库缺可识别 LICENSE 文件，是真实信号。[实证: 2026-08-31 本机]
4. **发布节奏看 releases**：`gh api repos/<o>/<r>/releases --jq '.[0:4]...'`，rmux 实测 0.8 到 0.10 约 two-week 节奏，配 crates.io 的 updated_at 互证。[实证: 2026-08-31 本机]
5. **code search 找真实用法**：`gh search code "<签名片段>" --language=Rust`；对 rmux-sdk 实测返回空——新库无第三方公开用法，这本身是采用风险信号（生态薄），机制可用性由 investigator 的 CS 系列用例背书。[实证: 空结果 2026-08-31；机制 经验: investigator verification]
6. **clone 两档按需选**：`--depth 1`（只看当前代码，本仓 rust-guidelines 与 clum 研究已两次实证）；`--filter=blob:none --no-checkout`（拿全 commit 历史不拿文件内容，1.9MB 可跑 `git log`，先判断演进再决定要不要内容）。[实证: 2026-08-31 blob:none 模式 1.9M 对比；depth 1 引 2026-08-29/31 两次旧实证]
7. **clone 之后衔接 investigator 六步**（rg 扫描、ast-grep 结构、git 溯源），本方法不重复其命令面；其速查表的两条陷阱在 repos 搜索同样成立：`--json` 字段名 search 与 view 不同（`stargazersCount` 对 `stargazerCount`）、bool flag 必须 `=` 传值。[经验: investigator 陷阱表；字段名差异本机复现]
8. **gh 无独立文档搜索子命令，正解是 code search 加 `extension:md` 内联**（gh 2.97.0 的 `gh search` 只有 code / commits / issues / prs / repos）。规范写法 `gh search code "<关键词> extension:md"` 跨仓只扫 markdown，实测一次命中 fzf CHANGELOG、tabby README 等真实文档；`--filename "*.md"` 通配**不可靠**（实测空结果，印证 investigator 陷阱）。[实证: 2026-08-31 本机三组对照]
9. **搜 GitHub 官方文档 = 定点其源仓**：`gh search code "<词>" --repo github/docs`（docs.github.com 内容的开源镜像仓）。实测 webhooks 命中 `content/rest/**/*.md` 一列源文件；某词返回空即「官方文档没写这概念」，本身是信号。[实证: 2026-08-31 本机]

## 现状或实测

### 本机实证命令与结果（2026-08-31，gh 已登录）

| 步骤 | 命令 | 结果 |
| --- | --- | --- |
| 流行度排序 | `gh search repos "terminal multiplexer" --language=Rust --sort=stars --limit 6 --json ...` | 通；一屏分辨 tab-rs（684 星停更）与 limux（546 星活跃） |
| 新库筛选 | `gh search repos "terminal created:>2026-01-01" --language=Rust --sort=stars` | 通；qualifier 必须与关键词同在内联串里 |
| 单仓信号 | `gh repo view Helvesec/rmux --json stargazerCount,pushedAt,licenseInfo,isArchived,issues,latestRelease,repositoryTopics` | 通；`issues.totalCount` 取 open 数，无 `openIssues` 字段 |
| 发布节奏 | `gh api repos/Helvesec/rmux/releases --jq '.[0:4]...'` | 通；四版间隔约两周 |
| 用法搜索 | `gh search code "rmux_sdk ensure_session" --language=Rust` | 空（新库无第三方公开用法） |
| 轻量克隆 | `gh repo clone Helvesec/rmux rmux-probe -- --filter=blob:none --no-checkout` | 通；1.9MB 全历史可 `git log`，零工作区 |
| 浅克隆 | `git clone --depth 1` | 已有 rust-guidelines、clum 两次旧实证 |
| 文档搜索（跨仓 md） | `gh search code "bracketed paste extension:md"` | 通；命中 fzf CHANGELOG、tabby README 等 |
| 文档搜索（通配法） | `gh search code "bracketed paste" --filename "*.md"` | 空：`--filename` 通配不可靠，用 `extension:md` |
| 官方文档定点 | `gh search code "webhooks" --repo github/docs` | 通；命中 `content/rest/**/*.md` 源文件 |

### 推荐工作流（选型研究四步）

```bash
# 1 领域扫描：流行与活跃一屏分辨（stars 排序看 pushedAt）
gh search repos "<领域词>" --language=Rust --sort=stars --limit 10 \
  --json fullName,stargazersCount,pushedAt,description \
  --jq '.[] | "\(.stargazersCount)\t\(.fullName)\t\(.pushedAt[0:10])\t\(.description)"'

# 1b 找新秀：加 created 限定换 sort 或看新星
gh search repos "<领域词> created:>2026-01-01" --language=Rust --sort=stars ...

# 2 定点核证：stars、pushed、license、archived、issues、latest、topics
gh repo view <owner>/<repo> --json stargazerCount,pushedAt,licenseInfo,isArchived,issues,latestRelease,repositoryTopics

# 3 发布节奏与用法
gh api repos/<owner>/<repo>/releases --jq '.[0:4] | .[] | "\(.tag_name)\t\(.published_at[0:10])"'
gh search code "<库的签名片段>" --language=Rust --limit 5

# 3b 文档证据：跨仓只扫 md；官方文档定点其源仓
gh search code "<概念词> extension:md" --limit 5
gh search code "<概念词>" --repo github/docs --limit 5

# 4 深读：先 blob:none 看历史节奏，再决定 depth 1 拿代码
gh repo clone <owner>/<repo> <dir> -- --filter=blob:none --no-checkout   # 1.9M 级
git clone --depth 1 https://github.com/<owner>/<repo>                    # 要读代码时
# clone 后进入 investigator 六步（rg / ast-grep / git），见其 SKILL.md
```

### 与方法一的分工

| 问题 | 方法一（crates.io） | 方法二（GitHub） |
| --- | --- | --- |
| 哪个 crate 稳 | recent_downloads、max_stable、updated | stars、pushedAt、releases 节奏 |
| 生态厚不厚 | reverse_dependencies | code search 用法命中数 |
| 源码值不值得学 | —（只到元数据） | clone 深读、blob:none 看演进 |
| license 可信度 | crates 元数据 | 仓库 LICENSE 文件（rmux 实测两处不一致，以仓库为准需人工核） |

## 踩坑沉淀

| 坑 | 现象 | 正确处理 |
| --- | --- | --- |
| qualifier 混独立 flag | `--language=rust created:>... terminal` 报「None of the search qualifiers apply」 | created 等限定词与关键词同在引号内联串，语言用独立 flag `--language=Rust` |
| view 无 openIssues 字段 | `--json openIssues` exit 1 | 用 `issues` 对象取 `.issues.totalCount`；search 命令才是 `stargazersCount` |
| 星数当唯一标准 | tab-rs 星最高但停更 | stars 与 pushedAt 并看，新秀另走 created 筛选 |
| crates 与仓库 license 不一致 | rmux crates 标 MIT OR Apache、GitHub license null | 定型前人工核仓库文件，元数据只当线索 |
| 全量 clone 浪费 | 只想看历史 | `--filter=blob:none --no-checkout` 先行，要代码再 depth 1 |
| 以为 gh 有 docs 子命令 | `gh search docs` unknown command | 文档搜索用 code search 加 `extension:md` 内联；官方文档定点 `--repo github/docs` |
| `--filename "*.md"` 通配 | 空结果 | 扩展名限定用内联 `extension:md`，filename 给具体名 |

## 待办

1. 六态已齐，与方法一合并升规则：浓缩为 guide 细则（双通道检索手册），AGENTS 写 Rust 规则引用。
2. ast-grep 本机未装（investigator 兼容性要求）：clone 深读需要结构搜索时再装并补实证。
