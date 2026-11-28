//! Complete GBA BGM track catalogs for EXE1, EXE2, EXE3, EXE4, EXE5, and EXE6 (`S-135`).
//!
//! Every track in each ROM's M4A music player (`PlayerId(31)`) is extracted into
//! `m4a::SoundBank` when the game loads (or on-demand from `RomSource` for
//! audio-only games EXE1 and EXE2). Entries are ordered to match the official
//! *Mega Man Battle Network Legacy Collection* (ROCKMAN EXE 合集) Music Player
//! tracklist, with ROM-only extras (reused earlier-game tracks, short fanfare
//! variants, and BN4's unused alternate battle track) appended at the end of
//! each game's list.

/// One GBA ROM BGM entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GbaBgmTrack {
    pub key: &'static str,
    pub song_id: u16,
    pub looping: bool,
}

/// Optional audio-only games extracted directly from `RomSource` (e.g. Legacy
/// Collection Vol. 1 `exe1.dat` / `exe2.dat` or ROM files) without requiring a
/// playable battle content pack. Candidate ROM codes are listed in preference
/// order (US → JP → EU).
pub const EXTRA_BGM_GAMES: &[(&str, &[&str])] = &[
    ("exe1", &["AREE", "AREJ", "AREP"]),
    ("exe2", &["AE2E", "AE2J", "AE2P"]),
];

/// Complete EXE1 (BN1) GBA BGM catalog (`0x000..=0x015`), ordered per Legacy Collection Vol. 1.
pub const EXE1_BGM_TRACKS: &[GbaBgmTrack] = &[
    GbaBgmTrack { key: "title-screen", song_id: 0x000, looping: true },
    GbaBgmTrack { key: "acdc-town", song_id: 0x001, looping: true },
    GbaBgmTrack { key: "school", song_id: 0x014, looping: true },
    GbaBgmTrack { key: "indoors", song_id: 0x002, looping: true },
    GbaBgmTrack { key: "suspicious-mood", song_id: 0x003, looping: true },
    GbaBgmTrack { key: "event-occurance", song_id: 0x004, looping: true },
    GbaBgmTrack { key: "transmission", song_id: 0x005, looping: false },
    GbaBgmTrack { key: "fire-field", song_id: 0x006, looping: true },
    GbaBgmTrack { key: "running-through-cyber-world", song_id: 0x007, looping: true },
    GbaBgmTrack { key: "boundless-network", song_id: 0x008, looping: true },
    GbaBgmTrack { key: "hour-of-fate", song_id: 0x009, looping: true },
    GbaBgmTrack { key: "cold-and-silent", song_id: 0x00A, looping: true },
    GbaBgmTrack { key: "red-or-blue", song_id: 0x00B, looping: true },
    GbaBgmTrack { key: "electrical-crisis", song_id: 0x00C, looping: true },
    GbaBgmTrack { key: "void", song_id: 0x00D, looping: true },
    GbaBgmTrack { key: "operation", song_id: 0x00E, looping: true },
    GbaBgmTrack { key: "net-battle", song_id: 0x00F, looping: true },
    GbaBgmTrack { key: "final-battle", song_id: 0x010, looping: true },
    GbaBgmTrack { key: "winner", song_id: 0x011, looping: true },
    GbaBgmTrack { key: "game-over", song_id: 0x012, looping: false },
    GbaBgmTrack { key: "loser", song_id: 0x015, looping: true },
    GbaBgmTrack { key: "credits", song_id: 0x013, looping: true },
];

/// Complete EXE2 (BN2) GBA BGM catalog (`0x000..=0x025`), ordered per Legacy Collection Vol. 1.
pub const EXE2_BGM_TRACKS: &[GbaBgmTrack] = &[
    GbaBgmTrack { key: "title-screen", song_id: 0x000, looping: true },
    GbaBgmTrack { key: "secret-maneuver", song_id: 0x001, looping: true },
    GbaBgmTrack { key: "acdc-town", song_id: 0x002, looping: true },
    GbaBgmTrack { key: "okuden-valley", song_id: 0x005, looping: true },
    GbaBgmTrack { key: "marine-harbor", song_id: 0x006, looping: true },
    GbaBgmTrack { key: "official-center", song_id: 0x007, looping: true },
    GbaBgmTrack { key: "going-on-a-journey", song_id: 0x008, looping: true },
    GbaBgmTrack { key: "foreign-town", song_id: 0x009, looping: true },
    GbaBgmTrack { key: "netopia-castle", song_id: 0x00A, looping: true },
    GbaBgmTrack { key: "alley", song_id: 0x023, looping: true },
    GbaBgmTrack { key: "dungeon", song_id: 0x024, looping: true },
    GbaBgmTrack { key: "sky-travel", song_id: 0x00B, looping: true },
    GbaBgmTrack { key: "determination", song_id: 0x00C, looping: true },
    GbaBgmTrack { key: "suspicious-mood", song_id: 0x00D, looping: true },
    GbaBgmTrack { key: "event-occurance", song_id: 0x00E, looping: true },
    GbaBgmTrack { key: "facing-sadness", song_id: 0x00F, looping: true },
    GbaBgmTrack { key: "proof-of-courage", song_id: 0x010, looping: true },
    GbaBgmTrack { key: "transmission", song_id: 0x011, looping: false },
    GbaBgmTrack { key: "smoky-field", song_id: 0x012, looping: true },
    GbaBgmTrack { key: "time-limit", song_id: 0x013, looping: true },
    GbaBgmTrack { key: "serious-mission", song_id: 0x014, looping: true },
    GbaBgmTrack { key: "fear-in-the-castle", song_id: 0x015, looping: true },
    GbaBgmTrack { key: "magnetic-airplane", song_id: 0x016, looping: true },
    GbaBgmTrack { key: "you-cant-go-back", song_id: 0x017, looping: true },
    GbaBgmTrack { key: "internet-world", song_id: 0x018, looping: true },
    GbaBgmTrack { key: "fearful", song_id: 0x019, looping: true },
    GbaBgmTrack { key: "know-the-truth", song_id: 0x01A, looping: true },
    GbaBgmTrack { key: "virus-battle", song_id: 0x01B, looping: true },
    GbaBgmTrack { key: "battle-spirit", song_id: 0x01C, looping: true },
    GbaBgmTrack { key: "final-battle", song_id: 0x01D, looping: true },
    GbaBgmTrack { key: "style-change", song_id: 0x01F, looping: true },
    GbaBgmTrack { key: "pressure", song_id: 0x022, looping: true },
    GbaBgmTrack { key: "credits", song_id: 0x025, looping: true },
];

/// Complete EXE3 (BN3) GBA BGM catalog (30 tracks), ordered per Legacy Collection Vol. 1.
/// Shared EXE1/EXE2 tracks (`school`, `indoors`, `winner`, `style-change`, `loser`, `game-over`)
/// are kept once under their originating game (EXE1 / EXE2) to avoid duplicates.
pub const EXE3_BGM_TRACKS: &[GbaBgmTrack] = &[
    GbaBgmTrack { key: "title-screen", song_id: 0x000, looping: true },
    GbaBgmTrack { key: "www-theme", song_id: 0x001, looping: true },
    GbaBgmTrack { key: "acdc-town", song_id: 0x002, looping: true },
    GbaBgmTrack { key: "netbattle-4", song_id: 0x005, looping: true },
    GbaBgmTrack { key: "yoka", song_id: 0x006, looping: true },
    GbaBgmTrack { key: "beach-street", song_id: 0x007, looping: true },
    GbaBgmTrack { key: "scilab", song_id: 0x008, looping: true },
    GbaBgmTrack { key: "tv-station", song_id: 0x009, looping: true },
    GbaBgmTrack { key: "n1-grand-prix", song_id: 0x00A, looping: true },
    GbaBgmTrack { key: "hades-isle", song_id: 0x00B, looping: true },
    GbaBgmTrack { key: "wily-lab", song_id: 0x024, looping: true },
    GbaBgmTrack { key: "suspicious-mood", song_id: 0x00C, looping: true },
    GbaBgmTrack { key: "event-occurance", song_id: 0x00D, looping: true },
    GbaBgmTrack { key: "wipe-your-tears", song_id: 0x00E, looping: true },
    GbaBgmTrack { key: "proof-of-courage", song_id: 0x00F, looping: true },
    GbaBgmTrack { key: "transmission", song_id: 0x010, looping: false },
    GbaBgmTrack { key: "blind-mode", song_id: 0x011, looping: true },
    GbaBgmTrack { key: "maze-of-wildness", song_id: 0x012, looping: true },
    GbaBgmTrack { key: "save-a-life", song_id: 0x013, looping: true },
    GbaBgmTrack { key: "final-transmission", song_id: 0x015, looping: true },
    GbaBgmTrack { key: "network", song_id: 0x016, looping: true },
    GbaBgmTrack { key: "undernet", song_id: 0x017, looping: true },
    GbaBgmTrack { key: "secret-area", song_id: 0x018, looping: true },
    GbaBgmTrack { key: "netbattle-1", song_id: 0x019, looping: true },
    GbaBgmTrack { key: "netbattle-2", song_id: 0x01A, looping: true },
    GbaBgmTrack { key: "final-battle", song_id: 0x01B, looping: true },
    GbaBgmTrack { key: "netbattle-3", song_id: 0x01C, looping: true },
    GbaBgmTrack { key: "boss-prelude", song_id: 0x022, looping: true },
    GbaBgmTrack { key: "navi-customizer", song_id: 0x025, looping: true },
    GbaBgmTrack { key: "credits", song_id: 0x023, looping: true },
];

/// Complete EXE4 (BN4) GBA BGM catalog (`0x000..=0x021`), ordered per Legacy Collection.
pub const EXE4_BGM_TRACKS: &[GbaBgmTrack] = &[
    GbaBgmTrack { key: "title-screen", song_id: 0x000, looping: true },
    GbaBgmTrack { key: "naxa", song_id: 0x008, looping: true },
    GbaBgmTrack { key: "acdc-town", song_id: 0x002, looping: true },
    GbaBgmTrack { key: "indoors", song_id: 0x003, looping: true },
    GbaBgmTrack { key: "elec-town", song_id: 0x004, looping: true },
    GbaBgmTrack { key: "crises", song_id: 0x00B, looping: true },
    GbaBgmTrack { key: "transmission", song_id: 0x00E, looping: false },
    GbaBgmTrack { key: "elec-tower-comp", song_id: 0x00F, looping: true },
    GbaBgmTrack { key: "virus-battle", song_id: 0x014, looping: true },
    GbaBgmTrack { key: "winner-special", song_id: 0x018, looping: true },
    GbaBgmTrack { key: "silent-darkness", song_id: 0x001, looping: true },
    GbaBgmTrack { key: "den-dome", song_id: 0x005, looping: true },
    GbaBgmTrack { key: "player-admission", song_id: 0x009, looping: true },
    GbaBgmTrack { key: "tournament-battle", song_id: 0x017, looping: true },
    GbaBgmTrack { key: "event-occurance", song_id: 0x01F, looping: true },
    GbaBgmTrack { key: "network", song_id: 0x012, looping: true },
    GbaBgmTrack { key: "castillo", song_id: 0x006, looping: true },
    GbaBgmTrack { key: "vampire-mansion", song_id: 0x00A, looping: true },
    GbaBgmTrack { key: "toyrobo-comp", song_id: 0x010, looping: true },
    GbaBgmTrack { key: "vampire", song_id: 0x020, looping: true },
    GbaBgmTrack { key: "boss-prelude", song_id: 0x01B, looping: true },
    GbaBgmTrack { key: "boss-battle", song_id: 0x015, looping: true },
    GbaBgmTrack { key: "navi-customizer", song_id: 0x01D, looping: true },
    GbaBgmTrack { key: "foreign-town", song_id: 0x007, looping: true },
    GbaBgmTrack { key: "undernet", song_id: 0x013, looping: true },
    GbaBgmTrack { key: "sad", song_id: 0x00C, looping: true },
    GbaBgmTrack { key: "heroism", song_id: 0x00D, looping: true },
    GbaBgmTrack { key: "asteroid-comp", song_id: 0x011, looping: true },
    GbaBgmTrack { key: "game-over", song_id: 0x01A, looping: false },
    GbaBgmTrack { key: "loser", song_id: 0x019, looping: true },
    GbaBgmTrack { key: "final-battle", song_id: 0x016, looping: true },
    GbaBgmTrack { key: "credits", song_id: 0x01C, looping: true },
    // ROM-only extras not listed separately in Legacy Collection Music Player:
    GbaBgmTrack { key: "winner", song_id: 0x01E, looping: false },
    GbaBgmTrack { key: "virus-battle-alt", song_id: 0x021, looping: true },
];

/// Complete EXE5 (BN5) GBA BGM catalog (`0x001..=0x009`, `0x00B..=0x024`), ordered per Legacy Collection.
pub const EXE5_BGM_TRACKS: &[GbaBgmTrack] = &[
    GbaBgmTrack { key: "title-screen", song_id: 0x001, looping: true },
    GbaBgmTrack { key: "nebula-theme", song_id: 0x002, looping: true },
    GbaBgmTrack { key: "acdc-town", song_id: 0x003, looping: true },
    GbaBgmTrack { key: "indoors", song_id: 0x004, looping: true },
    GbaBgmTrack { key: "scilab", song_id: 0x005, looping: true },
    GbaBgmTrack { key: "crises", song_id: 0x00C, looping: true },
    GbaBgmTrack { key: "transmission", song_id: 0x00F, looping: false },
    GbaBgmTrack { key: "main-comp", song_id: 0x010, looping: true },
    GbaBgmTrack { key: "virus-battle", song_id: 0x015, looping: true },
    GbaBgmTrack { key: "winner-0", song_id: 0x019, looping: true },
    GbaBgmTrack { key: "oran-island", song_id: 0x006, looping: true },
    GbaBgmTrack { key: "event-occurance", song_id: 0x00B, looping: true },
    GbaBgmTrack { key: "drill-comp", song_id: 0x011, looping: true },
    GbaBgmTrack { key: "network", song_id: 0x013, looping: true },
    GbaBgmTrack { key: "boss-prelude", song_id: 0x01C, looping: true },
    GbaBgmTrack { key: "liberate", song_id: 0x022, looping: true },
    GbaBgmTrack { key: "boss-battle", song_id: 0x016, looping: true },
    GbaBgmTrack { key: "liberate-boss-fanfare", song_id: 0x018, looping: false },
    GbaBgmTrack { key: "navi-customizer", song_id: 0x01E, looping: true },
    GbaBgmTrack { key: "queen-bohemia", song_id: 0x007, looping: true },
    GbaBgmTrack { key: "ship-comp", song_id: 0x020, looping: true },
    GbaBgmTrack { key: "end-city", song_id: 0x008, looping: true },
    GbaBgmTrack { key: "castle-comp", song_id: 0x021, looping: true },
    GbaBgmTrack { key: "undernet", song_id: 0x014, looping: true },
    GbaBgmTrack { key: "sad", song_id: 0x00D, looping: true },
    GbaBgmTrack { key: "you-are-not-alone", song_id: 0x00E, looping: true },
    GbaBgmTrack { key: "dark-chip-factory", song_id: 0x009, looping: true },
    GbaBgmTrack { key: "factory-comp", song_id: 0x012, looping: true },
    GbaBgmTrack { key: "trial", song_id: 0x023, looping: true },
    GbaBgmTrack { key: "1turn-delete", song_id: 0x024, looping: false },
    GbaBgmTrack { key: "game-over", song_id: 0x01B, looping: false },
    GbaBgmTrack { key: "loser", song_id: 0x01A, looping: true },
    GbaBgmTrack { key: "final-battle", song_id: 0x017, looping: true },
    GbaBgmTrack { key: "credits", song_id: 0x01D, looping: true },
    // ROM-only short fanfare variant:
    GbaBgmTrack { key: "winner-1", song_id: 0x01F, looping: false },
];

/// Complete EXE6 (BN6) GBA BGM catalog (`0x001..=0x025`), ordered per Legacy Collection.
pub const EXE6_BGM_TRACKS: &[GbaBgmTrack] = &[
    GbaBgmTrack { key: "title-screen", song_id: 0x001, looping: true },
    GbaBgmTrack { key: "www-theme", song_id: 0x002, looping: true },
    GbaBgmTrack { key: "cyber-city", song_id: 0x003, looping: true },
    GbaBgmTrack { key: "indoors", song_id: 0x004, looping: true },
    GbaBgmTrack { key: "school", song_id: 0x005, looping: true },
    GbaBgmTrack { key: "crises", song_id: 0x00C, looping: true },
    GbaBgmTrack { key: "transmission", song_id: 0x00F, looping: false },
    GbaBgmTrack { key: "robo-control-comp", song_id: 0x010, looping: true },
    GbaBgmTrack { key: "seaside-town", song_id: 0x006, looping: true },
    GbaBgmTrack { key: "aquarium-comp", song_id: 0x011, looping: true },
    GbaBgmTrack { key: "shark-chase", song_id: 0x023, looping: true },
    GbaBgmTrack { key: "virus-battle", song_id: 0x015, looping: true },
    GbaBgmTrack { key: "winner-0", song_id: 0x019, looping: true },
    GbaBgmTrack { key: "green-town", song_id: 0x008, looping: true },
    GbaBgmTrack { key: "event-occurance", song_id: 0x00B, looping: true },
    GbaBgmTrack { key: "judge-tree-comp", song_id: 0x012, looping: true },
    GbaBgmTrack { key: "boss-prelude", song_id: 0x01C, looping: true },
    GbaBgmTrack { key: "boss-battle", song_id: 0x016, looping: true },
    GbaBgmTrack { key: "network", song_id: 0x013, looping: true },
    GbaBgmTrack { key: "sky-town", song_id: 0x007, looping: true },
    GbaBgmTrack { key: "sad", song_id: 0x00D, looping: true },
    GbaBgmTrack { key: "mr-weather-comp", song_id: 0x00A, looping: true },
    GbaBgmTrack { key: "cyberbeasts", song_id: 0x021, looping: true },
    GbaBgmTrack { key: "acdc-town", song_id: 0x024, looping: true },
    GbaBgmTrack { key: "happy", song_id: 0x00E, looping: true },
    GbaBgmTrack { key: "undernet", song_id: 0x014, looping: true },
    GbaBgmTrack { key: "navi-customizer", song_id: 0x01E, looping: true },
    GbaBgmTrack { key: "expo", song_id: 0x025, looping: true },
    GbaBgmTrack { key: "graveyard-area", song_id: 0x009, looping: true },
    GbaBgmTrack { key: "crossover-battle", song_id: 0x022, looping: true },
    GbaBgmTrack { key: "game-over", song_id: 0x01B, looping: false },
    GbaBgmTrack { key: "loser", song_id: 0x01A, looping: true },
    GbaBgmTrack { key: "pavilion", song_id: 0x018, looping: true },
    GbaBgmTrack { key: "pavilion-comp", song_id: 0x020, looping: true },
    GbaBgmTrack { key: "final-battle", song_id: 0x017, looping: true },
    GbaBgmTrack { key: "credits", song_id: 0x01D, looping: true },
    // ROM-only short fanfare variant:
    GbaBgmTrack { key: "winner-1", song_id: 0x01F, looping: false },
];

/// Built-in GBA BGM catalog for `game` (`"exe1"`..=`"exe6"`).
pub fn gba_bgm_tracks(game: &str) -> &'static [GbaBgmTrack] {
    match game {
        "exe1" => EXE1_BGM_TRACKS,
        "exe2" => EXE2_BGM_TRACKS,
        "exe3" => EXE3_BGM_TRACKS,
        "exe4" => EXE4_BGM_TRACKS,
        "exe5" => EXE5_BGM_TRACKS,
        "exe6" => EXE6_BGM_TRACKS,
        _ => &[],
    }
}

/// Look up one GBA BGM entry by `game` and `key`.
pub fn gba_bgm_track(game: &str, key: &str) -> Option<GbaBgmTrack> {
    gba_bgm_tracks(game).iter().copied().find(|t| t.key == key)
}

/// Whether `(game, key)` is an official battle theme (used by the Music Library's
/// "Favorite all battle themes" action).
pub fn gba_bgm_is_battle_theme(game: &str, key: &str) -> bool {
    match game {
        "exe1" => matches!(key, "operation" | "net-battle" | "final-battle"),
        "exe2" => matches!(key, "virus-battle" | "battle-spirit" | "final-battle"),
        "exe3" => matches!(key, "netbattle-1" | "netbattle-2" | "final-battle" | "netbattle-3"),
        "exe4" => matches!(key, "virus-battle" | "tournament-battle" | "boss-battle" | "final-battle"),
        "exe5" => matches!(key, "virus-battle" | "boss-battle" | "final-battle"),
        "exe6" => matches!(key, "virus-battle" | "boss-battle" | "crossover-battle" | "final-battle"),
        _ => false,
    }
}

/// Extract an optional audio-only GBA `m4a::SoundBank` for `game` (e.g. `"exe1"` or `"exe2"`)
/// from `source` if any matching ROM is present. Returns `None` silently if absent.
pub fn extract_extra_bgm_bank(source: &crate::RomSource, game: &str) -> Option<m4a::SoundBank> {
    let &(_, codes) = EXTRA_BGM_GAMES.iter().find(|&&(g, _)| g == game)?;
    let wanted: Vec<String> = codes.iter().map(|c| (*c).to_string()).collect();
    let roms = source.find(&wanted).unwrap_or_else(|(_, found)| found);
    for rom in roms {
        let Ok(bytes) = rom.read() else { continue };
        match m4a::rom::extract(&bytes) {
            Ok((bank, _)) => return Some(bank),
            Err(e) => log::warn!("extracting {game} BGM from {}: {e}", rom.at),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalogs_have_unique_keys_and_song_ids() {
        for game in ["exe1", "exe2", "exe3", "exe4", "exe5", "exe6"] {
            let tracks = gba_bgm_tracks(game);
            assert!(!tracks.is_empty(), "{game} catalog empty");
            let mut keys = HashSet::new();
            let mut ids = HashSet::new();
            for t in tracks {
                assert!(keys.insert(t.key), "{game}: duplicate key {}", t.key);
                assert!(ids.insert(t.song_id), "{game}: duplicate song_id 0x{:03X}", t.song_id);
            }
        }
    }

    #[test]
    fn extra_bgm_returns_none_silently_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let src = crate::RomSource::Copies(dir.path().to_path_buf());
        assert!(extract_extra_bgm_bank(&src, "exe1").is_none());
        assert!(extract_extra_bgm_bank(&src, "exe2").is_none());
        assert!(extract_extra_bgm_bank(&src, "exe9").is_none());
    }

    #[test]
    fn extracts_real_exe1_and_exe2_roms_when_available() {
        let Ok(src) = crate::RomSource::from_env() else { return };
        for game in ["exe1", "exe2"] {
            let Some(bank) = extract_extra_bgm_bank(&src, game) else { continue };
            for track in gba_bgm_tracks(game) {
                let song = bank
                    .song(m4a::SongId(track.song_id))
                    .unwrap_or_else(|| panic!("{game}/{}: missing song_id 0x{:03X}", track.key, track.song_id));
                assert_eq!(
                    song.is_looping(),
                    track.looping,
                    "{game}/{} (0x{:03X}): looping mismatch",
                    track.key,
                    track.song_id
                );
            }
        }
    }
}
