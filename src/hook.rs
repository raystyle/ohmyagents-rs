use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value as Json};

use crate::pathutil::keys_match;

/// Map a hook event (already normalized) to a four-state label.
pub fn map_event(event: &str) -> &'static str {
    match event {
        "session" | "sessionstart" | "idle" | "stop" | "interrupt" | "sessionend" => "idle",
        "userpromptsubmit" | "userpromptuse" | "pretooluse" | "posttooluse"
        | "posttoolusefailure" | "subagentstart" | "subagentstop" | "precompact"
        | "permissionresult" | "working" => "working",
        "permissionrequest" | "blocked" => "blocked",
        "notification" | "unknown" => "unknown",
        _ => "unknown",
    }
}

fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn event_from_payload(v: &Json) -> String {
    let raw = v
        .get("hook_event_name")
        .or_else(|| v.get("hookEventName"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    normalize(raw)
}

fn notification_kind(v: &Json) -> String {
    let raw = v
        .get("notification_type")
        .or_else(|| v.get("notificationType"))
        .or_else(|| v.get("matcher"))
        .and_then(|x| x.as_str())
        .unwrap_or("");
    normalize(raw)
}

/// Claude Notification is mixed (tips vs permission). Only permission-shaped
/// kinds count as blocked; the rest stay unknown so we do not spur idle.
pub fn state_for_payload(event: &str, payload: Option<&Json>) -> &'static str {
    if event == "notification" {
        if let Some(v) = payload {
            let kind = notification_kind(v);
            if kind.contains("permission") || kind.contains("elicitation") {
                return "blocked";
            }
        }
        return "unknown";
    }
    if event == "elicitation" || event == "elicitationresult" {
        return "blocked";
    }
    map_event(event)
}

fn env_nonempty(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(s) if !s.is_empty() => Some(s),
        _ => None,
    }
}

fn project_allows(payload: Option<&Json>) -> bool {
    let Some(project) = env_nonempty("OHMYAGENTS_PROJECT") else {
        return true;
    };
    let Some(v) = payload else {
        return true;
    };
    let Some(cwd) = v.get("cwd").and_then(|x| x.as_str()) else {
        return true;
    };
    keys_match(cwd, &project)
        || cwd
            .replace('\\', "/")
            .starts_with(&project.replace('\\', "/"))
        || Path::new(cwd).starts_with(&project)
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn atomic_write(path: &Path, body: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })
}

fn read_stdin_json() -> Option<Json> {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).ok()?;
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}

/// Hook entry: always exit-path friendly. Missing env means "not our session".
pub fn run(event_arg: Option<&str>) -> Result<Option<PathBuf>, String> {
    let Some(state_file) = env_nonempty("OHMYAGENTS_STATE_FILE") else {
        return Ok(None);
    };
    let payload = if event_arg.is_some() {
        None
    } else {
        read_stdin_json()
    };
    if !project_allows(payload.as_ref()) {
        return Ok(None);
    }
    let event = if let Some(arg) = event_arg {
        normalize(arg)
    } else if let Some(ref v) = payload {
        event_from_payload(v)
    } else {
        String::new()
    };
    if event.is_empty() {
        return Ok(None);
    }
    let state = state_for_payload(&event, payload.as_ref());
    let agent = env_nonempty("OHMYAGENTS_AGENT").unwrap_or_default();
    let record = json!({
        "state": state,
        "event": event,
        "agent": agent,
        "ts": unix_secs(),
    });
    let path = PathBuf::from(state_file);
    let body = serde_json::to_string(&record).map_err(|e| e.to_string())? + "\n";
    atomic_write(&path, &body)?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn maps_core_events() {
        assert_eq!(map_event("userpromptsubmit"), "working");
        assert_eq!(map_event("stop"), "idle");
        assert_eq!(map_event("permissionrequest"), "blocked");
        assert_eq!(map_event("notification"), "unknown");
        assert_eq!(map_event("sessionstart"), "idle");
    }

    #[test]
    fn permission_notification_is_blocked() {
        let v = json!({
            "hook_event_name": "Notification",
            "notification_type": "permission_prompt"
        });
        assert_eq!(state_for_payload("notification", Some(&v)), "blocked");
        let tips = json!({
            "hookEventName": "notification",
            "notificationType": "idle_prompt"
        });
        assert_eq!(state_for_payload("notification", Some(&tips)), "unknown");
    }

    #[test]
    fn grok_camel_case_normalizes() {
        let v = json!({ "hookEventName": "user_prompt_submit" });
        assert_eq!(event_from_payload(&v), "userpromptsubmit");
        assert_eq!(
            state_for_payload(&event_from_payload(&v), Some(&v)),
            "working"
        );
    }

    #[test]
    fn run_is_silent_without_env() {
        let _g = ENV_LOCK.lock().unwrap();
        env::remove_var("OHMYAGENTS_STATE_FILE");
        assert_eq!(run(Some("blocked")).unwrap(), None);
    }

    #[test]
    fn run_writes_blocked() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir =
            std::env::temp_dir().join(format!("oma-hook-{}-{}", std::process::id(), unix_secs()));
        let file = dir.join("claude.json");
        env::set_var("OHMYAGENTS_STATE_FILE", &file);
        env::set_var("OHMYAGENTS_AGENT", "claude");
        let wrote = run(Some("PermissionRequest")).unwrap();
        env::remove_var("OHMYAGENTS_STATE_FILE");
        env::remove_var("OHMYAGENTS_AGENT");
        assert_eq!(wrote.as_deref(), Some(file.as_path()));
        let v: Json = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(v["state"], "blocked");
        assert_eq!(v["event"], "permissionrequest");
        let _ = fs::remove_dir_all(&dir);
    }
}
