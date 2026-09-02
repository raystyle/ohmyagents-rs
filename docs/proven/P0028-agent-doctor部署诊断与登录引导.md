# P0028-agent-doctor部署诊断与登录引导

> 2026-09-02 当日闭环（四端验收）。缘起：用户定调「agent 部署、管理、验收与诊断」核心轴，S026 grok/kimi OAuth 登录取证结项后落地项升格——doctor 从「无阻塞诊断」扩为一次性部署诊断面，登录引导补齐闭环。判据依据 `docs\research\S026-grok与kimi的OAuth登录流.md`（登录态）与 S025（状态栏矩阵）；当日同场完成仓库更名收口与四仓生态定调（R001 四仓生态节）。

## 方案

### 1. doctor warn 层与四类部署检查

> src\doctor.rs。

- **`Status::Warn` 层**：部署缺口（登录缺失、状态栏未配、会话死路）只诊断不进 `blocked()`——block 语义仍只对交互阻塞负责，契约测试钉死新四类检查永不 block。
- **登录态**：grok `~/.grok/auth.json` scope 键 + `key`/`refresh_token` 有凭据，过期看 `expires_at`（RFC3339，`time` crate 只开 parsing feature）缺省 `create_time + 30 天`兜底、提前 300s 视过期，过期带 refresh_token 判「自动刷新」warn；kimi credentials `hasToken`（只看 access_token 非空不看过期）加空串墓碑区分吊销与未登录。`login_state()` 开放 pub(crate) 供 login 复用。
- **hook 形态**：`hooks.form` 行——claude/grok JSON 注册判 bare/absolute/none，codex `.codex/hooks.json` 判 per-OS 字段占据（command=Unix / commandWindows=Windows，绝对路径是设计态），kimi n/a（S015 无项目级注册）。
- **状态栏**：四家用户级配置 oma bar 标记 + oma 数据根脚本在位 + pwsh 咨询（缺失追加提示不单独成行）。
- **会话健康**：`agent=oma check=session`——manifest 在才 rmux 只读探活（`list-sessions`），无 manifest 是合法部署前态出 info 行不误报；探活注入式设计（`alive: Option<bool>`）保测试无 rmux 依赖。

### 2. oma agents login 设备码引导

> src\login.rs。

- **形态修正 S026 原推断**：pane 加扫屏改为**子进程捕获**——两家登录流输出都是纯 eprintln / process.stderr.write、无 TTY 依赖（grok-build auth/device_code.rs 与 kimi-code login-flow.ts 源码实证），与 agents 探针同形态、免 rmux 依赖。
- **解析契约**：grok 提示行（open this URL / Then enter this code / Confirm this code）与值之间有 `eprintln!()` 空行——取下一**非空**行；kimi URL 与 code 同行尾缀（`device login: <url>` / `enter code: <code>`）。成功标记 `✓ Signed in` / `Logged in to`；失败行 kimi `Login cancelled|failed`、grok 靠非零退出码。
- **跨机 UX（用户定调）**：只出 `login.url=`（code URL）/`login.code=`/`login.hint` 干净三行，**不转发**原始 stderr（grok 试开浏览器的噪音不灌用户）——设备码流天生跨机，URL 加 code 拿到任何机器完成；失败 detail 失败行优先、无则尾部三行诊断。
- **成功判据不单信标记**：退出 0 **且** doctor 登录态判据（落盘凭据）过才 `login.ok=true`；超时杀进程（缺省 600s、0 不限时）。

### 3. is_ours 调用操作符根修

> src\deploy.rs。

- codex Windows 侧 `& "exe" hook` 形态首 token 是 PowerShell 调用操作符 `&`，`is_ours` 漏判——doctor 跨环境漏检 Windows 字段、deploy 换 exe 路径后旧条目被当外来者重复追加。剥前导 `&` 后按首 token 判 stem，回归测试钉六形态。由 codex per-OS 测试当场暴露（测试即取证）。

## 过程

- 黄金样例测试抓真坑两处：grok 空行分隔（首版「取下一行」当场败）；`is_ours` 的 `&` 盲区。
- WSL 双半程实证：失败路径（grok 轮询遇 TLS 断连：失败行捕获、`login_state=warn?` 转述、exit 1）与成功路径（用户定调跨机 UX 后重跑，在另一台机器完成授权，`login.ok=true` 带 scope 与 expires_at，doctor 登录态翻绿）。
- 四端验收：本机全 ok；WSL 补登录翻绿；lan-win release 二进制直拷 `D:\ohmyenv\cargo\bin\oma.exe`（同架构）；lan-mac `git archive` 源码包 scp 加 `cargo install --path`——**未推远端也能拿当日代码做远程验收**。两端裸验收目录部署缺口全 warn 无误报、session 无 manifest 不误报、`doctor.blocked=true` 来自信任面属预期。
- grok 登录态四端四态全数活体实证：本机存活 / WSL 缺失后补齐 / lan-win 缺失 / lan-mac **过期带 refresh_token**（第三分支活捉）。
- 同日事件：会话中 cargo DLL_NOT_FOUND 根因 = ohmyenv-rs 侧 `ome install rust` 触发 `rustup update stable` 就地滚动 1.97.1→1.98.0；本仓 rustfmt 1.9.0 零漂移、clippy 新 lint 未命中、110+12 全绿——零改动适配。

## 钩子

- 研究依据：S026（登录流与判据）、S025（状态栏矩阵）、S015（hook 形态）、S021（caps 段）。
- 命令面：`oma doctor`（R002 无阻塞诊断行）、`oma agents login <grok|kimi>`（R002 设备码登录引导行）。
- 测试：105→110 lib 加 12 集成全绿；登录黄金样例来自源码取证非实现镜像（R004）。
- 遗留：kimi 真登录待需时验；lan-win / lan-mac 的 init 与状态栏配置未做（验收只到 doctor 判读）。
