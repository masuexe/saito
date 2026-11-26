//! Per-game rule, lobby, and UI capabilities.
//!
//! Centralizes game-specific feature availability so match validation,
//! lobby wire codecs, and UI screens query capabilities instead of branching
//! on literal game ID strings.

/// Capabilities declared for a game pack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameCapabilities {
    /// Chip copy limits are tiered by MB (e.g. EXE6) instead of a single flat
    /// standard-chip copy limit.
    pub mb_tier_copy_limits: bool,
    /// Ruleset supports a flat standard-chip copy limit override (e.g. EXE4/EXE5).
    pub flat_standard_copy_limit: bool,
    /// Ruleset supports a patch-card MB budget override (e.g. EXE5/EXE6).
    pub patch_card_mb_limit: bool,
    /// Supports the Legacy Collection All Guard patch-card disable toggle (EXE4).
    pub all_guard_toggle: bool,
    /// Supports the Legacy Collection refused dimming dark-chip cut-in fix (EXE5).
    pub dark_cut_in_fix: bool,
    /// Supports tiered ruleset sub-modes (e.g. EXE5 5DS / Gate-Party).
    pub ruleset_sub_modes: bool,
    /// Supports Random Battle mode (EXE6).
    pub random_battle: bool,
}

const EXE4: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: false,
    flat_standard_copy_limit: true,
    patch_card_mb_limit: false,
    all_guard_toggle: true,
    dark_cut_in_fix: false,
    ruleset_sub_modes: false,
    random_battle: false,
};

const EXE5: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: false,
    flat_standard_copy_limit: true,
    patch_card_mb_limit: true,
    all_guard_toggle: false,
    dark_cut_in_fix: true,
    ruleset_sub_modes: true,
    random_battle: false,
};

const EXE6: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: true,
    flat_standard_copy_limit: false,
    patch_card_mb_limit: true,
    all_guard_toggle: false,
    dark_cut_in_fix: false,
    ruleset_sub_modes: false,
    random_battle: true,
};

const UNKNOWN: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: false,
    flat_standard_copy_limit: false,
    patch_card_mb_limit: false,
    all_guard_toggle: false,
    dark_cut_in_fix: false,
    ruleset_sub_modes: false,
    random_battle: false,
};

/// Capabilities for the given game ID.
pub fn for_game(game: &str) -> GameCapabilities {
    match game {
        "exe4" => EXE4,
        "exe5" => EXE5,
        "exe6" => EXE6,
        _ => UNKNOWN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_games_match_expected_capabilities() {
        let e4 = for_game("exe4");
        assert!(e4.flat_standard_copy_limit);
        assert!(!e4.mb_tier_copy_limits);
        assert!(!e4.patch_card_mb_limit);
        assert!(e4.all_guard_toggle);
        assert!(!e4.dark_cut_in_fix);
        assert!(!e4.ruleset_sub_modes);
        assert!(!e4.random_battle);

        let e5 = for_game("exe5");
        assert!(e5.flat_standard_copy_limit);
        assert!(!e5.mb_tier_copy_limits);
        assert!(e5.patch_card_mb_limit);
        assert!(!e5.all_guard_toggle);
        assert!(e5.dark_cut_in_fix);
        assert!(e5.ruleset_sub_modes);
        assert!(!e5.random_battle);

        let e6 = for_game("exe6");
        assert!(!e6.flat_standard_copy_limit);
        assert!(e6.mb_tier_copy_limits);
        assert!(e6.patch_card_mb_limit);
        assert!(!e6.all_guard_toggle);
        assert!(!e6.dark_cut_in_fix);
        assert!(!e6.ruleset_sub_modes);
        assert!(e6.random_battle);

        assert_eq!(for_game("exe3"), GameCapabilities::default());
    }
}
