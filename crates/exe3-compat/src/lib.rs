//! Compatibility with EXE3's original games (White and Blue/Black, US and
//! Japanese): their numbers for EXE3's content (`content/exe3/compat`), and
//! what reads them.
//!
//! - [`Compat`]: the tables mapping content keys to the original games'
//!   numbers — chips (all 385 entries with codes, element, power, MB, rarity,
//!   class, flags, and White/Blue exclusivity), assets, text encodings (US and
//!   JP), rules, NetBattle stages, ROM shifts, object kinds, navi actions,
//!   record numbers (styles/elements/navis/barriers), 11 panel types, and 50
//!   NaviCust programs (204 parts).
//! - [`codec`]: EXE3's binary records in the engine's terms — the `0x40`-byte
//!   NaviStats, 11 panel types, `0x24`-byte chip hand block, and `0x100`-byte
//!   link record.
//! - [`save`]: EXE3's unencrypted `0x57B0`-byte save image, `"ROCKMANEXE3 20021002"`
//!   signature, White (`+0x16`) / Blue (`+0x22`) checksum verification,
//!   4-byte folder entries, Regular chip, Style, NaviCust list/grid, and event
//!   flags.
//! - [`setup`]: translating save/link NaviCust parts and Style into engine
//!   setup facts.
//! - [`import`]: importing an EXE3 `.sav` file into a [`saito_match::Side`].
//! - [`trace`]: parsing and replaying EXE3 mGBA differential traces against
//!   [`saito_battle::Battle`].

#![forbid(unsafe_code)]

pub mod codec;
pub mod import;
pub mod save;
pub mod setup;
pub mod trace;

use saito_battle::field::PanelType;
use saito_content_api::Pool;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// EXE3's game content root (`content/exe3`).
pub const ROOT: &str = "exe3";

/// An EXE3 game version (`white` = White US `A6BE` / White JP `A6BJ`,
/// `blue` = Blue US `A3XE` / Black JP `A3XJ`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Version {
    White,
    Blue,
}

impl Version {
    /// Canonical lowercase version name (`white`, `blue`).
    pub fn name(self) -> &'static str {
        match self {
            Version::White => "white",
            Version::Blue => "blue",
        }
    }

    /// Parse a version name (`white`, `blue`, or Japanese alias `black`).
    pub fn named(name: &str) -> Option<Version> {
        match name {
            "white" => Some(Version::White),
            "blue" | "black" => Some(Version::Blue),
            _ => None,
        }
    }
}

/// EXE3's 8 Style types (upper 5 bits of the Style byte, `style_byte >> 3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum StyleType {
    Normal = 0,
    Guts = 1,
    Custom = 2,
    Team = 3,
    Shield = 4,
    Ground = 5,
    Shadow = 6,
    Bug = 7,
}

impl StyleType {
    /// Convert from raw 3-bit style type number (`0..=7`).
    pub fn from_u8(v: u8) -> Option<StyleType> {
        match v {
            0 => Some(StyleType::Normal),
            1 => Some(StyleType::Guts),
            2 => Some(StyleType::Custom),
            3 => Some(StyleType::Team),
            4 => Some(StyleType::Shield),
            5 => Some(StyleType::Ground),
            6 => Some(StyleType::Shadow),
            7 => Some(StyleType::Bug),
            _ => None,
        }
    }

    /// Canonical lowercase name (`normal`, `guts`, `custom`, `team`, `shield`, `ground`, `shadow`, `bug`).
    pub fn name(self) -> &'static str {
        match self {
            StyleType::Normal => "normal",
            StyleType::Guts => "guts",
            StyleType::Custom => "custom",
            StyleType::Team => "team",
            StyleType::Shield => "shield",
            StyleType::Ground => "ground",
            StyleType::Shadow => "shadow",
            StyleType::Bug => "bug",
        }
    }

    /// Version exclusivity of a StyleType, if any: Ground is White-only,
    /// Shadow is Blue/Black-only.
    pub fn exclusive_version(self) -> Option<Version> {
        match self {
            StyleType::Ground => Some(Version::White),
            StyleType::Shadow => Some(Version::Blue),
            _ => None,
        }
    }
}

/// EXE3's 5 elements (low 3 bits of the Style byte `style_byte & 7`, and chip
/// record `+0x06`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Element {
    Null = 0,
    Elec = 1,
    Heat = 2,
    Aqua = 3,
    Wood = 4,
}

impl Element {
    /// Convert from raw element number (`0..=4`).
    pub fn from_u8(v: u8) -> Option<Element> {
        match v {
            0 => Some(Element::Null),
            1 => Some(Element::Elec),
            2 => Some(Element::Heat),
            3 => Some(Element::Aqua),
            4 => Some(Element::Wood),
            _ => None,
        }
    }

    /// Canonical lowercase name (`null`, `elec`, `heat`, `aqua`, `wood`).
    pub fn name(self) -> &'static str {
        match self {
            Element::Null => "null",
            Element::Elec => "elec",
            Element::Heat => "heat",
            Element::Aqua => "aqua",
            Element::Wood => "wood",
        }
    }
}

/// An EXE3 Style: `StyleType × Element`, packed into a single byte as
/// `(style_type << 3) | element` (verified at `0x080072B8` & `0x0804736C`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Style {
    pub kind: StyleType,
    pub element: Element,
}

impl Style {
    pub const NORMAL: Style = Style { kind: StyleType::Normal, element: Element::Null };

    /// Pack into EXE3's byte representation `(kind << 3) | element`.
    pub const fn pack(self) -> u8 {
        ((self.kind as u8) << 3) | (self.element as u8)
    }

    /// Unpack from EXE3's byte representation `(kind << 3) | element`.
    pub fn unpack(byte: u8) -> Option<Style> {
        let kind = StyleType::from_u8(byte >> 3)?;
        let element = Element::from_u8(byte & 0x07)?;
        Some(Style { kind, element })
    }
}

/// EXE3's object pools: how many slots each has (`docs/design/exe3-map.md` §3:
/// 8 actors @ `0x0800369C`, 32 attacks @ `0x08003980`, 32 effects @ `0x08003EBC`).
pub fn pool_slots(pool: Pool) -> usize {
    match pool {
        Pool::Actor => 8,
        Pool::Attack => 32,
        Pool::Effect => 32,
    }
}

/// The pool of an EXE3 object type number (`1` actor, `3` attack, `4` effect).
pub fn pool_of_type(t: u8) -> Option<Pool> {
    match t {
        1 => Some(Pool::Actor),
        3 => Some(Pool::Attack),
        4 => Some(Pool::Effect),
        _ => None,
    }
}

/// The EXE3 object type number of a pool.
pub fn type_of_pool(p: Pool) -> u8 {
    match p {
        Pool::Actor => 1,
        Pool::Attack => 3,
        Pool::Effect => 4,
    }
}

/// A chip's class in EXE3 (`standard`, `mega`, `giga`, `pa`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChipClass {
    #[default]
    Standard,
    Mega,
    Giga,
    Pa,
}

/// A chip entry (`chips.toml`): extracted from EXE3's 385 × `0x20`-byte chip
/// table (`0x08011510` US / `0x08011474` JP).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChipEntry {
    pub id: u16,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub codes: Vec<String>,
    #[serde(default)]
    pub element: String,
    #[serde(default)]
    pub power: u16,
    #[serde(default)]
    pub mb: u8,
    #[serde(default)]
    pub rarity: u8,
    #[serde(default)]
    pub class: ChipClass,
    #[serde(default)]
    pub family: u8,
    #[serde(default)]
    pub subfamily: u8,
    #[serde(default)]
    pub library: u16,
    #[serde(default)]
    pub flags: u8,
    /// Present for version-exclusive Giga chips (`white`: 304..=308;
    /// `blue`: 302, 303, 309..=311).
    #[serde(default)]
    pub version: Option<Version>,
}

impl ChipEntry {
    /// Whether this chip is legal in a folder of `version` (non-exclusive chips
    /// are legal in both versions; exclusive Giga chips only in their version).
    pub fn is_legal_in(&self, version: Version) -> bool {
        self.version.map_or(true, |v| v == version)
    }
}

/// EXE3's asset names (`assets.toml`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetNames {
    #[serde(default)]
    pub sprites: BTreeMap<String, String>,
    #[serde(default)]
    pub sounds: BTreeMap<String, u16>,
    #[serde(default)]
    pub backgrounds: BTreeMap<String, u8>,
    #[serde(default)]
    pub banners: BTreeMap<String, u8>,
}

/// EXE3's text encodings (`text.toml`): US (`glyphs`: 0x00..0x6F) and JP
/// (`jp.glyphs`: 0x00..0xE3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub first_control: u8,
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub dialogue_glyphs: Vec<String>,
    #[serde(default)]
    pub jp: Encoding,
}

/// The Japanese ROMs' encoding (`Text::jp`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encoding {
    pub glyphs: Vec<String>,
    #[serde(default)]
    pub dialogue_glyphs: Vec<String>,
}

/// An EXE3 panel type (`panels.toml`): 11 panel types (`0..=10`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelEntry {
    pub name: String,
    pub flags: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PanelsFile {
    types: BTreeMap<String, PanelEntry>,
}

/// EXE3's battle status byte numbers (`rules.toml`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleNumbers {
    #[serde(default)]
    pub statuses: BTreeMap<String, u8>,
}

/// One of EXE3's 8 NetBattle stages (`stages.toml`, `0x0803EAB0`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageEntry {
    pub index: u8,
    pub background: u8,
    pub name: String,
}

/// An object kind (`kinds.toml`): EXE3's pool and index.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindEntry {
    pub pool: String,
    pub index: u8,
}

/// Address shifts of a ROM relative to White US (`A6BE`) in `games.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameAddresses {
    pub settings: i32,
    #[serde(default)]
    pub code_upper: i32,
}

/// `games.toml`: the other three Legacy Collection Vol. 1 EXE3 ROMs' shifts.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Games {
    pub blue: GameAddresses,
    #[serde(rename = "jp-white")]
    pub jp_white: GameAddresses,
    #[serde(rename = "jp-black")]
    pub jp_black: GameAddresses,
}

impl Games {
    /// Shifts for `(version, japanese)`; `None` for White US (`A6BE`, the base).
    pub fn of(&self, version: Version, japanese: bool) -> Option<&GameAddresses> {
        match (version, japanese) {
            (Version::White, false) => None,
            (Version::Blue, false) => Some(&self.blue),
            (Version::White, true) => Some(&self.jp_white),
            (Version::Blue, true) => Some(&self.jp_black),
        }
    }
}

/// What EXE3's records name by number (`records.toml`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordNumbers {
    #[serde(default)]
    pub navis: BTreeMap<String, u8>,
    #[serde(default)]
    pub styles: BTreeMap<String, u8>,
    #[serde(default)]
    pub elements: BTreeMap<String, u8>,
    #[serde(default)]
    pub barriers: BTreeMap<String, u8>,
}

/// EXE3's NaviCust programs (`navicust.toml`): each program's number (`id >> 2`),
/// its colored variants (`id & 3`), and their color names.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviCustNumbers {
    #[serde(default)]
    pub programs: BTreeMap<String, u8>,
    #[serde(default)]
    pub variants: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    pub colors: BTreeMap<String, Vec<String>>,
}

/// EXE3's compat tables (`content/exe3/compat`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Compat {
    pub chips: BTreeMap<String, ChipEntry>,
    pub chip_keys: BTreeMap<u16, String>,
    pub assets: AssetNames,
    pub text: Text,
    pub rules: RuleNumbers,
    pub stages: BTreeMap<String, StageEntry>,
    pub games: Games,
    pub kinds: BTreeMap<String, KindEntry>,
    pub actions: BTreeMap<String, u8>,
    pub records: RecordNumbers,
    pub panels: BTreeMap<u8, PanelEntry>,
    pub navicust: NaviCustNumbers,
}

/// The 11 files of `content/exe3/compat/`.
pub const FILES: [&str; 11] = [
    "chips.toml",
    "assets.toml",
    "text.toml",
    "rules.toml",
    "stages.toml",
    "games.toml",
    "kinds.toml",
    "actions.toml",
    "records.toml",
    "panels.toml",
    "navicust.toml",
];

#[cfg(feature = "builtin")]
const EXE3: [(&str, &str); 11] = [
    ("chips.toml", include_str!("../../../content/exe3/compat/chips.toml")),
    ("assets.toml", include_str!("../../../content/exe3/compat/assets.toml")),
    ("text.toml", include_str!("../../../content/exe3/compat/text.toml")),
    ("rules.toml", include_str!("../../../content/exe3/compat/rules.toml")),
    ("stages.toml", include_str!("../../../content/exe3/compat/stages.toml")),
    ("games.toml", include_str!("../../../content/exe3/compat/games.toml")),
    ("kinds.toml", include_str!("../../../content/exe3/compat/kinds.toml")),
    ("actions.toml", include_str!("../../../content/exe3/compat/actions.toml")),
    ("records.toml", include_str!("../../../content/exe3/compat/records.toml")),
    ("panels.toml", include_str!("../../../content/exe3/compat/panels.toml")),
    ("navicust.toml", include_str!("../../../content/exe3/compat/navicust.toml")),
];

fn parse_sprite(id: &str) -> Option<(u8, u8)> {
    let (c, i) = id.split_once('-')?;
    Some((u8::from_str_radix(c, 16).ok()?, u8::from_str_radix(i, 16).ok()?))
}

fn parse_pool(name: &str) -> Option<Pool> {
    match name {
        "actor" => Some(Pool::Actor),
        "attack" => Some(Pool::Attack),
        "effect" => Some(Pool::Effect),
        _ => None,
    }
}

impl Compat {
    /// EXE3's compat compiled into the crate (requires `builtin` feature).
    #[cfg(feature = "builtin")]
    pub fn exe3() -> &'static Compat {
        static EXE3_COMPAT: std::sync::OnceLock<Compat> = std::sync::OnceLock::new();
        EXE3_COMPAT.get_or_init(|| {
            Compat::parse(|file| {
                Ok(EXE3
                    .iter()
                    .find(|(f, _)| *f == file)
                    .map(|(_, t)| (*t).to_string())
                    .unwrap_or_default())
            })
            .unwrap_or_else(|e| panic!("content/exe3/compat: {e}"))
        })
    }

    /// Read `content/exe3/compat` under a content root directory.
    pub fn of_content(content: &Path) -> Result<Compat, String> {
        Compat::read(&content.join(ROOT).join("compat"))
    }

    /// Read all compat TOML files from `dir`.
    pub fn read(dir: &Path) -> Result<Compat, String> {
        Compat::parse(|file| {
            let path = dir.join(file);
            saito_files::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))
        })
    }

    fn parse(text: impl Fn(&str) -> Result<String, String>) -> Result<Compat, String> {
        let chips: BTreeMap<String, ChipEntry> =
            toml::from_str(&text("chips.toml")?).map_err(|e| format!("chips.toml: {e}"))?;
        let mut chip_keys = BTreeMap::new();
        for (k, c) in &chips {
            if let Some(other) = chip_keys.insert(c.id, k.clone()) {
                return Err(format!("chips.toml: {k} and {other} are both chip {:#05x}", c.id));
            }
        }
        let assets: AssetNames =
            toml::from_str(&text("assets.toml")?).map_err(|e| format!("assets.toml: {e}"))?;
        for (name, id) in &assets.sprites {
            parse_sprite(id)
                .ok_or_else(|| format!("assets.toml: sprite {name} is {id:?}, not \"cc-ii\""))?;
        }
        let mut sounds = BTreeMap::new();
        for (name, &id) in &assets.sounds {
            if let Some(other) = sounds.insert(id, name) {
                return Err(format!("assets.toml: sounds {name} and {other} are both {id:#05x}"));
            }
        }
        let text_file: Text =
            toml::from_str(&text("text.toml")?).map_err(|e| format!("text.toml: {e}"))?;
        if text_file.glyphs.len() > text_file.first_control as usize
            || text_file.jp.glyphs.len() > text_file.first_control as usize
        {
            return Err(format!(
                "text.toml: more glyphs than bytes below first_control ({:#04x})",
                text_file.first_control
            ));
        }
        let rules: RuleNumbers =
            toml::from_str(&text("rules.toml")?).map_err(|e| format!("rules.toml: {e}"))?;
        let mut statuses = BTreeMap::new();
        for (k, &n) in &rules.statuses {
            if let Some(other) = statuses.insert(n, k) {
                return Err(format!("rules.toml: statuses {k} and {other} are both {n:#04x}"));
            }
        }
        let stages: BTreeMap<String, StageEntry> =
            toml::from_str(&text("stages.toml")?).map_err(|e| format!("stages.toml: {e}"))?;
        let games: Games =
            toml::from_str(&text("games.toml")?).map_err(|e| format!("games.toml: {e}"))?;
        let kinds: BTreeMap<String, KindEntry> =
            toml::from_str(&text("kinds.toml")?).map_err(|e| format!("kinds.toml: {e}"))?;
        for (k, e) in &kinds {
            parse_pool(&e.pool)
                .ok_or_else(|| format!("kinds.toml: {k}'s pool {:?} is no pool", e.pool))?;
        }
        let actions: BTreeMap<String, u8> =
            toml::from_str(&text("actions.toml")?).map_err(|e| format!("actions.toml: {e}"))?;
        let records: RecordNumbers =
            toml::from_str(&text("records.toml")?).map_err(|e| format!("records.toml: {e}"))?;
        let panels_file: PanelsFile =
            toml::from_str(&text("panels.toml")?).map_err(|e| format!("panels.toml: {e}"))?;
        let mut panels = BTreeMap::new();
        for (n, p) in panels_file.types {
            let n: u8 = n.parse().map_err(|_| format!("panels.toml: type {n:?} is no number"))?;
            panels.insert(n, p);
        }
        let navicust: NaviCustNumbers =
            toml::from_str(&text("navicust.toml")?).map_err(|e| format!("navicust.toml: {e}"))?;
        Ok(Compat {
            chips,
            chip_keys,
            assets,
            text: text_file,
            rules,
            stages,
            games,
            kinds,
            actions,
            records,
            panels,
            navicust,
        })
    }

    /// Decode a save's NaviCust part id (`4 * program + variant`) into
    /// `(program_key, color_index)`. Returns `Ok(None)` for empty part `0`.
    pub fn navicust_part(&self, id: u8) -> Result<Option<(&str, u8)>, String> {
        if id == 0 {
            return Ok(None);
        }
        let number = id >> 2;
        let key = self
            .navicust
            .programs
            .iter()
            .find(|(_, n)| **n == number)
            .map(|(k, _)| k.as_str())
            .ok_or_else(|| format!("navicust.toml has no program {number} (part id {id:#04x})"))?;
        let variants = self
            .navicust
            .variants
            .get(key)
            .ok_or_else(|| format!("navicust.toml has no variants for {key}"))?;
        let color = variants
            .iter()
            .position(|&v| v == id & 3)
            .ok_or_else(|| format!("{key} has no color in variant {} (part id {id:#04x})", id & 3))?;
        Ok(Some((key, color as u8)))
    }

    /// NaviCust program key by save part id (`4 * program + variant`).
    /// Returns `None` for empty part `0` or unknown part ids.
    pub fn navicust_key(&self, id: u8) -> Option<&str> {
        self.navicust_part(id).ok().flatten().map(|(k, _)| k)
    }

    /// Chip key (`cannon`) by chip table id (`0..=384`).
    pub fn chip_key(&self, id: u16) -> Option<&str> {
        self.chip_keys.get(&id).map(String::as_str)
    }

    /// Owned chip key by chip table id.
    pub fn chip(&self, id: u16) -> Option<String> {
        self.chip_key(id).map(String::from)
    }

    /// Chip entry by compat key.
    pub fn chip_entry(&self, key: &str) -> Option<&ChipEntry> {
        self.chips.get(key)
    }

    /// Chip entry by table id.
    pub fn chip_by_id(&self, id: u16) -> Option<&ChipEntry> {
        self.chip_key(id).and_then(|k| self.chips.get(k))
    }

    /// Whether chip table `id` is legal in `version`.
    pub fn is_legal_chip(&self, id: u16, version: Version) -> bool {
        self.chip_by_id(id).map_or(false, |c| c.is_legal_in(version))
    }

    /// EXE3 panel type `n` (`0..=10`).
    pub fn panel_type(&self, n: u8) -> Result<PanelType, String> {
        self.panels
            .get(&n)
            .map(|_| PanelType(n))
            .ok_or_else(|| format!("panels.toml has no type {n}"))
    }

    /// Panel type name for EXE3 number `n` (`0..=10`).
    pub fn panel_name(&self, n: u8) -> Result<&str, String> {
        self.panels
            .get(&n)
            .map(|p| p.name.as_str())
            .ok_or_else(|| format!("panels.toml has no type {n}"))
    }

    /// NetBattle stage key by its stage table index (`0..=7`).
    pub fn stage_by_index(&self, index: u8) -> Option<(&str, &StageEntry)> {
        self.stages.iter().find(|(_, e)| e.index == index).map(|(k, e)| (k.as_str(), e))
    }

    /// NetBattle stage key by its background id (`[0, 4, 5, 6, 23, 16, 2, 10]`).
    pub fn stage_by_background(&self, background: u8) -> Option<(&str, &StageEntry)> {
        self.stages.iter().find(|(_, e)| e.background == background).map(|(k, e)| (k.as_str(), e))
    }

    /// Navi key by number (`0` = `"megaman"`).
    pub fn navi_key(&self, n: u8) -> Option<&str> {
        self.records.navis.iter().find(|&(_, &v)| v == n).map(|(k, _)| k.as_str())
    }

    /// Style type name by number (`0..=7`).
    pub fn style_name(&self, n: u8) -> Option<&str> {
        self.records.styles.iter().find(|&(_, &v)| v == n).map(|(k, _)| k.as_str())
    }

    /// Element name by number (`0..=4`).
    pub fn element_name(&self, n: u8) -> Option<&str> {
        self.records.elements.iter().find(|&(_, &v)| v == n).map(|(k, _)| k.as_str())
    }

    /// Barrier key by number (`0` = `Ok(None)`, `8` = `Ok(Some("barrier/first-barrier"))`).
    pub fn barrier(&self, n: u8) -> Result<Option<String>, String> {
        if n == 0 {
            return Ok(None);
        }
        self.records
            .barriers
            .iter()
            .find(|&(_, &v)| v == n)
            .map(|(k, _)| Some(k.clone()))
            .ok_or_else(|| format!("barrier {n}"))
    }

    /// Status key by status byte.
    pub fn status(&self, byte: u8) -> Option<String> {
        self.rules.statuses.iter().find(|&(_, &n)| n == byte).map(|(k, _)| k.clone())
    }

    /// Object kind `(Pool, index)` by key.
    pub fn kind(&self, key: &str) -> Option<(Pool, u8)> {
        let e = self.kinds.get(key)?;
        Some((parse_pool(&e.pool)?, e.index))
    }

    /// EXE3's action number for object `o` in battle `b`: for non-actor
    /// objects this is `x.action`; for player actors, maps through
    /// `actions.toml` when present, falling back to `x.action`.
    pub fn navi_action(&self, b: &saito_battle::Battle, o: saito_battle::object::ObjectRef) -> u8 {
        use saito_battle::kinds::player::{NaviAction, navi_action};
        let x = b.objects.get(o);
        if x.actor.is_none() {
            return x.action;
        }
        let key = match navi_action(b, o) {
            NaviAction::Engine(e) => e.key(),
            NaviAction::Content(h) => b.content.defs.action(h).key.as_str(),
            _ => return x.action,
        };
        self.actions
            .get(key)
            .or_else(|| self.actions.get(saito_content_api::keys::local(key)))
            .copied()
            .unwrap_or(x.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_packs_and_unpacks_all_valid_combinations() {
        for k in 0u8..=7 {
            let kind = StyleType::from_u8(k).unwrap();
            for e in 0u8..=4 {
                let element = Element::from_u8(e).unwrap();
                let s = Style { kind, element };
                let packed = s.pack();
                assert_eq!(packed, (k << 3) | e);
                assert_eq!(Style::unpack(packed), Some(s));
            }
            // Invalid elements 5..=7 must fail unpack
            for bad_e in 5u8..=7 {
                assert_eq!(Style::unpack((k << 3) | bad_e), None);
            }
        }
        // Invalid style type >= 8 must fail unpack
        assert_eq!(Style::unpack(8 << 3), None);
        assert_eq!(StyleType::Ground.exclusive_version(), Some(Version::White));
        assert_eq!(StyleType::Shadow.exclusive_version(), Some(Version::Blue));
        assert_eq!(StyleType::Custom.exclusive_version(), None);
    }

    #[test]
    fn pool_slots_match_exe3() {
        assert_eq!(pool_slots(Pool::Actor), 8);
        assert_eq!(pool_slots(Pool::Attack), 32);
        assert_eq!(pool_slots(Pool::Effect), 32);
        for (n, p) in [(1, Pool::Actor), (3, Pool::Attack), (4, Pool::Effect)] {
            assert_eq!(pool_of_type(n), Some(p));
            assert_eq!(type_of_pool(p), n);
        }
        assert_eq!(pool_of_type(2), None);
    }

    #[test]
    fn builtin_compat_reads_and_validates_rom_tables() {
        let c = Compat::exe3();
        assert_eq!(c.chips.len(), 385);
        assert_eq!(c.chip_key(1), Some("cannon"));
        assert_eq!(c.chip_entry("cannon").map(|e| (e.power, e.mb)), Some((40, 12)));

        // Version-exclusive Giga chips: White 304..=308, Blue 302, 303, 309..=311
        assert_eq!(c.chip_entry("navrcycl").map(|e| (e.id, e.version)), Some((0x130, Some(Version::White))));
        assert_eq!(c.chip_entry("foldrbak").map(|e| (e.id, e.version)), Some((0x12F, Some(Version::Blue))));
        assert!(c.is_legal_chip(0x130, Version::White));
        assert!(!c.is_legal_chip(0x130, Version::Blue));
        assert!(c.is_legal_chip(0x12F, Version::Blue));
        assert!(!c.is_legal_chip(0x12F, Version::White));
        assert!(c.is_legal_chip(0x138, Version::White)); // BassGS
        assert!(c.is_legal_chip(0x138, Version::Blue));

        // Assets: Battle MegaMan is 00-00
        assert_eq!(c.assets.sprites.get("megaman").map(String::as_str), Some("00-00"));
        assert_eq!(c.assets.backgrounds.len(), 30);
        assert_eq!(c.assets.banners.len(), 13);

        // Text encodings: US 0x70, JP 0xE4
        assert_eq!((c.text.first_control, c.text.glyphs.len()), (0xE4, 0x70));
        assert_eq!(c.text.jp.glyphs.len(), 0xE4);
        assert_eq!(c.text.glyphs[0x25], "a"); // EXE3 US has 'a' at 0x25 (no '*')

        // 11 panel types (0..=10), type 10 is sand
        assert_eq!(c.panels.len(), 11);
        assert_eq!((c.panel_type(10), c.panel_name(10)), (Ok(PanelType(10)), Ok("sand")));
        assert!(c.panel_type(11).is_err());

        // 8 NetBattle stages: [0, 4, 5, 6, 23, 16, 2, 10]
        assert_eq!(c.stages.len(), 8);
        let expected_bgs = [0u8, 4, 5, 6, 23, 16, 2, 10];
        for (i, &bg) in expected_bgs.iter().enumerate() {
            let (_, st) = c.stage_by_index(i as u8).unwrap();
            assert_eq!(st.background, bg);
        }

        // NaviCust: 50 programs
        assert_eq!(c.navicust.programs.len(), 50);
        assert_eq!(c.navicust_part(0), Ok(None));
        assert_eq!(c.navicust_part(7), Ok(Some(("sprarmor", 0)))); // program 1, variant 3 (red)
        assert_eq!(c.navicust_part(41 * 4 + 2), Ok(Some(("atk-1", 2)))); // Atk+1, variant 2

        // Records & kinds
        assert_eq!(c.navi_key(0), Some("megaman"));
        assert_eq!(c.style_name(7), Some("bug"));
        assert_eq!(c.element_name(3), Some("aqua"));
        assert_eq!(c.barrier(8), Ok(Some("barrier/first-barrier".into())));
        assert_eq!(c.kind("engine/player"), Some((Pool::Actor, 0)));
    }

    #[test]
    fn compat_folder_reads_from_disk_matching_builtin() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/exe3/compat");
        assert_eq!(&Compat::read(&dir).unwrap(), Compat::exe3());
    }
}
