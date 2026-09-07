use std::path::{Path, PathBuf};

/// 项目与家目录下的 oma 数据目录名（D14，2026-09-07 用户裁定）。
pub const DIR: &str = ".oma";
/// 2026-08-31 至 D14 的旧名。仅旧在、新不在时改名迁过去。
pub const LEGACY_DIR: &str = ".ohmyagents";

/// `<parent>/.oma`。仅有旧名则 rename；两者都在则用新名、旧目录不动。
pub fn data_dir(parent: &Path) -> PathBuf {
    let neu = parent.join(DIR);
    if neu.exists() {
        return neu;
    }
    let old = parent.join(LEGACY_DIR);
    if old.exists() {
        if std::fs::rename(&old, &neu).is_ok() {
            return neu;
        }
        return old;
    }
    neu
}

/// 项目侧数据根：`<project>/.oma`（session / state / tasks）。
pub fn project_dir(root: &Path) -> PathBuf {
    data_dir(root)
}

/// Absolute path without Windows `\\?\` prefix. Agent trust stores use this shape.
pub fn abs_display(path: &Path) -> PathBuf {
    let canon = std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    });
    let raw = canon.to_string_lossy();
    let stripped = raw
        .strip_prefix(r"\\?\")
        .or_else(|| raw.strip_prefix("//?/"))
        .unwrap_or(&raw);
    PathBuf::from(stripped)
}

pub fn native_slash(path: &Path) -> String {
    abs_display(path).to_string_lossy().into_owned()
}

pub fn forward_slash(path: &Path) -> String {
    native_slash(path).replace('\\', "/")
}

#[cfg(windows)]
pub fn norm_key(s: &str) -> String {
    s.replace('/', "\\").to_ascii_lowercase()
}

#[cfg(not(windows))]
pub fn norm_key(s: &str) -> String {
    s.replace('\\', "/").to_string()
}

pub fn keys_match(a: &str, b: &str) -> bool {
    norm_key(a) == norm_key(b)
}

/// Resolve an executable name through PATH (`which` handles PATHEXT on
/// Windows, so "oma" finds oma.exe). None when absent — callers decide the
/// fallback. No oma-specific policy here.
pub fn find_on_path(name: &str) -> Option<PathBuf> {
    which::which(name).ok().map(|p| abs_display(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn tmp_parent() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-pathutil-{}-{}-{}",
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
    fn data_dir_renames_legacy_when_new_absent() {
        let parent = tmp_parent();
        let old = parent.join(LEGACY_DIR);
        std::fs::create_dir_all(old.join("state")).unwrap();
        std::fs::write(old.join("session.json"), "{}").unwrap();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(got.join("session.json").is_file());
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn data_dir_prefers_new_when_both_exist() {
        let parent = tmp_parent();
        std::fs::create_dir_all(parent.join(DIR)).unwrap();
        std::fs::create_dir_all(parent.join(LEGACY_DIR)).unwrap();
        std::fs::write(parent.join(DIR).join("marker"), "new").unwrap();
        std::fs::write(parent.join(LEGACY_DIR).join("marker"), "old").unwrap();
        let got = data_dir(&parent);
        assert_eq!(std::fs::read_to_string(got.join("marker")).unwrap(), "new");
        assert!(parent.join(LEGACY_DIR).exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn data_dir_defaults_to_new_when_neither_exists() {
        let parent = tmp_parent();
        let got = data_dir(&parent);
        assert_eq!(got.file_name().unwrap(), DIR);
        assert!(!got.exists());
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[test]
    fn find_on_path_missing_name_is_none() {
        assert!(find_on_path("oma-cargo-test-no-such-bin").is_none());
    }

    #[test]
    fn find_on_path_platform_shell_is_some() {
        // Expectation from the platform contract (cmd on Windows, sh
        // elsewhere), not from the implementation under test.
        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(find_on_path(shell).is_some());
    }

    #[test]
    fn keys_match_slash_and_case() {
        assert!(keys_match(r"D:\foo\bar", "D:/foo/bar"));
        assert!(keys_match(r"d:\Foo", r"D:\foo") || !cfg!(windows));
        if cfg!(windows) {
            assert!(keys_match(r"D:\Foo", r"d:/foo"));
        }
    }
}
