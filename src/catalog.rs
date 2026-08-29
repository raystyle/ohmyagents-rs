use serde::Deserialize;

pub const PIN_TOML: &str = include_str!("../catalog/rmux.toml");

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
}
