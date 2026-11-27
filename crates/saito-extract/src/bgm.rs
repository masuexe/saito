//! Complete GBA BGM track catalogs for EXE4, EXE5, and EXE6 (`S-135`).
//!
//! Every track in each ROM's M4A music player (`PlayerId(31)`) is extracted into
//! `m4a::SoundBank` when the game loads. Entries are ordered to match the
//! official *Mega Man Battle Network Legacy Collection* (ROCKMAN EXE 合集)
//! Music Player tracklist, with ROM-only extras (short fanfare variants, BN4's
//! unused alternate battle track, and BN6's Boktai crossover track) appended at
//! the end of each game's list.

/// One GBA ROM BGM entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GbaBgmTrack {
    pub key: &'static str,
    pub song_id: u16,
    pub looping: bool,
}

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

/// Built-in GBA BGM catalog for `game` (`"exe4"`, `"exe5"`, `"exe6"`).
pub fn gba_bgm_tracks(game: &str) -> &'static [GbaBgmTrack] {
    match game {
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
