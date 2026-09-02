//! 密钥 hook 安全拦截（S030）：oma hook 的第二职责，接管 ohmypwsh
//! secret-guard 的会话出口闸。误报八层防线（S030 误报策略节）：
//! ① 精确前缀硬阻断 ② 实值比对零误报通道 ③ 熵值门（通用赋值类）
//! ④ stopwords 占位符豁免 ⑤ 语料豁免（测试运行时拼接构造）⑥ warn-only
//! 分级出口 ⑦ 日志掩码 ⑧ fail-open（异常不挡活）。
//!
//! 阻断语义对齐 ohmypwsh：PreToolUse / UserPromptSubmit 命中 block 级 →
//! 调用方 exit 2；PostToolUse 只观察不阻断。

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as Json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Block,
    Warn,
}

#[derive(Debug)]
pub struct Finding {
    pub label: &'static str,
    pub masked: String,
    pub tier: Tier,
}

/// guard 判定：`block` 为真时调用方 exit 2；`reasons` 面向 agent stderr。
#[derive(Debug, Default)]
pub struct GuardVerdict {
    pub block: bool,
    pub reasons: Vec<String>,
    pub findings: Vec<Finding>,
}

struct PatternSpec {
    regex: &'static str,
    label: &'static str,
    ignore_case: bool,
    tier: Tier,
    /// 通用赋值类：命中后过 stopword 与熵值门（防线 3/4），低置信放行。
    /// 正则带 1 号捕获组圈住值部，熵值按组算。
    generic: bool,
}

const fn spec(regex: &'static str, label: &'static str) -> PatternSpec {
    PatternSpec {
        regex,
        label,
        ignore_case: false,
        tier: Tier::Block,
        generic: false,
    }
}

const fn generic(regex: &'static str, label: &'static str) -> PatternSpec {
    PatternSpec {
        regex,
        label,
        ignore_case: true,
        tier: Tier::Block,
        generic: true,
    }
}

const fn warn(regex: &'static str, label: &'static str) -> PatternSpec {
    PatternSpec {
        regex,
        label,
        ignore_case: true,
        tier: Tier::Warn,
        generic: false,
    }
}

/// 模式表（清单对齐 ohmypwsh secret-guard；provider 前缀类大小写敏感、
/// 通用赋值类忽略大小写、password 类只 warn——659 误报教训）。
static PATTERNS: &[PatternSpec] = &[
    // provider 前缀（block，防线 1：构造性低误报）
    spec(r"sk-proj-[A-Za-z0-9_-]{20,}", "OpenAI Project Key"),
    spec(
        r"sk-svcacct-[A-Za-z0-9_-]{20,}",
        "OpenAI Service Account Key",
    ),
    spec(r"sk-ant-[a-zA-Z0-9_-]{20,}", "Anthropic API Key"),
    spec(r"sk_live_[a-zA-Z0-9]{24,}", "Stripe Live Secret Key"),
    spec(r"sk_test_[a-zA-Z0-9]{24,}", "Stripe Test Secret Key"),
    spec(r"sk-[a-zA-Z0-9_-]{20,}", "OpenAI API Key"),
    spec(r"kimi-[a-zA-Z0-9]{24,}", "Kimi API Key"),
    spec(r"moonshot-[a-zA-Z0-9]{24,}", "Moonshot API Key"),
    spec(r"AKIA[0-9A-Z]{16}", "AWS Access Key ID"),
    spec(r"ASIA[0-9A-Z]{16}", "AWS Session Key"),
    spec(r"ghp_[a-zA-Z0-9]{36}", "GitHub Personal Token"),
    spec(r"gho_[a-zA-Z0-9]{36}", "GitHub OAuth Token"),
    spec(r"ghu_[a-zA-Z0-9]{36}", "GitHub User Token"),
    spec(r"glpat-[a-zA-Z0-9_-]{20,}", "GitLab Personal Token"),
    spec(r"xox[baprs]-[0-9a-zA-Z]{10,48}", "Slack Token"),
    spec(r"AIza[0-9A-Za-z_-]{35}", "Google API Key"),
    spec(
        r"eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*",
        "JWT Token",
    ),
    spec(
        r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----",
        "PEM Private Key",
    ),
    // 带密码 URI（block，忽略大小写）
    generic(
        r"mongodb\+srv://[^:\s]+:[^@\s]+@",
        "MongoDB URI with password",
    ),
    generic(
        r"postgres(ql)?://[^:\s]+:[^@\s]+@",
        "PostgreSQL URI with password",
    ),
    generic(r"mysql://[^:\s]+:[^@\s]+@", "MySQL URI with password"),
    generic(r"redis://[^:\s]+:[^@\s]+@", "Redis URI with password"),
    // 通用赋值类（block 级但走 stopword + 熵值门）。值部用命名组 v 圈住
    // （URI 类的 `(ql)?` 括号是语法组不是值部，门取不到 v 时用整段）。
    generic(
        r#"api[_-]?key\s*[:=]\s*["']?(?P<v>[A-Za-z0-9_.\-]{16,})["']?"#,
        "Generic API Key",
    ),
    generic(
        r#"secret[_-]?key\s*[:=]\s*["']?(?P<v>[A-Za-z0-9_.\-]{16,})["']?"#,
        "Generic Secret Key",
    ),
    generic(
        r#"token\s*[:=]\s*["']?(?P<v>[A-Za-z0-9_.\-]{16,})["']?"#,
        "Generic Token",
    ),
    generic(r"bearer\s+(?P<v>[A-Za-z0-9_\-\.]{20,})", "Bearer Token"),
    // password 类（只 warn，防线 6：659 误报教训）
    warn(
        r#"password\s*[:=]\s*"[^"']{8,}""#,
        "Hardcoded Password (double-quoted)",
    ),
    warn(
        r"password\s*[:=]\s*'[^']{8,}'",
        "Hardcoded Password (single-quoted)",
    ),
    warn(
        r#"password\s*[:=]\s*[^"'\s]{8,}["']?"#,
        "Hardcoded Password (bare)",
    ),
];

static COMPILED: LazyLock<Vec<(Regex, &PatternSpec)>> = LazyLock::new(|| {
    PATTERNS
        .iter()
        .filter_map(|p| {
            let mut b = regex::RegexBuilder::new(p.regex);
            if p.ignore_case {
                b.case_insensitive(true);
            }
            b.build().ok().map(|r| (r, p))
        })
        .collect()
});

/// stopwords（防线 4，gitleaks 同款思路）：只作用于通用赋值类的值部。
const STOPWORDS: &[&str] = &[
    "example",
    "yourkey",
    "your_key",
    "your-api-key",
    "your_api_key",
    "changeme",
    "change-me",
    "change_me",
    "dummy",
    "placeholder",
    "xxxxx",
    "insert_",
    "0123456789",
    "1234567890",
    "abcdefghijklmnop",
    "qwertyuiop",
];

/// 熵值门下限（防线 3，kingfisher min_entropy / gitleaks entropy 同款）。
const ENTROPY_MIN: f32 = 3.5;

/// Shannon 熵（bits/char）。纯函数不引库。
fn shannon_entropy(s: &str) -> f32 {
    if s.is_empty() {
        return 0.0;
    }
    let mut counts = std::collections::BTreeMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0u32) += 1;
    }
    let n = s.chars().count() as f32;
    counts
        .values()
        .map(|&k| {
            let p = k as f32 / n;
            -p * p.log2()
        })
        .sum()
}

/// 掩码（防线 7）：审计与 stderr 只见前 4 后 4。
pub fn mask(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > 8 {
        let head: String = chars[..4].iter().collect();
        let tail: String = chars[chars.len() - 4..].iter().collect();
        format!("{head}{}{tail}", "*".repeat(chars.len() - 8))
    } else {
        "*".repeat(chars.len())
    }
}

/// 实值比对通道的敏感环境变量名（清单对齐 ohmypwsh secret-guard）。
const SECRET_ENV_NAMES: &[&str] = &[
    "OPENAI_API_KEY",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "GITHUB_TOKEN",
    "GITLAB_TOKEN",
    "SLACK_TOKEN",
    "STRIPE_SECRET_KEY",
    "DATABASE_URL",
    "REDIS_URL",
    "MONGO_URI",
    "MONGODB_URI",
    "PRIVATE_KEY",
    "SECRET_KEY",
    "API_KEY",
    "AUTH_TOKEN",
    "BEARER_TOKEN",
    "JWT_SECRET",
    "KIMI_API_KEY",
    "MOONSHOT_API_KEY",
    "AZURE_OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "DEEPSEEK_API_KEY",
];

/// 实值比对条目：`value` 只在本模块内做子串比对，出口一律掩码。
struct RealSecret {
    label: &'static str,
    display: String,
    value: String,
}

/// 实值比对（防线 2，构造性零误报）：本机真实密钥值直接子串比对。
/// providers.toml **只读明文形态**——sops 密文要起子进程解密，hook 每个
/// 工具调用都跑，不起（密文形态的实值拦截留给 spawn 注入面）。
fn real_secret_values() -> Vec<RealSecret> {
    let mut out = Vec::new();
    for name in SECRET_ENV_NAMES {
        if let Ok(v) = std::env::var(name) {
            if v.len() >= 8 {
                out.push(RealSecret {
                    label: "Real secret value in environment",
                    display: (*name).to_string(),
                    value: v,
                });
            }
        }
    }
    if let Ok(root) = crate::install::oma_home() {
        let path = root.join("providers.toml");
        if let Ok(text) = std::fs::read_to_string(&path) {
            if !text.contains("\n[sops]") && !text.starts_with("[sops]") {
                if let Ok(book) = toml::from_str::<crate::providers::ProviderBook>(&text) {
                    for provider in book.providers.values() {
                        for launch in provider.agents.values() {
                            for (k, v) in &launch.env {
                                if v.len() >= 8 {
                                    out.push(RealSecret {
                                        label: "Real secret value from providers.toml",
                                        display: format!("providers.toml:{k}"),
                                        value: v.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// 扫描：命中列表（含 warn 级，调用方分级处置）。
pub fn scan(text: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    if text.len() < 8 {
        return out;
    }
    for (re, spec) in COMPILED.iter() {
        for m in re.find_iter(text) {
            let whole = m.as_str();
            if spec.generic {
                let value = re
                    .captures(whole)
                    .and_then(|c| c.name("v"))
                    .map(|g| g.as_str())
                    .unwrap_or(whole);
                let lower = value.to_ascii_lowercase();
                if STOPWORDS.iter().any(|s| lower.contains(s)) {
                    continue;
                }
                if shannon_entropy(value) < ENTROPY_MIN {
                    continue;
                }
            }
            out.push(Finding {
                label: spec.label,
                masked: mask(whole),
                tier: spec.tier,
            });
        }
    }
    for real in real_secret_values() {
        if text.contains(&real.value) {
            out.push(Finding {
                label: real.label,
                masked: real.display,
                tier: Tier::Block,
            });
        }
    }
    out
}

/// 从 hook payload 抽待扫描文本（claude/codex/kimi/grok 信封，snake_case
/// 与 camelCase 都认）。返回 (上下文标签, 文本)。
pub fn scan_text(event: &str, payload: &Json) -> Option<(&'static str, String)> {
    let get_str = |keys: &[&str]| -> Option<String> {
        keys.iter()
            .find_map(|k| payload.get(*k).and_then(|x| x.as_str()).map(str::to_string))
    };
    let get_value = |keys: &[&str]| -> Option<&Json> { keys.iter().find_map(|k| payload.get(*k)) };
    match event {
        "pretooluse" => {
            let tool = get_str(&["tool_name", "toolName"])
                .unwrap_or_default()
                .to_lowercase();
            let input = get_value(&["tool_input", "toolInput"])?;
            if matches!(tool.as_str(), "bash" | "shell" | "powershell" | "pwsh") {
                let cmd = input.get("command").and_then(|x| x.as_str())?;
                Some(("Bash command", cmd.to_string()))
            } else {
                let body = serde_json::to_string(input).ok()?;
                Some(("tool input", body))
            }
        }
        "userpromptsubmit" => {
            let p = get_str(&["prompt", "userPrompt"])?;
            Some(("prompt", p))
        }
        "posttooluse" => {
            let resp = get_value(&["tool_response", "toolResponse", "output"])?;
            let body = serde_json::to_string(resp).ok()?;
            Some(("tool response", body))
        }
        _ => None,
    }
}

/// guard 主判定（fail-open：任何一步拿不到文本都放行）。
/// PreToolUse / UserPromptSubmit：block 级命中 → block=true；
/// PostToolUse：只观察不阻断（Codex 路的输出替换不属本层）。
pub fn guard(event: &str, payload: Option<&Json>) -> GuardVerdict {
    let mut v = GuardVerdict::default();
    let Some(payload) = payload else {
        return v;
    };
    let Some((context, text)) = scan_text(event, payload) else {
        return v;
    };
    let findings = scan(&text);
    if findings.is_empty() {
        return v;
    }
    let blocking = matches!(event, "pretooluse" | "userpromptsubmit");
    for f in &findings {
        let reason = format!(
            "{} in {context} ({}{})",
            f.label,
            f.masked,
            if f.tier == Tier::Warn {
                ", warn-only"
            } else {
                ""
            }
        );
        if blocking && f.tier == Tier::Block {
            v.block = true;
            v.reasons.push(reason);
        }
    }
    v.findings = findings;
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // 黄金语料移植自 ohmypwsh `_test_secret_guard`（期望值来自其测试契约，
    // 独立 oracle 非实现镜像）；token 一律运行时拼接构造——oma 源码不落
    // 字面密钥，自家 guard 不误伤自己的开发会话（防线 5）。
    fn ghp() -> String {
        format!("{}{}", "ghp_", "abcdefghijklmnopqrstuvwxyz0123456789")
    }
    fn sk_ant() -> String {
        format!("{}{}", "sk-ant-", "abcdefghijklmnopqrstuvwxyz012345")
    }
    fn sk_generic() -> String {
        format!("{}{}", "sk-", "abcdefghijklmnopqrstuvwxyz012345")
    }

    #[test]
    fn mask_keeps_head4_tail4() {
        assert_eq!(mask("abcdefghijklmnop"), "abcd********mnop");
        assert_eq!(mask("short"), "*****");
    }

    #[test]
    fn shannon_entropy_separates_placeholder_from_secret() {
        assert!(shannon_entropy("aaaaaaaaaaaaaaaa") < ENTROPY_MIN);
        assert!(shannon_entropy("aB3xK9mQ2vZ7nR5t") > ENTROPY_MIN);
    }

    #[test]
    fn provider_prefix_hits_block_tier() {
        let f = scan(&format!(
            "curl -H 'Authorization: Bearer {}' https://x",
            ghp()
        ));
        assert!(f
            .iter()
            .any(|x| x.label == "GitHub Personal Token" && x.tier == Tier::Block));
        let f = scan(&format!("export KEY={}", sk_ant()));
        assert!(f.iter().any(|x| x.label == "Anthropic API Key"));
        let f = scan(&format!("export DEEPSEEK_API_KEY={}", sk_generic()));
        assert!(f.iter().any(|x| x.label == "OpenAI API Key"));
        let f = scan(&format!("echo {}", "AKIAIOSFODNN7EXAMPLE"));
        assert!(f.iter().any(|x| x.label == "AWS Access Key ID"));
        let f = scan("-----BEGIN OPENSSH PRIVATE KEY-----");
        assert!(f.iter().any(|x| x.label == "PEM Private Key"));
    }

    #[test]
    fn uri_with_password_blocks_case_insensitive() {
        let uri = format!(
            "{}{}",
            "MongoDB+SRV", "://admin:hunter2@cluster.example.com/db"
        );
        let f = scan(&uri);
        assert!(f
            .iter()
            .any(|x| x.label == "MongoDB URI with password" && x.tier == Tier::Block));
        let pg = format!("{}{}", "POSTGRESQL", "://user:pa55word@localhost/db");
        assert!(scan(&pg).iter().any(|x| x.label.contains("PostgreSQL")));
    }

    #[test]
    fn entropy_gate_passes_placeholder_generic_values() {
        // 低熵占位：放行（防线 3）。
        assert!(scan("api_key=aaaaaaaaaaaaaaaa").is_empty());
        assert!(scan("token=000000000000000000").is_empty());
        // 高熵通用赋值：阻断。
        let val = "aB3xK9mQ2vZ7nR5tW";
        let f = scan(&format!("api_key={val}"));
        assert!(f
            .iter()
            .any(|x| x.label == "Generic API Key" && x.tier == Tier::Block));
        // bearer 低熵占位（文档写法）放行；变量引用不命中。
        assert!(scan("bearer aaaaaaaaaaaaaaaaaaaaaa").is_empty());
        assert!(scan("Authorization: Bearer $TOKEN").is_empty());
    }

    #[test]
    fn stopwords_pass_placeholder_values() {
        assert!(scan("api_key=YOUR_API_KEY_HERE").is_empty());
        assert!(scan("token=changeme-please-not-real").is_empty());
        assert!(scan("secret_key=XXXXXXXXXXXXXXXX").is_empty());
    }

    #[test]
    fn password_tier_is_warn_not_block() {
        let pw = format!("{}{}", "su", "persecret123");
        let f = scan(&format!("ok: password = '{pw}'"));
        let hit = f.iter().find(|x| x.label.contains("Password")).unwrap();
        assert_eq!(hit.tier, Tier::Warn);
    }

    #[test]
    fn clean_text_has_no_findings() {
        assert!(scan("git push origin main").is_empty());
        assert!(scan("rename the function to parse_config").is_empty());
        // sha256 hex 不误伤。
        assert!(
            scan("sha256: a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2")
                .is_empty()
        );
    }

    #[test]
    fn real_env_value_channel_is_zero_false_positive() {
        let _g = crate::testenv::ENV_LOCK.lock().unwrap();
        let val = "zq9wKxP2mNvB7tRy";
        std::env::set_var("SECRET_KEY", val);
        let f = scan(&format!("echo {val}"));
        std::env::remove_var("SECRET_KEY");
        assert!(
            f.iter()
                .any(|x| x.label == "Real secret value in environment" && x.masked == "SECRET_KEY"),
            "{f:?}"
        );
        // 名单内变量名本体（无值命中）不算。
        assert!(!scan("export SECRET_KEY=")
            .iter()
            .any(|x| x.label.contains("Real secret")));
    }

    #[test]
    fn providers_plaintext_channel_hits_and_sops_is_skipped() {
        let _g = crate::testenv::ENV_LOCK.lock().unwrap();
        let home = std::env::temp_dir().join(format!(
            "oma-guard-prov-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&home).unwrap();
        let val = "PlaintextProvValue9k2";
        std::fs::write(
            home.join("providers.toml"),
            format!("[providers.zhipu.agents.claude.env]\nANTHROPIC_AUTH_TOKEN = \"{val}\"\n"),
        )
        .unwrap();
        std::env::set_var("OMA_HOME", &home);
        let hit = scan(&format!("curl -H auth:{val}"));
        // sops 密文形态：跳过实值通道（不起 sops 子进程）。
        std::fs::write(
            home.join("providers.toml"),
            format!("[providers.zhipu.agents.claude.env]\nfoo = \"bar\"\n\n[sops]\nmac = \"x\"\n"),
        )
        .unwrap();
        let skipped = scan(&format!("curl -H auth:{val}"));
        std::env::remove_var("OMA_HOME");
        let _ = std::fs::remove_dir_all(&home);
        assert!(
            hit.iter()
                .any(|x| x.label == "Real secret value from providers.toml"),
            "{hit:?}"
        );
        assert!(!skipped
            .iter()
            .any(|x| x.label == "Real secret value from providers.toml"));
    }

    #[test]
    fn scan_text_covers_envelope_shapes() {
        let bash = json!({ "tool_name": "Bash", "tool_input": { "command": "echo hi" } });
        assert_eq!(
            scan_text("pretooluse", &bash).map(|(_, t)| t),
            Some("echo hi".into())
        );
        let camel = json!({ "toolName": "bash", "toolInput": { "command": "echo hi" } });
        assert!(scan_text("pretooluse", &camel).is_some());
        let other = json!({ "tool_name": "Write", "tool_input": { "content": "abc" } });
        let (_, t) = scan_text("pretooluse", &other).unwrap();
        assert!(t.contains("abc"));
        let prompt = json!({ "prompt": "hello" });
        assert_eq!(
            scan_text("userpromptsubmit", &prompt).map(|(_, t)| t),
            Some("hello".into())
        );
        let post = json!({ "tool_response": "done" });
        assert!(scan_text("posttooluse", &post).is_some());
        assert!(scan_text("stop", &post).is_none());
    }

    #[test]
    fn posttooluse_observes_without_blocking() {
        let payload = json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "tool_response": format!("ok value={}", ghp()),
        });
        let v = guard("posttooluse", Some(&payload));
        assert!(!v.block);
        assert!(!v.findings.is_empty(), "观察层仍要记发现");
    }

    #[test]
    fn guard_blocks_pretooluse_and_prompt() {
        let pre = json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": format!("echo {}", ghp()) },
        });
        assert!(guard("pretooluse", Some(&pre)).block);
        let prompt =
            json!({ "hook_event_name": "UserPromptSubmit", "prompt": format!("use {}", sk_ant()) });
        assert!(guard("userpromptsubmit", Some(&prompt)).block);
        // fail-open：无 payload / 干净文本放行。
        assert!(!guard("pretooluse", None).block);
        let clean = json!({ "hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_input": { "command": "git status" } });
        assert!(!guard("pretooluse", Some(&clean)).block);
    }
}
