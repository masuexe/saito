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
    /// Where the APK is downloaded from.
    pub url: String,
    /// The APK's size, in bytes (0 if unknown before GET).
    pub size: u64,
    /// The APK's SHA-256, as GitHub says it (in hexadecimal, lowercase).
    pub sha256: String,
}

impl Found {
    #[cfg_attr(not(android), allow(dead_code))]
    pub fn version(&self) -> &str {
        &self.version
    }
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
    Ok(Some(Found { version: version.to_owned(), url: apk.browser_download_url, size: apk.size, sha256 }))
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
        size: 0,
        sha256,
    })
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
                size: 0,
                sha256: sum.into(),
            }
        );
    }
}
