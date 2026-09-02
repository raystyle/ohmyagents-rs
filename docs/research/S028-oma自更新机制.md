# S028-oma自更新机制

> 2026-09-02。用户定调：oma 应有 update 功能自己去 GitHub 升级新版本；目前本地测试、**还不封版**（无 releases），命令面用 `oma self update`。

## 需求

- oma 自更新：查 GitHub Releases、按平台取资产、自替换；封版前 releases 为空要体面降级（`--git` 源码安装为主路径）。

## 关键结论

### 1. 机制

- **查询**：`GET api.github.com/repos/<owner>/<repo>/releases/latest`（GH_TOKEN 自动附带）；404 = 无 releases → 打 `update.release=unavailable` 与 `--git` 提示，退出码 0（封版前常态，不算错）。
- **版本比较**：tag 容忍 `v` 前缀，按点分数值逐段比（非数字段按 0）；不新于当前 → `update.ok=already-latest`（`--force` 跳过）。
- **资产约定**：**资产名即编译目标** `oma-<target-triple>.zip`（Windows）/ `.tar.gz`（Unix），按本机 OS 关键词（windows-msvc / apple-darwin / linux-gnu）加架构（x86_64 / aarch64）匹配；无平台匹配兜底任一 oma 资产（打日志供人工核对）。版本判据：dev 走资产 sha256，stable 走 release tag。
- **自替换**：新二进制先 copy 到旁路暂存；Windows 走 rename 舞步（运行中 exe 不能覆盖但可改名——当前改名 `.old` → 新就位 → 删 `.old`；失败回滚旧件保持可启动），Unix 直接 rename 覆盖（原子）。
- **`--git`**：`cargo install --git https://github.com/<repo>.git --force`（PATH 探针找 cargo）——封版前主路径。
- **`--repo owner/name`**：私有 fork / 改仓时覆盖（缺省 `raystyle/OhMyAgents`）。

### 2. CI 滚动 dev release 与部署位

- 工作流 `.github/workflows/dev-release.yml`：main 每推 → 三平台**构建加测试**（Windows x86_64-msvc、Linux x86_64-gnu、macOS **仅 arm64 不要 Intel**，用户定调）→ 覆盖发布 prerelease tag `dev`（delete + recreate，资产带 `.sha256` 附带文件）。
- **正式版靠版本触发**（用户定调）：`v*` tag 推送 → 同一矩阵出正式 release（`--latest`），`oma self update --stable` 消费。
- **部署位切换**：`oma self update` 缺省通道 = **dev 滚动源**（`releases/tags/dev`）；dev 通道判新 = **资产 digest 对上次安装记录**（`~/.ohmyagents/selfupdate.json`；实测纠正：digest 是压缩包哈希，与 exe 哈希不可比；缺记录或缺摘要保守更新）；stable（`--stable`）按 release tag 版本比较。
- 资产名即编译目标：`oma-<target-triple>.zip|.tar.gz`（用户定稿；版本判据走 release tag，dev 走 sha256）。

### 3. 实测

[实证: 本机 2026-09-02]

- `oma self update` 于无 releases 仓实跑：`update.current=0.1.0` → `update.release=unavailable`（404 路径）→ hint `--git`，退出码 0。
- 单测：版本比较（含 v 前缀/双位/非数字后缀）、资产匹配（三平台断言 + 兜底）。

## 待办

- 工作流推上 GitHub 后首跑验证（gh release delete/create 路由、资产 digest 字段是否随 API 返回）
- 封版时：release workflow 产 `oma-<triple>.(zip|tar.gz)` 资产 + sha256 附带文件（download 后校验，install.rs sha256_file 复用位已留）。
- `oma agents update`（agent 层）与 `oma self update`（自身）语义对照进 R002。

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| 本机 | `oma self update` 实跑（无 releases 404 路径） | 2026-09-02 | 降级行为实证 |
| web | GitHub REST releases/latest 语义（install.rs 既用同 API 族） | 2026-09-02 | 查询与资产字段 |
