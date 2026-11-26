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

/// What the file keeps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The player's name, shown to their opponent.
    pub name: String,
    /// The language, by its code (`ja`); none: the system's.
    pub language: Option<String>,
    /// The game last used: the one the app opens on.
    pub game: Option<String>,
    /// The most the battle's picture is, in multiples of 240 by 160; 0: the largest that fits.
    pub picture_size: u32,
    pub full_screen: bool,
    /// On-screen virtual touch pad shown over the battle in mobile landscape.
    pub landscape_touch_pad: bool,
    /// Hide Android's top status bar in landscape orientation.
    pub hide_landscape_status_bar: bool,
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
    /// Hide the "200" number on LifeAura barriers.
    pub hide_aura_number: bool,
    /// Use Legacy Collection high-resolution chip artwork when available.
    pub hd_chip_art: bool,
    /// 0 to 100.
    pub music: u32,
    pub effects: u32,
    /// Netplay's input delay, in frames: the value the lobby starts with (the library's
    /// `NetOptions::input_delay`).
    pub input_delay: u32,
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
            game: None,
            picture_size: 0,
            full_screen: false,
            landscape_touch_pad: true,
            hide_landscape_status_bar: false,
            keyboard_only_custom: false,
            reduce_motion: None,
            check_updates: true,
            sharp_text: true,
            sharp_hud: true,
            smooth_motion: true,
            fill_window: true,
            reduce_flashes: false,
            hide_aura_number: false,
            hd_chip_art: false,
            music: 80,
            effects: 100,
            input_delay: saito_frontend::netplay::DEFAULT_INPUT_DELAY,
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
            hd_chip_art: self.hd_chip_art,
        }
    }

    pub fn set_enhancements(&mut self, e: saito_frontend::game::Enhancements) {
        (self.sharp_text, self.sharp_hud, self.smooth_motion, self.fill_window, self.reduce_flashes, self.hide_aura_number, self.hd_chip_art) =
            (e.text, e.hud, e.motion, e.fill, e.reduce_flashes, e.hide_aura_number, e.hd_chip_art);
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
        assert!(!other.hd_chip_art, "default HD chip art off");
        let mut toggled = Settings { hide_aura_number: true, hd_chip_art: true, ..Settings::default() };
        assert!(toggled.enhancements().hide_aura_number);
        assert!(toggled.enhancements().hd_chip_art);
        let toggled_toml = toml::to_string(&toggled).unwrap();
        assert_eq!(toml::from_str::<Settings>(&toggled_toml).unwrap(), toggled);
        toggled.set_enhancements(saito_frontend::game::Enhancements::NONE);
        assert!(!toggled.hide_aura_number);
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
}
