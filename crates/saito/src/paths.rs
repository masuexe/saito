//! Where the app keeps what the player makes: their builds, the replays it
//! records, their settings and its log, each in the app's data folder
//! unless an environment variable names another.
//!
//! The data folder is `$SAITO_DATA` (or `$SAITO_DATA`), else the system's place
//! for an app's data (`saito_files::folders::app_data`):
//! `~/Library/Application Support/saito` on macOS,
//! `$XDG_DATA_HOME/saito` (`~/.local/share/saito`) on Linux and the other Unixes,
//! `%APPDATA%\saito` on Windows; on iOS the app's own `Library/Application Support/saito`,
//! on Android its files folder's `saito`; the working folder where none of those can be found.

use std::path::PathBuf;

/// Read `primary` env var first, falling back to legacy `secondary`.
fn env_var(primary: &str, secondary: &str) -> Option<PathBuf> {
    std::env::var_os(primary).or_else(|| std::env::var_os(secondary)).map(PathBuf::from)
}

/// The app's data folder.
pub fn data() -> PathBuf {
    // (The browser's are the page's files, under one folder the page keeps: packaging/web.)
    if cfg!(target_arch = "wasm32") {
        return PathBuf::from("/data");
    }
    if let Some(dir) = env_var("SAITO_DATA", "SAITO_DATA") {
        return dir;
    }
    saito_files::folders::app_data().map(|d| d.join("saito")).unwrap_or_default()
}

/// A folder of the data folder, unless `primary` or `secondary` names another.
fn folder(primary: &str, secondary: &str, name: &str) -> PathBuf {
    env_var(primary, secondary).unwrap_or_else(|| data().join(name))
}

/// Where the builds and the rulesets are kept instead, once [`keep_builds_in`] says.
static BUILDS: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);
static RULESETS: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// The builds, a folder per game: `$SAITO_BUILDS` (`$SAITO_BUILDS`), else `builds`.
pub fn builds() -> PathBuf {
    if let Some(dir) = BUILDS.lock().ok().and_then(|kept| kept.clone()) {
        return dir;
    }
    folder("SAITO_BUILDS", "SAITO_BUILDS", "builds")
}

/// Keep the builds in `dir` and the rulesets in `rulesets` from here on, whatever the environment says: the
/// screenshot pass makes and edits builds and rulesets of its own, and never the player's.
#[cfg_attr(not(feature = "tour"), allow(dead_code))]
pub fn keep_builds_in(dir: PathBuf, rulesets: PathBuf) {
    if let Ok(mut kept) = BUILDS.lock() {
        *kept = Some(dir);
    }
    if let Ok(mut kept) = RULESETS.lock() {
        *kept = Some(rulesets);
    }
}

/// The player's own rulesets, a folder per game: `$SAITO_RULESETS` (`$SAITO_RULESETS`), else `rulesets`.
pub fn rulesets() -> PathBuf {
    if let Some(dir) = RULESETS.lock().ok().and_then(|kept| kept.clone()) {
        return dir;
    }
    folder("SAITO_RULESETS", "SAITO_RULESETS", "rulesets")
}

/// The replays recorded and listed: `$SAITO_REPLAYS` (`$SAITO_REPLAYS`), else `replays`.
pub fn replays() -> PathBuf {
    folder("SAITO_REPLAYS", "SAITO_REPLAYS", "replays")
}

/// UI theme plugins (`.saitotheme` archives and theme folders): `$SAITO_THEMES`, else `themes`.
pub fn themes() -> PathBuf {
    folder("SAITO_THEMES", "SAITO_THEMES", "themes")
}

/// The log, which takes what goes wrong as the code says it (the player reads a plain sentence instead:
/// `crate::errors`): `$SAITO_LOG` (`$SAITO_LOG`), else `saito.log`.
pub fn log() -> PathBuf {
    env_var("SAITO_LOG", "SAITO_LOG").unwrap_or_else(|| data().join("saito.log"))
}

/// The settings' file: `$SAITO_SETTINGS` (`$SAITO_SETTINGS`), else `settings.toml`.
pub fn settings() -> PathBuf {
    env_var("SAITO_SETTINGS", "SAITO_SETTINGS").unwrap_or_else(|| data().join("settings.toml"))
}

/// Persistent ABD Mega/Giga/Dark/PA chip usage log (JSONL): `$SAITO_ABD_STATS`, else `abd-stats.jsonl`.
pub fn abd_stats() -> PathBuf {
    env_var("SAITO_ABD_STATS", "SAITO_ABD_STATS").unwrap_or_else(|| data().join("abd-stats.jsonl"))
}

/// Integrity-checked region verification cache (intentionally not env-overridable).
pub fn geo_cache() -> PathBuf {
    data().join("region.cache")
}
