# S026-grok与kimi的OAuth登录流

> 2026-09-02。用户定调：grok 与 kimi 非 api-key 配置、需 OAuth 登录，oma 要能向用户呈现登录 URL 与 auth code 交互，并检测登录态。一手源码取证（浅克隆 HEAD）。

## 需求

- 核实两家登录流形态、入口命令与用户可见输出、凭据落盘位置与格式、无头（无浏览器）路径、刷新与过期。
- 为 `oma agents login`（引导登录）与 agent doctor 登录态检测供依据。

## 关键结论

### 1. grok-build

[实证: 浅克隆 crates/codegen/xai-grok-shell/src/auth/*，2026-09-02]

- **流程**：OAuth 2.1，默认 loopback（授权码+PKCE+本地回调），可切 RFC 8628 设备码（`grok login --device-code`；旗标 > `GROK_LOGIN_DEVICE_FLOW` env > `[auth] login_device_flow` 配置）。issuer `https://auth.x.ai`。
- **用户可见输出**（stderr）：`To sign in, open this URL in your browser: <url>` + `Then enter this code: <user_code>`——URL+code 可复制到任何机器的浏览器，**设备码流天生无头友好**。
- **凭据落盘**：`~/.grok/auth.json`（`GROK_AUTH_PATH` 可覆盖）；scope 键 → {key, refresh_token, expires_at(RFC3339), email,...} 的 map。无 `expires_at` 时按 `create_time + 30 天`兜底；提前 300s 视过期（env 可调）。
- **刷新**：自动（AuthManager 静默 refresh，flock 防并发）；替代路径 `XAI_API_KEY` env。
- **登录态检测**（doctor 可用）：文件存在 + 目标 scope 键存在 + 未过期——纯文件判断，无需起进程。

### 2. kimi-code

[实证: 浅克隆 packages/oauth/src/*、apps/kimi-code/src/cli/sub/login*.ts，2026-09-02]

- **流程**：**仅设备码流**（RFC 8628，源码明言）；host 双区 `https://auth.kimi.com`（mainland-cn）/ `auth.kimi.ai`（global），client_id 两区共用。
- **用户可见输出**（stderr，浏览器拉起前就打）：`Opening browser for Kimi device login: <url>` + `enter code: <userCode>`——无头安全（URL+code 先于浏览器打印）。
- **凭据落盘**：`~/.kimi-code/credentials/kimi-code.json`（0600）：{access_token, refresh_token, expires_at(Unix 秒), scope,...}。
- **刷新**：自动（动态阈值 min(300s, expiresIn/2)，单飞合并 + 跨进程锁）；401/403 写空串墓碑（吊销态 ≠ 未登录）。
- **登录态检测**：文件存在且 `access_token` 非空（`hasToken()` 不看过期，过期与否读 `expires_at` 自比）。

### 3. oma 落点

- 两家都是「URL + user_code 落 stderr」的可复制形态：oma 在 pane 里跑 `grok login --device-code` / `kimi login`，用扫屏原语捕获 URL+code 转发给用户（`spawn.alert=` 同型通道），完成后再扫 `✓ Signed in` / `Logged in` 确认。[推断: 基于两家输出契约]
- 登录态检测纯文件化，直接进 agent doctor 检查项。
- 差异：grok 有 API key 替代路径；kimi 设备码是唯一 OAuth 形态。

## 待办

- doctor 登录态检查行（grok auth.json / kimi credentials json）：已落地（`oma doctor` 出 `check=login` 行，warn 不进 blocked 汇总，2026-09-02）
- `oma agents login [名]`：pane 内起登录命令、扫屏转发 URL+code、确认成功——独立切片待立项

## 事实源

| 类型 | 定位 | 日期 | 提供 |
| --- | --- | --- | --- |
| git | grok-build 浅克隆（auth/{config,device_code,flow,model,manager,oidc/*}） | 2026-09-02 | 双传输流、auth.json 格式、无头路径、刷新 |
| git | kimi-code 浅克隆（packages/oauth、cli/sub/login*） | 2026-09-02 | 设备码唯一流、credentials 落盘、双区 host |
