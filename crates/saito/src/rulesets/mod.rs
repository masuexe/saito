//! Rulesets (docs/design/app-ui.md §2.59; docs/app.md §8): what may be played in a match, the game's presets and
//! the player's own. What any screen needs of them is here:
//!
//! - the player's own on disk ([`store`]: a file each, by game, as builds are kept);
//! - what a player can pick of a game, in order ([`pickable`]: the presets, then their own by name) and a
//!   ruleset's name in the language shown ([`shown_name`]);
//! - the ruleset a build is made for ([`made_for`]: the one its file names, `crate::builds::store::Build::ruleset`),
//!   which it is checked under outside a lobby, and a ruleset's name as a build's file and the settings write it
//!   ([`name_text`], [`name_of_text`]); the player's own renamed, renamed in the builds made for it ([`rename`]);
//! - the ruleset the player last chose, one for each game, remembered ([`chosen_last`], [`remember`]): the fallback a
//!   lobby starts on where no build says;
//! - a ruleset's page ([`page`]), which the lobby and the build editor open over themselves.
//!
//! What a ruleset is, what a game's categories, groups and presets are and what a build breaks under one are
//! saito-match's `ruleset`; the app names no category, no version and no preset.

use crate::builds::Kit;
use crate::games::Names;
use crate::shell::Shell;
use saito_match::ruleset::{Name, Ruleset, TEXT_TABLE};
use std::path::PathBuf;

pub mod page;
pub mod store;

/// The most letters a ruleset's name has.
pub const NAME_LENGTH: usize = 32;

/// A ruleset a player can pick: a preset (no file), or their own (its file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pickable {
    pub ruleset: Ruleset,
    pub path: Option<PathBuf>,
}

/// The rulesets a player can pick of `kit`'s game, in order: the game's presets, then the player's own by name
/// (one whose file doesn't read as one is left out; why is in the log). None where the game says nothing of its
/// rulesets.
pub fn pickable(kit: &Kit) -> Vec<Pickable> {
    let Some(rulesets) = kit.rulesets() else { return Vec::new() };
    let mut out: Vec<Pickable> = rulesets.presets.iter().map(|p| Pickable { ruleset: p.clone(), path: None }).collect();
    for listed in store::list(&kit.game) {
        match store::read(&rulesets, kit.content(), &listed.text) {
            Ok(ruleset) => out.push(Pickable { ruleset, path: Some(listed.path) }),
            Err(why) => crate::errors::log(&format!("reading the ruleset {}", listed.path.display()), &why),
        }
    }
    out
}

/// A ruleset's name as a player reads it: a preset's in the game's strings in `names`' language (its id where
/// they have none), the player's own as they named it.
pub fn shown_name(names: &Names, name: &Name) -> String {
    match name {
        Name::Preset(id) => name.text_key().and_then(|k| names.text(TEXT_TABLE, &k)).unwrap_or_else(|| id.clone()),
        Name::Theirs => name.text_key().and_then(|k| names.text(TEXT_TABLE, &k)).unwrap_or_default(),
        Name::Own(own) => own.clone(),
    }
}

/// The ruleset a build is made for (`Build::ruleset`), as it is found among those the player can pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MadeFor {
    /// The one it is checked under: the one it names, else the game's first preset.
    pub pick: Pickable,
    /// The name it names, where no ruleset has it now (the player's own, deleted, or renamed outside the app): it
    /// is shown with "(missing)", and the build is checked under the game's first preset.
    pub missing: Option<Name>,
}

impl MadeFor {
    /// The name the build names: the missing one, else its pick's.
    pub fn name(&self) -> &Name {
        self.missing.as_ref().unwrap_or(&self.pick.ruleset.name)
    }

    /// Its name as a player reads it ([`shown_name`]), and whether it is missing.
    pub fn shown(&self, names: &Names) -> (String, bool) {
        (shown_name(names, self.name()), self.missing.is_some())
    }
}

/// Top-level ruleset class for tiered lobby selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresetClass {
    Masters,
    Senior,
    Gate,
}

impl PresetClass {
    /// Base preset ID whose localized name labels this top-level class.
    pub fn base_id(self) -> &'static str {
        match self {
            PresetClass::Masters => "masters",
            PresetClass::Senior => "senior",
            PresetClass::Gate => "gate",
        }
    }

    /// Resolve `(game, class, sub, mixed)` to the canonical built-in preset ID.
    /// In EXE5, `sub` is `5ds` for Masters/Senior and `party` for Gate; in EXE4/EXE6 `sub` is ignored.
    pub fn preset_id(self, game: &str, sub: bool, mixed: bool) -> &'static str {
        match (game, self, sub, mixed) {
            ("exe5", PresetClass::Masters, false, false) => "masters",
            ("exe5", PresetClass::Masters, false, true) => "masters_mixed",
            ("exe5", PresetClass::Masters, true, false) => "masters_5ds",
            ("exe5", PresetClass::Masters, true, true) => "masters_5ds_mixed",
            ("exe5", PresetClass::Senior, false, false) => "senior",
            ("exe5", PresetClass::Senior, false, true) => "senior_mixed",
            ("exe5", PresetClass::Senior, true, false) => "senior_5ds",
            ("exe5", PresetClass::Senior, true, true) => "senior_5ds_mixed",
            ("exe5", PresetClass::Gate, false, false) => "gate",
            ("exe5", PresetClass::Gate, false, true) => "gate_mixed",
            ("exe5", PresetClass::Gate, true, false) => "gate_party",
            ("exe5", PresetClass::Gate, true, true) => "gate_party_mixed",
            (_, PresetClass::Masters, _, false) => "masters",
            (_, PresetClass::Masters, _, true) => "masters_mixed",
            (_, PresetClass::Senior, _, false) => "senior",
            (_, PresetClass::Senior, _, true) => "senior_mixed",
            (_, PresetClass::Gate, _, false) => "gate",
            (_, PresetClass::Gate, _, true) => "gate_mixed",
        }
    }
}

/// Decomposed tiered coordinates of a built-in preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresetTier {
    pub class: PresetClass,
    pub sub: bool,
    pub mixed: bool,
}

/// Parse a built-in preset ID into its `(class, sub, mixed)` coordinates.
pub fn preset_tier(id: &str) -> Option<PresetTier> {
    match id {
        "masters" => Some(PresetTier { class: PresetClass::Masters, sub: false, mixed: false }),
        "masters_mixed" => Some(PresetTier { class: PresetClass::Masters, sub: false, mixed: true }),
        "masters_5ds" => Some(PresetTier { class: PresetClass::Masters, sub: true, mixed: false }),
        "masters_5ds_mixed" => Some(PresetTier { class: PresetClass::Masters, sub: true, mixed: true }),
        "senior" => Some(PresetTier { class: PresetClass::Senior, sub: false, mixed: false }),
        "senior_mixed" => Some(PresetTier { class: PresetClass::Senior, sub: false, mixed: true }),
        "senior_5ds" => Some(PresetTier { class: PresetClass::Senior, sub: true, mixed: false }),
        "senior_5ds_mixed" => Some(PresetTier { class: PresetClass::Senior, sub: true, mixed: true }),
        "gate" => Some(PresetTier { class: PresetClass::Gate, sub: false, mixed: false }),
        "gate_mixed" => Some(PresetTier { class: PresetClass::Gate, sub: false, mixed: true }),
        "gate_party" => Some(PresetTier { class: PresetClass::Gate, sub: true, mixed: false }),
        "gate_party_mixed" => Some(PresetTier { class: PresetClass::Gate, sub: true, mixed: true }),
        _ => None,
    }
}

/// Legacy preset ID compatibility: old `unrestricted` maps to the equivalent mixed Masters preset.
fn legacy_preset<'a>(picks: &'a [Pickable], name: &Name) -> Option<&'a Pickable> {
    let Name::Preset(id) = name else { return None };
    if id != "unrestricted" {
        return None;
    }
    picks
        .iter()
        .find(|p| p.ruleset.name == Name::Preset("masters_5ds_mixed".into()))
        .or_else(|| picks.iter().find(|p| p.ruleset.name == Name::Preset("masters_mixed".into())))
}

/// The ruleset a build that names `name` is made for, of `picks` (what the player can pick of its game, in order:
/// [`pickable`]): the one named; the game's first preset where it names none (a file from before builds named
/// theirs), or one no longer there (missing). None where the game has no rulesets.
pub fn made_for(picks: &[Pickable], name: Option<&Name>) -> Option<MadeFor> {
    let first = picks.first()?;
    let Some(name) = name else { return Some(MadeFor { pick: first.clone(), missing: None }) };
    Some(match picks.iter().find(|p| p.ruleset.name == *name).or_else(|| legacy_preset(picks, name)) {
        Some(pick) => MadeFor { pick: pick.clone(), missing: None },
        None => MadeFor { pick: first.clone(), missing: Some(name.clone()) },
    })
}

/// What the settings remember the ruleset a player last chose under, by game.
fn remembered_key(game: &str) -> String {
    format!("ruleset.{game}")
}

/// A ruleset's name as a build's file and the settings write it: `preset:<id>` or `own:<name>` (the other
/// player's, which no player picks for a build, `theirs`).
pub fn name_text(name: &Name) -> String {
    match name {
        Name::Preset(id) => format!("preset:{id}"),
        Name::Own(own) => format!("own:{own}"),
        Name::Theirs => "theirs".to_string(),
    }
}

/// A ruleset's name read from its text ([`name_text`]). None: it is none of those.
pub fn name_of_text(text: &str) -> Option<Name> {
    if let Some(id) = text.strip_prefix("preset:") {
        return Some(Name::Preset(id.to_string()));
    }
    if let Some(own) = text.strip_prefix("own:") {
        return Some(Name::Own(own.to_string()));
    }
    (text == "theirs").then_some(Name::Theirs)
}

/// The ruleset the player last chose in `kit`'s game (for a build in its editor, or as a lobby's host): the one
/// remembered for it, where it can still be picked; else the game's first preset. What a lobby starts on where no
/// build says. None where the game says nothing of its rulesets.
pub fn chosen_last(shell: &Shell, kit: &Kit) -> Option<Pickable> {
    let picks = pickable(kit);
    let remembered = shell.settings.remembered.get(&remembered_key(&kit.game));
    remembered
        .and_then(|text| {
            picks
                .iter()
                .find(|p| name_text(&p.ruleset.name) == *text)
                .or_else(|| name_of_text(text).and_then(|n| legacy_preset(&picks, &n)))
        })
        .or_else(|| picks.first())
        .cloned()
}

/// Remember `name` as the ruleset the player last chose in `game`.
pub fn remember(shell: &mut Shell, game: &str, name: &Name) {
    let (key, text) = (remembered_key(game), name_text(name));
    if shell.settings.remembered.get(&key) != Some(&text) {
        shell.settings.remembered.insert(key, text);
        shell.save_settings();
    }
}

/// The player's own ruleset `from` of `kit`'s game renamed `to` (on its page): each build made for it is made for
/// it by its new name (its file written again; one that doesn't read is left as it is, why in the log), and so is
/// the choice remembered. (One deleted is left named in its builds, which say it is missing.)
pub fn rename(shell: &mut Shell, kit: &Kit, from: &Name, to: &Name) {
    for (path, why) in crate::builds::store::rename_ruleset(kit.content(), &kit.game, from, to) {
        crate::errors::log(&format!("renaming the ruleset in the build {}", path.display()), &why);
    }
    if shell.settings.remembered.get(&remembered_key(&kit.game)) == Some(&name_text(from)) {
        remember(shell, &kit.game, to);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A ruleset's name is written by kind and name, so a player's own named like a preset's id isn't
    /// taken for it, and reads back.
    #[test]
    fn a_name_is_written_by_its_kind() {
        assert_eq!(name_text(&Name::Preset("standard".into())), "preset:standard");
        assert_eq!(name_text(&Name::Own("standard".into())), "own:standard");
        assert_eq!(remembered_key("exe6"), "ruleset.exe6");
        for name in [Name::Preset("masters".into()), Name::Own("own: \"cup\"".into()), Name::Theirs] {
            assert_eq!(name_of_text(&name_text(&name)), Some(name));
        }
        assert_eq!(name_of_text("masters"), None);
    }

    /// A build is made for the ruleset it names where there is one; else (it names none, or one no longer there:
    /// missing) the game's first preset is the one it is checked under.
    #[test]
    fn a_build_is_made_for_its_ruleset() {
        let pick = |name: Name| Pickable { ruleset: Ruleset::open(name), path: None };
        let picks = [pick(Name::Preset("masters".into())), pick(Name::Preset("senior".into())), pick(Name::Own("Cup".into()))];
        let made = |name: Option<Name>| made_for(&picks, name.as_ref()).unwrap();
        assert_eq!(made(None), MadeFor { pick: picks[0].clone(), missing: None });
        assert_eq!(made(Some(Name::Own("Cup".into()))), MadeFor { pick: picks[2].clone(), missing: None });
        let gone = made(Some(Name::Own("Gone".into())));
        assert_eq!(gone, MadeFor { pick: picks[0].clone(), missing: Some(Name::Own("Gone".into())) });
        assert_eq!(gone.name(), &Name::Own("Gone".into()));
        assert_eq!(made_for(&[], None), None);
    }

    #[test]
    fn preset_tiers_roundtrip_and_migrate_unrestricted() {
        for (game, ids) in [
            ("exe4", &["masters", "masters_mixed", "senior", "senior_mixed"][..]),
            (
                "exe5",
                &[
                    "masters", "masters_mixed", "masters_5ds", "masters_5ds_mixed",
                    "senior", "senior_mixed", "senior_5ds", "senior_5ds_mixed",
                    "gate", "gate_mixed", "gate_party", "gate_party_mixed",
                ][..],
            ),
            ("exe6", &["masters", "masters_mixed", "senior", "senior_mixed", "gate", "gate_mixed"][..]),
        ] {
            for &id in ids {
                let tier = preset_tier(id).unwrap_or_else(|| panic!("{game}: missing tier for {id}"));
                assert_eq!(tier.class.preset_id(game, tier.sub, tier.mixed), id, "{game}: {id}");
            }
        }
        let pick = |id: &str| Pickable { ruleset: Ruleset::open(Name::Preset(id.into())), path: None };
        let exe6_picks = [pick("masters"), pick("masters_mixed"), pick("senior")];
        let migrated6 = made_for(&exe6_picks, Some(&Name::Preset("unrestricted".into()))).unwrap();
        assert_eq!(migrated6.missing, None);
        assert_eq!(migrated6.pick.ruleset.name, Name::Preset("masters_mixed".into()));

        let exe5_picks = [pick("masters"), pick("masters_mixed"), pick("masters_5ds_mixed")];
        let migrated5 = made_for(&exe5_picks, Some(&Name::Preset("unrestricted".into()))).unwrap();
        assert_eq!(migrated5.missing, None);
        assert_eq!(migrated5.pick.ruleset.name, Name::Preset("masters_5ds_mixed".into()));
    }
}
