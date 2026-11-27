//! UI theme engine and `.saitotheme` plugin loader (`docs/design/ui-themes.md`).
//!
//! Provides two built-in themes — `pet-hd` (HD Vector PET, default) and `terminal` (classic dark network
//! terminal driven by `content/<game>/look.toml`) — plus user-installed themes in `crate::paths::themes()`
//! (either `<folder>/theme.toml` or `<name>.saitotheme` ZIP archives).
//!
//! Purely presentation-layer: nothing here touches `saito-battle`, rollback state, or replay digests.

use saito_ui::{Look, PatternKind};
use serde::Deserialize;
use slint::{Color, Image, Rgba8Pixel, SharedPixelBuffer};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Default built-in theme ID (HD Vector PET).
pub const DEFAULT_THEME: &str = "pet-hd";
/// Classic dark terminal built-in theme ID.
pub const TERMINAL_THEME: &str = "terminal";
/// Manifest filename inside a theme folder or `.saitotheme` archive.
pub const MANIFEST_FILE: &str = "theme.toml";
/// File extension for packaged theme archives.
pub const EXTENSION: &str = "saitotheme";
/// Current supported schema version.
pub const SCHEMA_VERSION: u32 = 1;
/// Max bytes read from a single theme asset (16 MiB).
const MAX_ASSET_BYTES: usize = 16 * 1024 * 1024;
/// Max bytes read for a `theme.toml` manifest (256 KiB).
const MAX_MANIFEST_BYTES: usize = 256 * 1024;

/// Registered external font cache so repeated theme switches never re-register the same font file.
static REGISTERED_FONTS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// Summary entry shown in Settings' theme picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub author: String,
    pub builtin: bool,
}

/// Optional visual assets resolved for the active theme.
#[derive(Clone, Debug, Default)]
pub struct ThemeAssets {
    pub backdrop_image: Option<Image>,
    pub backdrop_image_opacity: f32,
    pub display_font: Option<String>,
    pub body_font: Option<String>,
    pub mono_font: Option<String>,
}

/// A fully resolved theme ready to drive `Looks` and `saito_ui::Theme`.
#[derive(Clone, Debug)]
pub struct ResolvedTheme {
    pub id: String,
    pub pet: bool,
    pub default_look: Look,
    pub games: HashMap<String, Look>,
    pub assets: ThemeAssets,
}

/// An RGB color in `theme.toml`: accepts either `0xRRGGBB` integers or `"#RRGGBB"` strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeRgb(pub [u8; 3]);

impl ThemeRgb {
    pub const fn from_u32(v: u32) -> Self {
        Self([((v >> 16) & 0xFF) as u8, ((v >> 8) & 0xFF) as u8, (v & 0xFF) as u8])
    }

    pub fn color(self) -> Color {
        Color::from_rgb_u8(self.0[0], self.0[1], self.0[2])
    }

    pub fn with_alpha(self, alpha: f32) -> Color {
        self.color().with_alpha(alpha.clamp(0.0, 1.0))
    }
}

impl<'de> Deserialize<'de> for ThemeRgb {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Int(i64),
            Str(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Int(v) => u32::try_from(v)
                .ok()
                .filter(|c| *c <= 0xFF_FFFF)
                .map(ThemeRgb::from_u32)
                .ok_or_else(|| serde::de::Error::custom(format!("{v:#x} is not a 0xRRGGBB color"))),
            Raw::Str(s) => {
                let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
                if hex.len() != 6 {
                    return Err(serde::de::Error::custom(format!("{s:?} is not a #RRGGBB color")));
                }
                u32::from_str_radix(hex, 16)
                    .map(ThemeRgb::from_u32)
                    .map_err(|_| serde::de::Error::custom(format!("{s:?} is not a #RRGGBB color")))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StyleKind {
    Pet,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BackdropPattern {
    PetGrid,
    Hexagons,
    IsometricGrid,
    Starfield,
}

impl From<BackdropPattern> for PatternKind {
    fn from(p: BackdropPattern) -> Self {
        match p {
            BackdropPattern::PetGrid => PatternKind::PetGrid,
            BackdropPattern::Hexagons => PatternKind::Hexagons,
            BackdropPattern::IsometricGrid => PatternKind::IsometricGrid,
            BackdropPattern::Starfield => PatternKind::Starfield,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct StylePatch {
    kind: Option<StyleKind>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct ColorsPatch {
    ground_top: Option<ThemeRgb>,
    ground_bottom: Option<ThemeRgb>,
    glow: Option<ThemeRgb>,
    glow_strength: Option<f32>,
    panel: Option<ThemeRgb>,
    raised: Option<ThemeRgb>,
    line: Option<ThemeRgb>,
    inset: Option<ThemeRgb>,
    text: Option<ThemeRgb>,
    muted: Option<ThemeRgb>,
    dim: Option<ThemeRgb>,
    cursor: Option<ThemeRgb>,
    action: Option<ThemeRgb>,
    action_ink: Option<ThemeRgb>,
    frame_stroke: Option<ThemeRgb>,
    btn_shadow: Option<ThemeRgb>,
    active_top: Option<ThemeRgb>,
    active_bot: Option<ThemeRgb>,
    active_border: Option<ThemeRgb>,
    active_shadow: Option<ThemeRgb>,
    slot_bg: Option<ThemeRgb>,
    slot_border: Option<ThemeRgb>,
    bar_start: Option<ThemeRgb>,
    bar_end: Option<ThemeRgb>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct BackdropPatch {
    kind: Option<BackdropPattern>,
    color: Option<ThemeRgb>,
    strength: Option<f32>,
    image: Option<String>,
    image_opacity: Option<f32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct FontsPatch {
    display: Option<String>,
    body: Option<String>,
    mono: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct GamePatch {
    #[serde(default)]
    colors: ColorsPatch,
    #[serde(default)]
    backdrop: BackdropPatch,
}

#[derive(Clone, Debug, Deserialize)]
struct Manifest {
    #[serde(default = "default_schema")]
    schema: u32,
    id: String,
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    #[allow(dead_code)]
    version: String,
    #[serde(default = "default_extends")]
    extends: String,
    #[serde(default)]
    style: StylePatch,
    #[serde(default)]
    colors: ColorsPatch,
    #[serde(default)]
    backdrop: BackdropPatch,
    #[serde(default)]
    fonts: FontsPatch,
    #[serde(default)]
    games: BTreeMap<String, GamePatch>,
}

fn default_schema() -> u32 {
    SCHEMA_VERSION
}

fn default_extends() -> String {
    DEFAULT_THEME.to_string()
}

/// Source backing an installed user theme.
#[derive(Clone, Debug)]
enum Source {
    Dir(PathBuf),
    Zip { path: PathBuf, prefix: String },
}

impl Source {
    fn read_asset(&self, rel: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        let clean = sanitize_rel_path(rel)?;
        match self {
            Source::Dir(dir) => {
                let path = dir.join(&clean);
                let data = saito_files::read(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
                if data.len() > max_bytes {
                    return Err(format!("{} exceeds {} bytes", path.display(), max_bytes));
                }
                Ok(data)
            }
            Source::Zip { path, prefix } => {
                let bytes = saito_files::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
                let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("opening {}: {e}", path.display()))?;
                let target = if prefix.is_empty() { clean } else { format!("{prefix}/{clean}") };
                let entry = archive.by_name(&target).map_err(|_| format!("{target} not found in {}", path.display()))?;
                if entry.size() as usize > max_bytes {
                    return Err(format!("{target} exceeds {max_bytes} bytes"));
                }
                let mut out = Vec::with_capacity(entry.size() as usize);
                entry.take((max_bytes + 1) as u64).read_to_end(&mut out).map_err(|e| format!("reading {target}: {e}"))?;
                if out.len() > max_bytes {
                    return Err(format!("{target} exceeds {max_bytes} bytes"));
                }
                Ok(out)
            }
        }
    }

    fn cache_key(&self, rel: &str) -> String {
        match self {
            Source::Dir(dir) => format!("dir:{}:{}", dir.display(), rel),
            Source::Zip { path, prefix } => format!("zip:{}:{}:{}", path.display(), prefix, rel),
        }
    }
}

/// Sanitize a theme-relative path: must be relative, non-empty, and contain no `..` or root prefixes.
pub fn sanitize_rel_path(raw: &str) -> Result<String, String> {
    let normalized = raw.replace('\\', "/");
    let path = Path::new(&normalized);
    let mut parts = Vec::new();
    for c in path.components() {
        match c {
            Component::Normal(s) => {
                let s = s.to_str().ok_or_else(|| format!("non-utf8 path {raw:?}"))?;
                if s.contains(':') {
                    return Err(format!("invalid path {raw:?}"));
                }
                parts.push(s);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!("unsafe relative path {raw:?}"));
            }
        }
    }
    if parts.is_empty() {
        return Err("empty asset path".to_string());
    }
    Ok(parts.join("/"))
}

/// Sanitize a theme ID into a safe filename stem.
pub fn sanitize_id(raw: &str) -> String {
    let clean: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(48)
        .collect();
    if clean.is_empty() { "custom-theme".to_string() } else { clean.to_ascii_lowercase() }
}

/// Built-in HD Vector PET look for a specific game (`exe3`, `exe4`, `exe5`, `exe6`).
pub fn pet_hd_look(game: Option<&str>) -> Look {
    match game {
        Some("exe3") => Look {
            ground_top: ThemeRgb::from_u32(0x222328).color(),
            ground_bottom: ThemeRgb::from_u32(0x09090B).color(),
            ground_glow: ThemeRgb::from_u32(0xD4D8E2).with_alpha(0.16),
            panel: ThemeRgb::from_u32(0x141519).color(),
            raised: ThemeRgb::from_u32(0x1D1F25).color(),
            line: ThemeRgb::from_u32(0xD4D8E2).color(),
            inset: ThemeRgb::from_u32(0x0B0C0F).color(),
            text: ThemeRgb::from_u32(0xF5F6F8).color(),
            muted: ThemeRgb::from_u32(0xB4B8C4).color(),
            dim: ThemeRgb::from_u32(0x6F7380).color(),
            cursor: ThemeRgb::from_u32(0xE2E6F0).color(),
            action: ThemeRgb::from_u32(0xFF9400).color(),
            action_ink: ThemeRgb::from_u32(0xFFFFFF).color(),
            pattern: PatternKind::PetGrid,
            pattern_color: ThemeRgb::from_u32(0xD4D8E2).color(),
            pattern_strength: 0.09,
            frame_stroke: ThemeRgb::from_u32(0x464954).color(),
            btn_shadow: ThemeRgb::from_u32(0x050507).color(),
            active_top: ThemeRgb::from_u32(0xFFA408).color(),
            active_bot: ThemeRgb::from_u32(0xFF7800).color(),
            active_border: ThemeRgb::from_u32(0xFFF4D6).color(),
            active_shadow: ThemeRgb::from_u32(0xD98200).color(),
            slot_bg: ThemeRgb::from_u32(0x272932).color(),
            slot_border: ThemeRgb::from_u32(0xC8CDD8).color(),
            bar_start: ThemeRgb::from_u32(0x7A7F8E).color(),
            bar_end: ThemeRgb::from_u32(0xE2E6F0).color(),
        },
        Some("exe4") => Look {
            ground_top: ThemeRgb::from_u32(0x321D78).color(),
            ground_bottom: ThemeRgb::from_u32(0x120933).color(),
            ground_glow: ThemeRgb::from_u32(0xC49BFF).with_alpha(0.18),
            panel: ThemeRgb::from_u32(0x1C1144).color(),
            raised: ThemeRgb::from_u32(0x221459).color(),
            line: ThemeRgb::from_u32(0xC49BFF).color(),
            inset: ThemeRgb::from_u32(0x10092C).color(),
            text: ThemeRgb::from_u32(0xF2F9FF).color(),
            muted: ThemeRgb::from_u32(0xC4B5EA).color(),
            dim: ThemeRgb::from_u32(0x7B69AB).color(),
            cursor: ThemeRgb::from_u32(0xC49BFF).color(),
            action: ThemeRgb::from_u32(0xFF9400).color(),
            action_ink: ThemeRgb::from_u32(0xFFFFFF).color(),
            pattern: PatternKind::PetGrid,
            pattern_color: ThemeRgb::from_u32(0xC49AFF).color(),
            pattern_strength: 0.12,
            frame_stroke: ThemeRgb::from_u32(0x4B249E).color(),
            btn_shadow: ThemeRgb::from_u32(0x0E072B).color(),
            active_top: ThemeRgb::from_u32(0xFFA408).color(),
            active_bot: ThemeRgb::from_u32(0xFF7800).color(),
            active_border: ThemeRgb::from_u32(0xFFF4D6).color(),
            active_shadow: ThemeRgb::from_u32(0xD98200).color(),
            slot_bg: ThemeRgb::from_u32(0x3B2280).color(),
            slot_border: ThemeRgb::from_u32(0xD2B4FF).color(),
            bar_start: ThemeRgb::from_u32(0x8F5CFF).color(),
            bar_end: ThemeRgb::from_u32(0xD4B8FF).color(),
        },
        Some("exe5") => Look {
            ground_top: ThemeRgb::from_u32(0x0B4A5E).color(),
            ground_bottom: ThemeRgb::from_u32(0x06212C).color(),
            ground_glow: ThemeRgb::from_u32(0x59F7D6).with_alpha(0.18),
            panel: ThemeRgb::from_u32(0x0A2C38).color(),
            raised: ThemeRgb::from_u32(0x0B3A4A).color(),
            line: ThemeRgb::from_u32(0x59F7D6).color(),
            inset: ThemeRgb::from_u32(0x061A23).color(),
            text: ThemeRgb::from_u32(0xF2F9FF).color(),
            muted: ThemeRgb::from_u32(0x9ADBD0).color(),
            dim: ThemeRgb::from_u32(0x558C85).color(),
            cursor: ThemeRgb::from_u32(0x59F7D6).color(),
            action: ThemeRgb::from_u32(0xFF9400).color(),
            action_ink: ThemeRgb::from_u32(0xFFFFFF).color(),
            pattern: PatternKind::PetGrid,
            pattern_color: ThemeRgb::from_u32(0x59F7D6).color(),
            pattern_strength: 0.11,
            frame_stroke: ThemeRgb::from_u32(0x0A6368).color(),
            btn_shadow: ThemeRgb::from_u32(0x041C24).color(),
            active_top: ThemeRgb::from_u32(0xFFA408).color(),
            active_bot: ThemeRgb::from_u32(0xFF7800).color(),
            active_border: ThemeRgb::from_u32(0xFFF4D6).color(),
            active_shadow: ThemeRgb::from_u32(0xD98200).color(),
            slot_bg: ThemeRgb::from_u32(0x125C63).color(),
            slot_border: ThemeRgb::from_u32(0x7EFCE2).color(),
            bar_start: ThemeRgb::from_u32(0x12B89A).color(),
            bar_end: ThemeRgb::from_u32(0x6AFCE0).color(),
        },
        _ => Look {
            ground_top: ThemeRgb::from_u32(0x0B3C91).color(),
            ground_bottom: ThemeRgb::from_u32(0x061B47).color(),
            ground_glow: ThemeRgb::from_u32(0x5AF4FF).with_alpha(0.18),
            panel: ThemeRgb::from_u32(0x09244A).color(),
            raised: ThemeRgb::from_u32(0x092E59).color(),
            line: ThemeRgb::from_u32(0x5AF4FF).color(),
            inset: ThemeRgb::from_u32(0x051630).color(),
            text: ThemeRgb::from_u32(0xF2F9FF).color(),
            muted: ThemeRgb::from_u32(0x9BC4EE).color(),
            dim: ThemeRgb::from_u32(0x5C83B0).color(),
            cursor: ThemeRgb::from_u32(0x5AF4FF).color(),
            action: ThemeRgb::from_u32(0xFF9400).color(),
            action_ink: ThemeRgb::from_u32(0xFFFFFF).color(),
            pattern: PatternKind::PetGrid,
            pattern_color: ThemeRgb::from_u32(0x60FAFF).color(),
            pattern_strength: 0.11,
            frame_stroke: ThemeRgb::from_u32(0x0A44B8).color(),
            btn_shadow: ThemeRgb::from_u32(0x051B38).color(),
            active_top: ThemeRgb::from_u32(0xFFA408).color(),
            active_bot: ThemeRgb::from_u32(0xFF7800).color(),
            active_border: ThemeRgb::from_u32(0xFFF4D6).color(),
            active_shadow: ThemeRgb::from_u32(0xD98200).color(),
            slot_bg: ThemeRgb::from_u32(0x15458A).color(),
            slot_border: ThemeRgb::from_u32(0x7AE9FF).color(),
            bar_start: ThemeRgb::from_u32(0x1A9BFF).color(),
            bar_end: ThemeRgb::from_u32(0x5AF4FF).color(),
        },
    }
}

/// Convert a `saito_content::look::Look` (classic terminal) into a `saito_ui::Look`.
pub fn terminal_look_from_content(look: &saito_content::look::Look) -> Look {
    use saito_content::look::{PatternKind as Kind, Rgb, Strength};
    let rgb = |c: Rgb| {
        let [r, g, b] = c.channels();
        Color::from_rgb_u8(r, g, b)
    };
    let with = |c: Rgb, s: Strength| rgb(c).with_alpha(s.0);
    let line = rgb(look.line);
    let inset = rgb(look.inset);
    let cursor = rgb(look.cursor);
    let action = rgb(look.action);
    Look {
        ground_top: rgb(look.ground_top),
        ground_bottom: rgb(look.ground_bottom),
        ground_glow: with(look.glow, look.glow_strength),
        panel: rgb(look.panel),
        raised: rgb(look.raised),
        line,
        inset,
        text: rgb(look.text),
        muted: rgb(look.muted),
        dim: rgb(look.dim),
        cursor,
        action,
        action_ink: rgb(look.action_ink),
        pattern: match look.pattern.kind {
            Kind::Hexagons => PatternKind::Hexagons,
            Kind::IsometricGrid => PatternKind::IsometricGrid,
            Kind::Starfield => PatternKind::Starfield,
        },
        pattern_color: rgb(look.pattern.color),
        pattern_strength: look.pattern.strength.0,
        frame_stroke: line,
        btn_shadow: inset,
        active_top: action,
        active_bot: action,
        active_border: cursor,
        active_shadow: inset,
        slot_bg: inset,
        slot_border: line,
        bar_start: cursor,
        bar_end: cursor,
    }
}

/// Default classic terminal look (BN6 night-blue) when no `look.toml` is available.
pub fn terminal_default_look() -> Look {
    let line = ThemeRgb::from_u32(0x27508F).color();
    let inset = ThemeRgb::from_u32(0x040813).color();
    let cursor = ThemeRgb::from_u32(0x66E0FF).color();
    let action = ThemeRgb::from_u32(0xFFCC1A).color();
    Look {
        ground_top: ThemeRgb::from_u32(0x0A1330).color(),
        ground_bottom: ThemeRgb::from_u32(0x050916).color(),
        ground_glow: ThemeRgb::from_u32(0x286EFF).with_alpha(0.28),
        panel: ThemeRgb::from_u32(0x0C1836).color(),
        raised: ThemeRgb::from_u32(0x13254D).color(),
        line,
        inset,
        text: ThemeRgb::from_u32(0xEAF3FF).color(),
        muted: ThemeRgb::from_u32(0x8EA8D4).color(),
        dim: ThemeRgb::from_u32(0x55698F).color(),
        cursor,
        action,
        action_ink: ThemeRgb::from_u32(0x000000).color(),
        pattern: PatternKind::Hexagons,
        pattern_color: ThemeRgb::from_u32(0x62B4FF).color(),
        pattern_strength: 0.078,
        frame_stroke: line,
        btn_shadow: inset,
        active_top: action,
        active_bot: action,
        active_border: cursor,
        active_shadow: inset,
        slot_bg: inset,
        slot_border: line,
        bar_start: cursor,
        bar_end: cursor,
    }
}

fn apply_colors_patch(look: &mut Look, p: &ColorsPatch) {
    if let Some(c) = p.ground_top {
        look.ground_top = c.color();
    }
    if let Some(c) = p.ground_bottom {
        look.ground_bottom = c.color();
    }
    if let Some(c) = p.glow {
        let a = p.glow_strength.unwrap_or_else(|| look.ground_glow.alpha() as f32 / 255.0);
        look.ground_glow = c.with_alpha(a);
    } else if let Some(a) = p.glow_strength {
        look.ground_glow = look.ground_glow.with_alpha(a.clamp(0.0, 1.0));
    }
    if let Some(c) = p.panel {
        look.panel = c.color();
    }
    if let Some(c) = p.raised {
        look.raised = c.color();
    }
    if let Some(c) = p.line {
        look.line = c.color();
    }
    if let Some(c) = p.inset {
        look.inset = c.color();
    }
    if let Some(c) = p.text {
        look.text = c.color();
    }
    if let Some(c) = p.muted {
        look.muted = c.color();
    }
    if let Some(c) = p.dim {
        look.dim = c.color();
    }
    if let Some(c) = p.cursor {
        look.cursor = c.color();
    }
    if let Some(c) = p.action {
        look.action = c.color();
    }
    if let Some(c) = p.action_ink {
        look.action_ink = c.color();
    }
    if let Some(c) = p.frame_stroke {
        look.frame_stroke = c.color();
    }
    if let Some(c) = p.btn_shadow {
        look.btn_shadow = c.color();
    }
    if let Some(c) = p.active_top {
        look.active_top = c.color();
    }
    if let Some(c) = p.active_bot {
        look.active_bot = c.color();
    }
    if let Some(c) = p.active_border {
        look.active_border = c.color();
    }
    if let Some(c) = p.active_shadow {
        look.active_shadow = c.color();
    }
    if let Some(c) = p.slot_bg {
        look.slot_bg = c.color();
    }
    if let Some(c) = p.slot_border {
        look.slot_border = c.color();
    }
    if let Some(c) = p.bar_start {
        look.bar_start = c.color();
    }
    if let Some(c) = p.bar_end {
        look.bar_end = c.color();
    }
}

fn apply_backdrop_patch(look: &mut Look, p: &BackdropPatch) {
    if let Some(k) = p.kind {
        look.pattern = k.into();
    }
    if let Some(c) = p.color {
        look.pattern_color = c.color();
    }
    if let Some(s) = p.strength {
        look.pattern_strength = s.clamp(0.0, 1.0);
    }
}

fn parse_manifest(text: &str, label: &str) -> Result<Manifest, String> {
    let m: Manifest = toml::from_str(text).map_err(|e| format!("{label}: {}", e.message()))?;
    if m.schema != SCHEMA_VERSION {
        return Err(format!("{label}: unsupported schema {} (expected {SCHEMA_VERSION})", m.schema));
    }
    if m.id.trim().is_empty() || m.name.trim().is_empty() {
        return Err(format!("{label}: id and name are required"));
    }
    Ok(m)
}

/// Find `theme.toml` inside a ZIP archive (either at root or inside a single top-level folder).
fn find_zip_manifest(bytes: &[u8]) -> Result<(String, Manifest), String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("invalid zip: {e}"))?;
    let mut found: Option<(String, String)> = None;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| format!("zip entry {i}: {e}"))?;
        let name = entry.name().replace('\\', "/");
        if name == MANIFEST_FILE {
            found = Some((String::new(), name));
            break;
        }
        if let Some((prefix, rest)) = name.split_once('/')
            && rest == MANIFEST_FILE
            && !prefix.is_empty()
            && !prefix.contains("..")
        {
            found = Some((prefix.to_string(), name));
        }
    }
    let (prefix, entry_name) = found.ok_or_else(|| format!("{MANIFEST_FILE} not found in archive"))?;
    let entry = archive.by_name(&entry_name).map_err(|e| format!("reading {entry_name}: {e}"))?;
    if entry.size() as usize > MAX_MANIFEST_BYTES {
        return Err(format!("{entry_name} exceeds {MAX_MANIFEST_BYTES} bytes"));
    }
    let mut text = String::new();
    entry.take((MAX_MANIFEST_BYTES + 1) as u64).read_to_string(&mut text).map_err(|e| format!("reading {entry_name}: {e}"))?;
    let manifest = parse_manifest(&text, &entry_name)?;
    Ok((prefix, manifest))
}

fn decode_image(bytes: &[u8], rel: &str) -> Result<Image, String> {
    let lower = rel.to_ascii_lowercase();
    if lower.ends_with(".svg") {
        return Image::load_from_svg_data(bytes).map_err(|e| format!("svg {rel}: {e}"));
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
        let mut reader = decoder.read_info().map_err(|e| format!("png {rel}: {e}"))?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| format!("png frame {rel}: {e}"))?;
        let raw = &buf[..info.buffer_size()];
        let rgba: Vec<u8> = match info.color_type {
            png::ColorType::Rgba => raw.to_vec(),
            png::ColorType::Rgb => raw.chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            png::ColorType::GrayscaleAlpha => raw.chunks_exact(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect(),
            png::ColorType::Grayscale => raw.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            other => return Err(format!("unsupported png color type {other:?} in {rel}")),
        };
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&rgba, info.width, info.height);
        return Ok(Image::from_rgba8(buffer));
    }
    Err(format!("unsupported image format {rel} (use .png or .svg)"))
}

fn register_theme_font(source: &Source, rel: &str) -> Result<String, String> {
    let key = source.cache_key(rel);
    let cache = REGISTERED_FONTS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(map) = cache.lock()
        && let Some(family) = map.get(&key)
    {
        return Ok(family.clone());
    }
    let bytes = source.read_asset(rel, MAX_ASSET_BYTES)?;
    use slint::fontique_011::fontique;
    let mut collection = slint::fontique_011::shared_collection();
    let blob = fontique::Blob::new(Arc::new(bytes));
    let registered = collection.register_fonts(blob, None);
    let (family_id, _) = registered.into_iter().next().ok_or_else(|| format!("font {rel} has no valid font faces"))?;
    let family_name = collection
        .family_name(family_id)
        .ok_or_else(|| format!("font {rel} has no family name"))?
        .to_string();
    if let Ok(mut map) = cache.lock() {
        map.insert(key, family_name.clone());
    }
    Ok(family_name)
}

/// Discover all available themes (built-ins first, then user themes in `dir`).
pub fn discover_in(dir: &Path, pet_name: &str, terminal_name: &str) -> Vec<ThemeSummary> {
    let mut list = vec![
        ThemeSummary { id: DEFAULT_THEME.into(), name: pet_name.into(), author: "EXESSR Hub".into(), builtin: true },
        ThemeSummary { id: TERMINAL_THEME.into(), name: terminal_name.into(), author: "saito".into(), builtin: true },
    ];
    let mut seen: HashSet<String> = [DEFAULT_THEME.to_string(), TERMINAL_THEME.to_string()].into_iter().collect();
    for (manifest, _) in scan_user_themes(dir) {
        if seen.insert(manifest.id.clone()) {
            list.push(ThemeSummary { id: manifest.id, name: manifest.name, author: manifest.author, builtin: false });
        }
    }
    list
}

fn scan_user_themes(dir: &Path) -> Vec<(Manifest, Source)> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            let manifest_path = path.join(MANIFEST_FILE);
            if manifest_path.is_file() {
                match saito_files::read_to_string(&manifest_path).map_err(|e| e.to_string()).and_then(|t| parse_manifest(&t, &manifest_path.display().to_string())) {
                    Ok(m) => found.push((m, Source::Dir(path))),
                    Err(why) => crate::errors::log("reading theme folder", &why),
                }
            }
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION)) {
            match saito_files::read(&path).map_err(|e| e.to_string()).and_then(|b| find_zip_manifest(&b)) {
                Ok((prefix, m)) => found.push((m, Source::Zip { path, prefix })),
                Err(why) => crate::errors::log("reading theme archive", &why),
            }
        }
    }
    found
}

/// Resolve theme `id` against `terminal_games` (content's `look.toml` per game) and user themes in `dir`.
/// Unknown or broken themes fall back cleanly to `pet-hd`.
pub fn resolve_in(dir: &Path, id: &str, terminal_games: &HashMap<String, Look>) -> ResolvedTheme {
    if id == TERMINAL_THEME {
        return build_terminal(terminal_games);
    }
    if id == DEFAULT_THEME || id.is_empty() {
        return build_pet_hd();
    }
    for (manifest, source) in scan_user_themes(dir) {
        if manifest.id == id {
            return resolve_manifest(&manifest, &source, terminal_games);
        }
    }
    build_pet_hd()
}

fn build_pet_hd() -> ResolvedTheme {
    let mut games = HashMap::new();
    for g in ["exe3", "exe4", "exe5", "exe6"] {
        games.insert(g.to_string(), pet_hd_look(Some(g)));
    }
    ResolvedTheme {
        id: DEFAULT_THEME.into(),
        pet: true,
        default_look: pet_hd_look(None),
        games,
        assets: ThemeAssets::default(),
    }
}

fn build_terminal(terminal_games: &HashMap<String, Look>) -> ResolvedTheme {
    ResolvedTheme {
        id: TERMINAL_THEME.into(),
        pet: false,
        default_look: terminal_default_look(),
        games: terminal_games.clone(),
        assets: ThemeAssets::default(),
    }
}

fn resolve_manifest(m: &Manifest, source: &Source, terminal_games: &HashMap<String, Look>) -> ResolvedTheme {
    let mut base = if m.extends == TERMINAL_THEME { build_terminal(terminal_games) } else { build_pet_hd() };
    base.id = m.id.clone();
    if let Some(kind) = m.style.kind {
        base.pet = matches!(kind, StyleKind::Pet);
    }
    apply_colors_patch(&mut base.default_look, &m.colors);
    apply_backdrop_patch(&mut base.default_look, &m.backdrop);
    for look in base.games.values_mut() {
        apply_colors_patch(look, &m.colors);
        apply_backdrop_patch(look, &m.backdrop);
    }
    for (game, patch) in &m.games {
        let entry = base.games.entry(game.clone()).or_insert_with(|| base.default_look.clone());
        apply_colors_patch(entry, &patch.colors);
        apply_backdrop_patch(entry, &patch.backdrop);
    }
    if let Some(rel) = m.backdrop.image.as_deref().filter(|s| !s.trim().is_empty()) {
        match source.read_asset(rel, MAX_ASSET_BYTES).and_then(|b| decode_image(&b, rel)) {
            Ok(img) => {
                base.assets.backdrop_image = Some(img);
                base.assets.backdrop_image_opacity = m.backdrop.image_opacity.unwrap_or(0.35).clamp(0.0, 1.0);
            }
            Err(why) => crate::errors::log("loading theme backdrop image", &why),
        }
    }
    for (slot, rel) in [
        (&mut base.assets.display_font, m.fonts.display.as_deref()),
        (&mut base.assets.body_font, m.fonts.body.as_deref()),
        (&mut base.assets.mono_font, m.fonts.mono.as_deref()),
    ] {
        if let Some(rel) = rel.filter(|s| !s.trim().is_empty()) {
            match register_theme_font(source, rel) {
                Ok(family) => *slot = Some(family),
                Err(why) => crate::errors::log("registering theme font", &why),
            }
        }
    }
    base
}

/// Validate and import a `.saitotheme` archive into `dest_dir`, returning its theme ID.
pub fn import_archive_into(source_file: &Path, dest_dir: &Path) -> Result<String, String> {
    let bytes = saito_files::read(source_file).map_err(|e| format!("reading {}: {e}", source_file.display()))?;
    let (_prefix, manifest) = find_zip_manifest(&bytes)?;
    if manifest.id == DEFAULT_THEME || manifest.id == TERMINAL_THEME {
        return Err(format!("theme id {:?} is reserved", manifest.id));
    }
    saito_files::create_dir_all(dest_dir).map_err(|e| format!("creating {}: {e}", dest_dir.display()))?;
    let file_name = format!("{}.{EXTENSION}", sanitize_id(&manifest.id));
    let target = dest_dir.join(file_name);
    saito_files::write(&target, &bytes).map_err(|e| format!("writing {}: {e}", target.display()))?;
    Ok(manifest.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_hex_and_int_colors_and_inherits_from_pet_hd() {
        let dir = tempfile::tempdir().unwrap();
        let theme_dir = dir.path().join("my-pet");
        std::fs::create_dir_all(&theme_dir).unwrap();
        std::fs::write(
            theme_dir.join(MANIFEST_FILE),
            r##"
schema = 1
id = "crimson-pet"
name = "Crimson PET"
author = "Tester"
extends = "pet-hd"

[colors]
line = "#FF4466"
cursor = 0xFF4466

[games.exe4.colors]
ground_top = "#441166"
"##,
        )
        .unwrap();

        let listed = discover_in(dir.path(), "HD PET", "Terminal");
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[2].id, "crimson-pet");

        let resolved = resolve_in(dir.path(), "crimson-pet", &HashMap::new());
        assert!(resolved.pet);
        assert_eq!(resolved.default_look.line, Color::from_rgb_u8(0xFF, 0x44, 0x66));
        assert_eq!(resolved.games["exe4"].ground_top, Color::from_rgb_u8(0x44, 0x11, 0x66));
        // Inherited exe5 ground_top stays PET-HD emerald, while top-level line override applies.
        assert_eq!(resolved.games["exe5"].ground_top, Color::from_rgb_u8(0x0B, 0x4A, 0x5E));
        assert_eq!(resolved.games["exe5"].line, Color::from_rgb_u8(0xFF, 0x44, 0x66));
    }

    #[test]
    fn imports_and_resolves_zip_saitotheme_and_rejects_traversal() {
        assert!(sanitize_rel_path("../secret.png").is_err());
        assert!(sanitize_rel_path("/etc/passwd").is_err());
        assert_eq!(sanitize_rel_path("assets\\bg.png").unwrap(), "assets/bg.png");

        let src_dir = tempfile::tempdir().unwrap();
        let dest_dir = tempfile::tempdir().unwrap();
        let archive_path = src_dir.path().join("demo.saitotheme");
        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zw = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            zw.start_file("theme.toml", opts).unwrap();
            zw.write_all(
                br##"
schema = 1
id = "zip-pet"
name = "Zip PET"
extends = "terminal"

[style]
kind = "pet"

[colors]
slot_border = "#123456"
"##,
            )
            .unwrap();
            zw.finish().unwrap();
        }
        let id = import_archive_into(&archive_path, dest_dir.path()).unwrap();
        assert_eq!(id, "zip-pet");
        let resolved = resolve_in(dest_dir.path(), "zip-pet", &HashMap::new());
        assert!(resolved.pet);
        assert_eq!(resolved.default_look.slot_border, Color::from_rgb_u8(0x12, 0x34, 0x56));
    }

    #[test]
    fn unknown_theme_falls_back_to_pet_hd() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_in(dir.path(), "does-not-exist", &HashMap::new());
        assert_eq!(resolved.id, DEFAULT_THEME);
        assert!(resolved.pet);
        assert_eq!(resolved.games["exe3"].ground_bottom, Color::from_rgb_u8(0x09, 0x09, 0x0B));
    }
}
