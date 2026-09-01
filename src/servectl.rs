//! serve 守护化（P0025）：`oma serve` 即调即退——后台拉起编排面进程
//! （Windows CREATE_NO_WINDOW + CREATE_NEW_PROCESS_GROUP，日志重定向文
//! 件；**不用 DETACHED_PROCESS**——零控制台下 rmux CLI 子进程会卡死，
//! P0026 实证），状态记 `~/.ohmyagents/serve/<slug>.json`；`stop` 协议化
//! 优先（DELETE /shutdown）；`status` 探活。与 rmux 的 hidden daemon 同
//! 一进程模型（S023）。

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServeRecord {
    pub pid: u32,
    pub port: u16,
    pub project: String,
    pub started_at: u64,
}

fn slug(root: &Path) -> String {
    crate::orch::project_slug(root)
}

fn record_path(root: &Path) -> PathBuf {
    crate::install::oma_home()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("serve")
        .join(format!("{}.json", slug(root)))
}

fn log_path(root: &Path) -> PathBuf {
    crate::install::oma_home()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("serve")
        .join(format!("{}.log", slug(root)))
}

fn read_record(root: &Path) -> Option<ServeRecord> {
    let text = std::fs::read_to_string(record_path(root)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_record(root: &Path, rec: &ServeRecord) -> Result<(), String> {
    let p = record_path(root);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let body = serde_json::to_string_pretty(rec).map_err(|e| e.to_string())? + "\n";
    std::fs::write(&p, body).map_err(|e| format!("{}: {e}", p.display()))
}

fn remove_record(root: &Path) {
    let _ = std::fs::remove_file(record_path(root));
}

/// pid 是否活着。Windows 走 FFI OpenProcess（免 tasklist 子进程开销与
/// Job Object 下 stdout 死锁的坑——DETACHED 子进程的管道在宿主 Job 内
/// 会被塞住，`.output()` 永远等不回来）。
pub fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return false;
        }
        let mut exit_code: u32 = 0;
        let ok = unsafe { GetExitCodeProcess(handle, &mut exit_code) };
        unsafe { CloseHandle(handle) };
        ok != 0 && exit_code == STILL_ACTIVE
    }
    #[cfg(not(windows))]
    {
        // Unix kill 语义里 0 指「本进程组」，kill -0 0 会成功；超过
        // i32::MAX 的 pid 经 kill CLI 的 int 解析会回绕成负数（u32::MAX
        // 即 -1，kill -0 -1 广播成功）。两者都不是真实进程，按不活处理，
        // 与 Windows OpenProcess 对非法 pid 失败的语义对齐。合法 Linux
        // pid 上限 4194304（2^22），远小于 i32::MAX，不受影响。
        if pid == 0 || pid > i32::MAX as u32 {
            return false;
        }
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(windows)]
const STILL_ACTIVE: u32 = 259;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> *mut core::ffi::c_void;
    fn GetExitCodeProcess(handle: *mut core::ffi::c_void, exit_code: *mut u32) -> i32;
    fn CloseHandle(handle: *mut core::ffi::c_void) -> i32;
}

/// 已活检测：记录存在 + pid 活 + 端口在听。
fn live_record(root: &Path) -> Option<ServeRecord> {
    let rec = read_record(root)?;
    if !pid_alive(rec.pid) {
        return None;
    }
    // 端口探测：TCP 连一下（127.0.0.1:port 的 /api）。
    if !port_listening(rec.port) {
        return None;
    }
    Some(rec)
}

fn port_listening(port: u16) -> bool {
    std::net::TcpStream::connect(("127.0.0.1", port)).is_ok()
}

/// 后台启动：已活直接返回 addr；否则孤儿化拉起并等端口就绪。
pub fn serve_start(root: &Path, port: u16) -> Result<String, String> {
    if let Some(rec) = live_record(root) {
        return Ok(format!("http://127.0.0.1:{}", rec.port));
    }
    remove_record(root);

    let exe = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let log = log_path(root);
    if let Some(dir) = log.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let log_file = std::fs::File::options()
        .create(true)
        .append(true)
        .open(&log)
        .map_err(|e| format!("{}: {e}", log.display()))?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.args(["serve-daemon", "--port", &port.to_string(), "--project"])
        .arg(root)
        .stdout(log_file.try_clone().map_err(|e| e.to_string())?)
        .stderr(log_file);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW 而非 DETACHED_PROCESS：后者零控制台，daemon 再
        // spawn 的 rmux CLI（TUI 程序，初始化要碰 console）会卡死在控制台
        // API 上——GET / 起镜像时整个 serve 挂死（P0026 切片 1 实证：前台
        // 同代码同 manifest 秒回）。CREATE_NO_WINDOW 给隐藏 conhost，子进
        // 程 console 可用且不可见。
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
    }
    #[cfg(unix)]
    {
        // Windows CREATE_NEW_PROCESS_GROUP 的等价物：daemon 进独立进程组，
        // 终端关闭的 SIGHUP 打不到它（无 Job Object，不需要逃逸，只防挂断）。
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("spawn serve daemon: {e}"))?;
    let pid = child.id();
    // 故意不 wait：daemon 必须活得过这个短命客户端（rmux 同款，S023）。
    drop(child);

    // 等端口就绪（最长 10s）。
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if port_listening(port) {
            let rec = ServeRecord {
                pid,
                port,
                project: root.display().to_string(),
                started_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            };
            write_record(root, &rec)?;
            return Ok(format!("http://127.0.0.1:{port}"));
        }
        if !pid_alive(pid) {
            let tail = std::fs::read_to_string(&log)
                .map(|t| t.lines().rev().take(3).collect::<Vec<_>>().join(" | "))
                .unwrap_or_default();
            return Err(format!("serve daemon exited early; log tail: {tail}"));
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    Err(format!("serve daemon port {port} not ready in 10s; log: {}", log.display()))
}

/// 停止：协议化优先（`DELETE /shutdown` 请 daemon 优雅排空自退，rmux
/// kill-server 同构），等不到再降级强杀（Windows taskkill /T 连子进程；
/// Unix kill）。ureq 本就是装机链的非可选依赖，featureless 构建同样走
/// 优雅路。
pub fn serve_stop(root: &Path) -> Result<bool, String> {
    let Some(rec) = read_record(root) else {
        return Ok(false);
    };
    if pid_alive(rec.pid) {
        // 1. 协议化：让 daemon 自己排空在途请求后退出。
        let asked = ureq::delete(&format!("http://127.0.0.1:{}/shutdown", rec.port))
            .timeout(Duration::from_secs(2))
            .call()
            .is_ok();
        // 2. 等它死（最长 5s）。
        if asked {
            let deadline = std::time::Instant::now() + Duration::from_secs(5);
            while std::time::Instant::now() < deadline {
                if !pid_alive(rec.pid) {
                    remove_record(root);
                    return Ok(true);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
        // 3. 兜底：协议路不通或超时未退，强杀。
        let forced = if cfg!(windows) {
            std::process::Command::new("taskkill")
                .args(["/PID", &rec.pid.to_string(), "/T", "/F"])
                .output()
                .map(|o| o.status.success())
                .map_err(|e| format!("taskkill: {e}"))?
        } else {
            std::process::Command::new("kill")
                .arg(rec.pid.to_string())
                .output()
                .map(|o| o.status.success())
                .map_err(|e| format!("kill: {e}"))?
        };
        remove_record(root);
        return Ok(forced);
    }
    remove_record(root);
    Ok(true)
}

/// 状态：`(活?, 记录)`。
pub fn serve_status(root: &Path) -> (bool, Option<ServeRecord>) {
    let rec = read_record(root);
    let live = rec.as_ref().map(|r| pid_alive(r.pid) && port_listening(r.port)).unwrap_or(false);
    (live, rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_zero_is_not_alive() {
        assert!(!pid_alive(0));
        assert!(!pid_alive(u32::MAX));
    }
}
