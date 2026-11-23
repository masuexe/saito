//! Recent Replays feed client: uploads completed online replays (when opted in) and fetches / downloads
//! recent replays from the signaling server's `/replays` HTTP API.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Maximum JSON response size when listing recent replays.
const MAX_LIST_BYTES: u64 = 512 * 1024;
/// Maximum response size for an upload acknowledgement.
const MAX_UPLOAD_RESPONSE_BYTES: u64 = 16 * 1024;
/// Maximum replay file size accepted for upload or download (512 KiB).
pub const MAX_REPLAY_BYTES: u64 = 512 * 1024;

/// Summary of an uploaded replay returned by `GET /replays`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FeedItem {
    pub id: String,
    pub game: String,
    pub engine: String,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub played_at: u64,
    #[serde(default)]
    pub uploaded_at: u64,
    #[serde(default)]
    pub names: [String; 2],
    #[serde(default)]
    pub score: [u32; 2],
    #[serde(default)]
    pub rounds: u32,
    #[serde(default)]
    pub ticks: u32,
    #[serde(default)]
    pub result: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Deserialize)]
struct ListResponse {
    #[serde(default)]
    replays: Vec<FeedItem>,
}

/// Match outcome recorded from side 0's (P1's) perspective at the end of an online set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UploadOutcome {
    pub score: [u32; 2],
    pub result: &'static str,
}

#[derive(Serialize)]
struct UploadPayload<'a> {
    dedup_key: String,
    game: &'a str,
    engine: &'a str,
    content_hash: String,
    played_at: u64,
    names: [&'a str; 2],
    score: [u32; 2],
    rounds: usize,
    ticks: usize,
    result: &'a str,
    data_b64: String,
}

/// Resolve the HTTP(S) base URL of the active signaling server (`$SAITO_SIGNAL` > settings > default).
pub fn server_http_base(configured: &str) -> Option<String> {
    let raw = std::env::var("SAITO_SIGNAL")
        .ok()
        .map(|s| crate::settings::clean_signaling_server(&s))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let s = crate::settings::clean_signaling_server(configured);
            (!s.is_empty()).then_some(s)
        })
        .or_else(|| {
            let s = crate::settings::clean_signaling_server(crate::settings::DEFAULT_SIGNAL_SERVER);
            (!s.is_empty()).then_some(s)
        })?;
    ws_to_http(&raw)
}

/// Convert a signaling WebSocket URL (`ws://` / `wss://`, or already `http://` / `https://`) to an HTTP base URL.
pub fn ws_to_http(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if let Some(rest) = trimmed.strip_prefix("wss://") {
        (!rest.is_empty()).then(|| format!("https://{rest}"))
    } else if let Some(rest) = trimmed.strip_prefix("ws://") {
        (!rest.is_empty()).then(|| format!("http://{rest}"))
    } else if let Some(rest) = trimmed.strip_prefix("https://") {
        (!rest.is_empty()).then(|| format!("https://{rest}"))
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        (!rest.is_empty()).then(|| format!("http://{rest}"))
    } else {
        None
    }
}

/// Cache directory where downloaded recent replays are stored for playback without polluting the local library.
pub fn cache_dir() -> PathBuf {
    crate::paths::data().join("replays-feed-cache")
}

/// Valid server replay ID: 16..=64 lowercase hex characters.
pub fn is_valid_id(id: &str) -> bool {
    (16..=64).contains(&id.len()) && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Local cache path for a downloaded feed replay `id`.
pub fn cached_path(id: &str) -> Option<PathBuf> {
    is_valid_id(id).then(|| cache_dir().join(format!("{id}.{}", saito_frontend::replay::EXTENSION)))
}

/// Compute a deterministic deduplication key shared by both players of the same online match.
///
/// Uses `game`, `engine`, `content` hash, `start` digest, and `match_bytes` (omitting local `Info.side`
/// and local clock `Info.when`, which differ between the two players' recordings).
pub fn dedup_key(replay: &saito_frontend::replay::Replay) -> String {
    let mut h = Sha256::new();
    h.update(replay.head.game.as_bytes());
    h.update([0]);
    h.update(replay.head.engine.as_bytes());
    h.update([0]);
    h.update(replay.head.content.to_le_bytes());
    h.update(replay.head.start.to_le_bytes());
    h.update((replay.match_bytes.len() as u64).to_le_bytes());
    h.update(&replay.match_bytes);
    let digest = h.finalize();
    let mut hex = String::with_capacity(32);
    for b in &digest[..16] {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

/// Standard base64 encoding (RFC 4648) with padding.
pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[((triple >> 6) & 0x3f) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(triple & 0x3f) as usize] as char } else { '=' });
    }
    out
}

/// Fetch the recent replays list for `game` from `{base}/replays`.
pub async fn fetch_list(base: &str, game: &str, limit: usize) -> Result<Vec<FeedItem>, String> {
    let base = base.trim_end_matches('/');
    let url = format!("{base}/replays?game={game}&limit={limit}");
    let bytes = crate::http::get(&url, MAX_LIST_BYTES).await?;
    let parsed: ListResponse = serde_json::from_slice(&bytes).map_err(|e| format!("invalid replay list JSON: {e}"))?;
    Ok(parsed.replays.into_iter().filter(|r| is_valid_id(&r.id)).collect())
}

/// Download replay `id` from `{base}/replays/{id}` into the feed cache (or return the existing cached file).
pub async fn download_replay(base: &str, id: &str) -> Result<PathBuf, String> {
    let target = cached_path(id).ok_or_else(|| format!("invalid replay id: {id}"))?;
    if target.is_file() {
        return Ok(target);
    }
    let base = base.trim_end_matches('/');
    let url = format!("{base}/replays/{id}");
    let bytes = crate::http::get(&url, MAX_REPLAY_BYTES).await?;
    if bytes.len() < 5 || &bytes[..4] != saito_replay::MAGIC || bytes[4] != saito_replay::LAYOUT {
        return Err("downloaded file is not a valid saito replay".into());
    }
    let dir = cache_dir();
    saito_files::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = target.with_extension("tmp");
    saito_files::write(&tmp, &bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    saito_files::rename(&tmp, &target).map_err(|e| format!("{}: {e}", target.display()))?;
    Ok(target)
}

/// Copy a cached feed replay into the user's local replays folder so it stays permanently available offline.
pub fn save_to_local(cached: &Path, item: &FeedItem) -> Result<PathBuf, String> {
    let dir = crate::replays::folder();
    saito_files::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stamp = chrono::DateTime::from_timestamp(item.played_at as i64, 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%Y%m%d-%H%M%S").to_string())
        .unwrap_or_else(|| "recent".into());
    let short_id = &item.id[..item.id.len().min(8)];
    let dest = dir.join(format!("{stamp}-{}-{short_id}.{}", item.game, saito_frontend::replay::EXTENSION));
    if !dest.is_file() {
        let bytes = saito_files::read(cached).map_err(|e| format!("{}: {e}", cached.display()))?;
        saito_files::write(&dest, &bytes).map_err(|e| format!("{}: {e}", dest.display()))?;
    }
    Ok(dest)
}

/// Upload a finished local online replay file to `{base}/replays`.
pub async fn upload_file(base: &str, path: &Path, outcome: UploadOutcome) -> Result<(), String> {
    let bytes = saito_files::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.is_empty() || bytes.len() as u64 > MAX_REPLAY_BYTES {
        return Err(format!("replay size {} out of bounds", bytes.len()));
    }
    let replay = saito_frontend::replay::Replay::read(bytes.as_slice()).map_err(|e| format!("reading replay: {e}"))?;
    if replay.ticks.is_empty() {
        return Err("empty replay".into());
    }
    let payload = UploadPayload {
        dedup_key: dedup_key(&replay),
        game: &replay.head.game,
        engine: &replay.head.engine,
        content_hash: format!("{:016x}", replay.head.content),
        played_at: replay.info.when,
        names: [&replay.info.names[0], &replay.info.names[1]],
        score: outcome.score,
        rounds: replay.round_starts().len().max(1),
        ticks: replay.ticks.len(),
        result: outcome.result,
        data_b64: base64_encode(&bytes),
    };
    let body = serde_json::to_vec(&payload).map_err(|e| format!("serializing upload payload: {e}"))?;
    let url = format!("{}/replays", base.trim_end_matches('/'));
    crate::http::post_json(&url, body, MAX_UPLOAD_RESPONSE_BYTES).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_to_http_converts_schemes() {
        assert_eq!(ws_to_http("wss://signal.example.com/"), Some("https://signal.example.com".into()));
        assert_eq!(ws_to_http("ws://127.0.0.1:8787"), Some("http://127.0.0.1:8787".into()));
        assert_eq!(ws_to_http("https://example.com/"), Some("https://example.com".into()));
        assert_eq!(ws_to_http("ftp://example.com"), None);
        assert_eq!(ws_to_http("wss://"), None);
    }

    #[test]
    fn base64_encode_matches_rfc4648() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn valid_id_rejects_traversal_and_short_ids() {
        assert!(is_valid_id("0123456789abcdef"));
        assert!(is_valid_id("0123456789abcdef0123456789abcdef"));
        assert!(!is_valid_id("0123456789abcde"));
        assert!(!is_valid_id("../etc/passwd0000"));
        assert!(!is_valid_id("0123456789ABCDEF"));
    }
}
