# D16 oma self update 镜像通道

- 状态：已完成（代码与文档落地；镜像端到端待 ohmycloud 播种后实证）
- 日期：2026-09-08
- 关联：PRD D16；`docs\research\S028-oma自更新机制.md` 追记；ohmycloud#6 五点回执

## 背景与问题

ohmycloud 对账五点第 5 问：oma 有无 ome 同款 OME_MIRROR 类开关。答案是没有，而本机当日实测 api.github.com TLS 超时频发，无直连环境（含 D13 的 mac）需要镜像通道。用户裁「要做」。

## 方案

四点裁定（PRD D16 第 1 轮）：`OMA_MIRROR=<基址>` 环境变量对齐 ome OME_MIRROR；只覆盖 dev 通道（镜像无 manifest，stable 留 GitHub）；边车 sha256 判新免 manifest（裸哈希归一 `sha256:<hex>` 与 GitHub digest 记录互认）；网络类失败回落 GitHub，哈希不符属安全问题报错不回落。

## 落地与验收

- 全部改动在 `src\update.rs` 一节：`mirror_base` / `host_asset_name` / `mirror_asset_urls` / `parse_sidecar` / `dev_via_mirror`；下载复用 `install::download_asset`，校验复用 `archive::sha256_file`，安装复用 self_replace；记录 tag 记 `dev-mirror`。
- marker 只增不改（R011 冻结面）：新增 `update.mirror` / `update.source` / `update.fallback`，既有键语义不动。
- 新增 5 个纯函数单测（三元组资产名 oracle 取 dev-release.yml 命名约定字面量、sha256sum 格式解析、digest 跨源互认）；102 单元加 17 集成全绿 [实证： 2026-09-08 本机 cargo test]；四门禁绿。
- 未跑真网络 self update（会替换本机二进制）；镜像路径网络分支走查未端到端，待 ohmycloud 播 oma/dev 段后实证。

## 经验

- 判新键跨源互认要先归一：GitHub digest 是 `sha256:<hex>`、sha256sum 边车是裸 hex，归一到同形再比对，否则换源必触发一次误更新 [实证： 本轮 digest_matches 互认单测]。
- 「网络失败回落、校验失败不回落」是镜像通道的安全分界：可用性问题可以降级，完整性问题不行。
- 初版单测把实现同款 cfg 分支抄进断言（重言式，R004 点名高发），oracle 改用工作流命名约定字面量后才算数。
