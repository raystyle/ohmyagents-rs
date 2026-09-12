//! 过渡期 oma stub（D29，v0.6.0 一个版本后随 oma/ 镜像段退役）：旧 `oma`
//! 命令警告更名后转调 `hst`。转调序：同目录姊妹 hst 加 PATH 上的 hst；
//! 都缺则打安装指路退出 1。让旧 `oma self update` 升到本 stub 后被引导到
//! hst，而不是 404 断链。

fn main() {
    eprintln!("oma has been renamed to hst (Hooks, Statusline, Trace); forwarding...");
    let exe = std::env::current_exe().ok();
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(p) = &exe {
        if let Some(dir) = p.parent() {
            let hst = if cfg!(windows) {
                dir.join("hst.exe")
            } else {
                dir.join("hst")
            };
            candidates.push(hst);
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            candidates.push(if cfg!(windows) {
                dir.join("hst.exe")
            } else {
                dir.join("hst")
            });
        }
    }
    for c in candidates {
        if c.is_file() {
            let err = std::process::Command::new(&c)
                .args(std::env::args().skip(1))
                .status()
                .map_err(|e| e.to_string())
                .and_then(|s| {
                    if s.success() {
                        Ok(())
                    } else {
                        Err(format!("exit {}", s.code().unwrap_or(-1)))
                    }
                })
                .err();
            if let Some(e) = err {
                eprintln!("hst invocation failed: {e}");
                std::process::exit(1);
            }
            return;
        }
    }
    eprintln!("hst not found; install it from https://github.com/raystyle/hst_rs/releases");
    std::process::exit(1);
}
