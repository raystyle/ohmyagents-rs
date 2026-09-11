# -*- coding: utf-8 -*-
# 一次性编辑：doctor 三家 yolo 双级冲突告警加 CTA（D28 第 4 令）。
import io

p = 'src/doctor.rs'
s = io.open(p, encoding='utf-8').read()

old = """    let claude_mode = json_file(&claude_shared)
        .as_ref()
        .and_then(|v| v.get("permissions"))
        .and_then(|p| p.get("defaultMode"))
        .and_then(|m| m.as_str())
        .map(str::to_string)
        .or_else(|| {
            json_file(&claude_user_yolo)
                .as_ref()
                .and_then(|v| v.get("permissions"))
                .and_then(|p| p.get("defaultMode"))
                .and_then(|m| m.as_str())
                .map(str::to_string)
        });
    let yolo_claude = claude_mode.as_deref() == Some("bypassPermissions");
    push(
        &mut findings,
        "claude",
        "yolo",
        yolo_claude,
        &claude_user_yolo,
        if yolo_claude {
            format!(
                "defaultMode=bypassPermissions ({} level)",
                if claude_shared.exists() { "project" } else { "user" }
            )
        } else {
            "missing permissions.defaultMode=bypassPermissions (tool prompt will block)".into()
        },
    );"""
new = """    let claude_mode_at = |p: &Path| -> Option<String> {
        json_file(p)
            .as_ref()
            .and_then(|v| v.get("permissions"))
            .and_then(|pp| pp.get("defaultMode"))
            .and_then(|m| m.as_str())
            .map(str::to_string)
    };
    let claude_proj_mode = claude_mode_at(&claude_shared);
    let claude_user_mode = claude_mode_at(&claude_user_yolo);
    match (&claude_proj_mode, &claude_user_mode) {
        // 双级冲突（D28 第 4 令）：项目遮蔽用户，warn 加对齐 CTA。
        (Some(p), Some(u)) if p != u => push_status(
            &mut findings,
            "claude",
            "yolo",
            Status::Warn,
            &claude_shared,
            format!(
                "conflict: project defaultMode={p} shadows user {u}; align via \
                 `oma init --project-yolo` or drop one level (D28 r4)"
            ),
        ),
        (Some(p), Some(_)) | (Some(p), None) => push(
            &mut findings,
            "claude",
            "yolo",
            p == "bypassPermissions",
            &claude_shared,
            format!("defaultMode={p} (project level)"),
        ),
        (None, Some(u)) => push(
            &mut findings,
            "claude",
            "yolo",
            u == "bypassPermissions",
            &claude_user_yolo,
            format!("defaultMode={u} (user level)"),
        ),
        (None, None) => push(
            &mut findings,
            "claude",
            "yolo",
            false,
            &claude_user_yolo,
            "missing permissions.defaultMode=bypassPermissions (tool prompt will block)",
        ),
    }"""
assert old in s, "claude cta"
s = s.replace(old, new)

old = """    let codex_yolo_at = |t: Option<&Toml>| -> bool {
        t.is_some_and(|t| {
            toml_str(t, "sandbox_mode") == Some("danger-full-access")
                && toml_str(t, "approval_policy") == Some("never")
        })
    };
    let yolo_codex = codex_yolo_at(proj_toml.as_ref()) || codex_yolo_at(user_toml.as_ref());
    push(
        &mut findings,
        "codex",
        "yolo",
        yolo_codex,
        &codex_user,
        if yolo_codex {
            format!(
                "sandbox_mode=danger-full-access approval_policy=never ({} level)",
                if codex_yolo_at(proj_toml.as_ref()) {
                    "project"
                } else {
                    "user"
                }
            )
        } else {
            "missing sandbox/approval yolo keys".into()
        },
    );"""
new = """    let codex_pair_at = |t: Option<&Toml>| -> Option<(String, String)> {
        let t = t?;
        Some((
            toml_str(t, "sandbox_mode")?.to_string(),
            toml_str(t, "approval_policy")?.to_string(),
        ))
    };
    let codex_proj_pair = codex_pair_at(proj_toml.as_ref());
    let codex_user_pair = codex_pair_at(user_toml.as_ref());
    let codex_yolo_pair =
        |p: &(String, String)| p == &("danger-full-access".to_string(), "never".to_string());
    match (&codex_proj_pair, &codex_user_pair) {
        (Some(p), Some(u)) if p != u => push_status(
            &mut findings,
            "codex",
            "yolo",
            Status::Warn,
            &codex_proj,
            format!(
                "conflict: project sandbox/approval shadows user; align via \
                 `oma init --project-yolo` or drop one level (D28 r4)"
            ),
        ),
        (Some(p), _) => push(
            &mut findings,
            "codex",
            "yolo",
            codex_yolo_pair(p),
            &codex_proj,
            format!("sandbox/approval (project level): {} {}", p.0, p.1),
        ),
        (None, Some(u)) => push(
            &mut findings,
            "codex",
            "yolo",
            codex_yolo_pair(u),
            &codex_user,
            format!("sandbox/approval (user level): {} {}", u.0, u.1),
        ),
        (None, None) => push(
            &mut findings,
            "codex",
            "yolo",
            false,
            &codex_user,
            "missing sandbox/approval yolo keys",
        ),
    }"""
assert old in s, "codex cta"
s = s.replace(old, new)

old = """    let kimi_mode = toml_file(&kimi_proj)
        .as_ref()
        .and_then(|t| toml_str(t, "default_permission_mode").map(|s| s.to_string()))
        .or_else(|| {
            toml_file(&kimi_user)
                .as_ref()
                .and_then(|t| toml_str(t, "default_permission_mode").map(|s| s.to_string()))
        });
    let yolo_kimi = matches!(kimi_mode.as_deref(), Some("yolo" | "auto"));
    push(
        &mut findings,
        "kimi",
        "yolo",
        yolo_kimi,
        &kimi_user,
        match kimi_mode.as_deref() {
            Some(m) => format!("default_permission_mode={m}"),
            None => "missing default_permission_mode auto|yolo".into(),
        },
    );"""
new = """    let kimi_mode_at = |p: &Path| -> Option<String> {
        toml_file(p).and_then(|t| {
            t.get("default_permission_mode")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
    };
    let kimi_proj_mode = kimi_mode_at(&kimi_proj);
    let kimi_user_mode = kimi_mode_at(&kimi_user);
    let kimi_ok = |m: &str| matches!(m, "yolo" | "auto");
    match (&kimi_proj_mode, &kimi_user_mode) {
        (Some(p), Some(u)) if p != u => push_status(
            &mut findings,
            "kimi",
            "yolo",
            Status::Warn,
            &kimi_proj,
            format!(
                "conflict: project default_permission_mode={p} shadows user {u}; align via \
                 `oma init --project-yolo` or drop one level (D28 r4)"
            ),
        ),
        (Some(p), _) => push(
            &mut findings,
            "kimi",
            "yolo",
            kimi_ok(p),
            &kimi_proj,
            format!("default_permission_mode={p} (project level)"),
        ),
        (None, Some(u)) => push(
            &mut findings,
            "kimi",
            "yolo",
            kimi_ok(u),
            &kimi_user,
            format!("default_permission_mode={u} (user level)"),
        ),
        (None, None) => push(
            &mut findings,
            "kimi",
            "yolo",
            false,
            &kimi_user,
            "missing default_permission_mode auto|yolo",
        ),
    }"""
assert old in s, "kimi cta"
s = s.replace(old, new)
io.open(p, 'w', encoding='utf-8', newline='').write(s)
print("CTA done")
