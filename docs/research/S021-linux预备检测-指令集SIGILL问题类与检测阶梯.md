# S021：linux 预备检测：指令集 SIGILL 问题类与检测阶梯

- 日期：2026-08-31
- 关联：`P0012`（Linux/mac 接管，用户定调排后——本文是切换环境前的预备检测研究）；`S017`（四家 agent 渠道与制品形态）
- 研究法：用户提供问题类框架（机理与谱系），公开案例经 web 检索核实到 issue 级；检测阶梯的 shell 侧命令为 Linux 通用口径，oma 侧落点为设计口径（本机 Windows 无法实跑 Linux 面）

## 一、为什么研究

P0012 切到 Linux 后最可能的「所有 agent 都跑不起来」形态：预编译原生二进制在编译期启用了 AVX-512（或 AVX2），目标机 CPU / 虚拟机 / 云主机并不真正具备，进程一启动就 SIGILL（非法指令）。rmux / herdr 这类 multiplexer 本身一般不要求 AVX-512——崩的是它挂进 pane 的各家 agent，于是症状表现为「编排器活着、所有路全灭」。切换环境前把检测与缓解钉死，P0012 验收时这个坑一次过。

## 二、问题机理

> 谱系与机理为用户提供框架加文献核实。[实证: 文献]

- 预编译 native 二进制（Bun 运行时 / Rust 默认 target / C++ 推理库）直接含 EVEX / ZMM 指令；启动或加载模型走 SIMD 热路径，第一条非法指令即整个进程退出（SIGILL，signal 4）。
- 三层不匹配：
  1. CPU 没有 avx2 / avx512f（老 CPU 或阉割虚拟 CPU）。
  2. CPUID「看起来有 AVX-512」但 OS 未开放相应 xsave 状态（XCR0 未覆盖 opmask / ZMM）——云主机（Cloud Run、部分 KVM）常见：标志在、运行照崩。
  3. hypervisor 层暴露标志但宿主实际关闭 ZMM——guest 内一切静态检查都可能被骗过。
- CPU 谱系：Intel 消费级自 Alder Lake 起基本关闭 AVX-512（13 代 Raptor Lake 同样无）；AMD 自 Zen 4 才原生完整支持，Zen 3 及更早没有；发布包按「新机器」编出且不做运行时降级，就命中上述三层之一。

## 三、公开案例

> 检索核实到 issue/论坛级。[实证: 文献]

| 案例 | 事实 | 缓解 |
| --- | --- | --- |
| [openai/codex 17410](https://github.com/openai/codex/issues/17410) | VS Code 扩展 bundled `codex.exe` 要求 AVX-512，在 13 代 i9-13900H 与 AMD Zen 3 上 illegal-instruction 崩 | 换非原生制品 |
| [openai/codex 25367](https://github.com/openai/codex/issues/25367) | CLI 0.135.0 启动即 STATUS_ILLEGAL_INSTRUCTION | 降版 `@openai/codex@0.134.0` 同机即起 |
| [anthropics/claude-code 50466](https://github.com/anthropics/claude-code/issues/50466)（同族 [50384](https://github.com/anthropics/claude-code/issues/50384)、[56850](https://github.com/anthropics/claude-code/issues/56850)） | 2.1.112 起原生二进制回归要求 AVX/AVX2，Ivy Bridge 等 CPU 上 SIGILL（Bun 运行时） | npm 版（Node 跑）或钉旧版；旧 Node 也要挑不要求 AVX 的 |
| [Google Antigravity CLI](https://discuss.ai.google.dev/t/antigravity-cli-fails-with-illegal-instruction-sigill-on-legacy-cpus-lacking-avx/147357) | 老 x86_64 无 AVX 即 SIGILL | 同族缓解 |
| [opencode 13282](https://github.com/anomalyco/opencode/issues/13282) | Xeon E3 v2 上 SIGILL，Bun 系 | 换 Node 形态 |
| llama.cpp / ggml / vLLM CPU | 默认选 AVX-512 路径后同样非法指令 | 自编译关 native / 选 CPU 变体 [记忆: 文献转述，未逐一核] |

规律：Bun 系（claude code、opencode、antigravity）踩 AVX/AVX2；Rust 原生（codex 新版）踩 AVX-512；C++ 推理库两者都踩。

## 四、检测阶梯

> 四级由浅入深；前两级是筛查，第四级是权威。shell 命令为 Linux 通用口径。[经验: 通用实践]

```bash
# 第 1 级：flags 筛查（快，但 VM 谎报面存在）
lscpu | grep -iE 'avx|flags'
grep -m1 flags /proc/cpuinfo
```

读法：有 `avx2` 无 `avx512f` 是常态（12 代后 Intel 消费级、Zen 3 及更早、不少云主机被关 ZMM）；连 `avx` 都没有则 Bun 系全灭。注意 guest 内核的 flags 视图通常已按自身 xstate 做过门控，但 hypervisor 层的谎报仍可能穿透——flags 只能排「肯定没有」，不能担保「真有」。[推断: 门控细节待 Linux 实机核]

```bash
# 第 2 级：OS 使能面（OSXSAVE 与 XCR0）
# 用户态无直接读 XCR0 的通用工具；x86info / 自编 xgetbv 探针（先 CPUID 验 OSXSAVE 再执行）
```

第 3 级：**子进程实测指令**——起一个极小的 AVX-512（或 AVX2）指令探针子进程，收尸看信号。这是唯一不被 flags 谎报欺骗的面（第 2 类与第 3 类不匹配都在此现形）。

第 4 级：**逐二进制 `--version` 探针**——oma 已有这个原生面（Windows 上 `oma agents` 装后探针、`oma check` 探 rmux `-V`）。Linux 下把退出形态记全：

```bash
<agent-bin> --version; echo "exit=$?"
# exit=132（128+SIGILL=4）即命中本问题类；bash 惯例 128+N
```

Rust 侧 `std::os::unix::process::ExitStatusExt::signal() == Some(4)`（`code()` 为 None）；Windows 对应形态是 `STATUS_ILLEGAL_INSTRUCTION`（0xC000001D）。**探针打到真二进制上，比任何 CPU 检查都权威**——它测的就是「这份制品在这台机器」的组合。

## 五、oma 落点（P0012 预备，设计口径）

- `oma agents`：Linux 下探针退出形态记全——signal 4 单列报告行（如 `probe=sigill hint=cpu lacks AVX-512; try npm variant or older build`），与现有「校验产物不校验退出码」的装机探针合流。[设计口径]
- `oma doctor`：Linux 加 CPU 能力段——`lscpu` flags 摘要（avx / avx2 / avx512f 三布尔）加各已装二进制探针结果；任一 sigill 则该路 `status=block`（doctor 现有语义）。[设计口径]
- `oma check`：rmux 二进制同探针口径（multiplexer 一般不要求 AVX-512，但同一报告面顺手覆盖）。[设计口径]
- 缓解提示表（诊断输出带 hint）：claude 用 npm 版（Node 跑）；codex 降版或 npm 形态；Bun 系 opencode 换 Node 形态；本地推理库自编译关 native 优化。缓解有效性逐条属 [记忆: 文献转述]，Linux 实机验收时复核。

## 六、关键结论

1. 这类「编排器活着、所有 agent SIGILL」的第一嫌疑就是指令集不匹配；检测入口应该挂在 oma 已有的逐二进制探针上，而不是另造 CPU 检查器——探针测的是制品与机器的组合，天然覆盖 flags 谎报面。[推断: 检测阶梯裁决]
2. flags 筛查只能排除「肯定没有」；CPUID 有而 XCR0 无、hypervisor 谎报两类必须靠实测指令或真二进制探针兜底。[实证: 文献]
3. P0012 的 Linux 验收清单应含一节「指令集预备检测」：先跑第 1 级与第 4 级（oma agents），有 sigill 再走缓解表，避免把环境问题误诊为 oma 接管缺陷。[设计口径]
4. Windows 本机不受此问题类影响（本机四家已装机全绿）；本文全部 Linux 面断言待环境切换后按六态升级。[记忆: 待实机复核]
