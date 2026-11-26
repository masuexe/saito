//! Official EXE4 / EXE5 Dark Soul Auto Battle Data (ABD) behavior modes:
//!
//! - [`AbdMode::Original`]: Original GBA EXE4 / EXE5 rules (default, unchanged).
//! - [`AbdMode::LegacyCollection`]: Mega Man Battle Network Legacy Collection (BNLC)
//!   ABD chip-pool rules (EXE4 & EXE5):
//!   1. Dark Chips cannot occupy top priority slots:
//!      - If fewer than 3 non-Dark Standard chips exist alongside Dark Chips (including
//!        EXE4 `DrkVulcn` in the Mega list), the top 3 slots are padded in order with
//!        `Cannon`, `MiniBomb`, `AirShot`, placing the first Dark Chip at rank 4.
//!      - If 3 non-Dark Standard chips exist, they occupy ranks 1–3 and the first Dark
//!        Chip sits at rank 4.
//!      - If 4 or more non-Dark Standard chips exist, any Dark Chip in ranks 1–4 is
//!        swapped down with a non-Dark Standard chip (so Dark Chips only occupy 1-copy
//!        slots at ranks 5–16).
//!   2. High-class chip unlock thresholds based on unique Standard/Dark chip count:
//!      - Mega chips: 1/2/3/4/5 unlocked at 3/6/10/14/16 unique Standard/Dark chips.
//!      - Giga chip & Program Advance: unlocked at 12 unique Standard/Dark chips.
//! - [`AbdMode::DoubleTeamDs`]: Mega Man Battle Network 5: Double Team DS (5DS, EXE5 only):
//!   1. Battle AI (`effects::ABD_DS` in `content/exe5/rules/auto_battle/ai.luau`): after
//!      using any Dark Chip, SearchMan (V1/SP/DS), ShakPar (1–3), or BigNoise, the AI
//!      must execute 8 three-shot buster runs before attempting another chip.
//!   2. ABD chip pool is sorted by internal BattleChip ID ascending (instead of usage count)
//!      before applying the 4/4/2/2/1×12 repeat weights.

use crate::facts::Stated;
use crate::{Side, ids};
use saito_battle::content::{ChipFlags, Content};
use saito_battle::rules::Fact;
use saito_content_api::{ChipHandle, Registry, Value};
use std::collections::HashMap;

/// Dark Soul Auto Battle Data (ABD) behavior mode for EXE4 and EXE5.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AbdMode {
    /// Original GBA behavior (default).
    #[default]
    Original = 0,
    /// Mega Man Battle Network Legacy Collection (EXE4 & EXE5).
    LegacyCollection = 1,
    /// Mega Man Battle Network 5: Double Team DS (EXE5 only).
    DoubleTeamDs = 2,
}

impl AbdMode {
    pub const EXE4: [AbdMode; 2] = [AbdMode::Original, AbdMode::LegacyCollection];
    pub const EXE5: [AbdMode; 3] = [AbdMode::Original, AbdMode::LegacyCollection, AbdMode::DoubleTeamDs];

    /// Whether `game` exposes the Dark Soul ABD mode option (EXE4 and EXE5).
    pub fn supported(game: &str) -> bool {
        matches!(game, "exe4" | "exe5")
    }

    /// Modes available for `game` (empty for games without ABD).
    pub fn available(game: &str) -> &'static [AbdMode] {
        match game {
            "exe4" => &Self::EXE4,
            "exe5" => &Self::EXE5,
            _ => &[],
        }
    }

    /// Whether `self` is legal for `game` (`Original` is always legal).
    pub fn allowed_for(self, game: &str) -> bool {
        self == AbdMode::Original || Self::available(game).contains(&self)
    }

    pub fn from_u8(v: u8) -> Option<AbdMode> {
        match v {
            0 => Some(AbdMode::Original),
            1 => Some(AbdMode::LegacyCollection),
            2 => Some(AbdMode::DoubleTeamDs),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Stable string key for serialization (`"gba"`, `"lc"`, `"ds"`).
    pub fn key(self) -> &'static str {
        match self {
            AbdMode::Original => "gba",
            AbdMode::LegacyCollection => "lc",
            AbdMode::DoubleTeamDs => "ds",
        }
    }

    /// Parse a serialized mode key (`"gba"`, `"lc"`, `"ds"`).
    pub fn from_key(key: &str) -> Option<AbdMode> {
        match key {
            "gba" => Some(AbdMode::Original),
            "lc" => Some(AbdMode::LegacyCollection),
            "ds" => Some(AbdMode::DoubleTeamDs),
            _ => None,
        }
    }

    /// Cycle to the next available mode for `game`.
    pub fn next_for(self, game: &str) -> AbdMode {
        let modes = Self::available(game);
        if modes.is_empty() {
            return AbdMode::Original;
        }
        let pos = modes.iter().position(|&m| m == self).unwrap_or(0);
        modes[(pos + 1) % modes.len()]
    }
}

/// Whether `game` supports the Dark Soul ABD mode option (EXE4 and EXE5).
pub fn supported(game: &str) -> bool {
    AbdMode::supported(game)
}

/// Modes available for `game` (empty for games without ABD).
pub fn available(game: &str) -> &'static [AbdMode] {
    AbdMode::available(game)
}

/// Default per-round hard cap on repeated uses of the same Mega/Giga/Dark/PA chip by ABD when enabled.
pub const DEFAULT_CHIP_CAP: u8 = 5;
/// Minimum allowed per-round ABD high-class chip cap.
pub const MIN_CHIP_CAP: u8 = 1;
/// Maximum allowed per-round ABD high-class chip cap (fits in 4 bits of `effects` and wire tag).
pub const MAX_CHIP_CAP: u8 = 15;

/// Whether `cap` is valid for `game` (`None` is always valid; `Some(1..=15)` only for EXE4/EXE5).
pub fn chip_cap_allowed(game: &str, cap: Option<u8>) -> bool {
    match cap {
        None => true,
        Some(n) => supported(game) && (MIN_CHIP_CAP..=MAX_CHIP_CAP).contains(&n),
    }
}

const PLACES_FIELD: &str = "auto_battle_places";
const PLACES: usize = 42;
const FIRST_RANGE: std::ops::Range<usize> = 0..3;
const STD_RANGE: std::ops::Range<usize> = 3..27;
const MEGA_RANGE: std::ops::Range<usize> = 27..32;
const GIGA_SLOT: usize = 32;
const PA_SLOT: usize = 41;

/// Repeat count table for top 16 Standard/Dark chips in slots 4..=27 (GBA `0x0800F1F8` / `0x0802C790`).
const STD_WEIGHTS: [usize; 16] = [4, 4, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1];

/// Legacy Collection placeholder chips used when fewer than 3 non-Dark Standard chips are registered.
const LC_PADDING_KEYS: [&str; 3] = ["cannon", "minibomb", "airshot"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Place {
    #[default]
    Empty,
    Zero,
    Chip(ChipHandle),
    Pattern(u8),
}

fn read_places(content: &Content, side: &Side) -> Option<[Place; PLACES]> {
    let Stated::List(items) = side.facts.get(content, PLACES_FIELD)? else {
        return None;
    };
    let mut out = [Place::Empty; PLACES];
    for (slot, item) in out.iter_mut().zip(&items) {
        let Stated::Record(fields) = item else { continue };
        let field = |name: &str| fields.iter().find(|(n, _)| n == name).map(|(_, v)| v);
        *slot = match field("chip") {
            Some(Stated::Def(Registry::Chip, Some(h))) => Place::Chip(ChipHandle(*h)),
            _ => match field("pattern") {
                Some(Stated::Number(n)) if *n > 0 => Place::Pattern(*n as u8),
                _ if matches!(field("zero"), Some(Stated::Flag(true))) => Place::Zero,
                _ => Place::Empty,
            },
        };
    }
    Some(out)
}

fn write_places(content: &Content, side: &mut Side, places: &[Place; PLACES]) {
    let last = places.iter().rposition(|p| *p != Place::Empty).map_or(0, |i| i + 1);
    let facts: Vec<Fact> = places[..last]
        .iter()
        .map(|p| {
            Fact::Record(match *p {
                Place::Empty => Vec::new(),
                Place::Zero => vec![("zero", Fact::Value(Value::Bool(true)))],
                Place::Chip(c) => vec![("chip", Fact::Value(Value::Def(Registry::Chip, c.0)))],
                Place::Pattern(n) => vec![("pattern", Fact::Value(Value::Int(n as i64)))],
            })
        })
        .collect();
    let _ = side.set_fact(content, PLACES_FIELD, &facts);
}

fn is_dark(content: &Content, chip: ChipHandle) -> bool {
    content.chip(chip).flags.has(ChipFlags::DARK)
}

/// Extract ordered unique chips from `slots`, ranked by `(copy_count desc, first_slot asc)`.
fn ranked_unique(slots: &[Place]) -> Vec<ChipHandle> {
    let mut counts: HashMap<ChipHandle, (usize, usize)> = HashMap::new();
    for (idx, p) in slots.iter().enumerate() {
        if let Place::Chip(c) = *p {
            counts
                .entry(c)
                .and_modify(|(n, _)| *n += 1)
                .or_insert((1, idx));
        }
    }
    let mut list: Vec<(ChipHandle, usize, usize)> = counts.into_iter().map(|(c, (n, first))| (c, n, first)).collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.2.cmp(&b.2)));
    list.into_iter().map(|(c, _, _)| c).collect()
}

fn fill_weighted_standard(places: &mut [Place; PLACES], ranked: &[ChipHandle]) {
    for p in &mut places[STD_RANGE] {
        *p = Place::Empty;
    }
    let mut slot = STD_RANGE.start;
    for (rank, &chip) in ranked.iter().take(STD_WEIGHTS.len()).enumerate() {
        for _ in 0..STD_WEIGHTS[rank] {
            if slot < STD_RANGE.end {
                places[slot] = Place::Chip(chip);
                slot += 1;
            }
        }
    }
}

fn transform_legacy_collection(content: &Content, game: &str, places: &mut [Place; PLACES]) {
    let mut ordered = ranked_unique(&places[STD_RANGE]);
    ordered.truncate(STD_WEIGHTS.len());
    let std_unique_count = ordered.len();

    let non_dark: Vec<ChipHandle> = ordered.iter().copied().filter(|&c| !is_dark(content, c)).collect();
    let dark: Vec<ChipHandle> = ordered.iter().copied().filter(|&c| is_dark(content, c)).collect();
    let has_mega_dark = game == "exe4"
        && places[MEGA_RANGE]
            .iter()
            .any(|p| matches!(*p, Place::Chip(c) if is_dark(content, c)));
    let dark_in_top = ordered.iter().take(4).any(|&c| is_dark(content, c)) || has_mega_dark;

    let final_std: Vec<ChipHandle> = if dark_in_top {
        if non_dark.len() < 3 {
            let mut ranked = non_dark.clone();
            let blanks = 3 - ranked.len();
            for key in LC_PADDING_KEYS.iter().take(blanks) {
                if let Some(chip) = ids::chip(content, game, key) {
                    ranked.push(chip);
                }
            }
            ranked.extend(dark.iter().copied());
            ranked.truncate(STD_WEIGHTS.len());
            ranked
        } else if non_dark.len() == 3 {
            let mut ranked = non_dark.clone();
            ranked.extend(dark.iter().copied());
            ranked.truncate(STD_WEIGHTS.len());
            ranked
        } else {
            let mut ranked = ordered.clone();
            for i in 0..4.min(ranked.len()) {
                if is_dark(content, ranked[i])
                    && let Some(j) = (4..ranked.len()).find(|&k| !is_dark(content, ranked[k]))
                {
                    ranked.swap(i, j);
                }
            }
            ranked
        }
    } else {
        ordered
    };

    if !final_std.is_empty() {
        fill_weighted_standard(places, &final_std);
    }

    // Keep `first` (slots 0..3) free of Dark Chips in Legacy Collection mode.
    for (idx, p) in places[FIRST_RANGE].iter_mut().enumerate() {
        if let Place::Chip(c) = *p
            && is_dark(content, c)
        {
            *p = final_std
                .get(idx)
                .copied()
                .filter(|&rep| !is_dark(content, rep))
                .or_else(|| LC_PADDING_KEYS.get(idx).and_then(|k| ids::chip(content, game, k)))
                .map_or(Place::Empty, Place::Chip);
        }
    }

    // High-class chip unlock thresholds (Mega: 3/6/10/14/16; Giga & PA: 12).
    let allowed_mega = match std_unique_count {
        16.. => 5,
        14..=15 => 4,
        10..=13 => 3,
        6..=9 => 2,
        3..=5 => 1,
        _ => 0,
    };
    let mut kept_mega = 0usize;
    for p in &mut places[MEGA_RANGE] {
        if matches!(*p, Place::Chip(_)) {
            if kept_mega < allowed_mega {
                kept_mega += 1;
            } else {
                *p = Place::Empty;
            }
        }
    }
    if std_unique_count < 12 {
        places[GIGA_SLOT] = Place::Empty;
        places[PA_SLOT] = Place::Empty;
    }
}

/// EXE5 internal BattleChip ID ordering (`0x001`..`0x15D`, matching `content/exe5/compat/chips.toml`).
const EXE5_INTERNAL_ORDER: &[&str] = &[
    "cannon", "hicannon", "m-cannon", "airshot", "airhoc", "boomer", "silence", "tornado",
    "widesht1", "widesht2", "widesht3", "mrkcan1", "mrkcan2", "mrkcan3", "vulcan1", "vulcan2",
    "vulcan3", "spreader", "thunder", "iceseed", "pulsar1", "pulsar2", "pulsar3", "spshake1",
    "spshake2", "spshake3", "static", "lifesync", "minibomb", "energbom", "megenbom", "gundels1",
    "gundels2", "gundels3", "quake1", "quake2", "quake3", "crakbom", "parabom", "resetbom",
    "bugbomb", "grasseed", "lavaseed", "cannball", "geyser", "blkbomb", "seaseed", "sword",
    "wideswrd", "longswrd", "wideblde", "longblde", "windrack", "custswrd", "varswrd", "slasher",
    "moonbld1", "moonbld2", "moonbld3", "katana1", "katana2", "katana3", "tankcan1", "tankcan2",
    "tankcan3", "redfrut1", "redfrut2", "redfrut3", "skully1", "skully2", "skully3", "drilarm1",
    "drilarm2", "drilarm3", "timebom1", "timebom2", "timebom3", "voltz1", "voltz2", "voltz3",
    "lance", "yoyo", "wind", "fan", "boybomb1", "boybomb2", "boybomb3", "guard1",
    "guard2", "guard3", "crakout", "dublcrak", "tripcrak", "magnum", "metagel", "snake",
    "circgun", "mine", "rockcube", "fanfare", "discord", "timpani", "vdoll", "astroid1",
    "astroid2", "astroid3", "recov10", "recov30", "recov50", "recov80", "recov120", "recov150",
    "recov200", "recov300", "busterup", "panlgrab", "areagrab", "grabrvng", "grabbnsh", "slogauge",
    "fstgauge", "pnlretrn", "geddon1", "geddon2", "geddon3", "rainyday", "colorpt", "elemrage",
    "blinder", "airspin1", "airspin2", "airspin3", "invisibl", "bblwrap", "barrier", "barr100",
    "barr200", "nrthwind", "holypanl", "antifire", "antiwatr", "antielec", "antiwood", "antinavi",
    "antidmg", "antiswrd", "antirecv", "copydmg", "attck-10", "navi-20", "firehit1", "firehit2",
    "firehit3", "hotbody1", "hotbody2", "hotbody3", "aqwhirl1", "aqwhirl2", "aqwhirl3", "sidebub1",
    "sidebub2", "sidebub3", "elcreel1", "elcreel2", "elcreel3", "cusvolt1", "cusvolt2", "cusvolt3",
    "crsshld1", "crsshld2", "crsshld3", "wavepit", "redwave", "mudwave", "cactbal1", "cactbal2",
    "cactbal3", "woodnos1", "woodnos2", "woodnos3", "darkcirc", "drksword", "darkinvs", "darkplus",
    "drklance", "darkwide", "darkthnd", "drkrecov", "darkmetr", "darkdril", "darktorn", "drksonic",
    "lifeaur", "muramasa", "guardian", "anubis", "attck-30", "bugfix", "dblpoint", "snctuary",
    "fullcust", "meteors", "numbrbl", "jealousy", "poltrgst", "blakwing", "otenko", "justcone",
    "neovari", "z-saver", "gundelex", "suprvulc", "roll", "roll-sp", "roll-ds", "gyroman",
    "gyroman-sp", "gyroman-ds", "napalmmn", "naplmmn-sp", "naplmmn-ds", "serchman", "serchmn-sp", "serchmn-ds",
    "magnetmn", "magntmn-sp", "magntmn-ds", "meddy", "meddy-sp", "meddy-ds", "protoman", "protomn-sp",
    "protomn-ds", "numbrman", "numbrmn-sp", "numbrmn-ds", "colonel", "colonel-sp", "colonel-ds", "shadoman",
    "shadomn-sp", "shadomn-ds", "tmhwkman", "tmhwkmn-sp", "tmhwkmn-ds", "knightmn", "knigtmn-sp", "knigtmn-ds",
    "toadman", "toadman-sp", "toadman-ds", "shademan", "shademn-sp", "shademn-ds", "blizman", "blizman-sp",
    "blizman-ds", "cloudman", "cloudmn-sp", "cloudmn-ds", "cosmoman", "cosmomn-sp", "cosmomn-ds", "larkman",
    "larkman-sp", "larkman-ds", "gridman", "gridman-sp", "gridman-ds", "django", "django-sp", "django-ds",
    "cannmode", "cannball-mode", "swrdmode", "yoyomode", "drilmode", "lcrsshld", "lstepswd", "lcounter",
    "elempowr", "finalgun", "bass", "deltaray", "bugcurse", "metrknuk", "omegarkt", "bassanly",
    "holydrem", "bighook", "crossdiv", "bugcharg", "leadraid", "chaoslrd", "phoenix", "dethphnx",
    "batcan1", "batcan2", "batcan3", "batcan4", "gigacan1", "gigacan2", "gigacan3", "infvulc1",
    "infvulc2", "infvulc3", "boxer1", "boxer2", "boxer3", "suprspr1", "suprspr2", "suprspr3",
    "shakpar1", "shakpar2", "shakpar3", "cacdanc1", "cacdanc2", "cacdanc3", "h-burst", "lifesrd",
    "greatyo", "pithoky", "poisphar", "csmopris", "wildbird", "football", "bignoise", "timebom",
    "bodygrd", "piledrvr",
];

fn exe5_internal_rank(content: &Content, chip: ChipHandle) -> (usize, u16) {
    let key = ids::local(&content.defs.chip(chip).key);
    let pos = EXE5_INTERNAL_ORDER.iter().position(|&k| k == key).unwrap_or(usize::MAX);
    (pos, chip.0)
}

fn transform_double_team_ds(content: &Content, places: &mut [Place; PLACES]) {
    let mut std_chips = ranked_unique(&places[STD_RANGE]);
    std_chips.sort_by_key(|&c| exe5_internal_rank(content, c));
    std_chips.truncate(STD_WEIGHTS.len());
    if !std_chips.is_empty() {
        fill_weighted_standard(places, &std_chips);
    }

    let mut first_chips: Vec<ChipHandle> = places[FIRST_RANGE]
        .iter()
        .filter_map(|p| match *p {
            Place::Chip(c) => Some(c),
            _ => None,
        })
        .collect();
    first_chips.sort_by_key(|&c| exe5_internal_rank(content, c));
    for (slot, p) in places[FIRST_RANGE].iter_mut().enumerate() {
        *p = first_chips.get(slot).copied().map_or(Place::Empty, Place::Chip);
    }

    let mut mega_chips: Vec<ChipHandle> = places[MEGA_RANGE]
        .iter()
        .filter_map(|p| match *p {
            Place::Chip(c) => Some(c),
            _ => None,
        })
        .collect();
    mega_chips.sort_by_key(|&c| exe5_internal_rank(content, c));
    for (slot, p) in places[MEGA_RANGE].iter_mut().enumerate() {
        *p = mega_chips.get(slot).copied().map_or(Place::Empty, Place::Chip);
    }
}

/// Transform `side`'s `auto_battle_places` according to `mode` for round setup.
/// Leaves `side` untouched under [`AbdMode::Original`] or unsupported games.
pub fn transform_side(content: &Content, game: &str, mode: AbdMode, side: &Side) -> Side {
    if mode == AbdMode::Original || !AbdMode::supported(game) {
        return side.clone();
    }
    let Some(mut places) = read_places(content, side) else {
        return side.clone();
    };
    if places.iter().all(|p| *p == Place::Empty) {
        return side.clone();
    }
    match mode {
        AbdMode::Original => return side.clone(),
        AbdMode::LegacyCollection => transform_legacy_collection(content, game, &mut places),
        AbdMode::DoubleTeamDs if game == "exe5" => transform_double_team_ds(content, &mut places),
        AbdMode::DoubleTeamDs => return side.clone(),
    }
    let mut out = side.clone();
    write_places(content, &mut out, &places);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{exe4_content, exe5_content};

    fn make_abd_side(content: &Content, game: &str, std_chips: &[&str], mega: &[&str], giga: Option<&str>, pa: Option<&str>) -> Side {
        let mut side = Side::fresh(content, game).unwrap();
        let mut places = [Place::Empty; PLACES];
        let mut slot = STD_RANGE.start;
        for (rank, key) in std_chips.iter().take(STD_WEIGHTS.len()).enumerate() {
            let c = ids::chip(content, game, key).unwrap_or_else(|| panic!("missing chip {key} in {game}"));
            for _ in 0..STD_WEIGHTS[rank] {
                places[slot] = Place::Chip(c);
                slot += 1;
            }
        }
        for (i, key) in mega.iter().take(5).enumerate() {
            let c = ids::chip(content, game, key).unwrap_or_else(|| panic!("missing mega {key} in {game}"));
            places[MEGA_RANGE.start + i] = Place::Chip(c);
        }
        if let Some(key) = giga {
            places[GIGA_SLOT] = Place::Chip(ids::chip(content, game, key).unwrap());
        }
        if let Some(key) = pa {
            places[PA_SLOT] = Place::Chip(ids::chip(content, game, key).unwrap());
        }
        write_places(content, &mut side, &places);
        side
    }

    #[test]
    fn legacy_collection_pads_dark_only_abd_and_gates_mega_giga_pa() {
        for (game, content, dark, mega, giga, pa) in [
            ("exe4", exe4_content(), "drksword", "roll", "redsun", "lifesrd"),
            ("exe5", exe5_content(), "darkthnd", "roll", "bass", "bignoise"),
        ] {
            let side = make_abd_side(&content, game, &[dark], &[mega], Some(giga), Some(pa));
            let lc = transform_side(&content, game, AbdMode::LegacyCollection, &side);
            let places = read_places(&content, &lc).unwrap();
            let cannon = ids::chip(&content, game, "cannon").unwrap();
            let minibomb = ids::chip(&content, game, "minibomb").unwrap();
            let airshot = ids::chip(&content, game, "airshot").unwrap();
            let dark_h = ids::chip(&content, game, dark).unwrap();
            // Ranks 1-3 padded with Cannon (4), MiniBomb (4), AirShot (2); Dark placed at rank 4 (2).
            assert_eq!(&places[3..7], &[Place::Chip(cannon); 4], "{game}");
            assert_eq!(&places[7..11], &[Place::Chip(minibomb); 4], "{game}");
            assert_eq!(&places[11..13], &[Place::Chip(airshot); 2], "{game}");
            assert_eq!(&places[13..15], &[Place::Chip(dark_h); 2], "{game}");
            // Only 1 unique standard/dark chip -> 0 Mega, 0 Giga, 0 PA.
            assert!(places[MEGA_RANGE].iter().all(|p| *p == Place::Empty), "{game}");
            assert_eq!(places[GIGA_SLOT], Place::Empty, "{game}");
            assert_eq!(places[PA_SLOT], Place::Empty, "{game}");
        }
    }

    #[test]
    fn legacy_collection_swaps_dark_below_top4_and_unlocks_at_12_standards() {
        let content = exe5_content();
        let stds = [
            "darkthnd", "drksword", "sword", "wideswrd", "longswrd", "areagrab",
            "panlgrab", "recov300", "recov200", "invisibl", "barrier", "barr100",
        ];
        let megas = ["roll", "protoman", "serchman", "gyroman", "anubis"];
        let side = make_abd_side(&content, "exe5", &stds, &megas, Some("bass"), Some("bignoise"));
        let lc = transform_side(&content, "exe5", AbdMode::LegacyCollection, &side);
        let places = read_places(&content, &lc).unwrap();
        // Top 4 ranks (slots 3..15) contain zero Dark Chips.
        for p in &places[3..15] {
            if let Place::Chip(c) = *p {
                assert!(!is_dark(&content, c), "top 4 must not contain dark chip");
            }
        }
        // DarkThnd and DrkSword still appear at 1-copy slots (15..27).
        let darkthnd = ids::chip(&content, "exe5", "darkthnd").unwrap();
        let drksword = ids::chip(&content, "exe5", "drksword").unwrap();
        assert_eq!(places[15..27].iter().filter(|&&p| p == Place::Chip(darkthnd)).count(), 1);
        assert_eq!(places[15..27].iter().filter(|&&p| p == Place::Chip(drksword)).count(), 1);
        // 12 unique standard/dark chips -> 3 Mega (10..13), 1 Giga (>=12), 1 PA (>=12).
        let kept_megas = places[MEGA_RANGE].iter().filter(|p| matches!(p, Place::Chip(_))).count();
        assert_eq!(kept_megas, 3);
        assert!(matches!(places[GIGA_SLOT], Place::Chip(_)));
        assert!(matches!(places[PA_SLOT], Place::Chip(_)));
    }

    #[test]
    fn exe5_ds_mode_sorts_by_internal_id_and_sets_abd_ds_effect_bit() {
        let content = exe5_content();
        // Put sword (0x030), airshot (0x004), cannon (0x001) in reverse order.
        let side = make_abd_side(&content, "exe5", &["sword", "airshot", "cannon"], &["protoman", "roll"], Some("bass"), Some("bignoise"));
        let ds = transform_side(&content, "exe5", AbdMode::DoubleTeamDs, &side);
        let places = read_places(&content, &ds).unwrap();
        let cannon = ids::chip(&content, "exe5", "cannon").unwrap();
        let airshot = ids::chip(&content, "exe5", "airshot").unwrap();
        let sword = ids::chip(&content, "exe5", "sword").unwrap();
        let roll = ids::chip(&content, "exe5", "roll").unwrap();
        let protoman = ids::chip(&content, "exe5", "protoman").unwrap();
        assert_eq!(&places[3..7], &[Place::Chip(cannon); 4]);
        assert_eq!(&places[7..11], &[Place::Chip(airshot); 4]);
        assert_eq!(&places[11..13], &[Place::Chip(sword); 2]);
        assert_eq!(&places[MEGA_RANGE.start..MEGA_RANGE.start + 2], &[Place::Chip(roll), Place::Chip(protoman)]);

        // Match round sets effects::ABD_DS and binary roundtrip preserves abd_mode.
        let mut m = crate::pick::live(&content, "exe5", 7, None).unwrap();
        m.abd_mode = AbdMode::DoubleTeamDs;
        let round = m.round(&content, 7);
        assert_ne!(round.settings.effects & saito_battle::setup::effects::ABD_DS, 0);
        let bytes = crate::binary::match_bytes(&content, &m).unwrap();
        let back = crate::binary::read_match(&content, &bytes).unwrap();
        assert_eq!(back.abd_mode, AbdMode::DoubleTeamDs);
    }

    #[test]
    fn exe5_internal_order_entries_all_resolve() {
        let content = exe5_content();
        for &key in EXE5_INTERNAL_ORDER {
            assert!(ids::chip(&content, "exe5", key).is_some(), "missing exe5 chip {key}");
        }
    }

    #[test]
    fn exe5_ds_cooldown_forces_seven_buster_runs_after_dark_chip() {
        use saito_battle::Battle;
        use saito_battle::custom::{FolderChip, SavedFolder};
        use saito_battle::input::{PlayerTick, keys};
        use saito_content_api::FieldValue;

        let content = exe5_content();
        let mut base = crate::pick::live(&content, "exe5", 7, None).unwrap();
        let darkinvs = ids::chip(&content, "exe5", "darkinvs").unwrap();
        let cannon = ids::chip(&content, "exe5", "cannon").unwrap();
        let darkthnd = ids::chip(&content, "exe5", "darkthnd").unwrap();
        let code_of = |c: ChipHandle| content.defs.chip(c).record.codes[0];
        let mut chips = [FolderChip::new(cannon, code_of(cannon)); 30];
        chips[0] = FolderChip::new(darkinvs, code_of(darkinvs));
        let folder = SavedFolder { chips, regular: Some(0), tags: None };
        base.sides[0].set_folder(&content, &folder.into()).unwrap();
        base.sides[0].set_fact(&content, "karma", &[Fact::Value(Value::Int(0))]).unwrap();
        base.sides[0].set_fact(&content, "hp", &[Fact::Value(Value::Int(997))]).unwrap();
        let mut places = [Place::Empty; PLACES];
        for slot in 3..7 {
            places[slot] = Place::Chip(darkthnd);
        }
        write_places(&content, &mut base.sides[0], &places);

        let run_mode = |mode: AbdMode| -> (u8, std::collections::BTreeSet<u8>) {
            let mut m = base.clone();
            m.abd_mode = mode;
            let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
            // Advance through custom screen so side 0 picks its regular chip (darkinvs).
            let mut last = [None; 2];
            for _ in 0..3000 {
                if b.round.mode == saito_battle::battle::mode::FIGHTING && b.fight.state == saito_battle::battle::fight::FIGHTING {
                    break;
                }
                let input = [0, 1].map(|side| {
                    last[side] = saito_battle::scenario::custom_choice(&b, side, last[side]);
                    PlayerTick::buttons(0).with_choice(last[side])
                });
                b.tick(&input, Default::default());
            }
            assert_eq!(b.fight.state, saito_battle::battle::fight::FIGHTING);
            // Press A on side 0 to activate DarkInvs (hands side 0 to auto battle AI for 600 ticks).
            b.tick(&[PlayerTick::buttons(keys::A), PlayerTick::buttons(0)], Default::default());

            let mut max_cd = 0u8;
            let mut cooldown_levels = std::collections::BTreeSet::new();
            for _ in 0..560 {
                b.tick(&[PlayerTick::buttons(0), PlayerTick::buttons(0)], Default::default());
                let (schema, state) = b.rules_state(0).unwrap();
                let shots_place = schema.place(schema.index_of("drive").unwrap()).field("shots").unwrap();
                if let FieldValue::U8(s) = state.get_at(shots_place) {
                    let cd = s >> 4;
                    max_cd = max_cd.max(cd);
                    if cd > 0 {
                        cooldown_levels.insert(cd);
                    }
                }
            }
            (max_cd, cooldown_levels)
        };

        let (orig_max, orig_cd) = run_mode(AbdMode::Original);
        let (ds_max, ds_cd) = run_mode(AbdMode::DoubleTeamDs);
        assert!(orig_max == 0 && orig_cd.is_empty(), "Original must never enter 5DS cooldown: max={orig_max}");
        assert_eq!(ds_max, 8, "5DS ROM sets chip[+0x1E] = 8 on picking DarkThnd: levels={ds_cd:?}");
        assert!(ds_cd.contains(&8) && ds_cd.contains(&7) && ds_cd.len() >= 5, "5DS must decrement cooldown across buster runs: levels={ds_cd:?}");
    }

    #[test]
    fn exe5_abd_chip_cap_tracks_and_hard_caps_dark_chips_and_roundtrips_binary() {
        use saito_battle::Battle;
        use saito_battle::custom::{FolderChip, SavedFolder};
        use saito_battle::input::{PlayerTick, keys};

        let content = exe5_content();
        let mut base = crate::pick::live(&content, "exe5", 7, None).unwrap();
        let darkinvs = ids::chip(&content, "exe5", "darkinvs").unwrap();
        let cannon = ids::chip(&content, "exe5", "cannon").unwrap();
        let darkthnd = ids::chip(&content, "exe5", "darkthnd").unwrap();
        let code_of = |c: ChipHandle| content.defs.chip(c).record.codes[0];
        let mut chips = [FolderChip::new(cannon, code_of(cannon)); 30];
        chips[0] = FolderChip::new(darkinvs, code_of(darkinvs));
        let folder = SavedFolder { chips, regular: Some(0), tags: None };
        base.sides[0].set_folder(&content, &folder.into()).unwrap();
        base.sides[0].set_fact(&content, "karma", &[Fact::Value(Value::Int(0))]).unwrap();
        base.sides[0].set_fact(&content, "hp", &[Fact::Value(Value::Int(997))]).unwrap();
        let mut places = [Place::Empty; PLACES];
        for slot in 3..7 {
            places[slot] = Place::Chip(darkthnd);
        }
        write_places(&content, &mut base.sides[0], &places);

        // Binary roundtrip & validation.
        let mut capped = base.clone();
        capped.abd_chip_cap = Some(2);
        let bytes = crate::binary::match_bytes(&content, &capped).unwrap();
        let back = crate::binary::read_match(&content, &bytes).unwrap();
        assert_eq!(back.abd_chip_cap, Some(2));
        let round = capped.round(&content, 7);
        assert_eq!(saito_battle::setup::effects::abd_chip_cap(round.settings.effects), 2);

        let run_with_cap = |cap: Option<u8>| -> u16 {
            let mut m = base.clone();
            m.abd_chip_cap = cap;
            let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
            let mut last = [None; 2];
            for _ in 0..3000 {
                if b.round.mode == saito_battle::battle::mode::FIGHTING && b.fight.state == saito_battle::battle::fight::FIGHTING {
                    break;
                }
                let input = [0, 1].map(|side| {
                    last[side] = saito_battle::scenario::custom_choice(&b, side, last[side]);
                    PlayerTick::buttons(0).with_choice(last[side])
                });
                b.tick(&input, Default::default());
            }
            b.tick(&[PlayerTick::buttons(keys::A), PlayerTick::buttons(0)], Default::default());
            for _ in 0..580 {
                b.tick(&[PlayerTick::buttons(0), PlayerTick::buttons(0)], Default::default());
            }
            b.abd_chip_uses_of(0, darkthnd)
        };

        let unlimited_uses = run_with_cap(None);
        let capped_uses = run_with_cap(Some(2));
        assert!(unlimited_uses > 2, "unlimited DarkInvs should use DarkThnd more than twice (got {unlimited_uses})");
        assert_eq!(capped_uses, 2, "hard cap = 2 must stop DarkThnd at exactly 2 uses");
    }
}
