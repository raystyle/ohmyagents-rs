# Windows 全量收口

- 状态：已完成（2026-08-31 当日达成，五切片验收全过）
- 日期：2026-08-31
- 关联：用户定调「linux mac 接管排后 先在windows上把所有都开发好」；`S005`（间隔铁律）、`S016`（命令即 skill 末件）、`S007`（grok 无头缺口）、`P0011`（trace 三传输对齐）

## 背景与问题

设计命令全部落地（P0016）后盘点 Windows 侧残余缺口，坐实四件：orch::send 文本后紧连 Enter（S005「隔开发」铁律未进产品路径，真 TUI 会吞 Enter）；api::trace_* 三函数只有 CLI 与 MCP 消费、HTTP 没挂（三传输不对齐）；deploy.rs 的 SKILL.md 是写死四行（S016 末件命令图生成未做）；grok `-p` 无头从未本机实跑（S007 缺口）。顺手加 `oma mcp --print-config`。

## 方案与切片

1. **send 间隔产品化**：粘贴或 send_text 后，用 rmux 原生 `expect_visible_text` 等载荷末行短头（前 24 字符）可见再单独发 Enter——不是盲 sleep（S005 口径）；超时降级为照发 Enter 并 `send.echo=timeout` 留痕（echo 缺失的极端面不致 send 失败）。
2. **HTTP trace 三端点**：`GET /trace/sessions`、`/trace/timeline?agent&file&limit`、`/trace/search?q&agent&limit` 直接挂 api 层现成函数；`/api` 自述到 11 端点；网页加轨迹面板（agent 下拉、glob、检索词，JSON 落 `<pre>`）。
3. **SKILL 命令图生成**：deploy.rs 内置 `COMMAND_MAP`（意图加命令对），`skill_md()` 生成 SKILL.md；覆写语义三态——带生成标记的同步覆写、旧静态版全文识别后升级、无标记的用户内容跳过；`.agents` 源加三家副本同规则。
4. **grok 无头实跑**：本机 grok 1.0.13 `grok --always-approve -p "<任务>"` 写文件 exit 0、产物精确；S007 缺口回填。
5. **mcp 配置打印**：`oma mcp --print-config` 出 Claude Code（`claude mcp add`）、codex（`[mcp_servers.oma]`）、通用 mcpServers 三形态片段；exe 绝对路径加 `--project` 锚定；无 mcp feature 的构建也可用。

## 验收标准与结果

- send 间隔：stub `send --confirm` 链路 `send.echo=visible` 后 Enter，confirm 命中。过。[实证]
- HTTP trace：本仓真数据 sessions 4 家、timeline 3 条、search `q=trace.rs` 2 命中 43 块、`/api` 11 端点。过。[实证]
- skill 生成：单测四态（新写、幂等跳过、旧版升级、用户内容不动）加生成内容含标记与全命令图；活体 init 后 `.agents` 源与 grok 副本同步带标记、用户手改文件原样保留。过。[实证]
- grok 无头：产物内容精确匹配；同场联邦 trace 检出该无头会话（updates.jsonl 主源、真实时间戳、双意图、tool=write）——无头到检索全链闭环。过。[实证]
- print-config：featureless 与全 feature 两构建都出三种片段。过。[实证]
- 基线：69+10（无 feature）与 72+10（server,mcp）全绿零 warning。过。[实证]

## 实施过程与经验

- 间隔的正解是「等回显」不是「睡两秒」：S005 早就写了「rmux 原生静默等待，不用盲 sleep」，P0010 的 2 秒是手动救急值——产品化时回读研究文档避免把救急值焊死进代码。降级分支（超时照发）是 echo 异常面的兼容垫，留痕不静默。
- grok 的 `-p` 是 `--single <PROMPT>` 短名：值必须紧跟，`-p --always-approve "任务"` 把 flag 挤进缺参位 exit 2——无头命令的 flag 顺序也是接口契约，S007 回填记档。
- 命令图数组的覆写三态沿用 hook 部署的「ours 才动」哲学：标记识别（generated marker）加旧版全文指纹（legacy const）双保险，用户内容零风险。
- `oma mcp --print-config` 在无 feature 构建也放行：注册片段是纯文本生成，不依赖 rmcp——feature 门只挡真 server。
