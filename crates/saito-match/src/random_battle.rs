//! Official Mega Man Battle Network 6 (EXE6) Random Battle (ランダムバトル) mode.
//!
//! Faithfully reproduces the GBA original (`asm36.s` 0x08131588 / 0x081318D4–0x08131EE4 / 0x0800B144):
//! - **Ranks 1–3**: fixed MegaMan stats (no Crosses, no Beast Out, no NaviCust/Patch Cards,
//!   SP chip deletion times set to 0x0474 = 1140 frames / 19.00s, `effects::RANDOM` set).
//!   - Rank 1: 800 HP, Buster 1/1/1, Custom 6, SuperArmor + UnderShirt
//!   - Rank 2: 1000 HP, Buster 2/2/2, Custom 7, SuperArmor + AirShoes
//!   - Rank 3: 1200 HP, Buster 2/2/2, Custom 7, ChipShuffle + Shield (`B + Back`)
//! - **40 official 10-chip packs** combined into a shared 30-chip folder for both players:
//!   - Rank 1: 3 distinct packs from Tier A (16 packs)
//!   - Rank 2: 2 distinct packs from Tier A + 1 pack from Tier B (16 packs)
//!   - Rank 3: 1 pack from Tier A + 1 pack from Tier B + 1 pack from Tier C (8 packs),
//!     with 50/50 Gregar vs Falzar Giga swap (`byte_8131BF4` <-> `byte_8131BFC`) determined by seed.

use crate::folders::Folder;
use crate::{Picks, Side, ids};
use saito_battle::content::{ChipCode, Content};
use saito_battle::custom::FolderChip;
use saito_battle::rules::Fact;
use saito_battle::setup::NaviStats;
use saito_content_api::{Registry, Value};

/// EXE6 Random Battle rank (Rank 1, Rank 2, or Rank 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RandomRank {
    Rank1 = 1,
    Rank2 = 2,
    Rank3 = 3,
}

impl RandomRank {
    pub const ALL: [RandomRank; 3] = [RandomRank::Rank1, RandomRank::Rank2, RandomRank::Rank3];

    pub fn from_u8(v: u8) -> Option<RandomRank> {
        match v {
            1 => Some(RandomRank::Rank1),
            2 => Some(RandomRank::Rank2),
            3 => Some(RandomRank::Rank3),
            _ => None,
        }
    }

    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Fixed MegaMan HP for this rank (0x320 = 800, 0x3E8 = 1000, 0x4B0 = 1200).
    pub fn hp(self) -> u16 {
        match self {
            RandomRank::Rank1 => 800,
            RandomRank::Rank2 => 1000,
            RandomRank::Rank3 => 1200,
        }
    }

    /// Fixed Custom screen hand size for this rank (6, 7, 7).
    pub fn custom_level(self) -> u8 {
        match self {
            RandomRank::Rank1 => 6,
            RandomRank::Rank2 | RandomRank::Rank3 => 7,
        }
    }

    /// Fixed Buster Attack / Rapid / Charge levels for this rank (1/1/1, 2/2/2, 2/2/2).
    pub fn buster_level(self) -> u8 {
        match self {
            RandomRank::Rank1 => 1,
            RandomRank::Rank2 | RandomRank::Rank3 => 2,
        }
    }
}

/// Number of stages in EXE6 `BattleSettingsList1` eligible for Random Battle (`sub_81209DC(2)`: 0x44 = 68).
pub const RANDOM_BATTLE_STAGES: usize = 68;

/// SP navi deletion time filled by `sub_800B144` in Random Battle (`0x0474` = 1140 frames = 19.00s).
pub const RANDOM_SP_FRAMES: u16 = 0x0474;

/// Whether `game` supports official Random Battle mode (EXE6 only).
pub fn supported(game: &str) -> bool {
    game == "exe6"
}

const SP_CHIPS: [&str; 18] = [
    "protomn-sp", "heatman-sp", "elecman-sp", "slashmn-sp", "erasemn-sp", "chrgemn-sp",
    "spoutmn-sp", "tmhkman-sp", "tengumn-sp", "grndman-sp", "dustman-sp", "blastmn-sp",
    "diveman-sp", "crcusmn-sp", "judgemn-sp", "elmntmn-sp", "colonel-sp", "count-sp",
];

type Pack = [(&'static str, char); 10];

/// Tier A (16 × 10-chip packs, `byte_81318D4`..`byte_8131A00`).
const TIER_A: [Pack; 16] = [
    [
        ("cannon", 'A'), ("cannon", 'B'), ("cannon", '*'), ("hicannon", 'M'), ("vulcan1", '*'),
        ("vulcan1", '*'), ("spreadr1", '*'), ("spreadr1", '*'), ("yoyo", 'M'), ("airhocky", 'M'),
    ],
    [
        ("firebrn1", '*'), ("firebrn1", '*'), ("blkbomb", 'F'), ("fireswrd", 'F'), ("firehit1", 'F'),
        ("firehit1", 'F'), ("burnsqr1", 'H'), ("burnsqr1", 'H'), ("boomer", 'K'), ("boomer", 'K'),
    ],
    [
        ("spreadr1", '*'), ("bblstar1", 'E'), ("thunder", '*'), ("thunder", '*'), ("dolthdr1", 'E'),
        ("dolthdr1", 'E'), ("elcpuls2", 'E'), ("elecswrd", 'E'), ("wavearm1", 'E'), ("wavearm1", 'E'),
    ],
    [
        ("sword", 'S'), ("sword", '*'), ("wideswrd", 'S'), ("longswrd", 'S'), ("wideblde", 'B'),
        ("longblde", 'B'), ("rflectr1", '*'), ("rflectr1", '*'), ("fanfare", '*'), ("areagrab", '*'),
    ],
    [
        ("trnarrw1", 'K'), ("trnarrw1", 'K'), ("cornsht1", 'K'), ("cornsht1", 'K'), ("rlnglog1", 'K'),
        ("ironshl1", 'K'), ("machgun1", '*'), ("machgun1", '*'), ("iceseed", '*'), ("golmhit1", 'K'),
    ],
    [
        ("tankcan1", 'R'), ("widesht", 'R'), ("tornado", 'R'), ("windrack", '*'), ("airspin1", 'R'),
        ("airspin1", 'R'), ("airraid1", 'R'), ("airraid1", 'R'), ("recov80", '*'), ("roll", 'R'),
    ],
    [
        ("gundels1", '*'), ("gundels1", '*'), ("drilarm", 'M'), ("megenbom", '*'), ("megenbom", '*'),
        ("moonbld", 'M'), ("snake", 'M'), ("recov150", 'M'), ("areagrab", '*'), ("geddon", '*'),
    ],
    [
        ("bigbomb", 'P'), ("bigbomb", 'P'), ("dublshot", '*'), ("trplshot", '*'), ("sumnblk1", 'P'),
        ("sumnblk1", 'P'), ("aquandl1", 'P'), ("aquandl1", 'P'), ("recov50", '*'), ("recov120", 'P'),
    ],
    [
        ("vulcan1", '*'), ("vulcan1", '*'), ("gundels1", '*'), ("gundels1", '*'), ("grasseed", '*'),
        ("crakshot", '*'), ("firehit1", 'D'), ("firehit1", 'D'), ("burnsqr2", 'D'), ("gundels2", 'R'),
    ],
    [
        ("spreadr2", 'C'), ("spreadr2", 'C'), ("aurahed1", 'C'), ("energbom", '*'), ("energbom", '*'),
        ("dublshot", '*'), ("dublshot", '*'), ("aquandl1", 'C'), ("timebom2", 'C'), ("colonel", 'C'),
    ],
    [
        ("bblstar1", 'B'), ("bblstar1", 'B'), ("rskyhny1", 'B'), ("rskyhny1", 'B'), ("aurahed1", 'B'),
        ("aurahed1", 'B'), ("wideblde", 'B'), ("stepswrd", 'B'), ("areagrab", '*'), ("protoman", 'B'),
    ],
    [
        ("flshbom1", '*'), ("flshbom1", '*'), ("airraid1", 'K'), ("discord", '*'), ("panlgrab", '*'),
        ("busterup", '*'), ("busterup", '*'), ("whicapsl", '*'), ("atk-10", '*'), ("atk-10", '*'),
    ],
    [
        ("vulcan1", '*'), ("vulcan1", '*'), ("firebrn1", 'F'), ("firebrn1", 'F'), ("machgun1", '*'),
        ("machgun1", '*'), ("magnum", 'F'), ("windrack", '*'), ("lilbolr1", 'F'), ("lilbolr1", 'F'),
    ],
    [
        ("elcpuls1", 'J'), ("elcpuls1", 'J'), ("ironshl1", 'J'), ("ironshl1", 'J'), ("sensor1", 'J'),
        ("boomer", 'J'), ("boomer", 'J'), ("recov50", '*'), ("panlgrab", '*'), ("judgeman", 'J'),
    ],
    [
        ("tankcan1", 'A'), ("tankcan1", 'A'), ("rflectr1", '*'), ("rflectr1", '*'), ("sandwrm1", 'A'),
        ("sandwrm1", 'A'), ("rockcube", '*'), ("rockcube", '*'), ("recov30", '*'), ("elmntman", 'E'),
    ],
    [
        ("tankcan1", 'G'), ("tankcan1", 'G'), ("airspin1", 'G'), ("airspin1", 'G'), ("airraid1", 'G'),
        ("timebom1", 'G'), ("timebom1", 'G'), ("timpani", '*'), ("colarmy", '*'), ("diveman", 'D'),
    ],
];

/// Tier B (16 × 10-chip packs, `byte_8131A14`..`byte_8131B40`).
const TIER_B: [Pack; 16] = [
    [
        ("hicannon", 'L'), ("hicannon", 'M'), ("hicannon", 'N'), ("m-cannon", 'R'), ("vulcan2", 'L'),
        ("vulcan2", 'L'), ("tankcan2", 'L'), ("tankcan2", 'L'), ("gundels2", 'R'), ("gundels2", 'R'),
    ],
    [
        ("firebrn2", 'S'), ("firebrn2", 'T'), ("firebrn2", 'U'), ("blkbomb", 'F'), ("firehit2", 'S'),
        ("fireswrd", 'F'), ("burnsqr2", 'T'), ("burnsqr2", 'T'), ("areagrab", '*'), ("blastmn-ex", 'B'),
    ],
    [
        ("widesht", 'P'), ("widesht", 'R'), ("bblstar2", 'L'), ("bblstar2", 'L'), ("aquaswrd", 'A'),
        ("aquandl2", 'T'), ("aquandl2", 'T'), ("blzrdbal", 'T'), ("bblwrap", 'Z'), ("diveman-ex", 'D'),
    ],
    [
        ("dolthdr2", 'L'), ("dolthdr2", 'L'), ("elcpuls1", 'J'), ("elcpuls2", 'J'), ("elcpuls2", 'J'),
        ("elecswrd", 'E'), ("elecswrd", 'E'), ("sensor2", 'Y'), ("sensor2", 'Y'), ("judgemn-ex", 'J'),
    ],
    [
        ("cornsht2", 'C'), ("cornsht2", 'D'), ("cornsht2", 'E'), ("rskyhny2", 'C'), ("rskyhny2", 'C'),
        ("rlnglog2", 'E'), ("rlnglog2", 'E'), ("bambswrd", 'H'), ("lance", '*'), ("elmntmn-ex", 'E'),
    ],
    [
        ("sword", '*'), ("sword", '*'), ("wideswrd", 'S'), ("longswrd", 'S'), ("stepswrd", 'L'),
        ("stepswrd", 'L'), ("varswrd", 'V'), ("varswrd", 'V'), ("areagrab", '*'), ("protomn-ex", 'B'),
    ],
    [
        ("tornado", 'L'), ("tornado", 'L'), ("grasseed", '*'), ("windrack", '*'), ("windrack", '*'),
        ("airspin2", 'L'), ("airspin2", 'L'), ("wind", '*'), ("fan", '*'), ("barr100", 'O'),
    ],
    [
        ("machgun2", 'G'), ("machgun2", 'G'), ("machgun2", 'G'), ("moonbld", 'T'), ("flshbom2", 'G'),
        ("magnum", 'W'), ("magnum", 'W'), ("circgun", 'T'), ("circgun", 'T'), ("colonel-ex", 'C'),
    ],
    [
        ("lilbolr2", 'E'), ("lilbolr2", 'E'), ("airraid2", 'T'), ("timebom2", 'C'), ("timebom2", 'D'),
        ("timebom2", 'E'), ("mine", 'T'), ("mine", 'T'), ("fanfare", '*'), ("gundels3", 'N'),
    ],
    [
        ("ironshl2", 'C'), ("ironshl2", 'D'), ("ironshl2", 'E'), ("aurahed2", 'E'), ("aurahed2", 'E'),
        ("drilarm", 'G'), ("drilarm", 'G'), ("golmhit2", 'D'), ("golmhit2", 'D'), ("protomn-ex", 'B'),
    ],
    [
        ("rflectr2", '*'), ("wavearm2", 'L'), ("wavearm2", 'M'), ("wavearm2", 'N'), ("snake", 'M'),
        ("snake", 'M'), ("panlgrab", '*'), ("areagrab", '*'), ("geddon", '*'), ("crcusmn-ex", 'C'),
    ],
    [
        ("recov150", 'M'), ("holypanl", '*'), ("holypanl", '*'), ("invisibl", '*'), ("barr100", 'H'),
        ("barr100", 'H'), ("atk-10", '*'), ("atk-10", '*'), ("navi-20", '*'), ("roll2", 'R'),
    ],
    [
        ("rockcube", '*'), ("rockcube", '*'), ("colarmy", 'G'), ("colarmy", 'G'), ("elemtrap", 'G'),
        ("antidmg", '*'), ("antidmg", '*'), ("antiswrd", '*'), ("antirecv", 'V'), ("colonel-ex", 'C'),
    ],
    [
        ("spreadr2", 'A'), ("spreadr2", 'B'), ("spreadr2", 'C'), ("energbom", '*'), ("energbom", '*'),
        ("bugbomb", 'V'), ("sandwrm2", 'B'), ("sandwrm2", 'B'), ("grabbnsh", 'B'), ("gundels3", 'N'),
    ],
    [
        ("trplshot", '*'), ("sumnblk2", 'H'), ("sumnblk2", 'H'), ("blzrdbal", 'H'), ("timpani", '*'),
        ("timpani", '*'), ("vdoll", 'F'), ("vdoll", 'F'), ("fstgauge", '*'), ("whicapsl", '*'),
    ],
    [
        ("yoyo", 'L'), ("yoyo", 'N'), ("widesht", 'Q'), ("trnarrw2", 'M'), ("trnarrw2", 'M'),
        ("airhocky", 'M'), ("airhocky", 'M'), ("megenbom", '*'), ("megenbom", '*'), ("iceseed", '*'),
    ],
];

/// Tier C (8 × 10-chip packs, `byte_8131B54`..`byte_8131BE0`).
const TIER_C: [Pack; 8] = [
    [
        ("m-cannon", '*'), ("suprvulc", 'V'), ("tankcan3", 'M'), ("firebrn3", 'D'), ("dolthdr3", 'R'),
        ("invisibl", '*'), ("whicapsl", '*'), ("roll3", 'R'), ("diveman-sp", 'D'), ("bighook", 'H'),
    ],
    [
        ("vulcan3", 'G'), ("gundels3", 'Q'), ("sensor3", 'Q'), ("varswrd", 'V'), ("antidmg", '*'),
        ("colarmy", '*'), ("lifeaur", 'U'), ("diveman-sp", 'D'), ("elmntmn-sp", 'E'), ("deltaray", 'Z'),
    ],
    [
        ("machgun3", 'M'), ("elcpuls3", 'J'), ("cornsht3", 'R'), ("ironshl3", 'M'), ("muramasa", 'M'),
        ("barr200", 'U'), ("whicapsl", '*'), ("judgemn-sp", 'J'), ("roll3", 'R'), ("bass", 'F'),
    ],
    [
        ("ironshl3", 'N'), ("machgun3", 'B'), ("assnswrd", 'Y'), ("burnsqr3", 'E'), ("justcone", 'J'),
        ("recov300", 'Y'), ("whicapsl", '*'), ("blastmn-sp", 'B'), ("elmntmn-sp", 'E'), ("bighook", 'H'),
    ],
    [
        ("sumnblk3", 'Z'), ("firehit3", 'A'), ("firehit3", 'B'), ("firehit3", 'C'), ("golmhit3", 'V'),
        ("antidmg", '*'), ("uninstll", 'R'), ("protomn-sp", 'B'), ("colonel-sp", 'C'), ("colforce", 'Q'),
    ],
    [
        ("varswrd", 'V'), ("neovari", 'N'), ("elecdrgn", 'V'), ("airraid3", 'Z'), ("discord", '*'),
        ("colarmy", '*'), ("antirecv", '*'), ("protomn-sp", 'B'), ("blastmn-sp", 'B'), ("deltaray", 'Z'),
    ],
    [
        ("burnsqr3", 'E'), ("m-boomer", 'I'), ("elecdrgn", 'L'), ("fanfare", '*'), ("invisibl", '*'),
        ("whicapsl", '*'), ("neovari", 'N'), ("protomn-sp", 'B'), ("diveman-sp", 'D'), ("bass", 'F'),
    ],
    [
        ("aurahed3", 'G'), ("flshbom3", 'S'), ("elemswrd", 'M'), ("firehit3", 'B'), ("guardian", 'O'),
        ("recov300", 'O'), ("barr200", 'U'), ("crcusmn-sp", 'C'), ("colonel-sp", 'C'), ("colforce", 'Q'),
    ],
];

/// Gregar -> Falzar Giga chip swap table (`byte_8131BF4` -> `byte_8131BFC`).
const FALZAR_GIGA_SWAP: [((&'static str, char), (&'static str, char)); 4] = [
    (("bass", 'F'), ("bassanly", 'F')),
    (("bighook", 'H'), ("metrknuk", 'N')),
    (("deltaray", 'Z'), ("crossdiv", 'D')),
    (("colforce", 'Q'), ("hubbatc", 'J')),
];

/// Build the shared 30-chip Random Battle folder for `rank` deterministically from `seed`.
pub fn build_folder(content: &Content, rank: RandomRank, seed: u32) -> Result<Folder, String> {
    let mut picks = Picks::new(seed ^ 0x5241_4E44);
    let mut chosen: Vec<&Pack> = Vec::with_capacity(3);
    match rank {
        RandomRank::Rank1 => {
            let mut indices: Vec<usize> = (0..TIER_A.len()).collect();
            picks.shuffle(&mut indices);
            for &i in &indices[..3] {
                chosen.push(&TIER_A[i]);
            }
        }
        RandomRank::Rank2 => {
            let first = picks.below(TIER_A.len());
            let mut second = picks.below(TIER_A.len() - 1);
            if second >= first {
                second += 1;
            }
            let third = picks.below(TIER_B.len());
            chosen.push(&TIER_A[first]);
            chosen.push(&TIER_A[second]);
            chosen.push(&TIER_B[third]);
        }
        RandomRank::Rank3 => {
            chosen.push(&TIER_A[picks.below(TIER_A.len())]);
            chosen.push(&TIER_B[picks.below(TIER_B.len())]);
            chosen.push(&TIER_C[picks.below(TIER_C.len())]);
        }
    }
    let use_falzar_gigas = rank == RandomRank::Rank3 && (picks.next() & 1) == 1;
    let mut chips: [Option<FolderChip>; 30] = [None; 30];
    let mut slot = 0;
    for pack in chosen {
        for &(mut key, mut code_ch) in pack {
            if use_falzar_gigas {
                for &(gregar, falzar) in &FALZAR_GIGA_SWAP {
                    if gregar == (key, code_ch) {
                        key = falzar.0;
                        code_ch = falzar.1;
                        break;
                    }
                }
            }
            let id = ids::chip(content, "exe6", key)
                .ok_or_else(|| format!("random battle: unknown EXE6 chip {key:?}"))?;
            let code = ChipCode::from_letter(code_ch)
                .ok_or_else(|| format!("random battle: invalid chip code {code_ch:?}"))?;
            chips[slot] = Some(FolderChip::new(id, code));
            slot += 1;
        }
    }
    Ok(Folder { chips, regular: None, tags: None })
}

/// Build a Random Battle [`Side`] holding `folder`, MegaMan, no NaviCust/Patch Cards/Crosses/Beast Out,
/// and all SP chip deletion times set to [`RANDOM_SP_FRAMES`] (19.00s).
pub fn build_side(content: &Content, folder: &Folder) -> Result<Side, String> {
    let mut side = Side::fresh(content, "exe6")?;
    let megaman = crate::first_navi(content, "exe6")
        .ok_or_else(|| "random battle: exe6 has no MegaMan navi".to_string())?;
    side.set_navi(content, megaman)?;
    // Cybeast is required by EXE6 setup schema, though Beast Out is disabled by `effects::RANDOM`.
    if let Some(form) = ids::form(content, "exe6", "gregar-beast") {
        let _ = side.set_fact(content, "cybeast", &[Fact::Value(Value::Def(Registry::Form, form.0))]);
    }
    let _ = side.set_fact(content, "crosses", &[]);
    let _ = side.set_fact(content, "beast_out", &[Fact::Value(Value::Bool(false))]);
    let _ = side.set_fact(content, "navicust_expansions", &[Fact::Value(Value::Nil)]);
    let _ = side.set_fact(content, "navicust_programs", &[]);
    let _ = side.set_fact(content, "patch_cards", &[]);
    let sp_records: Vec<Fact> = SP_CHIPS
        .iter()
        .filter_map(|key| ids::chip(content, "exe6", key))
        .map(|c| {
            Fact::Record(vec![
                ("chip", Fact::Value(Value::Def(Registry::Chip, c.0))),
                ("frames", Fact::Value(Value::Int(RANDOM_SP_FRAMES as i64))),
            ])
        })
        .collect();
    let _ = side.set_fact(content, "sp_times", &sp_records);
    side.set_folder(content, folder)?;
    Ok(side)
}

/// Build both players' identical Random Battle sides from `rank` and `seed`.
pub fn build_match_sides(content: &Content, rank: RandomRank, seed: u32) -> Result<[Side; 2], String> {
    let folder = build_folder(content, rank, seed)?;
    let side = build_side(content, &folder)?;
    Ok([side.clone(), side])
}

/// Apply the official EXE6 Random Battle rank stats (`sub_8131588` / `off_8131600`) onto `stats`.
pub fn apply_rank_stats(content: &Content, rank: RandomRank, stats: &mut NaviStats) {
    let hp = rank.hp();
    stats.hp = hp;
    stats.max_hp = hp;
    stats.max_base_hp = hp;
    stats.navi_variant = (hp / 100) as u8;
    let buster = rank.buster_level();
    stats.attack = buster;
    stats.rapid = buster;
    stats.charge = buster;
    stats.custom_level = rank.custom_level();
    match rank {
        RandomRank::Rank1 => {
            stats.super_armor = true;
            stats.undershirt = true;
        }
        RandomRank::Rank2 => {
            stats.super_armor = true;
            stats.air_shoes = true;
        }
        RandomRank::Rank3 => {
            let _ = stats.set_game_stat(content, "chip_shuffle", Value::Bool(true));
            if let Some(shield) = content.defs.weapon_by_key("megaman/shield").or_else(|| content.defs.weapon_by_key("exe6:megaman/shield")) {
                stats.weapons.back_special = Some(shield);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::exe6_content;
    use saito_battle::Battle;

    #[test]
    fn all_40_official_packs_and_giga_swaps_resolve_in_exe6_content() {
        let content = exe6_content();
        for (i, pack) in TIER_A.iter().chain(TIER_B.iter()).chain(TIER_C.iter()).enumerate() {
            for &(key, code) in pack {
                assert!(ids::chip(&content, "exe6", key).is_some(), "pack {i}: missing chip {key}");
                assert!(ChipCode::from_letter(code).is_some(), "pack {i}: bad code {code}");
            }
        }
        for &(g, f) in &FALZAR_GIGA_SWAP {
            assert!(ids::chip(&content, "exe6", g.0).is_some(), "missing Gregar giga {}", g.0);
            assert!(ids::chip(&content, "exe6", f.0).is_some(), "missing Falzar giga {}", f.0);
        }
        for key in SP_CHIPS {
            assert!(ids::chip(&content, "exe6", key).is_some(), "missing SP chip {key}");
        }
        assert!(content.defs.weapon_by_key("megaman/shield").is_some());
    }

    #[test]
    fn random_battle_ranks_apply_exact_stats_and_effects() {
        let content = exe6_content();
        for rank in RandomRank::ALL {
            let sides = build_match_sides(&content, rank, 42).unwrap();
            assert_eq!(sides[0], sides[1]);
            let m = crate::Match {
                game: "exe6".into(),
                seed: Some(42),
                rounds: vec![crate::RoundSettings::default(); 3],
                sides,
                backgrounds: crate::BackgroundPick::Game,
                custom_screen_time: Some(60),
                random_battle: Some(rank),
                abd_mode: crate::AbdMode::Original,
            };
            let round = m.round(&content, 42);
            assert_ne!(round.settings.effects & saito_battle::setup::effects::RANDOM, 0);
            let b = Battle::new(round, content.clone());
            for side in 0..2 {
                let st = &b.stats[side];
                assert_eq!((st.hp, st.max_hp, st.max_base_hp), (rank.hp(), rank.hp(), rank.hp()));
                assert_eq!((st.attack, st.rapid, st.charge), (rank.buster_level(), rank.buster_level(), rank.buster_level()));
                assert_eq!(st.custom_level, rank.custom_level());
                match rank {
                    RandomRank::Rank1 => assert!(st.super_armor && st.undershirt && !st.air_shoes),
                    RandomRank::Rank2 => assert!(st.super_armor && st.air_shoes && !st.undershirt),
                    RandomRank::Rank3 => {
                        assert!(!st.super_armor && !st.air_shoes && !st.undershirt);
                        assert_eq!(st.weapons.back_special, content.defs.weapon_by_key("megaman/shield"));
                    }
                }
            }
            // Binary roundtrip preserves random_battle rank.
            let bytes = crate::binary::match_bytes(&content, &m).unwrap();
            let back = crate::binary::read_match(&content, &bytes).unwrap();
            assert_eq!(back.random_battle, Some(rank));
        }
    }
}
