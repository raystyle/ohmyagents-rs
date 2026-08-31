# S011-Command-LineRust测试方法论与oma测试分层

> 2026-08-31。方案 P0005 的 POC 渐次转绿、产品子命令在即，用户点名研究测试规范（冒烟 / 集成 / 回归 / 验收等）。三个来源：底本《Command-Line Rust》（Ken Youens Clark，O'Reilly 2022，ISBN 978-1-098-10943-1，pdftotext 抽取精读）作地基；用户提供的 2026 社区主流实践综述作现代层；microsoft/rust-guidelines（浅克隆 commit `c1d2efc`，2026-08-19）作企业规范层，三源对照后裁剪到本仓。

## 背景

`oma` 目前只有模块内两个 `#[test]` 与手动跑的 POC examples。写 `spawn` / `send` / `cleanup` 产品命令前，需要定测试分层与规范，避免 POC 验收永远停在「人跑退出码」。

## 关键结论

1. 术语只是意图，方法对应上即可：**冒烟**要的是「活着」（exit 0 断言，第 1 章 `runs()`）；**集成**要的是「按用户方式整跑」（assert_cmd + predicates 断 stdout/stderr/退出码，第 1-2 章）；**回归**要的是「基线重跑」（`tests/expected/*.txt` 黄金文件，第 2-3 章）；**验收**要的是「对照 oracle」（`mk-outs.sh` 调原版工具生成期望输出）。书不使用这些专名，但每个意图都有现成方法。[实证: 2026-08-31 精读对应章节]
2. 书的两层（单元 / 集成）与社区三层（单元 / 集成 / 文档测试）不冲突：doctest 是官方第三类，书没展开、社区补上；地基完全一致（`#[cfg(test)]` + `tests/*.rs` 每文件独立 crate + 共享夹具放 `tests/common/mod.rs`）。[经验: 用户提供 2026 综述；官方约定属公开常识]
3. 书的实践在 2026 年的对应升级：`cargo test` 执行层升级为 cargo-nextest（进程隔离、超时、重试、JUnit）；黄金文件升级为 insta 快照 + review 流程；`time`/hyperfine 粗测之外补 criterion 微基准；门禁从「本地 cargo test」升级为 fmt / clippy -D warnings / test --locked / audit 的 CI 集合。[经验: 用户综述，未本机复核]
4. 三源在原则上完全收敛：测行为不测实现（MS 的 M-INTEGRATION-TESTS 明说「能集成就不单元」）、一个测试一件事、覆盖错误路径、随机与外部状态要确定化、性能与正确性分离。这些不变，变的只是工具。[实证: 三源并列对照，rust-guidelines commit c1d2efc]
5. MS 规范层补了两条前两源没有的硬规则：**测试设施必须 feature gate**（mock、安全检查旁路、假数据收进单一 `test-util` feature，M-TEST-UTIL）；**测试不得断言重言式**（M-TAUTOLOGICAL-TESTS：不得用被测同款逻辑复述期望值或镜像实现分支，否则按构造通过、纯噪声；点名这是 Agent 生成测试的高发病）。[实证: rust-guidelines 对应条目]
6. mock 策略两说调和：社区「少 mock 多真替身」与 MS「做 IO/系统调用的类型必须可 mock」（M-MOCKABLE-SYSCALLS，含文件、网络、时钟、熵源）不冲突——真依赖做集成路径，mock 口留给难触发的边界（故障注入、不可复现环境）。[推断: 两条并列读]
7. 本仓落地：`oma` 的无 daemon 子命令（check/agents/hook/init）可直接套地基全套；有 rmux 依赖的 POC 需加闸门与隔离，这是三源都没有的场景；现代层按阶段引入，不一步到位（AGENTS 写 Rust 规则：最少依赖）。[推断: 对照本仓 POC 已证的 Job Object 与 label 端点坑]

## 现状或实测

### 书的方法论清单

> 按章节

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

> 术语列只标意图；书列出处为上表章节，2026 列来自用户综述（社区主流，[经验]，未本机复核），取舍列为本仓裁剪。

| 意图 | 书 2022 | 社区 2026 | oma 现阶段取舍 |
| --- | --- | --- | --- |
| 冒烟（活着即可） | `runs()` 断 exit 0 | 同 | `oma --version` / `oma check --no-install` 退出码 |
| 单元（函数正确） | `#[cfg(test)]` 测纯函数 | 同 + rstest 参数化 | 用官方形态；rstest 暂缓（case 少，AGENTS 写 Rust 规则） |
| 集成（用户方式整跑） | tests/cli.rs + assert_cmd/predicates | 同；执行层换 cargo-nextest | tests/cli.rs 先行；nextest 待 daemon 类测试上量再引（超时/隔离价值在彼） |
| 文档示例 | 未展开 | doctest（`cargo test --doc`，nextest 不跑） | lib 公开 API 少；暂不做 |
| 回归（改动不破旧） | 黄金文件 + 全量重跑 | insta 快照 + `cargo insta review`，CI 禁静默改快照 | 黄金文件对齐 `poc.*` 标记行已够；status 输出复杂后再 insta |
| 不变量（任意输入成立） | 手写多 case | proptest 属性测试 | 版本串解析、TOML 往返是候选；后置 |
| 验收（对照外部标准） | 原版工具当 oracle 生成期望 | 同思路（snapshot 当人审金标准） | POC example（退出码 + 标记行）已是此形态 |
| 外部依赖隔离 | 真依赖（oracle 对比） | trait 注入 + mockall / wiremock；少 mock 多替身 | 本仓就是真 daemon 集成；不 mock（与两源原则一致） |
| 随机可测 | PRNG 种子注入 | 同 | pid+tag 命名已确定化，无需种子 |
| 平台差异 | `.windows` 期望 + `#[cfg]` | CI 多 OS 矩阵 | Linux/mac 委托后续仓库；CI 矩阵远期 |
| 环境稳定 | `set-test-perms.sh` fixture | tempfile / testcontainers / serial_test | WMI 起 daemon、临时项目目录（yolo POC 已此法）；并发抢资源时再看 serial_test |
| 性能 | `time` + hyperfine 参数化 | criterion 微基准为主，hyperfine 仍用于 CLI 对比 | 无当前需求，放目标外 |
| 质量门禁 | 本地 `cargo test` | fmt / clippy -D warnings / nextest --locked / audit 与 deny；llvm-cov + mutants 测「断言质量」 | 最小集：fmt + clippy + test --locked（本地脚本化）；CI 门禁待立项；覆盖率不进门禁 |

### MS 规范层的补充条目

> 书与社区综述没有、rust-guidelines 独有的裁决。[实证: commit c1d2efc 对应条目]

| 规则 | 内容 | oma 取舍 |
| --- | --- | --- |
| M-INTEGRATION-TESTS | 只碰公开 API 的测试放 `tests/`，不塞 `mod tests{}`；能集成就不单元 | 与既定方向一致；oma 的 lib 边界公开面（rmuxpoc 等）照此 |
| M-TEST-UTIL | mock、敏感数据检查、安全旁路、假数据全部收进单一 `test-util` feature | 产品命令期的桩 agent 注入、hook 事件伪造走此 feature；闸门 skip 不算旁路，不需要 |
| M-MOCKABLE-SYSCALLS | 做 IO/系统调用的类型必须可 mock（文件、网络、时钟、熵源）；库不做 ad-hoc IO | oma 的 rmux 端点保持可替换（WindowsPipe/UnixSocket 已是值类型）；时钟与随机若进产品路径，留注入口 |
| M-TAUTOLOGICAL-TESTS | 不断言重言式：不得复述实现逻辑或镜像分支；改断属性 | 对 AI 协作仓最高优先：写测试先自检「期望值是否来自被测同款逻辑」 |
| M-PANIC-ON-BUG / M-PANIC-MESSAGE | 契约违约 panic、可失败输入 Result；panic 消息带原因与实际值 | oma POC 风格是 `Result<String>` 一路到底；产品化时按此分岔（编程 bug panic，用户输入可失败走 Result） |

### 三源的原则收敛

对照后不变的部分，比工具更值得记：

1. **测行为不测实现**：书的「集成优先、用户视角」、社区「内部重构不应批量炸测试」、MS「能集成就不单元」三说同源。oma 的 POC 即行为验收，转回归时保持断言公开输出。
2. **一个测试一件事**：书用 helper 收敛重复，社区用可读命名（`login_locks_after_three_failures`）；本仓 `poc.*` 标记行即一行一事的输出侧体现。
3. **少 mock、多真替身；边界留 mock 口**：书拿原版工具当 oracle，社区推 tempfile / 内存库，oma 拿真 rmux daemon（POC 全程真依赖）；MS 补「非确定边界要可 mock」用于故障注入。mock 是手段不是默认。
4. **环境差异与全局状态**：书的 Testing Underground（只断稳定字段）与社区「nextest 会暴露靠同进程全局状态碰巧通过的测试」是同一件事的两面。daemon 类测试天然进程隔离，正好受益。
5. **性能与正确性分离**：书 biggie/hyperfine 分离，社区 criterion 放夜间；一致的「别让计时进断言」。
6. **锁文件**：社区补的 `--locked` 门禁，书未涉及；本仓 Cargo.lock 已入仓，CI 化后照做。

### 本仓现状与差距

- 已有：`src` 内模块级 `#[test]`（endpoint 形状、错误分类）；examples 为人工验收。无 `tests/` 目录、无 dev-dependencies、无黄金文件。[实证: 2026-08-31 查本仓 Cargo.toml 与目录]
- 书的依赖版本是 2022 年的（assert_cmd 1-2、predicates 2、clap 2.33）；本仓 clap 已是 4，引入时按 crates.io 现版复核。[经验: 书版权页与 Cargo.toml 示例；版本时效未复核]
- 社区综述中的量化断言（nextest 常见 1.5 到 3 倍加速、RustRover 2026.1 原生集成）未独立核实，按 [记忆] 待复核处理；工具存在性与官方三层约定属公开常识。

## 踩坑沉淀

| 坑 | 书的对策 | 对 oma 的启示 |
| --- | --- | --- |
| 期望输出含 owner/mtime 等机器差异字段 | 只断言稳定列 | 断言 `poc.*` 标记行与退出码，不断言时间戳/pid |
| 输出行序不稳定 | 双侧 sort 后比对 | session/pane 列表类输出先排序再比 |
| 随机行为不可断言 | PRNG 种子化 | oma 的随机命名（pid+tag）已确定化，无需额外种子 |
| 大文件拖慢常规测试 | 压力文件与断言分离 | rmux 压测（多 pane 大输出）单独 example，不进 cargo test |
| Windows 语义不同（symlink、权限） | 期望文件分平台 + `#[cfg]` 测试 | 已有「Linux/mac 委托后续仓库」口径，测试同法 |
| 测试靠同进程全局状态碰巧通过 | （书未涉及）换 nextest 后暴露，属测试设计问题 | daemon 类测试保持进程隔离；不写依赖环境变量的隐式共享 |
| 覆盖率高但断言弱 | （书未涉及）llvm-cov 配 cargo-mutants 看变异存活 | 断言写行为（标记行、退出码）而非「跑过了」；mutants 放夜间按需 |
| 重言式测试（Agent 高发） | MS：期望值来自被测同款逻辑、或镜像实现分支 | 写测试先自检：期望值必须来自独立来源（规范、黄金文件、属性），不得复述实现；为凑覆盖写的重言测试宁可删 |
| 测试设施泄漏进生产构建 | MS：mock/旁路/假数据不 feature gate 会进正式构建 | 产品命令期的桩注入收进 `test-util` feature；CI 按无此 feature 构建 |

## 待办

> 按阶段三段式引入，不一步到位（AGENTS 写 Rust 规则最少依赖；每段绿了再进下一段）。
> **2026-08-31 沉淀**：本研究的规则性结论已固化为 AGENTS 写测试规则「写测试时」与 `docs\references\R004-测试标准细则-分层断言与门禁流程.md`（细则事实性断言均标六态溯源回本文三源）；本节三段即细则第五节演进路线，后续以细则为准。

**第一段（本目标内可做，地基）**

1. 建 `tests/cli.rs` + `[dev-dependencies]`（assert_cmd、predicates，按 crates.io 现版复核），先给 `oma check --no-install`、`oma agents`、`oma hook` 写冒烟与负例（含 `dies_` 命名）。
2. 本地门禁脚本：`cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo test --locked`。
3. lib/bin 职责对照检查：本仓 `src/main.rs` 是否已是薄壳。
4. AI 写测试的自检规则（M-TAUTOLOGICAL-TESTS）：期望值来自规范或黄金文件，不得复述实现；先复现再固化。

**第二段（产品命令期，回归上量）**

4. POC examples 转回归：`tests/` 内以闸门（rmux pin 在否）决定跑或 skip，黄金文件对齐 `poc.*` 标记行。
5. MISTAKES 高频行（如 Job Object、`-S` 拒绝）各配一个回归测试。
6. daemon 类测试上量后引入 cargo-nextest（超时、重试、进程隔离；doctest 另跑 `cargo test --doc`）。
7. `oma status` 输出复杂后以 insta 快照管理期望，CI 用「快照必须已提交且一致」模式。

**第三段（CI 化与质量层，按需）**

8. GitHub Actions 门禁立项：fmt / clippy -D warnings / nextest --locked / audit（缓存 cargo 与 target）。
9. 依需求选配：proptest（解析往返）、llvm-cov（作回归观测不作门禁）、mutants（夜间）、多 OS 矩阵（Linux/mac 仓库就位后）。
10. criterion / hyperfine 基准与 fuzz / Miri：出现对应需求（性能 SLA、unsafe、不可信输入解析）再立项。[直觉: 先正确后快]
