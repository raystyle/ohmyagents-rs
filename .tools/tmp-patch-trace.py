import io

p = r"D:\ohmyagents\src\trace.rs"
s = io.open(p, encoding="utf-8").read()
start = s.index("/// 四家环境入口：项目内全部编辑事件（当前实现 claude 与 codex；grok/kimi 待 S019 源码核实后接）。")
end = s.index("// ---- 过滤与检索 ----")

new_section = '''/// 四家环境入口：项目内全部编辑事件，按统一 epoch ms 排序（无时间的排最后）。
pub fn timeline(project: &Path) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    for s in list_sessions(project) {
        match s.agent.as_str() {
            "claude" => out.extend(claude_events(&s)),
            "codex" => out.extend(codex_events(&s)),
            "grok" => out.extend(grok_events(&s)),
            "kimi" => out.extend(kimi_events(&s)),
            _ => {}
        }
    }
    out.sort_by(|a, b| a.ts_ms.cmp(&b.ts_ms));
    out
}

/// claude transcript：父链近似为行序——tool_use 前最近的 assistant text 是操作意图、
/// 最近的真实 user text 是用户意图（S018 算法的顺序文件版）。
pub fn claude_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    for v in read_json_lines(&session.file) {
        let t = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
        match t {
            "user" => {
                if v.get("isMeta").and_then(|x| x.as_bool()).unwrap_or(false) {
                    continue;
                }
                let msg = v.get("message").cloned().unwrap_or_default();
                if let Some(text) = msg.get("content").and_then(|c| c.as_str()) {
                    if !text.trim().is_empty() {
                        user_intent = Some(clean_intent(text));
                    }
                }
            }
            "assistant" => {
                let msg = v.get("message").cloned().unwrap_or_default();
                let ts = v.get("timestamp").and_then(|x| x.as_str()).map(|s| s.to_string());
                let ts_ms = ts.as_deref().and_then(ts_to_ms);
                if let Some(blocks) = msg.get("content").and_then(|c| c.as_array()) {
                    for b in blocks {
                        let bt = b.get("type").and_then(|x| x.as_str()).unwrap_or("");
                        if bt == "text" {
                            let text = b.get("text").and_then(|x| x.as_str()).unwrap_or("");
                            if !text.trim().is_empty() {
                                op_intent = Some(clean_intent(text));
                            }
                        } else if bt == "tool_use" {
                            let name = b.get("name").and_then(|x| x.as_str()).unwrap_or("");
                            if !matches!(name, "Edit" | "Write" | "MultiEdit" | "NotebookEdit") {
                                continue;
                            }
                            let input = b.get("input").cloned().unwrap_or_default();
                            let file = input
                                .get("file_path")
                                .and_then(|x| x.as_str())
                                .map(|f| relativize(&normalize_file(f), &session.project));
                            let patch = input
                                .get("new_string")
                                .or_else(|| input.get("content"))
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string());
                            out.push(TraceEvent {
                                agent: session.agent.clone(),
                                session_id: session.id.clone(),
                                call_id: b
                                    .get("id")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                tool: Some(name.to_string()),
                                file,
                                kind: EditKind::Modify,
                                user_intent: user_intent.clone(),
                                op_intent: op_intent.clone(),
                                ts: ts.clone(),
                                ts_ms,
                                patch,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// codex rollout：编辑主源是 `event_msg/item_completed` 的 `FileChange` item（绝对路径 +
/// add/delete/update + 内容或 unified_diff + call_id + completed_at_ms，S019 源码核实）；
/// 旧版无 FileChange 时退回 `custom_tool_call(apply_patch)` 补丁头解析。意图按行序回溯。
pub fn codex_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    let mut fc: Vec<TraceEvent> = Vec::new();
    let mut ct: Vec<TraceEvent> = Vec::new();
    for line in read_json_lines(&session.file) {
        let lt = line.get("type").and_then(|x| x.as_str()).unwrap_or("");
        let payload = line.get("payload").cloned().unwrap_or_default();
        match lt {
            "response_item" => {
                let pt = payload.get("type").and_then(|x| x.as_str()).unwrap_or("");
                if pt == "message" {
                    let role = payload.get("role").and_then(|x| x.as_str()).unwrap_or("");
                    let text = payload
                        .get("content")
                        .and_then(|c| c.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|b| b.get("text"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("");
                    if text.trim().is_empty() {
                        continue;
                    }
                    match role {
                        "user" => {
                            if is_codex_injected_context(text) {
                                continue;
                            }
                            user_intent = Some(clean_intent(text));
                        }
                        "assistant" => op_intent = Some(clean_intent(text)),
                        _ => {}
                    }
                } else if pt == "custom_tool_call" {
                    let name = payload.get("name").and_then(|x| x.as_str()).unwrap_or("");
                    if name != "apply_patch" {
                        continue;
                    }
                    let input = payload.get("input").and_then(|x| x.as_str()).unwrap_or("");
                    let ts = line
                        .get("timestamp")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string());
                    for (kind, file) in parse_apply_patch(input) {
                        ct.push(TraceEvent {
                            agent: session.agent.clone(),
                            session_id: session.id.clone(),
                            call_id: payload
                                .get("call_id")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                            tool: Some("apply_patch".into()),
                            file: Some(relativize(&file, &session.project)),
                            kind,
                            user_intent: user_intent.clone(),
                            op_intent: op_intent.clone(),
                            ts: ts.clone(),
                            ts_ms: ts.as_deref().and_then(ts_to_ms),
                            patch: Some(input.to_string()),
                        });
                    }
                }
            }
            "event_msg" => {
                if payload.get("type").and_then(|x| x.as_str()) != Some("item_completed") {
                    continue;
                }
                let item = payload.get("item").cloned().unwrap_or_default();
                if item.get("type").and_then(|x| x.as_str()) != Some("FileChange") {
                    continue;
                }
                let call_id = item
                    .get("id")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                let ms = payload
                    .get("completed_at_ms")
                    .and_then(|x| x.as_u64())
                    .or_else(|| payload.get("started_at_ms").and_then(|x| x.as_u64()));
                if let Some(changes) = item.get("changes").and_then(|c| c.as_object()) {
                    for (path, change) in changes {
                        let ct_tag = change.get("type").and_then(|x| x.as_str()).unwrap_or("");
                        let kind = match ct_tag {
                            "add" => EditKind::Create,
                            "delete" => EditKind::Delete,
                            _ => EditKind::Modify,
                        };
                        let patch = change
                            .get("unified_diff")
                            .or_else(|| change.get("content"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string());
                        fc.push(TraceEvent {
                            agent: session.agent.clone(),
                            session_id: session.id.clone(),
                            call_id: call_id.clone(),
                            tool: Some("apply_patch".into()),
                            file: Some(relativize(&normalize_file(path), &session.project)),
                            kind,
                            user_intent: user_intent.clone(),
                            op_intent: op_intent.clone(),
                            ts: ms.map(ms_to_iso),
                            ts_ms: ms,
                            patch,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    if fc.is_empty() {
        ct
    } else {
        fc
    }
}

/// codex 的环境注入 user message（指令/环境块）不是用户意图；marker 清单对齐
/// codex 源码 `CONTEXTUAL_USER_FRAGMENT_MATCHERS` 的主要项（S019 核实）。
fn is_codex_injected_context(text: &str) -> bool {
    let head = text.trim_start();
    head.starts_with("# AGENTS.md")
        || head.starts_with("<environment_context>")
        || head.starts_with("<user_instructions>")
        || head.starts_with("<turn_context>")
        || head.starts_with("<turn_aborted>")
        || head.starts_with("<user_shell_command>")
        || head.starts_with("<subagent_notification>")
        || head.starts_with("<current_time_reminder>")
}

/// 解析 apply_patch 补丁头的 Add/Update/Delete File 行。
pub fn parse_apply_patch(input: &str) -> Vec<(EditKind, String)> {
    let mut out = Vec::new();
    for line in input.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("*** Add File: ") {
            out.push((EditKind::Create, normalize_file(rest.trim())));
        } else if let Some(rest) = line.strip_prefix("*** Update File: ") {
            out.push((EditKind::Modify, normalize_file(rest.trim())));
        } else if let Some(rest) = line.strip_prefix("*** Delete File: ") {
            out.push((EditKind::Delete, normalize_file(rest.trim())));
        }
    }
    out
}

/// grok chat_history.jsonl（ConversationItem 行，S019 源码核实）：真实 user 行
/// （`synthetic_reason == null`）更用户意图；assistant.content 是操作意图；
/// 编辑在 `assistant.tool_calls[]`（name 属写文件族，arguments 是 JSON 串取 file_path）。
/// 行无时间戳，ts 用会话 uuid v7 的生成时刻近似（官方口径前 48 位 unix ms）。
/// 注：权威日志是 updates.jsonl（chat_history 是派生缓存可被重建）；v1 读 chat_history，
/// updates.jsonl 形状（{timestamp,method,params} 信封）留作升级路径。
pub fn grok_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    let session_ms = grok_uuid_v7_ms(&session.id);
    let ts = session_ms.map(ms_to_iso);
    for v in read_json_lines(&session.file) {
        match v.get("type").and_then(|x| x.as_str()).unwrap_or("") {
            "user" => {
                if !v.get("synthetic_reason").map(|s| s.is_null()).unwrap_or(true) {
                    continue;
                }
                let text = concat_text_parts(v.get("content"));
                if !text.trim().is_empty() {
                    user_intent = Some(clean_intent(&text));
                }
            }
            "assistant" => {
                if let Some(text) = v.get("content").and_then(|c| c.as_str()) {
                    if !text.trim().is_empty() {
                        op_intent = Some(clean_intent(text));
                    }
                }
                if let Some(calls) = v.get("tool_calls").and_then(|c| c.as_array()) {
                    for tc in calls {
                        let name = tc.get("name").and_then(|x| x.as_str()).unwrap_or("");
                        if !matches!(
                            name,
                            "search_replace" | "write" | "edit" | "hashline_edit" | "apply_patch"
                        ) {
                            continue;
                        }
                        let args = tc
                            .get("arguments")
                            .and_then(|x| x.as_str())
                            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                            .unwrap_or_default();
                        let file = args
                            .get("file_path")
                            .or_else(|| args.get("path"))
                            .and_then(|x| x.as_str())
                            .map(|f| relativize(&normalize_file(f), &session.project));
                        let patch = args
                            .get("new_string")
                            .or_else(|| args.get("content"))
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string());
                        out.push(TraceEvent {
                            agent: session.agent.clone(),
                            session_id: session.id.clone(),
                            call_id: tc
                                .get("id")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string()),
                            tool: Some(name.to_string()),
                            file,
                            kind: EditKind::Modify,
                            user_intent: user_intent.clone(),
                            op_intent: op_intent.clone(),
                            ts: ts.clone(),
                            ts_ms: session_ms,
                            patch,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// kimi wire.jsonl（协议 1.5，S019 源码核实）：`turn.prompt` 且 `origin.kind=="user"` 是
/// 用户意图权威源；`context.append_message` role=assistant 的 text part 是操作意图；
/// 编辑 = loop event `tool.call`（name 属 Edit/Write，args.path 是路径键）。时间原生 epoch ms。
pub fn kimi_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    for v in read_json_lines(&session.file) {
        let ms = v.get("time").and_then(|x| x.as_u64());
        let ts = ms.map(ms_to_iso);
        match v.get("type").and_then(|x| x.as_str()).unwrap_or("") {
            "turn.prompt" => {
                let origin = v
                    .get("origin")
                    .and_then(|o| o.get("kind"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                if origin != "user" {
                    continue;
                }
                let text = concat_text_parts(v.get("input"));
                if !text.trim().is_empty() {
                    user_intent = Some(clean_intent(&text));
                }
            }
            "context.append_message" => {
                let msg = v.get("message").cloned().unwrap_or_default();
                if msg.get("role").and_then(|x| x.as_str()) != Some("assistant") {
                    continue;
                }
                let text = concat_text_parts(msg.get("content"));
                if !text.trim().is_empty() {
                    op_intent = Some(clean_intent(&text));
                }
            }
            "context.append_loop_event" => {
                let ev = v.get("event").cloned().unwrap_or_default();
                if ev.get("type").and_then(|x| x.as_str()) != Some("tool.call") {
                    continue;
                }
                let name = ev.get("name").and_then(|x| x.as_str()).unwrap_or("");
                if !matches!(name, "Edit" | "Write") {
                    continue;
                }
                let args = ev.get("args").cloned().unwrap_or_default();
                let file = args
                    .get("path")
                    .or_else(|| args.get("file_path"))
                    .and_then(|x| x.as_str())
                    .map(|f| relativize(&normalize_file(f), &session.project));
                let patch = args
                    .get("new_string")
                    .or_else(|| args.get("content"))
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                out.push(TraceEvent {
                    agent: session.agent.clone(),
                    session_id: session.id.clone(),
                    call_id: ev
                        .get("toolCallId")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string()),
                    tool: Some(name.to_string()),
                    file,
                    kind: EditKind::Modify,
                    user_intent: user_intent.clone(),
                    op_intent: op_intent.clone(),
                    ts: ts.clone(),
                    ts_ms: ms,
                    patch,
                });
            }
            _ => {}
        }
    }
    out
}

/// ContentPart 数组（[{type:"text",text},...]）取 text 拼接；跳过 think/blob_ref/媒体 part。
fn concat_text_parts(v: Option<&serde_json::Value>) -> String {
    let Some(v) = v else { return String::new() };
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    let Some(arr) = v.as_array() else { return String::new() };
    let mut out = String::new();
    for p in arr {
        if p.get("type").and_then(|x| x.as_str()) == Some("text") {
            if let Some(t) = p.get("text").and_then(|x| x.as_str()) {
                out.push_str(t);
            }
        }
    }
    out
}

'''

s = s[:start] + new_section + s[end:]
io.open(p, "w", encoding="utf-8", newline="\n").write(s)
print("section replaced")
