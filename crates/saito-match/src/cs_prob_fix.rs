//! Per-effect ChargeShot Addition probability fix configuration for EXE5 and EXE6.
//!
//! In original GBA EXE5 (`megaman/program-charged-shot`, `0x0800FDD0`) and EXE6
//! (`patch-cards/chrgs`, `0x8D`), the Patch Card bug "B PowerAttack: ChargeShot"
//! (`Ⓑ集气CS` / `ⒷためチャージS`) skips the random draw and activates the navi's
//! `charge_shot_kind` at 100% probability (the "ChargeShot Addition glitch",
//! fixed in Mega Man Battle Network Legacy Collection).
//!
//! [`CsProbFix`] is a per-effect bitmask letting the lobby host (or practice
//! player) choose which ChargeShot Addition effects apply the normal random
//! draw (1/2 in EXE5; 1/8 for rare effects and 1/2 for the rest in EXE6) and
//! which remain at 100% original GBA behavior.

/// One configurable ChargeShot Addition effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectEntry {
    /// Bit in [`CsProbFix`] (matches `CS_PROB_FIX_BITS` in Luau).
    pub bit: u16,
    /// Projectile variant key (`shot/...`), also the suffix of the
    /// `patch_card_effects` locale key `charge_shot_kind.<variant_id>`.
    pub variant_id: &'static str,
    /// True when the fixed probability is 1/8 (12.5%); false for 1/2 (50%).
    pub rare: bool,
}

impl EffectEntry {
    /// Locale key in the game's `patch_card_effects` string table.
    pub fn locale_key(&self) -> String {
        format!("charge_shot_kind.{}", self.variant_id)
    }
}

/// EXE5 ChargeShot Addition effects (11 effects; all 1/2 when fixed).
pub const EXE5: [EffectEntry; 11] = [
    EffectEntry { bit: 0x0001, variant_id: "shot/breaking", rare: false },
    EffectEntry { bit: 0x0002, variant_id: "shot/charged-attack-a0", rare: false },
    EffectEntry { bit: 0x0004, variant_id: "shot/charged-blinding", rare: false },
    EffectEntry { bit: 0x0008, variant_id: "shot/charged-confusing", rare: false },
    EffectEntry { bit: 0x0010, variant_id: "shot/charged-hp-bug", rare: false },
    EffectEntry { bit: 0x0020, variant_id: "shot/charged-lava", rare: false },
    EffectEntry { bit: 0x0040, variant_id: "shot/charged-panel-breaking", rare: false },
    EffectEntry { bit: 0x0080, variant_id: "shot/charged-paralyzing", rare: false },
    EffectEntry { bit: 0x0100, variant_id: "shot/charged-poisoning", rare: false },
    EffectEntry { bit: 0x0200, variant_id: "shot/charged-uninstalling", rare: false },
    EffectEntry { bit: 0x0400, variant_id: "shot/chip-destruction", rare: false },
];

/// EXE6 ChargeShot Addition effects (13 effects; 2 rare at 1/8, 11 at 1/2 when fixed).
pub const EXE6: [EffectEntry; 13] = [
    EffectEntry { bit: 0x0001, variant_id: "shot/breaking", rare: false },
    EffectEntry { bit: 0x0002, variant_id: "shot/charged-bubbling", rare: false },
    EffectEntry { bit: 0x0004, variant_id: "shot/charged-confusing", rare: false },
    EffectEntry { bit: 0x0008, variant_id: "shot/charged-freezing", rare: false },
    EffectEntry { bit: 0x0010, variant_id: "shot/charged-grass", rare: false },
    EffectEntry { bit: 0x0020, variant_id: "shot/charged-hp-bug", rare: false },
    EffectEntry { bit: 0x0040, variant_id: "shot/charged-hp-bug-marked", rare: true },
    EffectEntry { bit: 0x0080, variant_id: "shot/charged-ice", rare: false },
    EffectEntry { bit: 0x0100, variant_id: "shot/charged-paralyzing", rare: false },
    EffectEntry { bit: 0x0200, variant_id: "shot/charged-pulling", rare: false },
    EffectEntry { bit: 0x0400, variant_id: "shot/charged-road", rare: false },
    EffectEntry { bit: 0x0800, variant_id: "shot/charged-road-back", rare: false },
    EffectEntry { bit: 0x1000, variant_id: "shot/chip-destruction", rare: true },
];

/// Per-effect bitmask of ChargeShot Addition probability fixes (0 = all original).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CsProbFix(pub u16);

impl CsProbFix {
    /// All effects keep original GBA 100% behavior.
    pub const NONE: CsProbFix = CsProbFix(0);

    /// Whether `game` supports the ChargeShot Addition probability fix (EXE5 and EXE6).
    pub fn supported(game: &str) -> bool {
        matches!(game, "exe5" | "exe6")
    }

    /// Configurable effects for `game` (empty for unsupported games).
    pub fn effects(game: &str) -> &'static [EffectEntry] {
        match game {
            "exe5" => &EXE5,
            "exe6" => &EXE6,
            _ => &[],
        }
    }

    /// Bitmask with every supported effect for `game` enabled (0 for unsupported games).
    pub fn all_mask(game: &str) -> u16 {
        match game {
            "exe5" => 0x07FF,
            "exe6" => 0x1FFF,
            _ => 0,
        }
    }

    /// Every supported effect for `game` enabled.
    pub fn all(game: &str) -> CsProbFix {
        CsProbFix(Self::all_mask(game))
    }

    /// Construct from raw bitmask.
    pub const fn from_bits(bits: u16) -> CsProbFix {
        CsProbFix(bits)
    }

    /// Raw bitmask.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// True if no effects have the probability fix enabled.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// True if all supported effects for `game` are enabled (false for unsupported games).
    pub fn is_all(self, game: &str) -> bool {
        let mask = Self::all_mask(game);
        mask != 0 && (self.0 & mask) == mask
    }

    /// Number of enabled effects for `game`.
    pub fn count(self, game: &str) -> usize {
        (self.0 & Self::all_mask(game)).count_ones() as usize
    }

    /// Whether `self` only sets bits valid for `game` (`NONE` is always legal).
    pub fn allowed_for(self, game: &str) -> bool {
        self.0 & !Self::all_mask(game) == 0
    }

    /// Restrict bits to those valid for `game`.
    pub fn clamp_for(self, game: &str) -> CsProbFix {
        CsProbFix(self.0 & Self::all_mask(game))
    }

    /// Whether a specific effect bit is enabled.
    pub const fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }

    /// Toggle a specific effect bit.
    pub const fn toggled(self, bit: u16) -> CsProbFix {
        CsProbFix(self.0 ^ bit)
    }
}

/// Whether `game` supports the ChargeShot Addition probability fix (EXE5 and EXE6).
pub fn supported(game: &str) -> bool {
    CsProbFix::supported(game)
}

/// Configurable effects for `game` (empty for unsupported games).
pub fn effects(game: &str) -> &'static [EffectEntry] {
    CsProbFix::effects(game)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_match_effect_tables() {
        for game in ["exe5", "exe6"] {
            let mut union = 0u16;
            for e in effects(game) {
                assert_eq!(e.bit.count_ones(), 1, "{game}: non-single bit for {}", e.variant_id);
                assert_eq!(union & e.bit, 0, "{game}: duplicate bit for {}", e.variant_id);
                union |= e.bit;
            }
            assert_eq!(union, CsProbFix::all_mask(game), "{game}: all_mask mismatch");
            assert!(CsProbFix::all(game).is_all(game));
            assert_eq!(CsProbFix::all(game).count(game), effects(game).len());
            assert!(CsProbFix::all(game).allowed_for(game));
        }
        assert!(!supported("exe4"));
        assert!(CsProbFix::NONE.allowed_for("exe4"));
        assert!(!CsProbFix(1).allowed_for("exe4"));
        assert!(!CsProbFix(0x0800).allowed_for("exe5"));
        assert!(!CsProbFix(0x2000).allowed_for("exe6"));
    }

    /// Every entry in EXE5 and EXE6 resolves to an existing `projectile-variant` record
    /// in the game's content and has a localized `charge_shot_kind.<id>` string.
    #[test]
    fn every_effect_exists_in_content() {
        for (game, content) in [("exe5", crate::testing::exe5_content()), ("exe6", crate::testing::exe6_content())] {
            for e in effects(game) {
                assert!(
                    content.defs.record(e.variant_id).is_some(),
                    "{game}: missing projectile variant {}",
                    e.variant_id
                );
            }
        }
    }

    /// End-to-end simulation check for EXE5 (`megaman/program-charged-shot`) and EXE6
    /// (`patch-cards/chrgs`): unfixed effects trigger 100%, fixed effects roll their
    /// normal random chance (1/2 in EXE5; 1/2 or 1/8 in EXE6), and fixing a different
    /// effect leaves the active effect at 100%.
    #[test]
    fn exe5_and_exe6_program_charged_shot_probability_fix() {
        use saito_battle::battle::{fight, mode};
        use saito_battle::input::{PlayerTick, keys};
        use saito_battle::{Battle, Rng};

        fn fighting_base(
            content: &std::sync::Arc<saito_battle::Content>,
            game: &str,
            weapon_key: &str,
            variant_key: &str,
            fix: CsProbFix,
        ) -> Battle {
            let mut m = crate::pick::live(content, game, 1, None).unwrap();
            let megaman = crate::ids::navi(content, game, "megaman").unwrap();
            for side in &mut m.sides {
                side.set_navi(content, megaman).unwrap();
            }
            m.cs_prob_fix = fix;
            let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
            let mut last = [None; 2];
            for _ in 0..3000 {
                if b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING {
                    let weapon = content.weapon_by_key(weapon_key);
                    let variant = content.defs.record(variant_key).expect(variant_key);
                    b.stats[0].weapons.charge_shot = Some(weapon);
                    b.stats[0].weapons.charge_shot_kind = Some(variant);
                    let p0 = b.player(0).expect("side 0 navi");
                    let aid = b.objects.get(p0).actor.expect("side 0 actor");
                    b.actors.get_mut(aid).charge_shot = Some(weapon);
                    return b;
                }
                let input = [0, 1].map(|side| {
                    last[side] = saito_battle::scenario::custom_choice(&b, side, last[side]);
                    PlayerTick::buttons(0).with_choice(last[side])
                });
                b.tick(&input, Default::default());
            }
            panic!("{game}: fight never started");
        }

        fn count_triggers(
            base: &Battle,
            trials: u32,
            triggered: impl Fn(&Battle, &Battle) -> bool,
        ) -> u32 {
            let mut hits = 0u32;
            for i in 1..=trials {
                let mut b = base.clone();
                b.rng = Rng::new(i.wrapping_mul(0x9E37_79B1) ^ 0x1357_2468);
                // Hold B long enough to fully charge (max 100 ticks), then release to fire.
                for _ in 0..105 {
                    b.tick(&[PlayerTick::buttons(keys::B), PlayerTick::default()], Default::default());
                }
                for _ in 0..25 {
                    b.tick(&[PlayerTick::default(), PlayerTick::default()], Default::default());
                }
                if triggered(base, &b) {
                    hits += 1;
                }
            }
            hits
        }

        let panel_changed = |before: &Battle, after: &Battle| {
            let p1 = after.player(1).expect("side 1 navi");
            let pos = after.objects.get(p1).panel;
            after.field.panel(pos.x, pos.y).unwrap().kind != before.field.panel(pos.x, pos.y).unwrap().kind
        };
        let bugs_changed = |before: &Battle, after: &Battle| after.stats[1].bugs != before.stats[1].bugs;

        // EXE5: shot/charged-lava (1/2 when fixed)
        let exe5 = crate::testing::exe5_content();
        let lava_bit5 = EXE5.iter().find(|e| e.variant_id == "shot/charged-lava").unwrap().bit;
        let other_bit5 = EXE5.iter().find(|e| e.variant_id == "shot/charged-paralyzing").unwrap().bit;
        let base5_none = fighting_base(&exe5, "exe5", "megaman/program-charged-shot", "shot/charged-lava", CsProbFix::NONE);
        assert_eq!(count_triggers(&base5_none, 64, panel_changed), 64, "EXE5 unfixed = 100%");
        let base5_other = fighting_base(&exe5, "exe5", "megaman/program-charged-shot", "shot/charged-lava", CsProbFix::from_bits(other_bit5));
        assert_eq!(count_triggers(&base5_other, 64, panel_changed), 64, "EXE5 other effect fixed = 100%");
        let base5_fixed = fighting_base(&exe5, "exe5", "megaman/program-charged-shot", "shot/charged-lava", CsProbFix::from_bits(lava_bit5));
        let fixed5 = count_triggers(&base5_fixed, 64, panel_changed);
        assert!(fixed5 > 15 && fixed5 < 49, "EXE5 fixed should be ~50%, got {fixed5}/64");

        // EXE6: shot/charged-grass (1/2 when fixed) and shot/charged-hp-bug-marked (1/8 when fixed)
        let exe6 = crate::testing::exe6_content();
        let grass_bit6 = EXE6.iter().find(|e| e.variant_id == "shot/charged-grass").unwrap().bit;
        let instant_bit6 = EXE6.iter().find(|e| e.variant_id == "shot/charged-hp-bug-marked").unwrap().bit;
        let base6_none = fighting_base(&exe6, "exe6", "patch-cards/chrgs", "shot/charged-grass", CsProbFix::NONE);
        assert_eq!(count_triggers(&base6_none, 64, panel_changed), 64, "EXE6 unfixed = 100%");
        let base6_other = fighting_base(&exe6, "exe6", "patch-cards/chrgs", "shot/charged-grass", CsProbFix::from_bits(instant_bit6));
        assert_eq!(count_triggers(&base6_other, 64, panel_changed), 64, "EXE6 other effect fixed = 100%");
        let base6_fixed = fighting_base(&exe6, "exe6", "patch-cards/chrgs", "shot/charged-grass", CsProbFix::from_bits(grass_bit6));
        let fixed6_half = count_triggers(&base6_fixed, 64, panel_changed);
        assert!(fixed6_half > 15 && fixed6_half < 49, "EXE6 normal fixed should be ~50%, got {fixed6_half}/64");

        let base6_rare_unfixed = fighting_base(&exe6, "exe6", "patch-cards/chrgs", "shot/charged-hp-bug-marked", CsProbFix::NONE);
        assert_eq!(count_triggers(&base6_rare_unfixed, 32, bugs_changed), 32, "EXE6 rare unfixed = 100%");
        let base6_rare_fixed = fighting_base(&exe6, "exe6", "patch-cards/chrgs", "shot/charged-hp-bug-marked", CsProbFix::from_bits(instant_bit6));
        let fixed6_rare = count_triggers(&base6_rare_fixed, 128, bugs_changed);
        assert!(fixed6_rare > 3 && fixed6_rare < 35, "EXE6 rare fixed should be ~12.5%, got {fixed6_rare}/128");
    }
}
