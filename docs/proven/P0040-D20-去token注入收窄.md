# D20 去 token 注入收窄

- 状态：已完成（124 单元加 21 集成全绿；四门禁绿；只改程序代码未动任何部署位）
- 日期：2026-09-09
- 关联：PRD D20；D07 迁册（install/update 的 deprecated 来源）；S026（login）/ S027（providers）/ S031（secrets）研究底座；R001 / R002 命令面

## 背景与问题

用户裁定：oma 不再管理 agent token 的环境变量注入，删除这个功能；oma 专注五个功能（agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置）。这是继 D15 去编排之后第二次定位收窄，与四仓分工对齐（密钥安全归 ohmypwsh、agent 二进制归 ome）。

第 1 轮四裁：providers 别名簿一并删除；`oma agents login` 设备码引导删除（doctor 登录态诊断保留）；deprecated 的 `agents install/update` 兼容层顺带删除；本地安装的先不动、只改程序代码、不要安装（shell profile 懒注入块与各端部署位不动，不部署新二进制，vault 文件不动）。

## 方案

- 删除面：`oma agents secrets`（init / set / env / inject / status，S031 一钥两密文与四 shell 懒注入）、`oma agents providers`（别名簿）、`oma agents login`（设备码引导）、`oma agents install` / `update`（D07 deprecated 兼容层）与 catalog 安装机器（`catalog\agents.toml` 冻结锚随删，数据权威早在 ome）。
- 保留面：五功能命令（doctor / agents 检测 / statusline / verify / init / hook / trace / self update / completions）；`oma hook` 的密钥拦截闸（S030）保留，实值比对层从「环境变量加 providers.toml 明文」收窄为只看环境变量。
- 迁移姿态：纯代码变更。已写进 shell profile 的懒注入块（内部调 `oma agents secrets env`）与各端 oma 部署位不动；存量 vault 文件（app.key / identity.enc / secrets.yaml）不动不删。旧 oma 在位期间 profile 块仍工作；用户换装新版前自行处置（密钥面归 ohmypwsh 接管或手动摘块）。

## 落地与验收

- 净删约三千行：`src\secrets.rs`（714）/ `src\install.rs` 安装机器（995 瘦至约 160，留 oma_home / managed_binaries / managed_version / download_asset）/ `src\login.rs`（349）/ `src\providers.rs`（213）/ `src\catalog.rs`（250）与 `catalog\agents.toml`；main.rs 五命令面与 handler、COMMAND_MAP 三行、SKILL 再生。
- `oma agents` 缺装 hint 从 `oma agents install <名>` 改指 `ome install <名>`。
- 测试 [实证]：单元 145 降 124、集成 24 降 21（随面删除同步收缩）；全绿零 unused 警告；四门禁绿。
- 文档八面：AGENTS（本质五功能、边界、四仓分工、路由删五行）、R001（本质与边界加 D20 条）、R002（全表与 2.2 / 2.3 重排，移除面历史口径注记）、R011（删 install 排期句）、INDEX（代码表删五行加移除注记）、README（五功能重述，token 节删除）、CHANGELOG、四原语。

## 坑与处理

1. M054 同型四犯：heredoc python 里 `\\` 序列被工具参数 JSON 编码吃层（本轮 `\c` / `\l` 与 `\r` 各炸一次，后者把 `docs\research` 劈成回车）。处置升级：含反斜杠路径的 md 批改一律 Edit / Write 直改，heredoc 只做无反斜杠文本。

## 经验

- 收窄式重构的生成物链路清单（D15 教训沿用）：COMMAND_MAP 源头、SKILL 再生、skill description、hint 文案、R002 全表、AGENTS 三节、INDEX 代码表、R001 定位、README 功能列表，一处漏即留旧口径 [实证： 本轮逐面过一遍后 grep 复扫才清干净]。
- 「只改程序代码」的边界要在 PLAN 写死：不动部署位、不动 profile、不部署新二进制，验收判据因此不含本机实跑（测试面全绿即收）。
