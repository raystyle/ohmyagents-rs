# Oh My Agents

**Agent 全平台部署配置与诊断工具**，专注五个功能：agent 可用性诊断、hook 设置、状态栏设置、对话 trace、yolo 不阻塞设置。当前适配 Claude Code / Codex / Grok / Kimi 四家，Windows / macOS / Linux（含 WSL）同一命令面。CLI 名 `oma`。

- `oma doctor`：agent 可用性只读体检，warn 与 block 分层，block 才退出 1
- `oma init`：按各家规则把 hook、skill、yolo 键落进项目，幂等合并，不写家目录注册
- `oma agents statusline`：四家状态栏写入面（starship 风格、支持用户级定制）
- `oma trace`：项目内四家 agent 对话历史只读检索
- `oma agents verify`：四家无头验收（hook 落盘加状态栏脚本直跑）

oma 不做编排（不拉会话、不发任务），不管 token 注入（密钥安全归 [ohmypwsh]，agent 二进制安装归姊妹工具 `ome`：`ome install claude`）。

[ohmypwsh]: https://github.com/raystyle/ohmypwsh

## 安装

预编译二进制覆盖三平台。装好后把 `oma`（Windows 为 `oma.exe`）放进 PATH 上的任一目录即可。

### Windows

```powershell
# 最新正式版
Invoke-WebRequest https://github.com/raystyle/ohmyagents-rs/releases/latest/download/oma-x86_64-pc-windows-msvc.zip -OutFile oma.zip
Expand-Archive oma.zip -DestinationPath $HOME\.oma\bin
Move-Item $HOME\.oma\bin\oma-x86_64-pc-windows-msvc\oma.exe $HOME\.oma\bin\
# 把 $HOME\.oma\bin 加进 PATH 后重开终端
oma --version
```

### macOS

```bash
curl -L https://github.com/raystyle/ohmyagents-rs/releases/latest/download/oma-aarch64-apple-darwin.tar.gz | tar xz
mkdir -p ~/.local/bin && mv oma-aarch64-apple-darwin/oma ~/.local/bin/
oma --version    # ~/.local/bin 需在 PATH
```

### Linux x86_64 与 WSL

```bash
curl -L https://github.com/raystyle/ohmyagents-rs/releases/latest/download/oma-x86_64-unknown-linux-gnu.tar.gz | tar xz
mkdir -p ~/.local/bin && mv oma-x86_64-unknown-linux-gnu/oma ~/.local/bin/
oma --version
```

### 源码安装

```bash
cargo install --git https://github.com/raystyle/ohmyagents-rs
```

### 前置与说明

- 状态栏运行时是 pwsh（PowerShell 7），全平台一致：装了才有状态栏，缺了只是不渲染，不影响其它命令
- 想跟开发滚动版：上面的资产 URL 把 `releases/latest/download/` 换成 `releases/download/dev/`；配了镜像环境可 `OMA_MIRROR=https://env.ohmygh.com oma self update` 走镜像
- oma 自管数据根是 `~/.oma`，不碰家目录注册

## 快速上手

```powershell
cd D:\my\proj          # 进你的项目，后续命令都不用再带路径
oma init               # 部署 hook / skill / yolo 键（幂等，重复跑安全）
oma agents statusline  # 配置四家状态栏
oma doctor             # 体检：有 block 级问题才退出 1
```

## 使用示例

全部命令支持 `--format kv|json|jsonl` 与 `--json` 信封输出。

### 部署到项目

```powershell
oma init                        # 全套：yolo 键加四家 hook/skill
oma init --yolo                 # 仅无阻塞键
oma init --yolo --pretrust      # 额外预写家目录信任库（四家）
oma init --project D:\my\proj   # 不进目录也能指定项目
```

### 状态栏

```powershell
oma agents statusline           # 四家全配（幂等）
oma agents statusline codex     # 只配一家
oma agents statusline --example # 用户级定制模板（~/.oma/statusline.toml）
```

定制三层：段落显隐与顺序（`segments`）、段内模板与图标（`[template]` / `[icons]`）、整脚本替换（`oma agents statusline --script <路径>`，`--builtin` 还原）。改完配置重跑一次 `oma agents statusline` 生效。

### 诊断与验收

```powershell
oma doctor                  # yolo / 信任 / 登录态 / hook 形态 / 状态栏 / CPU 指令集
oma agents                  # 列四家检测：装没装、来源与版本
oma agents verify           # 四家无头验收：hook 落盘加状态栏直跑
oma agents verify kimi --timeout 120   # 单家加超时
```

### 活性诊断

打真网关（llm.d3fend.cn）烧最小 token，与 doctor 的零网络体检分家；凭据读 agent 侧配置，也可用 `OMA_GATEWAY_URL` / `OMA_GATEWAY_KEY` 覆盖。

```powershell
oma diagnose cache                    # 全别名缓存命中矩阵（双连探测）
oma diagnose cache zy-gpt56sol-codex  # 只测指定别名
oma diagnose agents                   # 配置指向、别名在册、key 活性、thinking 上限
```

### 对话历史检索

```powershell
oma trace sessions          # 项目内各 agent 的会话
oma trace search "重构"      # 按正则检索四家的修改与意图
oma trace file src\main.rs  # 单文件轨迹：谁、何时、为何改的
oma trace agent claude      # 某家 agent 的操作块时间线
```

### 自维护

```powershell
oma self update             # 自更新（缺省 dev 滚动源）
oma self update --stable    # 走正式版
oma completions powershell  # 补全（bash / zsh / fish / powershell）
```

## 更多文档

- 命令手册细则：`docs\references\R002-常用命令与管理流程-从项目init到部署诊断.md`
- 需求与设计：`PRD.md` 与 `docs\` 各分册（`INDEX.md` 是总索引）

## 注意

`oma init` 的 yolo 旗标会关掉 agent 的审批与沙箱，只在自己信任的项目目录用。
