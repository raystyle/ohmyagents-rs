# agent 意图操作块与编辑轨迹检索

- 状态：进行中（S018 研究已备，实现切片推进）
- 日期：2026-08-31
- 关联：研究 `S018`（aitrace 机制与 oma 映射）、`S009`（状态四层）、`S015`（hook 矩阵）；前置 P0006-P0012（编排与安装底座）；用户定调 2026-08-31：「增加研究 D:\aitrace，实现指定项目下的各 Agent 意图操作块及编辑文件轨迹的检索功能」

## 背景与问题

oma 编排四路 agent 改同一项目，事后无法回答「哪个 agent、基于什么意图、何时、改了什么文件」。aitrace（S018）已把「文件编辑真相 + hook 元数据 + transcript 意图」三源关联做通但只支持 Claude Code 一家、且无 agent 过滤、会话不记项目路径。oma 的主场恰是多路 agent：pane 清单与 hook 身份都是 oma 自己发的。

## 目标与非目标

- 目标：
  - 采集：项目级编辑事件落 `.ohmyagents\trace\`（edits.jsonl + meta.json，meta 显式落 project_path 与 agents——补 aitrace 两缺口）
  - 归组：`operation_id`（agent 会话标识加 tool 调用标识）串 hook 元数据与会话日志意图，双意图字段（operation_intent / intent）分开
  - 检索：`oma trace` 子命令族（sessions / timeline / search / file），支持**按 agent、按项目、按文件 glob、按 regex** 过滤加分页 clamp
  - 四家逐步接入：Claude 先行（transcript 结构已知），codex/grok/kimi 逐家研究会话日志格式后接
- 非目标：
  - 不做恢复/回滚（restore）；内容寻址快照暂缓，只存 diff + 哈希
  - 不自建常驻录制 daemon（v1 用 oma hook 事件 + 查询时会话日志回溯；watcher 引入与否在切片 3 决）
  - 不改各 agent 用户级配置；trace 数据只落项目目录（AGENTS 边界）

## 方案

### 数据形状

```text
.ohmyagents\trace\
  sessions\<YYYYMMDD-HHMMSS-6f>\
    meta.json     { id, project_path(显式), agents[]: {agent, label, pane_id, first_seen} }
    edits.jsonl   每行一个编辑事件（首事件才建目录，防空会话堆积）：
                  { id, ts, file(写库即归一化: 相对+正斜杠+小写), kind,
                    patch, before/after_sha256,
                    agent, operation_id, operation_intent, intent, tool }
```

### 意图回溯

查询时活走会话日志父链（assistant text 胜出、thinking 次之）+ 同 id 追加补账（`.ohmyagents\trace\backfill.json`，上限 32）——照抄 S018 算法，中文截断按字符。

### 并发归组

按 `agent + operation` 维度排队（不是按文件路径单 FIFO——S018 坑 5：两 agent 同改一文件会张冠李戴）；关联键写库即归一化。

### 切片

1. **S019 四家会话日志格式研究**：Claude transcript（已知）；codex rollout jsonl；grok、kimi 会话文件定位与结构（源码法，同 S015）
2. **trace 存储层**：`src\trace.rs`（edits.jsonl 追加 + read_all 按 id 去重保最后 + meta 落库）
3. **采集 v1**：oma hook 扩 PostToolUse（Claude，S015 矩阵）+ 编辑真相源切片内定（notify watcher 或 hook 单源）
4. **检索面**：`oma trace sessions|timeline|search`（agent/project/file glob/regex/分页 clamp 100-1000）；输出带 buildHash 风格版本行
5. **P0011 联动**：检索面同签名挂 MCP tool（P0011 mcp 模块）

## 风险与回滚

- 四家 transcript 格式漂移（无官方契约）：解析按「尽力而为 + null 兜底」，格式变化不崩检索
- edits.jsonl 线性扫性能：v1 接受，量大再 sqlite（R005）
- 回滚：trace 是新增目录与新增子命令，关掉即无痕

## 验收标准

- 指定项目跑一路真实 agent（Claude 先行）改文件后：`oma trace timeline` 能按 operation_id 出块、双意图可见、`oma trace search <词>` 命中 patch/intent/file 三域
- 按 agent 过滤在多路会话下各归各（stub 双路互改同文件不张冠李戴）
- meta.json 的 project_path 非空；空跑不建会话目录
- `cargo test` 全绿；文档三件套过；R002/INDEX/TODO/GOAL/diary 同步

## 实施过程与经验

（进行中）
