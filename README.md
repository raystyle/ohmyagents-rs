# HST

**HST**，全称 Hooks, Statusline, Trace（原 Oh My Agents / oma，v0.6.0 更名）：Claude Code / Codex / Grok / Kimi 四家的 hook 落盘、状态栏、只读对话 trace、可用性诊断与 yolo 非阻塞配置，Windows / macOS / Linux（含 WSL）同一命令面。CLI 名 `hst`。与 Hipo 的 hst history picker 共存：本工具装用户目录（如 ~/.local/bin、~/.hst/bin），不覆盖 /usr/bin/hst。

- `hst doctor`：agent 可用性只读体检，warn 与 block 分层，block 才退出 1
- `hst init`：把 hook 注册与 yolo / 非阻塞键落进各家用户级配置（claude / codex / grok / kimi 四家统一，未 init 的项目也有状态数据；yolo 缺省用户级全机生效，`--project-yolo` 显式选项目级），skill 落进项目，幂等合并；注册指向自包含状态 shim（`~/.hst/hooks/`），hst 二进制任意时刻可无痛升级轮换
- `hst statusline`：四家状态栏写入面（starship 风格、支持用户级定制）
- `hst trace`：项目内四家 agent 对话历史只读检索
- `hst agents verify`：四家无头验收（hook 落盘加状态栏脚本直跑）

hst 不做编排（不拉会话、不发任务），不管 token 注入（密钥安全归 [ohmypwsh]，agent 二进制安装归姊妹工具 `ark`（Agent Runtime Kit，原 ome）：`ark install claude`）。

[ohmypwsh]: https://github.com/raystyle/ohmypwsh

## 安装

预编译二进制覆盖三平台。装好后把 `hst`（Windows 为 `hst.exe`）放进 PATH 上的任一目录即可（Windows 建议 `%USERPROFILE%` 下的 `.hst` 的 `bin`）。

### Windows

```powershell
# 最新正式版
Invoke-WebRequest https://github.com/raystyle/hst-rs/releases/latest/download/hst-x86_64-pc-windows-msvc.zip -OutFile hst.zip
Expand-Archive hst.zip -DestinationPath $HOME\.hst\bin
Move-Item $HOME\.hst\bin\hst-x86_64-pc-windows-msvc\hst.exe $HOME\.hst\bin\
# 把 $HOME\.hst\bin 加进 PATH 后重开终端
hst --version
```

### macOS

```bash
curl -L https://github.com/raystyle/hst-rs/releases/latest/download/hst-aarch64-apple-darwin.tar.gz | tar xz
mkdir -p ~/.local/bin && mv hst-aarch64-apple-darwin/hst ~/.local/bin/
hst --version    # ~/.local/bin 需在 PATH
```

### Linux x86_64 与 WSL

```bash
curl -L https://github.com/raystyle/hst-rs/releases/latest/download/hst-x86_64-unknown-linux-gnu.tar.gz | tar xz
mkdir -p ~/.local/bin && mv hst-x86_64-unknown-linux-gnu/hst ~/.local/bin/
hst --version
```

### 源码安装

```bash
cargo install --git https://github.com/raystyle/hst-rs
```

### 前置与说明

- 状态栏运行时是 pwsh（PowerShell 7），全平台一致：装了才有状态栏，缺了只是不渲染，不影响其它命令
- 想跟开发滚动版：上面的资产 URL 把 `releases/latest/download/` 换成 `releases/download/dev/`；配了镜像环境可设 `HST_MIRROR=https://env.ohmygh.com` 后 `hst self update` 走镜像（旧 OMA_MIRROR 一个版本内仍读并提示）
- hst 自管数据根是 `~/.hst`（旧 `~/.oma` 首启自动同卷 rename 迁移）（shim、session 分键状态、状态栏脚本都在这）；hook 注册与 yolo / 非阻塞键写各家用户级配置（yolo 全机生效），skill 落项目目录

## 快速上手

```powershell
cd D:\my\proj          # 进你的项目，后续命令都不用再带路径
hst init               # 部署 hook / skill / yolo 键加旧 ~/.oma 迁移 heal（幂等）
hst statusline         # 配置四家状态栏
hst doctor             # 体检：有 block 级问题才退出 1
```

## 使用示例

全部命令支持 `--format kv|json|jsonl` 与 `--json` 信封输出。

### 部署到项目

```powershell
hst init                        # 全套：用户级 yolo 键加四家 hook/skill 加 heal 迁移
hst hook init                   # 仅 hook 面（注册加 shim 落位）
hst init --yolo                 # 仅用户级无阻塞键（全机生效）
hst init --project-yolo         # 仅项目级无阻塞键（项目覆盖用户级）
hst init --pre-trust             # 额外预写家目录信任库（四家）
hst init --project D:\my\proj   # 不进目录也能指定项目
```

### 状态栏

```powershell
hst statusline                  # 四家全配（幂等）
hst statusline codex            # 只配一家
hst statusline --example        # 用户级定制模板（~/.hst/statusline.toml）
```

定制三层：段落显隐与顺序（`segments`）、段内模板与图标（`[template]` / `[icons]`）、整脚本替换（`hst statusline --script <路径>`，`--builtin` 还原）。改完配置重跑一次 `hst statusline` 生效。

### 诊断与验收

```powershell
hst doctor                  # yolo / 信任 / 登录态 / hook 形态 / 状态栏 / CPU 指令集
hst agents                  # 列四家检测：装没装、来源与版本
hst agents verify           # 四家无头验收：hook 落盘加状态栏直跑
hst hook verify kimi --timeout 120     # 单家 hook 层验收
```

### 活性诊断

打真网关（api 缓存回归测试端点，配置注入）烧最小 token，与 doctor 的零网络体检分家；凭据读 agent 侧配置，也可用 `HST_GATEWAY_URL` / `HST_GATEWAY_KEY` 覆盖（旧 `OMA_GATEWAY_*` 一个版本内仍读并提示）。

```powershell
hst diagnose cache                    # 全别名缓存命中矩阵（双连探测）
hst diagnose cache zy-gpt56sol-codex  # 只测指定别名
hst diagnose agents                   # 配置指向、别名在册、key 活性、thinking 上限
```

### 对话历史检索

```powershell
hst trace sessions          # 项目内各 agent 的会话
hst trace search "重构"      # 按正则检索四家的修改与意图
hst trace file src\main.rs  # 单文件轨迹：谁、何时、为何改的
hst trace agent claude      # 某家 agent 的操作块时间线
```

### 自维护

```powershell
hst self update             # 自更新（缺省 dev 滚动源；镜像 HST_MIRROR）
hst self update --stable    # 走正式版
hst completions powershell  # 补全（bash / zsh / fish / powershell）
hst skill --write           # 生成 hst 自身技能到 ~/.claude/skills/（自适应命令树）
```

## 更多文档

- 命令手册细则：`docs\references\R002-常用命令与管理流程-从项目init到部署诊断.md`
- 需求与设计：`PRD.md` 与 `docs\` 各分册（`INDEX.md` 是总索引）

## 注意

`hst init` 的 yolo 面（缺省用户级）会关掉 agent 的审批与沙箱且**全机所有项目生效**，只在自己信任的机器与账户上用；要收窄到单项目用 `--project-yolo`。oma 旧命令由过渡 stub 警告后转调 hst（release 兼容期同挂 oma-* stub 资产）。
