//! The player's settings, kept between runs: a TOML file in the app's data folder (`crate::paths::settings`),
//! written each time one changes and read as the app starts. A setting the file leaves out (or one it can't
//! read) is the app's own default.
//!
//! A screen that remembers something of its own (the builds Practice last used, the rounds last played) keeps
//! it by a key in [`Settings::remembered`], so it adds no field here.

use crate::input::{BUTTONS, Bindings, bind_name, bind_named, pad_name, pad_named};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The most a name is, as the lobby carries it.
pub const NAME_LENGTH: usize = saito_frontend::lobby::NAME_LENGTH;

/// The signaling server used when neither `$SAITO_SIGNAL` nor `Settings::signaling_server` is set.
pub const DEFAULT_SIGNAL_SERVER: &str = "wss://matchmaking.exessrhub.top";

/// The longest signaling server URL kept in settings.
pub const SIGNAL_SERVER_LENGTH: usize = 256;

/// Minimum and maximum scale multipliers for customizable virtual touch pad controls.
pub const TOUCH_SCALE_MIN: f32 = 0.6;
pub const TOUCH_SCALE_MAX: f32 = 1.6;

/// Minimum and maximum sensitivity gain for EXE5DS Danger microphone input.
pub const MIC_SENSITIVITY_MIN: f32 = saito_audio::mic::SENSITIVITY_MIN;
pub const MIC_SENSITIVITY_MAX: f32 = saito_audio::mic::SENSITIVITY_MAX;

/// Position offset (fraction of pad container) and size multiplier for one virtual touch pad control.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchControlLayout {
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
}

impl Default for TouchControlLayout {
    fn default() -> Self {
        Self { dx: 0.0, dy: 0.0, scale: 1.0 }
    }
}

impl TouchControlLayout {
    pub fn sanitized(self) -> Self {
        let dx = if self.dx.is_finite() { self.dx.clamp(-1.0, 1.0) } else { 0.0 };
        let dy = if self.dy.is_finite() { self.dy.clamp(-1.0, 1.0) } else { 0.0 };
        let scale = if self.scale.is_finite() { self.scale.clamp(TOUCH_SCALE_MIN, TOUCH_SCALE_MAX) } else { 1.0 };
        Self { dx, dy, scale }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Customizable virtual touch pad controls for one screen orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchPadLayout {
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub dpad: TouchControlLayout,
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub a: TouchControlLayout,
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub b: TouchControlLayout,
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub l: TouchControlLayout,
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub r: TouchControlLayout,
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub select: TouchControlLayout,
    #[serde(skip_serializing_if = "TouchControlLayout::is_default")]
    pub start: TouchControlLayout,
}

impl TouchPadLayout {
    pub fn get_mut(&mut self, index: i32) -> Option<&mut TouchControlLayout> {
        Some(match index {
            0 => &mut self.dpad,
            1 => &mut self.a,
            2 => &mut self.b,
            3 => &mut self.l,
            4 => &mut self.r,
            5 => &mut self.select,
            6 => &mut self.start,
            _ => return None,
        })
    }

    pub fn sanitized(self) -> Self {
        Self {
            dpad: self.dpad.sanitized(),
            a: self.a.sanitized(),
            b: self.b.sanitized(),
            l: self.l.sanitized(),
            r: self.r.sanitized(),
            select: self.select.sanitized(),
            start: self.start.sanitized(),
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// Portrait and landscape virtual touch pad layouts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchPadLayouts {
    #[serde(skip_serializing_if = "TouchPadLayout::is_default")]
    pub portrait: TouchPadLayout,
    #[serde(skip_serializing_if = "TouchPadLayout::is_default")]
    pub landscape: TouchPadLayout,
}

impl TouchPadLayouts {
    pub fn for_orientation_mut(&mut self, landscape: bool) -> &mut TouchPadLayout {
        if landscape { &mut self.landscape } else { &mut self.portrait }
    }

    pub fn sanitized(self) -> Self {
        Self { portrait: self.portrait.sanitized(), landscape: self.landscape.sanitized() }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// EXE3 HUD & Custom Screen version policy (`Settings::exe3_hud_style`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exe3HudStylePref {
    /// Derive White vs Black from the build's exclusive Giga chips or Style.
    #[default]
    Auto,
    /// Always use EXE3 White's HUD and Custom Screen.
    White,
    /// Always use EXE3 Black's green pointed banners and monochrome Custom Screen.
    Black,
}

impl Exe3HudStylePref {
    pub fn as_index(self) -> usize {
        match self {
            Self::Auto => 0,
            Self::White => 1,
            Self::Black => 2,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index.rem_euclid(3) {
            1 => Self::White,
            2 => Self::Black,
            _ => Self::Auto,
        }
    }

    pub fn is_auto(&self) -> bool {
        *self == Self::Auto
    }

    pub fn to_render(self) -> saito_frontend::game::Exe3HudStyle {
        match self {
            Self::Auto => saito_frontend::game::Exe3HudStyle::Auto,
            Self::White => saito_frontend::game::Exe3HudStyle::White,
            Self::Black => saito_frontend::game::Exe3HudStyle::Black,
        }
    }

    pub fn from_render(style: saito_frontend::game::Exe3HudStyle) -> Self {
        match style {
            saito_frontend::game::Exe3HudStyle::Auto => Self::Auto,
            saito_frontend::game::Exe3HudStyle::White => Self::White,
            saito_frontend::game::Exe3HudStyle::Black => Self::Black,
        }
    }
}

/// Battle favorite BGM rotation mode (`Settings::favorite_battle_bgm`, `S-135`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavoriteBgmMode {
    /// Do not override battle BGM with favorite tracks.
    #[default]
    Off,
    /// Pick a random favorite track each round (avoiding consecutive repeats when possible).
    Random,
    /// Step through favorite tracks in order, advancing each round.
    Sequential,
}

impl FavoriteBgmMode {
    pub const COUNT: usize = 3;

    pub fn as_index(self) -> usize {
        match self {
            Self::Off => 0,
            Self::Random => 1,
            Self::Sequential => 2,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index.rem_euclid(Self::COUNT as i32) {
            1 => Self::Random,
            2 => Self::Sequential,
            _ => Self::Off,
        }
    }

    pub fn is_off(&self) -> bool {
        *self == Self::Off
    }
}

/// Mobile screen orientation preference (`Settings::screen_orientation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenOrientation {
    /// Lock upright portrait everywhere (default).
    #[default]
    Portrait,
    /// Lock landscape everywhere (allows 180-degree sensor flip).
    Landscape,
    /// Portrait in menus/lobby/builds; automatically landscape in battles, spectating, and replays.
    BattleLandscape,
    /// Follow the device's gravity / system auto-rotate setting.
    Auto,
}

impl ScreenOrientation {
    pub const COUNT: usize = 4;

    pub fn as_index(self) -> usize {
        match self {
            Self::Portrait => 0,
            Self::Landscape => 1,
            Self::BattleLandscape => 2,
            Self::Auto => 3,
        }
    }

    pub fn from_index(index: i32) -> Self {
        match index.rem_euclid(Self::COUNT as i32) {
            1 => Self::Landscape,
            2 => Self::BattleLandscape,
            3 => Self::Auto,
            _ => Self::Portrait,
        }
    }
}

/// Effective orientation lock sent to the mobile OS.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrientationLock {
    /// Follow system auto-rotate / gravity (`SCREEN_ORIENTATION_UNSPECIFIED`).
    Auto,
    /// Lock upright portrait (`SCREEN_ORIENTATION_PORTRAIT`).
    Portrait,
    /// Lock landscape with 180-degree sensor flip (`SCREEN_ORIENTATION_SENSOR_LANDSCAPE`).
    Landscape,
}

impl OrientationLock {
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub fn as_android_mode(self) -> i32 {
        match self {
            Self::Auto => 0,
            Self::Portrait => 1,
            Self::Landscape => 2,
        }
    }
}

/// Resolve the active OS orientation lock from the saved preference, whether a full-screen battle/viewer is
/// active, and any temporary manual override (editor preview or in-battle toggle).
pub fn resolve_orientation(pref: ScreenOrientation, in_battle: bool, override_landscape: Option<bool>) -> OrientationLock {
    if let Some(landscape) = override_landscape {
        return if landscape { OrientationLock::Landscape } else { OrientationLock::Portrait };
    }
    match pref {
        ScreenOrientation::Portrait => OrientationLock::Portrait,
        ScreenOrientation::Landscape => OrientationLock::Landscape,
        ScreenOrientation::BattleLandscape => {
            if in_battle {
                OrientationLock::Landscape
            } else {
                OrientationLock::Portrait
            }
        }
        ScreenOrientation::Auto => OrientationLock::Auto,
    }
}

/// What the file keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The player's name, shown to their opponent.
    pub name: String,
    /// The language, by its code (`ja`); none: the system's.
    pub language: Option<String>,
    /// Active UI theme ID (`pet-hd`, `terminal`, or a user `.saitotheme` ID).
    pub ui_theme: String,
    /// The game last used: the one the app opens on.
    pub game: Option<String>,
    /// The most the battle's picture is, in multiples of 240 by 160; 0: the largest that fits.
    pub picture_size: u32,
    pub full_screen: bool,
    /// On-screen virtual touch pad shown over the battle in mobile landscape.
    pub landscape_touch_pad: bool,
    /// Hide Android's top status bar in landscape orientation.
    pub hide_landscape_status_bar: bool,
    /// Mobile screen orientation preference (portrait, landscape, battle-only landscape, or follow gravity).
    pub screen_orientation: ScreenOrientation,
    /// Pure keyboard mode on the custom screen: disables mouse/touch chip selection.
    pub keyboard_only_custom: bool,
    /// Motion turned down: everything is a cut. None: as the system asks.
    pub reduce_motion: Option<bool>,
    /// An installed app asks saito's GitHub releases at start whether a newer one is out (saito-updater).
    pub check_updates: bool,
    /// The graphics enhancements battles are drawn with (`Enhancements`), each on or off; all off, as the game
    /// draws them. Sharp text: the strings in a font at the window's resolution.
    pub sharp_text: bool,
    /// Sharp HUD: the HUD's and the custom screen's frames, words and numbers redrawn at the window's resolution.
    pub sharp_hud: bool,
    /// Smooth motion: what the game moves in whole pixels drawn between them (the background's glide, the gauge's
    /// fill, the banners' squash, the custom screen's slide).
    pub smooth_motion: bool,
    /// Fill the window: the picture as wide and as tall as the window at its multiple, the HUD to its edges.
    pub fill_window: bool,
    /// Soften sudden full-screen white flashes in battle (photosensitive accessibility).
    pub reduce_flashes: bool,
    /// Hide the "200" number on LifeAura barriers (EXE4–5).
    pub hide_aura_number: bool,
    /// Sync EXE5 Dark/Light MegaMan's buster arm palette with his body palette (Normal, Sol Cross, Bass Cross).
    pub sync_dark_buster_arm: bool,
    /// EXE3 HUD & Custom Screen version policy (`auto` from build, always `white`, or always `black`).
    #[serde(skip_serializing_if = "Exe3HudStylePref::is_auto")]
    pub exe3_hud_style: Exe3HudStylePref,
    /// Use Legacy Collection high-resolution chip artwork when available.
    pub hd_chip_art: bool,
    /// 0 to 100.
    pub music: u32,
    pub effects: u32,
    /// Ordered list of favorited looping tracks (`"<game>/<song>"`, `S-135`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub favorite_music: Vec<String>,
    /// Auto-play favorite tracks in battle, switching each round (`S-135`).
    #[serde(skip_serializing_if = "FavoriteBgmMode::is_off")]
    pub favorite_battle_bgm: FavoriteBgmMode,
    /// Optional custom main-menu BGM (`"<game>/<song>"`, looping only; `None` = selected game's title theme, `S-135`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_bgm: Option<String>,
    /// Netplay's input delay, in frames: the value the lobby starts with (the library's
    /// `NetOptions::input_delay`).
    pub input_delay: u32,
    /// When true, automatically calculate netplay's input delay from the measured ping and jitter
    /// when a match starts (locked for the match; manual override still works in-match).
    pub auto_input_delay: bool,
    /// Custom signaling server URL (`ws://...` or `wss://...`); empty uses `DEFAULT_SIGNAL_SERVER`.
    pub signaling_server: String,
    /// Automatically upload completed online match replays to the signaling server's recent-replays feed.
    /// Off by default (opt-in).
    pub auto_upload_replays: bool,
    /// The folder the player chose the Legacy Collection's install in, where Steam doesn't list it (docs/app.md
    /// §17); none: Steam's alone.
    pub legacy_collection: Option<std::path::PathBuf>,
    /// Optional path to an EXE5DS (`.nds`) ROM for DS-authentic SolCross, Danger HUD artwork & battle music.
    pub exe5ds_rom: Option<std::path::PathBuf>,
    /// Whether EXE5 default / background / escalating battles use arranged EXE5DS battle music when a DS ROM is loaded.
    pub exe5ds_music: bool,
    /// Whether EXE5 battles play EXE5DS battle voices (Lan, MegaMan, ProtoMan, Colonel) when a DS ROM is loaded.
    pub exe5ds_voices: bool,
    /// Opt-in privacy-first EXE5DS Danger microphone shout detection (default `false`).
    pub mic_shout_enabled: bool,
    /// Microphone sensitivity multiplier for EXE5DS Crisis Shout (`0.25..=4.0`, default `1.0`).
    pub mic_sensitivity: f32,
    /// Simulated shout volume tier sent by Quick-Shout key/button in Real 3-Tier mode (`1` = Low, `2` = Medium, `3` = High).
    pub quick_shout_tier: u8,
    /// The keys and the pad buttons bound, by the GBA button's name; one left out is the default.
    pub keys: BTreeMap<String, String>,
    pub pad: BTreeMap<String, String>,
    /// What screens remember, by their own keys (`play.build.<game>`).
    pub remembered: BTreeMap<String, String>,
    /// Custom virtual touch pad button offsets and sizes, separately for portrait and landscape.
    #[serde(skip_serializing_if = "TouchPadLayouts::is_default")]
    pub touch_pad_layout: TouchPadLayouts,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            name: system_name(),
            language: None,
            ui_theme: crate::themes::DEFAULT_THEME.into(),
            game: None,
            picture_size: 0,
            full_screen: false,
            landscape_touch_pad: true,
            hide_landscape_status_bar: false,
            screen_orientation: ScreenOrientation::default(),
            keyboard_only_custom: false,
            reduce_motion: None,
            check_updates: true,
            sharp_text: true,
            sharp_hud: true,
            smooth_motion: true,
            fill_window: true,
            reduce_flashes: false,
            hide_aura_number: false,
            sync_dark_buster_arm: false,
            exe3_hud_style: Exe3HudStylePref::Auto,
            hd_chip_art: false,
            music: 80,
            effects: 100,
            favorite_music: Vec::new(),
            favorite_battle_bgm: FavoriteBgmMode::Off,
            menu_bgm: None,
            input_delay: saito_frontend::netplay::DEFAULT_INPUT_DELAY,
            auto_input_delay: false,
            signaling_server: String::new(),
            auto_upload_replays: false,
            keys: BTreeMap::new(),
            legacy_collection: None,
            exe5ds_rom: None,
            exe5ds_music: true,
            exe5ds_voices: true,
            mic_shout_enabled: false,
            mic_sensitivity: 1.0,
            quick_shout_tier: 3,
            pad: BTreeMap::new(),
            remembered: BTreeMap::new(),
            touch_pad_layout: TouchPadLayouts::default(),
        }
    }
}

impl Settings {
    /// The graphics enhancements battles are drawn with.
    pub fn enhancements(&self) -> saito_frontend::game::Enhancements {
        saito_frontend::game::Enhancements {
            text: self.sharp_text,
            hud: self.sharp_hud,
            motion: self.smooth_motion,
            fill: self.fill_window,
            reduce_flashes: self.reduce_flashes,
            hide_aura_number: self.hide_aura_number,
            sync_dark_buster_arm: self.sync_dark_buster_arm,
            hd_chip_art: self.hd_chip_art,
            exe3_hud_style: self.exe3_hud_style.to_render(),
        }
    }

    pub fn set_enhancements(&mut self, e: saito_frontend::game::Enhancements) {
        (self.sharp_text, self.sharp_hud, self.smooth_motion, self.fill_window, self.reduce_flashes, self.hide_aura_number, self.sync_dark_buster_arm, self.hd_chip_art) =
            (e.text, e.hud, e.motion, e.fill, e.reduce_flashes, e.hide_aura_number, e.sync_dark_buster_arm, e.hd_chip_art);
        self.exe3_hud_style = Exe3HudStylePref::from_render(e.exe3_hud_style);
    }

    /// Step netplay's input delay across `Auto` (`-1`) and fixed `0..=MAX_INPUT_DELAY`.
    pub fn step_input_delay(&mut self, by: i32) {
        let cur = if self.auto_input_delay { -1 } else { self.input_delay as i32 };
        let next = (cur + by).clamp(-1, MAX_INPUT_DELAY as i32);
        if next < 0 {
            self.auto_input_delay = true;
        } else {
            self.auto_input_delay = false;
            self.input_delay = next as u32;
        }
    }

    /// Resolve the concrete input delay (in ticks) to apply to a netplay match from the latest
    /// measured ping. Falls back to `DEFAULT_INPUT_DELAY` when `auto_input_delay` is enabled but
    /// no ping sample has been measured yet.
    pub fn resolved_input_delay(&self, ping: Option<saito_frontend::connection::Ping>) -> u32 {
        if self.auto_input_delay {
            ping.map_or(saito_frontend::netplay::DEFAULT_INPUT_DELAY, saito_frontend::connection::Ping::suggested_input_delay)
        } else {
            self.input_delay
        }
    }

    /// Whether canonical track key `"<game>/<song>"` is currently in `favorite_music`.
    pub fn is_favorite_music(&self, key: &str) -> bool {
        self.favorite_music.iter().any(|k| k == key)
    }

    /// Toggle a looping track key in `favorite_music`; returns the new favorited state.
    pub fn toggle_favorite_music(&mut self, key: &str) -> bool {
        if let Some(pos) = self.favorite_music.iter().position(|k| k == key) {
            self.favorite_music.remove(pos);
            false
        } else if crate::music::is_eligible_loop_key(key) {
            self.favorite_music.push(key.to_string());
            true
        } else {
            false
        }
    }

    /// Sanitize music favorites and menu BGM (deduplicate, drop malformed or known non-looping keys).
    pub fn sanitize_music(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.favorite_music.retain(|k| crate::music::is_eligible_loop_key(k) && seen.insert(k.clone()));
        if self.menu_bgm.as_deref().is_some_and(|k| !crate::music::is_eligible_loop_key(k)) {
            self.menu_bgm = None;
        }
    }
}

/// The most the input delay can be set to: the library's.
pub const MAX_INPUT_DELAY: u32 = *saito_frontend::netplay::INPUT_DELAY.end();

/// Whether the system asks for less motion (its accessibility setting), where that can be asked without a
/// library of its own: macOS's and GNOME's.
pub fn system_reduces_motion() -> bool {
    let said = |program: &str, args: &[&str]| std::process::Command::new(program).args(args).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    if cfg!(target_os = "macos") {
        said("defaults", &["read", "com.apple.universalaccess", "reduceMotion"]).is_some_and(|v| v == "1")
    } else if cfg!(all(unix, not(target_os = "macos"))) {
        said("gsettings", &["get", "org.gnome.desktop.interface", "enable-animations"]).is_some_and(|v| v == "false")
    } else {
        false
    }
}

/// The name a player has before they choose one: their account's on this machine.
fn system_name() -> String {
    let name = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
    clean_name(&name)
}

/// A name as it is kept: no control characters, 16 at most.
pub fn clean_name(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).take(NAME_LENGTH).collect()
}

/// A signaling server URL while being typed: no control characters, [`SIGNAL_SERVER_LENGTH`] at most.
/// Untrimmed so the UI field doesn't jump its caret mid-edit.
pub fn clean_signaling_server_draft(url: &str) -> String {
    url.chars().filter(|c| !c.is_control()).take(SIGNAL_SERVER_LENGTH).collect()
}

/// A signaling server URL as it is saved or used: trimmed, no control characters, [`SIGNAL_SERVER_LENGTH`] at most.
pub fn clean_signaling_server(url: &str) -> String {
    clean_signaling_server_draft(url.trim())
}

impl Settings {
    /// The settings as the file has them (the defaults where it isn't there or doesn't read).
    pub fn load() -> Settings {
        let mut s: Settings = saito_files::read_to_string(crate::paths::settings()).ok().and_then(|t| toml::from_str(&t).ok()).unwrap_or_default();
        s.name = clean_name(&s.name);
        s.music = s.music.min(100);
        s.effects = s.effects.min(100);
        s.input_delay = s.input_delay.min(MAX_INPUT_DELAY);
        s.signaling_server = clean_signaling_server(&s.signaling_server);
        s.mic_sensitivity = if s.mic_sensitivity.is_finite() {
            s.mic_sensitivity.clamp(MIC_SENSITIVITY_MIN, MIC_SENSITIVITY_MAX)
        } else {
            1.0
        };
        s.quick_shout_tier = s.quick_shout_tier.clamp(1, 3);
        s.touch_pad_layout = s.touch_pad_layout.sanitized();
        s.sanitize_music();
        saito_extract::nds::set_exe5ds_rom_path(s.exe5ds_rom.clone());
        crate::music::set_prefer_ds_music(s.exe5ds_music);
        saito_frontend::ds_voices::set_enabled(s.exe5ds_voices);
        s
    }

    /// Write the settings to the file.
    pub fn save(&self) -> Result<(), String> {
        let path = crate::paths::settings();
        if let Some(dir) = path.parent() {
            saito_files::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        saito_files::write(&path, format!("# saito's settings (docs/app.md §9).\n\n{text}")).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The battle's buttons as bound: the defaults, with what the file binds otherwise.
    pub fn bindings(&self) -> Bindings {
        let mut b = Bindings::default();
        for (i, (_, name)) in BUTTONS.iter().enumerate() {
            if let Some(key) = self.keys.get(*name).and_then(|k| bind_named(k)) {
                b.keys[i] = key;
            }
            if let Some(button) = self.pad.get(*name).and_then(|p| pad_named(p)) {
                b.pad[i] = button;
            }
        }
        b
    }

    /// Keep `bindings`: what differs from the defaults.
    pub fn set_bindings(&mut self, bindings: &Bindings) {
        let usual = Bindings::default();
        self.keys.clear();
        self.pad.clear();
        for (i, (_, name)) in BUTTONS.iter().enumerate() {
            if bindings.keys[i] != usual.keys[i]
                && let Some(key) = bind_name(bindings.keys[i])
            {
                self.keys.insert(name.to_string(), key.to_string());
            }
            if bindings.pad[i] != usual.pad[i]
                && let Some(button) = pad_name(bindings.pad[i])
            {
                self.pad.insert(name.to_string(), button.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use i_slint_backend_winit::winit::event::MouseButton;
    use i_slint_backend_winit::winit::keyboard::KeyCode;

    /// Settings read back as written; a file of other keys, or none, is the defaults.
    #[test]
    fn settings_read_back() {
        let mut s = Settings {
            name: "Lan".into(),
            language: Some("ja".into()),
            music: 30,
            full_screen: true,
            hide_landscape_status_bar: true,
            keyboard_only_custom: true,
            input_delay: 2,
            signaling_server: "wss://signal.example.com".into(),
            legacy_collection: Some("Steam Library/Legacy Collection Vol. 2".into()),
            ..Settings::default()
        };
        s.remembered.insert("play.build.exe6".into(), "Falzar heat".into());
        s.keys.insert("a".into(), "KeyJ".into());
        let text = toml::to_string(&s).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), s);
        let other: Settings = toml::from_str("music = 40\nunknown = 1\n").unwrap();
        assert_eq!(other, Settings { music: 40, ..Settings::default() });
        assert!(other.landscape_touch_pad, "default landscape touch pad on");
        assert!(!other.hide_landscape_status_bar, "default landscape status bar visible");
        assert!(!other.keyboard_only_custom, "default mouse custom selection enabled");
        assert!(!other.hide_aura_number, "default LifeAura number visible");
        assert!(!other.sync_dark_buster_arm, "default EXE5 dark buster arm sync off");
        assert_eq!(other.exe3_hud_style, Exe3HudStylePref::Auto, "default EXE3 HUD style is Auto");
        assert!(!other.hd_chip_art, "default HD chip art off");
        let mut toggled = Settings { hide_aura_number: true, sync_dark_buster_arm: true, exe3_hud_style: Exe3HudStylePref::Black, hd_chip_art: true, ..Settings::default() };
        assert!(toggled.enhancements().hide_aura_number);
        assert!(toggled.enhancements().sync_dark_buster_arm);
        assert_eq!(toggled.enhancements().exe3_hud_style, saito_frontend::game::Exe3HudStyle::Black);
        assert!(toggled.enhancements().hd_chip_art);
        let toggled_toml = toml::to_string(&toggled).unwrap();
        assert_eq!(toml::from_str::<Settings>(&toggled_toml).unwrap(), toggled);
        toggled.set_enhancements(saito_frontend::game::Enhancements::NONE);
        assert!(!toggled.hide_aura_number);
        assert!(!toggled.sync_dark_buster_arm);
        assert_eq!(toggled.exe3_hud_style, Exe3HudStylePref::Auto);
        assert!(!toggled.hd_chip_art);
        assert_eq!(other.signaling_server, "");
        assert_eq!(Settings::default().input_delay, 4, "the input delay is 4 until the player sets another");
        assert_eq!(clean_signaling_server("  ws://127.0.0.1:8787\n "), "ws://127.0.0.1:8787");
        assert_eq!(clean_signaling_server("   "), "");
    }

    /// The bindings kept are those that differ from the defaults, and read back.
    #[test]
    fn bindings_read_back() {
        let mut s = Settings::default();
        let mut b = s.bindings();
        assert_eq!(b, Bindings::default());
        b.bind_key(4, KeyCode::KeyJ);
        b.bind_key(5, MouseButton::Right);
        b.bind_pad(5, saito_gamepad::Button::LeftStick);
        s.set_bindings(&b);
        assert_eq!((s.keys.len(), s.pad.len()), (2, 1));
        assert_eq!(s.keys.get("b").map(String::as_str), Some("MouseRight"));
        assert_eq!(s.bindings(), b);
    }

    #[test]
    fn touch_pad_layout_roundtrips_and_sanitizes() {
        let mut s = Settings::default();
        assert!(!toml::to_string(&s).unwrap().contains("touch_pad_layout"), "default layout omitted from TOML");

        s.touch_pad_layout.portrait.a = TouchControlLayout { dx: -0.25, dy: 0.15, scale: 1.3 };
        s.touch_pad_layout.landscape.dpad = TouchControlLayout { dx: 0.2, dy: -0.1, scale: 0.8 };
        let text = toml::to_string(&s).unwrap();
        assert!(text.contains("touch_pad_layout"));
        let decoded: Settings = toml::from_str(&text).unwrap();
        assert_eq!(decoded.touch_pad_layout, s.touch_pad_layout);

        let dirty = TouchControlLayout { dx: 4.0, dy: f32::NAN, scale: 0.1 }.sanitized();
        assert_eq!(dirty, TouchControlLayout { dx: 1.0, dy: 0.0, scale: TOUCH_SCALE_MIN });
        let infinite = TouchControlLayout { dx: f32::NEG_INFINITY, dy: -2.5, scale: f32::INFINITY }.sanitized();
        assert_eq!(infinite, TouchControlLayout { dx: 0.0, dy: -1.0, scale: 1.0 });
    }

    #[test]
    fn exe5ds_mic_and_rom_settings_roundtrip_and_default() {
        // Older settings.toml without new EXE5DS fields deserializes cleanly to privacy-first defaults.
        let legacy: Settings = toml::from_str("music = 55\n").unwrap();
        assert_eq!(legacy.exe5ds_rom, None);
        assert!(legacy.exe5ds_music);
        assert!(legacy.exe5ds_voices);
        assert!(!legacy.mic_shout_enabled, "microphone shout must be opt-in by default");
        assert_eq!(legacy.mic_sensitivity, 1.0);
        assert_eq!(legacy.quick_shout_tier, 3);

        // Full roundtrip with custom EXE5DS ROM & mic settings.
        let s = Settings {
            exe5ds_rom: Some("/roms/exe5ds.nds".into()),
            exe5ds_music: false,
            exe5ds_voices: false,
            mic_shout_enabled: true,
            mic_sensitivity: 1.75,
            quick_shout_tier: 2,
            ..Settings::default()
        };
        let encoded = toml::to_string(&s).unwrap();
        let decoded: Settings = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, s);
    }

    #[test]
    fn screen_orientation_roundtrip_default_and_resolution() {
        let legacy: Settings = toml::from_str("music = 60\n").unwrap();
        assert_eq!(legacy.screen_orientation, ScreenOrientation::Portrait, "default screen orientation is Portrait");

        for (i, pref) in [
            ScreenOrientation::Portrait,
            ScreenOrientation::Landscape,
            ScreenOrientation::BattleLandscape,
            ScreenOrientation::Auto,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(pref.as_index(), i);
            assert_eq!(ScreenOrientation::from_index(i as i32), pref);
            let s = Settings { screen_orientation: pref, ..Settings::default() };
            let encoded = toml::to_string(&s).unwrap();
            let decoded: Settings = toml::from_str(&encoded).unwrap();
            assert_eq!(decoded.screen_orientation, pref);
        }
        assert_eq!(ScreenOrientation::from_index(-1), ScreenOrientation::Auto);
        assert_eq!(ScreenOrientation::from_index(4), ScreenOrientation::Portrait);

        // No override: follows preference and battle state.
        assert_eq!(resolve_orientation(ScreenOrientation::Portrait, false, None), OrientationLock::Portrait);
        assert_eq!(resolve_orientation(ScreenOrientation::Portrait, true, None), OrientationLock::Portrait);
        assert_eq!(resolve_orientation(ScreenOrientation::Landscape, false, None), OrientationLock::Landscape);
        assert_eq!(resolve_orientation(ScreenOrientation::Landscape, true, None), OrientationLock::Landscape);
        assert_eq!(resolve_orientation(ScreenOrientation::BattleLandscape, false, None), OrientationLock::Portrait);
        assert_eq!(resolve_orientation(ScreenOrientation::BattleLandscape, true, None), OrientationLock::Landscape);
        assert_eq!(resolve_orientation(ScreenOrientation::Auto, false, None), OrientationLock::Auto);
        assert_eq!(resolve_orientation(ScreenOrientation::Auto, true, None), OrientationLock::Auto);

        // Temporary manual override always takes precedence over preference (including Auto).
        for pref in [
            ScreenOrientation::Portrait,
            ScreenOrientation::Landscape,
            ScreenOrientation::BattleLandscape,
            ScreenOrientation::Auto,
        ] {
            assert_eq!(resolve_orientation(pref, true, Some(true)), OrientationLock::Landscape);
            assert_eq!(resolve_orientation(pref, true, Some(false)), OrientationLock::Portrait);
        }
        assert_eq!(OrientationLock::Auto.as_android_mode(), 0);
        assert_eq!(OrientationLock::Portrait.as_android_mode(), 1);
        assert_eq!(OrientationLock::Landscape.as_android_mode(), 2);
    }

    #[test]
    fn auto_input_delay_steps_resolves_and_roundtrips() {
        let legacy: Settings = toml::from_str("input_delay = 3\n").unwrap();
        assert!(!legacy.auto_input_delay);
        assert_eq!(legacy.input_delay, 3);

        let mut s = Settings { input_delay: 1, auto_input_delay: false, ..Settings::default() };
        s.step_input_delay(-1);
        assert!(!s.auto_input_delay && s.input_delay == 0);
        s.step_input_delay(-1);
        assert!(s.auto_input_delay, "stepping below 0 enters Auto");
        s.step_input_delay(-1);
        assert!(s.auto_input_delay, "Auto clamps at the lower bound");

        // Resolved delay in Auto mode uses ping+jitter or falls back to default when unmeasured.
        assert_eq!(s.resolved_input_delay(None), saito_frontend::netplay::DEFAULT_INPUT_DELAY);
        assert_eq!(s.resolved_input_delay(Some(saito_frontend::connection::Ping { ms: 50.0, jitter_ms: 5.0 })), 2);

        // Roundtrip in TOML.
        let encoded = toml::to_string(&s).unwrap();
        let decoded: Settings = toml::from_str(&encoded).unwrap();
        assert!(decoded.auto_input_delay);

        // Stepping right from Auto returns to 0 and clamps at MAX_INPUT_DELAY.
        s.step_input_delay(1);
        assert!(!s.auto_input_delay && s.input_delay == 0);
        assert_eq!(s.resolved_input_delay(Some(saito_frontend::connection::Ping { ms: 200.0, jitter_ms: 10.0 })), 0);
        s.step_input_delay(99);
        assert!(!s.auto_input_delay && s.input_delay == MAX_INPUT_DELAY);
    }

    #[test]
    fn favorite_music_and_menu_bgm_roundtrip_and_reject_non_looping() {
        let legacy: Settings = toml::from_str("music = 70\n").unwrap();
        assert!(legacy.favorite_music.is_empty());
        assert_eq!(legacy.favorite_battle_bgm, FavoriteBgmMode::Off);
        assert_eq!(legacy.menu_bgm, None);

        let mut s = Settings::default();
        assert!(s.toggle_favorite_music("exe6/virus-battle"));
        assert!(s.toggle_favorite_music("exe5/ds-liberate"));
        // Non-looping jingles are preview-only and must be rejected from favorites.
        assert!(!s.toggle_favorite_music("exe6/transmission"));
        assert!(!s.toggle_favorite_music("exe6/game-over"));
        assert!(!s.toggle_favorite_music("exe5/ds-1turn-delete"));
        assert!(!s.toggle_favorite_music("malformed-no-slash"));
        assert_eq!(s.favorite_music, vec!["exe6/virus-battle", "exe5/ds-liberate"]);

        // Toggling an existing favorite removes it.
        assert!(!s.toggle_favorite_music("exe6/virus-battle"));
        assert_eq!(s.favorite_music, vec!["exe5/ds-liberate"]);

        s.favorite_battle_bgm = FavoriteBgmMode::Sequential;
        s.menu_bgm = Some("exe6/acdc-town".into());
        let encoded = toml::to_string(&s).unwrap();
        let decoded: Settings = toml::from_str(&encoded).unwrap();
        assert_eq!(decoded, s);

        // Sanitization deduplicates and drops non-looping entries from hand-edited TOML.
        let mut dirty: Settings = toml::from_str(
            "favorite_music = [\"exe6/acdc-town\", \"exe6/winner-1\", \"exe6/acdc-town\", \"bad\"]\nmenu_bgm = \"exe4/game-over\"\n",
        )
        .unwrap();
        dirty.sanitize_music();
        assert_eq!(dirty.favorite_music, vec!["exe6/acdc-town"]);
        assert_eq!(dirty.menu_bgm, None);
    }
}
