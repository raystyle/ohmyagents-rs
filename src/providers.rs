//! 提供商别名簿（用户定调 2026-09-02）：oma 管命名 provider profile，
//! spawn 按 `agent@alias` 注入该路的 env（claude 走 `ANTHROPIC_*`）与 argv
//! 追加（codex 走 `-c key=value` 运行时覆写，0.149 实证）。别名与 agent
//! 任意组合（zhipu/deepseek 都兼容 claude 与 codex）。
//! 存储在 oma 自管根 `~/.ohmyagents/providers.toml`，**标准 sops 托管**：
//! 密钥值密文落盘（键名可读），调度时 `sops -d` 内存解密、仅注入 pane env
//! （其它 sops 兼容 CLI 同格式可读）；明文形态兼容过渡。

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

/// 一个 (provider, agent) 组合的注入载荷。
#[derive(Debug, Default, Clone, Deserialize)]
pub struct AgentLaunch {
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub argv: Vec<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Provider {
    #[serde(default)]
    pub agents: BTreeMap<String, AgentLaunch>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ProviderBook {
    #[serde(default)]
    pub providers: BTreeMap<String, Provider>,
}

/// Store path under the oma data root.
pub fn store_path() -> Result<PathBuf, String> {
    Ok(crate::install::oma_home()?.join("providers.toml"))
}

/// Missing file is an empty book, not an error. A sops-managed file (standard
/// sops TOML with `[sops]` metadata) is decrypted IN MEMORY via `sops -d`
/// before parsing — keys stay ciphertext at rest, plaintext exists only in
/// this process and the pane env it injects. Plaintext files parse directly
/// (transition-friendly).
pub fn load() -> Result<ProviderBook, String> {
    let path = store_path()?;
    if !path.exists() {
        return Ok(ProviderBook::default());
    }
    let on_disk = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = if on_disk.contains("\n[sops]") || on_disk.starts_with("[sops]") {
        decrypt_with_sops(&path)?
    } else {
        on_disk
    };
    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn decrypt_with_sops(path: &std::path::Path) -> Result<String, String> {
    let sops = crate::pathutil::find_on_path("sops").ok_or(
        "providers.toml is sops-encrypted but `sops` is not on PATH; install sops or keep the file plaintext",
    )?;
    let out = std::process::Command::new(&sops)
        .arg("-d")
        .arg(path)
        .output()
        .map_err(|e| format!("sops -d {}: {e}", path.display()))?;
    if !out.status.success() {
        return Err(format!(
            "sops -d {} failed: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("sops -d output not utf-8: {e}"))
}

/// Resolve the launch payload for `agent@alias`; the error names the agents
/// the alias actually defines (typo guard).
pub fn resolve(book: &ProviderBook, alias: &str, agent: &str) -> Result<AgentLaunch, String> {
    let provider = book.providers.get(alias).ok_or_else(|| {
        format!(
            "unknown provider alias '{alias}'; defined: {}",
            aliases(book).join(", ")
        )
    })?;
    provider.agents.get(agent).cloned().ok_or_else(|| {
        format!(
            "provider '{alias}' defines no launch for agent '{agent}' (has: {})",
            provider
                .agents
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        )
    })
}

pub fn aliases(book: &ProviderBook) -> Vec<String> {
    book.providers.keys().cloned().collect()
}

/// `agent` 或 `agent@alias` → (agent, Option<alias>)；`@` 在 agent 名里非法
/// （AGENTS 四家名均无 @）。
pub fn split_agent_profile(spec: &str) -> Result<(String, Option<String>), String> {
    match spec.split_once('@') {
        Some((agent, alias)) => {
            if agent.is_empty() || alias.is_empty() {
                Err(format!(
                    "invalid agent spec '{spec}' (empty agent or alias)"
                ))
            } else {
                Ok((agent.to_string(), Some(alias.to_string())))
            }
        }
        None => Ok((spec.to_string(), None)),
    }
}

/// Printable template for `oma agents providers example`：明文形态（sops
/// 托管后密钥值变 ENC[...]，键名保持可读；`sops <文件>` 编辑即加密）。
pub const EXAMPLE_TOML: &str = r#"# ~/.ohmyagents/providers.toml
# 提供商别名簿：oma spawn --agents claude@zhipu,codex@deepseek 按别名注入该路。
# 别名与 agent 任意组合（四格官方口径见 S027：智谱/deepseek 都兼容 claude 与 codex）。
# 密钥值建议 sops 托管：`sops ~/.ohmyagents/providers.toml` 编辑保存后为标准
# sops 密文（oma 调度时内存解密注入 env，其它 sops 兼容 CLI 同样可读）。

# claude x 智谱（官方 Claude API 兼容端点）
[providers.zhipu.agents.claude.env]
ANTHROPIC_BASE_URL = "https://open.bigmodel.cn/api/anthropic"
ANTHROPIC_DEFAULT_SONNET_MODEL = "glm-5.3[1m]"
ANTHROPIC_DEFAULT_OPUS_MODEL = "glm-5.3[1m]"
ANTHROPIC_DEFAULT_HAIKU_MODEL = "glm-5.3-flash[1m]"

# claude x deepseek（官方 Agent Integrations 端点与模型映射）
[providers.deepseek.agents.claude.env]
ANTHROPIC_BASE_URL = "https://api.deepseek.com/anthropic"
ANTHROPIC_MODEL = "deepseek-v4-pro[1m]"
ANTHROPIC_DEFAULT_OPUS_MODEL = "deepseek-v4-pro[1m]"
ANTHROPIC_DEFAULT_SONNET_MODEL = "deepseek-v4-pro[1m]"
ANTHROPIC_DEFAULT_HAIKU_MODEL = "deepseek-v4-flash"
CLAUDE_CODE_SUBAGENT_MODEL = "deepseek-v4-flash"
CLAUDE_CODE_EFFORT_LEVEL = "max"
CLAUDE_CODE_AUTO_COMPACT_WINDOW = "786432"

# codex x deepseek（自包含 -c 覆写，key 走 DEEPSEEK_API_KEY env）
[providers.deepseek.agents.codex]
argv = ["-c", "model_provider=deepseek", "-c", "model=deepseek-v4-pro",
        "-c", "model_providers.deepseek.name=deepseek",
        "-c", "model_providers.deepseek.base_url=https://api.deepseek.com/",
        "-c", "model_providers.deepseek.wire_api=responses",
        "-c", "model_providers.deepseek.env_key=DEEPSEEK_API_KEY"]

# codex x 智谱（官方 Coding Plan：OpenAI Response 协议；models.json 前置见 S027）
[providers.zhipu.agents.codex]
argv = ["-c", "model_provider=ZAI", "-c", "model=glm-5.3",
        "-c", "model_providers.ZAI.name=ZAI",
        "-c", "model_providers.ZAI.base_url=https://open.bigmodel.cn/api/v1",
        "-c", "model_providers.ZAI.wire_api=responses"]
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profiles_and_resolves_any_combination() {
        let book: ProviderBook = toml::from_str(
            r#"
[providers.zhipu.agents.claude.env]
ANTHROPIC_BASE_URL = "https://zhipu.example"
[providers.deepseek.agents.codex]
argv = ["-c", "model_provider=deepseek"]
[providers.deepseek.agents.claude.env]
ANTHROPIC_BASE_URL = "https://deepseek.example"
"#,
        )
        .unwrap();
        let claude = resolve(&book, "zhipu", "claude").unwrap();
        assert_eq!(
            claude.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://zhipu.example")
        );
        // 交叉组合：claude@deepseek。
        let cross = resolve(&book, "deepseek", "claude").unwrap();
        assert_eq!(
            cross.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://deepseek.example")
        );
        let codex = resolve(&book, "deepseek", "codex").unwrap();
        assert_eq!(codex.argv, vec!["-c", "model_provider=deepseek"]);

        let err = resolve(&book, "zhipu", "grok").unwrap_err();
        assert!(err.contains("no launch for agent 'grok'"), "{err}");
        let err = resolve(&book, "nope", "claude").unwrap_err();
        assert!(err.contains("unknown provider alias"), "{err}");
    }

    #[test]
    fn splits_agent_at_profile() {
        assert_eq!(
            split_agent_profile("claude@zhipu").unwrap(),
            ("claude".into(), Some("zhipu".into()))
        );
        assert_eq!(
            split_agent_profile("codex").unwrap(),
            ("codex".into(), None)
        );
        assert!(split_agent_profile("claude@").is_err());
        assert!(split_agent_profile("@zhipu").is_err());
    }
}
