//! Local store for historical WASM replay simulation kernels (`<data>/kernels/`).
//!
//! ```text
//! <store>/<line>.wasm    the uncompressed Wasm module (`saito-sim-<line>.wasm`)
//! <store>/<line>.toml    its metadata record (`KernelRecord`: line, engine, abi_version, sha256)
//! ```
//!
//! Also supports an override directory via `$SAITO_KERNELS` containing `saito-sim-<line>.wasm`
//! or `<line>.wasm` files for offline or development testing.

use std::io::Read;
use std::path::PathBuf;

use flate2::read::GzDecoder;
use saito_files as fs;
use saito_sim_abi::{ABI_VERSION, KERNELS_DIR_ENV, KernelRecord};
use sha2::{Digest, Sha256};

/// A kernel stored on disk and ready to instantiate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredKernel {
    pub path: PathBuf,
    pub record: KernelRecord,
}

/// Compute lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Decompress gzipped bytes if they begin with the gzip magic (`1f 8b`), else return raw bytes.
pub fn maybe_gunzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut out = Vec::new();
        GzDecoder::new(bytes).read_to_end(&mut out).map_err(|e| format!("invalid gzip kernel: {e}"))?;
        Ok(out)
    } else {
        Ok(bytes.to_vec())
    }
}

/// Local directory store for simulation kernels.
#[derive(Clone, Debug)]
pub struct KernelStore {
    pub dir: PathBuf,
    override_dir: Option<PathBuf>,
}

impl KernelStore {
    pub fn at(dir: PathBuf) -> KernelStore {
        let override_dir = std::env::var_os(KERNELS_DIR_ENV).filter(|v| !v.is_empty()).map(PathBuf::from);
        KernelStore { dir, override_dir }
    }

    pub fn with_override(dir: PathBuf, override_dir: Option<PathBuf>) -> KernelStore {
        KernelStore { dir, override_dir }
    }

    fn wasm_path(&self, line: &str) -> PathBuf {
        self.dir.join(format!("{line}.wasm"))
    }

    fn record_path(&self, line: &str) -> PathBuf {
        self.dir.join(format!("{line}.toml"))
    }

    /// Look up a cached (or `$SAITO_KERNELS` override) kernel for engine minor line `line`.
    pub fn get(&self, line: &str) -> Option<StoredKernel> {
        if let Some(dir) = &self.override_dir {
            for name in [format!("saito-sim-{line}.wasm"), format!("{line}.wasm")] {
                let path = dir.join(name);
                if fs::is_file(&path) {
                    let bytes = fs::read(&path).ok()?;
                    return Some(StoredKernel {
                        path,
                        record: KernelRecord {
                            line: line.to_string(),
                            engine: line.to_string(),
                            abi_version: ABI_VERSION,
                            sha256: sha256_hex(&bytes),
                        },
                    });
                }
            }
        }
        let path = self.wasm_path(line);
        if !fs::is_file(&path) {
            return None;
        }
        let text = fs::read_to_string(self.record_path(line)).ok()?;
        let record: KernelRecord = toml::from_str(&text).ok()?;
        if record.line != line || record.abi_version != ABI_VERSION {
            return None;
        }
        Some(StoredKernel { path, record })
    }

    /// Store a downloaded (possibly gzipped) kernel module after verifying its uncompressed SHA-256
    /// against `expected_sha256` and ensuring its Wasm magic header (`\0asm`).
    pub fn add(
        &self,
        line: &str,
        engine: &str,
        abi_version: u32,
        expected_sha256: &str,
        raw_or_gz: &[u8],
    ) -> Result<StoredKernel, String> {
        if abi_version != ABI_VERSION {
            return Err(format!("kernel {line} has ABI version {abi_version}, host expects {ABI_VERSION}"));
        }
        let wasm = maybe_gunzip(raw_or_gz)?;
        if wasm.get(..4) != Some(b"\0asm".as_slice()) {
            return Err(format!("kernel {line} is not a WebAssembly module"));
        }
        let actual = sha256_hex(&wasm);
        if !actual.eq_ignore_ascii_case(expected_sha256) {
            return Err(format!("kernel {line} SHA-256 mismatch: got {actual}, expected {expected_sha256}"));
        }
        fs::create_dir_all(&self.dir).map_err(|e| format!("{}: {e}", self.dir.display()))?;
        let wasm_path = self.wasm_path(line);
        let part_path = self.dir.join(format!("{line}.wasm.part"));
        fs::write(&part_path, &wasm).map_err(|e| format!("{}: {e}", part_path.display()))?;
        fs::rename(&part_path, &wasm_path).map_err(|e| format!("{}: {e}", wasm_path.display()))?;
        let record = KernelRecord {
            line: line.to_string(),
            engine: engine.to_string(),
            abi_version,
            sha256: actual,
        };
        let toml_text = format!(
            "# Historical WASM replay simulation kernel (S-022).\n{}",
            toml::to_string(&record).map_err(|e| e.to_string())?
        );
        let rec_path = self.record_path(line);
        let rec_part = self.dir.join(format!("{line}.toml.part"));
        fs::write(&rec_part, toml_text).map_err(|e| format!("{}: {e}", rec_part.display()))?;
        fs::rename(&rec_part, &rec_path).map_err(|e| format!("{}: {e}", rec_path.display()))?;
        Ok(StoredKernel { path: wasm_path, record })
    }

    /// Remove a stored kernel and its metadata record.
    pub fn remove(&self, line: &str) -> Result<(), String> {
        for p in [self.record_path(line), self.wasm_path(line)] {
            if fs::exists(&p) {
                fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            }
        }
        Ok(())
    }

    /// Clean up partial downloads (`.part` files) left by an interrupted store.
    pub fn tidy(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.dir) else { return Vec::new() };
        let mut failed = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "part")
                && let Err(e) = fs::remove_file(&path)
            {
                failed.push(format!("removing {}: {e}", path.display()));
            }
        }
        failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};
    use std::io::Write;

    #[test]
    fn kernel_store_verifies_sha256_and_supports_gzip_and_override() {
        let dir = tempfile::tempdir().unwrap();
        let store = KernelStore::with_override(dir.path().join("kernels"), None);
        let fake_wasm = b"\0asm\x01\x00\x00\x00";
        let hash = sha256_hex(fake_wasm);

        // Gzipped kernel verifies and stores cleanly.
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(fake_wasm).unwrap();
        let gz_bytes = gz.finish().unwrap();

        let stored = store.add("0.1", "0.1.2", ABI_VERSION, &hash, &gz_bytes).unwrap();
        assert_eq!(stored.record.line, "0.1");
        assert_eq!(store.get("0.1").unwrap(), stored);

        // Wrong hash is rejected and leaves nothing.
        assert!(store.add("0.2", "0.2.0", ABI_VERSION, &"0".repeat(64), fake_wasm).is_err());
        assert!(store.get("0.2").is_none());

        // Override directory takes precedence.
        let ov = dir.path().join("override");
        std::fs::create_dir_all(&ov).unwrap();
        std::fs::write(ov.join("saito-sim-0.3.wasm"), fake_wasm).unwrap();
        let store_ov = KernelStore::with_override(dir.path().join("kernels"), Some(ov));
        assert!(store_ov.get("0.3").is_some());
    }
}
