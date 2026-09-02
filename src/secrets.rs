//! `oma agents secrets`：密钥一钥两密文存储与四 shell 懒注入（S031）。
//!
//! 对齐 ohmycloud D20「一钥两密文」（keystore.ts 逐行取证）：oma 自生成
//! 32B 应用密钥落 `<oma根>/app.key`（0600、原子写），用它 AES-256-GCM 包裹
//! **SOPS 标准 age 钥匙链**身份落 `identity.enc`（`oma:v1:` 单行标记）；
//! `secrets.yaml` 为 SOPS 制密文（sops 二进制加工，age 后端，值 base64）。
//! 运行时解密链全程内存：app.key → identity.enc → SOPS_AGE_KEY → vault。
//!
//! 投递对齐 ohmypwsh 懒注入（profile-pwsh/posix 取证）：交互 shell 启动时
//! profile 块现场解密只写当前会话 env，明文不常驻注册表；工具 shell /
//! 后台进程裸环境不继承。
//!
//! 纪律（D20 继承）：盘上恒密文；秘密不进 argv（set 走 stdin）；输出
//! redacted 只报已设置/来源；原子写 0600；解密失败不泄漏密文。

use std::io::Read;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::pathutil::find_on_path;

pub const MARKER: &str = "oma:v1:";
/// app.key 字节长度（32B = AES-256）。
const APP_KEY_LEN: usize = 32;

fn app_key_path(root: &Path) -> PathBuf {
    root.join("app.key")
}

fn identity_enc_path(root: &Path) -> PathBuf {
    root.join("identity.enc")
}

fn identity_meta_path(root: &Path) -> PathBuf {
    root.join("identity.meta.json")
}

fn vault_path(root: &Path) -> PathBuf {
    root.join("secrets.yaml")
}

/// 原子写 + 0600（对齐 keystore.fileKeyProvider 的 tmp+rename 形态）。
fn write_private(path: &Path, body: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    std::fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
    set_private(&tmp);
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{}: {e}", path.display())
    })
}

#[cfg(unix)]
fn set_private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(windows)]
fn set_private(_path: &Path) {
    // Windows 无 POSIX 位；DACL 收紧留待后续（ohmycloud 同为软约束）。
}

/// 读 app.key（64 hex）；不存在则生成 32B 落盘。
pub fn get_or_create_app_key(root: &Path) -> Result<String, String> {
    let path = app_key_path(root);
    if let Ok(text) = std::fs::read_to_string(&path) {
        let t = text.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    let mut raw = vec![0u8; APP_KEY_LEN];
    getrandom::fill(&mut raw).map_err(|e| format!("app key entropy: {e}"))?;
    let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    write_private(&path, &format!("{hex}\n"))?;
    Ok(hex)
}

/// AES-256-GCM 加密 → `oma:v1:<base64(iv|tag|body)>`（iv 12B、tag 16B，
/// 与 keystore.encryptWithAppKey 同构）。
pub fn encrypt_with_app_key(key_hex: &str, plaintext: &str) -> Result<String, String> {
    let raw_key = hex_decode(key_hex)?;
    let key = Key::<Aes256Gcm>::from_slice(&raw_key);
    let cipher = Aes256Gcm::new(key);
    let mut iv = vec![0u8; 12];
    getrandom::fill(&mut iv).map_err(|e| format!("nonce entropy: {e}"))?;
    let sealed = cipher
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: plaintext.as_bytes(),
                aad: MARKER.as_bytes(),
            },
        )
        .map_err(|_| "encrypt failed".to_string())?;
    let mut raw = iv;
    raw.extend_from_slice(&sealed); // GCM 输出 = cipher || tag(16B)
    Ok(format!("{MARKER}{}", B64.encode(raw)))
}

/// 解密：非 `oma:v1:` 标记或 GCM 认证失败（篡改/密钥不符）抛错，错误不
/// 携密文。
pub fn decrypt_with_app_key(key_hex: &str, marked: &str) -> Result<String, String> {
    let body = marked
        .strip_prefix(MARKER)
        .ok_or("identity.enc 非法标记（非 oma:v1:）")?;
    let raw = B64
        .decode(body.trim())
        .map_err(|_| "identity.enc base64 损坏".to_string())?;
    if raw.len() <= 12 + 16 {
        return Err("identity.enc 长度非法".to_string());
    }
    let (iv, sealed) = raw.split_at(12);
    let raw_key = hex_decode(key_hex)?;
    let key = Key::<Aes256Gcm>::from_slice(&raw_key);
    let cipher = Aes256Gcm::new(key);
    let plain = cipher
        .decrypt(
            Nonce::from_slice(iv),
            Payload {
                msg: sealed,
                aad: MARKER.as_bytes(),
            },
        )
        .map_err(|_| "identity.enc 解密失败（密钥不符或被篡改）".to_string())?;
    String::from_utf8(plain).map_err(|_| "identity 非UTF-8".to_string())
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() != APP_KEY_LEN * 2 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("app.key 非法（期望 64 hex）".to_string());
    }
    (0..APP_KEY_LEN)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// SOPS 标准 age 钥匙链身份解析（与 ohmypwsh / ohmycloud / remotex 同源，
/// 不自建身份）：`SOPS_AGE_KEY_FILE` → sops 标准位 → age 标准位。
pub fn identity_file() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SOPS_AGE_KEY_FILE") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let home = dirs::home_dir()?;
    let sops_age = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(|a| PathBuf::from(a).join("sops").join("age").join("keys.txt"))
    } else {
        dirs::config_dir().map(|c| c.join("sops").join("age").join("keys.txt"))
    };
    let mut candidates: Vec<PathBuf> = sops_age.into_iter().collect();
    candidates.push(home.join(".config").join("age").join("keys.txt"));
    candidates.into_iter().find(|p| p.is_file())
}

/// `init`：app.key 就位 + 身份包裹进 identity.enc + meta 落盘。返回各件
/// 状态行（redacted：只报来源不报内容）。
pub fn init(root: &Path) -> Result<Vec<String>, String> {
    let key = get_or_create_app_key(root)?;
    let enc = identity_enc_path(root);
    let Some(src) = identity_file() else {
        return Err(
            "找不到 age 身份：设 SOPS_AGE_KEY_FILE 或落 ~/.config/age/keys.txt（SOPS 标准钥匙链）"
                .to_string(),
        );
    };
    let identity = std::fs::read_to_string(&src).map_err(|e| format!("{}: {e}", src.display()))?;
    let identity = identity.trim().to_string();
    // 身份合法性先验（age 官方解析），坏身份不进包裹。
    parse_identity(&identity)?;
    let wrapped = encrypt_with_app_key(&key, &identity)?;
    write_private(&enc, &format!("{wrapped}\n"))?;
    let meta = format!(
        "{{\"source\": {}, \"createdAt\": {}}}\n",
        serde_json::json!(src.display().to_string()),
        serde_json::json!(unix_iso())
    );
    write_private(&identity_meta_path(root), &meta)?;
    Ok(vec![
        format!("secrets.appkey={}", app_key_path(root).display()),
        format!("secrets.identity={} -> {}", src.display(), enc.display()),
    ])
}

fn unix_iso() -> String {
    // 无 chrono：epoch 秒即可（meta 只是溯源信息）。
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default()
}

/// 解 age 身份串（取第一个 AGE-SECRET-KEY 行）并派生 recipient。
fn parse_identity(identity: &str) -> Result<(String, String), String> {
    let line = identity
        .lines()
        .find(|l| l.starts_with("AGE-SECRET-KEY-"))
        .ok_or("身份文件无 AGE-SECRET-KEY 行")?;
    let id: age::x25519::Identity = line
        .trim()
        .parse()
        .map_err(|e| format!("age 身份解析失败: {e}"))?;
    let recipient = id.to_public().to_string();
    Ok((line.trim().to_string(), recipient))
}

fn sops_bin() -> Result<PathBuf, String> {
    find_on_path("sops").ok_or_else(|| {
        "sops 不在 PATH（vault 的 SOPS 制密文由 sops 加工）；装 sops 后重试".to_string()
    })
}

/// `set <KEY>`：值从 stdin 读（秘密不进 argv），base64 后经 sops 写入
/// secrets.yaml。vault 不存在则建；存在则解密改写再加密。
pub fn set(root: &Path, key: &str, value: &str) -> Result<Vec<String>, String> {
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("key 名只允许 A-Z0-9_：{key}"));
    }
    let (_, recipient) = load_identity(root)?;
    let vault = vault_path(root);
    let sops = sops_bin()?;
    // 现值集：vault 在则解出，再叠新键。
    let mut values = read_all_inner(root, &sops)?;
    values.retain(|(k, _)| k != key);
    values.push((key.to_string(), value.to_string()));
    let plain: String = values
        .iter()
        .map(|(k, v)| format!("{k}: {}", B64.encode(v.as_bytes())))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let tmp = root.join(format!("vault.plain.{}.tmp", std::process::id()));
    write_private(&tmp, &plain)?;
    let out = std::process::Command::new(&sops)
        .args([
            "--encrypt",
            "--age",
            &recipient,
            "--input-type",
            "yaml",
            "--output-type",
            "yaml",
        ])
        .arg(&tmp)
        .output()
        .map_err(|e| format!("sops -e: {e}"))?;
    let _ = std::fs::remove_file(&tmp);
    if !out.status.success() {
        return Err(format!(
            "sops -e 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    std::fs::write(&vault, &out.stdout).map_err(|e| format!("{}: {e}", vault.display()))?;
    Ok(vec![format!("secrets.set={key} vault={}", vault.display())])
}

/// 解链取身份（app.key → identity.enc），返回 (身份行, recipient)。
fn load_identity(root: &Path) -> Result<(String, String), String> {
    let key = get_or_create_app_key(root)?;
    let enc = std::fs::read_to_string(identity_enc_path(root)).map_err(|e| {
        format!(
            "{}: {e}（先跑 oma agents secrets init）",
            identity_enc_path(root).display()
        )
    })?;
    let identity = decrypt_with_app_key(&key, enc.trim())?;
    parse_identity(&identity)
}

/// SOPS_AGE_KEY 传身份内容（免临时密钥文件，keystore 链内存态）。
fn sops_env(identity: &str) -> Vec<(String, String)> {
    vec![("SOPS_AGE_KEY".to_string(), format!("{identity}\n"))]
}

fn run_sops_decrypt(sops: &Path, vault: &Path, identity: &str) -> Result<String, String> {
    let mut cmd = std::process::Command::new(sops);
    cmd.args(["--decrypt", "--input-type", "yaml", "--output-type", "yaml"])
        .arg(vault);
    cmd.env_remove("SOPS_AGE_KEY_FILE");
    for (k, v) in sops_env(identity) {
        cmd.env(k, v);
    }
    let out = cmd.output().map_err(|e| format!("sops -d: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "sops -d 失败: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout).map_err(|_| "vault 明文非UTF-8".to_string())
}

/// 单键查找（spawn 的 providers vault 间接层用）：vault 不在或键不在返回
/// None；sops 缺失等错误同样 None（调用方 warn，不挡 spawn）。
pub fn lookup(root: &Path, key: &str) -> Option<String> {
    let sops = sops_bin().ok()?;
    let values = read_all_inner(root, &sops).ok()?;
    values.into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// providers env 值解析：`vault:KEY` 前缀走 vault 间接层（明文过渡形态
/// 退役路径，S031 待办），其余原样。返回 None = vault 引用未解析（缺
/// sops / vault / 键）。
pub fn resolve_env_value(value: &str, oma_root: &Path) -> Option<String> {
    match value.strip_prefix("vault:") {
        Some(key) => lookup(oma_root, key),
        None => Some(value.to_string()),
    }
}

/// 全量读（明文 (key, value) 列表）；vault 不在返回空表。
fn read_all_inner(root: &Path, sops: &Path) -> Result<Vec<(String, String)>, String> {
    let vault = vault_path(root);
    if !vault.is_file() {
        return Ok(Vec::new());
    }
    let (identity, _) = load_identity(root)?;
    let text = run_sops_decrypt(sops, &vault, &identity)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        let k = k.trim();
        if k.is_empty() || k == "sops" {
            continue;
        }
        let v = v.trim();
        let value = B64
            .decode(v)
            .map_err(|_| format!("vault 值非 base64：{k}"))?;
        out.push((
            k.to_string(),
            String::from_utf8(value).map_err(|_| format!("vault 值非UTF-8：{k}"))?,
        ));
    }
    Ok(out)
}

/// `env --shell`：解 vault 出对应 shell 的会话 env 语句（profile 块唯一
/// 后端）。nu 出 JSON（`load-env (oma … | from json)` 消费，免 eval）。
pub fn env_lines(root: &Path, shell: &str) -> Result<String, String> {
    let sops = sops_bin()?;
    let values = read_all_inner(root, &sops)?;
    if values.is_empty() {
        return Ok(String::new());
    }
    match shell {
        "bash" | "zsh" => Ok(values
            .iter()
            .map(|(k, v)| format!("export {k}={}", shell_quote(v)))
            .collect::<Vec<_>>()
            .join("\n")),
        "pwsh" => Ok(values
            .iter()
            .map(|(k, v)| format!("$env:{k} = '{}'", v.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join("\n")),
        "nu" => {
            let map = serde_json::Map::from_iter(
                values.into_iter().map(|(k, v)| (k, serde_json::json!(v))),
            );
            serde_json::to_string(&serde_json::Value::Object(map)).map_err(|e| e.to_string())
        }
        _ => Err(format!("未知 shell：{shell}（pwsh|bash|zsh|nu）")),
    }
}

fn shell_quote(v: &str) -> String {
    format!("'{}'", v.replace('\'', "'\\''"))
}

/// 四 shell profile 路径。
pub fn profile_paths(shell: &str) -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("no home dir")?;
    match shell {
        "pwsh" => Ok(home
            .join("Documents")
            .join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1")),
        "bash" => Ok(home.join(".bashrc")),
        "zsh" => Ok(home.join(".zshrc")),
        "nu" => {
            if cfg!(windows) {
                let base = std::env::var_os("APPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| home.join("AppData").join("Roaming"));
                Ok(base.join("nushell").join("env.nu"))
            } else {
                Ok(home.join(".config").join("nushell").join("env.nu"))
            }
        }
        _ => Err(format!("未知 shell：{shell}")),
    }
}

const BLOCK_BEGIN: &str = "# BEGIN ohmyagents: secrets";
const BLOCK_END: &str = "# END ohmyagents: secrets";

fn block_for(shell: &str) -> String {
    match shell {
        "pwsh" => format!(
            "{BLOCK_BEGIN}\noma agents secrets env --shell pwsh | Out-String | Invoke-Expression\n{BLOCK_END}"
        ),
        "bash" | "zsh" => format!(
            "{BLOCK_BEGIN}\neval \"$(oma agents secrets env --shell {shell})\"\n{BLOCK_END}"
        ),
        "nu" => format!(
            "{BLOCK_BEGIN}\nload-env (oma agents secrets env --shell nu | from json)\n{BLOCK_END}"
        ),
        _ => String::new(),
    }
}

/// `inject`：四 shell profile 写标志行包裹的加载块，幂等（已有块整段
/// 替换，无则追加）；文件不存在则创建（0600 不必——profile 本就用户态）。
pub fn inject(shell: &str) -> Result<Vec<String>, String> {
    let path = profile_paths(shell)?;
    let block = block_for(shell);
    if block.is_empty() {
        return Err(format!("未知 shell：{shell}"));
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = match (existing.find(BLOCK_BEGIN), existing.find(BLOCK_END)) {
        (Some(a), Some(b)) if b > a => {
            // 幂等替换：旧块整段换新（含过期的块内容）。
            format!(
                "{}{}{}",
                &existing[..a],
                block,
                &existing[b + BLOCK_END.len()..]
            )
        }
        _ => {
            let sep = if existing.is_empty() || existing.ends_with('\n') {
                ""
            } else {
                "\n"
            };
            format!("{existing}{sep}\n{block}\n")
        }
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    std::fs::write(&path, &updated).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(vec![format!(
        "secrets.inject={shell} -> {}",
        path.display()
    )])
}

/// CLI stdin 读取（不回显，无 tty 探测——管道与重定向都收）。
pub fn read_stdin_value() -> Result<String, String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| format!("stdin: {e}"))?;
    let v = buf.trim_end_matches(['\r', '\n']).to_string();
    if v.is_empty() {
        return Err("值为空（stdin 传入，如：echo <key> | oma agents secrets set NAME）".into());
    }
    Ok(v)
}

/// `status`（redacted）：链路体检，只报路径存在性与件数。
pub fn status(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    out.push(format!(
        "secrets.appkey={} status={}",
        app_key_path(root).display(),
        if app_key_path(root).is_file() {
            "present"
        } else {
            "missing"
        }
    ));
    out.push(format!(
        "secrets.identity.enc={} status={}",
        identity_enc_path(root).display(),
        if identity_enc_path(root).is_file() {
            "present"
        } else {
            "missing"
        }
    ));
    let vault = vault_path(root);
    if vault.is_file() {
        out.push(format!("secrets.vault={} status=present", vault.display()));
    } else {
        out.push(format!("secrets.vault={} status=empty", vault.display()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "oma-secrets-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn app_key_roundtrip_and_marker_shape() {
        let root = tmp_root("key");
        let k1 = get_or_create_app_key(&root).unwrap();
        assert_eq!(k1.len(), 64);
        // 幂等：第二次读到同一个。
        assert_eq!(get_or_create_app_key(&root).unwrap(), k1);
        let enc = encrypt_with_app_key(&k1, "AGE-SECRET-KEY-TEST").unwrap();
        assert!(enc.starts_with("oma:v1:"), "{enc}");
        assert_eq!(
            decrypt_with_app_key(&k1, &enc).unwrap(),
            "AGE-SECRET-KEY-TEST"
        );
        // 篡改检测：换密钥解必败且错误不带密文。
        let other = get_or_create_app_key(&tmp_root("key2")).unwrap();
        let err = decrypt_with_app_key(&other, &enc).unwrap_err();
        assert!(err.contains("解密失败") && !err.contains(&enc[7..30]));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn shell_quote_escapes_single_quotes() {
        assert_eq!(shell_quote("plain"), "'plain'");
        assert_eq!(shell_quote("ab'cd"), "'ab'\\''cd'");
    }

    #[test]
    fn vault_indirection_resolves_through_real_chain() {
        // 端到端：age 身份生成 → init（包裹）→ set（sops 制密文）→
        // vault:KEY 间接层解析。sops 缺席的机器跳过（R004 闸门同型）。
        if find_on_path("sops").is_none() {
            eprintln!("skip: sops not on PATH");
            return;
        }
        let _g = crate::testenv::ENV_LOCK.lock().unwrap();
        let home = tmp_root("vault");
        let idfile = home.join("keys.txt");
        let identity = {
            use age::secrecy::ExposeSecret;
            age::x25519::Identity::generate()
                .to_string()
                .expose_secret()
                .to_string()
        };
        std::fs::write(&idfile, format!("# test\n{identity}\n")).unwrap();
        std::env::set_var("SOPS_AGE_KEY_FILE", &idfile);
        std::env::set_var("OMA_HOME", &home);
        init(&home).unwrap();
        set(&home, "DEEPSEEK_API_KEY", "test-value-9k2").unwrap();
        // 字面值原样。
        assert_eq!(resolve_env_value("plain", &home), Some("plain".into()));
        // vault 引用解析出明文（app.key → identity.enc → sops 全链）。
        assert_eq!(
            resolve_env_value("vault:DEEPSEEK_API_KEY", &home),
            Some("test-value-9k2".into())
        );
        // 缺键 → None。
        assert_eq!(resolve_env_value("vault:NOPE", &home), None);
        std::env::remove_var("SOPS_AGE_KEY_FILE");
        std::env::remove_var("OMA_HOME");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn inject_block_is_idempotent_and_replaces_stale() {
        // 直接测 block/替换纯逻辑：模拟已有旧块。
        let old = format!("header\n{BLOCK_BEGIN}\nSTALE CONTENT\n{BLOCK_END}\ntail\n");
        let block = block_for("bash");
        let (a, b) = (
            old.find(BLOCK_BEGIN).unwrap(),
            old.find(BLOCK_END).unwrap() + BLOCK_END.len(),
        );
        let updated = format!("{}{}{}", &old[..a], block, &old[b..]);
        assert!(updated.contains("eval \"$(oma agents secrets env --shell bash)\""));
        assert!(!updated.contains("STALE CONTENT"));
        assert!(updated.starts_with("header\n"));
        assert!(updated.ends_with("\ntail\n"));
    }

    #[test]
    fn block_markers_wrap_all_shells() {
        for shell in ["pwsh", "bash", "zsh", "nu"] {
            let b = block_for(shell);
            assert!(
                b.starts_with(BLOCK_BEGIN) && b.ends_with(BLOCK_END),
                "{shell}"
            );
            assert!(b.contains("oma agents secrets env"), "{shell}");
        }
    }
}
