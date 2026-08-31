# M104-rmux安装与CLI调用错误

> 关键词：安装、exe、libexec、版本、-V、-S、-L、pipe、cmd()、逃生舱。编号全局递增（MNNN），规则见 `MISTAKES.md`。

| 编号 | 错误现象 | 根因 | 正确处理 | 首次踩 |
| --- | --- | --- | --- | --- |
| M006 | `rmux --version` 非 0 且打印 usage | tmux 风格，版本在 `-V` | 读版本用 `rmux -V`（`rmux 0.10.0`） | 2026-08-29 |
| M007 | 只拷一个 `rmux.exe` | tiny CLI 找不到 libexec helper | 必须保留 `rmux` + `libexec/rmux/rmux` + `rmux-daemon` | 2026-08-29 |
| M016 | Windows `-S` 拒自定义 pipe 名 | **订正**：`-S` 对一切形态无条件拒绝（含 `\\.\pipe\rmux-...` 前缀与 SID 派生全名），报错文案误导 | CLI 专用端点只用 `-L <label>`（pipe 名 `\\.\pipe\rmux-S-<SID>-il-medium-<label>`）；`\\.\pipe\rmux-...` 形只用于 SDK `WindowsPipe` 与 `--__internal-daemon` | 2026-08-29 |
| M020 | SDK `Rmux::cmd()` 在 Windows 必败 | cmd() 给 CLI 注入 `-S <pipe>`，被无条件拒绝 | paste 等命令自 spawn `rmux.exe -L <label> ...`；SDK 只走协议 API（session/pane/send_key/expect） | 2026-08-31 |
