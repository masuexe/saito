//! Version-stable host/guest ABI for sandboxed WASM historical replay kernels (`saito-sim-<line>.wasm`).
//!
//! The boundary carries only pure pixels, pure stereo PCM samples, and version-independent JSON metadata —
//! never internal `Battle`, `Object`, or `HudState` structs — so new versions of saito can host historical
//! simulation kernels indefinitely without maintaining legacy engine code.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Current host/guest ABI version. Hosts refuse kernels with an unsupported ABI version.
pub const ABI_VERSION: u32 = 1;

/// Default remote index URL for published historical simulation kernels.
pub const DEFAULT_KERNELS_INDEX: &str = "https://github.com/masuexe/saito/releases/download/sim-kernels/index.json";

/// Environment variable overriding the remote kernel index URL (e.g. a local test server or mirror).
pub const KERNELS_INDEX_ENV: &str = "SAITO_KERNELS_INDEX";

/// Environment variable naming a local directory of `saito-sim-<line>.wasm` files (for development or offline use).
pub const KERNELS_DIR_ENV: &str = "SAITO_KERNELS";

/// Guest virtual path mount points (preopened strictly read-only by the host).
pub const VFS_CONTENT: &str = "/content";
pub const VFS_POOL_PREFIX: &str = "/pool";
pub const VFS_ROMS: &str = "/roms";

/// How the host mounted ROM files inside the guest's read-only VFS.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RomMount {
    /// Legacy Collection folder mounted at `dir` (guest calls `RomSource::at`).
    Collection { dir: String },
    /// Per-game zip copies mounted at `dir` (guest calls `RomSource::Copies`).
    Copies { dir: String },
    /// Individual ROM files mounted at virtual paths (guest calls `RomSource::Files`).
    Files { paths: Vec<String> },
}

/// Request to play a replay to completion without graphics or sound (`saito_sim_play_out`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayOutRequest {
    pub game: String,
    pub content_dir: String,
    #[serde(default)]
    pub pool: Vec<(String, String)>,
}

/// Result of headless play-out, mirroring `saito_frontend::replay::Outcome`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayOutResponse {
    pub ticks: u64,
    pub rounds: Vec<Option<u8>>,
    /// `BattleResult as u8`, if the set finished.
    #[serde(default)]
    pub result: Option<u8>,
    /// `0` = Set, `1` = Open, `2` = Cut.
    pub end: u8,
    #[serde(default)]
    pub diverged: Option<String>,
    #[serde(default)]
    pub stopped: Option<String>,
}

/// Rendering enhancement flags passed to the guest renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnhancementsConfig {
    #[serde(default)]
    pub text: bool,
    #[serde(default)]
    pub hud: bool,
    #[serde(default)]
    pub motion: bool,
    #[serde(default)]
    pub fill: bool,
    #[serde(default)]
    pub reduce_flashes: bool,
    #[serde(default)]
    pub hide_aura_number: bool,
}

/// Request to render a 240×160 still frame from a replay (`saito_sim_still`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StillRequest {
    pub game: String,
    pub content_dir: String,
    #[serde(default)]
    pub pool: Vec<(String, String)>,
    pub roms: RomMount,
    pub side: u8,
    pub lang: String,
    #[serde(default)]
    pub enhancements: EnhancementsConfig,
}

/// Request to open an interactive replay viewer inside the guest (`saito_sim_viewer_open`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerOpenRequest {
    pub game: String,
    pub content_dir: String,
    #[serde(default)]
    pub pool: Vec<(String, String)>,
    pub roms: RomMount,
    #[serde(default)]
    pub view: Option<u8>,
    pub lang: String,
    #[serde(default)]
    pub enhancements: EnhancementsConfig,
    #[serde(default = "default_true")]
    pub want_sound: bool,
}

fn default_true() -> bool {
    true
}

/// Static replay metadata returned by `saito_sim_viewer_open`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerOpenResponse {
    pub handle: u32,
    pub length: u32,
    pub rounds: Vec<u32>,
    /// `0` = Set, `1` = Open, `2` = Cut.
    pub end: u8,
    pub engine: String,
    pub when: u64,
    pub side: u8,
    /// `0` = Offline, `1` = Online.
    pub kind: u8,
    pub open_setups: bool,
    pub names: [String; 2],
    pub has_sound: bool,
}

/// One party member's HUD state, identified by its version-independent navi key.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyMemberSnapshot {
    pub navi_key: String,
    pub hp: u16,
    pub max_hp: u16,
}

/// One side's party HUD snapshot (for EXE5 DS-style party battles).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartySnapshot {
    pub members: Vec<PartyMemberSnapshot>,
    pub active: u8,
    pub left: Option<u8>,
    pub right: Option<u8>,
    pub switches: u8,
    #[serde(default)]
    pub limit: Option<u32>,
}

/// Dynamic playback state queried from an open guest viewer (`saito_sim_viewer_state`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerState {
    pub position: u32,
    pub indexed: u32,
    pub whole: bool,
    pub paused: bool,
    /// `0` = Half, `1` = Normal, `2` = Double, `3` = Quadruple.
    pub speed: u8,
    pub view: u8,
    pub both: bool,
    pub at_end: bool,
    #[serde(default)]
    pub stopped: Option<String>,
    #[serde(default)]
    pub diverged: Option<String>,
    /// Current tick's held GBA button masks (low 10 bits), indexed by side `[0, 1]`.
    pub held_inputs: [u16; 2],
    /// Optional party strip states, indexed by side `[0, 1]`.
    #[serde(default)]
    pub parties: [Option<PartySnapshot>; 2],
}

/// Published historical simulation kernels index (`kernels/index.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelIndex {
    #[serde(default = "default_abi")]
    pub abi_version: u32,
    /// Published kernels keyed by engine minor line (e.g. `"0.1"`, `"0.2"`, `"0.3"`, `"0.4"`).
    #[serde(default)]
    pub kernels: BTreeMap<String, PublishedKernel>,
}

fn default_abi() -> u32 {
    ABI_VERSION
}

/// One published simulation kernel entry in `kernels/index.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishedKernel {
    pub line: String,
    pub engine: String,
    #[serde(default = "default_abi")]
    pub abi_version: u32,
    /// Hex-encoded SHA-256 of the uncompressed `.wasm` module bytes.
    pub sha256: String,
    /// Download size in bytes (of the `.wasm.gz` or `.wasm` at `url`).
    pub size: u64,
    /// Full HTTPS URL of the `.wasm.gz` (or `.wasm`) artifact.
    pub url: String,
    #[serde(default)]
    pub date: String,
}

/// Local metadata record stored beside `<line>.wasm` (`<line>.toml`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelRecord {
    pub line: String,
    pub engine: String,
    pub abi_version: u32,
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_types_roundtrip_json() {
        let index = KernelIndex {
            abi_version: ABI_VERSION,
            kernels: [(
                "0.4".into(),
                PublishedKernel {
                    line: "0.4".into(),
                    engine: "0.4.0".into(),
                    abi_version: ABI_VERSION,
                    sha256: "a".repeat(64),
                    size: 123_456,
                    url: "https://example.com/saito-sim-0.4.wasm.gz".into(),
                    date: "2026-11-23".into(),
                },
            )]
            .into(),
        };
        let json = serde_json::to_string(&index).unwrap();
        let back: KernelIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(back, index);

        let req = ViewerOpenRequest {
            game: "exe5".into(),
            content_dir: VFS_CONTENT.into(),
            pool: vec![("exe5".into(), format!("{VFS_POOL_PREFIX}/exe5"))],
            roms: RomMount::Collection { dir: VFS_ROMS.into() },
            view: Some(0),
            lang: "en".into(),
            enhancements: EnhancementsConfig { text: true, hud: true, motion: true, ..Default::default() },
            want_sound: true,
        };
        let back_req: ViewerOpenRequest = serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();
        assert_eq!(back_req, req);
    }
}
