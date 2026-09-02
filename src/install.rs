//! oma 自适应本机安装（P0012）：catalog pin、多渠道下载（github 默认、CDN 兜底）、
//! sha256 信任锚校验、自管根布局（`~/.ohmyagents/agents/<name>/<version>/`）、
//! 装后版本探针、`oma agents update` 的最新版解析与用户本地 pin 写回。

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::catalog::{
    AgentAsset, AgentKind, AgentPin, AgentsCatalog, CdnStyle, PinSource, SumsMode,
};
use crate::rmux::{copy_dir, extract_tar_gz, extract_zip, sha256_file};

const UA: &str = "ohmyagents-oma";
const MANIFEST_NAME: &str = ".oma-agent-manifest.toml";

// ---- 根与 pin 解析 ----

/// oma 应用数据根：`OMA_HOME` 环境变量 > `~/.ohmyagents`（用户定调 2026-08-31：
/// oma 在用户 home 下建立维护自己的应用数据）。
pub fn oma_home() -> Result<PathBuf, String> {
    if let Some(v) = std::env::var_os("OMA_HOME") {
        if !v.is_empty() {
            return Ok(PathBuf::from(v));
        }
    }
    let home = dirs::home_dir().ok_or("cannot resolve home dir")?;
    Ok(home.join(".ohmyagents"))
}

pub fn agents_root(home: &Path) -> PathBuf {
    home.join("agents")
}

pub fn user_catalog_path(home: &Path) -> PathBuf {
    home.join("catalog").join("agents.toml")
}

/// pin 解析：用户本地层优先（oma 自己维护，`oma agents update` 写回），缺省回落仓内出厂锚。
/// 用户层存在但损坏是硬错误——自维护文件坏了不该静默降级到旧 pin。
pub fn resolve_catalog(home: &Path) -> Result<AgentsCatalog, String> {
    let path = user_catalog_path(home);
    if path.is_file() {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let cat = AgentsCatalog::load_from(&text)?;
        cat.validate()
            .map_err(|e| format!("{}: {e}（删除该文件可重置为出厂 pin）", path.display()))?;
        return Ok(cat);
    }
    let cat = AgentsCatalog::load()?;
    cat.validate()?;
    Ok(cat)
}

// ---- 安装 ----

#[derive(Debug)]
pub enum InstallOutcome {
    Installed {
        version: String,
        probed: Option<String>,
        path: PathBuf,
    },
    Skipped {
        detail: String,
    },
}

/// 自适应安装：已装（任何来源）即跳过；缺则按 sources 顺序（github 主、CDN 兜底）尝试。
pub fn install_agent(
    catalog: &AgentsCatalog,
    name: &str,
    home: &Path,
    force: bool,
) -> Result<InstallOutcome, String> {
    let pin = catalog
        .find(name)
        .ok_or_else(|| unknown_agent(catalog, name))?;
    install_pin(pin, home, force)
}

/// names 为空 = catalog 全部；逐家安装，单家失败不阻断其余（结果聚合给调用方打印）。
pub fn install_missing(
    catalog: &AgentsCatalog,
    names: &[String],
    home: &Path,
    force: bool,
) -> Vec<(String, Result<InstallOutcome, String>)> {
    let wanted: Vec<&str> = if names.is_empty() {
        catalog.agents.iter().map(|p| p.name.as_str()).collect()
    } else {
        names.iter().map(|s| s.as_str()).collect()
    };
    wanted
        .into_iter()
        .map(|n| {
            let r = match catalog.find(n) {
                Some(pin) => install_pin(pin, home, force),
                None => Err(unknown_agent(catalog, n)),
            };
            (n.to_string(), r)
        })
        .collect()
}

pub fn install_pin(pin: &AgentPin, home: &Path, force: bool) -> Result<InstallOutcome, String> {
    if !force {
        if let Some(hit) = crate::agents::Probe::from_env().find(&pin.name) {
            return Ok(InstallOutcome::Skipped {
                detail: format!(
                    "already installed ({}: {})",
                    hit.source.as_str(),
                    hit.path.display()
                ),
            });
        }
    }
    let (os, arch) = crate::rmux::host_os_arch();
    let mut last_err = String::new();
    for (idx, src) in pin.sources.iter().enumerate() {
        let Some(asset) = pin.asset_for(idx, os, arch) else {
            last_err = format!(
                "source {} has no pinned asset for {os}-{arch}",
                src.kind_name()
            );
            continue;
        };
        match install_from_source(pin, src, asset, home) {
            Ok(outcome) => return Ok(outcome),
            Err(e) => {
                eprintln!("oma: {}: source {} failed: {e}", pin.name, src.kind_name());
                last_err = e;
            }
        }
    }
    Err(if last_err.is_empty() {
        format!("agent {} has no usable source", pin.name)
    } else {
        last_err
    })
}

fn install_from_source(
    pin: &AgentPin,
    src: &PinSource,
    asset: &AgentAsset,
    home: &Path,
) -> Result<InstallOutcome, String> {
    let tmp = agents_root(home).join(format!(".tmp-{}-{}", pin.name, std::process::id()));
    let result = (|| {
        if tmp.exists() {
            fs::remove_dir_all(&tmp).ok();
        }
        fs::create_dir_all(&tmp).map_err(|e| format!("mkdir {}: {e}", tmp.display()))?;

        let archive = tmp.join(&asset.name);
        let url = asset_url(pin, src, asset)?;
        eprintln!("oma: downloading {url}");
        download(&url, &archive)?;
        let actual = sha256_file(&archive).map_err(|e| e.to_string())?;
        if actual != asset.sha256 {
            return Err(format!(
                "checksum mismatch for {}: expected {}, got {}",
                asset.name, asset.sha256, actual
            ));
        }

        let pkg = tmp.join("pkg");
        fs::create_dir_all(&pkg).map_err(|e| format!("mkdir {}: {e}", pkg.display()))?;
        let binary_file = binary_file_name(&pin.binary);
        match asset.kind {
            AgentKind::Single => {
                fs::copy(&archive, pkg.join(&binary_file))
                    .map_err(|e| format!("copy single binary: {e}"))?;
            }
            AgentKind::Zip => extract_zip(&archive, &pkg).map_err(|e| e.to_string())?,
            AgentKind::TarGz => extract_tar_gz(&archive, &pkg).map_err(|e| e.to_string())?,
        }
        let bin = find_binary(&pkg, &pin.binary)
            .ok_or_else(|| format!("{} not found inside package", binary_file))?;

        let dest = agents_root(home).join(&pin.name).join(&pin.version);
        if dest.exists() {
            fs::remove_dir_all(&dest).map_err(|e| format!("clear {}: {e}", dest.display()))?;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        if fs::rename(&pkg, &dest).is_err() {
            copy_dir(&pkg, &dest).map_err(|e| format!("move package: {e}"))?;
            fs::remove_dir_all(&pkg).ok();
        }
        let bin_at_dest = dest.join(bin.strip_prefix(&pkg).unwrap_or(Path::new("")));
        mark_executable(&bin_at_dest);
        write_manifest(&dest, pin, asset, &bin_at_dest)?;
        let probed = probe_version(&bin_at_dest);
        Ok(InstallOutcome::Installed {
            version: pin.version.clone(),
            probed,
            path: bin_at_dest,
        })
    })();
    fs::remove_dir_all(&tmp).ok();
    result
}

fn unknown_agent(catalog: &AgentsCatalog, name: &str) -> String {
    let known: Vec<&str> = catalog.agents.iter().map(|p| p.name.as_str()).collect();
    format!("unknown agent {name}; catalog has: {}", known.join(", "))
}

fn binary_file_name(binary: &str) -> String {
    if cfg!(windows) {
        format!("{binary}.exe")
    } else {
        binary.to_string()
    }
}

/// 渠道 URL 规则：github release 直链；CDN direct 是 base 拼资产名；
/// CDN manifest 是 base/binaries/<version>/<资产>（kimi 官方 install.sh 实证）。
pub fn asset_url(pin: &AgentPin, src: &PinSource, asset: &AgentAsset) -> Result<String, String> {
    match src {
        PinSource::Github { repo, .. } => Ok(format!(
            "https://github.com/{repo}/releases/download/{}/{asset_name}",
            pin.tag,
            asset_name = asset.name
        )),
        PinSource::Cdn { base, style, .. } => match style {
            CdnStyle::Direct => Ok(format!("{base}/{}", asset.name)),
            CdnStyle::Manifest => Ok(format!("{base}/binaries/{}/{}", pin.version, asset.name)),
        },
    }
}

fn mark_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perm = meta.permissions();
            perm.set_mode(perm.mode() | 0o755);
            fs::set_permissions(path, perm).ok();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// leaf 名递归找二进制（深度有界、浅者优先）：免疫各家归档顶层布局差异与版本前缀目录。
pub fn find_binary(root: &Path, binary: &str) -> Option<PathBuf> {
    let target = binary_file_name(binary);
    let mut queue: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    let mut best: Option<(usize, PathBuf)> = None;
    while let Some((dir, depth)) = queue.pop() {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                if depth + 1 <= 4 {
                    queue.push((p, depth + 1));
                }
            } else if p.file_name().and_then(|n| n.to_str()) == Some(target.as_str()) {
                let better = match &best {
                    None => true,
                    Some((d, _)) => depth + 1 < *d,
                };
                if better {
                    best = Some((depth + 1, p.clone()));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

fn write_manifest(
    dir: &Path,
    pin: &AgentPin,
    asset: &AgentAsset,
    bin: &Path,
) -> Result<(), String> {
    let rel = bin.strip_prefix(dir).unwrap_or(bin);
    let bin_sha = sha256_file(bin).map_err(|e| e.to_string())?;
    let body = format!(
        "name = \"{}\"\ntag = \"{}\"\nversion = \"{}\"\nasset = \"{}\"\narchive_sha256 = \"{}\"\nbinary_rel = \"{}\"\nbinary_sha256 = \"{}\"\n",
        pin.name,
        pin.tag,
        pin.version,
        asset.name,
        asset.sha256,
        rel.display(),
        bin_sha,
    );
    let path = dir.join(MANIFEST_NAME);
    let mut f = File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    f.write_all(body.as_bytes())
        .map_err(|e| format!("write {}: {e}", path.display()))
}

/// 装后 `--version` 探针：重试 5 次 × 500ms（杀软扫描锁文件的瞬态，ohmyenv 同法）。
pub fn probe_version(bin: &Path) -> Option<String> {
    for _ in 0..5 {
        if let Ok(out) = Command::new(bin).arg("--version").output() {
            let text = if out.stdout.is_empty() {
                String::from_utf8_lossy(&out.stderr).into_owned()
            } else {
                String::from_utf8_lossy(&out.stdout).into_owned()
            };
            if let Some(line) = text.lines().find(|l| !l.trim().is_empty()) {
                return Some(line.trim().chars().take(120).collect());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    None
}

// ---- 探测集成 ----

/// oma 自管安装的二进制清单（agents.rs 的 Probe 以 source=oma 消费）。
pub fn managed_binaries(home: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    let root = agents_root(home);
    let Ok(names) = fs::read_dir(&root) else {
        return out;
    };
    for ent in names.flatten() {
        let name_dir = ent.path();
        if !name_dir.is_dir() {
            continue;
        }
        let agent = ent.file_name().to_string_lossy().into_owned();
        if let Ok(vers) = fs::read_dir(&name_dir) {
            for v in vers.flatten() {
                let manifest = v.path().join(MANIFEST_NAME);
                if let Some(rel) = manifest_field(&manifest, "binary_rel") {
                    let bin = v.path().join(&rel);
                    if bin.is_file() {
                        out.push((agent.clone(), bin));
                    }
                }
            }
        }
    }
    out
}

/// oma 自管已装版本（manifest 的 version 字段；多个版本目录取最大）。
pub fn managed_version(home: &Path, name: &str) -> Option<String> {
    let dir = agents_root(home).join(name);
    let mut best: Option<String> = None;
    let Ok(vers) = fs::read_dir(&dir) else {
        return None;
    };
    for v in vers.flatten() {
        let manifest = v.path().join(MANIFEST_NAME);
        if let Some(ver) = manifest_field(&manifest, "version") {
            let take = match &best {
                Some(b) => version_ge(&ver, b),
                None => true,
            };
            if take {
                best = Some(ver);
            }
        }
    }
    best
}

fn manifest_field(path: &Path, key: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim().trim_start_matches('=').trim().trim_matches('"');
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

// ---- 下载 ----

/// 单文件下载（github 域自动带 GH_TOKEN）；update.rs 复用。
pub(crate) fn download_asset(url: &str, dest: &Path) -> Result<(), String> {
    download(url, dest)
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    let mut req = ureq::get(url).set("User-Agent", UA);
    if url.starts_with("https://github.com/") || url.starts_with("https://api.github.com/") {
        if let Ok(token) = std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN")) {
            if !token.is_empty() {
                req = req.set("Authorization", &format!("Bearer {token}"));
            }
        }
    }
    let resp = req.call().map_err(|e| format!("GET {url}: {e}"))?;
    if resp.status() != 200 {
        return Err(format!("GET {url} -> HTTP {}", resp.status()));
    }
    let mut reader = resp.into_reader();
    let mut file = File::create(dest).map_err(|e| format!("create {}: {e}", dest.display()))?;
    io::copy(&mut reader, &mut file).map_err(|e| format!("write {}: {e}", dest.display()))?;
    Ok(())
}

fn http_text(url: &str) -> Result<String, String> {
    let mut req = ureq::get(url).set("User-Agent", UA);
    if url.starts_with("https://api.github.com/") {
        if let Ok(token) = std::env::var("GH_TOKEN").or_else(|_| std::env::var("GITHUB_TOKEN")) {
            if !token.is_empty() {
                req = req.set("Authorization", &format!("Bearer {token}"));
            }
        }
    }
    let resp = req.call().map_err(|e| format!("GET {url}: {e}"))?;
    if resp.status() != 200 {
        return Err(format!("GET {url} -> HTTP {}", resp.status()));
    }
    let mut text = String::new();
    resp.into_reader()
        .read_to_string(&mut text)
        .map_err(|e| format!("read {url}: {e}"))?;
    Ok(text)
}

// ---- update：最新版解析、取证、写回 pin ----

#[derive(Debug)]
pub enum UpdateOutcome {
    Updated { from: String, to: String },
    UpToDate { version: String },
    Skipped { detail: String },
}

pub fn update_agent(home: &Path, name: &str, force: bool) -> Result<UpdateOutcome, String> {
    let mut catalog = resolve_catalog(home)?;
    let pin = catalog
        .find(name)
        .ok_or_else(|| unknown_agent(&catalog, name))?
        .clone();

    let latest = resolve_latest(&pin)?;
    if !force && version_ge(&pin.version, &latest.version) {
        return Ok(UpdateOutcome::UpToDate {
            version: pin.version,
        });
    }

    let new_pin = refresh_pin(&pin, &latest)?;
    let from = pin.version.clone();
    let to = new_pin.version.clone();
    install_pin(&new_pin, home, true)?;

    // 写回用户本地 pin 层（oma 自己维护 pin）。
    for entry in catalog.agents.iter_mut() {
        if entry.name == new_pin.name {
            *entry = new_pin.clone();
        }
    }
    let path = user_catalog_path(home);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    fs::write(&path, render_catalog(&catalog))
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(UpdateOutcome::Updated { from, to })
}

pub struct Latest {
    pub tag: String,
    pub version: String,
}

/// 最新版解析：github 走 releases/latest（tag_name）；CDN 走版本通道
/// （direct 用 version_url 裸文本，manifest 用 {base}/latest 裸文本）。
pub fn resolve_latest(pin: &AgentPin) -> Result<Latest, String> {
    match &pin.sources[0] {
        PinSource::Github { repo, .. } => {
            let text = http_text(&format!(
                "https://api.github.com/repos/{repo}/releases/latest"
            ))?;
            let json: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| format!("parse release json: {e}"))?;
            let tag = json["tag_name"]
                .as_str()
                .ok_or("release json has no tag_name")?
                .to_string();
            let version = version_from_tag(&tag);
            Ok(Latest { tag, version })
        }
        PinSource::Cdn {
            base,
            style,
            version_url,
            ..
        } => match style {
            CdnStyle::Direct => {
                let url = version_url
                    .as_deref()
                    .ok_or_else(|| format!("cdn source {base} has no version channel"))?;
                let version = http_text(url)?.trim().to_string();
                if version.is_empty() {
                    return Err(format!("version channel {url} returned empty"));
                }
                let tag = tag_replace(&pin.tag, &pin.version, &version);
                Ok(Latest { tag, version })
            }
            CdnStyle::Manifest => {
                let url = format!("{base}/latest");
                let version = http_text(&url)?.trim().to_string();
                if version.is_empty() {
                    return Err(format!("version channel {url} returned empty"));
                }
                let tag = tag_replace(&pin.tag, &pin.version, &version);
                Ok(Latest { tag, version })
            }
        },
    }
}

/// 生成新 pin：逐 source 逐资产取官方 sha256 证据（不猜、不全则整体失败保旧 pin）。
fn refresh_pin(pin: &AgentPin, latest: &Latest) -> Result<AgentPin, String> {
    let mut sources = Vec::new();
    for src in &pin.sources {
        match src {
            PinSource::Github { repo, sums, assets } => {
                let new_assets = refresh_github_assets(repo, pin, latest, sums, assets)?;
                sources.push(PinSource::Github {
                    repo: repo.clone(),
                    sums: sums.clone(),
                    assets: new_assets,
                });
            }
            PinSource::Cdn {
                base,
                style,
                version_url,
                assets,
            } => match style {
                CdnStyle::Manifest => {
                    let new_assets = refresh_manifest_assets(base, latest, assets)?;
                    sources.push(PinSource::Cdn {
                        base: base.clone(),
                        style: *style,
                        version_url: version_url.clone(),
                        assets: new_assets,
                    });
                }
                CdnStyle::Direct => {
                    // 无官方清单（grok）：逐资产下载自算 sha（取证即下载，装时复用缓存不可靠故直接算）。
                    let mut new_assets = Vec::new();
                    for a in assets {
                        let name = renamed_asset(a, pin, latest);
                        let url = format!("{base}/{name}");
                        eprintln!("oma: hashing {url}");
                        let tmp = std::env::temp_dir().join(format!(
                            "oma-sha-{}-{}",
                            std::process::id(),
                            name.replace('/', "_")
                        ));
                        download(&url, &tmp)?;
                        let sha = sha256_file(&tmp).map_err(|e| e.to_string())?;
                        fs::remove_file(&tmp).ok();
                        new_assets.push(AgentAsset {
                            os: a.os.clone(),
                            arch: a.arch.clone(),
                            name,
                            sha256: sha.to_ascii_lowercase(),
                            kind: a.kind,
                        });
                    }
                    sources.push(PinSource::Cdn {
                        base: base.clone(),
                        style: *style,
                        version_url: version_url.clone(),
                        assets: new_assets,
                    });
                }
            },
        }
    }
    Ok(AgentPin {
        name: pin.name.clone(),
        tag: latest.tag.clone(),
        version: latest.version.clone(),
        binary: pin.binary.clone(),
        sources,
    })
}

/// GitHub 取证：release JSON 的 `assets[].digest`（`sha256:<hex>`，codex install.sh 同法），
/// 缺 digest 再回落 sums 清单（asset 模式整文件或 sidecar 逐资产）。
fn refresh_github_assets(
    repo: &str,
    pin: &AgentPin,
    latest: &Latest,
    sums: &Option<crate::catalog::Sums>,
    assets: &[AgentAsset],
) -> Result<Vec<AgentAsset>, String> {
    let text = http_text(&format!(
        "https://api.github.com/repos/{repo}/releases/tags/{}",
        latest.tag
    ))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse release json: {e}"))?;
    let mut digests: Vec<(String, String)> = Vec::new();
    if let Some(arr) = json["assets"].as_array() {
        for a in arr {
            let name = a["name"].as_str().unwrap_or("").to_string();
            let digest = a["digest"].as_str().unwrap_or("").to_string();
            if !name.is_empty() {
                digests.push((name, digest));
            }
        }
    }
    let mut out = Vec::new();
    for a in assets {
        let name = renamed_asset(a, pin, latest);
        let mut sha = digests
            .iter()
            .find(|(n, _)| n == &name)
            .and_then(|(_, d)| d.strip_prefix("sha256:").map(|s| s.to_ascii_lowercase()));
        if sha.is_none() {
            sha = Some(github_sums_sha(repo, latest, sums, &name)?);
        }
        out.push(AgentAsset {
            os: a.os.clone(),
            arch: a.arch.clone(),
            name,
            sha256: sha.unwrap(),
            kind: a.kind,
        });
    }
    Ok(out)
}

fn github_sums_sha(
    repo: &str,
    latest: &Latest,
    sums: &Option<crate::catalog::Sums>,
    asset_name: &str,
) -> Result<String, String> {
    let Some(s) = sums else {
        return Err(format!(
            "no digest for {asset_name} and no sums config (tag {})",
            latest.tag
        ));
    };
    match s.mode {
        SumsMode::Asset => {
            let name = s.name.as_deref().unwrap_or("");
            let text = http_text(&format!(
                "https://github.com/{repo}/releases/download/{}/{name}",
                latest.tag
            ))?;
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Some((sha, file)) = line.split_once(char::is_whitespace) {
                    if file.trim() == asset_name && is_sha256_hex(sha) {
                        return Ok(sha.to_ascii_lowercase());
                    }
                }
            }
            Err(format!("{asset_name} not in {name}"))
        }
        SumsMode::Sidecar => {
            let text = http_text(&format!(
                "https://github.com/{repo}/releases/download/{}/{asset_name}.sha256",
                latest.tag
            ))?;
            let sha = text
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if is_sha256_hex(&sha) {
                Ok(sha)
            } else {
                Err(format!("sidecar for {asset_name} has no sha256"))
            }
        }
    }
}

/// kimi CDN manifest 取证：platforms["<plat>"].filename/checksum 一次拿全。
fn refresh_manifest_assets(
    base: &str,
    latest: &Latest,
    assets: &[AgentAsset],
) -> Result<Vec<AgentAsset>, String> {
    let text = http_text(&format!("{base}/binaries/{}/manifest.json", latest.version))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse manifest json: {e}"))?;
    let mut out = Vec::new();
    for a in assets {
        let plat = platform_key(&a.os, &a.arch);
        let entry = json["platforms"][&plat]
            .as_object()
            .ok_or_else(|| format!("manifest has no platform {plat}"))?;
        let name = entry["filename"]
            .as_str()
            .ok_or("manifest entry no filename")?;
        let sha = entry["checksum"]
            .as_str()
            .ok_or("manifest entry no checksum")?;
        if !is_sha256_hex(sha) {
            return Err(format!("manifest checksum invalid for {plat}: {sha}"));
        }
        out.push(AgentAsset {
            os: a.os.clone(),
            arch: a.arch.clone(),
            name: name.to_string(),
            sha256: sha.to_ascii_lowercase(),
            kind: AgentKind::Single,
        });
    }
    Ok(out)
}

fn platform_key(os: &str, arch: &str) -> String {
    let os = match os {
        "windows" => "win32",
        "macos" => "darwin",
        other => other,
    };
    let arch = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os}-{arch}")
}

/// 资产更名：名字含旧版本则替换（codex/grok），不含则原名（claude/kimi 的 zip 家族）。
fn renamed_asset(a: &AgentAsset, pin: &AgentPin, latest: &Latest) -> String {
    if a.name.contains(&pin.version) {
        a.name.replace(&pin.version, &latest.version)
    } else {
        a.name.clone()
    }
}

fn tag_replace(tag: &str, old_version: &str, new_version: &str) -> String {
    if tag.contains(old_version) {
        tag.replace(old_version, new_version)
    } else {
        new_version.to_string()
    }
}

/// 版本号提取：取最后一段 `@` 之后、首个 ASCII 数字起（v2.1.251、rust-v0.151.0、
/// @moonshot-ai/kimi-code@0.39.1、1.0.13 四形全覆盖）。
pub fn version_from_tag(tag: &str) -> String {
    let tail = tag.rsplit('@').next().unwrap_or(tag);
    match tail.find(|c: char| c.is_ascii_digit()) {
        Some(i) => tail[i..].to_string(),
        None => tail.to_string(),
    }
}

/// 点分数值比较（a >= b）；非数值段按 0 处理。
pub fn version_ge(a: &str, b: &str) -> bool {
    let key = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap_or(0))
            .collect()
    };
    let (mut ka, mut kb) = (key(a), key(b));
    let n = ka.len().max(kb.len());
    ka.resize(n, 0);
    kb.resize(n, 0);
    ka >= kb
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

// ---- pin 写回渲染 ----

/// 渲染成 catalog/agents.toml 同 schema（oma agents update 写用户本地层用）。
pub fn render_catalog(catalog: &AgentsCatalog) -> String {
    let mut out = String::new();
    out.push_str("# oma 用户本地 pin 层（oma agents update 写回；删除本文件可重置为出厂 pin）。\n");
    for pin in &catalog.agents {
        out.push_str("\n[[agents]]\n");
        out.push_str(&format!("name = \"{}\"\n", pin.name));
        out.push_str(&format!("tag = \"{}\"\n", pin.tag));
        out.push_str(&format!("version = \"{}\"\n", pin.version));
        out.push_str(&format!("binary = \"{}\"\n", pin.binary));
        for src in &pin.sources {
            match src {
                PinSource::Github { repo, sums, assets } => {
                    out.push_str("\n[[agents.sources]]\nkind = \"github\"\n");
                    out.push_str(&format!("repo = \"{repo}\"\n"));
                    if let Some(s) = sums {
                        out.push_str("\n[agents.sources.sums]\n");
                        out.push_str(&format!(
                            "mode = \"{}\"\n",
                            match s.mode {
                                SumsMode::Asset => "asset",
                                SumsMode::Sidecar => "sidecar",
                            }
                        ));
                        if let Some(name) = &s.name {
                            out.push_str(&format!("name = \"{name}\"\n"));
                        }
                    }
                    push_assets(&mut out, assets);
                }
                PinSource::Cdn {
                    base,
                    style,
                    version_url,
                    assets,
                } => {
                    out.push_str("\n[[agents.sources]]\nkind = \"cdn\"\n");
                    out.push_str(&format!("base = \"{base}\"\n"));
                    out.push_str(&format!(
                        "style = \"{}\"\n",
                        match style {
                            CdnStyle::Direct => "direct",
                            CdnStyle::Manifest => "manifest",
                        }
                    ));
                    if let Some(v) = version_url {
                        out.push_str(&format!("version_url = \"{v}\"\n"));
                    }
                    push_assets(&mut out, assets);
                }
            }
        }
    }
    out
}

fn push_assets(out: &mut String, assets: &[AgentAsset]) {
    for a in assets {
        out.push_str("\n[[agents.sources.assets]]\n");
        out.push_str(&format!("os = \"{}\"\n", a.os));
        out.push_str(&format!("arch = \"{}\"\n", a.arch));
        out.push_str(&format!("name = \"{}\"\n", a.name));
        out.push_str(&format!("sha256 = \"{}\"\n", a.sha256));
        out.push_str(&format!(
            "kind = \"{}\"\n",
            match a.kind {
                AgentKind::Zip => "zip",
                AgentKind::TarGz => "tar_gz",
                AgentKind::Single => "single",
            }
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn fresh() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-install-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn touch(path: &Path) {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(path, b"fake").unwrap();
    }

    #[test]
    fn find_binary_prefers_shallowest_codex_layout() {
        let root = fresh();
        let name = binary_file_name("codex");
        touch(&root.join("bin").join(&name));
        touch(&root.join("codex-resources").join("zsh").join(&name));
        let found = find_binary(&root, "codex").expect("found");
        assert!(found.ends_with(root.join("bin").join(&name)));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn find_binary_nested_kimi_layout() {
        let root = fresh();
        let name = binary_file_name("kimi");
        touch(&root.join("kimi-code").join("bin").join(&name));
        let found = find_binary(&root, "kimi").expect("found");
        assert!(found.ends_with(root.join("kimi-code").join("bin").join(&name)));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn version_from_tag_covers_four_shapes() {
        assert_eq!(version_from_tag("v2.1.251"), "2.1.251");
        assert_eq!(version_from_tag("rust-v0.151.0"), "0.151.0");
        assert_eq!(version_from_tag("@moonshot-ai/kimi-code@0.39.1"), "0.39.1");
        assert_eq!(version_from_tag("1.0.13"), "1.0.13");
    }

    #[test]
    fn version_ge_orders() {
        assert!(version_ge("0.151.0", "0.149.1"));
        assert!(!version_ge("0.149.1", "0.151.0"));
        assert!(version_ge("2.1.251", "2.1.246"));
        assert!(version_ge("1.0.13", "1.0.9"));
        assert!(version_ge("1.0.13", "1.0.13"));
    }

    #[test]
    fn render_catalog_round_trips() {
        let cat = AgentsCatalog::load().expect("catalog");
        let text = render_catalog(&cat);
        let back = AgentsCatalog::load_from(&text).expect("reparse");
        back.validate().expect("revalidate");
        assert_eq!(back.agents.len(), cat.agents.len());
        for (a, b) in cat.agents.iter().zip(back.agents.iter()) {
            assert_eq!(a.name, b.name);
            assert_eq!(a.version, b.version);
            assert_eq!(a.sources.len(), b.sources.len());
        }
    }

    #[test]
    fn asset_url_matches_channel_shapes() {
        let cat = AgentsCatalog::load().expect("catalog");
        let claude = cat.find("claude").unwrap();
        let a = claude.asset_for(0, "windows", "x86_64").unwrap();
        assert_eq!(
            asset_url(claude, &claude.sources[0], a).unwrap(),
            "https://github.com/anthropics/claude-code/releases/download/v2.1.251/claude-win32-x64.zip"
        );
        let kimi = cat.find("kimi").unwrap();
        let k = kimi.asset_for(1, "windows", "x86_64").unwrap();
        assert_eq!(
            asset_url(kimi, &kimi.sources[1], k).unwrap(),
            "https://code.kimi.com/kimi-code/binaries/0.39.1/kimi-code-win32-x64.exe"
        );
        let grok = cat.find("grok").unwrap();
        let g = grok.asset_for(0, "linux", "x86_64").unwrap();
        assert_eq!(
            asset_url(grok, &grok.sources[0], g).unwrap(),
            "https://x.ai/cli/grok-1.0.13-linux-x86_64"
        );
    }

    #[test]
    fn managed_binaries_reads_manifest() {
        let home = fresh();
        let dest = agents_root(&home).join("claude").join("2.1.251");
        let bin = dest.join("claude.exe");
        touch(&bin);
        let asset = AgentAsset {
            os: "windows".into(),
            arch: "x86_64".into(),
            name: "claude-win32-x64.zip".into(),
            sha256: "a".repeat(64),
            kind: AgentKind::Zip,
        };
        let pin = AgentPin {
            name: "claude".into(),
            tag: "v2.1.251".into(),
            version: "2.1.251".into(),
            binary: "claude".into(),
            sources: vec![],
        };
        write_manifest(&dest, &pin, &asset, &bin).unwrap();
        let found = managed_binaries(&home);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "claude");
        assert!(found[0].1.ends_with(bin));
        assert_eq!(managed_version(&home, "claude").as_deref(), Some("2.1.251"));
        let _ = fs::remove_dir_all(&home);
    }
}
