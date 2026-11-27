//! Presentation-only Mega Man Battle Network 5: Double Team DS (EXE5DS)
//! battle voice observer.
//!
//! Observes read-only [`Battle`] state transitions each fresh tick on the
//! local viewer's side and triggers authentic EXE5DS `SSAR` voice clips on
//! [`BattleAudio`] (Lan, MegaMan, ProtoMan, and Colonel).
//!
//! Strictly isolated from battle simulation: never mutates [`Battle`], never
//! touches simulation RNG, and produces zero digest or wire-protocol changes.

use saito_audio::BattleAudio;
use saito_battle::battle::{battle_flags, fight, mode};
use saito_battle::collision::f1;
use saito_battle::content::{ChipClass, ChipFlags};
use saito_battle::custom::screen::{Phase as CustomPhase, ProgramAdvanceStep};
use saito_battle::setup::effects;
use saito_battle::transform::{SequencerState, TransformPhase};
use saito_battle::Battle;
use saito_content_api::{keys, ChipHandle, FormHandle};
use std::sync::atomic::{AtomicBool, Ordering};

/// Voice mixer channel for operator (Lan) callouts.
pub const CH_LAN: usize = 0;
/// Voice mixer channel for active Navi cries (MegaMan / ProtoMan / Colonel).
pub const CH_NAVI: usize = 1;

/// Global presentation toggle controlled by the host's Settings (`exe5ds_voices`).
static ENABLED: AtomicBool = AtomicBool::new(true);

/// Whether EXE5DS battle voices are currently enabled.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Enable or disable EXE5DS battle voices globally.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Deterministic presentation-only hash (splitmix64) so random clip variant
/// selection never touches simulation RNG.
fn voice_hash(ticks: u32, side: u8, salt: u32) -> u32 {
    let mut z = ((ticks as u64) << 32) ^ ((side as u64) << 24) ^ (salt as u64) ^ 0x9E37_79B9_7F4A_7C15;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as u32
}

/// SSAR voice ID offset for a voiced EXE5DS playable Navi (verified from
/// EXE5DS JP ARM9 table `0x020CBA48`: MegaMan = `+0`, ProtoMan = `+100`,
/// Colonel = `+200`, all other Team Navis = `None`).
///
/// DarkSoul (`dark_megaman`) is explicitly muted (`None`), matching EXE5DS
/// ARM9 checks on `NaviStats +0x58 != 0` (`0x02098A74`, `0x02098AC8`,
/// `0x02098AE0`, `0x02098B16`, `0x02002B1C`, `0x0221ED52`).
pub fn navi_voice_offset(navi_local_key: &str) -> Option<u16> {
    match navi_local_key {
        "megaman" => Some(0x00),
        "protoman" => Some(0x64),
        "colonel" => Some(0xC8),
        _ => None,
    }
}

/// Navi's Turn 1 Battle Start callout from EXE5DS ARM9 `0x02002AF0` / table
/// `0x020CBAC4` (called at `0x0208DABC` & `0x020A2B4E` on turn 1 start banner):
/// - MegaMan: `0x261..=0x264` (`V_ROCK0401..0404`, 4 clips)
/// - ProtoMan: `0x2C5..=0x2C7` (`V_BLUES0401..0403`, 3 clips)
/// - Colonel: `0x329..=0x32B` (`V_COLONEL0401..0403`, 3 clips)
/// - All other Team Navis / DarkSoul: silent (`None`).
pub fn battle_start_voice(navi_local_key: &str, variant: u32) -> Option<u16> {
    let table: &[u16] = match navi_local_key {
        "megaman" => &[0x261, 0x262, 0x263, 0x264],
        "protoman" => &[0x2C5, 0x2C6, 0x2C7],
        "colonel" => &[0x329, 0x32A, 0x32B],
        _ => return None,
    };
    Some(table[variant as usize % table.len()])
}

/// Navi's Danger Microphone Shout cry from EXE5DS `OV423` `T4 0x93`
/// (`0x0221ED44`, table `0x02223D10`), indexed by Navi and shout strength
/// tier (`0` = low, `1` = medium, `2` = high/Full Synchro):
/// - MegaMan: `[0x289, 0x28A, 0x286]` (`V_ROCK0811`, `0812`, `0808`)
/// - ProtoMan: `[0x2EE, 0x2ED, 0x2EC]` (`V_BLUES0812`, `0811`, `0810`)
/// - Colonel: `[0x352, 0x350, 0x34E]` (`V_COLONEL0812`, `0810`, `0808`)
/// - All other Team Navis / DarkSoul: silent (`None`).
pub fn danger_shout_voice(navi_local_key: &str, strength: u8) -> Option<u16> {
    let idx = (strength as usize).min(2);
    let table: [u16; 3] = match navi_local_key {
        "megaman" => [0x289, 0x28A, 0x286],
        "protoman" => [0x2EE, 0x2ED, 0x2EC],
        "colonel" => [0x352, 0x350, 0x34E],
        _ => return None,
    };
    Some(table[idx])
}

/// Lan's Soul Unison (`0x1FC..=0x207`) or Chaos Unison (`0x208..=0x213`) voice
/// for the 12 EXE5 Double Souls.
pub fn soul_voice(form_local_key: &str, chaos: bool) -> Option<u16> {
    let idx: u16 = match form_local_key {
        "protosoul" => 0,
        "magnetsoul" => 1,
        "gyrosoul" => 2,
        "napalmsoul" => 3,
        "searchsoul" => 4,
        "meddysoul" => 5,
        "colonelsoul" => 6,
        "knightsoul" => 7,
        "shadowsoul" => 8,
        "tomahawksoul" => 9,
        "numbersoul" => 10,
        "toadsoul" => 11,
        _ => return None,
    };
    Some(if chaos { 0x208 + idx } else { 0x1FC + idx })
}

/// Lan's Party Navi Change voices `(NC_id, PLS_id)` from ARM9 `0x021309E4`
/// (`BANK[37..=62]`).
pub fn navi_switch_voices(navi_local_key: &str) -> Option<(u16, u16)> {
    match navi_local_key {
        "megaman" => Some((0x214, 0x221)),
        "protoman" => Some((0x215, 0x222)),
        "magnetman" => Some((0x216, 0x223)),
        "gyroman" => Some((0x217, 0x224)),
        "napalmman" => Some((0x218, 0x225)),
        "searchman" => Some((0x219, 0x226)),
        "meddy" => Some((0x21A, 0x227)),
        "colonel" => Some((0x21B, 0x228)),
        "knightman" => Some((0x21C, 0x229)),
        "shadowman" => Some((0x21D, 0x22A)),
        "tomahawkman" => Some((0x21E, 0x22B)),
        "numberman" => Some((0x21F, 0x22C)),
        "toadman" => Some((0x220, 0x22D)),
        _ => None,
    }
}

/// Lan's Program Advance callout (`BANK[63..=81]`, `0x23E..=0x250`).
pub fn pa_voice(chip_local_key: &str, class: ChipClass) -> Option<u16> {
    let specific = match chip_local_key {
        "gigacan1" | "gigacan2" | "gigacan3" => Some(0x23F),
        "infvulc1" | "infvulc2" | "infvulc3" => Some(0x240),
        "boxer1" | "boxer2" | "boxer3" => Some(0x241),
        "suprspr1" | "suprspr2" | "suprspr3" => Some(0x242),
        "shakpar1" | "shakpar2" | "shakpar3" => Some(0x243),
        "cacdanc1" | "cacdanc2" | "cacdanc3" => Some(0x244),
        "h-burst" => Some(0x245),
        "lifesrd" => Some(0x246),
        "greatyo" => Some(0x247),
        "pithoky" => Some(0x248),
        "poisphar" => Some(0x249),
        "timebom" => Some(0x24A),
        "bodygrd" => Some(0x24B),
        "piledrvr" => Some(0x24C),
        "csmopris" => Some(0x24D),
        "wildbird" => Some(0x24E),
        "football" => Some(0x24F),
        "bignoise" => Some(0x250),
        _ => None,
    };
    specific.or_else(|| (class == ChipClass::ProgramAdvance).then_some(0x23E))
}

/// Damaging chip attack cry base ID (4 damage tiers matching ARM9 `0x02098A60`:
/// `<=80`, `<=139`, `<=200`, `>200`), 3 variants per tier.
pub fn chip_cry_voice(damage_total: u16, speaker_offset: u16, variant: u32) -> u16 {
    let base = if damage_total <= 80 {
        0x27F
    } else if damage_total <= 139 {
        0x282
    } else if damage_total <= 200 {
        0x285
    } else {
        0x288
    };
    base + (variant % 3) as u16 + speaker_offset
}

/// Hurt/damage cry ID (3 damage tiers matching ARM9 `0x02098B04` / `0x021309D8`:
/// `<70`, `<150`, `>=150`), 3 variants per tier.
pub fn hurt_cry_voice(damage_taken: u16, speaker_offset: u16, variant: u32) -> u16 {
    let base = if damage_taken < 70 {
        0x26E
    } else if damage_taken < 150 {
        0x271
    } else {
        0x274
    };
    base + (variant % 3) as u16 + speaker_offset
}

/// Status-infliction / stun cry (`0x02098AE0`, called from `0x020615B4`):
/// 6 variants (`0x277..=0x27C`) + speaker offset.
pub fn status_cry_voice(speaker_offset: u16, variant: u32) -> u16 {
    0x277 + (variant % 6) as u16 + speaker_offset
}

/// Presentation-only state tracker that observes one console side of an EXE5
/// battle and fires EXE5DS voice clips on [`BattleAudio`].
#[derive(Clone, Debug, Default)]
pub struct DsVoiceObserver {
    initialized: bool,
    last_ticks: u32,
    started_round: bool,
    prev_hp: Option<u16>,
    prev_total_damage: Option<u16>,
    prev_party_active: u8,
    prev_form: Option<FormHandle>,
    announced_form: Option<FormHandle>,
    prev_used_chip: Option<(ChipHandle, u8)>,
    prev_telop: Option<(u8, Option<ChipHandle>)>,
    prev_pa_step: Option<ProgramAdvanceStep>,
    prev_danger: bool,
    prev_stunned: bool,
    prev_transform_fade: bool,
    result_played: bool,
    deleted_played: bool,
}

impl DsVoiceObserver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear tracked state for a fresh round or battle restart.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Observe `battle` from `side`'s console and fire any triggered EXE5DS
    /// battle voices into `audio`.
    pub fn observe(&mut self, battle: &Battle, side: u8, audio: &mut BattleAudio) {
        if !enabled() || battle.content.game() != "exe5" {
            return;
        }
        let s = side as usize & 1;

        let ticks = battle.round.ticks;
        if self.initialized && ticks < self.last_ticks {
            self.reset();
        }

        let active_navi = battle.stats[s].navi;
        let navi_key = battle
            .content
            .defs
            .navis
            .get(active_navi.0 as usize)
            .map(|d| keys::local(&d.key))
            .unwrap_or("");
        let speaker_offset = navi_voice_offset(navi_key);
        let player_ref = battle.player(side);
        let cur_hp = player_ref.map(|r| battle.objects.get(r).hp);
        // `actor.total_damage_taken` accumulates only direct hit damage (`final_damage`),
        // matching EXE5DS ARM9 `0x020629B0..0x02062A16` where hurt cries (`0x02098B04`)
        // trigger only on direct hits (`+0x78`), never on passive poison / HP-bug drain (`+0x84`).
        let cur_total_damage = player_ref
            .and_then(|r| battle.objects.get(r).actor)
            .map(|a| battle.actors.get(a).total_damage_taken);
        let cur_stunned = player_ref.is_some_and(|r| {
            battle.objects.get(r).collision.is_some_and(|c| {
                battle.collision.get(c).f1 & (f1::PARALYZED | f1::FROZEN | f1::BUBBLED | f1::IMMOBILIZED) != 0
            })
        });
        let cur_party_active = battle.party[s].active;
        let cur_form = battle.stats[s].form;
        let cur_danger = battle.is_danger(side);
        let cur_used = battle.used_chips[s].map(|u| (u.chip, u.ticks));
        let cur_telop = battle.banner.telop.map(|t| (t.side, t.chip));
        let cur_pa = battle.custom.sides[s].screen.as_ref().and_then(|scr| match scr.phase {
            CustomPhase::ProgramAdvance { anim } => scr.program_advance.map(|pa| (anim.step, pa.chip)),
            _ => None,
        });
        let cur_pa_step = cur_pa.map(|(step, _)| step);
        let in_transform_fade = matches!(
            battle.transform_seq.state,
            SequencerState::Transform { phase: TransformPhase::FadeOut, .. }
        );

        if !self.initialized {
            self.initialized = true;
            self.last_ticks = ticks;
            self.prev_hp = cur_hp;
            self.prev_total_damage = cur_total_damage;
            self.prev_party_active = cur_party_active;
            self.prev_form = Some(cur_form);
            self.announced_form = None;
            self.prev_used_chip = cur_used;
            self.prev_telop = cur_telop;
            self.prev_pa_step = cur_pa_step;
            self.prev_danger = cur_danger;
            self.prev_stunned = cur_stunned;
            self.prev_transform_fade = in_transform_fade;
            return;
        }
        self.last_ticks = ticks;

        // 1. Battle Start (Turn 1 START_BANNER): ARM9 `0x02002AF0` / table `0x020CBAC4`
        //    Navi battle-start line (MegaMan `0x261..=0x264`, ProtoMan `0x2C5..=0x2C7`,
        //    Colonel `0x329..=0x32B`; other Team Navis / DarkSoul silent).
        if !self.started_round && battle.round.mode == mode::FIGHTING && battle.fight.state == fight::START_BANNER {
            self.started_round = true;
            let h = voice_hash(ticks, side, 0x11);
            if let Some(clip) = battle_start_voice(navi_key, h) {
                audio.play_voice(clip, CH_NAVI);
            }
        }

        // 2. Program Advance Formation (Custom Screen PA reveal, ARM9 `0x0200CB1A` -> `0x0200CDC0`):
        //    fires at the exact `BannerIn -> Names` transition (20 ticks after PA banner holds).
        if let Some((ProgramAdvanceStep::Names, pa_chip)) = cur_pa
            && self.prev_pa_step != Some(ProgramAdvanceStep::Names)
            && let Some(chip_def) = battle.content.defs.chips.get(pa_chip.0 as usize)
        {
            let chip_data = battle.content.chip(pa_chip);
            if let Some(pa_clip) = pa_voice(keys::local(&chip_def.key), chip_data.class) {
                audio.play_voice(pa_clip, CH_LAN);
            }
        }

        // 3. Party Navi Switch:
        //    - Custom-screen switch (`0x02007D64` / table `0x020CC204`): always `NC` (`0x214..=0x220`).
        //    - In-battle switch (`0x02098B90` / table `0x021309E4`): 50/50 `NC` or `PLS` (`0x221..=0x22D`).
        if cur_party_active != self.prev_party_active {
            if let Some((nc_id, pls_id)) = navi_switch_voices(navi_key) {
                let from_custom = battle.transform_seq.busy || battle.turn_transforms[s].navi_switch.is_some();
                let clip = if from_custom {
                    nc_id
                } else {
                    let h = voice_hash(ticks, side, 0x51);
                    if (h & 1) == 0 { nc_id } else { pls_id }
                };
                audio.play_voice(clip, CH_LAN);
            }
            // Avoid mistaking the incoming navi's different HP / damage / status for damage or stun.
            self.prev_hp = cur_hp;
            self.prev_total_damage = cur_total_damage;
            self.prev_stunned = cur_stunned;
            self.deleted_played = false;
        }

        // 4. Soul Unison / Chaos Unison transformation (`0x0205FED6` -> `0x02007D64`):
        //    plays on `TransformPhase::FadeOut` entry for this side's own request.
        if in_transform_fade && !self.prev_transform_fade {
            let req = battle.transform_seq.requests[s];
            if let Some(form) = req.form
                && let Some(form_def) = battle.content.defs.forms.get(form.0 as usize)
                && let Some(clip) = soul_voice(keys::local(&form_def.key), req.alternate)
            {
                self.announced_form = Some(form);
                audio.play_voice(clip, CH_LAN);
            }
        } else if self.prev_form.is_some_and(|prev| prev != cur_form) {
            if self.announced_form == Some(cur_form) {
                self.announced_form = None;
            } else if let Some(form_def) = battle.content.defs.forms.get(cur_form.0 as usize)
                && let Some(clip) = soul_voice(keys::local(&form_def.key), battle.turn_transforms[s].alternate)
            {
                audio.play_voice(clip, CH_LAN);
            }
        }

        // 5. Chip Used (non-dimming via `used_chips[s]`, dimming/cut-in via `telop`):
        //    ARM9 `0x02007086` -> `0x02098A60` Navi attack cry (including PA chip execution,
        //    since Lan already called out the PA name during Custom Screen formation).
        let mut newly_used_chip: Option<(ChipHandle, u16)> = None;
        if let Some(used) = battle.used_chips[s] {
            let is_new = match self.prev_used_chip {
                None => true,
                Some((prev_chip, prev_ticks)) => prev_chip != used.chip || used.ticks > prev_ticks,
            };
            if is_new {
                newly_used_chip = Some((used.chip, used.damage.saturating_add(used.bonus)));
            }
        }
        if newly_used_chip.is_none()
            && let Some(telop) = battle.banner.telop
            && telop.side == side
            && let Some(chip) = telop.chip
            && self.prev_telop != Some((telop.side, Some(chip)))
        {
            newly_used_chip = Some((chip, telop.damage.saturating_add(telop.bonus)));
        }
        if let Some((chip, shown_dmg)) = newly_used_chip
            && let Some(offset) = speaker_offset
        {
            let chip_data = battle.content.chip(chip);
            let has_damage = (chip_data.flags.0 & ChipFlags::HAS_DAMAGE) != 0 || shown_dmg > 0;
            if has_damage {
                let dmg = if shown_dmg > 0 { shown_dmg } else { chip_data.damage };
                let h = voice_hash(ticks, side, chip.0 as u32 ^ 0xA3);
                audio.play_voice(chip_cry_voice(dmg, offset, h), CH_NAVI);
            }
        }

        // 6. Danger Microphone Shout triggered (`OV423` `T4 0x93`, `0x0221ED44` / `0x02223D10`):
        if self.prev_danger
            && !cur_danger
            && battle.mic_levels[s] >= 30
            && (battle.round.flags & battle_flags::DIMMED) != 0
        {
            let level = battle.mic_levels[s];
            let max_only = (battle.setup.settings.effects & effects::DS_SHOUT_MAX_ONLY) != 0;
            let strength = if max_only || level > 0x4B {
                2
            } else if level > 0x28 {
                1
            } else {
                0
            };
            if let Some(clip) = danger_shout_voice(navi_key, strength) {
                audio.play_voice(clip, CH_NAVI);
            }
        }

        // 7. Status-infliction / stun cry (`0x020615B4` -> `0x02098AE0`):
        if cur_stunned
            && !self.prev_stunned
            && cur_hp.is_some_and(|hp| hp > 0)
            && let Some(offset) = speaker_offset
        {
            let h = voice_hash(ticks, side, 0x77);
            audio.play_voice(status_cry_voice(offset, h), CH_NAVI);
        }

        // 8. Hurt cry (`0x02098B04`, direct hits only — excludes poison/HP-bug drain)
        //    & Deletion cry (`0x02098ABC`, any HP -> 0).
        let mut deleted_this_tick = false;
        if let (Some(prev), Some(cur)) = (self.prev_hp, cur_hp)
            && cur < prev
            && let Some(offset) = speaker_offset
        {
            if cur == 0 && !self.deleted_played {
                self.deleted_played = true;
                deleted_this_tick = true;
                audio.play_voice(0x265 + offset, CH_NAVI);
            } else if cur > 0
                && let (Some(prev_dmg), Some(cur_dmg)) = (self.prev_total_damage, cur_total_damage)
                && cur_dmg > prev_dmg
            {
                let hit_dmg = cur_dmg - prev_dmg;
                // ARM9 `0x02098B04`: 2/8 (25%) chance on direct hit, using pure presentation hash.
                let h = voice_hash(ticks, side, ((cur as u32) << 16) | hit_dmg as u32);
                if (h & 7) >= 6 {
                    audio.play_voice(hurt_cry_voice(hit_dmg, offset, h >> 3), CH_NAVI);
                }
            }
        }

        // 9. Round Win (`0x02070FC4` / `0x02109800`) & Lose (`0x02074A3E`) Result voices
        if !self.result_played && let Some(offset) = speaker_offset {
            match battle.fight.state {
                fight::WIN => {
                    self.result_played = true;
                    let h = voice_hash(ticks, side, 0x91);
                    let max_hp = battle.stats[s].max_hp.max(1) as u32;
                    let hp = cur_hp.unwrap_or(0) as u32;
                    let base = if hp * 4 >= max_hp * 3 {
                        0x268 // Rank >= 9 tier
                    } else if hp * 4 >= max_hp {
                        0x26A // Rank 5..=8 tier
                    } else {
                        0x26C // Rank <= 4 tier
                    };
                    audio.play_voice(base + (h & 1) as u16 + offset, CH_NAVI);
                }
                fight::LOSE => {
                    self.result_played = true;
                    // Deletion cry (`0x265`) plays ~90 frames earlier when HP hits 0;
                    // when `fight::LOSE` begins, play the post-defeat line (`0x266..=0x267`)
                    // unless deletion happened on this exact tick.
                    if !deleted_this_tick {
                        let h = voice_hash(ticks, side, 0x92);
                        audio.play_voice(0x266 + (h & 1) as u16 + offset, CH_NAVI);
                    }
                }
                _ => {}
            }
        }

        self.prev_hp = cur_hp;
        self.prev_total_damage = cur_total_damage;
        self.prev_party_active = cur_party_active;
        self.prev_form = Some(cur_form);
        self.prev_used_chip = cur_used;
        self.prev_telop = cur_telop;
        self.prev_pa_step = cur_pa_step;
        self.prev_danger = cur_danger;
        self.prev_stunned = cur_stunned;
        self.prev_transform_fade = in_transform_fade;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speaker_offsets_and_battle_start_and_shout_match_arm9_tables() {
        assert_eq!(navi_voice_offset("megaman"), Some(0));
        assert_eq!(navi_voice_offset("dark_megaman"), None);
        assert_eq!(navi_voice_offset("protoman"), Some(0x64));
        assert_eq!(navi_voice_offset("colonel"), Some(0xC8));
        assert_eq!(navi_voice_offset("gyroman"), None);
        assert_eq!(navi_voice_offset("tomahawkman"), None);

        // ARM9 0x020CBAC4 Battle Start Navi callouts
        assert_eq!(battle_start_voice("megaman", 0), Some(0x261));
        assert_eq!(battle_start_voice("megaman", 3), Some(0x264));
        assert_eq!(battle_start_voice("protoman", 0), Some(0x2C5));
        assert_eq!(battle_start_voice("protoman", 2), Some(0x2C7));
        assert_eq!(battle_start_voice("protoman", 3), Some(0x2C5));
        assert_eq!(battle_start_voice("colonel", 0), Some(0x329));
        assert_eq!(battle_start_voice("colonel", 2), Some(0x32B));
        assert_eq!(battle_start_voice("dark_megaman", 0), None);
        assert_eq!(battle_start_voice("gyroman", 0), None);

        // OV423 0x02223D10 Danger Shout cries by strength tier (0, 1, 2)
        assert_eq!(danger_shout_voice("megaman", 0), Some(0x289));
        assert_eq!(danger_shout_voice("megaman", 1), Some(0x28A));
        assert_eq!(danger_shout_voice("megaman", 2), Some(0x286));
        assert_eq!(danger_shout_voice("protoman", 0), Some(0x2EE));
        assert_eq!(danger_shout_voice("protoman", 1), Some(0x2ED));
        assert_eq!(danger_shout_voice("protoman", 2), Some(0x2EC));
        assert_eq!(danger_shout_voice("colonel", 0), Some(0x352));
        assert_eq!(danger_shout_voice("colonel", 1), Some(0x350));
        assert_eq!(danger_shout_voice("colonel", 2), Some(0x34E));
        assert_eq!(danger_shout_voice("dark_megaman", 2), None);
        assert_eq!(danger_shout_voice("tomahawkman", 2), None);

        // ARM9 0x02098AE0 Status/stun cry (6 variants)
        assert_eq!(status_cry_voice(0, 0), 0x277);
        assert_eq!(status_cry_voice(0, 5), 0x27C);
        assert_eq!(status_cry_voice(0x64, 1), 0x2DC);
        assert_eq!(status_cry_voice(0xC8, 2), 0x341);
    }

    #[test]
    fn damage_tiers_match_arm9_thresholds() {
        // Chip cries: <=80, <=139, <=200, >200
        assert_eq!(chip_cry_voice(80, 0, 0), 0x27F);
        assert_eq!(chip_cry_voice(81, 0, 0), 0x282);
        assert_eq!(chip_cry_voice(139, 0, 0), 0x282);
        assert_eq!(chip_cry_voice(140, 0, 0), 0x285);
        assert_eq!(chip_cry_voice(200, 0, 0), 0x285);
        assert_eq!(chip_cry_voice(201, 0, 0), 0x288);
        assert_eq!(chip_cry_voice(50, 0x64, 2), 0x2E5);
        assert_eq!(chip_cry_voice(300, 0xC8, 1), 0x351);

        // Hurt cries: <70, <150, >=150
        assert_eq!(hurt_cry_voice(69, 0, 0), 0x26E);
        assert_eq!(hurt_cry_voice(70, 0, 0), 0x271);
        assert_eq!(hurt_cry_voice(149, 0, 0), 0x271);
        assert_eq!(hurt_cry_voice(150, 0, 0), 0x274);
        assert_eq!(hurt_cry_voice(150, 0x64, 2), 0x2DA);
        assert_eq!(hurt_cry_voice(150, 0xC8, 2), 0x33E);
    }

    #[test]
    fn all_souls_navis_and_program_advances_map_to_ssar_ids() {
        let souls = [
            "protosoul", "magnetsoul", "gyrosoul", "napalmsoul", "searchsoul", "meddysoul",
            "colonelsoul", "knightsoul", "shadowsoul", "tomahawksoul", "numbersoul", "toadsoul",
        ];
        for (i, s) in souls.iter().enumerate() {
            assert_eq!(soul_voice(s, false), Some(0x1FC + i as u16));
            assert_eq!(soul_voice(s, true), Some(0x208 + i as u16));
        }

        let navis = [
            "megaman", "protoman", "magnetman", "gyroman", "napalmman", "searchman", "meddy",
            "colonel", "knightman", "shadowman", "tomahawkman", "numberman", "toadman",
        ];
        for (i, n) in navis.iter().enumerate() {
            assert_eq!(navi_switch_voices(n), Some((0x214 + i as u16, 0x221 + i as u16)));
        }

        let pas = [
            ("gigacan1", 0x23F),
            ("infvulc2", 0x240),
            ("boxer3", 0x241),
            ("suprspr1", 0x242),
            ("shakpar2", 0x243),
            ("cacdanc3", 0x244),
            ("h-burst", 0x245),
            ("lifesrd", 0x246),
            ("greatyo", 0x247),
            ("pithoky", 0x248),
            ("poisphar", 0x249),
            ("timebom", 0x24A),
            ("bodygrd", 0x24B),
            ("piledrvr", 0x24C),
            ("csmopris", 0x24D),
            ("wildbird", 0x24E),
            ("football", 0x24F),
            ("bignoise", 0x250),
        ];
        for (key, expected) in pas {
            assert_eq!(pa_voice(key, ChipClass::ProgramAdvance), Some(expected));
        }
        assert_eq!(pa_voice("unknown_pa", ChipClass::ProgramAdvance), Some(0x23E));
        assert_eq!(pa_voice("cannon", ChipClass::Standard), None);
    }

    #[test]
    fn voice_observer_triggers_on_exe5_battle_without_affecting_digest() {
        use crate::driver::{InFight, LivePlayer, Opponent};
        use crate::player::Player;
        use saito_audio::{BattleAudio, Songs};
        use saito_battle::battle::mode;
        use saito_match::Set;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        let content = saito_match::testing::exe5_content();
        let m = saito_match::pick::live(&content, "exe5", 11, None).expect("exe5 live match");
        let set_a = Set::of(&content, &m, 11);
        let set_b = Set::of(&content, &m, 11);

        // Minimal silent M4A bank + synthetic DS voice bank containing Battle Start + chip cries.
        let m4a_bank = Arc::new(m4a::SoundBank {
            mixer: m4a::bank::MixerConfig {
                mix_rate: 13_379,
                ds_channels: 4,
                master_volume: 15,
                reverb: 0,
                dac_resolution: 1,
            },
            players: Vec::new(),
            songs: Vec::new(),
            voicegroups: Vec::new(),
            key_maps: Vec::new(),
            samples: Vec::new(),
            waves: Vec::new(),
        });
        let songs = Songs::numbers(content.assets.sounds.len().max(1));
        let wave = sdat::Wave {
            samples: (0..2048).map(|i| if (i / 12) % 2 == 0 { 14000 } else { -14000 }).collect(),
            sample_rate: 11_025,
            looped: false,
            loop_start: 0,
            loop_end: 2048,
        };
        let mut clips = BTreeMap::new();
        for id in (0x261u16..=0x28A)
            .chain(0x2C5..=0x2EE)
            .chain(0x329..=0x352)
        {
            clips.insert(id, sdat::VoiceClip { volume: 88, steps: vec![sdat::VoiceStep { wave: 0, delay_samples: 0 }] });
        }
        let voice_bank = Arc::new(sdat::VoiceBank { waves: vec![wave], clips });

        let audio_silent = BattleAudio::new(m4a_bank.clone(), songs.clone());
        let mut audio_voiced = BattleAudio::new(m4a_bank, songs);
        audio_voiced.set_voice_bank(Some(voice_bank));

        set_enabled(true);
        let opp = Opponent { in_fight: InFight::AttacksAtRandom, ..Opponent::default() };
        let mut p_silent = Player::with(
            crate::testing::graphics(&content, 0x7FFF).renderer(crate::game::Enhancements::NONE, None),
            None,
            Some(audio_silent),
            Box::new(LivePlayer::against(set_a, opp.clone(), 5)),
        );
        let mut p_voiced = Player::with(
            crate::testing::graphics(&content, 0x7FFF).renderer(crate::game::Enhancements::NONE, None),
            None,
            Some(audio_voiced),
            Box::new(LivePlayer::against(set_b, opp, 5)),
        );

        let (mut samples_silent, mut samples_voiced) = (Vec::new(), Vec::new());
        let mut custom_step = 0u8;
        for tick in 0..350u32 {
            // Pick first chip and confirm custom screen once it finishes sliding in.
            let choosing = p_silent
                .battle()
                .presented(0)
                .is_some_and(|p| p.screen.phase == saito_battle::custom::Phase::Choosing);
            if choosing {
                if custom_step == 0 {
                    p_silent.choose(saito_battle::ScreenChoice::Pick(0));
                    p_voiced.choose(saito_battle::ScreenChoice::Pick(0));
                    custom_step = 1;
                } else if custom_step == 1 {
                    p_silent.choose(saito_battle::ScreenChoice::Ok);
                    p_voiced.choose(saito_battle::ScreenChoice::Ok);
                    custom_step = 2;
                }
            } else {
                custom_step = 0;
            }
            let btn = if p_silent.battle().round.mode == mode::FIGHTING && tick % 10 == 0 {
                saito_battle::input::keys::A
            } else {
                0
            };
            assert!(p_silent.tick(btn));
            assert!(p_voiced.tick(btn));
            p_silent.take_samples(&mut samples_silent);
            p_voiced.take_samples(&mut samples_voiced);
        }

        assert!(p_voiced.battle().round.turn >= 1, "fight must have started");
        // Strict simulation isolation: digests must be 100% bit-identical!
        assert_eq!(p_silent.battle().digest(), p_voiced.battle().digest(), "DS voices must never alter Battle digest");
        let peak_silent = samples_silent.iter().flat_map(|s| *s).map(f32::abs).fold(0.0_f32, f32::max);
        let peak_voiced = samples_voiced.iter().flat_map(|s| *s).map(f32::abs).fold(0.0_f32, f32::max);
        assert_eq!(peak_silent, 0.0, "silent M4A bank without voices stays silent");
        assert!(peak_voiced > 0.1, "EXE5 battle must trigger audible DS voices, peak={peak_voiced}");
    }
}
