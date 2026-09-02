use std::path::{Path, PathBuf};

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
