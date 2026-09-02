# S031-密钥一钥两密文存储与四shell懒注入接管

> 2026-09-02 用户定调两条合一：①参考 D:\ohmycloud 的 app.key、identity.enc、vault.yaml（D20「一钥两密文」）方式保存 deepseek 与智谱密钥；②参考 D:\ohmypwsh 历史的懒注入（pwsh/bash/zsh/nushell profile），彻底平移接管 agent 的密钥管理。四仓分工下密钥**主权在 ohmypwsh**（R001），oma 接管的是 **agent 会话的存储与投递面**——本机 agent 键的存取不再依赖 ohmypwsh 载体。

## ohmycloud 一钥两密文

> D20 应用标准。

[实证: src\lib\keystore.ts 逐行读 + ~/.omcf 实体文件结构核对]

- **app.key**：自生成 32B（64 hex）落 `~/.omcf/app.key`，0600、tmp+rename 原子写；`OMCF_VAULT_KEY` env 显式覆盖优先于文件。
- **identity.enc**：age 身份私钥用 app.key 做 **AES-256-GCM** 二次加密，单行标记 `omcf:v1:<base64(iv|tag|cipher)>`（iv 12B、tag 16B、body）；GCM 认证——篡改或密钥不符抛错且错误不携密文。
- **vault.yaml**：SOPS 制密文（逐值 `ENC[AES256_GCM,...]`，sops 段带 recipient），由 age 身份解。**运行时解密链全程内存：app.key → identity.enc → 身份 → vault**。
- **identity.meta.json**：source（标准 age 钥匙链路径）、recipient（公钥可明文）、createdAt。
- 身份走 **SOPS 标准 age 钥匙链**（`SOPS_AGE_KEY_FILE` → `%APPDATA%\sops\age\keys.txt` / `~/.config/sops/age/keys.txt` → `~/.config/age/keys.txt`），与 ohmypwsh、remotex 同源——**身份不自建**（R003 裁决）。

## ohmypwsh 懒注入

> 历史形态取证。

[实证: scripts\profile-pwsh.ps1 与 profile-posix.ps1 逐行读、R001 命令行]

- **profile-pwsh.ps1**（71 行）：注入 `$PROFILE.CurrentUserAllHosts`（set-pwsh-profile.ps1 部署，标志行包裹幂等）。交互 shell 启动时 `sops -d` 现场（`SOPS_AGE_KEY_FILE=~/.config/age/keys.txt`）解 `.secrets\deepseek-key.yaml` 与 `zhipu-key.yaml`，base64 解码写**当前会话** `$env:DEEPSEEK_API_KEY` / `$env:ANTHROPIC_AUTH_TOKEN` + `ANTHROPIC_BASE_URL=bigmodel`——**明文不常驻注册表**。
- **profile-posix.ps1** → `env.sh`（bash/zsh 通用），WSL 侧 `~/.bashrc.d/ohmyenv-secrets.sh`；nushell 走继承。
- 值以 **base64 入 yaml**（避免 yaml 转义与引号坑）。
- 惰性注入只在**交互 shell 启动**时跑；工具 shell / 后台进程是裸环境不继承（S006 记档）——这正是 oma 会话需要的形态：agent pane 由 shell 拉起时吃到，后台不泄露。

## oma 接管设计

落 oma 数据根 `~/.ohmyagents/`（与 `.omcf` 同构，oma 自己的一套）：

```text
~/.ohmyagents/
  app.key        oma 自生成 32B（oma:v1 链的根）
  identity.enc   age 身份 AES-256-GCM 包裹（oma:v1:<base64>）
  identity.meta.json
  secrets.yaml   SOPS 制密文（sops 二进制加工，age 后端）——deepseek/zhipu 键值 base64
```

- **零自写协议**：AES-256-GCM 用 RustCrypto `aes-gcm` crate（主流稳定）；vault 的加解密**走 sops 二进制**（机器已装，ohmyenv 域）——SOPS 格式不重实现。身份用 `SOPS_AGE_KEY` env 直接传身份内容给 sops（免临时文件，keystore 链内存态）。
- **命令面**（`oma agents secrets` 子树）：
  - `init`：确保 app.key 生成 + 标准钥匙链身份包裹进 identity.enc + meta 落盘；空 vault 不预建（set 时建）。
  - `set <KEY>`：值走 **stdin**（纪律：秘密不进 argv，对齐 ohmycloud D20 纪律面）；base64 后 sops 写入 secrets.yaml。
  - `env [--shell pwsh|bash|zsh|nu]`：解链（app.key → identity → SOPS_AGE_KEY → `sops -d`）出对应 shell 的 export 行——profile 块的唯一后端。
  - `inject`：四 shell profile 写标志行包裹的加载块（pwsh `$PROFILE`、`~/.bashrc`、`~/.zshrc`、nushell `env.nu`），幂等、可重复执行。
- **投递形态**：交互 shell 启动 → 块内跑 `oma agents secrets env --shell <sh>` → 会话 env 即得 DEEPSEEK_API_KEY / ANTHROPIC_AUTH_TOKEN / ANTHROPIC_BASE_URL；oma spawn 的 pane 由 shell 拉起自然继承（ohmypwsh 同款语义）。
- **secretguard 联动**：实值通道后续可把 vault 解出的值并入防线 2（待办，不进本切片）。
- **providers.toml 关系**：现明文过渡形态保留；后续可让 providers env 值引用 vault 键（间接层）——待办不进本切片。

## 纪律

> 对齐 ohmycloud D20 与 ohmypwsh。

- 盘上恒密文（app.key 除外——它是链根，0600 文件态）；秘密不进 argv；输出 redacted 只报「已设置/来源」。
- 原子写 0600；解密失败不泄漏密文内容。

## 落地记

- `src\secrets.rs` + `oma agents secrets init|set|env|inject|status` 五命令落地；134+13 全绿。
- 实机部署即验收：本机 init（真身份 `~/.config/age/keys.txt` 包裹）；ohmypwsh 密文平移两键（`sops -d | base64 -d | oma set` 管道直传，不过 argv 不过记录）加 BASE_URL；三 shell env 输出掩码核对；`inject` 四 profile 齐写；pwsh `AK=True DS=True`、bash `AK=set DS=set URL=bigmodel`——端到端全通，与 ohmypwsh 历史形态同语义。
- oma 数据根自管（用户定调：oma 用自己的应用数据存用户目录下）——全部落 `~/.ohmyagents`，不碰 `.omcf`。
- 新 oma 已 `cargo install --path` 就位：secretguard 与 secrets 同批激活，本仓即刻 dogfood。
- 待办：providers.toml env 值引用 vault 键的间接层；secretguard 实值通道并入 vault 解出值。
