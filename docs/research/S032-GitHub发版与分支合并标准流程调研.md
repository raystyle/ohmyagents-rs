# GitHub 发版与分支开发合并标准流程调研

- 日期：2026-09-08
- 来源：用户转交的成稿调研（抓取取证随稿）；归档编号 S032
- 用途：回答「本仓发版与分支合并该按什么标准走」；为发版自动化选型供底

## 结论速览

1. 没有「唯一标准」，主流三选一：GitHub Flow（官方轻量流，持续交付首选）、Git Flow（显式版本化、多版本并存）、Trunk-Based（大团队高吞吐）。Git Flow 作者 2020 反思注：持续交付类别用 Git Flow，选更简单的 GitHub Flow [实证： nvie.com 原文反思注]。
2. 分支合并的落地单元是 PR 加分支保护：建分支、提交、开 PR、评审、合并、删分支六步 [实证： docs.github.com github-flow 全文]。
3. 合并三式（merge commit / squash / rebase）选型决定历史形态；发版官方载体是基于 tag 的 Release 加自动生成 notes [实证： docs.github.com about-releases]。
4. 发版自动化生态成熟：semantic-release 24.0k 星、GoReleaser 16.0k 星、release-please 7.5k 星（action 2.5k），共同前提是 Conventional Commits [实证： 2026-09-08 gh search 星数]。

## 三种分支模型

| 维度 | GitHub Flow | Git Flow | Trunk-Based |
| --- | --- | --- | --- |
| 长命分支 | main 一条 | master + develop 两条 | trunk 一条 |
| 短命分支 | feature 分支 | feature / release-* / hotfix-* | 单人短命分支（存活小于 1 天） |
| 发布动作 | 合并即部署候选 | release 分支定版本号，合回 master 打 tag | trunk 即时切 release 或直接发，fix forward |
| 适用 | Web 应用、持续交付 | 显式版本化、多版本并存（客户端、库） | 大团队高吞吐 |
| 出处 | GitHub 官方文档 | nvie 2010 博文 | trunkbaseddevelopment.com |

Git Flow 关键规则：feature 从 develop 出回 develop；release 分支上才定版本号，回 master 打 tag 同时回 develop；hotfix 从 master 对应 tag 出、双合回（有 release 分支在则改合 release）；合并一律 `--no-ff` [实证： nvie 原文]。

## 合并三式

| 方式 | Git 行为 | 得 | 失 |
| --- | --- | --- | --- |
| merge commit（默认） | `--no-ff`，全部提交入 base 加合并节点 | 保留全部历史与分组 | 历史分叉，log 噪音大 |
| squash and merge | 压成单提交，fast-forward | 历史最干净 | 丢原始 SHA 与逐提交时间；同分支再开 PR 会重复列已压提交 |
| rebase and merge | 逐提交重放，无合并节点 | 线性且保留逐提交 | GitHub 重写 committer 与 SHA；产物不带签名 |

配套开关：仓库可只允许一种合并方式；分支保护 Require linear history 可禁 merge commit [实证： about-merge-methods 加 about-protected-branches]。

## 分支保护常用组合

PR 必须加 N 个批准（可叠 code owner、dismiss stale）；CI 状态检查必过（strict 要求与 base 同步，合并队列是无痛替代）；对话全解决才许合；高吞吐上 merge queue；默认禁 force push 与禁删分支；新版形态叫 rulesets [实证： about-protected-branches 全文]。

## 发版机制

Release 基于 git tag（tag 日期与发布日期可不同）；notes 可手写或模板自动生成；自动附源码 zip/tarball，额外资产上限 1000 个、单文件 2 GiB；write 权限管理，read 可看可订阅；安全修复 release 应同步发 security advisory [实证： about-releases]。

## 发版自动化三工具

| 工具 | 星数 | 机制 |
| --- | --- | --- |
| semantic-release | 24028 | 解析 Conventional Commits，全自动 bump、CHANGELOG、GitHub Release/npm |
| goreleaser | 16024 | Go 生态标准，交叉构建打包发布一体 |
| release-please | 7462（action 2515） | Release PR 攒 changelog，人合 PR 即打 tag 发版，保留人工决策点 |

三者输入都是 Conventional Commits，前置纪律是规范提交信息 [推断： 三工具文档共同要求，未逐一抓全文；复核点各自 README quickstart]。

## 对本仓的适用结论

[推断： 对照本仓现状（2026-09-08）]

1. 本仓现状是「main 直推加 tag 封版」的单人 GitHub Flow 无 PR 变体：CI 门禁（test 加构建）已实质承担 status check 角色，dev 滚动 prerelease 加 v* tag 正式 release 双通道已落地（D15 后），D41 起产物镜像也随 CI 自动同步。对照三模型，此形态在单人加多 agent 协作规模下是合理解，不欠 Git Flow 的 develop/release 分支（无多版本并存需求）。
2. 提交前缀已是 `feat:` / `fix:` / `docs:` / `chore:`（AGENTS 工作规则第 7 条），与 Conventional Commits 兼容，未来上 release-please 或 semantic-release 的供料纪律已在。
3. 可选项（未裁）：main 分支保护（禁 force push、CI 必过）成本近零可开；发版自动化工具在「封版节奏仍由用户逐裁」的现状下收益低，暂缓。

## 来源

- docs.github.com：github-flow / about-releases / about-merge-methods / about-protected-branches（全文抓取）
- nvie.com Git Flow 原文（2010，2020 反思注）；trunkbaseddevelopment.com 首页
- 工具星数 gh search 实测（2026-09-08）；辅助对照 codewithmukesh 对比文等 SERP 摘要
