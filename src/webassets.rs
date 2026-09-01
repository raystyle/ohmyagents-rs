//! kanban 前端资源包（P0023，S022 路线 C）：build.rs 打的 tar.gz 在编译期嵌进
//! 二进制，首次（或指纹变更）运行时释放到 oma 自管数据根 `~/.ohmyagents/web/
//! <指纹>/`，serve 从那里托管。单 exe 自带资源，克隆仓不克隆仓都能跑。

use std::io::Read;
use std::path::{Path, PathBuf};

static KANBAN_TGZ: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kanban-web.tar.gz"));
static KANBAN_FP: &str = include_str!(concat!(env!("OUT_DIR"), "/kanban-web.fingerprint"));

/// 资源指纹（内容寻址目录名）。
pub fn fingerprint() -> &'static str {
    KANBAN_FP.trim()
}

/// 释放资源包到 `<home>/web/<指纹>/`：指纹目录已存在且完整则直接复用；
/// 否则清掉 web/ 下其它指纹目录再解压（一次一份，不积堆）。
/// 参数化 home 便于测试（serve 调用时传 `oma_home()`）。
pub fn ensure_web_assets_at(home: &Path) -> Result<PathBuf, String> {
    let web_root = home.join("web");
    let target = web_root.join(fingerprint());
    let intact = target.join("index.html").is_file();
    std::fs::create_dir_all(&web_root).map_err(|e| format!("{}: {e}", web_root.display()))?;
    // 一次一份：无论复用还是重释放，都清掉非当前指纹的目录（失败不致命）。
    if let Ok(rd) = std::fs::read_dir(&web_root) {
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() && p.file_name().and_then(|n| n.to_str()) != Some(fingerprint()) {
                let _ = std::fs::remove_dir_all(&p);
            }
        }
    }
    if intact {
        return Ok(target);
    }
    std::fs::create_dir_all(&target).map_err(|e| format!("{}: {e}", target.display()))?;
    extract_tar_gz_memory(KANBAN_TGZ, &target)?;
    Ok(target)
}

/// 内存解压（tar.gz 字节 → 目录）：与 install.rs 文件版同一纪律，错误带路径。
fn extract_tar_gz_memory(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .map_err(|e| format!("unpack kanban web assets to {}: {e}", dest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    #[test]
    fn fingerprint_is_eight_hex_chars() {
        let fp = fingerprint();
        assert_eq!(fp.len(), 8, "sha256 前 8 位");
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()), "{fp}");
    }

    #[test]
    fn release_then_idempotent_then_replace() {
        let home = std::env::temp_dir().join(format!(
            "oma-webassets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        // 1) 首次释放：目录出现且 index.html 可读。
        let dir = ensure_web_assets_at(&home).unwrap();
        assert!(dir.join("index.html").is_file());
        assert!(dir.ends_with(fingerprint()));
        // 2) 幂等：再跑返回同一路径，不重解压。
        let again = ensure_web_assets_at(&home).unwrap();
        assert_eq!(dir, again);
        // 3) 污染指纹目录后自动重建（index 被删视为不完整）。
        std::fs::remove_file(dir.join("index.html")).unwrap();
        let rebuilt = ensure_web_assets_at(&home).unwrap();
        assert_eq!(dir, rebuilt);
        assert!(dir.join("index.html").is_file());
        // 4) 旧指纹不积堆：伪造旧目录后再释放，只留当前指纹。
        let stale = home.join("web").join("deadbeef");
        std::fs::create_dir_all(&stale).unwrap();
        ensure_web_assets_at(&home).unwrap();
        assert!(!stale.exists());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn archive_carries_wasm_entry() {
        // E2EE 的 wasm 是空白事故的根（P0022）：包内必须带 _astro 目录。
        let gz = flate2::read::GzDecoder::new(KANBAN_TGZ);
        let mut archive = tar::Archive::new(gz);
        let entries = archive.entries().unwrap();
        let has_astro = entries
            .filter_map(Result::ok)
            .any(|e| e.path().map(|p| p.starts_with("_astro")).unwrap_or(false));
        assert!(has_astro, "kanban 资源包缺 _astro 目录");
    }
}
