//! oma trace：查询时联邦的四家会话日志检索（P0013，S019）。
//! 直接读各家原生会话库并归一化——零采集设施、可回溯 oma 部署前的历史。
//! 写库即归一化原则（S018 坑 3）：文件路径一律正斜杠；项目比较统一「正斜杠 + 小写」。

use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

/// 意图截断上限（S018：按字符不按字节，中文按字节截会 panic）。
const MAX_INTENT_CHARS: usize = 200;
/// 分页 clamp（S018 参数形状）。
pub const DEFAULT_LIMIT: usize = 100;
pub const MAX_LIMIT: usize = 1000;

#[derive(Debug, Clone)]
pub struct TraceSession {
    pub agent: String,
    pub id: String,
    pub project: PathBuf,
    pub file: PathBuf,
    pub started_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Create,
    Modify,
    Delete,
}

impl EditKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EditKind::Create => "create",
            EditKind::Modify => "modify",
            EditKind::Delete => "delete",
        }
    }
}

/// 归一化事件：四家 loader 的公共产出。operation_id = session_id:call_id（S018 核心设计）。
#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub agent: String,
    pub session_id: String,
    pub call_id: Option<String>,
    pub tool: Option<String>,
    pub file: Option<String>,
    pub kind: EditKind,
    pub user_intent: Option<String>,
    pub op_intent: Option<String>,
    pub ts: Option<String>,
    /// 可检索正文（patch、Edit new_string、Write content 等）。
    pub patch: Option<String>,
}

impl TraceEvent {
    pub fn operation_id(&self) -> String {
        format!(
            "{}:{}",
            self.session_id,
            self.call_id.as_deref().unwrap_or("-")
        )
    }
}

/// 意图操作块：同一 operation_id（一次工具调用，可能多文件）的事件聚合。
#[derive(Debug, Clone)]
pub struct TraceBlock {
    pub op: String,
    pub agent: String,
    pub session_id: String,
    pub files: Vec<String>,
    pub edits: usize,
    pub kinds: Vec<String>,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub user_intent: Option<String>,
    pub op_intent: Option<String>,
}

/// 按首次出现顺序聚合事件为块（一个 operation_id 一块）。
pub fn group_blocks(events: &[TraceEvent]) -> Vec<TraceBlock> {
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::BTreeMap<String, TraceBlock> = Default::default();
    for e in events {
        let op = e.operation_id();
        if !map.contains_key(&op) {
            order.push(op.clone());
            map.insert(
                op.clone(),
                TraceBlock {
                    op: op.clone(),
                    agent: e.agent.clone(),
                    session_id: e.session_id.clone(),
                    files: Vec::new(),
                    edits: 0,
                    kinds: Vec::new(),
                    first_ts: e.ts.clone(),
                    last_ts: e.ts.clone(),
                    user_intent: e.user_intent.clone(),
                    op_intent: e.op_intent.clone(),
                },
            );
        }
        let b = map.get_mut(&op).unwrap();
        b.edits += 1;
        if let Some(f) = &e.file {
            if !b.files.contains(f) {
                b.files.push(f.clone());
            }
        }
        let k = e.kind.as_str().to_string();
        if !b.kinds.contains(&k) {
            b.kinds.push(k);
        }
        b.last_ts = e.ts.clone().or_else(|| b.last_ts.clone());
    }
    order.into_iter().filter_map(|op| map.remove(&op)).collect()
}

pub struct TraceFilter<'a> {
    pub agent: Option<&'a str>,
    pub file_glob: Option<&'a str>,
    pub limit: usize,
}

impl<'a> TraceFilter<'a> {
    pub fn clamp_limit(&self) -> usize {
        self.limit.clamp(1, MAX_LIMIT)
    }
}

fn normalize_compare(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/").to_lowercase()
}

fn normalize_file(f: &str) -> String {
    f.replace('\\', "/")
}

/// 项目内路径相对化（比较用小写、返回保留原大小写的正斜杠相对形）。
fn relativize(file: &str, project: &Path) -> String {
    let pn = normalize_compare(project);
    let pn = pn.trim_end_matches('/');
    let fl = file.to_lowercase();
    if let Some(rest) = fl.strip_prefix(&format!("{pn}/")) {
        let cut = file.len() - rest.len();
        return file[cut..].to_string();
    }
    file.to_string()
}

fn truncate_chars(s: &str) -> String {
    s.chars().take(MAX_INTENT_CHARS).collect()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_intent(s: &str) -> String {
    truncate_chars(&collapse_ws(s))
}

/// 按行读 jsonl，坏行跳过（append-only 容错：截断尾行不崩检索）。
fn read_json_lines(path: &Path) -> Vec<serde_json::Value> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    io::BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect()
}

// ---- 会话发现：四家原生库 ----

/// `~/.claude/projects/<slug>/`：slug 规则是路径串里非字母数字一律换 `-`
/// （D:\ohmyagents → D--ohmyagents）。
pub fn claude_project_slug(project: &Path) -> String {
    project
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

pub fn claude_sessions_in(dir: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(dir) else { return out };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(id) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        out.push(TraceSession {
            agent: "claude".into(),
            id: id.to_string(),
            project: project.to_path_buf(),
            file: p,
            started_at: None,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// codex rollout：首行 session_meta.payload.cwd 决定项目归属。
pub fn codex_sessions_under(root: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !name.starts_with("rollout-") {
                    continue;
                }
                if let Some(meta) = codex_session_meta(&p) {
                    let cwd = meta.get("cwd").and_then(|v| v.as_str()).unwrap_or("");
                    if normalize_compare(Path::new(cwd)) == normalize_compare(project) {
                        let id = meta
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let started = meta
                            .get("timestamp")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        out.push(TraceSession {
                            agent: "codex".into(),
                            id,
                            project: project.to_path_buf(),
                            file: p,
                            started_at: started,
                        });
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file));
    out
}

fn codex_session_meta(file: &Path) -> Option<serde_json::Value> {
    let file = fs::File::open(file).ok()?;
    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
            if v.get("type").and_then(|t| t.as_str()) == Some("session_meta") {
                return v.get("payload").cloned();
            }
            // 首行不是 session_meta 就不必再扫（meta 恒在首行）。
            return None;
        }
    }
    None
}

/// grok：`~/.grok/sessions/<百分号编码的项目路径>/<会话uuid>/chat_history.jsonl`。
pub fn grok_sessions_in(root: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let want = normalize_compare(project);
    let Ok(rd) = fs::read_dir(root) else { return out };
    for ent in rd.flatten() {
        let p = ent.path();
        if !p.is_dir() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else { continue };
        if normalize_compare(Path::new(&percent_decode(name))) != want {
            continue;
        }
        let Ok(inner) = fs::read_dir(&p) else { continue };
        for sdir in inner.flatten() {
            let spath = sdir.path();
            let hist = spath.join("chat_history.jsonl");
            if !hist.is_file() {
                continue;
            }
            let Some(id) = spath.file_name().and_then(|s| s.to_str()) else { continue };
            out.push(TraceSession {
                agent: "grok".into(),
                id: id.to_string(),
                project: project.to_path_buf(),
                file: hist,
                started_at: None,
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() + 1 && i + 2 < bytes.len() {
            let hex = &s[i + 1..i + 3];
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// kimi：`~/.kimi-code/session_index.jsonl` 行 {sessionId, sessionDir, workDir}。
pub fn kimi_sessions_in(index: &Path, project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let want = normalize_compare(project);
    for v in read_json_lines(index) {
        let work = v.get("workDir").and_then(|x| x.as_str()).unwrap_or("");
        if normalize_compare(Path::new(work)) != want {
            continue;
        }
        let id = v
            .get("sessionId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let file = v
            .get("sessionDir")
            .and_then(|x| x.as_str())
            .map(PathBuf::from)
            .map(|d| d.join("agents").join("main").join("wire.jsonl"))
            .unwrap_or_default();
        if id.is_empty() || !file.is_file() {
            continue;
        }
        out.push(TraceSession {
            agent: "kimi".into(),
            id,
            project: project.to_path_buf(),
            file,
            started_at: None,
        });
    }
    out
}

// ---- 事件抽取 ----

/// 四家环境入口：列指定项目的全部会话。
pub fn list_sessions(project: &Path) -> Vec<TraceSession> {
    let mut out = Vec::new();
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    out.extend(claude_sessions_in(
        &home.join(".claude").join("projects").join(claude_project_slug(project)),
        project,
    ));
    out.extend(codex_sessions_under(&home.join(".codex").join("sessions"), project));
    out.extend(grok_sessions_in(&home.join(".grok").join("sessions"), project));
    out.extend(kimi_sessions_in(&home.join(".kimi-code").join("session_index.jsonl"), project));
    out
}

/// 四家环境入口：项目内全部编辑事件（当前实现 claude 与 codex；grok/kimi 待 S019 源码核实后接）。
pub fn timeline(project: &Path) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    for s in list_sessions(project) {
        match s.agent.as_str() {
            "claude" => out.extend(claude_events(&s)),
            "codex" => out.extend(codex_events(&s)),
            _ => {}
        }
    }
    out.sort_by(|a, b| a.ts.cmp(&b.ts));
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

/// codex rollout：顺序扫 response_item——assistant message 更新操作意图、真实 user message
/// 更新用户意图（跳过环境注入），custom_tool_call(apply_patch) 出编辑事件。
pub fn codex_events(session: &TraceSession) -> Vec<TraceEvent> {
    let mut out = Vec::new();
    let mut user_intent: Option<String> = None;
    let mut op_intent: Option<String> = None;
    for line in read_json_lines(&session.file) {
        if line.get("type").and_then(|x| x.as_str()) != Some("response_item") {
            continue;
        }
        let payload = line.get("payload").cloned().unwrap_or_default();
        let ts = line
            .get("timestamp")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let pt = payload.get("type").and_then(|x| x.as_str()).unwrap_or("");
        match pt {
            "message" => {
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
            }
            "custom_tool_call" => {
                let name = payload.get("name").and_then(|x| x.as_str()).unwrap_or("");
                if name != "apply_patch" {
                    continue;
                }
                let input = payload.get("input").and_then(|x| x.as_str()).unwrap_or("");
                for (kind, file) in parse_apply_patch(input) {
                    let file = relativize(&file, &session.project);
                    out.push(TraceEvent {
                        agent: session.agent.clone(),
                        session_id: session.id.clone(),
                        call_id: payload
                            .get("call_id")
                            .and_then(|x| x.as_str())
                            .map(|s| s.to_string()),
                        tool: Some("apply_patch".into()),
                        file: Some(file),
                        kind,
                        user_intent: user_intent.clone(),
                        op_intent: op_intent.clone(),
                        ts: ts.clone(),
                        patch: Some(input.to_string()),
                    });
                }
            }
            _ => {}
        }
    }
    out
}

/// codex 的环境注入 user message（指令/环境块）不是用户意图。
fn is_codex_injected_context(text: &str) -> bool {
    let head = text.trim_start();
    head.starts_with("# AGENTS.md")
        || head.starts_with("<environment_context>")
        || head.starts_with("<user_instructions>")
        || head.starts_with("<turn_context>")
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

// ---- 过滤与检索 ----

pub fn apply_filter(events: Vec<TraceEvent>, filter: &TraceFilter) -> Vec<TraceEvent> {
    let mut out: Vec<TraceEvent> = events
        .into_iter()
        .filter(|e| match filter.agent {
            Some(a) => e.agent == a,
            None => true,
        })
        .filter(|e| match filter.file_glob {
            Some(g) => e
                .file
                .as_deref()
                .map(|f| file_matches(f, g))
                .unwrap_or(false),
            None => true,
        })
        .collect();
    out.reverse();
    out.truncate(filter.clamp_limit());
    out.reverse();
    out
}

/// 文件过滤：glob 有则用 glob 匹配，解析失败退回子串（S018：regex 非法退字面子串的同款姿态）。
pub fn file_matches(file: &str, pattern: &str) -> bool {
    if let Ok(g) = glob::Pattern::new(pattern) {
        return g.matches(file);
    }
    file.contains(pattern)
}

/// 检索：query 按正则匹配 patch、file、双意图四域；非法正则退回字面子串。
pub fn search_matches(event: &TraceEvent, query: &str) -> bool {
    let re = regex::Regex::new(query).ok();
    let fields = [
        event.patch.as_deref().unwrap_or(""),
        event.file.as_deref().unwrap_or(""),
        event.user_intent.as_deref().unwrap_or(""),
        event.op_intent.as_deref().unwrap_or(""),
    ];
    match &re {
        Some(re) => fields.iter().any(|f| re.is_match(f)),
        None => fields.iter().any(|f| f.contains(query)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("trace")
            .join(name)
    }

    #[test]
    fn claude_slug_matches_observed_dirs() {
        assert_eq!(
            claude_project_slug(Path::new(r"D:\ohmyagents")),
            "D--ohmyagents"
        );
        assert_eq!(
            claude_project_slug(Path::new(r"C:\Users\ray")),
            "C--Users-ray"
        );
    }

    #[test]
    fn claude_events_extract_edit_with_dual_intent() {
        let dir = fixture("claude");
        let sessions = claude_sessions_in(&dir, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 1);
        let events = claude_events(&sessions[0]);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.agent, "claude");
        assert_eq!(e.tool.as_deref(), Some("Edit"));
        assert!(e.file.as_deref().unwrap().ends_with("src/lib.rs"));
        assert_eq!(e.kind, EditKind::Modify);
        assert_eq!(e.user_intent.as_deref(), Some("把 greet 改成中文"));
        assert_eq!(e.op_intent.as_deref(), Some("我来修改 greet 函数返回中文"));
        assert!(e.call_id.as_deref().unwrap().starts_with("call_"));
        assert!(e.ts.as_deref().unwrap().starts_with("2026-"));
        assert!(e.patch.as_deref().unwrap().contains("你好"));
        assert!(e.operation_id().contains(":call_"));
    }

    #[test]
    fn codex_events_extract_apply_patch() {
        let dir = fixture("codex");
        let sessions = codex_sessions_under(&dir, Path::new(r"D:\demo"));
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id.len(), 36);
        let events = codex_events(&sessions[0]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, EditKind::Create);
        assert!(events[0].file.as_deref().unwrap().ends_with("notes.md"));
        assert_eq!(events[1].kind, EditKind::Modify);
        assert!(events[1].file.as_deref().unwrap().ends_with("README.md"));
        assert_eq!(events[0].user_intent.as_deref(), Some("加一个笔记文件并更新 README"));
        assert!(events[0]
            .op_intent
            .as_deref()
            .unwrap()
            .contains("先创建笔记再改 README"));
        assert_eq!(events[0].tool.as_deref(), Some("apply_patch"));
        assert!(events[0].call_id.as_deref().unwrap().starts_with("call_"));
    }

    #[test]
    fn apply_patch_parser_handles_three_kinds() {
        let patch = "*** Begin Patch\n*** Add File: a\\b.txt\n+x\n*** Update File: c/d.rs\n-y\n+z\n*** Delete File: e.txt\n*** End Patch\n";
        let parsed = parse_apply_patch(patch);
        assert_eq!(
            parsed,
            vec![
                (EditKind::Create, "a/b.txt".to_string()),
                (EditKind::Modify, "c/d.rs".to_string()),
                (EditKind::Delete, "e.txt".to_string()),
            ]
        );
    }

    #[test]
    fn codex_injected_context_is_recognized() {
        assert!(is_codex_injected_context("# AGENTS.md instructions for D:\\x"));
        assert!(is_codex_injected_context("<environment_context>"));
        assert!(!is_codex_injected_context("正常用户输入"));
    }

    #[test]
    fn group_blocks_aggregates_multi_file_operation() {
        let dir = fixture("codex");
        let sessions = codex_sessions_under(&dir, Path::new(r"D:\demo"));
        let events = codex_events(&sessions[0]);
        let blocks = group_blocks(&events);
        assert_eq!(blocks.len(), 1);
        let b = &blocks[0];
        assert_eq!(b.edits, 2);
        assert_eq!(b.files, vec!["notes.md".to_string(), "README.md".to_string()]);
        assert_eq!(b.kinds, vec!["create".to_string(), "modify".to_string()]);
        assert_eq!(b.user_intent.as_deref(), Some("加一个笔记文件并更新 README"));
        assert!(b.op.ends_with(":call_xyz789"));
    }

    #[test]
    fn filter_and_search_clamp_and_match() {
        let mk = |i: usize| TraceEvent {
            agent: if i % 2 == 0 { "claude" } else { "codex" }.into(),
            session_id: format!("s{i}"),
            call_id: Some(format!("call_{i}")),
            tool: Some("Edit".into()),
            file: Some(format!("src/mod{i}.rs")),
            kind: EditKind::Modify,
            user_intent: Some(format!("任务{i}")),
            op_intent: None,
            ts: Some(format!("2026-08-31T00:00:{i:02}Z")),
            patch: Some(format!("fn v{i}() {{}}")),
        };
        let events: Vec<TraceEvent> = (0..10).map(mk).collect();
        let f = TraceFilter { agent: Some("claude"), file_glob: Some("src/*.rs"), limit: 999 };
        let got = apply_filter(events.clone(), &f);
        assert!(got.iter().all(|e| e.agent == "claude"));
        assert_eq!(got.len(), 5);
        let f = TraceFilter { agent: None, file_glob: None, limit: 3 };
        assert_eq!(apply_filter(events.clone(), &f).len(), 3);
        // 正则与字面退路。
        let re_hit = events.iter().find(|e| search_matches(e, r"fn v3")).unwrap();
        assert_eq!(re_hit.session_id, "s3");
        // "v3(" 是非法正则（未闭合分组），退字面子串仍命中 patch 里的 "fn v3() {}"。
        assert!(events.iter().any(|e| search_matches(e, "v3(")));
        assert_eq!(f.clamp_limit(), 3);
    }
}
