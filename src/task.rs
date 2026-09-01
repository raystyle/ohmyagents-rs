//! oma task：带产物等待的任务委派（用户定调 2026-09-01，学 reader_rs 的
//! SKILL 与输出契约形态）。
//!
//! 任务目录协议（`<project>\.ohmyagents\tasks\<id>\`）：
//! - `task.json`：元数据（id / agent / text / created / status）
//! - `prompt.md`：完整任务提示词（oma 写入，agent 可重读）
//! - `output.md`：任务产物（agent 写入）
//! - `DONE`：完成标记空文件（agent **最后**创建）——oma 只认 DONE 不认
//!   output 存在，防半写误触发
//!
//! `oma task <agent> "<文本>"` 即调即入等待：建目录、发任务（send 文本带
//! 协议尾注），随后**阻塞轮询 DONE**（后台阻塞命令形态：需要后台就放
//! shell 后台，oma 侧语义是「等到产物才算完」）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::orch;

fn tasks_dir(root: &Path) -> PathBuf {
    root.join(".ohmyagents").join("tasks")
}

fn task_dir(root: &Path, id: &str) -> PathBuf {
    tasks_dir(root).join(id)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskMeta {
    pub id: String,
    pub agent: String,
    pub text: String,
    pub created: u64,
}

/// 分配任务目录 id：scan 取 max + 1，目录 create 占位原子防撞（同
/// alloc_task_id 思路，目录形态；平文件 tNNN.json 在同一 scan 里兼容）。
fn alloc_task_dir(root: &Path) -> Result<String, String> {
    let dir = tasks_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut max = 0u32;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let stem = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if let Some(num) = stem.strip_prefix('t').and_then(|n| n.parse::<u32>().ok()) {
                max = max.max(num);
            }
        }
    }
    let mut n = max + 1;
    loop {
        let id = format!("t{n:03}");
        match std::fs::create_dir(task_dir(root, &id)) {
            Ok(()) => return Ok(id),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                n += 1;
                continue;
            }
            Err(e) => return Err(format!("task dir: {e}")),
        }
    }
}

/// 建任务并发给 agent。返回 (id, task_dir)。send 文本 = 用户文本 + 协议
/// 尾注（提示词全文在 prompt.md，agent 无论从 send 还是文件都能拿到任务）。
pub async fn task_new(
    root: &Path,
    agent: &str,
    text: &str,
) -> Result<(String, PathBuf), String> {
    let id = alloc_task_dir(root)?;
    let dir = task_dir(root, &id);
    let meta = TaskMeta {
        id: id.clone(),
        agent: agent.to_string(),
        text: text.to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let meta_body = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())? + "\n";
    std::fs::write(dir.join("task.json"), meta_body)
        .map_err(|e| format!("task.json: {e}"))?;
    std::fs::write(dir.join("prompt.md"), text).map_err(|e| format!("prompt.md: {e}"))?;

    let link = orch::connect(root, false).await?;
    let note = format!(
        "{text}\n\n（任务协议：提示词全文在 .ohmyagents/{id}/prompt.md；产物写到 .ohmyagents/{id}/output.md；写完最后创建空文件 .ohmyagents/{id}/DONE 表示完成）",
        id = format!("tasks/{id}"),
    );
    if let Err(e) = orch::send(&link, root, agent, &note, None).await {
        // send 失败回滚任务目录（codex 五轮低6：残留目录会被 task list 当
        // 未完成挂在清单里）。
        let _ = std::fs::remove_dir_all(&dir);
        return Err(e);
    }
    Ok((id, dir))
}

/// 阻塞等 DONE 出现，然后读产物。timeout_secs 为 0 表示无限等。
/// 返回产物文本；超时返回 Err（task 目录保留，产物晚到仍可 `task show` 收）。
pub fn task_wait(root: &Path, id: &str, timeout_secs: u64) -> Result<String, String> {
    let dir = task_dir(root, id);
    let done = dir.join("DONE");
    let output = dir.join("output.md");
    let deadline = if timeout_secs == 0 {
        None
    } else {
        Some(std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs))
    };
    loop {
        if done.exists() {
            // DONE 在但 output.md 缺失/空（违规 agent 先建 DONE 或还在追加，
            // codex 五轮低5）：给 3s 宽限再读，仍缺则报错不返回半截。
            if !output.exists() || output.metadata().map(|m| m.len() == 0).unwrap_or(true) {
                let grace = std::time::Instant::now() + std::time::Duration::from_secs(3);
                while std::time::Instant::now() < grace {
                    if output.exists() && output.metadata().map(|m| m.len() > 0).unwrap_or(false) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
            return std::fs::read_to_string(&output)
                .map_err(|e| format!("DONE present but output.md unreadable: {e}"));
        }
        if let Some(d) = deadline {
            if std::time::Instant::now() > d {
                return Err(format!(
                    "task {id}: no DONE within {timeout_secs}s (agent still working or stuck; retry `oma task show {id}` later)"
                ));
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}

/// 列任务：每条 (id, agent, done?)。
pub fn task_list(root: &Path) -> Result<Vec<(String, String, bool)>, String> {
    let dir = tasks_dir(root);
    let mut out = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Ok(out),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let Some(id) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let agent = std::fs::read_to_string(p.join("task.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<TaskMeta>(&t).ok())
            .map(|m| m.agent)
            .unwrap_or_else(|| "-".into());
        out.push((id.to_string(), agent, p.join("DONE").exists()));
    }
    out.sort();
    Ok(out)
}

/// 看一个任务：meta + 产物（有则全量返回）。
pub fn task_show(root: &Path, id: &str) -> Result<TaskMeta, String> {
    let dir = task_dir(root, id);
    let text = std::fs::read_to_string(dir.join("task.json"))
        .map_err(|e| format!("task {id}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("task {id}: corrupt meta: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-task-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn alloc_is_unique_and_monotonic() {
        let root = tmp_root();
        assert_eq!(alloc_task_dir(&root).unwrap(), "t001");
        assert_eq!(alloc_task_dir(&root).unwrap(), "t002");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn wait_returns_output_after_done_marker() {
        let root = tmp_root();
        let id = alloc_task_dir(&root).unwrap();
        let dir = task_dir(&root, &id);
        // 先有产物、无 DONE：wait 超时不误判（半写保护）。
        std::fs::write(dir.join("output.md"), "half-written").unwrap();
        assert!(task_wait(&root, &id, 1).is_err());
        std::fs::write(dir.join("DONE"), "").unwrap();
        assert_eq!(task_wait(&root, &id, 1).unwrap(), "half-written");
        let _ = std::fs::remove_dir_all(&root);
    }
}
