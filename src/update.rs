//! hst 自更新（用户定调 2026-09-02：去 GitHub 升级新版本；封版前本地测试，
//! releases 为空时走 `--git` 源码安装路径）。
//! 机制见 S028：releases/latest API、资产命名约定 `oma-<triple>.(zip|tar.gz)`、
//! Windows 运行中自替换（rename 舞步）、Unix 原子 rename 覆盖。
//! D16：`HST_MIRROR=<基址>` 镜像通道只覆盖 dev（镜像无 manifest，判新走
//! `<基址>/hst/dev/<资产名>.sha256` 边车）；stable 与未设置时行为不变。
//! 镜像侧仅网络类失败回落 GitHub；哈希不符是安全问题，报错不回落。

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Default repo; `--repo owner/name` overrides. 更名预备（2026-09-02）：
/// 仓库名 OhMyAgents → hst-rs；GitHub 更名后旧名 URL 自动重定向，
/// 此值在更名前后均可用。
pub const DEFAULT_REPO: &str = "raystyle/hst-rs";

const UA: &str = concat!("hst/", env!("CARGO_PKG_VERSION"));

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
                "release '{ch}' not published for {repo} yet (pre-release phase); use `hst self update --git`",
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

/// 上次安装记录：`~/.hst/selfupdate.json`（资产 digest 为判据——
/// digest 是压缩包哈希，与 exe 哈希不可比）。
fn record_path() -> Result<PathBuf, String> {
    Ok(crate::install::hst_home()?.join("selfupdate.json"))
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

// ===== D16 镜像通道（HST_MIRROR，只覆盖 dev） =====

/// 镜像开关：`HST_MIRROR=<基址>`（去空白与尾斜杠）；未设/空 = None（行为与现状一致）。
fn mirror_base() -> Option<String> {
    // D29 兼容：旧 OMA_MIRROR 一个版本内仍读，读旧打 stderr 提示。
    let v = match std::env::var("HST_MIRROR") {
        Ok(v) => v,
        Err(_) => std::env::var("OMA_MIRROR").ok().inspect(|_| {
            eprintln!(
                "hst: OMA_MIRROR is deprecated; rename it to HST_MIRROR (removed next release)"
            );
        })?,
    };
    let v = v.trim().trim_end_matches('/');
    if v.is_empty() {
        None
    } else {
        Some(v.to_string())
    }
}

/// 本机 host 三元组的确定性资产名（与 dev-release.yml 命名约定一致：
/// 资产名即编译目标三元组，windows 用 zip、其余 tar.gz）。
fn host_asset_name() -> String {
    let arch = if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x86_64"
    };
    if cfg!(target_os = "windows") {
        format!("oma-{arch}-pc-windows-msvc.zip")
    } else if cfg!(target_os = "macos") {
        format!("oma-{arch}-apple-darwin.tar.gz")
    } else {
        format!("oma-{arch}-unknown-linux-gnu.tar.gz")
    }
}

/// 镜像边车 URL（判新与校验）。`?t=<unix>` 缓存击穿取 origin 现值
/// （ohmycloud 2026-09-08 建议，CF 缓存键含 query；D16 尾巴修）。
fn mirror_sidecar_url(base: &str, name: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!(
        "{}/hst/dev/{name}.sha256?t={ts}",
        base.trim_end_matches('/')
    )
}

/// 镜像资产 URL。`?v=<边车锚>` 以边车哈希为缓存键：每滚天然新键，
/// 永久免疫「边车新、资产旧」的陈旧缓存窗口（R2 取对象只看 path）。
fn mirror_asset_url(base: &str, name: &str, anchor: &str) -> String {
    format!("{}/hst/dev/{name}?v={anchor}", base.trim_end_matches('/'))
}

/// sha256sum 边车解析：首字段即哈希（标准双空格、单空格均可），容错
/// `sha256:` 前缀；归一为 `sha256:<hex>` 小写。脏输入报错（数据面失败，
/// 不归网络回落）。
fn parse_sidecar(text: &str) -> Result<String, String> {
    let first = text
        .split_whitespace()
        .next()
        .ok_or_else(|| "empty sidecar".to_string())?;
    let lower = first.to_ascii_lowercase();
    let hex = lower.strip_prefix("sha256:").unwrap_or(&lower);
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("sidecar first field is not a sha256 hex: {first}"));
    }
    Ok(format!("sha256:{hex}"))
}

fn http_get_string(url: &str) -> Result<String, String> {
    ureq::get(url)
        .set("User-Agent", UA)
        .call()
        .map_err(|e| format!("{e}"))?
        .into_string()
        .map_err(|e| format!("read body: {e}"))
}

enum MirrorStep {
    /// 镜像路径走完（已最新或已替换），记录已写。
    Done,
    /// 网络类失败（边车/下载连不上、404、超时）：回落 GitHub dev 路径。
    Fallback(String),
}

/// dev 镜像路径：边车判新（免 manifest）→ 下载 → sha256 校验（不符即报错，
/// 安全问题不回落）→ 解包安装复用现状。
fn dev_via_mirror(base: &str, force: bool) -> Result<MirrorStep, String> {
    let name = host_asset_name();
    let sidecar_url = mirror_sidecar_url(base, &name);
    let sidecar = match http_get_string(&sidecar_url) {
        Ok(t) => t,
        Err(e) => return Ok(MirrorStep::Fallback(format!("sidecar {sidecar_url}: {e}"))),
    };
    let digest =
        parse_sidecar(&sidecar).map_err(|e| format!("mirror sidecar {sidecar_url}: {e}"))?;
    if !force && digest_matches(read_record_digest().as_deref(), Some(&digest)) {
        println!("update.source=mirror");
        println!("update.ok=already-latest");
        return Ok(MirrorStep::Done);
    }
    // 资产缓存键锚定边车哈希（裸 hex），消边车与资产的缓存不同步窗口。
    let anchor = digest.strip_prefix("sha256:").unwrap_or(&digest);
    let asset_url = mirror_asset_url(base, &name, anchor);
    println!("update.asset={name}");
    let tmp = std::env::temp_dir().join(format!(
        "oma-update-{}-{}",
        std::process::id(),
        name.replace('/', "_")
    ));
    if let Err(e) = crate::install::download_asset(&asset_url, &tmp) {
        return Ok(MirrorStep::Fallback(format!("download {asset_url}: {e}")));
    }
    // 安全闸：下载件必须与边车一致，不符拒绝安装且不回落（回落等于绕过校验）。
    let got = format!("sha256:{}", crate::archive::sha256_file(&tmp)?);
    if !got.eq_ignore_ascii_case(&digest) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "mirror asset sha256 mismatch: sidecar {digest} got {got}; refusing install"
        ));
    }
    let extracted = if name.ends_with(".zip") {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_zip(&tmp, &out)?;
        find_oma_bin(&out).ok_or("oma binary not found in archive")?
    } else {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_tar_gz(&tmp, &out)?;
        find_oma_bin(&out).ok_or("oma binary not found in archive")?
    };
    let final_path = self_replace(&extracted)?;
    println!("update.replaced={}", final_path.display());
    write_record(&digest, "dev-mirror");
    println!("update.source=mirror");
    println!("update.ok=true");
    Ok(MirrorStep::Done)
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

/// `hst self update` entry: release path with git fallback.
///
/// dev 通道（滚动源）判新：资产带 digest 且等于当前 exe 的 sha256 → 已最新；
/// 否则更新（滚动版版本号常不变，sha256 才是判据）。latest 通道按版本比较。
pub fn run(repo: &str, channel: Channel, git_mode: bool, force: bool) -> Result<(), String> {
    println!("update.current={}", env!("CARGO_PKG_VERSION"));
    println!("update.channel={}", channel.as_str());
    let mirror = mirror_base();
    println!("update.mirror={}", mirror.as_deref().unwrap_or("off"));
    if git_mode {
        return git_install(repo);
    }
    if let Some(base) = mirror.as_deref() {
        match channel {
            // 镜像无 manifest，只覆盖 dev；stable 直连 GitHub（说明后走原路径）。
            Channel::Latest => println!("update.mirror=skipped channel=stable"),
            Channel::Dev => match dev_via_mirror(base, force)? {
                MirrorStep::Done => return Ok(()),
                MirrorStep::Fallback(detail) => {
                    println!("update.mirror=failed detail={detail}");
                    println!("update.fallback=github");
                }
            },
        }
    }
    println!("update.source=github");
    let release = match fetch_release(repo, channel) {
        Ok(r) => r,
        Err(e) => {
            println!("update.release=unavailable detail={e}");
            println!("update.hint=hst self update --git 走源码安装（封版前主路径）");
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
        crate::archive::extract_zip(&tmp, &out)?;
        find_oma_bin(&out).ok_or("oma binary not found in archive")?
    } else if asset.name.ends_with(".tar.gz") {
        let out = tmp.with_extension("unpacked");
        crate::archive::extract_tar_gz(&tmp, &out)?;
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

    // ===== D16 镜像通道纯函数 =====

    #[test]
    fn host_asset_name_matches_release_convention() {
        // 期望值来自 dev-release.yml 命名约定（资产名即编译目标三元组，
        // windows 用 zip、其余 tar.gz），字面量断言本机期望。
        let name = host_asset_name();
        if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            assert_eq!(name, "oma-x86_64-pc-windows-msvc.zip");
        } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
            assert_eq!(name, "oma-aarch64-pc-windows-msvc.zip");
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(name, "oma-aarch64-apple-darwin.tar.gz");
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            assert_eq!(name, "oma-x86_64-apple-darwin.tar.gz");
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(name, "oma-x86_64-unknown-linux-gnu.tar.gz");
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            assert_eq!(name, "oma-aarch64-unknown-linux-gnu.tar.gz");
        } else {
            panic!("untested host target: {name}");
        }
    }

    #[test]
    fn mirror_urls_trim_trailing_slash() {
        // 缓存击穿约定（ohmycloud 2026-09-08）：边车带 ?t= 时间戳、资产带
        // ?v=<边车锚>；两者基址尾斜杠归一。
        let s = mirror_sidecar_url("https://env.ohmygh.com/", "oma-x.zip");
        assert!(
            s.starts_with("https://env.ohmygh.com/hst/dev/oma-x.zip.sha256?t="),
            "sidecar url: {s}"
        );
        assert!(
            s["https://env.ohmygh.com/hst/dev/oma-x.zip.sha256?t=".len()..]
                .chars()
                .all(|c| c.is_ascii_digit())
        );
        let a = mirror_asset_url("https://env.ohmygh.com/", "oma-x.zip", "abc123");
        assert_eq!(a, "https://env.ohmygh.com/hst/dev/oma-x.zip?v=abc123");
        let s2 = mirror_sidecar_url("https://env.ohmygh.com", "oma-x.zip");
        assert!(s2.starts_with("https://env.ohmygh.com/hst/dev/oma-x.zip.sha256?t="));
    }

    #[test]
    fn sidecar_parses_sha256sum_formats() {
        // Oracle：sha256sum 输出格式——哈希、两个空格、文件名（POSIX 文本模式）。
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let want = format!("sha256:{hex}");
        // 标准双空格加文件名
        assert_eq!(
            parse_sidecar(&format!("{hex}  oma-x86_64-pc-windows-msvc.zip\n")).unwrap(),
            want
        );
        // 单空格容错
        assert_eq!(parse_sidecar(&format!("{hex} oma.zip")).unwrap(), want);
        // 大写哈希归一小写
        assert_eq!(parse_sidecar(&hex.to_ascii_uppercase()).unwrap(), want);
        // sha256: 前缀容错（大小写均收）
        assert_eq!(parse_sidecar(&format!("sha256:{hex}")).unwrap(), want);
        assert_eq!(parse_sidecar(&format!("SHA256:{hex}")).unwrap(), want);
    }

    #[test]
    fn dies_sidecar_rejects_dirty_input() {
        // 空、非 hex、长度不符都报错（数据面失败，不走网络回落）。
        assert!(parse_sidecar("").is_err());
        assert!(parse_sidecar("   \n").is_err());
        assert!(parse_sidecar("not-a-hash  oma.zip").is_err());
        assert!(parse_sidecar("abcd  oma.zip").is_err());
        assert!(parse_sidecar(
            "zz7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  oma.zip"
        )
        .is_err());
    }

    #[test]
    fn mirror_digest_interops_with_github_record() {
        // GitHub 记录形如 sha256:ABC（大写），边车小写 hex：digest_matches
        // 口径下互认为同一版本。
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        let parsed = parse_sidecar(&format!("{hex}  oma.zip")).unwrap();
        assert!(digest_matches(
            Some(&format!("sha256:{}", hex.to_ascii_uppercase())),
            Some(&parsed)
        ));
    }
}
