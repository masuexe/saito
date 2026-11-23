//! Historical WASM simulation kernels (`saito-sim-<line>.wasm`) for exact cross-version replay playback.
//!
//! Mirrors `crate::contents`:
//! - Kernels are cached in `<data>/kernels/` (`saito_sim_host::KernelStore`) or overridden locally via
//!   `$SAITO_KERNELS` (directory containing `saito-sim-<line>.wasm` or `<line>.wasm`).
//! - Remote discovery uses `$SAITO_KERNELS_INDEX` or `Catalog::kernels` from `crate::contents::kernels_url()`.
//! - Downloads are SHA-256 verified against the index before being stored.
//! - Execution runs inside a sandboxed `WasmRuntime` (Cranelift JIT on desktop/Android, Pulley64 zero-JIT
//!   interpreter on iOS or when `SAITO_WASM_PULLEY=1`).

use saito_extract::RomSource;
use saito_frontend::game::Enhancements;
use saito_frontend::replay::Outcome;
use saito_sim_abi::{ABI_VERSION, KERNELS_INDEX_ENV, KernelIndex, PublishedKernel};
use saito_sim_host::{KernelStore, StoredKernel, WasmRuntime, WasmViewer};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const INDEX_FILE: &str = "index.json";
const MAX_INDEX: u64 = 1 << 20;
/// Kernels include bundled WOFF2 fonts for sharp HUD text and Luau; cap downloads generously.
const MAX_KERNEL: u64 = 128 << 20;

/// Whether this platform can instantiate sandboxed historical WASM simulation kernels.
pub fn supported() -> bool {
    !cfg!(target_arch = "wasm32")
}

struct Kernels {
    store: KernelStore,
    runtime: OnceLock<Result<WasmRuntime, String>>,
    env_index: Option<String>,
    read: Mutex<Option<KernelIndex>>,
}

fn kernels() -> &'static Kernels {
    static KERNELS: OnceLock<Kernels> = OnceLock::new();
    KERNELS.get_or_init(|| {
        let data = if cfg!(test) {
            std::env::temp_dir().join(format!("saito-tests-{}", std::process::id()))
        } else {
            crate::paths::data()
        };
        let store = KernelStore::at(data.join("kernels"));
        let env_index = std::env::var(KERNELS_INDEX_ENV).ok().filter(|u| !u.is_empty());
        Kernels::new(store, env_index)
    })
}

fn storing<T>(f: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    static STORING: Mutex<()> = Mutex::new(());
    let _held = saito_threads::lock(&STORING).map_err(|_| "the kernel store's lock was poisoned".to_string())?;
    f()
}

fn keep(path: PathBuf, body: &[u8]) {
    let written = path.parent().map_or(Ok(()), saito_files::create_dir_all).and_then(|()| saito_files::write(&path, body));
    if let Err(why) = written {
        crate::errors::log(&format!("keeping {}", path.display()), &why);
    }
}

/// Find a cached or `$SAITO_KERNELS`-overridden kernel for engine line `line`.
pub fn find(line: &str) -> Option<StoredKernel> {
    if !supported() {
        return None;
    }
    kernels().store.get(line)
}

/// Find a published kernel for engine line `line` in the last-read index (matching [`ABI_VERSION`]).
pub fn published(line: &str) -> Option<PublishedKernel> {
    if !supported() {
        return None;
    }
    kernels().published(line)
}

/// Whether a remote kernel index URL is configured (`$SAITO_KERNELS_INDEX` or `Catalog::kernels`).
pub fn has_index() -> bool {
    supported() && kernels().index_url().is_some()
}

/// Fetch and cache the remote kernel index.
pub async fn fetch_index() -> Result<KernelIndex, String> {
    kernels().fetch_index().await
}

/// Ensure engine line `line`'s kernel is stored locally, downloading and verifying it if listed in the index.
pub async fn fetch_kernel(line: &str, progress: impl FnMut(f32)) -> Result<Option<StoredKernel>, String> {
    if !supported() {
        return Ok(None);
    }
    kernels().fetch_kernel(line, progress).await
}

/// Execute headless `play_out` of a replay inside `kernel_path`.
pub fn play_out(
    kernel_path: &Path,
    game: &str,
    content_dir: &Path,
    pool: &[(String, PathBuf)],
    replay_bytes: &[u8],
) -> Result<Outcome, String> {
    kernels().runtime()?.play_out(kernel_path, game, content_dir, pool, replay_bytes)
}

/// Render a 240×160 still (`0x00RRGGBB` pixels) inside `kernel_path`.
#[allow(clippy::too_many_arguments)]
pub fn still(
    kernel_path: &Path,
    game: &str,
    content_dir: &Path,
    pool: &[(String, PathBuf)],
    roms: &RomSource,
    side: u8,
    lang: &str,
    enhancements: Enhancements,
    replay_bytes: &[u8],
) -> Result<Vec<u32>, String> {
    kernels().runtime()?.still(kernel_path, game, content_dir, pool, roms, side, lang, enhancements, replay_bytes)
}

/// Open an interactive [`WasmViewer`] for a replay inside `kernel_path`.
#[allow(clippy::too_many_arguments)]
pub fn open_viewer(
    kernel_path: &Path,
    game: &str,
    content_dir: &Path,
    pool: &[(String, PathBuf)],
    roms: &RomSource,
    view: Option<u8>,
    lang: &str,
    enhancements: Enhancements,
    want_sound: bool,
    replay_bytes: &[u8],
) -> Result<WasmViewer, String> {
    kernels().runtime()?.open_viewer(kernel_path, game, content_dir, pool, roms, view, lang, enhancements, want_sound, replay_bytes)
}

impl Kernels {
    fn new(store: KernelStore, env_index: Option<String>) -> Kernels {
        let kept = saito_files::read(store.dir.join(INDEX_FILE))
            .ok()
            .and_then(|body| serde_json::from_slice::<KernelIndex>(&body).ok());
        Kernels { store, runtime: OnceLock::new(), env_index, read: Mutex::new(kept) }
    }

    fn runtime(&self) -> Result<&WasmRuntime, String> {
        self.runtime.get_or_init(WasmRuntime::new).as_ref().map_err(Clone::clone)
    }

    fn index_url(&self) -> Option<String> {
        self.env_index.clone().or_else(crate::contents::kernels_url)
    }

    fn published(&self, line: &str) -> Option<PublishedKernel> {
        let read = saito_threads::lock(&self.read).ok()?;
        let entry = read.as_ref()?.kernels.get(line)?;
        (entry.abi_version == ABI_VERSION).then(|| entry.clone())
    }

    async fn fetch_index(&self) -> Result<KernelIndex, String> {
        let Some(url) = self.index_url() else {
            return Err("no remote simulation kernel index is configured".to_string());
        };
        let body = crate::http::get(&url, MAX_INDEX).await?;
        let index: KernelIndex = serde_json::from_slice(&body).map_err(|e| format!("{url}: kernel index parse failed: {e}"))?;
        keep(self.store.dir.join(INDEX_FILE), &body);
        if let Ok(mut read) = saito_threads::lock(&self.read) {
            *read = Some(index.clone());
        }
        Ok(index)
    }

    async fn fetch_kernel(&self, line: &str, mut progress: impl FnMut(f32)) -> Result<Option<StoredKernel>, String> {
        if let Some(stored) = self.store.get(line) {
            return Ok(Some(stored));
        }
        let published = match self.published(line) {
            Some(p) => p,
            None => {
                let index = self.fetch_index().await?;
                match index.kernels.get(line) {
                    Some(p) if p.abi_version == ABI_VERSION => p.clone(),
                    _ => return Ok(None),
                }
            }
        };
        let url = published.url.clone();
        let size = published.size.max(1) as f32;
        let bytes = crate::http::get_reporting(&url, MAX_KERNEL, |got| progress((got as f32 / size).min(1.0))).await?;
        let store = self.store.clone();
        let line_owned = line.to_string();
        let engine = published.engine.clone();
        let sha256 = published.sha256.clone();
        let abi_version = published.abi_version;
        let stored = saito_threads::off_thread("storing sim kernel", move || {
            storing(|| store.add(&line_owned, &engine, abi_version, &sha256, &bytes))
        })
        .await?
        .map_err(|why| format!("{url}: {why}"))?;
        Ok(Some(stored))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io::Write;

    fn serve(root: PathBuf) -> String {
        use std::io::Read as _;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let root = root.clone();
                std::thread::spawn(move || {
                    let mut request = Vec::new();
                    let mut byte = [0u8; 1];
                    while !request.ends_with(b"\r\n\r\n") && stream.read(&mut byte).is_ok_and(|n| n == 1) {
                        request.push(byte[0]);
                    }
                    let request = String::from_utf8_lossy(&request);
                    let path = request.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/').to_string();
                    let answer = match std::fs::read(root.join(&path)) {
                        Ok(body) => [format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).into_bytes(), body].concat(),
                        Err(_) => b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec(),
                    };
                    let _ = stream.write_all(&answer);
                });
            }
        });
        address
    }

    #[test]
    fn kernel_downloads_from_index_and_verifies_sha256() {
        let site = tempfile::tempdir().unwrap();
        let url = serve(site.path().to_path_buf());
        let store_dir = tempfile::tempdir().unwrap();
        let mgr = Kernels::new(KernelStore::at(store_dir.path().join("kernels")), Some(format!("{url}/index.json")));

        let raw = b"\0asm\x01\0\0\0test-kernel";
        let sha = saito_sim_host::store::sha256_hex(raw);
        let gz = saito_content::bundle::gzip(raw);
        std::fs::write(site.path().join("saito-sim-0.1.wasm.gz"), &gz).unwrap();
        std::fs::write(site.path().join("saito-sim-0.2.wasm.gz"), b"\0asm\x01\0\0\0wrong-kernel").unwrap();

        let mut map = BTreeMap::new();
        map.insert(
            "0.1".into(),
            PublishedKernel {
                line: "0.1".into(),
                engine: "0.1.2".into(),
                abi_version: ABI_VERSION,
                sha256: sha.clone(),
                size: gz.len() as u64,
                date: "2026-11-23".into(),
                url: format!("{url}/saito-sim-0.1.wasm.gz"),
            },
        );
        map.insert(
            "0.2".into(),
            PublishedKernel {
                line: "0.2".into(),
                engine: "0.2.0".into(),
                abi_version: ABI_VERSION,
                sha256: sha,
                size: 9,
                date: "2026-11-23".into(),
                url: format!("{url}/saito-sim-0.2.wasm.gz"),
            },
        );
        let index = KernelIndex { abi_version: ABI_VERSION, kernels: map };
        std::fs::write(site.path().join("index.json"), serde_json::to_vec(&index).unwrap()).unwrap();

        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
        runtime.block_on(async {
            let fetched = mgr.fetch_index().await.unwrap();
            assert_eq!(fetched.kernels.len(), 2);
            let mut progress = Vec::new();
            let stored = mgr.fetch_kernel("0.1", |p| progress.push(p)).await.unwrap().expect("0.1 stored");
            assert_eq!(stored.record.line, "0.1");
            assert_eq!(std::fs::read(&stored.path).unwrap(), raw);
            assert!(progress.last() == Some(&1.0));
            // Second fetch hits local cache.
            assert!(mgr.fetch_kernel("0.1", |_| {}).await.unwrap().is_some());
            // Unlisted line returns None.
            assert!(mgr.fetch_kernel("0.9", |_| {}).await.unwrap().is_none());
            // Hash mismatch is rejected.
            let err = mgr.fetch_kernel("0.2", |_| {}).await.unwrap_err();
            assert!(err.contains("SHA-256"), "{err}");
            assert!(mgr.store.get("0.2").is_none());
        });
    }
}
