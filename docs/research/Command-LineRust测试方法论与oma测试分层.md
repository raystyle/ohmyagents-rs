# Command-LineRust测试方法论与oma测试分层

> 2026-08-31。方案 0005 的 POC 渐次转绿、产品子命令在即，用户点名研究测试规范（冒烟 / 集成 / 回归 / 验收等），底本为《Command-Line Rust》（Ken Youens Clark，O'Reilly 2022，ISBN 978-1-098-10943-1）。全文经 pdftotext 抽取后按章节精读。

## 背景

`oma` 目前只有模块内两个 `#[test]` 与手动跑的 POC examples。写 `spawn` / `send` / `cleanup` 产品命令前，需要定测试分层与规范，避免 POC 验收永远停在「人跑退出码」。

## 关键结论

1. 术语只是意图，方法对应上即可：**冒烟**要的是「活着」（exit 0 断言，第 1 章 `runs()`）；**集成**要的是「按用户方式整跑」（assert_cmd + predicates 断 stdout/stderr/退出码，第 1-2 章）；**回归**要的是「基线重跑」（`tests/expected/*.txt` 黄金文件，第 2-3 章）；**验收**要的是「对照 oracle」（`mk-outs.sh` 调原版工具生成期望输出）。书不使用这些专名，但每个意图都有现成方法。[实证: 2026-08-31 精读对应章节]
2. 分层只有两层：单元（inside-out，`#[cfg(test)]` 测模块内函数）与集成（outside-out，`tests/cli.rs` 以用户视角跑整个二进制）。[实证: 第 1、4 章]
3. 对环境差异的裁决：只断言稳定字段（文件名、权限、大小），放过 owner、mtime、目录大小、列宽等因机器而异的字段。[实证: 第 14 章 Notes from the Testing Underground]
4. 随机可测靠 PRNG 种子注入：`--seed` 既是用户参数也是测试口。[实证: 第 12 章]
5. 性能验证与正确性测试分离：大文件由生成器造（biggie 百万行），基准用 hyperfine 参数化对比，不用 cargo test 断言时间。[实证: 第 11 章]
6. 本仓落地：`oma` 的无 daemon 子命令（check/agents/hook/init）可直接套书的全套；有 rmux 依赖的 POC 需加闸门与隔离，这是书没有的场景。[推断: 对照本仓 POC 已证的 Job Object 与 label 端点坑]

## 现状或实测

### 书的方法论清单（按章节）

| 主题 | 书中做法 | 出处 |
| --- | --- | --- |
| 项目结构 | `tests/` 与 `src/` 平行；`tests/cli.rs` 为集成入口 | 第 1 章 |
| 冒烟 | `Command::cargo_bin(PRG).assert().success()`，只看活没活 | 第 1 章 |
| 退出值 | 0 成功、1-255 失败；正例 `success()`、负例 `failure()` 成对写 | 第 1 章 |
| 依赖 | `[dev-dependencies]`：assert_cmd（找 crate 内二进制）、predicates（模糊匹配） | 第 1-2 章 |
| 负例 | 无参数应失败并打 USAGE：`.failure().stderr(predicate::str::contains("USAGE"))`；命名带 `dies_`，`cargo test dies` 过滤 | 第 2 章 |
| 黄金文件 | `tests/inputs/` 输入 + `tests/expected/` 期望；`mk-outs.sh` 用原版工具生成期望 | 第 2-3 章 |
| 错误传播 | `type TestResult = Result<(), Box<dyn std::error::Error>>`，测试体用 `?` 代替 `unwrap` | 第 2 章 |
| TDD | 第 3 章起测试先行：红、绿、重构；先跑全红再写实现 | 第 3 章 |
| 可测结构 | `src/lib.rs` 放 `run()/get_args()`，`src/main.rs` 只留薄壳调 `run()` | 第 3 章 |
| 单元测试 | `#[cfg(test)]` 紧跟被测函数；`parse_positive_int` 正/负/零三态断言 | 第 4 章 |
| 随机输入 | `gen_bad_file()` 随机生成不存在的文件名测警告路径 | 第 3 章 |
| 平台分支 | 期望文件成对（`name_a.txt` 与 `name_a.txt.windows`），`#[cfg(windows)]` 版 `format_file_name` 选文件；`#[cfg(not(windows))]` 只跑 Unix 专属（chmod 000 不可读目录） | 第 7 章 |
| 无序输出 | 两侧都按行 split、去空、sort 后 `assert_eq!` | 第 7 章 |
| 环境差异 | 长列表只查权限/大小/文件名；目录大小置空串跳过；`set-test-perms.sh` 在测试外固定权限 fixture | 第 14 章 |
| 辅助函数 | `run(args, expected_file)` / `run_stdin(input, args, expected)` / `run_long` / `dir_long` 收敛重复 | 第 3、14 章 |
| 常量收敛 | 测试文件顶部 `const PRG` / `const FOX` 等 ALL_CAPS 路径常量 | 第 3 章 |
| 随机可测 | PRNG 种子化：固定 `--seed` 得可复现选择 | 第 12 章 |
| 压力输入 | biggie 生成百万行大文件，只给压力与基准用，不进常规断言 | 第 11 章 |
| 基准 | `time` 粗测 + hyperfine `-L prg a,b '{prg} args'` 参数化对比；release 构建下测 | 第 11 章 |
| 并行语义 | 测试默认乱序并行；要顺序用 `cargo test -- --test-threads=1` | 第 1 章 |
| 失败可读性 | assert_cmd 失败输出 expected/actual diff + 命令 + 退出码 + 双流，「读失败输出是技能」 | 第 1 章 |

### 意图与方法的映射

> 术语列只标意图；方法列出处均为上表章节。

| 意图 | 方法 | oma 对应 |
| --- | --- | --- |
| 冒烟（活着即可） | `runs()` 断 exit 0 | `oma --version` / `oma check --no-install` 退出码 |
| 单元（函数正确） | `#[cfg(test)]` 测纯函数 | `rmuxpoc` 既有两测；后续 `parse_*` / 布局计算 |
| 集成（用户方式整跑） | assert_cmd 跑二进制断 stdout/stderr | `oma agents` / `oma hook`（stdin 喂事件断 state 文件） |
| 回归（改动不破旧） | 黄金文件 + 全量重跑 | `tests/expected/`；MISTAKES 每行配一个测试 |
| 验收（对照外部标准） | 原版工具当 oracle 生成期望 | POC example 本身（退出码 + `poc.*` 标记行即验收输出） |
| 平台差异 | `.windows` 后缀期望 + `#[cfg]` | 本仓 Linux/mac 委托后续仓库，同机制可用 |
| 环境稳定 | `set-test-perms.sh` fixture | WMI 起 daemon、临时项目目录（yolo POC 已此法） |

### 本仓现状与差距

- 已有：`src` 内模块级 `#[test]`（endpoint 形状、错误分类）；examples 为人工验收。无 `tests/` 目录、无 dev-dependencies、无黄金文件。[实证: 2026-08-31 查本仓 Cargo.toml 与目录]
- 书的依赖版本是 2022 年的（assert_cmd 1-2、predicates 2、clap 2.33）；本仓 clap 已是 4，引入时按 crates.io 现版复核。[经验: 书版权页与 Cargo.toml 示例；版本时效未复核]

## 踩坑沉淀

| 坑 | 书的对策 | 对 oma 的启示 |
| --- | --- | --- |
| 期望输出含 owner/mtime 等机器差异字段 | 只断言稳定列 | 断言 `poc.*` 标记行与退出码，不断言时间戳/pid |
| 输出行序不稳定 | 双侧 sort 后比对 | session/pane 列表类输出先排序再比 |
| 随机行为不可断言 | PRNG 种子化 | oma 的随机命名（pid+tag）已确定化，无需额外种子 |
| 大文件拖慢常规测试 | 压力文件与断言分离 | rmux 压测（多 pane 大输出）单独 example，不进 cargo test |
| Windows 语义不同（symlink、权限） | 期望文件分平台 + `#[cfg]` 测试 | 已有「Linux/mac 委托后续仓库」口径，测试同法 |

## 待办

1. 建 `tests/cli.rs` + `[dev-dependencies]`（assert_cmd、predicates，按现版复核），先给 `oma check --no-install`、`oma agents`、`oma hook` 写冒烟与负例。
2. POC examples 转回归：`tests/` 内以闸门（rmux pin 在否）决定跑或 skip，黄金文件对齐 `poc.*` 标记行。
3. MISTAKES 高频行（如 Job Object、`-S` 拒绝）各配一个回归测试。
4. lib/bin 职责对照检查：本仓 `src/main.rs` 是否已是薄壳。
5. 引入 hyperfine 基准放本目标之外（性能无当前需求）。[直觉: 先正确后快]
