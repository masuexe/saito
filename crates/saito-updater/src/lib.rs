//! The app's own updates, from its GitHub releases (the app names them: [`Releases`]; docs/app.md §19, "Updates"),
//! where the app has an updater:
//! the desktop's installed app (a package Velopack made: packaging/; a build run from the repository isn't installed,
//! and asks nothing), and Android's (the release's APK). A browser's page is new each time it is loaded, and iOS's app
//! is its sideloading tool's: neither asks, and no update is found there.
//!
//! [`Updater::check`] asks the releases, once, whether a newer one is out; [`Updater::check_now`] triggers a fresh check
//! anytime an update is idle or settled; [`Updater::poll`] says where the update stands, on the app's thread (the work
//! is on threads of this crate's): asked ([`CHECK_WAIT`] at most: releases that don't answer by then are let go, and asked
//! again next start), the one found downloading, then being installed.
//!
//! Each system is a module (`src/platform.rs`). Unsafe code is Android's JNI alone (`src/platform/android.rs`).
#![deny(unsafe_code)]

#[cfg(any(android, test))]
mod android_release;
#[cfg(any(desktop, android))]
mod http;
mod platform;

use platform::system;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

/// Whether this platform supports in-app updates.
pub fn supported() -> bool {
    cfg!(any(desktop, android))
}

/// Environment variable overriding the optional release mirror base URL (e.g. for local testing).
/// Set to an empty string, `"none"`, or `"off"` to disable the mirror and force GitHub fallback.
pub const MIRROR_ENV: &str = "SAITO_UPDATE_MIRROR";

/// Normalize a mirror base URL: trim whitespace and strip trailing slashes.
/// Returns `None` if empty or explicitly disabled (`"none"` / `"off"`).
fn normalize_mirror(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") || trimmed.eq_ignore_ascii_case("off") {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// Where an app's releases are published: a GitHub repository whose releases carry the desktop's Velopack packages
/// (each system's and architecture's channel) and Android's APK, with an optional direct mirror checked first.
/// The app names its own ([`Updater::new`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Releases {
    owner: String,
    repo: String,
    mirror: Option<String>,
    #[cfg(test)]
    github_web_base: Option<String>,
    #[cfg(test)]
    github_api_base: Option<String>,
}

impl Releases {
    /// The releases of the GitHub repository `owner`/`repo`.
    pub fn github(owner: &str, repo: &str) -> Releases {
        Releases {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            mirror: None,
            #[cfg(test)]
            github_web_base: None,
            #[cfg(test)]
            github_api_base: None,
        }
    }

    /// Configure an optional HTTP(S) mirror base URL (e.g. Cloudflare R2 custom domain) to try before GitHub.
    pub fn with_mirror(mut self, base_url: impl AsRef<str>) -> Releases {
        self.mirror = normalize_mirror(base_url.as_ref());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_github_bases(mut self, web_base: &str, api_base: &str) -> Releases {
        self.github_web_base = Some(web_base.trim_end_matches('/').to_owned());
        self.github_api_base = Some(api_base.trim_end_matches('/').to_owned());
        self
    }

    /// Resolve the effective mirror base URL, applying an optional environment override.
    pub(crate) fn resolve_mirror(&self, env_override: Option<&str>) -> Option<String> {
        match env_override {
            Some(raw) => normalize_mirror(raw),
            None => self.mirror.clone(),
        }
    }

    /// Effective mirror base URL, honoring [`MIRROR_ENV`] when set.
    pub fn mirror_base(&self) -> Option<String> {
        self.resolve_mirror(std::env::var(MIRROR_ENV).ok().as_deref())
    }

    /// Mirror URL for an asset under `<mirror>/latest/<asset>`.
    #[cfg_attr(not(any(desktop, android, test)), allow(dead_code))]
    pub(crate) fn mirror_latest(&self, asset: &str) -> Option<String> {
        let base = self.mirror_base()?;
        Some(format!("{base}/latest/{}", asset.trim_start_matches('/')))
    }

    /// Mirror URL for the lightweight Android release manifest (`<mirror>/latest/android.json`).
    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn mirror_android_manifest(&self) -> Option<String> {
        self.mirror_latest("android.json")
    }

    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn owner(&self) -> &str {
        &self.owner
    }

    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn repo(&self) -> &str {
        &self.repo
    }

    #[cfg_attr(not(any(desktop, android, test)), allow(dead_code))]
    fn github_web(&self) -> &str {
        #[cfg(test)]
        if let Some(base) = &self.github_web_base {
            return base;
        }
        "https://github.com"
    }

    #[cfg_attr(not(any(desktop, android, test)), allow(dead_code))]
    fn github_api(&self) -> &str {
        #[cfg(test)]
        if let Some(base) = &self.github_api_base {
            return base;
        }
        "https://api.github.com"
    }

    /// GitHub's API for its newest published release.
    #[cfg_attr(not(any(desktop, android, test)), allow(dead_code))]
    pub(crate) fn latest(&self) -> String {
        format!("{}/repos/{}/{}/releases/latest", self.github_api(), self.owner, self.repo)
    }

    /// GitHub's web endpoint for its newest release (returns tiny JSON with `Accept: application/json`,
    /// avoiding `api.github.com`'s strict 60 req/hr unauthenticated IP rate limit).
    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn web_latest(&self) -> String {
        format!("{}/{}/{}/releases/latest", self.github_web(), self.owner, self.repo)
    }

    /// GitHub's API for a specific release tag.
    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn release_by_tag(&self, tag: &str) -> String {
        format!("{}/repos/{}/{}/releases/tags/{tag}", self.github_api(), self.owner, self.repo)
    }

    /// GitHub's web fragment listing a release's assets and SHA-256 digests without hitting `api.github.com`.
    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn expanded_assets(&self, tag: &str) -> String {
        format!("{}/{}/{}/releases/expanded_assets/{tag}", self.github_web(), self.owner, self.repo)
    }

    /// Direct download URL for an asset in the latest published release.
    #[cfg_attr(not(any(desktop, test)), allow(dead_code))]
    pub(crate) fn latest_download(&self, asset: &str) -> String {
        format!("{}/{}/{}/releases/latest/download/{asset}", self.github_web(), self.owner, self.repo)
    }

    /// Direct download URL for an asset in a specific release tag.
    #[cfg_attr(not(any(android, test)), allow(dead_code))]
    pub(crate) fn tag_download(&self, tag: &str, asset: &str) -> String {
        format!("{}/{}/{}/releases/download/{tag}/{asset}", self.github_web(), self.owner, self.repo)
    }
}

/// How long the releases are waited on for their answer.
pub const CHECK_WAIT: Duration = Duration::from_secs(20);

/// Run first thing in the app's `main`: on the desktop Velopack's hook, which, run by its installer or its updater,
/// does its part and ends the process there; elsewhere nothing.
pub fn startup() {
    system::startup();
}

/// Where the update stands ([`Updater::poll`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Nothing is going on: not asked (by the app, or on a system with no updater), or nothing more this session (no
    /// update, not installed, or it settled).
    Idle,
    /// The releases are asked whether a newer one is out.
    Checking,
    /// An explicit check completed with no newer release available (said once, [`Status::Idle`] after).
    UpToDate,
    /// An explicit check failed or timed out (logged): said once, [`Status::Idle`] after.
    CheckFailed,
    /// The update to `version` is downloading, this far (0 to 1).
    Downloading { version: String, progress: f32 },
    /// The update to `version` is downloaded and being installed: the app ends once it is (Android's installer may
    /// ask the player first).
    Installing { version: String },
    /// Downloading or installing the update failed, just now (logged): said once, [`Status::Idle`] after.
    Failed,
}

/// What a system's module tells, from its threads (or Java's).
#[cfg_attr(any(web, ios), allow(dead_code))]
pub(crate) enum Told {
    Found(system::Found),
    /// Nothing to check (unsupported or uninstalled build).
    None,
    /// Check succeeded and the app is already up-to-date.
    UpToDate,
    /// Checking the release feed failed (reason logged).
    CheckFailed(String),
    /// The download, this far (0 to 1).
    Progress(f32),
    Downloaded(Result<(), String>),
    /// The update wasn't installed (Android's installer said so: it ends the app when it is): the player turned it
    /// down (`declined`), or it failed.
    #[cfg_attr(not(android), allow(dead_code))]
    NotInstalled { why: String, declined: bool },
}

enum PollOutcome {
    None,
    UpToDate,
    CheckFailed,
    Failed,
}

/// The app's update: one an app, asked as it starts or manually from Settings.
pub struct Updater {
    releases: Releases,
    current_version: Option<String>,
    stage: Stage,
    explicit: bool,
    tell: Sender<Told>,
    heard: Receiver<Told>,
}

enum Stage {
    /// Not asked yet.
    Idle,
    /// The releases are asked, since then.
    Checking(Instant),
    /// The update found is downloading, this far.
    Downloading(Box<system::Found>, f32),
    /// The update to this version is being installed.
    Installing(String),
    /// Nothing more until checked again.
    Settled,
}

impl Updater {
    /// The updater of an app published as `releases`.
    pub fn new(releases: Releases) -> Updater {
        let (tell, heard) = mpsc::channel();
        Updater { releases, current_version: None, stage: Stage::Idle, explicit: false, tell, heard }
    }

    /// Provide the compiled-in app version as a fallback when platform version lookup fails (e.g., Android JNI).
    pub fn with_current_version(mut self, version: impl Into<String>) -> Updater {
        self.current_version = Some(version.into());
        self
    }

    /// Ask the releases whether a newer one is out: once, as the app starts. Where the system has no updater nothing
    /// is asked.
    pub fn check(&mut self) {
        if !matches!(self.stage, Stage::Idle) {
            return;
        }
        self.explicit = false;
        // (Only a system with an updater asks, and keeps the time: a browser's `Instant` has none.)
        self.stage = if system::check(&self.releases, self.current_version.as_deref(), self.tell.clone()) {
            Stage::Checking(Instant::now())
        } else {
            Stage::Settled
        };
    }

    /// Ask the releases right now whether a newer one is out, even if checked earlier this session.
    /// If a silent startup check is already in progress, promotes it to an explicit check so the caller
    /// receives its result. Returns `false` if this target has no updater or a download/install is active.
    pub fn check_now(&mut self) -> bool {
        if matches!(self.stage, Stage::Checking(_)) {
            self.explicit = true;
            return true;
        }
        if !matches!(self.stage, Stage::Idle | Stage::Settled) {
            return false;
        }
        self.explicit = true;
        if system::check(&self.releases, self.current_version.as_deref(), self.tell.clone()) {
            self.stage = Stage::Checking(Instant::now());
            true
        } else {
            self.explicit = false;
            self.stage = Stage::Settled;
            false
        }
    }

    /// Where the update stands, with what the system said since: an update found is downloaded, one downloaded
    /// installed.
    pub fn poll(&mut self) -> Status {
        let mut outcome = PollOutcome::None;
        while let Ok(told) = self.heard.try_recv() {
            match self.take(told) {
                PollOutcome::Failed => outcome = PollOutcome::Failed,
                PollOutcome::CheckFailed if !matches!(outcome, PollOutcome::Failed) => outcome = PollOutcome::CheckFailed,
                PollOutcome::UpToDate if matches!(outcome, PollOutcome::None) => outcome = PollOutcome::UpToDate,
                _ => {}
            }
        }
        if let Stage::Checking(since) = self.stage
            && since.elapsed() >= CHECK_WAIT
        {
            log::warn!("no answer about an update in {} seconds: asked again next start", CHECK_WAIT.as_secs());
            let explicit = std::mem::replace(&mut self.explicit, false);
            self.stage = Stage::Settled;
            if explicit && !matches!(outcome, PollOutcome::Failed) {
                outcome = PollOutcome::CheckFailed;
            }
        }
        match outcome {
            PollOutcome::Failed => return Status::Failed,
            PollOutcome::CheckFailed => return Status::CheckFailed,
            PollOutcome::UpToDate => return Status::UpToDate,
            PollOutcome::None => {}
        }
        match &self.stage {
            Stage::Checking(_) => Status::Checking,
            Stage::Downloading(found, progress) => {
                Status::Downloading { version: found.version().to_owned(), progress: *progress }
            }
            Stage::Installing(version) => Status::Installing { version: version.clone() },
            Stage::Idle | Stage::Settled => Status::Idle,
        }
    }

    /// What a system said, taken: one-shot status outcome (`UpToDate` or `Failed`) where applicable.
    fn take(&mut self, told: Told) -> PollOutcome {
        match told {
            Told::Found(found) => {
                if !matches!(self.stage, Stage::Checking(_)) {
                    log::info!("saito {} is out: it is downloaded next start", found.version());
                    return PollOutcome::None;
                }
                self.explicit = false;
                system::download(&self.releases, &found, self.tell.clone());
                self.stage = Stage::Downloading(Box::new(found), 0.0);
                PollOutcome::None
            }
            Told::None => {
                if matches!(self.stage, Stage::Checking(_)) {
                    self.explicit = false;
                    self.stage = Stage::Settled;
                }
                PollOutcome::None
            }
            Told::UpToDate => {
                if !matches!(self.stage, Stage::Checking(_)) {
                    return PollOutcome::None;
                }
                let explicit = std::mem::replace(&mut self.explicit, false);
                self.stage = Stage::Settled;
                if explicit { PollOutcome::UpToDate } else { PollOutcome::None }
            }
            Told::CheckFailed(why) => {
                log::warn!("checking for an update: {why}");
                if !matches!(self.stage, Stage::Checking(_)) {
                    return PollOutcome::None;
                }
                let explicit = std::mem::replace(&mut self.explicit, false);
                self.stage = Stage::Settled;
                if explicit { PollOutcome::CheckFailed } else { PollOutcome::None }
            }
            Told::Progress(progress) => {
                if let Stage::Downloading(_, at) = &mut self.stage {
                    *at = progress.clamp(0.0, 1.0);
                }
                PollOutcome::None
            }
            Told::Downloaded(done) => {
                let Stage::Downloading(found, _) = std::mem::replace(&mut self.stage, Stage::Settled) else {
                    return PollOutcome::None;
                };
                if let Err(why) = done {
                    log::warn!("downloading the update to saito {}: {why}", found.version());
                    return PollOutcome::Failed;
                }
                self.stage = Stage::Installing(found.version().to_owned());
                match system::install(&self.releases, *found, self.tell.clone()) {
                    Ok(()) => PollOutcome::None,
                    Err(why) => {
                        log::warn!("installing the update: {why}");
                        self.stage = Stage::Settled;
                        PollOutcome::Failed
                    }
                }
            }
            Told::NotInstalled { why, declined } => {
                let Stage::Installing(version) = std::mem::replace(&mut self.stage, Stage::Settled) else {
                    return PollOutcome::None;
                };
                if declined {
                    log::info!("the update to saito {version} was turned down: {why}");
                } else {
                    log::warn!("installing the update to saito {version}: {why}");
                }
                if !declined { PollOutcome::Failed } else { PollOutcome::None }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_check_failure_reports_check_failed_not_install_failed() {
        let mut updater = Updater::new(Releases::github("masuexe", "saito"));
        updater.stage = Stage::Checking(Instant::now());
        updater.explicit = true;
        updater.tell.send(Told::CheckFailed("network unreachable".into())).unwrap();
        assert_eq!(updater.poll(), Status::CheckFailed);
        assert_eq!(updater.poll(), Status::Idle);
    }

    #[test]
    fn silent_startup_check_failure_stays_idle() {
        let mut updater = Updater::new(Releases::github("masuexe", "saito"));
        updater.stage = Stage::Checking(Instant::now());
        updater.explicit = false;
        updater.tell.send(Told::CheckFailed("network unreachable".into())).unwrap();
        assert_eq!(updater.poll(), Status::Idle);
    }

    #[test]
    fn explicit_check_up_to_date_reports_once() {
        let mut updater = Updater::new(Releases::github("masuexe", "saito"));
        updater.stage = Stage::Checking(Instant::now());
        updater.explicit = true;
        updater.tell.send(Told::UpToDate).unwrap();
        assert_eq!(updater.poll(), Status::UpToDate);
        assert_eq!(updater.poll(), Status::Idle);
    }

    #[test]
    fn check_now_promotes_in_flight_silent_check() {
        let mut updater = Updater::new(Releases::github("masuexe", "saito"));
        updater.stage = Stage::Checking(Instant::now());
        updater.explicit = false;
        assert!(updater.check_now());
        updater.tell.send(Told::UpToDate).unwrap();
        assert_eq!(updater.poll(), Status::UpToDate);
    }

    #[test]
    fn mirror_urls_normalize_trailing_slashes_and_env_overrides() {
        let releases = Releases::github("masuexe", "saito").with_mirror(" https://releases.saito.exessrhub.top/// ");
        assert_eq!(releases.resolve_mirror(None).as_deref(), Some("https://releases.saito.exessrhub.top"));
        assert_eq!(
            releases.resolve_mirror(Some("http://127.0.0.1:8080/mirror/")).as_deref(),
            Some("http://127.0.0.1:8080/mirror")
        );
        assert_eq!(releases.resolve_mirror(Some("")), None);
        assert_eq!(releases.resolve_mirror(Some("none")), None);
        assert_eq!(releases.resolve_mirror(Some("OFF")), None);

        // When SAITO_UPDATE_MIRROR is not set in the test environment, mirror_latest / mirror_android_manifest use the configured mirror.
        if std::env::var(MIRROR_ENV).is_err() {
            assert_eq!(
                releases.mirror_latest("releases.win-x64.json").as_deref(),
                Some("https://releases.saito.exessrhub.top/latest/releases.win-x64.json")
            );
            assert_eq!(
                releases.mirror_latest("/saito-0.4.2-win-x64-full.nupkg").as_deref(),
                Some("https://releases.saito.exessrhub.top/latest/saito-0.4.2-win-x64-full.nupkg")
            );
            assert_eq!(
                releases.mirror_android_manifest().as_deref(),
                Some("https://releases.saito.exessrhub.top/latest/android.json")
            );
            assert_eq!(Releases::github("masuexe", "saito").mirror_android_manifest(), None);
        }
    }
}
