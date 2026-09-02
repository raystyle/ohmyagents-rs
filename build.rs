//! build.rs：把 `docs/web/kanban`（前端构建产物）打成 tar.gz 资源包嵌进
//! 二进制（P0023，S022 路线 C）。产物经 OUT_DIR 交接：
//!   OUT_DIR/kanban-web.tar.gz       资源包本体
//!   OUT_DIR/kanban-web.fingerprint  内容指纹（sha256 前 8 位，指纹目录名）
//! 资产目录变更由 rerun-if-changed 感知，自动重打包重编。

use std::fs;
use std::path::Path;

fn main() {
    let src = Path::new("docs/web/kanban");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR");
    let out_dir = Path::new(&out_dir);

    println!("cargo:rerun-if-changed=docs/web/kanban");

    let index = src.join("index.html");
    if !index.is_file() {
        panic!(
            "docs/web/kanban/index.html missing; run the frontend build first (see docs/web/share-src)"
        );
    }

    let mut tar_buf = Vec::new();
    {
        let gz = flate2::write::GzEncoder::new(&mut tar_buf, flate2::Compression::default());
        let mut archive = tar::Builder::new(gz);
        let mut files = collect_files(src, src).expect("walk kanban assets");
        // 稳定排序：同内容产出的包字节稳定，指纹才可复现。
        files.sort();
        for rel in files {
            let data = fs::read(src.join(&rel)).expect("read asset");
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive
                .append_data(&mut header, &rel, data.as_slice())
                .expect("append asset");
        }
        archive
            .into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gz");
    }

    use sha2::{Digest, Sha256};
    let fp = format!("{:x}", Sha256::digest(&tar_buf));
    fs::write(out_dir.join("kanban-web.tar.gz"), &tar_buf).expect("write tarball");
    fs::write(out_dir.join("kanban-web.fingerprint"), &fp[..8]).expect("write fingerprint");
}

/// 递归收集相对路径（正斜杠，跨平台稳定入包）。
fn collect_files(base: &Path, dir: &Path) -> std::io::Result<Vec<String>> {
    let mut out = Vec::new();
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let p = ent.path();
        if p.is_dir() {
            out.extend(collect_files(base, &p)?);
        } else {
            let rel = p
                .strip_prefix(base)
                .expect("prefix")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(out)
}
