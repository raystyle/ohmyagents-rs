# PLAN：当前目标实施计划

## 当前目标：D10 G005 存量字符清理

> 回指 `PRD.md` D10。豁免区、SKIP_DIRS（diary/proven）、封闭清单只减不增，规则见 G005。

### 事实基线

> 2026-09-07 复测（掩豁免区，跳过 diary/proven）。

- 2026-09-02 量化 3671 处含历史归档 [记忆： TODO 排队行]
- 现 SKIP_DIRS 外 555 处：DASH 476、ARROW 72、FULLWIDTH 6、EMOJI 1；封闭清单 46 文件 [实证： 当日 mdcharlint 绕清单复测]
- 默认 `mdcharlint.py` 已进门禁且对新文件零容忍；清单内文件整文件跳过

### 方案骨架

1. **FULLWIDTH 与 EMOJI**：机械替换或改进行内代码（字形举证走豁免区）。6+1 处。
2. **ARROW**：按语义改写（流程改列表、映射写「对应」）；不进清单新债。
3. **DASH**：按语义改写（冒号、括号、拆句、「3 至 5」）；半角 `-` 合法保留。
4. **收口**：文件清零即从 `md-char-allow.txt` 删行；清单空则删文件；门禁保持 mdcharlint 零容忍。

### 验收口径

- SKIP_DIRS 外四类禁字 0；封闭清单 0 行或文件已删
- `uv run --script .tools\mdcharlint.py` 退出 0 且无豁免尾注
- rumdl 与 md-ref / md-heading 触碰文件过

### 门禁

触碰文件逐个 `mdcharlint.py`；不把新文件写入封闭清单。

> 角色：**当前目标方案文档**：基于 `docs\research\`（为什么）与 `docs\references\`（怎么做）撰写的执行计划；每条挂依据来源，随目标变化更新，不存历史目标。
> 分工：`PRD.md` = 要什么；`TODO.md` = 做到哪；本文件 = 怎么做；通用工作流见 `docs\guide\G003-工作流标准细则-从登记到归档五步.md`。
