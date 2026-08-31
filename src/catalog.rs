use serde::Deserialize;

pub const PIN_TOML: &str = include_str!("../catalog/rmux.toml");
pub const AGENTS_TOML: &str = include_str!("../catalog/agents.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct RmuxPin {
    pub tag: String,
    pub version: String,
    pub repo: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub os: String,
    pub arch: String,
    pub name: String,
    pub sha256: String,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Zip,
    TarGz,
}

impl RmuxPin {
    pub fn load() -> Result<Self, String> {
        toml::from_str(PIN_TOML).map_err(|e| format!("catalog/rmux.toml: {e}"))
    }

    pub fn asset_for(&self, os: &str, arch: &str) -> Result<&Asset, String> {
        self.assets
            .iter()
            .find(|a| a.os == os && a.arch == arch)
            .ok_or_else(|| format!("no pinned rmux asset for {os}-{arch}; pin is {}", self.tag))
    }
}

/// 一家 agent 的 pin：版本与二进制名，加上有序渠道表（github 默认、CDN 兜底）。
/// 资产按 source 绑定：同一家不同渠道的制品可能不同（kimi 实证：GitHub zip、CDN 裸单二进制）。
#[derive(Debug, Clone, Deserialize)]
pub struct AgentPin {
    pub name: String,
    pub tag: String,
    pub version: String,
    pub binary: String,
    pub sources: Vec<PinSource>,
}

impl AgentPin {
    /// 本机 (os, arch) 在指定渠道序号下的 pin 资产。
    pub fn asset_for(&self, source_idx: usize, os: &str, arch: &str) -> Option<&AgentAsset> {
        self.sources
            .get(source_idx)?
            .assets()
            .iter()
            .find(|a| a.os == os && a.arch == arch)
    }
}

/// 官方校验清单的取法：release 附带清单文件（asset）或逐资产 `.sha256` 边车（sidecar）。
/// update 取证时的兜底通道；GitHub release JSON 的 `assets[].digest` 是首选（codex install.sh 同法）。
#[derive(Debug, Clone, Deserialize)]
pub struct Sums {
    pub mode: SumsMode,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SumsMode {
    Asset,
    Sidecar,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PinSource {
    Github {
        repo: String,
        #[serde(default)]
        sums: Option<Sums>,
        #[serde(default)]
        assets: Vec<AgentAsset>,
    },
    Cdn {
        base: String,
        /// direct：URL = base/<asset>；manifest：版本通道 base/latest，资产在 base/binaries/<version>/<asset>
        /// （kimi 官方 install.sh 实证）。
        #[serde(default)]
        style: CdnStyle,
        #[serde(default)]
        version_url: Option<String>,
        #[serde(default)]
        assets: Vec<AgentAsset>,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CdnStyle {
    #[default]
    Direct,
    Manifest,
}

impl PinSource {
    pub fn kind_name(&self) -> &'static str {
        match self {
            PinSource::Github { .. } => "github",
            PinSource::Cdn { .. } => "cdn",
        }
    }

    pub fn assets(&self) -> &[AgentAsset] {
        match self {
            PinSource::Github { assets, .. } | PinSource::Cdn { assets, .. } => assets,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentAsset {
    pub os: String,
    pub arch: String,
    pub name: String,
    pub sha256: String,
    pub kind: AgentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Zip,
    TarGz,
    Single,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentsCatalog {
    pub agents: Vec<AgentPin>,
}

impl AgentsCatalog {
    pub fn load() -> Result<Self, String> {
        Self::load_from(AGENTS_TOML)
    }

    pub fn load_from(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("catalog/agents.toml: {e}"))
    }

    pub fn find(&self, name: &str) -> Option<&AgentPin> {
        self.agents.iter().find(|a| a.name == name)
    }

    /// 加载期 schema 校验（S017 反面教材三条：残条目、未实现 kind、双源漂移都拦在这）。
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::BTreeSet::new();
        for pin in &self.agents {
            if pin.name.is_empty() {
                return Err("agent entry with empty name".into());
            }
            if !seen.insert(pin.name.clone()) {
                return Err(format!("duplicate agent pin: {}", pin.name));
            }
            if pin.tag.is_empty() || pin.version.is_empty() || pin.binary.is_empty() {
                return Err(format!("agent {} missing tag/version/binary", pin.name));
            }
            if pin.sources.is_empty() {
                return Err(format!("agent {} has no sources", pin.name));
            }
            for src in &pin.sources {
                let label = format!("agent {} source {}", pin.name, src.kind_name());
                match src {
                    PinSource::Github { repo, sums, assets } => {
                        if repo.is_empty() {
                            return Err(format!("{label} without repo"));
                        }
                        if let Some(s) = sums {
                            if s.mode == SumsMode::Asset
                                && s.name.as_deref().unwrap_or("").is_empty()
                            {
                                return Err(format!("{label} sums mode asset without name"));
                            }
                        }
                        check_assets(&label, assets)?;
                    }
                    PinSource::Cdn { base, .. } => {
                        if base.is_empty() {
                            return Err(format!("{label} without base"));
                        }
                        check_assets(&label, src.assets())?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn check_assets(label: &str, assets: &[AgentAsset]) -> Result<(), String> {
    if assets.is_empty() {
        return Err(format!("{label} has no pinned assets"));
    }
    let known = ["windows", "linux", "macos"];
    for a in assets {
        if !is_sha256_hex(&a.sha256) {
            return Err(format!("{label} asset {} has invalid sha256", a.name));
        }
        if !known.contains(&a.os.as_str()) {
            return Err(format!("{label} asset {} unknown os {}", a.name, a.os));
        }
    }
    Ok(())
}

fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_parses_and_covers_four_os() {
        let pin = RmuxPin::load().expect("pin");
        assert_eq!(pin.version, "0.10.0");
        assert_eq!(pin.tag, "v0.10.0");
        for (os, arch) in [
            ("windows", "x86_64"),
            ("linux", "x86_64"),
            ("linux", "aarch64"),
            ("macos", "aarch64"),
            ("macos", "x86_64"),
        ] {
            let a = pin.asset_for(os, arch).unwrap();
            assert_eq!(a.sha256.len(), 64);
            assert!(a.name.contains(&pin.version));
        }
        assert!(pin.asset_for("windows", "aarch64").is_err());
    }

    #[test]
    fn agents_catalog_parses_and_validates() {
        let cat = AgentsCatalog::load().expect("catalog");
        cat.validate().expect("valid");
        for name in ["claude", "codex", "grok", "kimi"] {
            let pin = cat.find(name).unwrap_or_else(|| panic!("{name}"));
            assert!(pin.asset_for(0, "windows", "x86_64").is_some(), "{name}");
        }
        // grok 双 CDN（direct）：x.ai 主带版本通道，GCS 兜底。
        let grok = cat.find("grok").unwrap();
        assert_eq!(grok.sources.len(), 2);
        assert!(matches!(&grok.sources[0], PinSource::Cdn { version_url, .. } if version_url.is_some()));
        assert!(matches!(&grok.sources[1], PinSource::Cdn { version_url, .. } if version_url.is_none()));
        // kimi 双渠道（github 主 + cdn manifest 兜底），两家制品不同（zip vs single）。
        let kimi = cat.find("kimi").unwrap();
        assert_eq!(kimi.sources.len(), 2);
        assert!(matches!(kimi.sources[0], PinSource::Github { .. }));
        assert!(matches!(&kimi.sources[1], PinSource::Cdn { style, .. } if *style == CdnStyle::Manifest));
        assert_eq!(kimi.sources[0].assets()[0].kind, AgentKind::Zip);
        assert_eq!(kimi.sources[1].assets()[0].kind, AgentKind::Single);
        // github 家带 repo 与官方校验清单。
        let claude = cat.find("claude").unwrap();
        assert!(matches!(&claude.sources[0], PinSource::Github { repo, .. } if repo == "anthropics/claude-code"));
        assert!(matches!(&claude.sources[0], PinSource::Github { sums: Some(s), .. } if s.name.as_deref() == Some("SHASUMS256.txt")));
    }

    #[test]
    fn agents_catalog_rejects_schema_violations() {
        // 未实现的 kind 在解析层拒绝（oma 没有实现过的解压类型不许声明）。
        let bad_kind = AGENTS_TOML.replace("kind = \"zip\"", "kind = \"msi\"");
        assert!(AgentsCatalog::load_from(&bad_kind).is_err());
        // 重名、坏 sha 是加载期校验层拒绝。
        let dup = format!(
            "{AGENTS_TOML}\n[[agents]]\nname = \"claude\"\ntag = \"v9\"\nversion = \"9\"\nbinary = \"claude\"\n[[agents.sources]]\nkind = \"cdn\"\nbase = \"https://example.invalid/cli\"\nstyle = \"direct\"\n[[agents.sources.assets]]\nos = \"windows\"\narch = \"x86_64\"\nname = \"claude-fake.zip\"\nsha256 = \"{}\"\nkind = \"zip\"\n",
            "a".repeat(64)
        );
        let parsed = AgentsCatalog::load_from(&dup).expect("parses");
        assert!(parsed.validate().is_err());
        let bad_sha = AGENTS_TOML.replacen("22e6a1ee", "22e6a1eX", 1);
        let parsed = AgentsCatalog::load_from(&bad_sha).expect("parses");
        assert!(parsed.validate().is_err());
        // 无资产残条目（ohmypwsh kimi Win 的坑）在加载期校验拒绝。
        let empty = "[[agents]]\nname = \"x\"\ntag = \"v1\"\nversion = \"1\"\nbinary = \"x\"\n\
                     [[agents.sources]]\nkind = \"github\"\nrepo = \"a/b\"\n";
        let parsed = AgentsCatalog::load_from(empty).expect("parses");
        assert!(parsed.validate().is_err());
    }
}
