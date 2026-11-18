//! Embeds clean offline content bundles when building a packaged or WebAssembly target (`SAITO_PACKAGED` or
//! `wasm32`) so the app runs completely offline without fetching upstream content catalogs.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const LITTER: [&str; 3] = [".git", ".DS_Store", ".github"];

#[derive(Deserialize)]
struct PackManifest {
    id: String,
    kind: String,
    #[serde(default)]
    depends: Vec<String>,
}

#[derive(Serialize)]
struct BundleManifest {
    game: String,
    line: String,
    revision: u32,
    packs: BTreeMap<String, String>,
}

fn emit_rerun_tree(dir: &Path) {
    println!("cargo:rerun-if-changed={}", dir.display());
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if LITTER.contains(&name) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            emit_rerun_tree(&path);
        } else {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

fn collect_depends(packs: &BTreeMap<String, PackManifest>, id: &str, seen: &mut BTreeSet<String>) {
    if !seen.insert(id.to_string()) {
        return;
    }
    if let Some(manifest) = packs.get(id) {
        for dep in &manifest.depends {
            collect_depends(packs, dep, seen);
        }
    }
}

fn collect_files(prefix: &str, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).expect("read pack dir").flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_str().expect("UTF-8 file name");
        if LITTER.contains(&name) {
            continue;
        }
        let path = entry.path();
        let inner = format!("{prefix}/{name}");
        if path.is_dir() {
            collect_files(&inner, &path, out);
        } else {
            let data = std::fs::read(&path).expect("read pack file");
            out.insert(inner, data);
        }
    }
}

fn write_tar(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        builder.append_data(&mut header, path, data.as_slice()).expect("append tar entry");
    }
    builder.into_inner().expect("finish tar")
}

fn gzip(tar: &[u8]) -> Vec<u8> {
    let mut gz = flate2::GzBuilder::new().mtime(0).write(Vec::new(), flate2::Compression::best());
    gz.write_all(tar).expect("gzip tar");
    gz.finish().expect("finish gzip")
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SAITO_PACKAGED");

    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let embed = std::env::var_os("SAITO_PACKAGED").is_some()
        || std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32");

    if !embed {
        std::fs::write(out_dir.join("bundled.rs"), "pub const BUNDLED: &[(&str, BundleId, &[u8])] = &[];\n")
            .expect("write bundled.rs");
        return;
    }

    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.join("../..");
    let cargo_toml_path = root.join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", cargo_toml_path.display());
    let cargo_toml: toml::Value = toml::from_str(&std::fs::read_to_string(&cargo_toml_path).expect("read root Cargo.toml"))
        .expect("parse root Cargo.toml");
    let version = cargo_toml["workspace"]["package"]["version"].as_str().expect("workspace package version");
    let line: String = version.split('.').take(2).collect::<Vec<_>>().join(".");

    let content_dir = root.join("content");
    emit_rerun_tree(&content_dir);

    let mut packs: BTreeMap<String, PackManifest> = BTreeMap::new();
    for entry in std::fs::read_dir(&content_dir).expect("read content dir").flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.toml");
        if !manifest_path.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&manifest_path).expect("read pack manifest");
        let manifest: PackManifest = toml::from_str(&text).expect("parse pack manifest");
        packs.insert(manifest.id.clone(), manifest);
    }

    let mut generated = String::from("pub const BUNDLED: &[(&str, BundleId, &[u8])] = &[\n");
    for (game, manifest) in &packs {
        if manifest.kind != "game" {
            continue;
        }
        let mut needed = BTreeSet::new();
        collect_depends(&packs, game, &mut needed);
        let bundle_packs: BTreeMap<String, String> = needed.into_iter().map(|id| (id, "bundled".to_string())).collect();
        let bundle_manifest = BundleManifest {
            game: game.clone(),
            line: line.clone(),
            revision: 1,
            packs: bundle_packs.clone(),
        };

        let mut files = BTreeMap::new();
        files.insert("bundle.toml".to_string(), toml::to_string(&bundle_manifest).expect("serialize bundle.toml").into_bytes());
        for pack_id in bundle_packs.keys() {
            collect_files(pack_id, &content_dir.join(pack_id), &mut files);
        }

        let tar = write_tar(&files);
        let id: [u8; 32] = Sha256::digest(&tar).into();
        let gz = gzip(&tar);
        let gz_name = format!("{game}.tar.gz");
        std::fs::write(out_dir.join(&gz_name), gz).expect("write bundled tar.gz");

        writeln!(
            generated,
            "    ({game:?}, BundleId({id:?}), include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{gz_name}\"))),"
        )
        .expect("format bundled entry");
    }
    generated.push_str("];\n");
    std::fs::write(out_dir.join("bundled.rs"), generated).expect("write bundled.rs");
}
