//! Android release parsing, version comparison, and APK SHA-256 verification.
//!
//! Kept separate from Android JNI so the entire release-discovery and verification pipeline
//! compiles and is unit-tested on desktop CI as well as Android targets.

use crate::Releases;
use serde::Deserialize;

/// A newer release's APK.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The release's version (its tag's, without the `v`).
    pub version: String,
    /// Primary URL where the APK is downloaded from (mirror when available, otherwise GitHub).
    pub url: String,
    /// Optional fallback download URL (e.g. GitHub tag download when `url` points to a mirror).
    pub fallback_url: Option<String>,
    /// The APK's size, in bytes (0 if unknown before GET).
    pub size: u64,
    /// The APK's SHA-256, in hexadecimal (lowercase).
    pub sha256: String,
}

impl Found {
    #[cfg_attr(not(android), allow(dead_code))]
    pub fn version(&self) -> &str {
        &self.version
    }
}

/// Lightweight Android release manifest served at `<mirror>/latest/android.json`.
#[derive(Deserialize)]
struct MirrorAndroidManifest {
    tag: String,
    version: String,
    apk_name: String,
    apk_url: String,
    #[serde(default)]
    size: u64,
    sha256: String,
}

fn normalize_sha256(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let hex = trimmed.strip_prefix("sha256:").or_else(|| trimmed.strip_prefix("SHA256:")).unwrap_or(trimmed).trim();
    let lower = hex.to_ascii_lowercase();
    (lower.len() == 64 && lower.bytes().all(|b| b.is_ascii_hexdigit())).then_some(lower)
}

/// Parse a mirror's `<mirror>/latest/android.json` manifest against `installed`.
/// Validates version, APK filename, download URL, and 64-hex-char SHA-256 digest; attaches the
/// corresponding GitHub tag download URL as `fallback_url` so download can also fall back cleanly.
pub fn parse_mirror_android(json: &[u8], installed: &str, releases: &Releases) -> Result<Option<Found>, String> {
    let manifest: MirrorAndroidManifest =
        serde_json::from_slice(json).map_err(|e| format!("the mirror's android.json: {e}"))?;
    let tag = manifest.tag.trim();
    let version = manifest.version.trim();
    if tag.is_empty() || version.is_empty() {
        return Err("the mirror's android.json has an empty tag or version".into());
    }
    let tag_version = tag.strip_prefix('v').unwrap_or(tag);
    if tag_version != version {
        return Err(format!("the mirror's android.json tag {tag} does not match version {version}"));
    }
    if !is_newer_tag(tag, installed)? {
        return Ok(None);
    }
    let apk_name = manifest.apk_name.trim();
    if apk_name.is_empty() || apk_name.contains('/') || !apk_name.to_ascii_lowercase().ends_with(".apk") {
        return Err(format!("the mirror's android.json has an invalid apk_name {apk_name:?}"));
    }
    let apk_url = manifest.apk_url.trim();
    if !(apk_url.starts_with("https://") || apk_url.starts_with("http://")) {
        return Err(format!("the mirror's android.json has an invalid apk_url {apk_url:?}"));
    }
    let sha256 = normalize_sha256(&manifest.sha256)
        .ok_or_else(|| format!("the mirror's android.json for {tag} has an invalid SHA-256"))?;
    let github_fallback = releases.tag_download(tag, apk_name);
    let fallback_url = (github_fallback != apk_url).then_some(github_fallback);
    Ok(Some(Found {
        version: version.to_owned(),
        url: apk_url.to_owned(),
        fallback_url,
        size: manifest.size,
        sha256,
    }))
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    /// `sha256:` and the file's SHA-256 in hexadecimal (GitHub's, on every release asset since 2025).
    digest: Option<String>,
}

#[derive(Deserialize)]
struct WebLatest {
    tag_name: String,
}

/// Parse GitHub's `https://github.com/{owner}/{repo}/releases/latest` JSON (`Accept: application/json`)
/// for its `tag_name`.
pub fn parse_web_latest_tag(json: &[u8]) -> Result<String, String> {
    let latest: WebLatest = serde_json::from_slice(json).map_err(|e| format!("the latest release's JSON: {e}"))?;
    let tag = latest.tag_name.trim();
    if tag.is_empty() {
        return Err("the latest release's JSON has an empty tag_name".into());
    }
    Ok(tag.to_owned())
}

/// Whether `tag` (with optional leading `v`) is strictly newer than `installed`.
pub fn is_newer_tag(tag: &str, installed: &str) -> Result<bool, String> {
    let version = tag.strip_prefix('v').unwrap_or(tag);
    let (Some(out), Some(ours)) = (numbers(version), numbers(installed)) else {
        return Err(format!("the release's version {version} and the app's {installed} don't compare"));
    };
    Ok(out > ours)
}

/// The release's APK (its one asset whose name ends in `.apk`), if the release (GitHub's API JSON for it) is newer than
/// `installed`, the installed app's version; a release that has none, or more than one, or doesn't say its SHA-256, is
/// an error.
pub fn newer(json: &[u8], installed: &str) -> Result<Option<Found>, String> {
    let release: Release = serde_json::from_slice(json).map_err(|e| format!("the release's JSON: {e}"))?;
    let tag = release.tag_name;
    if !is_newer_tag(&tag, installed)? {
        return Ok(None);
    }
    let version = tag.strip_prefix('v').unwrap_or(&tag);
    // (The release's one APK, whatever its name: the release key signs each release's.)
    let mut apks = release.assets.into_iter().filter(|a| a.name.to_ascii_lowercase().ends_with(".apk"));
    let apk = apks.next().ok_or_else(|| format!("{tag} has no APK"))?;
    if let Some(other) = apks.next() {
        return Err(format!("{tag} has more than one APK ({} and {})", apk.name, other.name));
    }
    let sha256 = apk.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"));
    let sha256 = sha256.ok_or_else(|| format!("{tag}'s {} has no SHA-256", apk.name))?.to_ascii_lowercase();
    Ok(Some(Found {
        version: version.to_owned(),
        url: apk.browser_download_url,
        fallback_url: None,
        size: apk.size,
        sha256,
    }))
}

/// Parse GitHub's `https://github.com/{owner}/{repo}/releases/expanded_assets/{tag}` HTML fragment for
/// the release's single `.apk` asset and its `sha256:` digest (fallback when `api.github.com` is rate-limited
/// or unreachable).
pub fn parse_expanded_assets(html: &str, tag: &str, releases: &Releases) -> Result<Found, String> {
    let prefix = format!("href=\"/{}/{}/releases/download/{tag}/", releases.owner(), releases.repo());
    let mut apks: Vec<String> = Vec::new();
    let mut rest = html;
    while let Some(idx) = rest.find(&prefix) {
        let after = &rest[idx + prefix.len()..];
        let Some(end) = after.find('"') else { break };
        let name = &after[..end];
        if !name.contains('/') && name.to_ascii_lowercase().ends_with(".apk") && !apks.iter().any(|a| a == name) {
            apks.push(name.to_owned());
        }
        rest = &after[end + 1..];
    }
    let apk_name = match apks.as_slice() {
        [] => return Err(format!("{tag} has no APK")),
        [only] => only.as_str(),
        [first, second, ..] => return Err(format!("{tag} has more than one APK ({first} and {second})")),
    };
    let sha256 = extract_html_digest(html, apk_name).ok_or_else(|| format!("{tag}'s {apk_name} has no SHA-256"))?;
    let version = tag.strip_prefix('v').unwrap_or(tag).to_owned();
    Ok(Found {
        version,
        url: releases.tag_download(tag, apk_name),
        fallback_url: None,
        size: 0,
        sha256,
    })
}

/// The most of the release's JSON/HTML read (its assets and its notes: some tens of kilobytes).
#[cfg(any(android, desktop))]
const MAX_RELEASE: u64 = 1 << 20;

/// Check `releases` for an Android APK newer than `installed`:
/// tries the configured mirror (`<mirror>/latest/android.json`) first in a single request,
/// falling back to GitHub (`web_latest` -> tag API / `expanded_assets`, then `api.github.com`).
#[cfg(any(android, desktop))]
pub async fn check_android_releases(
    clients: &crate::http::Clients,
    releases: &Releases,
    installed: &str,
) -> Result<Option<Found>, String> {
    if let Some(mirror_url) = releases.mirror_android_manifest() {
        match clients
            .get_bytes(&mirror_url, MAX_RELEASE)
            .await
            .and_then(|json| parse_mirror_android(&json, installed, releases))
        {
            Ok(found) => return Ok(found),
            Err(mirror_err) => {
                log::warn!("mirror android release check failed ({mirror_err}); falling back to GitHub");
            }
        }
    }
    match fetch_web_latest(clients, releases, installed).await {
        Ok(found) => Ok(found),
        Err(web_err) => {
            log::warn!("github web release check failed ({web_err}); falling back to GitHub API");
            let json = clients
                .get_bytes(&releases.latest(), MAX_RELEASE)
                .await
                .map_err(|api_err| format!("{api_err} (web: {web_err})"))?;
            newer(&json, installed)
        }
    }
}

#[cfg(any(android, desktop))]
async fn fetch_web_latest(
    clients: &crate::http::Clients,
    releases: &Releases,
    installed: &str,
) -> Result<Option<Found>, String> {
    let web_json = clients
        .get_bytes_with_accept(&releases.web_latest(), Some("application/json"), MAX_RELEASE)
        .await?;
    let tag = parse_web_latest_tag(&web_json)?;
    if !is_newer_tag(&tag, installed)? {
        return Ok(None);
    }
    match clients.get_bytes(&releases.release_by_tag(&tag), MAX_RELEASE).await {
        Ok(json) => match newer(&json, installed) {
            Ok(found) => return Ok(found),
            Err(api_err) => log::warn!("github tag API parse failed ({api_err}); falling back to expanded_assets"),
        },
        Err(api_err) => log::warn!("github tag API failed ({api_err}); falling back to expanded_assets"),
    }
    let html_bytes = clients.get_bytes(&releases.expanded_assets(&tag), MAX_RELEASE).await?;
    let html = String::from_utf8_lossy(&html_bytes);
    let mut found = parse_expanded_assets(&html, &tag, releases)?;
    if let Some(len) = clients.head_content_length(&found.url).await {
        found.size = len;
    }
    Ok(Some(found))
}

/// Extract a 64-hex-char SHA-256 digest for `asset_name` from a GitHub `<clipboard-copy ...>` tag.
fn extract_html_digest(html: &str, asset_name: &str) -> Option<String> {
    let needle = format!("Copy to clipboard digest for {asset_name}\"");
    let pos = html.find(&needle)?;
    let tag_start = html[..pos].rfind('<')?;
    let tag_end = pos + html[pos..].find('>')?;
    let tag = &html[tag_start..=tag_end];
    let val_start = tag.find("value=\"sha256:")? + "value=\"sha256:".len();
    let after = &tag[val_start..];
    let val_end = after.find('"')?;
    let hex = after[..val_end].trim().to_ascii_lowercase();
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then_some(hex)
}

/// A version's numbers: major, minor, patch, and whether it is a stable release (no `-prerelease` suffix).
fn numbers(version: &str) -> Option<(u64, u64, u64, bool)> {
    let (core, is_stable) = match version.split_once('-') {
        Some((core, _)) => (core, false),
        None => (version, true),
    };
    let mut parts = core.split('.').map(|part| part.parse().ok());
    let numbers = (parts.next()??, parts.next()??, parts.next()??, is_stable);
    parts.next().is_none().then_some(numbers)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GitHub's JSON for a release of `tag` with these assets (name and digest), as much of it as is read.
    fn release(tag: &str, assets: &[(&str, Option<&str>)]) -> Vec<u8> {
        let assets: Vec<_> = assets
            .iter()
            .map(|(name, digest)| {
                serde_json::json!({
                    "name": name,
                    "size": 97_122_818,
                    "digest": digest,
                    "browser_download_url": format!("https://github.com/masuexe/saito/releases/download/{tag}/{name}"),
                })
            })
            .collect();
        serde_json::json!({ "tag_name": tag, "draft": false, "prerelease": false, "assets": assets }).to_string().into_bytes()
    }

    const SUM: &str = "5b06cc304ef839b194a770ccaa01ae4cbc49ec50c9883a8276d498ef6bc8fc1e";

    fn with_apk(tag: &str) -> Vec<u8> {
        let digest = format!("sha256:{}", SUM.to_ascii_uppercase());
        release(tag, &[("saito-ios.ipa", Some("sha256:00")), ("saito-android.apk", Some(&digest)), ("altstore.json", None)])
    }

    #[test]
    fn a_newer_release_gives_its_apk() {
        let found = newer(&with_apk("v0.21.0"), "0.20.0").unwrap();
        assert_eq!(
            found,
            Some(Found {
                version: "0.21.0".into(),
                url: "https://github.com/masuexe/saito/releases/download/v0.21.0/saito-android.apk".into(),
                fallback_url: None,
                size: 97_122_818,
                sha256: SUM.into(),
            })
        );
    }

    #[test]
    fn versions_compare_by_their_numbers() {
        assert!(newer(&with_apk("v0.10.0"), "0.9.3").unwrap().is_some());
        assert!(newer(&with_apk("v1.0.0"), "0.20.4").unwrap().is_some());
        assert!(newer(&with_apk("v0.20.1"), "0.20.0").unwrap().is_some());
        assert_eq!(newer(&with_apk("v0.9.3"), "0.10.0").unwrap(), None);
    }

    #[test]
    fn the_same_or_an_older_release_is_no_update() {
        assert_eq!(newer(&with_apk("v0.20.0"), "0.20.0").unwrap(), None);
        assert_eq!(newer(&with_apk("v0.19.3"), "0.20.0").unwrap(), None);
    }

    #[test]
    fn a_newer_release_without_its_apk_or_its_sum_is_an_error() {
        assert!(newer(&release("v0.21.0", &[("saito-ios.ipa", Some("sha256:00"))]), "0.20.0").is_err());
        assert!(newer(&release("v0.21.0", &[("saito-android.apk", None)]), "0.20.0").is_err());
    }

    #[test]
    fn the_apk_is_the_releases_one_apk() {
        let digest = format!("sha256:{SUM}");
        let other = release("v0.21.0", &[("altstore.json", None), ("Saito-0.21.0.APK", Some(&digest))]);
        let found = newer(&other, "0.20.0").unwrap().unwrap();
        assert!(found.url.ends_with("/Saito-0.21.0.APK"), "{}", found.url);
        let two = release("v0.21.0", &[("a.apk", Some(&digest)), ("b.apk", Some(&digest))]);
        assert!(newer(&two, "0.20.0").unwrap_err().contains("more than one APK"));
    }

    #[test]
    fn a_version_that_isnt_three_numbers_doesnt_compare() {
        assert!(newer(&with_apk("v0.21"), "0.20.0").is_err());
        assert!(newer(&with_apk("v0.21.0.1"), "0.20.0").is_err());
    }

    #[test]
    fn prerelease_suffixes_compare_below_their_stable_release() {
        assert!(newer(&with_apk("v0.20.0"), "0.20.0-rc1").unwrap().is_some());
        assert_eq!(newer(&with_apk("v0.20.0-rc1"), "0.20.0").unwrap(), None);
        assert!(newer(&with_apk("v0.21.0-rc1"), "0.20.0").unwrap().is_some());
    }

    #[test]
    fn parses_github_web_latest_json() {
        let body = br#"{"id":123,"tag_name":"v0.4.1","update_url":"/masuexe/saito/releases/tag/v0.4.1"}"#;
        let tag = parse_web_latest_tag(body).unwrap();
        assert_eq!(tag, "v0.4.1");
        assert!(is_newer_tag(&tag, "0.4.0").unwrap());
        assert!(!is_newer_tag(&tag, "0.4.1").unwrap());
    }

    #[test]
    fn parses_github_expanded_assets_html() {
        let releases = Releases::github("masuexe", "saito");
        let sum = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let html = format!(
            r#"<li>
  <a href="/masuexe/saito/releases/download/v0.5.0/altstore.json">altstore.json</a>
  <clipboard-copy aria-label="Copy to clipboard digest for altstore.json" value="sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"></clipboard-copy>
</li>
<li>
  <a href="/masuexe/saito/releases/download/v0.5.0/saito-android.apk" rel="nofollow">saito-android.apk</a>
  <clipboard-copy id="clipboard-button-sha256:{sum}" aria-label="Copy to clipboard digest for saito-android.apk" type="button" value="sha256:{sum}"></clipboard-copy>
</li>"#
        );
        let found = parse_expanded_assets(&html, "v0.5.0", &releases).unwrap();
        assert_eq!(
            found,
            Found {
                version: "0.5.0".into(),
                url: "https://github.com/masuexe/saito/releases/download/v0.5.0/saito-android.apk".into(),
                fallback_url: None,
                size: 0,
                sha256: sum.into(),
            }
        );
    }

    #[test]
    fn parses_mirror_android_manifest_and_validates_fields() {
        let releases = Releases::github("masuexe", "saito").with_mirror("https://releases.saito.exessrhub.top");
        let manifest = serde_json::json!({
            "tag": "v0.5.0",
            "version": "0.5.0",
            "apk_name": "saito-android.apk",
            "apk_url": "https://releases.saito.exessrhub.top/v0.5.0/saito-android.apk",
            "size": 97_122_818u64,
            "sha256": format!("sha256:{}", SUM.to_ascii_uppercase()),
        })
        .to_string();

        let found = parse_mirror_android(manifest.as_bytes(), "0.4.9", &releases).unwrap();
        assert_eq!(
            found,
            Some(Found {
                version: "0.5.0".into(),
                url: "https://releases.saito.exessrhub.top/v0.5.0/saito-android.apk".into(),
                fallback_url: Some("https://github.com/masuexe/saito/releases/download/v0.5.0/saito-android.apk".into()),
                size: 97_122_818,
                sha256: SUM.into(),
            })
        );
        assert_eq!(parse_mirror_android(manifest.as_bytes(), "0.5.0", &releases).unwrap(), None);

        // Missing/invalid SHA-256 must fail (never downgrade security).
        let bad_sha = serde_json::json!({
            "tag": "v0.5.0",
            "version": "0.5.0",
            "apk_name": "saito-android.apk",
            "apk_url": "https://releases.saito.exessrhub.top/v0.5.0/saito-android.apk",
            "size": 100u64,
            "sha256": "not-a-sha256",
        })
        .to_string();
        assert!(parse_mirror_android(bad_sha.as_bytes(), "0.4.9", &releases).is_err());

        // Mismatched tag/version must fail.
        let mismatch = serde_json::json!({
            "tag": "v0.5.0",
            "version": "0.5.1",
            "apk_name": "saito-android.apk",
            "apk_url": "https://releases.saito.exessrhub.top/v0.5.0/saito-android.apk",
            "size": 100u64,
            "sha256": SUM,
        })
        .to_string();
        assert!(parse_mirror_android(mismatch.as_bytes(), "0.4.9", &releases).is_err());

        // Invalid apk_url or apk_name must fail.
        let bad_url = serde_json::json!({
            "tag": "v0.5.0",
            "version": "0.5.0",
            "apk_name": "saito-android.apk",
            "apk_url": "file:///tmp/evil.apk",
            "size": 100u64,
            "sha256": SUM,
        })
        .to_string();
        assert!(parse_mirror_android(bad_url.as_bytes(), "0.4.9", &releases).is_err());
    }

    #[cfg(any(android, desktop))]
    fn spawn_http_server(routes: Vec<(String, u16, Vec<u8>)>) -> String {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let path = req.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("");
                let (status, body) = routes
                    .iter()
                    .find(|(p, _, _)| p == path)
                    .map(|(_, s, b)| (*s, b.as_slice()))
                    .unwrap_or((404, b"not found"));
                let reason = match status {
                    200 => "OK",
                    404 => "Not Found",
                    503 => "Service Unavailable",
                    _ => "Error",
                };
                let header = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(body);
            }
        });
        format!("http://{addr}")
    }

    #[cfg(any(android, desktop))]
    #[test]
    fn android_check_prefers_mirror_and_falls_back_to_github() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let clients = crate::http::Clients::direct_for_test().unwrap();

        // 1. Mirror succeeds in a single request.
        let mirror_manifest = serde_json::json!({
            "tag": "v0.6.0",
            "version": "0.6.0",
            "apk_name": "saito-android.apk",
            "apk_url": "https://releases.saito.exessrhub.top/v0.6.0/saito-android.apk",
            "size": 1234u64,
            "sha256": SUM,
        })
        .to_string()
        .into_bytes();
        let mirror_ok = spawn_http_server(vec![("/latest/android.json".into(), 200, mirror_manifest)]);
        let gh_unused = spawn_http_server(vec![]);
        let releases = Releases::github("masuexe", "saito")
            .with_mirror(&mirror_ok)
            .with_github_bases(&gh_unused, &gh_unused);
        let found = rt.block_on(check_android_releases(&clients, &releases, "0.5.0")).unwrap().unwrap();
        assert_eq!(found.version, "0.6.0");
        assert_eq!(found.url, "https://releases.saito.exessrhub.top/v0.6.0/saito-android.apk");
        assert_eq!(
            found.fallback_url.as_deref(),
            Some(format!("{gh_unused}/masuexe/saito/releases/download/v0.6.0/saito-android.apk").as_str())
        );

        // 2. Mirror returns 503 -> automatically falls back to GitHub web_latest + release_by_tag.
        let mirror_down = spawn_http_server(vec![("/latest/android.json".into(), 503, b"unavailable".to_vec())]);
        let gh_web_latest = br#"{"tag_name":"v0.6.1"}"#.to_vec();
        let gh_tag_api = with_apk("v0.6.1");
        let gh_ok = spawn_http_server(vec![
            ("/masuexe/saito/releases/latest".into(), 200, gh_web_latest),
            ("/repos/masuexe/saito/releases/tags/v0.6.1".into(), 200, gh_tag_api),
        ]);
        let releases = Releases::github("masuexe", "saito")
            .with_mirror(&mirror_down)
            .with_github_bases(&gh_ok, &gh_ok);
        let found = rt.block_on(check_android_releases(&clients, &releases, "0.5.0")).unwrap().unwrap();
        assert_eq!(found.version, "0.6.1");
        assert_eq!(found.url, "https://github.com/masuexe/saito/releases/download/v0.6.1/saito-android.apk");
        assert_eq!(found.sha256, SUM);
    }
}
