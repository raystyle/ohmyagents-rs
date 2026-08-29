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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_slash_and_case() {
        assert!(keys_match(r"D:\foo\bar", "D:/foo/bar"));
        assert!(keys_match(r"d:\Foo", r"D:\foo") || !cfg!(windows));
        if cfg!(windows) {
            assert!(keys_match(r"D:\Foo", r"d:/foo"));
        }
    }
}
