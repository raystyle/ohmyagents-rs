use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use crate::catalog::{Asset, Kind, RmuxPin};

const UA: &str = "ohmyagents-oma";

#[derive(Debug, Clone)]
pub struct Layout {
    pub dispatcher: PathBuf,
    pub helper: PathBuf,
    pub daemon: PathBuf,
}

#[derive(Debug)]
pub struct Report {
    pub source: Source,
    pub layout: Layout,
    pub version: String,
    pub dispatcher_sha256: String,
    pub helper_sha256: String,
    pub daemon_sha256: String,
    pub archive_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Managed,
    Path,
}

#[derive(Debug)]
pub enum CheckError {
    Message(String),
    Missing { reason: String },
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckError::Message(m) | CheckError::Missing { reason: m } => f.write_str(m),
        }
    }
}

pub fn host_os_arch() -> (&'static str, &'static str) {
    (std::env::consts::OS, std::env::consts::ARCH)
}

pub fn managed_root(pin: &RmuxPin) -> Result<PathBuf, CheckError> {
    // oma 应用数据根迁到 ~/.ohmyagents（用户定调 2026-08-31）；旧 LOCALAPPDATA 布局保留兼容探测。
    let home = crate::install::oma_home().map_err(CheckError::Message)?;
    Ok(home.join("rmux").join(&pin.version))
}

/// 2026-08-31 前的安装位（%LOCALAPPDATA%\ohmyagents\rmux\<ver>）。
pub fn legacy_root(pin: &RmuxPin) -> Option<PathBuf> {
    dirs::data_local_dir().map(|b| b.join("ohmyagents").join("rmux").join(&pin.version))
}

pub fn bin_dir(root: &Path) -> PathBuf {
    if cfg!(windows) {
        root.to_path_buf()
    } else {
        root.join("bin")
    }
}

pub fn dispatcher_name() -> &'static str {
    if cfg!(windows) {
        "rmux.exe"
    } else {
        "rmux"
    }
}

fn daemon_name() -> &'static str {
    if cfg!(windows) {
        "rmux-daemon.exe"
    } else {
        "rmux-daemon"
    }
}

fn helper_rel() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("libexec").join("rmux").join("rmux.exe")
    } else {
        PathBuf::from("libexec").join("rmux").join("rmux")
    }
}

pub fn layout_at_root(root: &Path) -> Option<Layout> {
    let dispatcher = bin_dir(root).join(dispatcher_name());
    let helper = root.join(helper_rel());
    let daemon = bin_dir(root).join(daemon_name());
    if dispatcher.is_file() && helper.is_file() && daemon.is_file() {
        Some(Layout {
            dispatcher,
            helper,
            daemon,
        })
    } else {
        None
    }
}

/// Official prefix: `<root>/bin/rmux` and `<root>/libexec/rmux/rmux`.
pub fn layout_from_dispatcher(dispatcher: &Path) -> Option<Layout> {
    let bin = dispatcher.parent()?;
    let root = bin.parent()?;
    let helper = root.join(helper_rel());
    let daemon = bin.join(daemon_name());
    if helper.is_file() && daemon.is_file() {
        Some(Layout {
            dispatcher: dispatcher.to_path_buf(),
            helper,
            daemon,
        })
    } else if cfg!(windows) {
        // Zip extract puts rmux.exe next to libexec\ (no extra bin\).
        let helper = bin.join(helper_rel());
        let daemon = bin.join(daemon_name());
        if helper.is_file() && daemon.is_file() {
            return Some(Layout {
                dispatcher: dispatcher.to_path_buf(),
                helper,
                daemon,
            });
        }
        None
    } else {
        None
    }
}

pub fn sha256_file(path: &Path) -> Result<String, CheckError> {
    let mut file =
        File::open(path).map_err(|e| CheckError::Message(format!("{}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| CheckError::Message(format!("read {}: {e}", path.display())))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn read_version(rmux: &Path) -> Result<String, CheckError> {
    let out = Command::new(rmux)
        .arg("-V")
        .output()
        .map_err(|e| CheckError::Message(format!("run {} -V: {e}", rmux.display())))?;
    if !out.status.success() {
        return Err(CheckError::Message(format!(
            "{} -V exited {}: {}",
            rmux.display(),
            out.status,
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_version(&text).ok_or_else(|| {
        CheckError::Message(format!(
            "cannot parse version from `rmux -V` output: {text:?}"
        ))
    })
}

pub fn parse_version(text: &str) -> Option<String> {
    let line = text.lines().find(|l| !l.trim().is_empty())?;
    let rest = line.trim().strip_prefix("rmux")?;
    let ver = rest.trim().split_whitespace().next()?;
    Some(ver.to_string())
}

pub fn helper_ok(rmux: &Path) -> Result<(), CheckError> {
    let out = Command::new(rmux)
        .arg("--help")
        .output()
        .map_err(|e| CheckError::Message(format!("run {} --help: {e}", rmux.display())))?;
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if combined.to_ascii_lowercase().contains("usage: rmux") {
        Ok(())
    } else {
        Err(CheckError::Message(format!(
            "{} --help did not reach full helper (tiny CLI only?). output:\n{combined}",
            rmux.display()
        )))
    }
}

fn inspect(
    layout: Layout,
    source: Source,
    archive_sha256: Option<String>,
) -> Result<Report, CheckError> {
    let version = read_version(&layout.dispatcher)?;
    helper_ok(&layout.dispatcher)?;
    Ok(Report {
        dispatcher_sha256: sha256_file(&layout.dispatcher)?,
        helper_sha256: sha256_file(&layout.helper)?,
        daemon_sha256: sha256_file(&layout.daemon)?,
        version,
        layout,
        source,
        archive_sha256,
    })
}

fn read_manifest_archive_sha(root: &Path) -> Option<String> {
    let path = root.join(".oma-rmux-manifest.toml");
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix("archive_sha256") {
            let v = v.trim().trim_start_matches('=').trim().trim_matches('"');
            if v.len() == 64 {
                return Some(v.to_string());
            }
        }
    }
    None
}

pub fn detect(pin: &RmuxPin) -> Result<Report, CheckError> {
    let root = managed_root(pin)?;
    let legacy = legacy_root(pin);
    for candidate in [Some(root), legacy].into_iter().flatten() {
        if let Some(layout) = layout_at_root(&candidate) {
            let archive = read_manifest_archive_sha(&candidate);
            let report = inspect(layout, Source::Managed, archive)?;
            if report.version != pin.version {
                return Err(CheckError::Missing {
                    reason: format!("managed rmux is {} want {}", report.version, pin.version),
                });
            }
            return Ok(report);
        }
    }

    if let Ok(found) = which::which(dispatcher_name()) {
        if let Some(layout) = layout_from_dispatcher(&found) {
            let report = inspect(layout, Source::Path, None)?;
            if report.version != pin.version {
                return Err(CheckError::Missing {
                    reason: format!("PATH rmux is {} want {}", report.version, pin.version),
                });
            }
            return Ok(report);
        }
        return Err(CheckError::Missing {
            reason: format!(
                "found {} but missing libexec helper / daemon (copying only rmux.exe is not a valid install)",
                found.display()
            ),
        });
    }

    Err(CheckError::Missing {
        reason: format!("rmux {} not found", pin.version),
    })
}

pub fn ensure(pin: &RmuxPin, install: bool) -> Result<Report, CheckError> {
    match detect(pin) {
        Ok(r) => Ok(r),
        Err(CheckError::Missing { reason }) if install => {
            eprintln!("oma: {reason}; installing pinned {}", pin.tag);
            install_pinned(pin)?;
            detect(pin)
        }
        Err(e) => Err(e),
    }
}

pub fn install_pinned(pin: &RmuxPin) -> Result<Layout, CheckError> {
    let (os, arch) = host_os_arch();
    let asset = pin.asset_for(os, arch).map_err(CheckError::Message)?;
    let root = managed_root(pin)?;
    if root.exists() {
        fs::remove_dir_all(&root)
            .map_err(|e| CheckError::Message(format!("clear {}: {e}", root.display())))?;
    }
    fs::create_dir_all(&root)
        .map_err(|e| CheckError::Message(format!("mkdir {}: {e}", root.display())))?;

    let tmp =
        root.parent()
            .unwrap_or(&root)
            .join(format!(".tmp-{}-{}", pin.version, std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).ok();
    }
    fs::create_dir_all(&tmp)
        .map_err(|e| CheckError::Message(format!("mkdir {}: {e}", tmp.display())))?;

    let result = (|| {
        let archive_path = tmp.join(&asset.name);
        download_asset(pin, asset, &archive_path)?;
        let actual = sha256_file(&archive_path)?;
        if actual != asset.sha256 {
            return Err(CheckError::Message(format!(
                "checksum mismatch for {}: expected {}, got {}",
                asset.name, asset.sha256, actual
            )));
        }
        extract_archive(asset, &archive_path, &tmp)?;
        let package = find_package_root(&tmp).ok_or_else(|| {
            CheckError::Message("archive is missing rmux + libexec helper + daemon".into())
        })?;
        copy_package(&package, &root)?;
        write_manifest(&root, pin, asset)?;
        layout_at_root(&root)
            .ok_or_else(|| CheckError::Message("install finished but layout incomplete".into()))
    })();

    fs::remove_dir_all(&tmp).ok();
    result
}

fn github_get(url: &str) -> Result<ureq::Response, CheckError> {
    let mut req = ureq::get(url).set("User-Agent", UA);
    if let Ok(token) = std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN")) {
        if !token.is_empty() {
            req = req.set("Authorization", &format!("Bearer {token}"));
        }
    }
    req.call()
        .map_err(|e| CheckError::Message(format!("GET {url}: {e}")))
}

fn download_asset(pin: &RmuxPin, asset: &Asset, dest: &Path) -> Result<(), CheckError> {
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        pin.repo, pin.tag, asset.name
    );
    eprintln!("oma: downloading {url}");
    let resp = github_get(&url)?;
    if resp.status() != 200 {
        return Err(CheckError::Message(format!(
            "GET {url} -> HTTP {}",
            resp.status()
        )));
    }
    let mut reader = resp.into_reader();
    let mut file = File::create(dest)
        .map_err(|e| CheckError::Message(format!("create {}: {e}", dest.display())))?;
    io::copy(&mut reader, &mut file)
        .map_err(|e| CheckError::Message(format!("write {}: {e}", dest.display())))?;
    Ok(())
}

fn extract_archive(asset: &Asset, archive: &Path, dest: &Path) -> Result<(), CheckError> {
    match asset.kind {
        Kind::Zip => extract_zip(archive, dest),
        Kind::TarGz => extract_tar_gz(archive, dest),
    }
}

pub fn extract_zip(archive: &Path, dest: &Path) -> Result<(), CheckError> {
    let file = File::open(archive)
        .map_err(|e| CheckError::Message(format!("{}: {e}", archive.display())))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| CheckError::Message(format!("zip {}: {e}", archive.display())))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| CheckError::Message(format!("zip entry: {e}")))?;
        let name = entry.name().replace('\\', "/");
        if name.contains("..") {
            continue;
        }
        let out = dest.join(name.trim_start_matches('/'));
        if entry.is_dir() || name.ends_with('/') {
            fs::create_dir_all(&out).ok();
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| CheckError::Message(format!("{}: {e}", parent.display())))?;
        }
        let mut outfile = File::create(&out)
            .map_err(|e| CheckError::Message(format!("create {}: {e}", out.display())))?;
        io::copy(&mut entry, &mut outfile)
            .map_err(|e| CheckError::Message(format!("extract {}: {e}", out.display())))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = entry.unix_mode() {
                fs::set_permissions(&out, fs::Permissions::from_mode(mode)).ok();
            }
        }
    }
    Ok(())
}

pub fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), CheckError> {
    let file = File::open(archive)
        .map_err(|e| CheckError::Message(format!("{}: {e}", archive.display())))?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);
    archive
        .unpack(dest)
        .map_err(|e| CheckError::Message(format!("tar {}: {e}", dest.display())))
}

fn find_package_root(extracted: &Path) -> Option<PathBuf> {
    if layout_at_root(extracted).is_some() {
        return Some(extracted.to_path_buf());
    }
    let rd = fs::read_dir(extracted).ok()?;
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_dir() && layout_at_root(&p).is_some() {
            return Some(p);
        }
    }
    None
}

fn copy_package(from: &Path, to: &Path) -> Result<(), CheckError> {
    copy_dir(from, to).map_err(|e| CheckError::Message(format!("copy package: {e}")))
}

pub fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for ent in fs::read_dir(from)? {
        let ent = ent?;
        let src = ent.path();
        let dst = to.join(ent.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
            #[cfg(unix)]
            {
                if let Ok(meta) = fs::metadata(&src) {
                    fs::set_permissions(&dst, meta.permissions())?;
                }
            }
        }
    }
    Ok(())
}

fn write_manifest(root: &Path, pin: &RmuxPin, asset: &Asset) -> Result<(), CheckError> {
    let layout = layout_at_root(root)
        .ok_or_else(|| CheckError::Message("cannot write manifest: layout incomplete".into()))?;
    let body = format!(
        "tag = \"{}\"\nversion = \"{}\"\nasset = \"{}\"\narchive_sha256 = \"{}\"\ndispatcher_sha256 = \"{}\"\nhelper_sha256 = \"{}\"\ndaemon_sha256 = \"{}\"\n",
        pin.tag,
        pin.version,
        asset.name,
        asset.sha256,
        sha256_file(&layout.dispatcher)?,
        sha256_file(&layout.helper)?,
        sha256_file(&layout.daemon)?,
    );
    let path = root.join(".oma-rmux-manifest.toml");
    let mut f =
        File::create(&path).map_err(|e| CheckError::Message(format!("{}: {e}", path.display())))?;
    f.write_all(body.as_bytes())
        .map_err(|e| CheckError::Message(format!("write {}: {e}", path.display())))
}

pub fn prepend_path(bin: &Path) {
    let bin = bin.display().to_string();
    let key = if cfg!(windows) { "Path" } else { "PATH" };
    let sep = if cfg!(windows) { ';' } else { ':' };
    let mut parts: Vec<String> = std::env::var_os(key)
        .map(|v| {
            v.to_string_lossy()
                .split(sep)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();
    parts.retain(|p| p != &bin);
    parts.insert(0, bin);
    std::env::set_var(key, parts.join(&sep.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rmux_dash_v() {
        assert_eq!(parse_version("rmux 0.10.0\n").as_deref(), Some("0.10.0"));
        assert_eq!(
            parse_version("rmux 0.10.0 (windows)").as_deref(),
            Some("0.10.0")
        );
        assert!(parse_version("usage: rmux").is_none());
    }
}
