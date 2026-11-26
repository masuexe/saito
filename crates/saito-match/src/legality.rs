//! Build legality data read from a game's `rules/legality` module:
//! - `unobtainable`: chip+code or NaviCust program+color combinations that exist in ROM tables
//!   (so replays and `validate` still accept them) but cannot be obtained in normal play.
//! - `ds_only`: content legitimately obtainable only in the DS release (EXE5 Double Team DS),
//!   controlled by [`crate::ruleset::Ruleset::allow_ds`].

use crate::ids;
use saito_battle::content::{ChipCode, Content};
use saito_content_api::{ChipHandle, Data, DataKey, EntryHandle};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MODULE: &str = "rules/legality";

/// A set of restricted items (either `unobtainable` or `ds_only`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LegalitySet {
    pub chip_codes: BTreeMap<ChipHandle, BTreeSet<u8>>,
    pub navicust_colors: BTreeMap<EntryHandle, BTreeSet<String>>,
    pub navicust_programs: BTreeSet<EntryHandle>,
    pub patch_cards: BTreeSet<EntryHandle>,
    pub facts: BTreeSet<String>,
}

impl LegalitySet {
    pub fn is_empty(&self) -> bool {
        self.chip_codes.is_empty()
            && self.navicust_colors.is_empty()
            && self.navicust_programs.is_empty()
            && self.patch_cards.is_empty()
            && self.facts.is_empty()
    }

    pub fn has_chip_code(&self, chip: ChipHandle, code: ChipCode) -> bool {
        self.chip_codes.get(&chip).is_some_and(|codes| codes.contains(&code.0))
    }

    pub fn has_navicust_program(&self, program: EntryHandle) -> bool {
        self.navicust_programs.contains(&program)
    }

    pub fn has_navicust_color(&self, program: EntryHandle, color: &str) -> bool {
        self.navicust_programs.contains(&program)
            || self.navicust_colors.get(&program).is_some_and(|colors| colors.contains(color))
    }

    pub fn has_patch_card(&self, card: EntryHandle) -> bool {
        self.patch_cards.contains(&card)
    }

    pub fn has_fact(&self, fact: &str) -> bool {
        self.facts.contains(fact)
    }
}

/// Legality rules for a game's builds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Legality {
    pub unobtainable: LegalitySet,
    pub ds_only: LegalitySet,
}

fn parse_set(content: &Content, game: &str, data: &Data) -> LegalitySet {
    let mut set = LegalitySet::default();
    if let Data::Map(entries) = data.field("chip_codes") {
        for (k, v) in entries {
            let DataKey::Str(name) = k else { continue };
            let Some(chip) = ids::chip(content, game, name) else { continue };
            let Data::List(codes) = v else { continue };
            let bucket = set.chip_codes.entry(chip).or_default();
            for c in codes {
                if let Some(code) = c.str().and_then(|s| s.chars().next()).and_then(ChipCode::from_letter) {
                    bucket.insert(code.0);
                }
            }
        }
    }
    if let Data::Map(entries) = data.field("navicust_colors") {
        for (k, v) in entries {
            let DataKey::Str(name) = k else { continue };
            let Some(entry) = ids::entry(content, game, "navicust_programs", name) else { continue };
            let Data::List(colors) = v else { continue };
            let bucket = set.navicust_colors.entry(entry).or_default();
            for c in colors {
                if let Some(color) = c.str() {
                    bucket.insert(color.to_string());
                }
            }
        }
    }
    if let Data::List(programs) = data.field("navicust_programs") {
        for p in programs {
            if let Some(entry) = p.str().and_then(|name| ids::entry(content, game, "navicust_programs", name)) {
                set.navicust_programs.insert(entry);
            }
        }
    }
    if let Data::List(cards) = data.field("patch_cards") {
        for c in cards {
            if let Some(entry) = c.str().and_then(|name| ids::entry(content, game, "patch_cards", name)) {
                set.patch_cards.insert(entry);
            }
        }
    }
    if let Data::List(facts) = data.field("facts") {
        for f in facts {
            if let Some(name) = f.str() {
                set.facts.insert(name.to_string());
            }
        }
    }
    set
}

impl Legality {
    /// Load `game`'s legality data from `{game}:rules/legality`. Returns an empty [`Legality`]
    /// if the game defines no legality module.
    pub fn of(content: &Arc<Content>, game: &str) -> Legality {
        let module = format!("{game}:{MODULE}");
        let Some(Ok(data)) = saito_battle::behavior::content_module_data(content, &module) else {
            return Legality::default();
        };
        Legality {
            unobtainable: parse_set(content, game, data.field("unobtainable")),
            ds_only: parse_set(content, game, data.field("ds_only")),
        }
    }

    /// Whether the game has any DS-only content controlled by [`crate::ruleset::Ruleset::allow_ds`].
    pub fn has_ds_only(&self) -> bool {
        !self.ds_only.is_empty()
    }

    /// Whether `(chip, code)` is legal to pick in the build editor under `allow_ds`.
    pub fn chip_code_allowed(&self, chip: ChipHandle, code: ChipCode, allow_ds: bool) -> bool {
        !self.unobtainable.has_chip_code(chip, code) && (allow_ds || !self.ds_only.has_chip_code(chip, code))
    }

    /// Whether NaviCust `program` in `color` is legal to pick in the build editor under `allow_ds`.
    pub fn navicust_color_allowed(&self, program: EntryHandle, color: &str, allow_ds: bool) -> bool {
        !self.unobtainable.has_navicust_color(program, color) && (allow_ds || !self.ds_only.has_navicust_color(program, color))
    }

    /// Whether patch card `card` is legal to pick in the build editor under `allow_ds`.
    pub fn patch_card_allowed(&self, card: EntryHandle, allow_ds: bool) -> bool {
        !self.unobtainable.has_patch_card(card) && (allow_ds || !self.ds_only.has_patch_card(card))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{Builds, Checked};
    use crate::ruleset::Rulesets;
    use crate::testing::{PlacedProgram, exe4_content, exe5_content, program_colors, set_navicust};
    use crate::{Wrong, check_match};
    use saito_battle::custom::FolderChip;

    fn place_exe4(content: &Arc<Content>, side: &mut crate::Side, name: &str, color_name: &str) {
        let program = ids::entry(content, "exe4", "navicust_programs", name).unwrap();
        let color = program_colors(content, program).iter().position(|c| *c == color_name).unwrap() as u8;
        let part = PlacedProgram { program, color, x: 3, y: 3, rotation: 0, compressed: false };
        let fact = saito_battle::rules::Fact::Record(vec![
            ("program", saito_battle::rules::Fact::Value(saito_content_api::Value::Def(saito_content_api::Registry::Entry, part.program.0))),
            ("color", saito_battle::rules::Fact::Name(color_name)),
            ("x", saito_battle::rules::Fact::Value(saito_content_api::Value::Int(part.x as i64))),
            ("y", saito_battle::rules::Fact::Value(saito_content_api::Value::Int(part.y as i64))),
            ("rotation", saito_battle::rules::Fact::Value(saito_content_api::Value::Int(0))),
            ("compressed", saito_battle::rules::Fact::Value(saito_content_api::Value::Bool(false))),
        ]);
        side.set_fact(content, "navicust_programs", &[fact]).unwrap();
    }

    #[test]
    fn exe4_unobtainable_navicust_colors_flagged_in_build_check_only() {
        let content = exe4_content();
        let leg = Legality::of(&content, "exe4");
        assert!(!leg.has_ds_only());
        let megfldr1 = ids::entry(&content, "exe4", "navicust_programs", "megfldr1").unwrap();
        assert!(!leg.navicust_color_allowed(megfldr1, "pink", false));
        assert!(leg.navicust_color_allowed(megfldr1, "green", false));

        let mut side = Builds::of(&content, "exe4").unwrap().new_build(&content, "redsun").unwrap();
        place_exe4(&content, &mut side, "megfldr1", "pink");
        // Build check flags it as Unobtainable:
        let checked = Checked::of(&content, "exe4", &side);
        assert!(checked.problems.iter().any(|p| p.what == Wrong::Unobtainable), "{:?}", checked.problems);
        // Match validation (`validate`) still accepts it for replays/recordings:
        let mut m = crate::Match::empty(&content, "exe4").unwrap();
        m.sides = [side.clone(), side];
        assert!(check_match(&content, &m).is_empty(), "{:?}", check_match(&content, &m));
    }

    #[test]
    fn exe5_unobtainable_and_ds_only_legality() {
        let content = exe5_content();
        let r = Rulesets::of(&content, "exe5").unwrap();
        assert!(r.legality.has_ds_only());
        assert!(!r.preset("masters").unwrap().allow_ds);
        assert!(!r.preset("masters_mixed").unwrap().allow_ds);
        assert!(r.preset("masters_5ds").unwrap().allow_ds);
        assert!(r.preset("masters_5ds_mixed").unwrap().allow_ds);
        assert!(r.preset("gate_party").unwrap().allow_ds);

        let star = ChipCode::from_letter('*').unwrap();
        // 1. Unobtainable chip code (drksonic *) is always flagged by build_check, even under 5DS ruleset:
        let drksonic = ids::chip(&content, "exe5", "drksonic").unwrap();
        assert!(!r.legality.chip_code_allowed(drksonic, star, true));
        assert!(r.legality.chip_code_allowed(drksonic, ChipCode::from_letter('E').unwrap(), false));
        let mut side = Builds::of(&content, "exe5").unwrap().new_build(&content, "protoman").unwrap();
        let mut folder = side.folder(&content);
        folder.chips[0] = Some(FolderChip::new(drksonic, star));
        side.set_folder(&content, &folder).unwrap();
        let checked_5ds = Checked::under(&content, "exe5", &side, &r, r.preset("masters_5ds").unwrap());
        assert!(checked_5ds.problems.iter().any(|p| p.what == Wrong::Unobtainable && p.entry == Some(0)), "{:?}", checked_5ds.problems);

        // 2. DS-only chip code (lavaseed *, numbrbl *, drkrecov *) is rejected in GBA and allowed in 5DS:
        for name in ["elemrage", "lavaseed", "sword", "wavepit", "redwave", "mudwave", "numbrbl", "drkrecov"] {
            let chip = ids::chip(&content, "exe5", name).unwrap();
            assert!(!r.legality.chip_code_allowed(chip, star, false), "{name} * should be hidden in GBA");
            assert!(r.legality.chip_code_allowed(chip, star, true), "{name} * should be allowed in 5DS");
        }
        let lavaseed = ids::chip(&content, "exe5", "lavaseed").unwrap();
        let mut ds_side = Builds::of(&content, "exe5").unwrap().new_build(&content, "protoman").unwrap();
        let mut f2 = ds_side.folder(&content);
        f2.chips[0] = Some(FolderChip::new(lavaseed, star));
        ds_side.set_folder(&content, &f2).unwrap();
        let under_gba = Checked::under(&content, "exe5", &ds_side, &r, r.preset("masters").unwrap());
        assert!(under_gba.problems.iter().any(|p| p.what == Wrong::DsOnly && p.entry == Some(0)), "{:?}", under_gba.problems);
        let under_5ds = Checked::under(&content, "exe5", &ds_side, &r, r.preset("masters_5ds").unwrap());
        assert!(under_5ds.problems.is_empty(), "{:?}", under_5ds.problems);

        // 3. DS-only NaviCust color (hp-500 yellow) and program (spport): rejected in GBA, allowed in 5DS;
        //    Unobtainable NaviCust program (hubbatc white): rejected in both.
        let hp500 = ids::entry(&content, "exe5", "navicust_programs", "hp-500").unwrap();
        let yellow = program_colors(&content, hp500).iter().position(|c| *c == "yellow").unwrap() as u8;
        let mut nc_side = Builds::of(&content, "exe5").unwrap().new_build(&content, "protoman").unwrap();
        set_navicust(&content, &mut nc_side, &[PlacedProgram { program: hp500, color: yellow, x: 3, y: 3, rotation: 0, compressed: false }], 2);
        assert!(Checked::under(&content, "exe5", &nc_side, &r, r.preset("masters").unwrap()).problems.iter().any(|p| p.what == Wrong::DsOnly));
        assert!(Checked::under(&content, "exe5", &nc_side, &r, r.preset("masters_5ds").unwrap()).problems.is_empty());

        let hubbatc = ids::entry(&content, "exe5", "navicust_programs", "hubbatc").unwrap();
        assert!(!r.legality.navicust_color_allowed(hubbatc, "white", true));
        let mut hub_side = Builds::of(&content, "exe5").unwrap().new_build(&content, "protoman").unwrap();
        set_navicust(&content, &mut hub_side, &[PlacedProgram { program: hubbatc, color: 0, x: 3, y: 3, rotation: 0, compressed: false }], 2);
        assert!(Checked::under(&content, "exe5", &hub_side, &r, r.preset("masters_5ds").unwrap()).problems.iter().any(|p| p.what == Wrong::Unobtainable));

        // 4. DS-only patch card (sol-cross-megaman): hidden in GBA, allowed in 5DS.
        let sol_cross = ids::entry(&content, "exe5", "patch_cards", "sol-cross-megaman").unwrap();
        assert!(!r.legality.patch_card_allowed(sol_cross, false));
        assert!(r.legality.patch_card_allowed(sol_cross, true));
    }
}
