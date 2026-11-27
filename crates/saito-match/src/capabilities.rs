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
    /// Practice dummy supports the DarkInvs AI action (EXE5).
    pub practice_dark_invs: bool,
    /// Supports the EXE5DS indoor/outdoor sunlight toggle (`sun_enabled`).
    pub ds_sun_toggle: bool,
    /// Supports EXE5DS Danger HUD and microphone Shout (`ds_shout_enabled` / `ds_shout_max_only`).
    pub ds_shout_toggle: bool,
}

const EXE4: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: false,
    flat_standard_copy_limit: true,
    patch_card_mb_limit: false,
    all_guard_toggle: true,
    dark_cut_in_fix: false,
    ruleset_sub_modes: false,
    random_battle: false,
    practice_dark_invs: false,
    ds_sun_toggle: false,
    ds_shout_toggle: false,
};

const EXE5: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: false,
    flat_standard_copy_limit: true,
    patch_card_mb_limit: true,
    all_guard_toggle: false,
    dark_cut_in_fix: true,
    ruleset_sub_modes: true,
    random_battle: false,
    practice_dark_invs: true,
    ds_sun_toggle: true,
    ds_shout_toggle: true,
};

const EXE6: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: true,
    flat_standard_copy_limit: false,
    patch_card_mb_limit: true,
    all_guard_toggle: false,
    dark_cut_in_fix: false,
    ruleset_sub_modes: false,
    random_battle: true,
    practice_dark_invs: false,
    ds_sun_toggle: false,
    ds_shout_toggle: false,
};

const UNKNOWN: GameCapabilities = GameCapabilities {
    mb_tier_copy_limits: false,
    flat_standard_copy_limit: false,
    patch_card_mb_limit: false,
    all_guard_toggle: false,
    dark_cut_in_fix: false,
    ruleset_sub_modes: false,
    random_battle: false,
    practice_dark_invs: false,
    ds_sun_toggle: false,
    ds_shout_toggle: false,
};

/// Historical simulation-compatibility epochs for replay playback across
/// engine minor lines.
///
/// Kept here instead of inside `saito-replay` so the replay format crate stays
/// game-agnostic.
pub const SIM_EPOCHS: &[saito_replay::SimEpoch] = &[
    // 0.1 -> 0.2 only modified EXE5 party knockout / custom switch state (S-004, S-005).
    // 0.2 -> 0.3 only modified EXE5 navi-switch guard clearing and party mood init (S-020, S-030).
    // 0.3 -> 0.4 only modified EXE5 party-switch hand size recomputation and barrier wind rule (S-050, S-056).
    // 0.4 -> 0.5 added EXE5 5DS ABD cooldown behavior in auto_battle/ai.luau (S-026).
    // 0.5 -> 0.6 modified battle simulation across EXE4, EXE5, and EXE6 (S-093, S-094, S-097, S-104).
    // 0.6 -> 0.7 added EXE4/EXE5 ABD high-class chip cap, EXE5 Gate-Party lead PartyCust/team support,
    // and battle/HUD state updates (S-107, S-109, S-110).
    // 0.7 -> 0.8 added 2-bit mic_tier input word, EXE5DS Sol Cross 70MB patch card, indoor/outdoor sun,
    // Danger/Shout gating, and EXE6 Random Battle replay validation (S-118, S-120).
    // 0.8 -> 0.9 refined EXE5 5DS battle rules (TomahawkMan StatusGuard scoping, 5DS ABD cooldown,
    // solo Team Navi dark chip restriction) and added EXE5DS SDAT music/voices & mic permissions (S-121..S-137).
    saito_replay::SimEpoch { line: "0.1", game: Some("exe5"), version: 1 },
    saito_replay::SimEpoch { line: "0.2", game: Some("exe5"), version: 2 },
    saito_replay::SimEpoch { line: "0.3", game: Some("exe5"), version: 3 },
    saito_replay::SimEpoch { line: "0.4", game: Some("exe5"), version: 4 },
    saito_replay::SimEpoch { line: "0.5", game: Some("exe5"), version: 5 },
    saito_replay::SimEpoch { line: "0.1", game: None, version: 2 },
    saito_replay::SimEpoch { line: "0.2", game: None, version: 2 },
    saito_replay::SimEpoch { line: "0.3", game: None, version: 2 },
    saito_replay::SimEpoch { line: "0.4", game: None, version: 2 },
    saito_replay::SimEpoch { line: "0.5", game: None, version: 2 },
    saito_replay::SimEpoch { line: "0.6", game: None, version: 6 },
    saito_replay::SimEpoch { line: "0.7", game: None, version: 7 },
    saito_replay::SimEpoch { line: "0.8", game: None, version: 8 },
    saito_replay::SimEpoch { line: "0.9", game: None, version: 9 },
];

/// Whether two engine versions share identical simulation behavior for `game`.
pub fn same_sim(a: &str, b: &str, game: &str) -> bool {
    saito_replay::same_sim(SIM_EPOCHS, a, b, game)
}

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
        assert!(!e4.practice_dark_invs);

        let e5 = for_game("exe5");
        assert!(e5.flat_standard_copy_limit);
        assert!(!e5.mb_tier_copy_limits);
        assert!(e5.patch_card_mb_limit);
        assert!(!e5.all_guard_toggle);
        assert!(e5.dark_cut_in_fix);
        assert!(e5.ruleset_sub_modes);
        assert!(!e5.random_battle);
        assert!(e5.practice_dark_invs);

        let e6 = for_game("exe6");
        assert!(!e6.flat_standard_copy_limit);
        assert!(e6.mb_tier_copy_limits);
        assert!(e6.patch_card_mb_limit);
        assert!(!e6.all_guard_toggle);
        assert!(!e6.dark_cut_in_fix);
        assert!(!e6.ruleset_sub_modes);
        assert!(e6.random_battle);
        assert!(!e6.practice_dark_invs);

        assert_eq!(for_game("exe3"), GameCapabilities::default());
    }

    #[test]
    fn sim_epochs_preserve_historical_replay_compatibility() {
        assert!(same_sim("0.1.2", "0.5.0", "exe6"));
        assert!(!same_sim("0.5.0", "0.6.0", "exe6"));
        assert!(same_sim("0.1.0", "0.5.0", "exe4"));
        assert!(!same_sim("0.5.0", "0.6.0", "exe4"));
        assert!(!same_sim("0.1.2", "0.2.0", "exe5"));
        assert!(!same_sim("0.4.0", "0.5.0", "exe5"));
        assert!(!same_sim("0.5.0", "0.6.0", "exe5"));
        assert!(same_sim("0.6.0", "0.6.1", "exe5"));
        assert!(!same_sim("0.6.0", "0.7.0", "exe4"));
        assert!(!same_sim("0.6.0", "0.7.0", "exe5"));
        assert!(!same_sim("0.6.0", "0.7.0", "exe6"));
        assert!(same_sim("0.7.0", "0.7.1", "exe5"));
        assert!(!same_sim("0.7.0", "0.8.0", "exe4"));
        assert!(!same_sim("0.7.0", "0.8.0", "exe5"));
        assert!(!same_sim("0.7.0", "0.8.0", "exe6"));
        assert!(same_sim("0.8.0", "0.8.1", "exe5"));
        assert!(!same_sim("0.8.0", "0.9.0", "exe4"));
        assert!(!same_sim("0.8.0", "0.9.0", "exe5"));
        assert!(!same_sim("0.8.0", "0.9.0", "exe6"));
        assert!(same_sim("0.9.0", "0.9.1", "exe5"));
    }
}
