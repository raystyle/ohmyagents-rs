# rmux检测版本哈希与全平台安装

- 状态：已完成（Windows 本机已装 0.10.0；Linux/mac 资产已 pin 未在本机跑）
- 日期：2026-08-29
- 关联：`oma check`；研究 pin 在 `catalog/rmux.toml`；对照 ohmypwsh 的 pin + SHA256SUMS

## 背景与问题

编排依赖 rmux 0.10 完整包（tiny CLI + `libexec` helper + daemon）。本机 PATH 无 rmux，win-rmux 写的 `D:\ohmyenv\rmux` 已过时。官方 `irm | iex` 不把哈希钉进本仓。项目必须自己检测、核版本与哈希、全平台缺失则安装。

## 目标与非目标

- 目标：
  - `oma check` 找到 rmux、读 `rmux -V`、校验哈希与完整布局
  - Windows / Linux / macOS / WSL；缺则按仓库 pin 下载 GitHub 资产并核 `SHA256SUMS`
  - 装到本工具自己的前缀，不覆盖用户已有官方安装；不 `kill-server`
- 非目标：
  - 本轮不安四路 agent
  - 不跑 `irm https://rmux.io/install.ps1 | iex`（无仓内哈希锚点）
  - 不把 `cargo install rmux` 当默认（缺官方资产哈希、布局不可控）

## 方案

pin 文件 `catalog/rmux.toml`：tag `v0.10.0`、各平台资产名与 SHA256（来自官方 `SHA256SUMS`）。信任锚是本仓提交的哈希，下载后的 `SHA256SUMS` 只作交叉核对。

安装根：

- Windows：`%LOCALAPPDATA%\ohmyagents\rmux\0.10.0\`
- Unix / WSL：`$XDG_DATA_HOME/ohmyagents/rmux/0.10.0\` 或 `~/.local/share/ohmyagents/rmux/0.10.0\`

完整布局（官方 installer 同构）：

- Windows：`rmux.exe` + `libexec\rmux\rmux.exe` + `rmux-daemon.exe`
- Unix：`bin/rmux` + `libexec/rmux/rmux` + `bin/rmux-daemon`

检测顺序：本前缀（清单哈希）优于 PATH。PATH 命中须版本等于 pin 且布局完整，否则按缺失安装到本前缀。`oma` 进程内始终把本前缀 bin 放 PATH 最前。

版本参数必须 `rmux -V`（`--version` 会 usage 且非 0）。

`oma check --no-install` 只诊断、不下载。默认缺则装。

## 备选方案

| 做法 | 取舍 |
| --- | --- |
| 官方 install.ps1 / install.sh | 核哈希，但 latest 不钉仓；本仓要 pin |
| winget / brew / scoop | 不保证与 0.10 资产哈希一致 |
| 只拷一个 `rmux.exe` | 无效安装 |

## 实施步骤

1. 写入 `catalog/rmux.toml` 与本方案
2. Cargo：`oma check` 检测 / 安装
3. 本机跑 `oma check`，装上后写回版本与哈希

## 风险与回滚

- GitHub 下载失败：提示用 `gh` 已登录环境或设 `GH_TOKEN`
- 哈希不一致：拒绝安装，不覆盖已有文件
- Windows 无 aarch64 预编译：明确失败，不静默 cargo install
- 回滚：删本前缀目录；不碰用户 `%LOCALAPPDATA%\rmux`

## 验收标准

- 无 rmux：`oma check` 下载 pin 资产、哈希一致、`rmux -V` 为 0.10.0、helper `--help` 含 `usage: rmux`，退出 0
- 已装且匹配：不再下载，打印路径、版本、哈希
- `--no-install` 且缺失：非 0
- 不调用 `kill-server`
