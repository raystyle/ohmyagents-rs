//! init: project-level hook/skill deployment. Temp project dir only; the
//! user home is hashed before and after to prove it is never touched.

use std::fs;
use std::path::PathBuf;

use oma::deploy;
use sha2::{Digest, Sha256};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("poc-init: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    println!("poc.name=init");
    println!("poc.os={}", std::env::consts::OS);

    let home = home_dir().ok_or_else(|| "no home dir".to_string())?;
    let watched = [
        home.join(".claude").join("settings.json"),
        home.join(".claude.json"),
        home.join(".codex").join("config.toml"),
        home.join(".codex").join("hooks.json"),
        home.join(".grok").join("config.toml"),
        home.join(".kimi-code").join("config.toml"),
    ];
    let before = fingerprint(&watched);

    let root = std::env::temp_dir().join(format!(
        "oma-poc-init-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ));
    fs::create_dir_all(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    println!("poc.project={}", root.display());

    let result = init_inner(&root);

    // Home check runs regardless of the deployment outcome.
    let after = fingerprint(&watched);
    if before != after {
        return Err("user home changed during init (must never happen)".into());
    }
    println!("poc.home.untouched=true");
    let _ = fs::remove_dir_all(&root);
    println!("poc.cleanup=tempdir");
    result?;
    println!("poc.ok=true");
    Ok(())
}

fn init_inner(root: &std::path::Path) -> Result<(), String> {
    // Foreign content that must survive every deploy untouched.
    let settings = root.join(".claude").join("settings.json");
    fs::create_dir_all(settings.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(
        &settings,
        r#"{"hooks": {"Stop": [{"matcher": "*", "hooks": [
            {"type": "command", "command": "C:\\tools\\fmt.sh"}]}]}}"#,
    )
    .map_err(|e| e.to_string())?;
    fs::write(root.join("AGENTS.md"), "# 用户自己的说明\n").map_err(|e| e.to_string())?;
    println!("poc.seed=foreign-hook+agents-md");

    let first = deploy::apply_project_hooks(root)?;
    for p in &first.wrote {
        println!("init.wrote={p}");
    }
    println!("init.wrote.count={}", first.wrote.len());
    println!(
        "init.skipped.agents_md={}",
        first.skipped.iter().any(|p| p.ends_with("AGENTS.md"))
    );

    // Full tree exists per the S015 deployment matrix.
    for rel in [
        r".claude\settings.json",
        r".codex\hooks.json",
        r".codex\config.toml",
        r".grok\hooks\ohmyagents-state.json",
        r".agents\skills\ohmyagents\SKILL.md",
        r".claude\skills\ohmyagents\SKILL.md",
        r".grok\skills\ohmyagents\SKILL.md",
        r".kimi-code\skills\ohmyagents\SKILL.md",
        r"CLAUDE.md",
    ] {
        let p = root.join(rel);
        if !p.exists() {
            return Err(format!("missing deployed file: {}", p.display()));
        }
        println!("init.tree={rel}");
    }
    // Kimi has no project-level hook registration (S015).
    if root.join(".kimi-code").join("config.toml").exists() {
        return Err("kimi project config.toml must not be written".into());
    }
    println!("init.kimi.hooks=none");
    // User AGENTS.md untouched; CLAUDE.md is the include line.
    let agents = fs::read_to_string(root.join("AGENTS.md")).map_err(|e| e.to_string())?;
    if agents != "# 用户自己的说明\n" {
        return Err("user AGENTS.md was overwritten".into());
    }
    println!("init.agents_md=preserved");
    let claude_md = fs::read_to_string(root.join("CLAUDE.md")).map_err(|e| e.to_string())?;
    if claude_md.trim() != "@AGENTS.md" {
        return Err(format!(
            "CLAUDE.md must be the include line, got {claude_md:?}"
        ));
    }
    println!("init.claude_md=include");

    // Foreign hook survived next to ours.
    let v: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&settings).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let stop = v["hooks"]["Stop"].as_array().ok_or("Stop groups missing")?;
    let foreign_kept = stop.iter().any(|g| {
        g["hooks"][0]["command"]
            .as_str()
            .is_some_and(|c| c.ends_with("fmt.sh"))
    });
    if !foreign_kept {
        return Err("foreign hook entry was dropped".into());
    }
    println!("init.foreign_kept=true");

    // Idempotence: second deploy writes nothing and changes no bytes.
    let snapshot: Vec<(PathBuf, String)> = walk_files(root)?
        .into_iter()
        .map(|p| {
            let body = fs::read_to_string(&p).map_err(|e| e.to_string())?;
            Ok((p.clone(), body))
        })
        .collect::<Result<_, String>>()?;
    let second = deploy::apply_project_hooks(root)?;
    if !second.wrote.is_empty() {
        return Err(format!("redeploy wrote files: {:?}", second.wrote));
    }
    for (p, body) in &snapshot {
        let now = fs::read_to_string(p).map_err(|e| e.to_string())?;
        if now != *body {
            return Err(format!("redeploy changed {}", p.display()));
        }
    }
    println!("init.idempotent=true");
    Ok(())
}

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

fn fingerprint(paths: &[PathBuf]) -> Vec<Option<String>> {
    paths
        .iter()
        .map(|p| {
            fs::read(p)
                .ok()
                .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        })
        .collect()
}

fn walk_files(root: &std::path::Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}
