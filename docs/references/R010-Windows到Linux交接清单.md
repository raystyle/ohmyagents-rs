# Windows 到 Linux 交接清单

> WSL/Linux 侧开发 oma 的**开工读本**：产品现状、已平台化的部分、Linux 特有欠账、关键文件与坑索引。Windows 侧 2026-09-01 收口（48+ 提交当日，测试 80+10 全绿）；用户定调切 WSL Linux 开发。

## 一、产品现状一句话

oma 是通用智能体多路复用任务编排器：rmux 后端上把 claude/codex/grok/kimi 编进项目会话，三通道（CLI/HTTP/MCP）编排，看板可视化，任务目录协议（`oma task`）带产物等待。**全部命令已在 Windows 实测落地**；Linux/mac 侧资产与代码路径就绪、待环境切换验收。

## 二、已平台化的部分：Linux 直接受益

- `project_slug` 纯词法归一：相对挂 cwd、清 `.`/`..`；小写折叠仅 Windows（Unix 大小写敏感保留）
- `agent_alive` 的 stub 进程名：Windows 记 pwsh、非 Windows 记 sh（`interactive_shell_argv` 同源）
- stub 形态单测按 `cfg!(windows)` 分支断言
- `process_names` 非 Windows 走 `ps`（Windows 走 pwsh+CIM）
- 指令集 SIGILL 检测阶梯研究已备：`docs\research\S021`（Bun AVX/AVX2、Rust AVX-512 案例，四级检测，oma 探针落点）；Windows 侧 caps 模块已落地可参照
- catalog 两层 pin、渠道序（github 主 CDN 兜底）与 sha256 信任锚是平台无关设计；Linux 资产形态在 `catalog`/`src\install.rs` 已预留

## 三、Linux 特有欠账：WSL 开工首查

1. **daemon 启动路径**：`ensure_label_daemon`（`src\rmuxpoc.rs`）Windows 用 WMI 起进程（默认 cwd=System32 是 M031/M040 家族的根因）；Linux 侧确认 rmux 的 daemon 拉起形态（fork/exec 或 CLI 自举），`wmi_new_session` 需要 Linux 等价或绕开
2. **rmux 本体**：oma 自管根装的是 Windows 资产；Linux 取 rmux 0.10.0 对应包（catalog 已 pin sha），验证 pipe 命名（`\\.\pipe\` → unix socket）在 SDK 的 endpoint_from_pipe 是否分叉
3. **agent 四家 Linux 安装**：官方安装脚本形态（S017 已逐家实证过渠道反转），Linux 资产名/解包/安装目录在 `oma agents install` 的 leaf 找二进制逻辑待真机验收
4. **后台进程形态**：serve daemon 的 `CREATE_NO_WINDOW` 是 Windows 分支；Linux 用 `setsid`/`nohup` 等价（`src\servectl.rs` 已有 `#[cfg]` 骨架）
5. **探针与探活**：`pid_alive` 的 FFI OpenProcess 是 Windows；Linux 走 `kill -0`（分支已在）
6. **终端分类器**：`detect_terminal_state` 的 marker 集合按四家 TUI 实测——Linux 下 TUI 输出可能有差（回车/颜色码），`oma status` 真机过一遍

## 四、关键文件与坑索引

- 编排核心：`src\orch.rs`（reconcile 三态/精确集合/先补后收、settle 白名单与窗口循环、send 三段式与开始确认、布局按路数 relayout）
- 任务协议：`src\task.rs`（prompt.md/output.md/DONE 三件、占位互斥、空产物报错）
- 传输：`src\api.rs`（信封与 *_locked/*_finalize 锁内外拆分）、`src\server.rs`（全局 Host 闸、gate）、`src\mcp.rs`（gate、九 tools）
- 坑速查：M031/M040（daemon cwd 与会话保命序）、M038/M039（settle 重复按与 C-c 守卫）、M106（进程与 daemon 启动族）、`S023`（Windows 进程模型）、`S005`（drive 铁律）
- 工作流：`.tools\review-round.py`（agent 轮询接力 review，FINDINGS 契约）、`KNOWN-WONTFIX.md`（已拍板不修清单）、任务收件人模式（SKILL.md）
- 测试：`cargo test`（75+10 无 feature）/ `--features server,mcp`（80+10）；隔离目录 `CARGO_TARGET_DIR=target-test`；rmux 依赖的测试按闸门 skip

## 五、开工顺序建议

1. WSL 里 clone 本仓 → `cargo test`（应绿——平台无关层）→ `cargo build --features server,mcp`
2. `oma check`：Linux 资产下载与安装真机验收（第三节 2）
3. `oma spawn --stub`：daemon 启动路径与 stub 判活（第三节 1）——单路全屏形态即可验证
4. `oma agents install` 四家 Linux 形态
5. 四路真身 + settle 白名单真机过（信任屏 marker 可能差）
6. 复用 `.tools\review-round.py` 接力 review（Linux 侧同样收敛到 FINDINGS=0）

## 六、Windows 侧遗留的已拍板事项

- KNOWN-WONTFIX 清单（`.ohmyagents\reviews\`，22 条取舍）跨平台有效，Linux 侧不要复修
- 看板前端（`docs\web\share-src` 构建）平台无关；serve 只绑 127.0.0.1 与 Host 回环闸同理
- review 接力工作流产物在 `.ohmyagents\reviews\relay\`（7 棒），可续棒
