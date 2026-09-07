# P0032：D11 状态栏工具链段扩展 zig go cpp

> 2026-09-07 当日闭环。缘起：状态栏 projKind 只认 rust / node / python。用户点名扩展 zig / golang / cpp。图标先 cmap 实证。

## 方案

1. **先到先得**：探测循环 rust / node / python 之后才认 `build.zig`、`go.mod`、`CMakeLists.txt` 或 `meson.build`，不抢现役三态。
2. **包版本**：zig 读 `build.zig.zon` `.version`；cpp 读 cmake `project(... VERSION)` 或 meson `version:`（单双引号）；go 无包版本段。
3. **工具链仅 nerd**：`zig version` / `go version` / `c++` 或 `g++` 或 `clang++ --version`。Grok ASCII 路径跳过子进程（M046）。
4. **图标**：seti-zig U+E6A9、seti-go U+E627、seti-cpp U+E646（CaskaydiaCove 与 0xProto cmap 均有映射；cmap 有映射不等于有字形）。

## 验收

- 单测钉探测标记、三枚码位、Cargo.toml 先于 build.zig。
- `oma agents statusline` 重放共享 ps1。
- 本机临时目录实跑：zig 包版本加工具链、go 仅工具链、cmake/meson 包版本、Grok 无工具链、Cargo.toml 压过 build.zig。

## 实施过程与经验

- 图标不猜：本机两字体 cmap 2026-09-07 才落码位。
- meson `version:` 既有单引号也有双引号，只匹配一种会漏。
- 本机无 `c++`/`g++`/`clang++` 时 cmake 仍出包版本、工具链静默缺段，符合既有 rustc 失败不致命口径。
- 临时目录 git 旗标会沿父级仓库冒出，与 D11 探测无关。

## 钩子

- 代码：`src\statusline.rs` 共享 ps1。
- 研究：S025 事实源补 cmap 行。
- 遗留：cpp 工具链本机未装编译器，未实跑图标段；Grok 仍跳过工具链。
