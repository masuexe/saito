//! The desktop's updates: Velopack's, for an app installed from one of its packages (packaging/), from the app's
//! GitHub releases ([`crate::Releases`]), on its system's and architecture's channel ([`options`]). A build run from
//! the repository isn't installed: its manager returns `NotInstalled`, and asks nothing.
//!
//! Instead of `velopack::sources::GithubSource` (which lists multiple releases via `api.github.com` and opens
//! separate `ureq` connections without system HTTP(S) proxy support), [`LatestGithubSource`] fetches
//! `releases.<channel>.json` and release packages directly via `https://github.com/{owner}/{repo}/releases/latest/download/...`
//! using `crate::http` (pooled `reqwest` + `rustls` + desktop system proxy with direct fallback), avoiding
//! `api.github.com`'s strict 60 req/hr unauthenticated IP rate limit and falling back to the GitHub API only if
//! direct download fails.

use crate::{Releases, Told};
use serde::Deserialize;
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::Mutex;
use velopack::sources::UpdateSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager, UpdateOptions, VelopackAsset, VelopackAssetFeed};

/// Max size of GitHub's `/releases/latest` JSON response and of `releases.<channel>.json`.
const MAX_RELEASE: u64 = 1 << 20;
const MAX_FEED: u64 = 4 << 20;

/// An update found: Velopack's.
pub struct Found(UpdateInfo);

impl Found {
    pub fn version(&self) -> &str {
        &self.0.TargetFullRelease.Version
    }
}

/// Velopack's own start, first thing in `main`: run as an installer's or an updater's hook, it does its part and the
/// process ends there; otherwise it does nothing.
pub fn startup() {
    velopack::VelopackApp::build().run();
}

#[derive(Deserialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Parse GitHub `/releases/latest` JSON into a case-insensitive map of `asset_name -> browser_download_url`.
pub(crate) fn parse_latest_release_assets(json: &[u8]) -> Result<HashMap<String, String>, String> {
    let release: GithubRelease = serde_json::from_slice(json).map_err(|e| format!("the release's JSON: {e}"))?;
    Ok(release.assets.into_iter().map(|a| (a.name.to_ascii_lowercase(), a.browser_download_url)).collect())
}

/// Fast single-release GitHub source backed by `crate::http` (pooled HTTPS + system proxy + direct fallback).
struct LatestGithubSource {
    releases: Releases,
    assets: Mutex<HashMap<String, String>>,
}

impl LatestGithubSource {
    fn new(releases: Releases) -> LatestGithubSource {
        LatestGithubSource {
            releases,
            assets: Mutex::new(HashMap::new()),
        }
    }

    fn with_runtime<T>(work: impl Future<Output = Result<T, String>>) -> Result<T, velopack::Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| velopack::Error::Other(format!("runtime: {e}")))?;
        runtime.block_on(work).map_err(velopack::Error::Other)
    }

    async fn fetch_assets(&self, clients: &crate::http::Clients) -> Result<HashMap<String, String>, String> {
        let json = clients.get_bytes(&self.releases.latest(), MAX_RELEASE).await?;
        let map = parse_latest_release_assets(&json)?;
        *self.assets.lock().unwrap_or_else(|e| e.into_inner()) = map.clone();
        Ok(map)
    }
}

impl UpdateSource for LatestGithubSource {
    fn get_release_feed(&self, channel: &str, _app: &velopack::bundle::Manifest, _staged_user_id: &str) -> Result<VelopackAssetFeed, velopack::Error> {
        let feed_name = format!("releases.{channel}.json").to_ascii_lowercase();
        Self::with_runtime(async {
            let clients = crate::http::Clients::new()?;
            // Prefer GitHub's direct `/releases/latest/download/<feed>` URL: single hop, no `api.github.com`
            // 60 req/hr unauthenticated rate limit (common on shared VPN/proxy nodes).
            let direct_url = self.releases.latest_download(&feed_name);
            let bytes = match clients.get_bytes(&direct_url, MAX_FEED).await {
                Ok(bytes) => bytes,
                Err(direct_err) => {
                    log::warn!("direct release feed download failed ({direct_err}); falling back to GitHub API");
                    let assets = self
                        .fetch_assets(&clients)
                        .await
                        .map_err(|api_err| format!("{direct_err} (api: {api_err})"))?;
                    let Some(url) = assets.get(&feed_name) else {
                        return Ok(VelopackAssetFeed { Assets: Vec::new() });
                    };
                    clients.get_bytes(url, MAX_FEED).await?
                }
            };
            serde_json::from_slice(&bytes).map_err(|e| format!("parsing {feed_name}: {e}"))
        })
    }

    fn download_release_entry(&self, asset: &VelopackAsset, local_file: &Path, progress_sender: Option<Sender<i16>>) -> Result<(), velopack::Error> {
        let key = asset.FileName.to_ascii_lowercase();
        let size = asset.Size;
        Self::with_runtime(async {
            let clients = crate::http::Clients::new()?;
            let primary_url = self
                .assets
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&key)
                .cloned()
                .unwrap_or_else(|| self.releases.latest_download(&asset.FileName));
            let send_pct = |pct| {
                if let Some(tx) = &progress_sender {
                    let _ = tx.send(pct);
                }
            };
            match clients.download_to_file(&primary_url, local_file, size, send_pct).await {
                Ok(()) => Ok(()),
                Err(direct_err) => {
                    log::warn!("direct release asset download failed ({direct_err}); falling back to GitHub API");
                    let assets = self
                        .fetch_assets(&clients)
                        .await
                        .map_err(|api_err| format!("{direct_err} (api: {api_err})"))?;
                    let url = assets
                        .get(&key)
                        .cloned()
                        .ok_or_else(|| format!("release asset {} not found in latest release", asset.FileName))?;
                    if url == primary_url {
                        return Err(direct_err);
                    }
                    clients.download_to_file(&url, local_file, size, send_pct).await
                }
            }
        })
    }
}

/// The installed app's update manager, of `releases`.
fn manager(releases: &Releases) -> Result<UpdateManager, velopack::Error> {
    let options = options(std::env::consts::OS, std::env::consts::ARCH);
    UpdateManager::new(LatestGithubSource::new(releases.clone()), Some(options), None)
}

/// The channel the app asks for its updates on, whatever channel its package was made on (`vpk pack --channel`,
/// which Velopack's manager asks otherwise): the system's and the architecture's, as the program was built for them
/// (`macos-arm64`, `macos-x64`, `win-x64`, `win-arm64`, `linux-x64`, `linux-arm64`: packaging/), so that an app from
/// a package on a channel before them takes its own at its next update: the channels up to 0.19.0, a Mac's
/// `osx-arm64` and `osx-x64`, Windows' `win` and Linux's `linux` (x86_64's alone), made again in 0.19.1 as the bridge;
/// and a Mac's universal `osx` (its only one up to 0.12.0, made up to 0.18.1), the slice it runs choosing
/// (docs/app.md §19, "Updates").
fn options(os: &str, arch: &str) -> UpdateOptions {
    let channel = match (os, arch) {
        ("macos", "aarch64") => Some("macos-arm64"),
        ("macos", "x86_64") => Some("macos-x64"),
        ("windows", "x86_64") => Some("win-x64"),
        ("windows", "aarch64") => Some("win-arm64"),
        ("linux", "x86_64") => Some("linux-x64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        _ => None,
    };
    UpdateOptions { ExplicitChannel: channel.map(str::to_owned), ..UpdateOptions::default() }
}

/// Ask the releases on a thread (always: whether the app is installed is the thread's to find out).
pub fn check(releases: &Releases, _fallback_version: Option<&str>, tell: Sender<Told>) -> bool {
    let releases = releases.clone();
    std::thread::spawn(move || {
        let told = match manager(&releases) {
            Err(velopack::Error::NotInstalled(_)) => Told::None,
            Err(why) => Told::CheckFailed(why.to_string()),
            Ok(manager) => match manager.check_for_updates() {
                Ok(UpdateCheck::UpdateAvailable(info)) => Told::Found(Found(*info)),
                Ok(_) => Told::UpToDate,
                Err(why) => Told::CheckFailed(why.to_string()),
            },
        };
        let _ = tell.send(told);
    });
    true
}

/// Download `found`'s update on a thread, telling how far.
pub fn download(releases: &Releases, found: &Found, tell: Sender<Told>) {
    let (releases, info) = (releases.clone(), found.0.clone());
    std::thread::spawn(move || {
        let manager = match manager(&releases) {
            Ok(m) => m,
            Err(why) => return drop(tell.send(Told::Downloaded(Err(why.to_string())))),
        };
        let (progress, heard) = mpsc::channel::<i16>();
        let forward = {
            let tell = tell.clone();
            std::thread::spawn(move || {
                for percent in heard {
                    let _ = tell.send(Told::Progress(f32::from(percent.clamp(0, 100)) / 100.0));
                }
            })
        };
        let done = manager.download_updates(&info, Some(progress)).map_err(|why| why.to_string());
        let _ = forward.join();
        let _ = tell.send(Told::Downloaded(done));
    });
}

/// The updater waits for this process to end, then puts the update in place silently (no window of its own) and
/// starts the app again. The process ends here (the app writes its settings as each changes).
pub fn install(releases: &Releases, found: Found, _tell: Sender<Told>) -> Result<(), String> {
    let manager = manager(releases).map_err(|why| why.to_string())?;
    match manager.wait_exit_then_apply_updates(&found.0.TargetFullRelease, true, true, Vec::<String>::new()) {
        Ok(()) => std::process::exit(0),
        Err(why) => Err(why.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{options, parse_latest_release_assets};
    use velopack::locator::VelopackLocatorConfig;
    use velopack::sources::FileSource;
    use velopack::{UpdateCheck, UpdateManager};

    #[test]
    fn parses_latest_release_assets_case_insensitively() {
        let json = br#"{
            "tag_name": "v0.2.0",
            "assets": [
                {
                    "name": "releases.win-x64.json",
                    "browser_download_url": "https://example.invalid/releases.win-x64.json"
                },
                {
                    "name": "Saito-0.2.0-win-x64-full.nupkg",
                    "browser_download_url": "https://example.invalid/saito-0.2.0-win-x64-full.nupkg"
                }
            ]
        }"#;
        let map = parse_latest_release_assets(json).unwrap();
        assert_eq!(map.get("releases.win-x64.json").map(String::as_str), Some("https://example.invalid/releases.win-x64.json"));
        assert_eq!(
            map.get("saito-0.2.0-win-x64-full.nupkg").map(String::as_str),
            Some("https://example.invalid/saito-0.2.0-win-x64-full.nupkg")
        );
    }

    /// The full package an installed app of `version`, packaged on `channel` and running on `os`/`arch`, updates
    /// to from a release's feeds (`releases.<channel>.json`, each channel's versions), if any: Velopack's own check,
    /// with the app's options.
    fn update_to(version: &str, channel: &str, (os, arch): (&str, &str), feeds: &[(&str, &[&str])]) -> Option<String> {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("saito.app");
        let programs = app.join("Contents/MacOS");
        let (packages, release) = (dir.path().join("packages"), dir.path().join("release"));
        for folder in [&programs, &packages, &release] {
            std::fs::create_dir_all(folder).unwrap();
        }
        // (What `vpk pack` puts beside the program, where Velopack's locator reads them: the manifest and the updater.)
        let manifest = programs.join("sq.version");
        std::fs::write(
            &manifest,
            format!(
                r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2010/07/nuspec.xsd"><metadata>
<id>saito</id><version>{version}</version><channel>{channel}</channel><mainExe>saito</mainExe>
</metadata></package>"#
            ),
        )
        .unwrap();
        let updater = programs.join("UpdateMac");
        std::fs::write(&updater, []).unwrap();
        for (channel, versions) in feeds {
            let assets: Vec<_> = versions
                .iter()
                .map(|version| {
                    serde_json::json!({
                        "PackageId": "saito",
                        "Version": version,
                        "Type": "Full",
                        "FileName": format!("saito-{version}-{channel}-full.nupkg"),
                    })
                })
                .collect();
            let feed = serde_json::json!({ "Assets": assets }).to_string();
            std::fs::write(release.join(format!("releases.{channel}.json")), feed).unwrap();
        }
        let locator = VelopackLocatorConfig {
            RootAppDir: app,
            UpdateExePath: updater,
            PackagesDir: packages,
            ManifestPath: manifest,
            CurrentBinaryDir: programs,
            IsPortable: true,
        };
        let manager = UpdateManager::new(FileSource::new(&release), Some(options(os, arch)), Some(locator)).unwrap();
        match manager.check_for_updates().unwrap() {
            UpdateCheck::UpdateAvailable(info) => Some(info.TargetFullRelease.FileName),
            UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => None,
        }
    }

    /// The full package of a version on a channel, named as the feeds above name it.
    fn full(version: &str, channel: &str) -> Option<String> {
        Some(format!("saito-{version}-{channel}-full.nupkg"))
    }

    const APPLE_SILICON: (&str, &str) = ("macos", "aarch64");
    const INTEL_MAC: (&str, &str) = ("macos", "x86_64");
    const WINDOWS_X64: (&str, &str) = ("windows", "x86_64");
    const WINDOWS_ARM64: (&str, &str) = ("windows", "aarch64");
    const LINUX_X64: (&str, &str) = ("linux", "x86_64");
    const LINUX_ARM64: (&str, &str) = ("linux", "aarch64");

    /// Each system and architecture's channel, and an app's system.
    const CHANNELS: [(&str, (&str, &str)); 6] = [
        ("macos-arm64", APPLE_SILICON),
        ("macos-x64", INTEL_MAC),
        ("win-x64", WINDOWS_X64),
        ("win-arm64", WINDOWS_ARM64),
        ("linux-x64", LINUX_X64),
        ("linux-arm64", LINUX_ARM64),
    ];

    /// The channels before, with an app's system and the channel it is to take: the bridge's (up to 0.19.0 the only
    /// ones, made again in 0.19.1), and the Mac's universal one (made up to 0.18.1), the slice it runs choosing. An app
    /// installed from a package made on one asks its architecture's channel, where the releases since are alone.
    const BEFORE: [(&str, (&str, &str), &str); 6] = [
        ("osx-arm64", APPLE_SILICON, "macos-arm64"),
        ("osx-x64", INTEL_MAC, "macos-x64"),
        ("win", WINDOWS_X64, "win-x64"),
        ("linux", LINUX_X64, "linux-x64"),
        ("osx", APPLE_SILICON, "macos-arm64"),
        ("osx", INTEL_MAC, "macos-x64"),
    ];

    /// A release of `versions` on every architecture's channel, and on none before.
    fn release(versions: &'static [&'static str]) -> Vec<(&'static str, &'static [&'static str])> {
        CHANNELS.iter().map(|(channel, _)| (*channel, versions)).collect()
    }

    #[test]
    fn an_app_on_a_channel_before_takes_its_architectures_package() {
        for (before, system, channel) in BEFORE {
            let update = update_to("0.19.1", before, system, &release(&["0.20.0"]));
            assert_eq!(update, full("0.20.0", channel), "{before}, {system:?}");
        }
    }

    #[test]
    fn an_app_on_a_channel_before_waits_for_a_newer_version() {
        // (The same version on its architecture's channel is no update: an app the bridge updated moves at the next
        // one, whichever channel its package was made on.)
        for (before, system, _) in BEFORE {
            assert_eq!(update_to("0.20.0", before, system, &release(&["0.20.0"])), None, "{before}, {system:?}");
        }
    }

    #[test]
    fn an_architectures_app_stays_on_its_channel() {
        for (channel, system) in CHANNELS {
            let update = update_to("0.20.0", channel, system, &release(&["0.21.0"]));
            assert_eq!(update, full("0.21.0", channel), "{channel}");
        }
    }
}
