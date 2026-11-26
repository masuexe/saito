//! The Navi tab of a build's editor (docs/design/app-ui.md §6, "Navi"): the game's navis as cards, the build's
//! navi beside them (the face its round starts with, its HP then, its element), under it the build's other choices
//! (the game's builds data's: Light or Dark, of the game's own navi alone), each a segmented control of its presets
//! (docs/design/app-ui.md §2.63), then the selector of the forms the build has (docs/design/versions.md §2.1; the
//! user: "you probably need to have some kind of soul and cross and cybeast selector in the editor now right?":
//! `Builds::form_choices`).
//!
//! The selector: each fact of forms the build chooses, by its name in the game's rulesets' words (EXE6's "Beast
//! form", the user: "instead of "Cybeasts" in bn6 it should really be just "Beast form""), its forms in the battle's
//! order, both versions' (EXE6's Cybeast at its head, Gregar's or Falzar's, one always chosen; then its Crosses, as
//! many as the Cross window offers; EXE4's and EXE5's souls, as many as the soul button's list), each a row of the
//! editor's lists (the user: "the rows in the editor are too tall, they should match the height of the regular
//! editor"): checked where it is the build's, its face, its name, and the mark of the version it is one's own (the
//! user: "you should indicate in the editor version exclusives"). Enter, a click or a tap on one chooses it, or takes
//! it out of a list (`Builds::choose_form`). A list has ALL and NONE (docs/design/app-ui.md §2.69; the user: "the
//! souls/crosses should be able to select all/none") on each version's head, or on its heading where it shows no
//! versions: that group's forms put in while the list has room, the rest refused, or taken out (`Builds::choose_forms`,
//! one change). What is wrong with the build is marked where it is (docs/design/app-ui.md
//! §2.66; the user: "if there's a problem with e.g. crosses or souls there's nowhere in the ui that shows it"): a
//! fact's heading in the error red where it has a problem, and a form of the build's a problem is about, its name; a
//! form the ruleset doesn't allow says so at its row's end, as a library's row does, and isn't put in (nor a navi it
//! doesn't allow chosen; ALL leaves it out: docs/design/app-ui.md §2.68). The tab has no strip (the user:
//! "i don't like having an info thing on the navi view, it's redundant"): what the build's problems have against the
//! form under the cursor or the pointer, the ruleset's among them (its own: one the ruleset holds back, a version's
//! in a build that mixes versions; then its fact's about no form of it), else against the navi (and the build where
//! no tab shows what it is about), is said on a line under the navi's facts, kept for it, so nothing moves when
//! one appears (the user: "error messages for beast form/crosses ends up shifting layout"). The lists are the game's
//! own navi's alone; the Cybeast is every navi's (MstrCros plays its version's Crosses).
//!
//! Choosing a navi is `Draft::choose_navi` (saito-match's `Builds::set_navi`, with what is the game's own
//! navi's alone kept aside in the build's file); a choice's preset is `Builds::take_preset`.
//!
//! On a phone the grid is a sheet from the bottom (`NaviOverlay`), which the navi's card on the tab opens
//! (`navi-pick`): a navi chosen in it, a tap on the shade or Back shuts it, the focus back on the card. The selector's
//! rows are one to a row there too, their names whole, scrolling in the room the panel has.

use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use saito_battle::Content;
use saito_battle::content::{Element, PlayerFact};
use saito_content_api::{FormHandle, NaviHandle, Registry};
use saito_match::Wrong;
use saito_match::check::Problem;
use saito_match::ruleset::Item;
use crate::builds::overview::{stat_line, stat_name};
use saito_ui::{EditorTab, NaviCard, NaviChoice, NaviForm, NaviFormGroup, NaviView, StatLine};
use slint::{ComponentHandle, ModelRc, VecModel};

#[derive(Default)]
pub struct Navi {
    /// The game's navis, as the grid has them.
    navis: Vec<NaviHandle>,
    /// The selector's forms, each with the fact it is of, in the focus order.
    forms: Vec<(String, FormHandle)>,
    /// What each ALL and NONE of the selector is of (their focus keys' index): a fact of forms, and its version's
    /// group's forms (or the whole list's, where it shows no versions).
    sets: Vec<(String, Vec<FormHandle>)>,
    /// The form the pointer is on (its place in the focus order), whose problems the fault's line says in place of the
    /// focused one's (`editor::hover`).
    hovered: Option<usize>,
}

/// The chip family whose icon the game shows for `element`: the family of the element's name, a navi of no
/// element's the family of the chips of none.
pub(super) fn family(content: &Content, element: Element) -> saito_battle::content::ChipFamily {
    let families = &content.rules().chip_families;
    let named = match element {
        Element::Null => None,
        Element::Fire => families.by_name("fire"),
        Element::Aqua => families.by_name("aqua"),
        Element::Elec => families.by_name("elec"),
        Element::Wood => families.by_name("wood"),
    };
    named.unwrap_or(families.non_elemental)
}

/// Whether problem `problem` of the build's fact `fact` of forms (`chosen`, in order) is about form `form`: a form the
/// ruleset doesn't allow, one of a version's in a build that mixes versions, or the entry it names.
fn about(problem: &Problem, fact: &str, chosen: &[FormHandle], form: FormHandle) -> bool {
    match &problem.what {
        Wrong::NotAllowed { registry: Registry::Form, item, .. } => item.def == form.0,
        Wrong::Mixed { items, .. } => items.contains(&(Registry::Form, Item::of(form.0))),
        _ => problem.field.as_deref() == Some(fact) && problem.entry.and_then(|e| chosen.get(e)) == Some(&form),
    }
}

/// The phone's sheet of the navis: its controls' prefix, and its cards' name.
const SHEET: &str = "sheet.";
const SHEET_NAVI: &str = "sheet.navi";

impl Navi {
    /// The navi's fact, by its setup field's name.
    fn navi_fact(content: &Content) -> Vec<String> {
        content.defs.fact_name(PlayerFact::Navi).map(str::to_string).into_iter().collect()
    }

    /// What the line under the navi's facts says (docs/design/app-ui.md §2.66, §2.70): what the build's problems have
    /// against the form under the pointer or the cursor, the ruleset's among them, where they have anything; else
    /// against the navi, and the build where no tab shows what it is about. Empty: the note.
    fn show_fault(&self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let focused = shell.nav.focused().filter(|k| k.id == "form").map(|k| k.index.max(0) as usize);
        let of_form: Vec<&Problem> = match self.hovered.or(focused).and_then(|i| self.forms.get(i)) {
            Some((fact, form)) => {
                let picked = draft.kit.builds.chosen_forms(content, draft.side(), fact);
                // (Its own problems, then its fact's that are about no form of it: a fact a build made by hand states
                // wrong, which every form of it says.)
                let of_fact: Vec<&Problem> = draft.checked.problems.iter().filter(|p| draft.facts_of(p).contains(fact)).collect();
                let own = of_fact.iter().copied().filter(|p| about(p, fact, &picked, *form));
                let unplaced = of_fact.iter().copied().filter(|p| !picked.iter().any(|&f| about(p, fact, &picked, f)));
                own.chain(unplaced).collect()
            }
            None => Vec::new(),
        };
        let problems = if of_form.is_empty() { draft.pane_problems(&Navi::navi_fact(content), true) } else { of_form };
        shell.ui().global::<NaviView>().set_fault(draft.line(shell, problems).into());
    }

    /// The phone's sheet of the navis opens, the focus on the build's navi's card.
    fn open_sheet(&mut self, shell: &mut Shell, draft: &Draft) {
        let at = self.navis.iter().position(|&n| n == draft.side().navi(draft.content())).unwrap_or(0);
        shell.ui().global::<NaviView>().set_sheet(true);
        shell.nav.open(SHEET, crate::nav::Key::new(SHEET_NAVI, at as i32));
    }

    /// The sheet shuts (if it is open), the focus back where it was under it: on the navi's card.
    fn close_sheet(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<NaviView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close(SHEET);
        }
    }
}

impl Tab for Navi {
    fn id(&self) -> EditorTab {
        EditorTab::Navi
    }

    /// The navi, its base HP (where customizable), optional boolean `sol_cross`, and the forms the build chooses.
    fn fields(&self, content: &Content) -> Vec<String> {
        let navi = content.defs.fact_name(PlayerFact::Navi).map(str::to_string);
        let hp = content.defs.fact_name(PlayerFact::BaseHp).map(str::to_string);
        let sol_cross = saito_match::facts::field(content, "sol_cross").map(|_| "sol_cross".to_string());
        navi.into_iter().chain(hp).chain(sol_cross).chain(saito_match::facts::form_facts(content).iter().map(|f| f.name.to_string())).collect()
    }

    fn main(&self) -> &'static str {
        "navi"
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        // (A phone turned sideways has the grid beside the panel: its sheet shuts.)
        if !shell.scale.phone {
            self.close_sheet(shell);
        }
        let content = draft.content().clone();
        let kit = draft.kit.clone();
        // (The game's own navi first: the content states no order of the others, which are by their keys.)
        self.navis = saito_match::navis(&content, &kit.game);
        self.navis.sort_by_key(|&n| !content.navi(n).changes_form());
        let chosen = draft.side().navi(&content);
        let ui = shell.ui();
        let view = ui.global::<NaviView>();
        let face = draft.checked.face().map(|f| kit.pictures.shown(&f)).unwrap_or_else(|| kit.pictures.navi(chosen));
        // (Its stats as the overview says them: HP, then those of its game's rules' own it has.)
        let side = draft.side().clone();
        let overview = draft.checked.overview(&content, &side);
        let offered = kit.builds.choices_of(&content, draft.side());
        draft.with_words(shell, |words| {
            let names = words.names;
            // (A navi the build's ruleset doesn't allow is marked: the navi fact's category.)
            let navi_fact = content.defs.fact_name(PlayerFact::Navi).unwrap_or_default();
            let cards: Vec<NaviCard> =
                self.navis.iter().map(|&n| NaviCard { name: names.navi(n).into(), face: kit.pictures.navi(n), chosen: n == chosen, banned: !draft.allows(navi_fact, n.0) }).collect();
            view.set_navis(ModelRc::new(VecModel::from(cards)));
            view.set_name(names.navi(chosen).into());
            let stats: Vec<StatLine> = match &overview {
                Some(o) => std::iter::once(stat_line(String::new(), o.hp))
                    .chain(o.members.iter().filter(|(m, _)| *m == 0).flat_map(|(_, s)| s).map(|s| stat_line(stat_name(words, s), s.stat)))
                    .collect(),
                None => vec![StatLine::default()],
            };
            view.set_stats(ModelRc::new(VecModel::from(stats)));
            // Each of the game's choices the build's navi has (a navi with none of one has nothing in its place): its
            // presets by their names in the game's strings, in capitals, a segment each, and the one the build is on.
            let choices: Vec<NaviChoice> = kit
                .builds
                .choices
                .iter()
                .enumerate()
                .filter(|(_, c)| offered.iter().any(|o| o.id == c.id))
                .filter_map(|(i, c)| {
                    let now = kit.builds.preset_of(&content, draft.side(), &c.id)?;
                    let presets: Vec<slint::SharedString> = c.presets.iter().map(|p| crate::builds::preset_name(names, c, p).into()).collect();
                    let chosen = c.presets.iter().position(|p| p.id == now.id).unwrap_or(0) as i32;
                    Some(NaviChoice { presets: ModelRc::new(VecModel::from(presets)), chosen, index: i as i32 })
                })
                .collect();
            view.set_choices(ModelRc::new(VecModel::from(choices)));
            if let Some((min, max)) = kit.builds.base_hp_range(&content, draft.side()) {
                let hp = kit.builds.base_hp(&content, draft.side()).unwrap_or(max);
                view.set_has_base_hp(true);
                view.set_base_hp(hp.to_string().into());
                view.set_base_hp_min(hp <= min);
                view.set_base_hp_max(hp >= max);
            } else {
                view.set_has_base_hp(false);
                view.set_base_hp(Default::default());
                view.set_base_hp_min(false);
                view.set_base_hp_max(false);
            }
            let has_sol_cross = has_sol_cross_toggle(&content, &kit, chosen);
            if has_sol_cross {
                let on = matches!(draft.side().facts.get(&content, "sol_cross"), Some(saito_match::facts::Stated::Flag(true)));
                let banned = sol_cross_banned(&content, &kit, draft);
                let flagged = draft.checked.problems.iter().any(|p| p.field.as_deref() == Some("sol_cross"));
                view.set_has_sol_cross(true);
                view.set_sol_cross(on);
                view.set_sol_cross_banned(banned);
                view.set_sol_cross_flagged(flagged);
            } else {
                view.set_has_sol_cross(false);
                view.set_sol_cross(false);
                view.set_sol_cross_banned(false);
                view.set_sol_cross_flagged(false);
            }
            let own = saito_match::first_navi(&content, &kit.game);
            view.set_own(own.map(|n| names.navi(n)).unwrap_or_default().into());
            view.set_other_navi(own.is_some_and(|n| n != chosen));
        });
        // The selector: each fact of forms the build chooses, named as the game's rulesets name it ("CROSSES"), with how
        // many it has of its room (none said of the one always chosen), in the error red where it has a problem; its
        // forms checked where they are the build's, under their versions' heads, a form's name in the error red where a
        // problem is about it, and "Not allowed" at the end of one the ruleset holds back.
        let choices = kit.builds.form_choices(&content, draft.side());
        self.forms = choices.iter().flat_map(|c| c.forms.iter().map(|&f| (c.fact.clone(), f))).collect();
        // (One column of rows, a phone's too, upright or sideways: two to a row there cut the names.)
        let (phone, one_column) = (shell.scale.phone, shell.scale.handheld());
        let mut sets: Vec<(String, Vec<FormHandle>)> = Vec::new();
        let groups: Vec<NaviFormGroup> = draft.with_words(shell, |words| {
            let mut at = 0;
            choices
                .iter()
                .map(|c| {
                    let chosen = kit.builds.chosen_forms(&content, draft.side(), &c.fact);
                    let problems: Vec<&Problem> = draft.checked.problems.iter().filter(|p| draft.facts_of(p).contains(&c.fact)).collect();
                    // (Grouped by version, in the groups' order, the battle's within each, as `form_choices` sorts
                    // them, those of none last: each group headed by its version's name, its rows from a row of
                    // their own. Where each of two versions has one form, as EXE6's Beast form's GBeast and FBeast,
                    // a desktop has the two groups side by side, a column each, one row under their heads; a phone
                    // one over the other, as half its width cuts their names.)
                    let versions: Vec<Option<String>> = c.forms.iter().map(|f| content.defs.forms.get(f.index()).and_then(|d| d.record.version.clone())).collect();
                    let side_by_side = !one_column && versions.len() == 2 && versions[0].is_some() && versions[1].is_some() && versions[0] != versions[1];
                    // (A list's ALL and NONE: on each version's head where its forms are grouped by version, a group of
                    // none's too, else on the fact's heading; whether each would do anything, ALL where the list has
                    // room for one of them it lacks that the ruleset allows, NONE where it has one. A fact of one form has
                    // none.)
                    let mut set_of = |forms: Vec<FormHandle>| -> (i32, bool, bool) {
                        if c.one {
                            return (-1, false, false);
                        }
                        let all = chosen.len() < c.room && forms.iter().any(|f| !chosen.contains(f) && draft.allows(&c.fact, f.0));
                        let none = forms.iter().any(|f| chosen.contains(f));
                        sets.push((c.fact.clone(), forms));
                        (sets.len() as i32 - 1, all, none)
                    };
                    let by_version = versions.iter().any(Option::is_some);
                    let whole = if by_version { (-1, false, false) } else { set_of(c.forms.clone()) };
                    let mut forms: Vec<NaviForm> = Vec::new();
                    let (mut rows, mut heads, mut first_row, mut place) = (0, 0, 0, 0);
                    let mut last: Option<Option<String>> = None;
                    for (n, (&f, version)) in c.forms.iter().zip(versions.clone()).enumerate() {
                        let tag = kit.version_tag_of(words.names, Registry::Form, &Item::of(f.0));
                        let starts = last.as_ref() != Some(&version);
                        let (set, all, none) = if starts && by_version {
                            set_of(c.forms.iter().zip(&versions).skip(n).take_while(|(_, v)| **v == version).map(|(&f, _)| f).collect())
                        } else {
                            (-1, false, false)
                        };
                        if starts && !side_by_side {
                            // (A group of none after the versions' has a head's room, and no name.)
                            if version.is_some() || last.is_some() {
                                heads += 1;
                            }
                            first_row = rows;
                            place = 0;
                        }
                        last = Some(version);
                        let (column, row) = if side_by_side { (n as i32, 0) } else { (0, first_row + place) };
                        if side_by_side {
                            heads = 1;
                        }
                        rows = rows.max(row + 1);
                        forms.push(NaviForm {
                            face: kit.pictures.face(f),
                            name: words.names.form(f).into(),
                            head: if starts { tag } else { Default::default() },
                            column,
                            row,
                            heads,
                            chosen: chosen.contains(&f),
                            flagged: problems.iter().any(|p| about(p, &c.fact, &chosen, f)),
                            banned: !draft.allows(&c.fact, f.0),
                            set,
                            span: 1,
                            all,
                            none,
                        });
                        place += 1;
                    }
                    let name = words.names.text(saito_match::ruleset::TEXT_TABLE, &format!("category.{}", c.fact)).unwrap_or_else(|| c.fact.clone());
                    let count = if c.one { String::new() } else { format!("{}/{}", chosen.len(), c.room) };
                    let columns = if side_by_side { 2 } else { 1 };
                    let group = NaviFormGroup {
                        name: name.into(),
                        count: count.into(),
                        flagged: !problems.is_empty(),
                        columns,
                        rows,
                        heads,
                        forms: ModelRc::new(VecModel::from(forms)),
                        at,
                        set: whole.0,
                        all: whole.1,
                        none: whole.2,
                    };
                    at += c.forms.len() as i32;
                    group
                })
                .collect()
        });
        self.sets = sets;
        view.set_form_groups(ModelRc::new(VecModel::from(groups)));
        let (ruleset, missing) = draft.ruleset_shown(shell);
        view.set_ruleset(ruleset.into());
        view.set_ruleset_missing(missing);
        view.set_face(face);
        view.set_element(kit.pictures.element(family(&content, content.navi(chosen).element), shell.settings.hd_chip_art));
        // (The focus starts on the build's navi's card: in the grid, or on a phone the one that opens its sheet.)
        let known = ["navi", "navi-pick", "form", "forms-all", "forms-none", "choice", "base-hp", "base-hp-step", "sol-cross", "tab", "ruleset", "ruleset-edit", "practice"];
        if shell.nav.focused().is_none_or(|k| (!known.contains(&k.id.as_str()) && !k.id.starts_with(SHEET)) || (!phone && k.id == "navi-pick")) {
            if phone {
                shell.nav.focus("navi-pick", 0);
            } else if let Some(at) = self.navis.iter().position(|&n| n == chosen) {
                shell.nav.focus("navi", at as i32);
            }
        }
        self.show_fault(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        self.hovered = None;
        self.close_sheet(shell);
    }

    /// The pointer came onto a form's row, or left it: the fault's line says the form's problems while the pointer is
    /// on it.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        if id != "form" {
            return;
        }
        if editor::hover(&mut self.hovered, index.max(0) as usize, on) {
            self.show_fault(shell, draft);
        }
    }

    /// (The fault's line follows the focus, the last thing moved, until the pointer goes onto another form.)
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, _id: &str, _index: i32) {
        self.hovered = None;
        self.show_fault(shell, draft);
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        match id {
            // (A navi the build's ruleset doesn't allow is refused: docs/design/app-ui.md §2.68. A phone's sheet stays
            // open for another; one chosen shuts it.)
            "navi" | SHEET_NAVI => {
                let Some(&navi) = self.navis.get(index.max(0) as usize) else { return };
                let navi_fact = Navi::navi_fact(draft.content());
                if navi != draft.side().navi(draft.content()) && navi_fact.first().is_some_and(|f| !draft.allows(f, navi.0)) {
                    return shell.sound.play(crate::sound::Cue::Refused);
                }
                self.close_sheet(shell);
                if draft.choose_navi(navi) {
                    self.show(shell, draft);
                }
            }
            // (A phone: the navi's card opens the grid's sheet; a tap on the shade shuts it.)
            "navi-pick" => self.open_sheet(shell, draft),
            "sheet.close" => self.close_sheet(shell),
            // (A form of the selector: chosen, or in a list taken out. One the ruleset doesn't allow is refused where it
            // would go in; one the build has can be taken out.)
            "form" => {
                let Some((fact, form)) = self.forms.get(index.max(0) as usize).cloned() else { return };
                let has = draft.kit.builds.chosen_forms(draft.content(), draft.side(), &fact).contains(&form);
                if !has && !draft.allows(&fact, form.0) {
                    return shell.sound.play(crate::sound::Cue::Refused);
                }
                if draft.choose_form(&fact, form) {
                    shell.sound.play(crate::sound::Cue::Pick);
                    self.show(shell, draft);
                } else {
                    shell.sound.play(crate::sound::Cue::Refused);
                }
            }
            // (ALL or NONE of a version's group of forms, or of a whole list: as many put in as the list has room for,
            // in order, and the rest refused, those the ruleset doesn't allow among them; or all taken out. One
            // change.)
            "forms-all" | "forms-none" => {
                let Some((fact, mut forms)) = self.sets.get(index.max(0) as usize).cloned() else { return };
                let all = id == "forms-all";
                let mut banned = 0;
                if all {
                    let has = draft.kit.builds.chosen_forms(draft.content(), draft.side(), &fact);
                    banned = forms.iter().filter(|f| !has.contains(f) && !draft.allows(&fact, f.0)).count();
                    forms.retain(|f| draft.allows(&fact, f.0));
                }
                let (changed, left) = draft.choose_forms(&fact, &forms, all);
                let left = left + banned;
                let cue = if changed && left == 0 { crate::sound::Cue::Pick } else { crate::sound::Cue::Refused };
                shell.sound.play(cue);
                if changed {
                    self.show(shell, draft);
                }
            }
            // (Enter on a choice takes its next preset, after the last its first: of two, the other.)
            "choice" => {
                if let Some(at) = preset_at(draft, index) {
                    take(self, shell, draft, index, at + 1);
                }
            }
            // (Step buttons for base HP: -20, -1, +1, +20.)
            "base-hp-step" => {
                let delta = match index {
                    0 => -20,
                    1 => -1,
                    2 => 1,
                    3 => 20,
                    _ => return,
                };
                if step_base_hp(draft, delta) {
                    shell.sound.play(crate::sound::Cue::Pick);
                    self.show(shell, draft);
                } else {
                    shell.sound.play(crate::sound::Cue::Refused);
                }
            }
            "sol-cross" => {
                let cur = matches!(draft.side().facts.get(draft.content(), "sol_cross"), Some(saito_match::facts::Stated::Flag(true)));
                toggle_sol_cross(self, shell, draft, !cur);
            }
            _ => {}
        }
    }

    fn edit(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, text: &str, done: bool) {
        if id != "base-hp" {
            return;
        }
        let kit = draft.kit.clone();
        let Some((min, max)) = kit.builds.base_hp_range(draft.content(), draft.side()) else { return };
        let digits: String = text.chars().filter(|c| c.is_ascii_digit()).take(4).collect();
        if !done {
            shell.ui().global::<NaviView>().set_base_hp(digits.clone().into());
            if let Ok(n) = digits.parse::<u16>()
                && n >= min
                && n <= max
                && set_base_hp(draft, n)
            {
                let view_text = digits;
                self.show(shell, draft);
                shell.ui().global::<NaviView>().set_base_hp(view_text.into());
            }
            return;
        }
        let target = digits
            .parse::<u32>()
            .ok()
            .map(|n| n.clamp(min as u32, max as u32) as u16)
            .unwrap_or_else(|| kit.builds.base_hp(draft.content(), draft.side()).unwrap_or(max));
        let _ = set_base_hp(draft, target);
        self.show(shell, draft);
    }

    /// Left or Right on a choice: its preset that way, none past its ends (a segmented control's).
    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, by: i32) {
        if id == "sol-cross" {
            toggle_sol_cross(self, shell, draft, by > 0);
            return;
        }
        if id != "choice" {
            return;
        }
        let Some(at) = preset_at(draft, index) else { return };
        let count = draft.kit.builds.choices.get(index.max(0) as usize).map_or(0, |c| c.presets.len() as i32);
        let to = (at + by).clamp(0, count - 1);
        if to != at {
            take(self, shell, draft, index, to);
        }
    }

    /// A click or a tap on a choice's segment: its preset.
    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        if id == "sol-cross" {
            toggle_sol_cross(self, shell, draft, choice == 1);
            return;
        }
        if id == "choice" {
            take(self, shell, draft, index, choice);
        }
    }

    /// Back on a phone's sheet of the navis shuts it.
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        if !shell.ui().global::<NaviView>().get_sheet() {
            return false;
        }
        self.close_sheet(shell);
        true
    }
}

/// Which of choice `index`'s presets the build is on (None: the build's navi has none of the choice).
fn preset_at(draft: &Draft, index: i32) -> Option<i32> {
    let kit = &draft.kit;
    let choice = kit.builds.choices.get(index.max(0) as usize)?;
    let now = kit.builds.preset_of(draft.content(), draft.side(), &choice.id)?;
    Some(choice.presets.iter().position(|p| p.id == now.id).unwrap_or(0) as i32)
}

/// The build takes preset `preset` of choice `index` (past the last, the first), and the tab shows it.
fn take(tab: &mut Navi, shell: &mut Shell, draft: &mut Draft, index: i32, preset: i32) {
    let kit = draft.kit.clone();
    let Some(choice) = kit.builds.choices.get(index.max(0) as usize) else { return };
    let Some(next) = choice.presets.get(preset.rem_euclid(choice.presets.len().max(1) as i32) as usize) else { return };
    let changed = draft.edit(|content, side| {
        let before = side.clone();
        match kit.builds.take_preset(content, side, &choice.id, &next.id) {
            Ok(()) => *side != before,
            Err(why) => {
                crate::errors::log("taking a build's preset", &why);
                false
            }
        }
    });
    if changed {
        tab.show(shell, draft);
    }
}

/// Set the build's base HP to `hp` (within its legal range). Returns whether it changed.
fn set_base_hp(draft: &mut Draft, hp: u16) -> bool {
    let kit = draft.kit.clone();
    draft.edit(|content, side| {
        let before = side.clone();
        match kit.builds.set_base_hp(content, side, hp) {
            Ok(()) => *side != before,
            Err(why) => {
                crate::errors::log("setting a build's base HP", &why);
                false
            }
        }
    })
}

/// Adjust the build's base HP by `delta`, clamped to its legal range. Returns whether it changed.
fn step_base_hp(draft: &mut Draft, delta: i32) -> bool {
    let kit = draft.kit.clone();
    let Some((min, max)) = kit.builds.base_hp_range(draft.content(), draft.side()) else { return false };
    let cur = kit.builds.base_hp(draft.content(), draft.side()).unwrap_or(max);
    let next = (cur as i32 + delta).clamp(min as i32, max as i32) as u16;
    if next == cur {
        return false;
    }
    set_base_hp(draft, next)
}

/// Whether the Navi tab should show a dedicated boolean Sol Cross toggle for `chosen`:
/// true when `content` declares a boolean `sol_cross` setup fact, `builds.choices` does not already
/// render a Choice for it, and `chosen` is the game's form-changing navi (MegaMan).
fn has_sol_cross_toggle(content: &Content, kit: &crate::builds::Kit, chosen: NaviHandle) -> bool {
    content.navi(chosen).changes_form()
        && saito_match::facts::field(content, "sol_cross").is_some_and(|f| matches!(f.ty, saito_content_api::FieldType::Bool))
        && !kit.builds.choices.iter().any(|c| c.facts().contains(&"sol_cross"))
}

/// Whether enabling `sol_cross` is disallowed under the build's current ruleset.
fn sol_cross_banned(content: &Content, kit: &crate::builds::Kit, draft: &Draft) -> bool {
    let Some(made) = &draft.ruleset else { return false };
    if !made.pick.ruleset.allow_ds {
        return true;
    }
    let Some(rulesets) = kit.rulesets() else { return false };
    let mut probe = draft.side().clone();
    let _ = probe.set_fact(content, "sol_cross", &[saito_battle::rules::Fact::Value(saito_content_api::Value::Bool(true))]);
    made.pick.ruleset.problems(&rulesets, content, &probe).iter().any(|p| p.field.as_deref() == Some("sol_cross"))
}

fn toggle_sol_cross(tab: &mut Navi, shell: &mut Shell, draft: &mut Draft, next: bool) {
    let content = draft.content().clone();
    let kit = draft.kit.clone();
    let chosen = draft.side().navi(&content);
    if !has_sol_cross_toggle(&content, &kit, chosen) {
        return;
    }
    if next && sol_cross_banned(&content, &kit, draft) {
        shell.sound.play(crate::sound::Cue::Refused);
        return;
    }
    let changed = draft.edit(|content, side| {
        let before = side.clone();
        match side.set_fact(content, "sol_cross", &[saito_battle::rules::Fact::Value(saito_content_api::Value::Bool(next))]) {
            Ok(()) => *side != before,
            Err(why) => {
                crate::errors::log("setting sol_cross", &why);
                false
            }
        }
    });
    if changed {
        shell.sound.play(crate::sound::Cue::Pick);
        tab.show(shell, draft);
    }
}
