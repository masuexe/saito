//! Region check for gating custom UI themes to mainland China (`CN`).
//!
//! Queries the fixed official geolocation endpoint (`GEO_ENDPOINT`, intentionally decoupled from the
//! user-configurable signaling server URL) and persists an integrity-checked local cache so confirmed
//! mainland China users keep working offline for up to [`OFFLINE_GRACE_SECS`] (30 days), while
//! unverified first launches fail closed to the default `pet-hd` theme.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Fixed official geolocation endpoint (decoupled from user-configurable signaling server).
pub const GEO_ENDPOINT: &str = "https://matchmaking.exessrhub.top/geo";

/// Fallback 1: Cloudflare's built-in trace endpoint (independent of `exessrhub.top`).
pub const CF_TRACE_ENDPOINT: &str = "https://www.cloudflare.com/cdn-cgi/trace";

/// Fallback 2: China-reachable public IP geolocation endpoint (covers Cloudflare outages).
pub const IPIP_ENDPOINT: &str = "https://myip.ipip.net/json";

/// Offline grace window for a previously confirmed mainland China verdict (30 days).
pub const OFFLINE_GRACE_SECS: u64 = 30 * 24 * 60 * 60;

/// Maximum tolerated forward clock skew for cached timestamps (24 hours).
const MAX_FUTURE_SKEW_SECS: u64 = 24 * 60 * 60;

/// Maximum JSON response bytes accepted from [`GEO_ENDPOINT`].
const MAX_GEO_BYTES: u64 = 4096;

/// Cache schema version.
const CACHE_VERSION: u32 = 1;

/// Integrity tag domain separator.
const CACHE_TAG_DOMAIN: &[u8] = b"saito-region-cache-v1:";

#[derive(Deserialize)]
struct GeoResponse {
    #[serde(default)]
    country: String,
    #[serde(default)]
    mainland: Option<bool>,
}

#[derive(Deserialize)]
struct IpipResponse {
    #[serde(default)]
    ret: String,
    #[serde(default)]
    data: Option<IpipData>,
}

#[derive(Deserialize)]
struct IpipData {
    #[serde(default)]
    location: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct GeoCache {
    v: u32,
    mainland: bool,
    verified_at: u64,
    sig: String,
}

fn cache_sig(mainland: bool, verified_at: u64) -> String {
    let mut h = Sha256::new();
    h.update(CACHE_TAG_DOMAIN);
    h.update(if mainland { b"1:" } else { b"0:" });
    h.update(verified_at.to_string().as_bytes());
    let digest = h.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

fn now_unix_secs() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

/// Parse the JSON payload returned by `GET /geo` into `Ok(is_mainland_cn)`.
///
/// Missing, empty, or unknown (`"XX"`) country codes return `Err` so indeterminate responses never
/// overwrite an existing valid offline cache.
pub fn parse_geo_response(bytes: &[u8]) -> Result<bool, String> {
    let parsed: GeoResponse = serde_json::from_slice(bytes).map_err(|e| format!("invalid geo JSON: {e}"))?;
    let country = normalize_country_code(&parsed.country)?;
    Ok(country == "CN" && parsed.mainland.unwrap_or(true))
}

/// Parse Cloudflare's `cdn-cgi/trace` plain-text response (`loc=<ISO-2>`).
pub fn parse_cf_trace_response(bytes: &[u8]) -> Result<bool, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("invalid utf-8 trace: {e}"))?;
    let loc = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("loc="))
        .ok_or_else(|| "missing loc= in cdn-cgi/trace".to_string())?;
    let country = normalize_country_code(loc)?;
    Ok(country == "CN")
}

/// Parse `myip.ipip.net/json` (`{"ret":"ok","data":{"location":["中国","省",...]}}`).
///
/// Explicitly excludes Hong Kong, Macau, and Taiwan so mainland China matches ISO `CN`.
pub fn parse_ipip_response(bytes: &[u8]) -> Result<bool, String> {
    let parsed: IpipResponse = serde_json::from_slice(bytes).map_err(|e| format!("invalid ipip JSON: {e}"))?;
    if !parsed.ret.eq_ignore_ascii_case("ok") {
        return Err(format!("ipip status not ok: {:?}", parsed.ret));
    }
    let loc = parsed.data.map(|d| d.location).unwrap_or_default();
    let country = loc.first().map(|s| s.trim()).unwrap_or("");
    if country.is_empty() || matches!(country, "局域网" | "本机地址" | "保留地址" | "共享地址") {
        return Err(format!("indeterminate ipip country: {country:?}"));
    }
    if country != "中国" {
        return Ok(false);
    }
    let region = loc.get(1).map(|s| s.trim()).unwrap_or("");
    let non_mainland = region.starts_with("香港") || region.starts_with("澳门") || region.starts_with("台湾");
    Ok(!non_mainland)
}

fn normalize_country_code(raw: &str) -> Result<String, String> {
    let country = raw.trim().to_ascii_uppercase();
    if country.len() != 2 || country == "XX" || !country.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
        return Err(format!("indeterminate geo country: {raw:?}"));
    }
    Ok(country)
}

/// Check whether `path` holds a valid, untampered, unexpired mainland China cache entry at `now`.
///
/// Fails closed (`false`) when the file is missing, unreadable, malformed, tampered with, expired,
/// or records a non-mainland verdict.
pub fn cached_mainland_allowed_at(path: &Path, now: u64) -> bool {
    let Ok(text) = saito_files::read_to_string(path) else {
        return false;
    };
    let Ok(cache) = toml::from_str::<GeoCache>(&text) else {
        return false;
    };
    if cache.v != CACHE_VERSION || !cache.mainland {
        return false;
    }
    if cache.sig != cache_sig(cache.mainland, cache.verified_at) {
        return false;
    }
    if cache.verified_at > now.saturating_add(MAX_FUTURE_SKEW_SECS) {
        return false;
    }
    now.saturating_sub(cache.verified_at) <= OFFLINE_GRACE_SECS
}

/// Persist an integrity-checked region verdict to `path`.
pub fn save_cache_at(path: &Path, mainland: bool, now: u64) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        saito_files::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let cache = GeoCache { v: CACHE_VERSION, mainland, verified_at: now, sig: cache_sig(mainland, now) };
    let text = toml::to_string(&cache).map_err(|e| e.to_string())?;
    saito_files::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Synchronous initial verdict read at startup before the live `/geo` check completes.
pub fn initial_allowed() -> bool {
    cached_mainland_allowed_at(&crate::paths::geo_cache(), now_unix_secs())
}

/// Perform a live geolocation check against [`GEO_ENDPOINT`], falling back to
/// [`CF_TRACE_ENDPOINT`] and [`IPIP_ENDPOINT`] if the primary endpoint is unreachable or returns
/// an indeterminate response.
///
/// On a determinate response, updates the local cache when `persist` is `true` and returns
/// `Ok(is_mainland)`. If every source fails, leaves any existing cache untouched.
pub async fn check(persist: bool) -> Result<bool, String> {
    let sources: [(&str, fn(&[u8]) -> Result<bool, String>); 3] = [
        (GEO_ENDPOINT, parse_geo_response),
        (CF_TRACE_ENDPOINT, parse_cf_trace_response),
        (IPIP_ENDPOINT, parse_ipip_response),
    ];
    let mut errors = Vec::with_capacity(sources.len());
    for (url, parse) in sources {
        match crate::http::get_direct_fast(url, MAX_GEO_BYTES).await.and_then(|b| parse(&b)) {
            Ok(mainland) => {
                if persist {
                    if let Err(why) = save_cache_at(&crate::paths::geo_cache(), mainland, now_unix_secs()) {
                        crate::errors::log("saving region cache", &why);
                    }
                }
                return Ok(mainland);
            }
            Err(why) => errors.push(why),
        }
    }
    Err(errors.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_and_rejects_indeterminate_geo_responses() {
        assert_eq!(parse_geo_response(br#"{"country":"CN","mainland":true}"#), Ok(true));
        assert_eq!(parse_geo_response(br#"{"country":"cn"}"#), Ok(true));
        assert_eq!(parse_geo_response(br#"{"country":"CN","mainland":false}"#), Ok(false));
        assert_eq!(parse_geo_response(br#"{"country":"US","mainland":false}"#), Ok(false));
        assert_eq!(parse_geo_response(br#"{"country":"HK","mainland":false}"#), Ok(false));
        assert_eq!(parse_geo_response(br#"{"country":"TW","mainland":false}"#), Ok(false));
        assert_eq!(parse_geo_response(br#"{"country":"T1","mainland":false}"#), Ok(false));

        assert!(parse_geo_response(br#"{"country":"XX","mainland":false}"#).is_err());
        assert!(parse_geo_response(br#"{"country":""}"#).is_err());
        assert!(parse_geo_response(br#"{"error":"geolocation unavailable"}"#).is_err());
        assert!(parse_geo_response(b"not json").is_err());
    }

    #[test]
    fn parses_cloudflare_trace_and_ipip_fallback_responses() {
        assert_eq!(parse_cf_trace_response(b"fl=1\nip=1.2.3.4\nloc=CN\ntls=TLSv1.3\n"), Ok(true));
        assert_eq!(parse_cf_trace_response(b"ip=1.2.3.4\nloc=TW\n"), Ok(false));
        assert_eq!(parse_cf_trace_response(b"ip=1.2.3.4\nloc=HK\n"), Ok(false));
        assert_eq!(parse_cf_trace_response(b"ip=1.2.3.4\nloc=US\n"), Ok(false));
        assert!(parse_cf_trace_response(b"ip=1.2.3.4\nloc=XX\n").is_err());
        assert!(parse_cf_trace_response(b"ip=1.2.3.4\ncolo=LAX\n").is_err());

        assert_eq!(
            parse_ipip_response(r#"{"ret":"ok","data":{"ip":"39.144.200.196","location":["中国","新疆","石河子","","移动"]}}"#.as_bytes()),
            Ok(true)
        );
        assert_eq!(
            parse_ipip_response(r#"{"ret":"ok","data":{"ip":"1.1.1.1","location":["中国","香港","","",""]}}"#.as_bytes()),
            Ok(false)
        );
        assert_eq!(
            parse_ipip_response(r#"{"ret":"ok","data":{"ip":"1.1.1.1","location":["中国","澳门","","",""]}}"#.as_bytes()),
            Ok(false)
        );
        assert_eq!(
            parse_ipip_response(r#"{"ret":"ok","data":{"ip":"1.1.1.1","location":["中国","台湾","台北","",""]}}"#.as_bytes()),
            Ok(false)
        );
        assert_eq!(
            parse_ipip_response(r#"{"ret":"ok","data":{"ip":"8.8.8.8","location":["美国","加利福尼亚","","",""]}}"#.as_bytes()),
            Ok(false)
        );
        assert!(parse_ipip_response(r#"{"ret":"ok","data":{"ip":"127.0.0.1","location":["本机地址","",""]}}"#.as_bytes()).is_err());
        assert!(parse_ipip_response(br#"{"ret":"err","data":null}"#).is_err());
    }

    #[test]
    fn cache_fails_closed_respects_grace_period_and_rejects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("region.cache");
        let t0 = 1_795_800_000;

        // First launch with no cache fails closed.
        assert!(!cached_mainland_allowed_at(&path, t0));

        // Confirmed mainland verdict allows offline use within 30-day grace window.
        save_cache_at(&path, true, t0).unwrap();
        assert!(cached_mainland_allowed_at(&path, t0));
        assert!(cached_mainland_allowed_at(&path, t0 + OFFLINE_GRACE_SECS));
        assert!(!cached_mainland_allowed_at(&path, t0 + OFFLINE_GRACE_SECS + 1));

        // Excessive future timestamp skew fails closed.
        assert!(!cached_mainland_allowed_at(&path, t0.saturating_sub(MAX_FUTURE_SKEW_SECS + 10)));

        // Non-mainland verdict immediately locks even within grace window.
        save_cache_at(&path, false, t0 + 100).unwrap();
        assert!(!cached_mainland_allowed_at(&path, t0 + 100));

        // Tampering with the cache file (flipping false -> true) fails signature check.
        let tampered = std::fs::read_to_string(&path).unwrap().replace("mainland = false", "mainland = true");
        std::fs::write(&path, tampered).unwrap();
        assert!(!cached_mainland_allowed_at(&path, t0 + 100));
    }
}
