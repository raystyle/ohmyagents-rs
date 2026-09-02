//! oma 自更新（用户定调 2026-09-02：去 GitHub 升级新版本；封版前本地测试，
//! releases 为空时走 `--git` 源码安装路径）。
//! 机制见 S028：releases/latest API、资产命名约定 `oma-<triple>.(zip|tar.gz)`、
//! Windows 运行中自替换（rename 舞步）、Unix 原子 rename 覆盖。

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Default repo; `--repo owner/name` overrides.
pub const DEFAULT_REPO: &str = "raystyle/OhMyAgents";

const UA: &str = concat!("oma/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
    /// false for prereleases/drafts (latest already excludes them).
    #[serde(default = "default_true")]
    pub draft: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    /// GitHub 资产摘要（新 API 形如 "sha256:<hex>"；旧响应可能缺省）。
    #[serde(default)]
    pub digest: Option<String>,
}

/// 更新通道：dev = 滚动预发布 tag `dev`（CI 每推覆盖，部署位缺省）；
/// latest = 正式封版后的 releases/latest。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Dev,
    Latest,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Dev => "dev",
            Channel::Latest => "latest",
        }
    }
}

pub fn fetch_release(repo: &str, channel: Channel) -> Result<Release, String> {
    let url = match channel {
        Channel::Latest => format!("https://api.github.com/repos/{repo}/releases/latest"),
        Channel::Dev => format!("https://api.github.com/repos/{repo}/releases/tags/dev"),
    };
    let resp = ureq::get(&url)
        .set("User-Agent", UA)
        .set("Accept", "application/vnd.github+json")
        .call();
    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return Err(format!(
                "release '{ch}' not published for {repo} yet (pre-release phase); use `oma self update --git`",
                ch = channel.as_str()
            ))
        }
        Err(e) => return Err(format!("github api: {e}")),
    };
    resp.into_string()
        .map_err(|e| format!("read body: {e}"))
        .and_then(|text| serde_json::from_str(&text).map_err(|e| format!("parse release: {e}")))
}

/// Host target triple keywords for asset matching (keep in sync with the
/// release asset naming convention: oma-<triple>.zip / .tar.gz).
fn host_keywords() -> &'static [&'static str] {
    if cfg!(target_os = "windows") {
        &["windows-msvc", "windows"]
    } else if cfg!(target_os = "macos") {
        &["apple-darwin", "darwin"]
    } else {
        &["linux-gnu", "linux"]
    }
}

/// Pick the oma asset for this host from a release's asset list.
pub fn pick_asset(assets: &[Asset]) -> Option<&Asset> {
    let kws = host_keywords();
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    assets
        .iter()
        .find(|a| {
            let n = a.name.to_ascii_lowercase();
            n.starts_with("oma")
                && (n.ends_with(".zip") || n.ends_with(".tar.gz"))
                && n.contains(arch)
                && kws.iter().any(|k| n.contains(k))
        })
        .or_else(|| {
            assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().starts_with("oma"))
        })
}

/// Dotted-numeric compare: is `tag` (v-prefix tolerated) strictly newer than
/// `current` (CARGO_PKG_VERSION)?
pub fn version_newer(tag: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.trim_start_matches('v')
            .split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (a, b) = (parse(tag), parse(current));
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

/// 上次安装记录：`~/.ohmyagents/selfupdate.json`（资产 digest 为判据——
/// digest 是压缩包哈希，与 exe 哈希不可比）。
fn record_path() -> Result<PathBuf, String> {
    Ok(crate::install::oma_home()?.join("selfupdate.json"))
}

fn read_record_digest() -> Option<String> {
    let text = std::fs::read_to_string(record_path().ok()?).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    v.get("digest").and_then(|d| d.as_str()).map(String::from)
}

fn write_record(digest: &str, tag: &str) {
    if let Ok(p) = record_path() {
        let body = format!(
            "{{\"digest\": \"{digest}\", \"tag\": \"{tag}\", \"ts\": {}}}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );
        let _ = std::fs::write(p, body);
    }
}

/// 判据纯函数：记录 digest 与资产 digest 一致即已最新（缺任一侧 = 需更新）。
fn digest_matches(record: Option<&str>, asset: Option<&str>) -> bool {
    match (record, asset) {
        (Some(r), Some(a)) => r.eq_ignore_ascii_case(a),
        _ => false,
    }
}

/// dev 通道判新：滚动源资产 digest 与上次安装记录一致即已最新。
fn dev_is_current(release: &Release) -> bool {
    let Some(asset) = pick_asset(&release.assets) else {
        return false;
    };
    digest_matches(read_record_digest().as_deref(), asset.digest.as_deref())
}

/// Atomic-ish self replace: write the new binary beside the current exe, then
/// swap. Windows cannot overwrite a running exe but CAN rename it away.
pub fn self_replace(new_bin: &Path) -> Result<PathBuf, String> {
    let cur = std::env::current_exe().map_err(|e| format!("current exe: {e}"))?;
    let dir = cur
        .parent()
        .ok_or_else(|| "no parent dir for current exe".to_string())?;
    let file = cur
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| "exe name not utf-8".to_string())?;
    let staged = dir.join(format!(".{file}.new-{}", std::process::id()));
    std::fs::copy(new_bin, &staged).map_err(|e| format!("stage {}: {e}", staged.display()))?;
    if cfg!(windows) {
        let old = dir.join(format!(".{file}.old-{}", std::process::id()));
        std::fs::rename(&cur, &old).map_err(|e| format!("rename current away: {e}"))?;
        if let Err(e) = std::fs::rename(&staged, &cur) {
            // Put the old binary back so the install stays bootable.
            let _ = std::fs::rename(&old, &cur);
            return Err(format!("swap in new binary: {e}"));
        }
        let _ = std::fs::remove_file(&old);
    } else {
        std::fs::rename(&staged, &cur).map_err(|e| format!("replace binary: {e}"))?;
    }
    Ok(cur)
}

/// `cargo install --git` 源码安装（封版前的主路径）。
pub fn git_install(repo: &str) -> Result<(), String> {
    let cargo = crate::pathutil::find_on_path("cargo")
        .ok_or_else(|| "cargo not on PATH; install Rust toolchain first".to_string())?;
    let url = format!("https://github.com/{repo}.git");
    println!("update.git.repo={url}");
    let status = std::process::Command::new(&cargo)
        .args(["install", "--git", &url, "--force"])
        .status()
        .map_err(|e| format!("cargo install: {e}"))?;
    if status.success() {
        println!("update.git.ok=true");
        Ok(())
    } else {
        Err("cargo install --git failed".into())
    }
}

/// `oma self update` entry: release path with git fallback.
///
/// dev 通道（滚动源）判新：资产带 digest 且等于当前 exe 的 sha256 → 已最新；
/// 否则更新（滚动版版本号常不变，sha256 才是判据）。latest 通道按版本比较。
pub fn run(repo: &str, channel: Channel, git_mode: bool, force: bool) -> Result<(), String> {
    println!("update.current={}", env!("CARGO_PKG_VERSION"));
    println!("update.channel={}", channel.as_str());
    if git_mode {
        return git_install(repo);
    }
    let release = match fetch_release(repo, channel) {
        Ok(r) => r,
        Err(e) => {
            println!("update.release=unavailable detail={e}");
            println!("update.hint=oma self update --git 走源码安装（封版前主路径）");
            return Ok(());
        }
    };
    println!("update.latest={}", release.tag_name);
    if !force {
        let up_to_date = match channel {
            Channel::Dev => dev_is_current(&release),
            // 资产名即编译目标（无版本段），版本判据走 release tag。
            Channel::Latest => !version_newer(&release.tag_name, env!("CARGO_PKG_VERSION")),
        };
        if up_to_date {
            println!("update.ok=already-latest");
            return Ok(());
        }
    }
    let Some(asset) = pick_asset(&release.assets) else {
        println!("update.release=asset-missing names={}", {
            let mut v: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
            v.sort();
            v.join(",")
        });
        println!("update.hint=oma update --git 走源码安装");
        return Ok(());
    };
    println!("update.asset={}", asset.name);
    let tmp = std::env::temp_dir().join(format!(
        "oma-update-{}-{}",
        std::process::id(),
        asset.name.replace('/', "_")
    ));
    crate::install::download_asset(&asset.browser_download_url, &tmp)?;
    // 压缩包解开找 oma 本体；裸二进制资产直接用。
    let extracted = if asset.name.ends_with(".zip") {
        let out = tmp.with_extension("unpacked");
        crate::rmux::extract_zip(&tmp, &out).map_err(|e| e.to_string())?;
        find_oma_bin(&out).ok_or("oma binary not found in archive")?
    } else if asset.name.ends_with(".tar.gz") {
        let out = tmp.with_extension("unpacked");
        crate::rmux::extract_tar_gz(&tmp, &out).map_err(|e| e.to_string())?;
        find_oma_bin(&out).ok_or("oma binary not found in archive")?
    } else {
        tmp.clone()
    };
    let final_path = self_replace(&extracted)?;
    println!("update.replaced={}", final_path.display());
    if let Some(d) = asset.digest.as_deref() {
        write_record(d, &release.tag_name);
    }
    println!("update.ok=true");
    Ok(())
}

fn find_oma_bin(root: &Path) -> Option<PathBuf> {
    if root.is_file() {
        return Some(root.to_path_buf());
    }
    let name = if cfg!(windows) { "oma.exe" } else { "oma" };
    let direct = root.join(name);
    if direct.is_file() {
        return Some(direct);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|s| s.to_str()) == Some(name) {
                return Some(p);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_dotted_numeric() {
        assert!(version_newer("v0.2.0", "0.1.0"));
        assert!(version_newer("0.1.10", "0.1.9"));
        assert!(!version_newer("v0.1.0", "0.1.0"));
        assert!(!version_newer("0.0.9", "0.1.0"));
        // 非数字段按 0 处理不炸。
        assert!(version_newer("v1.0.0-rc1", "0.9.0"));
    }

    #[test]
    fn digest_matches_is_the_rolling_freshness_rule() {
        assert!(digest_matches(Some("sha256:abc"), Some("sha256:ABC")));
        assert!(!digest_matches(Some("sha256:abc"), Some("sha256:def")));
        // 缺记录或缺摘要 = 需更新（保守）。
        assert!(!digest_matches(None, Some("sha256:abc")));
        assert!(!digest_matches(Some("sha256:abc"), None));
    }

    #[test]
    fn picks_host_asset_from_convention() {
        let mk = |names: &[&str]| -> Vec<Asset> {
            names
                .iter()
                .map(|n| Asset {
                    name: n.to_string(),
                    browser_download_url: format!("https://x/{n}"),
                    digest: None,
                })
                .collect()
        };
        // 期望来自命名约定（S028）：资产名即编译目标 oma-<triple>，
        // 本机平台与架构的 oma 包优先。
        let assets = mk(&[
            "oma-x86_64-unknown-linux-gnu.tar.gz",
            "oma-aarch64-apple-darwin.tar.gz",
            "oma-x86_64-pc-windows-msvc.zip",
            "notes.txt",
        ]);
        let picked = pick_asset(&assets).unwrap();
        if cfg!(windows) {
            assert_eq!(picked.name, "oma-x86_64-pc-windows-msvc.zip");
        } else if cfg!(target_os = "macos") {
            assert_eq!(picked.name, "oma-aarch64-apple-darwin.tar.gz");
        } else {
            assert_eq!(picked.name, "oma-x86_64-unknown-linux-gnu.tar.gz");
        }
        // 兜底：无平台匹配时拿任一 oma 资产（提示用户核对）。
        let fb_assets = mk(&["oma-any.bin", "x.txt"]);
        let fallback = pick_asset(&fb_assets).unwrap();
        assert_eq!(fallback.name, "oma-any.bin");
    }
}
