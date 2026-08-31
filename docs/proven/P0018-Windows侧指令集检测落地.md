# Windows 侧指令集检测落地

- 状态：已完成（2026-08-31 当日达成，本机实测过）
- 日期：2026-08-31
- 关联：研究 `S021`（检测阶梯与设计口径）；用户问答「没有研究 WINDOWS 会要检测这个 cpu 指令么」——答案是会：核到的两案（codex 17410 捆绑 exe、25367 CLI）恰都是 Windows 形态

## 背景与问题

S021 把指令集 SIGILL 检测留成 P0012 的 Linux 设计口径，但检测的两大件在 Windows 当场可落：std 的 `is_x86_feature_detected!`（内部先 CPUID 再验 OSXSAVE/XCR0，比 flags 筛查可靠，免依赖）与异常退出码分类（Windows 形态 `STATUS_ILLEGAL_INSTRUCTION` 0xC000001D）。本机四家装机全绿只说明「这台机器够」，oma 作为产品要能在别的 Windows 机器上把「所有 agent 启动即崩」诊断出来。

## 方案

- `src\caps.rs`：`detect()`（arch 加 avx/avx2/avx512f 三布尔，非 x86_64 为 None）加 `caps_line()` marker 形态；`classify_probe_exit()`（0 为 ok、Windows 命中 0xC000001D 为 illegal-instruction、其余 failed、None 为 signal-exit——Unix signal 4 细分留给 P0012）。
- `oma doctor`：首段加 CPU 能力 finding（`agent=cpu check=caps`，status=ok 的事实面陈述）。
- `oma agents`：version 为 None 的失败路径补跑一次 `--version` 只为拿退出码分类，命中 illegal-instruction 输出 S021 缓解 hint（npm 变体或降版）；正常路径零额外进程。
- `oma agents install` 装后探针 `probe=unavailable` 升级为 `probe=unavailable(<kind>)`。

## 验收标准与结果

- 单测：detect 结构锁（x86_64 上三字段有值）、caps_line 形态、退出分类四形态（Windows 分支含 0xC000001D）。过。[实证]
- 集成：doctor 输出含 `check=caps` 与 `avx2=`（tests/cli.rs 扩展断言）。过。[实证]
- 本机实测：`agent=cpu check=caps status=ok path=x86_64 detail=x86_64 avx=true avx2=true avx512f=false`——本机有 AVX2 无 AVX-512，恰是 S021 谱系里「Bun 系可跑、AVX-512 制品会灭」的典型形态。过。[实证]
- 基线：71+10（无 feature）全绿。过。[实证]

## 实施过程与经验

- 用户一句反问推翻了我「这是 Linux 专属问题」的默认框架——研究文档落 Linux 没错，但可测面在哪就该在哪落地；Windows 侧两件（std 检测加退出码分类）零新依赖当场收。
- 探针分类走「失败才补跑」：正常面不多起进程，print_reports 只在 version 为 None 时进分类——诊断成本只花在出问题的机器上。
- 本机 avx512f=false 是有价值的事实锚：若未来某 agent 升级到要求 AVX-512 的制品，本机会当场灭——doctor 的 CPU 段就是那时候的第一诊断入口。
