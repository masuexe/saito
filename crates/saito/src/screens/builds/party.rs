//! The Party tab of a build's editor, in a game whose setup takes a party (EXE5's party battle, EXE5 DS's:
//! docs/design/exe5ds-map.md §4, §11): the navi's partners, each with its PartyCust programs filling its gauge (§4.10).
//!
//! **Its form is the setup's schema's** (`crate::builds::party::Shape`): the field is a list of records (its room the most
//! partners), each with a field naming a navi and a list of a collection's entries (the programs; its room the most a
//! partner holds). What may be a partner and what fits a gauge are the game's rules': the navis a side can play but its
//! own are offered, the build's problems say what the rules refuse, and the gauge is the rules' overview's (a list budget
//! of the partner's entry: `Overview::budget_of`), as is a program's cost and the round's navi changes
//! (`Overview::switches`).
//!
//! **A two-level list and a library that follows it** (the user: "if you have a navi slot selected in the party editor
//! you pick from a navi and if you have a program slot you pick from a program"), built as the Patch cards tab is: the
//! strip describes what is under the cursor (or the pointer); PARTY framed at the left, its counts and CLEAR in its head,
//! its slots one list (`slots`, [`Slot`]): each partner's head, its programs a level in, an Empty row at their end while
//! its list has room, and an Empty head for each place the party has no partner in; the library at the right, of what
//! the slot the cursor was last on takes (marked as a list's chosen row): the navis for a head, the programs for a
//! program or a partner's Empty row. **Enter (or a click) on a slot** goes into the library (a phone's sheet), **Back**
//! comes back. **Enter on a library's row** puts it in the slot: a navi in the head's place (a partner's taken out with
//! its programs: another navi's gauge isn't its), a program in the program's place, or at the list's end from its Empty
//! row (a program may be in twice: the rules hold them to the gauge alone). A row that can't go in says why in the error
//! red, as the Patch cards library's do: a navi "In party" or "Not allowed" (the build's ruleset's `party` category; one
//! of the other version under one version per build goes in, as a Giga chip of it does in the folder: the mix is the
//! build's problem, the heads of its partners red, their strip saying "Mixed versions"); a program "Needs 3 slots" or "Not allowed" (its
//! `party.programs` category: held back for every partner, or for the marked slot's partner's navi). **Del** takes out what is under the cursor: a program, a partner. **CLEAR** takes every
//! partner out, asked first.
//!
//! **What a program does** is its effects (its definition's `effects`) in the words of the collection's own text table
//! (`<collection>_effects`: `partycust_programs_effects`), as the Patch cards tab words a card's lines; its kind is that
//! table's `kind.` and its definition's `kind`.
//!
//! **Dragged** as the Patch cards tab's cards are (`Drag`, `drag-over` and `drag-in-over`): a program among the slots (a
//! gap opening where it would go: into its partner's list, or the other's; the dragged row over it whole, saying why
//! where it can't go in, and let go refused as Enter is), onto the library (taken out); a library's program among them;
//! a library's navi onto a head (outlined: it goes in that place); a partner's head onto another's (they trade places,
//! their programs with them), onto the library (taken out). Nothing a drag reaches makes a list's rows anew while it is
//! under way (a model set anew would let the pointer go: see `select`).

use crate::builds::overview::{stat_line, stat_name};
use crate::builds::party::{
    FIELD, OWN_PROGRAMS_FIELD, Partner, Shape, has_own_party_cust, own_budget, own_programs, partners, program_tones, tone, write, write_own_programs,
};
use crate::input::Action;
use crate::screens::builds::editor::{self, Draft, Tab};
use crate::shell::Shell;
use crate::sound::Cue;
use saito_battle::Content;
use saito_content_api::{EntryHandle, NaviHandle, Registry};
use saito_match::build::{Budget, Checked, EntryRefusal, NamedStat, Stat};
use saito_match::check::Problem;
use saito_match::ruleset::Item;
use saito_ui::{BuildWords, EditorTab, PartyRow, PartySlot, PartySlotKind, PartyView, StatLine};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

/// PARTY's list of slots, and the library's rows.
const SLOTS: &str = "slots";
const ROWS: &str = "lib.rows";

/// Who owns a PartyCust program slot: the lead team navi (Gate-Party) or partner `i`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Owner {
    Lead,
    Partner(usize),
}

/// A slot of PARTY's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// The lead team navi's head (locked: not replaceable here; selects the program library).
    LeadHead,
    /// The lead team navi's PartyCust program `k`.
    LeadProgram(usize),
    /// The Empty row at the end of the lead team navi's programs, while its list has room.
    LeadEnd,
    /// Partner `i`'s head.
    Head(usize),
    /// Partner `i`'s program `k`.
    Program(usize, usize),
    /// The Empty row at the end of partner `i`'s programs, while its list has room.
    End(usize),
    /// An Empty head: place `i` of the party, which has no partner in it.
    Open(usize),
}

impl Slot {
    /// A partner head (a partner's, or an Empty one): what it takes is a navi.
    fn is_head(self) -> bool {
        matches!(self, Slot::Head(_) | Slot::Open(_))
    }

    /// Whose PartyCust list this slot belongs to (an Empty head's: none).
    fn owner(self) -> Option<Owner> {
        match self {
            Slot::LeadHead | Slot::LeadProgram(_) | Slot::LeadEnd => Some(Owner::Lead),
            Slot::Head(i) | Slot::Program(i, _) | Slot::End(i) => Some(Owner::Partner(i)),
            Slot::Open(_) => None,
        }
    }
}

/// PARTY's slots, in the list's order: optionally the lead team navi's head, its programs and Empty row; then each
/// partner's head, its programs and (while its list has room) its Empty row; then an Empty head for each place left of
/// the party's `room`.
fn slots_of(lead_count: Option<usize>, counts: &[usize], room: usize, program_room: usize) -> Vec<Slot> {
    let mut out = Vec::new();
    if let Some(n) = lead_count {
        out.push(Slot::LeadHead);
        out.extend((0..n).map(Slot::LeadProgram));
        if n < program_room {
            out.push(Slot::LeadEnd);
        }
    }
    for (i, &n) in counts.iter().enumerate() {
        out.push(Slot::Head(i));
        out.extend((0..n).map(|k| Slot::Program(i, k)));
        if n < program_room {
            out.push(Slot::End(i));
        }
    }
    out.extend((counts.len()..room).map(Slot::Open));
    out
}

/// Where a program let go over slot row `row` (the pointer in its upper half: `upper`) goes in: the owner the row is
/// of and the place among its programs (over its head, the first; over a program, before it or after it; over its
/// Empty row, the end). None: over an Empty head, or past the slots.
fn program_place(slots: &[Slot], row: usize, upper: bool, lead_count: usize, counts: &[usize]) -> Option<(Owner, usize)> {
    match *slots.get(row)? {
        Slot::LeadHead => Some((Owner::Lead, 0)),
        Slot::LeadProgram(k) => Some((Owner::Lead, if upper { k } else { k + 1 })),
        Slot::LeadEnd => Some((Owner::Lead, lead_count)),
        Slot::Head(i) => Some((Owner::Partner(i), 0)),
        Slot::Program(i, k) => Some((Owner::Partner(i), if upper { k } else { k + 1 })),
        Slot::End(i) => Some((Owner::Partner(i), counts.get(i).copied().unwrap_or(0))),
        Slot::Open(_) => None,
    }
}

/// The slot row a program going into `owner` at place `at` shows its gap at: the row it goes before (its Empty
/// row at the end; with none, the row after its last program).
fn row_of_place(slots: &[Slot], owner: Owner, at: usize) -> Option<usize> {
    let (target, end) = match owner {
        Owner::Lead => (Slot::LeadProgram(at), Slot::LeadEnd),
        Owner::Partner(i) => (Slot::Program(i, at), Slot::End(i)),
    };
    slots
        .iter()
        .position(|&s| s == target)
        .or_else(|| slots.iter().position(|&s| s == end))
        .or_else(|| slots.iter().rposition(|&s| s.owner() == Some(owner)).map(|r| r + 1))
}

/// The partner head a navi let go over slot row `row` goes in: the row's partner's head, or the Empty head itself
/// (never the locked lead navi head).
fn head_at(slots: &[Slot], row: usize) -> Option<usize> {
    match *slots.get(row)? {
        Slot::Open(_) => Some(row),
        s => match s.owner()? {
            Owner::Partner(i) => slots.iter().position(|&t| t == Slot::Head(i)),
            Owner::Lead => None,
        },
    }
}

/// The PartyCust budgets for the lead team navi (if any) and each partner.
#[derive(Clone, Default)]
struct PartyBudgets {
    lead: Option<Budget>,
    partners: Vec<Option<Budget>>,
}

impl PartyBudgets {
    fn of(&self, owner: Owner) -> Option<&Budget> {
        match owner {
            Owner::Lead => self.lead.as_ref(),
            Owner::Partner(i) => self.partners.get(i).and_then(Option::as_ref),
        }
    }
}

/// What program `h` does: its effects in text table `table`'s words, for a line that lists them (`I18n.join`).
fn effect_of(names: &crate::games::Names, h: EntryHandle, table: &str) -> ModelRc<slint::SharedString> {
    let named: crate::builds::cards::Named = &|_, _| None;
    crate::builds::overview::items(crate::builds::cards::lines_in(names, h, table, named).unwrap_or_default().into_iter().map(|l| l.text))
}

/// A program's row: its name, its slots as budget `b` costs them, its color. (What it does is the strip's: the user,
/// "no need to show effects in the library column since they get shown on mouseover".)
fn program_row(words: &crate::builds::words::Words, content: &Content, h: EntryHandle, b: Option<&Budget>) -> PartyRow {
    PartyRow { name: words.names.entry(h).into(), blocks: b.map_or(0, |b| b.cost_of(content, h)) as i32, tone: tone(content, h), ..Default::default() }
}

/// What the strip says of a navi as a build would field it: its HP, its buster's attack (as the games show it, 1 to 5),
/// the stats its game's rules give it (an EXE5 team navi's own chip's damage) and its gauge's slots: what its programs
/// raise. (No charge: none raises it; the user, "remove charge from the party member info as well since nothing
/// changes it".)
#[derive(Clone, Debug, Default)]
struct NaviFacts {
    hp: i32,
    attack: i32,
    named: Vec<NamedStat>,
    gauge: Option<u32>,
}

impl NaviFacts {
    /// Member `member` of a checked build (0 for lead team navi, `i + 1` for partner `i`).
    fn of_member(checked: &mut Checked, content: &std::sync::Arc<Content>, side: &saito_match::Side, member: usize) -> Option<NaviFacts> {
        let s = *checked.members().get(member)?;
        let o = checked.overview(content, side);
        let gauge = match member {
            0 => o.as_ref().and_then(|o| o.budget(OWN_PROGRAMS_FIELD)).map(|b| b.count.limit),
            m => o.as_ref().and_then(|o| o.budget_of(FIELD, m - 1)).map(|b| b.count.limit),
        };
        Some(NaviFacts {
            hp: s.max_hp.into(),
            attack: i32::from(s.attack) + 1,
            named: o.as_ref().and_then(|o| o.members.iter().find(|(m, _)| *m == member)).map(|(_, s)| s.clone()).unwrap_or_default(),
            gauge,
        })
    }

    /// Partner `i` of a checked build (its party's member `i + 1`).
    fn of(checked: &mut Checked, content: &std::sync::Arc<Content>, side: &saito_match::Side, i: usize) -> Option<NaviFacts> {
        Self::of_member(checked, content, side, i + 1)
    }

    /// Its stats as the strip says them, by what the navi `alone` (no programs) has: what its programs added in cyan,
    /// as the overview says it; its gauge's slots last, labeled `slots` (the user: "why is "gauge 8 blocks" not like a
    /// stat like everything else in the header in party editor?", then "or "slots" i guess instead of blocks"). (Their
    /// labels: HP and Attack the view's; a stat of the rules' own its name.)
    fn lines(&self, alone: &NaviFacts, words: &crate::builds::words::Words, slots: &str) -> Vec<StatLine> {
        let line = |with: i32, base: i32| stat_line(String::new(), Stat { base, value: with });
        let gauge = self.gauge.map(|g| stat_line(slots.to_string(), Stat { base: alone.gauge.unwrap_or(g) as i32, value: g as i32 }));
        [line(self.hp, alone.hp), line(self.attack, alone.attack)]
            .into_iter()
            .chain(self.named.iter().map(|s| stat_line(stat_name(words, s), s.stat)))
            .chain(gauge)
            .collect()
    }
}

/// The most letters a search has.
const SEARCH_LENGTH: usize = 24;

/// The library's sorts, in the order its select lists them (`PartyView.sorts`): the game's order (the navis' by their
/// versions), the name, the slots (the programs' alone).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Library,
    Name,
    Blocks,
}
const PROGRAM_SORTS: [Sort; 3] = [Sort::Library, Sort::Name, Sort::Blocks];
const NAVI_SORTS: [Sort; 2] = [Sort::Library, Sort::Name];

/// How a library is listed: what its search holds, its sort (by its place in the sorts) and direction.
#[derive(Clone, Default)]
struct Listing {
    search: String,
    sort: usize,
    descending: bool,
}

/// What the strip describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Described {
    /// A slot of PARTY's, by its row.
    Slot(usize),
    /// A row of the library.
    Row(usize),
}

/// What control `id` (at `index`) puts in the strip.
fn described(id: &str, index: i32) -> Option<Described> {
    let at = index.max(0) as usize;
    match id {
        SLOTS => Some(Described::Slot(at)),
        ROWS => Some(Described::Row(at)),
        _ => None,
    }
}

#[derive(Default)]
pub struct Party {
    shape: Option<Shape>,
    /// PARTY's slots as the list shows them, and the one the library is of: the slot the cursor was last on.
    slots: Vec<Slot>,
    slot: usize,
    /// The navis offered, in their versions' order (the builds data's groups'; those of none last), and the library's
    /// programs, in the game's order; and those the library's rows show of them, as searched and sorted.
    navis: Vec<NaviHandle>,
    library: Vec<EntryHandle>,
    shown_navis: Vec<NaviHandle>,
    shown_programs: Vec<EntryHandle>,
    navi_listing: Listing,
    program_listing: Listing,
    refused: i32,
    /// What the strip describes: the focused thing, and the one under the pointer, which it describes in its place.
    described: Option<Described>,
    hovered: Option<Described>,
    /// Why the last thing asked wasn't done, until the cursor moves.
    notice: Option<String>,
    /// A navi's facts alone (no programs), as the build would field it: asked once a navi (a check of the build with it
    /// the one partner), for the strip's description of a navi, and what a partner's programs add to it.
    alone: std::collections::HashMap<NaviHandle, Option<NaviFacts>>,
    /// The party's budgets, asked once until the build changes (`changed` forgets them): a drag asks them at every
    /// move.
    kept_budgets: Option<PartyBudgets>,
    /// A budget the programs are costed by (`learn_costing`).
    costing: Option<Budget>,
}

impl Party {
    /// The party's shape and the library's programs, read the first time.
    fn read(&mut self, draft: &Draft) -> Option<Shape> {
        if self.shape.is_none() {
            let content = draft.content();
            let shape = Shape::of(content)?;
            let mut programs: Vec<((), EntryHandle)> = crate::builds::edit::of_game(content, &draft.kit.game, Registry::Entry, Some(&shape.collection)).into_iter().map(|h| ((), EntryHandle(h))).collect();
            draft.kit.order.entries(content, &shape.collection, &mut programs);
            self.library = programs.into_iter().map(|(_, h)| h).collect();
            self.shape = Some(shape);
        }
        self.shape.clone()
    }

    fn partners(&self, draft: &Draft) -> Vec<Partner> {
        self.shape.as_ref().map(|s| partners(draft.content(), draft.side(), s)).unwrap_or_default()
    }

    fn lead_programs(&self, draft: &Draft) -> Vec<EntryHandle> {
        own_programs(draft.content(), draft.side())
    }

    /// The marked slot: what the library is of.
    fn marked(&self) -> Slot {
        self.slots.get(self.slot).copied().unwrap_or(Slot::Open(0))
    }

    /// The library lists navis (the marked slot is a partner head).
    fn navis_shown(&self) -> bool {
        self.marked().is_head()
    }

    /// Lead team navi and each partner's gauge, and the round's navi changes.
    fn budgets(draft: &mut Draft) -> (PartyBudgets, Option<u32>) {
        let content = draft.content().clone();
        let side = draft.side().clone();
        let overview = draft.checked.overview(&content, &side);
        let budgets = PartyBudgets {
            lead: own_budget(overview.as_ref()),
            partners: crate::builds::party::budgets(&content, &side, overview.as_ref()),
        };
        (budgets, overview.and_then(|o| o.switches))
    }

    /// The party's budgets, asked once until the build changes.
    fn kept_budgets(&mut self, draft: &mut Draft) -> PartyBudgets {
        if self.kept_budgets.is_none() {
            self.kept_budgets = Some(Party::budgets(draft).0);
        }
        self.kept_budgets.clone().unwrap_or_default()
    }

    /// The ruleset category of the partners' programs: the party's field and its records' programs' field, by their
    /// path (`party.programs`: saito-match's categories over a field of a list's records).
    fn programs_category(&self) -> String {
        format!("{FIELD}.{}", self.shape.as_ref().map_or("", |s| s.programs.as_str()))
    }

    /// The build's problems with the lead team navi's own PartyCust programs.
    fn own_problems(draft: &Draft) -> Vec<&Problem> {
        draft
            .checked
            .problems
            .iter()
            .filter(|p| draft.facts_of(p).iter().any(|f| f == OWN_PROGRAMS_FIELD))
            .collect()
    }

    /// The build's problems with partner `i`, `partner` (its own, the party's about no partner, and a mix of versions
    /// its navi or a program of it is in: a navi of the other version, put in as anything of it is anywhere, is marked,
    /// with every other thing of the mix, and says so).
    fn problems<'d>(draft: &'d Draft, i: usize, partner: Option<&Partner>) -> Vec<&'d Problem> {
        draft
            .checked
            .problems
            .iter()
            .filter(|p| match &p.what {
                saito_match::Wrong::Mixed { items, .. } => partner.is_some_and(|partner| {
                    let navi = partner.navi.map(|n| (Registry::Navi, Item::of(n.0)));
                    navi.into_iter().chain(partner.programs.iter().map(|h| (Registry::Entry, Item::of(h.0)))).any(|x| items.contains(&x))
                }),
                _ => draft.facts_of(p).iter().any(|f| f == FIELD) && (p.at(i) || p.entry.is_none()),
            })
            .collect()
    }

    /// Why program `h` can't go into slot `slot` now, if it can't: the build's ruleset holds it back (DS-only, for
    /// every partner, or for the slot's owner's navi), or the gauge has no room for it (in a program's place, with
    /// that program's slots free; at a list's end, as it is; a list full: no words).
    fn program_refusal(&self, shell: &Shell, draft: &Draft, budgets: &PartyBudgets, slot: Slot, h: EntryHandle) -> Option<String> {
        let ui = shell.ui();
        if !draft.allow_ds() {
            return Some(ui.global::<BuildWords>().invoke_not_allowed_here().into());
        }
        let content = draft.content();
        let partners = self.partners(draft);
        let owner_navi = match slot.owner()? {
            Owner::Lead => Some(draft.side().navi(content)),
            Owner::Partner(i) => partners.get(i).and_then(|p| p.navi),
        };
        if !draft.allows_for(&self.programs_category(), owner_navi.map(|n| n.0), h.0) {
            return Some(ui.global::<BuildWords>().invoke_not_allowed_here().into());
        }
        let needs = |needs: u32| Some(ui.global::<PartyView>().invoke_needs_blocks(needs as i32).into());
        let room = self.shape.as_ref().map_or(0, |s| s.program_room);
        match slot {
            Slot::LeadHead | Slot::LeadEnd => {
                if self.lead_programs(draft).len() >= room {
                    return Some(String::new());
                }
                match budgets.of(Owner::Lead).and_then(|b| b.refusal(content, draft.side(), h)) {
                    Some(EntryRefusal::Needs { needs: n, .. }) => needs(n),
                    _ => None,
                }
            }
            Slot::LeadProgram(k) => {
                let b = budgets.of(Owner::Lead)?;
                let old = self.lead_programs(draft).get(k).copied()?;
                let (cost, free) = (b.cost_of(content, h), b.count.free() + b.cost_of(content, old));
                if cost > free { needs(cost) } else { None }
            }
            Slot::End(i) => {
                if partners.get(i).is_none_or(|p| p.programs.len() >= room) {
                    return Some(String::new());
                }
                match budgets.of(Owner::Partner(i)).and_then(|b| b.refusal(content, draft.side(), h)) {
                    Some(EntryRefusal::Needs { needs: n, .. }) => needs(n),
                    _ => None,
                }
            }
            Slot::Program(i, k) => {
                let b = budgets.of(Owner::Partner(i))?;
                let old = partners.get(i).and_then(|p| p.programs.get(k).copied())?;
                let (cost, free) = (b.cost_of(content, h), b.count.free() + b.cost_of(content, old));
                if cost > free { needs(cost) } else { None }
            }
            _ => None,
        }
    }

    /// Why navi `n` can't go into a head now, if it can't: it is in the party already, or the build's ruleset holds it
    /// back (or forbids 5DS). One of the other version (a ruleset of one version per build) goes in, as anything of it
    /// does anywhere: the mix is the build's problem, its partners' heads red.
    fn navi_refusal(&self, shell: &Shell, draft: &Draft, n: NaviHandle) -> Option<String> {
        let ui = shell.ui();
        if !draft.allow_ds() {
            return Some(ui.global::<BuildWords>().invoke_not_allowed_here().into());
        }
        if self.partners(draft).iter().any(|p| p.navi == Some(n)) {
            return Some(ui.global::<PartyView>().get_in_party().into());
        }
        if !draft.allows(FIELD, n.0) {
            return Some(ui.global::<BuildWords>().invoke_not_allowed_here().into());
        }
        None
    }

    /// The library's rows for the marked slot: the navis (each its face, name and version's mark) or the programs
    /// (each its slots, name and cost), and why each can't go into the slot (red, the row greyed).
    fn library_rows(&self, shell: &Shell, draft: &Draft, words: &crate::builds::words::Words, budgets: &PartyBudgets) -> Vec<PartyRow> {
        let content = draft.content();
        let kit = &draft.kit;
        let slot = self.marked();
        if slot.is_head() {
            return self
                .shown_navis
                .iter()
                .map(|&n| PartyRow {
                    name: words.names.navi_long(n).into(),
                    face: kit.pictures.navi(n),
                    version: kit.version_tag_of(words.names, Registry::Navi, &Item::of(n.0)),
                    why: self.navi_refusal(shell, draft, n).unwrap_or_default().into(),
                    ..Default::default()
                })
                .collect();
        }
        let budget = slot.owner().and_then(|o| budgets.of(o));
        self.shown_programs
            .iter()
            .map(|&h| {
                let mut r = program_row(words, content, h, budget.or(self.costing.as_ref()));
                if let Some(why) = self.program_refusal(shell, draft, budgets, slot, h) {
                    r.why = why.into();
                }
                r
            })
            .collect()
    }

    /// A budget to cost the programs by, learned once (its cost field is the rules' overview's, every partner's the
    /// same): the lead's or a partner's, else a check's of the build with the first navi offered its one partner. The
    /// library's rows have their slots with no partner too, and the slots' column is as wide as the most a program fills.
    fn learn_costing(&mut self, draft: &Draft, budgets: &PartyBudgets) {
        if self.costing.is_some() {
            return;
        }
        self.costing = budgets.lead.clone().or_else(|| budgets.partners.iter().flatten().next().cloned()).or_else(|| {
            let shape = self.shape.clone()?;
            let n = *self.navis.first()?;
            let content = draft.content().clone();
            let mut side = draft.side().clone();
            write(&content, &mut side, &shape, &[Partner { navi: Some(n), programs: Vec::new() }]).then_some(())?;
            Checked::of(&content, &draft.kit.game, &side).overview(&content, &side)?.budget_of(FIELD, 0).cloned()
        });
    }

    /// The navis offered as partners, in their versions' order (team navis other than the build's lead navi).
    fn offered(&self, draft: &Draft) -> Vec<NaviHandle> {
        let content = draft.content();
        let kit = &draft.kit;
        let own = draft.side().navi(content);
        let rulesets = kit.rulesets();
        let group_of = |n: NaviHandle| rulesets.as_ref().and_then(|r| r.group_of(Registry::Navi, &Item::of(n.0))).map(|g| g.id.clone());
        let rank = |n: NaviHandle| group_of(n).and_then(|id| kit.builds.groups.iter().position(|g| g.id == id)).unwrap_or(usize::MAX);
        let mut offered: Vec<NaviHandle> = saito_match::navis(content, &kit.game)
            .into_iter()
            .filter(|&n| n != own && !content.navi(n).changes_form())
            .collect();
        offered.sort_by_key(|&n| rank(n));
        offered
    }

    /// The library's rows, as searched (a name that holds what was typed) and sorted (the game's order kept among
    /// equals), the navis' and the programs'.
    fn list(&mut self, shell: &Shell, draft: &Draft) {
        let content = draft.content();
        let costing = self.costing.clone();
        let (navi_listing, program_listing) = (self.navi_listing.clone(), self.program_listing.clone());
        let (navis, programs) = draft.with_words(shell, |words| {
            fn sorted<T: Copy>(mut found: Vec<(String, u32, T)>, sort: Sort, descending: bool) -> Vec<T> {
                match sort {
                    Sort::Library => {}
                    Sort::Name => found.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase())),
                    Sort::Blocks => found.sort_by_key(|f| f.1),
                }
                if descending {
                    found.reverse();
                }
                found.into_iter().map(|f| f.2).collect()
            }
            let wanted = |listing: &Listing, name: &str| listing.search.is_empty() || name.to_lowercase().contains(&listing.search.to_lowercase());
            let navis: Vec<(String, u32, NaviHandle)> = self.navis.iter().map(|&n| (words.names.navi_long(n), 0, n)).filter(|f| wanted(&navi_listing, &f.0)).collect();
            let programs: Vec<(String, u32, EntryHandle)> =
                self.library.iter().map(|&h| (words.names.entry(h), costing.as_ref().map_or(0, |b| b.cost_of(content, h)), h)).filter(|f| wanted(&program_listing, &f.0)).collect();
            (
                sorted(navis, NAVI_SORTS[navi_listing.sort.min(NAVI_SORTS.len() - 1)], navi_listing.descending),
                sorted(programs, PROGRAM_SORTS[program_listing.sort.min(PROGRAM_SORTS.len() - 1)], program_listing.descending),
            )
        });
        self.shown_navis = navis;
        self.shown_programs = programs;
    }

    /// The listing of the library shown.
    fn listing(&mut self) -> &mut Listing {
        if self.navis_shown() { &mut self.navi_listing } else { &mut self.program_listing }
    }

    /// Slot row `row` is the marked one, the library of what it takes beside it (its rows' refusals for it). Where the
    /// library lists the same kind as before (a drag over PARTY marks the slot it is over), its rows change in place:
    /// a model set anew makes every row anew, and a row the pointer pressed or holds would be let go.
    fn select(&mut self, shell: &Shell, draft: &mut Draft, row: usize) {
        let was = self.navis_shown();
        self.slot = row.min(self.slots.len().saturating_sub(1));
        let budgets = self.kept_budgets(draft);
        let rows = draft.with_words(shell, |words| self.library_rows(shell, draft, words, &budgets));
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let model = view.get_rows();
        if was == self.navis_shown() && model.row_count() == rows.len() {
            for (k, r) in rows.into_iter().enumerate() {
                model.set_row_data(k, r);
            }
        } else {
            view.set_rows(ModelRc::new(VecModel::from(rows)));
        }
        let navis = self.navis_shown();
        let listing = self.listing().clone();
        let free = self.marked().owner().and_then(|o| budgets.of(o)).map_or(0, |b| b.count.free());
        view.set_slot(self.slot as i32);
        view.set_navis_shown(navis);
        view.set_search(listing.search.as_str().into());
        view.set_sort(listing.sort as i32);
        view.set_descending(listing.descending);
        view.set_free(free as i32);
        self.show_detail(shell, draft);
    }

    /// Everything shown anew: PARTY's slots (`slot`: the row to mark, else the one marked, as near as there is), the
    /// library, the strip.
    fn changed(&mut self, shell: &Shell, draft: &mut Draft) {
        self.kept_budgets = None;
        let Some(shape) = self.read(draft) else { return };
        let content = draft.content().clone();
        let partners = partners(&content, draft.side(), &shape);
        let own = draft.side().navi(&content);
        let lead_progs = own_programs(&content, draft.side());
        let show_lead = has_own_party_cust(&content, draft.side());
        let (budgets, changes) = Party::budgets(draft);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let say = ui.global::<BuildWords>();
        let kit = draft.kit.clone();
        let category = self.programs_category();
        self.navis = self.offered(draft);
        self.learn_costing(draft, &budgets);
        let most = self.costing.as_ref().and_then(|b| self.library.iter().map(|&h| b.cost_of(&content, h)).max()).unwrap_or(1);
        self.list(shell, draft);
        let counts: Vec<usize> = partners.iter().map(|p| p.programs.len()).collect();
        self.slots = slots_of(show_lead.then_some(lead_progs.len()), &counts, shape.room, shape.program_room);
        self.slot = self.slot.min(self.slots.len().saturating_sub(1));
        let slot_rows: Vec<PartySlot> = draft.with_words(shell, |words| {
            let head_slot = |navi: Option<NaviHandle>, progs: &[EntryHandle], b: Option<&Budget>, bad: bool, locked: bool| {
                let mut runs: Vec<i32> = progs.iter().enumerate().flat_map(|(k, &h)| std::iter::repeat_n(k as i32, b.map_or(0, |b| b.cost_of(&content, h)) as usize)).collect();
                let gauge = b.map_or(0, |b| b.count.limit as usize);
                let last = progs.len() as i32;
                while runs.len() < gauge {
                    runs.push(last);
                }
                PartySlot {
                    kind: PartySlotKind::Head,
                    locked,
                    face: navi.map(|n| kit.pictures.navi(n)).unwrap_or_default(),
                    name: navi.map(|n| words.names.navi_long(n)).unwrap_or_default().into(),
                    version: navi.map(|n| kit.version_tag_of(words.names, Registry::Navi, &Item::of(n.0))).unwrap_or_default(),
                    bad,
                    gauge: gauge as i32,
                    used: b.map_or(0, |b| b.count.used as i32),
                    tones: ModelRc::new(VecModel::from(program_tones(&content, progs, b))),
                    runs: ModelRc::new(VecModel::from(runs)),
                    ..Default::default()
                }
            };
            self.slots
                .iter()
                .map(|&s| match s {
                    Slot::LeadHead => head_slot(Some(own), &lead_progs, budgets.of(Owner::Lead), !Party::own_problems(draft).is_empty(), true),
                    Slot::LeadProgram(k) => {
                        let h = lead_progs[k];
                        let mut row = program_row(words, &content, h, budgets.of(Owner::Lead));
                        if !draft.allow_ds() || !draft.allows_for(&category, Some(own.0), h.0) {
                            row.note = say.invoke_not_allowed_here();
                            row.bad = true;
                        }
                        PartySlot { kind: PartySlotKind::Program, row, ..Default::default() }
                    }
                    Slot::LeadEnd => PartySlot { kind: PartySlotKind::End, ..Default::default() },
                    Slot::Head(i) => {
                        let p = &partners[i];
                        head_slot(p.navi, &p.programs, budgets.of(Owner::Partner(i)), !Party::problems(draft, i, Some(p)).is_empty(), false)
                    }
                    Slot::Program(i, k) => {
                        let h = partners[i].programs[k];
                        let mut row = program_row(words, &content, h, budgets.of(Owner::Partner(i)));
                        if !draft.allow_ds() || !draft.allows_for(&category, partners[i].navi.map(|n| n.0), h.0) {
                            row.note = say.invoke_not_allowed_here();
                            row.bad = true;
                        }
                        PartySlot { kind: PartySlotKind::Program, row, ..Default::default() }
                    }
                    Slot::End(_) => PartySlot { kind: PartySlotKind::End, ..Default::default() },
                    Slot::Open(_) => PartySlot { kind: PartySlotKind::Open, ..Default::default() },
                })
                .collect()
        });
        view.set_most_blocks(most as i32);
        view.set_partners(partners.len() as i32);
        view.set_room(shape.room as i32);
        view.set_changes(changes.unwrap_or(0) as i32);
        view.set_slots(ModelRc::new(VecModel::from(slot_rows)));
        view.set_problems(draft.pane_line(shell, &[FIELD.to_string(), OWN_PROGRAMS_FIELD.to_string()], false).into());
        // (The library anew: its kind may be the same, its rows another number.)
        view.set_rows(ModelRc::new(VecModel::from(Vec::<PartyRow>::new())));
        self.select(shell, draft, self.slot);
    }

    /// The row of slot `slot`, if PARTY shows it.
    fn row_of(&self, slot: Slot) -> Option<usize> {
        self.slots.iter().position(|&s| s == slot)
    }

    /// The collection's own text table of its programs' effects and kinds (`<collection>_effects`).
    fn effects_table(&self) -> String {
        format!("{}_effects", self.shape.as_ref().map_or("", |s| s.collection.as_str()))
    }

    /// A navi's facts as the build would field it, and alone (no programs; kept, a navi once): a partner's its own,
    /// its programs' in; a navi not in the party's as a check of the build with it the one partner says.
    fn navi_facts(&mut self, draft: &mut Draft, n: NaviHandle) -> Option<(NaviFacts, NaviFacts)> {
        let content = draft.content().clone();
        let own = draft.side().navi(&content);
        if !self.alone.contains_key(&n) {
            let shape = self.shape.clone()?;
            let mut side = draft.side().clone();
            let facts = if n == own {
                let dummy = *self.navis.first()?;
                let _ = write_own_programs(&content, &mut side, &[]);
                write(&content, &mut side, &shape, &[Partner { navi: Some(dummy), programs: Vec::new() }])
                    .then(|| NaviFacts::of_member(&mut Checked::of(&content, &draft.kit.game, &side), &content, &side, 0))
                    .flatten()
            } else {
                write(&content, &mut side, &shape, &[Partner { navi: Some(n), programs: Vec::new() }])
                    .then(|| NaviFacts::of(&mut Checked::of(&content, &draft.kit.game, &side), &content, &side, 0))
                    .flatten()
            };
            self.alone.insert(n, facts);
        }
        let alone = self.alone.get(&n).cloned().flatten()?;
        let partners = self.partners(draft);
        let with = if n == own {
            let side = draft.side().clone();
            if partners.is_empty() && own_programs(&content, &side).is_empty() {
                alone.clone()
            } else {
                NaviFacts::of_member(&mut draft.checked, &content, &side, 0)?
            }
        } else {
            match partners.iter().position(|p| p.navi == Some(n)) {
                Some(i) => {
                    let side = draft.side().clone();
                    NaviFacts::of(&mut draft.checked, &content, &side, i)?
                }
                None => alone.clone(),
            }
        };
        Some((with, alone))
    }

    /// The strip: what the cursor is on; while the pointer is on a slot or a row, that one. A head or a navi
    /// of the library: the navi, its stats; a program: the program, and (the library's) whether it fits the marked slot.
    fn show_detail(&mut self, shell: &Shell, draft: &mut Draft) {
        let content = draft.content().clone();
        let own = draft.side().navi(&content);
        let lead_progs = own_programs(&content, draft.side());
        let partners = self.partners(draft);
        let budgets = self.kept_budgets(draft);
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let notice = if self.hovered.is_some_and(|h| Some(h) != self.described) { None } else { self.notice.clone() };
        let kit = draft.kit.clone();
        let table = self.effects_table();
        let described = self.hovered.or(self.described);
        let marked = self.marked();
        let navi = match described {
            Some(Described::Slot(r)) => match self.slots.get(r) {
                Some(&Slot::LeadHead) => Some(own),
                Some(&Slot::Head(i)) => partners.get(i).and_then(|p| p.navi),
                _ => None,
            },
            Some(Described::Row(r)) if self.navis_shown() => self.shown_navis.get(r).copied(),
            _ => None,
        };
        let facts = navi.and_then(|n| self.navi_facts(draft, n));
        let element = navi.map(|n| kit.pictures.element(super::navi::family(&content, content.navi(n).element), shell.settings.hd_chip_art));
        let library_navi = matches!(described, Some(Described::Row(_)));
        let refusal = navi.filter(|_| library_navi).and_then(|n| self.navi_refusal(shell, draft, n));
        let detail = draft.with_words(shell, |words| {
            let mut d = match (described, navi) {
                (_, Some(n)) => {
                    let tag = kit.version_tag_of(words.names, Registry::Navi, &Item::of(n.0));
                    let stats = facts.as_ref().map(|(with, alone)| with.lines(alone, words, &view.invoke_slots_label())).unwrap_or_default();
                    let fault = if library_navi {
                        String::new()
                    } else if n == own {
                        draft.line(shell, Party::own_problems(draft))
                    } else if let Some(i) = partners.iter().position(|p| p.navi == Some(n)) {
                        draft.line(shell, Party::problems(draft, i, partners.get(i)))
                    } else {
                        String::new()
                    };
                    saito_ui::PartyDetail {
                        there: true,
                        face: kit.pictures.navi(n),
                        name: words.names.navi_long(n).into(),
                        version: tag,
                        stats: ModelRc::new(VecModel::from(stats)),
                        element: element.clone().unwrap_or_default(),
                        fault: fault.into(),
                        problem: refusal.clone().unwrap_or_default().into(),
                        ..Default::default()
                    }
                }
                (Some(Described::Slot(r)), _) => match self.slots.get(r) {
                    Some(&Slot::LeadProgram(k)) => match lead_progs.get(k) {
                        Some(&h) => {
                            let mut d = self.program_detail(shell, draft, words, &table, &budgets, Slot::LeadProgram(k), h, false);
                            d.fault = draft.line(shell, Party::own_problems(draft)).into();
                            d
                        }
                        None => Default::default(),
                    },
                    Some(&Slot::Program(i, k)) => match partners.get(i).and_then(|p| p.programs.get(k)) {
                        Some(&h) => {
                            let mut d = self.program_detail(shell, draft, words, &table, &budgets, Slot::Program(i, k), h, false);
                            d.fault = draft.line(shell, Party::problems(draft, i, partners.get(i))).into();
                            d
                        }
                        None => Default::default(),
                    },
                    _ => Default::default(),
                },
                (Some(Described::Row(r)), _) => match self.shown_programs.get(r) {
                    Some(&h) if !self.navis_shown() => self.program_detail(shell, draft, words, &table, &budgets, marked, h, true),
                    _ => Default::default(),
                },
                _ => Default::default(),
            };
            if let Some(n) = &notice {
                d.problem = n.as_str().into();
            }
            d
        });
        view.set_detail(detail);
    }

    /// A program as the strip describes it, as the Folder tab's strip a chip: its slots and name, what it does; its
    /// slots, its kind, and whether they fit slot `slot` (`fitting`: it is the library's, which puts it there).
    #[allow(clippy::too_many_arguments)]
    fn program_detail(&self, shell: &Shell, draft: &Draft, words: &crate::builds::words::Words, table: &str, budgets: &PartyBudgets, slot: Slot, h: EntryHandle, fitting: bool) -> saito_ui::PartyDetail {
        let content = draft.content();
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let b = slot.owner().and_then(|o| budgets.of(o));
        let cost = b.or(self.costing.as_ref()).map_or(0, |b| b.cost_of(content, h)) as i32;
        let kind = content.defs.definitions.get(Registry::Entry, &content.defs.entry(h).key).and_then(|d| d.spec.field("kind").str().map(str::to_string));
        let kind = kind.and_then(|k| words.names.text(table, &format!("kind.{k}")));
        let why = if fitting { self.program_refusal(shell, draft, budgets, slot, h) } else { None };
        let fits = (fitting && why.is_none() && b.is_some()).then(|| view.invoke_fits_word(b.map_or(0, |b| b.count.free()) as i32));
        let facts: Vec<slint::SharedString> = [Some(view.invoke_blocks_word(cost)), kind.map(Into::into), fits].into_iter().flatten().collect();
        saito_ui::PartyDetail {
            there: true,
            name: words.names.entry(h).into(),
            blocks: cost,
            tone: tone(content, h),
            description: effect_of(words.names, h, table),
            facts: ModelRc::new(VecModel::from(facts)),
            problem: why.unwrap_or_default().into(),
            ..Default::default()
        }
    }

    /// Change the build's partners: whether they changed.
    fn change(&self, draft: &mut Draft, change: impl FnOnce(&mut Vec<Partner>) -> bool) -> bool {
        let Some(shape) = self.shape.clone() else { return false };
        draft.edit(|content, side| {
            let mut ps = partners(content, side, &shape);
            change(&mut ps) && write(content, side, &shape, &ps)
        })
    }

    /// Change the build's lead PartyCust programs and/or partners together: whether anything changed.
    fn change_all(&self, draft: &mut Draft, change: impl FnOnce(&mut Vec<EntryHandle>, &mut Vec<Partner>) -> bool) -> bool {
        let Some(shape) = self.shape.clone() else { return false };
        draft.edit(|content, side| {
            let mut lead = own_programs(content, side);
            let mut ps = partners(content, side, &shape);
            if !change(&mut lead, &mut ps) {
                return false;
            }
            let ok_lead = !saito_match::facts::takes(content, OWN_PROGRAMS_FIELD) || write_own_programs(content, side, &lead);
            let ok_ps = write(content, side, &shape, &ps);
            ok_lead && ok_ps
        })
    }

    /// The build changed: everything shown anew, slot `to` the marked one (where PARTY shows it).
    fn changed_to(&mut self, shell: &mut Shell, draft: &mut Draft, to: Slot) {
        self.changed(shell, draft);
        if let Some(row) = self.row_of(to) {
            self.select(shell, draft, row);
            if !shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) {
                shell.nav.focus(SLOTS, row as i32);
            }
        }
    }

    /// Refuse the library's row `row`: it shakes, with the refusal's sound.
    fn refuse(&mut self, shell: &mut Shell, row: usize) {
        shell.sound.play(Cue::Refused);
        self.refused += 1;
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        view.set_refused(row as i32);
        view.set_refused_seq(self.refused);
    }

    /// Navi `n` (the library's row `row`) put in head `slot`: an Empty head's place a partner of its own; a partner's
    /// head, that partner taken out (its programs with it: another navi's gauge isn't its) and `n` in its place.
    /// Refused as its row says.
    fn put_navi(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, slot: Slot) {
        let Some(&n) = self.shown_navis.get(row) else { return };
        if self.navi_refusal(shell, draft, n).is_some() {
            return self.refuse(shell, row);
        }
        let mut place = 0;
        if self.change(draft, |ps| {
            let partner = Partner { navi: Some(n), programs: Vec::new() };
            match slot {
                Slot::Head(i) if i < ps.len() => {
                    ps[i] = partner;
                    place = i;
                }
                _ => {
                    ps.push(partner);
                    place = ps.len() - 1;
                }
            }
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.changed_to(shell, draft, Slot::Head(place));
        }
    }

    /// Program `h` (the library's row `row`) put in program slot `slot`: in a program's place, in place of it; from a
    /// head or Empty row, at its list's end. Refused as its row says.
    fn put_program(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, slot: Slot) {
        let Some(&h) = self.shown_programs.get(row) else { return };
        let budgets = self.kept_budgets(draft);
        if self.program_refusal(shell, draft, &budgets, slot, h).is_some() {
            return self.refuse(shell, row);
        }
        let changed = self.change_all(draft, |lead, ps| match slot {
            Slot::LeadProgram(k) if k < lead.len() => {
                lead[k] = h;
                true
            }
            Slot::LeadHead | Slot::LeadEnd => {
                lead.push(h);
                true
            }
            Slot::Program(i, k) if k < ps.get(i).map_or(0, |p| p.programs.len()) => {
                ps[i].programs[k] = h;
                true
            }
            Slot::End(i) if i < ps.len() => {
                ps[i].programs.push(h);
                true
            }
            _ => false,
        });
        if changed {
            shell.sound.play(Cue::Pick);
            let room = self.shape.as_ref().map_or(0, |s| s.program_room);
            let to = match slot {
                Slot::LeadHead | Slot::LeadEnd => {
                    let n = self.lead_programs(draft).len();
                    if n < room { Slot::LeadEnd } else { Slot::LeadProgram(n.saturating_sub(1)) }
                }
                Slot::End(i) => {
                    let n = self.partners(draft).get(i).map_or(0, |p| p.programs.len());
                    if n < room { Slot::End(i) } else { Slot::Program(i, n.saturating_sub(1)) }
                }
                s => s,
            };
            self.changed_to(shell, draft, to);
        }
    }

    /// Program `h` (the library's row `row`) put in among `owner`'s at place `at` (a drag lets it go there),
    /// refused as an Empty row's would be.
    fn install(&mut self, shell: &mut Shell, draft: &mut Draft, row: usize, owner: Owner, at: usize) {
        let Some(&h) = self.shown_programs.get(row) else { return };
        let budgets = self.kept_budgets(draft);
        let end = match owner {
            Owner::Lead => Slot::LeadEnd,
            Owner::Partner(i) => Slot::End(i),
        };
        if self.program_refusal(shell, draft, &budgets, end, h).is_some() {
            return self.refuse(shell, row);
        }
        if self.change_all(draft, |lead, ps| match owner {
            Owner::Lead => {
                let at = at.min(lead.len());
                lead.insert(at, h);
                true
            }
            Owner::Partner(i) => {
                let Some(p) = ps.get_mut(i) else { return false };
                let at = at.min(p.programs.len());
                p.programs.insert(at, h);
                true
            }
        }) {
            shell.sound.play(Cue::Pick);
            let to = match owner {
                Owner::Lead => Slot::LeadProgram(at),
                Owner::Partner(i) => Slot::Program(i, at),
            };
            self.changed_to(shell, draft, to);
        }
    }

    /// Partner `i` taken out, its programs with it.
    fn remove_partner(&mut self, shell: &mut Shell, draft: &mut Draft, i: usize) {
        if self.change(draft, |ps| {
            if i >= ps.len() {
                return false;
            }
            ps.remove(i);
            true
        }) {
            shell.sound.play(Cue::Back);
            let left = self.partners(draft).len();
            self.changed_to(shell, draft, if i < left { Slot::Head(i) } else { Slot::Open(left) });
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    /// Program `k` of `owner`'s taken out.
    fn uninstall(&mut self, shell: &mut Shell, draft: &mut Draft, owner: Owner, k: usize) {
        if self.change_all(draft, |lead, ps| match owner {
            Owner::Lead if k < lead.len() => {
                lead.remove(k);
                true
            }
            Owner::Partner(i) => match ps.get_mut(i) {
                Some(p) if k < p.programs.len() => {
                    p.programs.remove(k);
                    true
                }
                _ => false,
            },
            _ => false,
        }) {
            shell.sound.play(Cue::Back);
            let to = match owner {
                Owner::Lead => {
                    let n = self.lead_programs(draft).len();
                    if k < n { Slot::LeadProgram(k) } else { Slot::LeadEnd }
                }
                Owner::Partner(i) => {
                    let n = self.partners(draft).get(i).map_or(0, |p| p.programs.len());
                    if k < n { Slot::Program(i, k) } else { Slot::End(i) }
                }
            };
            self.changed_to(shell, draft, to);
        } else {
            shell.sound.play(Cue::Refused);
        }
    }

    /// Program `k` of `from`'s moved to place `to` of `into`'s (the same or another owner); into another,
    /// refused as an install is.
    fn shift(&mut self, shell: &mut Shell, draft: &mut Draft, from: Owner, k: usize, into: Owner, to: usize) {
        let h = match from {
            Owner::Lead => self.lead_programs(draft).get(k).copied(),
            Owner::Partner(i) => self.partners(draft).get(i).and_then(|p| p.programs.get(k).copied()),
        };
        let Some(h) = h else { return };
        if into != from {
            let budgets = self.kept_budgets(draft);
            let end = match into {
                Owner::Lead => Slot::LeadEnd,
                Owner::Partner(i) => Slot::End(i),
            };
            if self.program_refusal(shell, draft, &budgets, end, h).is_some() {
                return shell.sound.play(Cue::Refused);
            }
        }
        if self.change_all(draft, |lead, ps| {
            let h = match from {
                Owner::Lead => lead.remove(k),
                Owner::Partner(i) => ps[i].programs.remove(k),
            };
            let at = match into {
                Owner::Lead => {
                    let at = to.min(lead.len());
                    lead.insert(at, h);
                    at
                }
                Owner::Partner(i) => {
                    let at = to.min(ps[i].programs.len());
                    ps[i].programs.insert(at, h);
                    at
                }
            };
            from != into || at != k
        }) {
            shell.sound.play(Cue::Pick);
            let slot = match into {
                Owner::Lead => {
                    let at = to.min(self.lead_programs(draft).len().saturating_sub(1));
                    Slot::LeadProgram(at)
                }
                Owner::Partner(i) => {
                    let at = to.min(self.partners(draft).get(i).map_or(0, |p| p.programs.len().saturating_sub(1)));
                    Slot::Program(i, at)
                }
            };
            self.changed_to(shell, draft, slot);
        }
    }

    /// Partners `from` and `to` trade places, their programs with them.
    fn trade_partners(&mut self, shell: &mut Shell, draft: &mut Draft, from: usize, to: usize) {
        if self.change(draft, |ps| {
            if from == to || from >= ps.len() || to >= ps.len() {
                return false;
            }
            ps.swap(from, to);
            true
        }) {
            shell.sound.play(Cue::Pick);
            self.changed_to(shell, draft, Slot::Head(to));
        }
    }

    /// Into the library of the marked slot: on a desktop the focus on its first row (its search, where it lists
    /// none), on a phone its sheet open.
    fn open_library(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        let to = if view.get_rows().row_count() == 0 { crate::nav::Key::new("lib.search", 0) } else { crate::nav::Key::new(ROWS, 0) };
        if shell.scale.phone {
            view.set_sheet(true);
            shell.nav.open("lib.", to);
        } else {
            shell.nav.focus(&to.id, to.index);
        }
    }

    fn close_sheet(&mut self, shell: &mut Shell) {
        let ui = shell.ui();
        let view = ui.global::<PartyView>();
        if view.get_sheet() {
            view.set_sheet(false);
            shell.nav.close("lib.");
        }
    }

    /// The slot row under window point `(x, y)` (a drag's pointer), and whether the pointer is in its upper half; none
    /// where it is off PARTY's slots.
    fn slot_at(&self, shell: &Shell, (x, y): (f32, f32)) -> Option<(usize, bool)> {
        let registry = shell.nav.registry();
        let registry = registry.borrow();
        let view = registry.view(SLOTS)?;
        let (first, pitch) = registry.first_row(SLOTS)?;
        if x < view.x || x > view.right() || y < view.y || y > view.bottom() {
            return None;
        }
        let at = (y - first) / pitch;
        let row = at.floor().max(0.0) as usize;
        (row < self.slots.len()).then_some((row, at.fract() < 0.5))
    }

    /// Tell the UI where what is dragged would land in PARTY's slots (`to`: a gap among the programs there; `swap`: the
    /// head there outlined), and why it can't go in, if it can't (said in the dragged row; let go, it is refused).
    fn show_slot_landing(&self, shell: &Shell, to: usize, swap: bool, why: Option<String>) {
        shell.ui().global::<PartyView>().set_drag_why(why.unwrap_or_default().into());
        editor::show_landing(shell, SLOTS, Some(to), swap);
    }

    /// Nothing shown of where a drag would land: into no list (the list it came from no longer scrolls near its
    /// edges for it).
    fn show_no_landing(&self, shell: &Shell) {
        shell.ui().global::<PartyView>().set_drag_why(Default::default());
        editor::show_landing(shell, "", None, false);
    }

    /// Where a program dragged to window point `at` would go among PARTY's: the owner and the place, and the slot row
    /// its gap shows at.
    fn program_landing(&self, shell: &Shell, draft: &Draft, at: i32) -> Option<(Owner, usize, usize)> {
        let (row, upper) = self.slot_at(shell, editor::pointer_at(at))?;
        let lead_count = self.lead_programs(draft).len();
        let counts: Vec<usize> = self.partners(draft).iter().map(|p| p.programs.len()).collect();
        let (owner, place) = program_place(&self.slots, row, upper, lead_count, &counts)?;
        Some((owner, place, row_of_place(&self.slots, owner, place)?))
    }
}

impl Tab for Party {
    fn id(&self) -> EditorTab {
        EditorTab::Party
    }

    fn fields(&self, _content: &Content) -> Vec<String> {
        vec![FIELD.into(), OWN_PROGRAMS_FIELD.into()]
    }

    /// Party is available to both MegaMan (5DS) and team navis (Gate-Party) under DS rulesets.
    fn own_navi_only(&self) -> bool {
        false
    }

    fn enabled_for(&self, draft: &Draft) -> bool {
        if draft.allow_ds() {
            return true;
        }
        let content = draft.content();
        let side = draft.side();
        let has_partners = Shape::of(content).is_some_and(|s| !partners(content, side, &s).is_empty());
        let has_own = !own_programs(content, side).is_empty();
        has_partners || has_own
    }

    fn main(&self) -> &'static str {
        SLOTS
    }

    fn cursor(&self) -> Option<usize> {
        Some(self.slot)
    }

    fn restore_cursor(&mut self, _draft: &Draft, _other_side: &saito_match::Side, cursor: Option<usize>, _is_undo: bool) {
        if let Some(slot) = cursor {
            self.slot = slot;
            self.described = Some(Described::Slot(slot));
        }
    }

    fn show(&mut self, shell: &mut Shell, draft: &mut Draft) {
        // (A phone turned sideways has the library beside PARTY: its sheet shuts.)
        if !shell.scale.phone {
            self.close_sheet(shell);
        }
        self.changed(shell, draft);
    }

    fn hide(&mut self, shell: &mut Shell, _draft: &mut Draft) {
        self.notice = None;
        self.hovered = None;
        self.close_sheet(shell);
    }

    fn activate(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        let index = index.max(0) as usize;
        match id {
            // (Enter on a slot: the library of what it takes; a phone's sheet.)
            SLOTS => {
                if index != self.slot {
                    self.select(shell, draft, index);
                }
                self.open_library(shell);
            }
            ROWS => {
                let slot = self.marked();
                if slot.is_head() { self.put_navi(shell, draft, index, slot) } else { self.put_program(shell, draft, index, slot) }
            }
            "library" => self.open_library(shell),
            "lib.close" => {
                self.close_sheet(shell);
                shell.nav.focus(SLOTS, self.slot as i32);
            }
            "lib.sort" => {
                let options = shell.ui().global::<PartyView>().get_sorts().iter().map(|s| s.to_string()).collect();
                let sort = self.listing().sort;
                shell.pick(id, 0, options, sort);
            }
            "lib.direction" => {
                let listing = self.listing();
                listing.descending = !listing.descending;
                self.changed(shell, draft);
            }
            "clear" => {
                let question = shell.ui().global::<PartyView>().get_clear_question();
                let empty = self.partners(draft).is_empty() && self.lead_programs(draft).is_empty();
                editor::ask_clear(shell, question, empty);
            }
            _ => {}
        }
    }

    fn choose(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, choice: i32) {
        let list = shell.ui().global::<saito_ui::Drag>().get_list().to_string();
        let from = usize::try_from(index).ok();
        let dragged_program = |slot: Option<Slot>| match slot {
            Some(Slot::LeadProgram(k)) => Some((Owner::Lead, k)),
            Some(Slot::Program(i, k)) => Some((Owner::Partner(i), k)),
            _ => None,
        };
        let end_slot = |owner: Owner| match owner {
            Owner::Lead => Slot::LeadEnd,
            Owner::Partner(i) => Slot::End(i),
        };
        match id {
            "lib.sort" => {
                let sorts = if self.navis_shown() { NAVI_SORTS.len() } else { PROGRAM_SORTS.len() };
                self.listing().sort = (choice.max(0) as usize).min(sorts - 1);
                self.changed(shell, draft);
            }
            // A slot dragged: a program among the programs (a gap where it would go; into another owner's, why it
            // can't, where it can't), a partner head onto another (outlined: they would trade places); onto the
            // library, what it holds is taken out let go there; elsewhere, nowhere (it goes back).
            "drag-over" if list == SLOTS => match from.and_then(|r| self.slots.get(r).copied()) {
                Some(Slot::Head(_) | Slot::Program(..) | Slot::LeadProgram(_)) if editor::over_library(shell, choice) => {
                    shell.ui().global::<PartyView>().set_drag_why(Default::default());
                    editor::show_remove(shell);
                }
                slot if dragged_program(slot).is_some() => {
                    let (src, k) = dragged_program(slot).unwrap();
                    match self.program_landing(shell, draft, choice) {
                        Some((dst, _, gap)) => {
                            let row = from.unwrap_or(0);
                            let to = if gap > row { gap - 1 } else { gap };
                            let why = if dst == src {
                                None
                            } else {
                                let budgets = self.kept_budgets(draft);
                                let h = match src {
                                    Owner::Lead => self.lead_programs(draft).get(k).copied(),
                                    Owner::Partner(i) => self.partners(draft).get(i).and_then(|p| p.programs.get(k).copied()),
                                };
                                h.and_then(|h| self.program_refusal(shell, draft, &budgets, end_slot(dst), h))
                            };
                            self.show_slot_landing(shell, to, false, why);
                        }
                        None => self.show_no_landing(shell),
                    }
                }
                Some(Slot::Head(i)) => match self.slot_at(shell, editor::pointer_at(choice)).and_then(|(r, _)| head_at(&self.slots, r)) {
                    Some(h) if matches!(self.slots[h], Slot::Head(j) if j != i) => self.show_slot_landing(shell, h, true, None),
                    _ => self.show_no_landing(shell),
                },
                _ => self.show_no_landing(shell),
            },
            "drag-drop" if list == SLOTS => {
                self.show_no_landing(shell);
                match from.and_then(|r| self.slots.get(r).copied()) {
                    slot if dragged_program(slot).is_some() && editor::over_library(shell, choice) => {
                        let (src, k) = dragged_program(slot).unwrap();
                        self.uninstall(shell, draft, src, k);
                    }
                    Some(Slot::Head(i)) if editor::over_library(shell, choice) => self.remove_partner(shell, draft, i),
                    slot if dragged_program(slot).is_some() => {
                        let (src, k) = dragged_program(slot).unwrap();
                        if let Some((dst, place, _)) = self.program_landing(shell, draft, choice) {
                            let to = if dst == src && place > k { place - 1 } else { place };
                            self.shift(shell, draft, src, k, dst, to);
                        }
                    }
                    Some(Slot::Head(i)) => {
                        if let Some(Slot::Head(j)) = self.slot_at(shell, editor::pointer_at(choice)).and_then(|(r, _)| head_at(&self.slots, r)).map(|h| self.slots[h]) {
                            self.trade_partners(shell, draft, i, j);
                        }
                    }
                    _ => {}
                }
            }
            // A library's row dragged: a program among an owner's (a gap where it would go), a navi onto a partner head
            // (outlined), each saying why it can't go in where it can't; let go there, it goes in, refused as Enter's
            // is; anywhere else, nothing.
            "drag-in-over" if index >= 0 => {
                let r = index as usize;
                if self.navis_shown() {
                    let target = self.slot_at(shell, editor::pointer_at(choice)).and_then(|(row, _)| head_at(&self.slots, row));
                    match (target, self.shown_navis.get(r).copied()) {
                        (Some(h), Some(n)) => {
                            let why = self.navi_refusal(shell, draft, n);
                            self.show_slot_landing(shell, h, true, why);
                        }
                        _ => self.show_no_landing(shell),
                    }
                } else {
                    match (self.program_landing(shell, draft, choice), self.shown_programs.get(r).copied()) {
                        (Some((dst, _, gap)), Some(h)) => {
                            let budgets = self.kept_budgets(draft);
                            let why = self.program_refusal(shell, draft, &budgets, end_slot(dst), h);
                            self.show_slot_landing(shell, gap, false, why);
                        }
                        _ => self.show_no_landing(shell),
                    }
                }
            }
            "drag-in-drop" => {
                self.show_no_landing(shell);
                let Some(r) = from else { return };
                if self.navis_shown() {
                    if let Some(h) = self.slot_at(shell, editor::pointer_at(choice)).and_then(|(row, _)| head_at(&self.slots, row)) {
                        let slot = self.slots[h];
                        self.put_navi(shell, draft, r, slot);
                    }
                } else if let Some((dst, place, _)) = self.program_landing(shell, draft, choice) {
                    self.install(shell, draft, r, dst, place);
                }
            }
            _ => {}
        }
    }

    fn step(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, by: i32) {
        if id == "lib.sort" {
            let sorts = if self.navis_shown() { NAVI_SORTS.len() } else { PROGRAM_SORTS.len() } as i32;
            let listing = self.listing();
            listing.sort = (listing.sort as i32 + by).rem_euclid(sorts) as usize;
            self.changed(shell, draft);
        }
    }

    fn edit(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, _index: i32, text: &str, _done: bool) {
        if id != "lib.search" {
            return;
        }
        self.listing().search = text.chars().filter(|c| !c.is_control()).take(SEARCH_LENGTH).collect();
        self.changed(shell, draft);
        // (What the search finds is shown from its top.)
        let ui = shell.ui();
        shell.nav.list_to_top(&ui, ROWS);
    }

    /// The pointer on a slot or a row: the strip describes it.
    fn hovered(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32, on: bool) {
        let Some(what) = described(id, index) else { return };
        if editor::hover(&mut self.hovered, what, on) {
            self.show_detail(shell, draft);
        }
    }

    /// The strip follows the cursor; the cursor on a slot marks it, the library beside it of what it takes; in the
    /// library, its title is lit.
    fn focused(&mut self, shell: &mut Shell, draft: &mut Draft, id: &str, index: i32) {
        self.hovered = None;
        self.notice = None;
        let ui = shell.ui();
        ui.global::<PartyView>().set_in_library(id.starts_with("lib."));
        if let Some(what) = described(id, index) {
            self.described = Some(what);
        }
        match id {
            SLOTS if index.max(0) as usize != self.slot => self.select(shell, draft, index.max(0) as usize),
            _ => self.show_detail(shell, draft),
        }
    }

    fn answered(&mut self, shell: &mut Shell, draft: &mut Draft, tag: &str, choice: usize) {
        if editor::cleared(tag, choice)
            && self.change_all(draft, |lead, ps| {
                let had_lead = !std::mem::take(lead).is_empty();
                let had_ps = !std::mem::take(ps).is_empty();
                had_lead || had_ps
            })
        {
            self.slot = 0;
            self.changed(shell, draft);
        }
    }

    /// X, CLEAR.
    fn key(&mut self, shell: &mut Shell, draft: &mut Draft, key: &crate::input::KeyPress) -> bool {
        if editor::clear_key(shell, key) {
            self.activate(shell, draft, "clear", 0);
            return true;
        }
        false
    }

    /// Delete takes out what is under the cursor: a program, a partner.
    fn action(&mut self, shell: &mut Shell, draft: &mut Draft, action: Action) -> bool {
        if action != Action::Delete {
            return false;
        }
        let Some(key) = shell.nav.focused() else { return false };
        match (key.id.as_str(), self.slots.get(key.index.max(0) as usize).copied()) {
            (SLOTS, Some(Slot::LeadProgram(k))) => self.uninstall(shell, draft, Owner::Lead, k),
            (SLOTS, Some(Slot::Program(i, k))) => self.uninstall(shell, draft, Owner::Partner(i), k),
            (SLOTS, Some(Slot::Head(i))) => self.remove_partner(shell, draft, i),
            _ => shell.sound.play(Cue::Refused),
        }
        true
    }

    /// Back from the library to the slot (a phone's sheet shuts).
    fn back(&mut self, shell: &mut Shell, _draft: &mut Draft) -> bool {
        let ui = shell.ui();
        if ui.global::<PartyView>().get_sheet() {
            self.close_sheet(shell);
            shell.nav.focus(SLOTS, self.slot as i32);
            return true;
        }
        if shell.nav.focused().is_some_and(|k| k.id.starts_with("lib.")) {
            shell.nav.focus(SLOTS, self.slot as i32);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_match::testing::exe5_content;

    /// PARTY's slots: each partner's head, its programs, its Empty row while its list has room; an Empty head for each
    /// place left.
    #[test]
    fn the_slots_are_heads_programs_and_empties() {
        use Slot::*;
        assert_eq!(slots_of(None, &[], 2, 10), vec![Open(0), Open(1)]);
        assert_eq!(slots_of(None, &[2], 2, 10), vec![Head(0), Program(0, 0), Program(0, 1), End(0), Open(1)]);
        assert_eq!(slots_of(None, &[0, 1], 2, 1), vec![Head(0), End(0), Head(1), Program(1, 0)]);
        assert_eq!(slots_of(Some(1), &[], 2, 10), vec![LeadHead, LeadProgram(0), LeadEnd, Open(0), Open(1)]);
    }

    /// A program let go over the slots goes into the owner the row is of, where the pointer is: over its head, its
    /// top; over a program, before or after it; over its Empty row, its end; over an Empty head, nowhere. Its gap shows
    /// at the row it goes before.
    #[test]
    fn a_program_lands_where_the_pointer_is() {
        let slots = slots_of(None, &[2, 1], 2, 10);
        let counts = [2, 1];
        // Head(0) Program(0,0) Program(0,1) End(0) Head(1) Program(1,0) End(1)
        assert_eq!(program_place(&slots, 0, false, 0, &counts), Some((Owner::Partner(0), 0)));
        assert_eq!(program_place(&slots, 1, true, 0, &counts), Some((Owner::Partner(0), 0)));
        assert_eq!(program_place(&slots, 1, false, 0, &counts), Some((Owner::Partner(0), 1)));
        assert_eq!(program_place(&slots, 3, true, 0, &counts), Some((Owner::Partner(0), 2)));
        assert_eq!(program_place(&slots, 4, true, 0, &counts), Some((Owner::Partner(1), 0)));
        assert_eq!(program_place(&slots, 6, false, 0, &counts), Some((Owner::Partner(1), 1)));
        assert_eq!(program_place(&slots, 7, false, 0, &counts), None);
        assert_eq!(row_of_place(&slots, Owner::Partner(0), 1), Some(2));
        assert_eq!(row_of_place(&slots, Owner::Partner(0), 2), Some(3));
        assert_eq!(row_of_place(&slots, Owner::Partner(1), 0), Some(5));
        // (A list full, no Empty row: the gap after its last program.)
        let full = slots_of(None, &[1], 2, 1);
        assert_eq!(row_of_place(&full, Owner::Partner(0), 1), Some(2));
        // A navi let go over a partner's rows goes in at its head; over an Empty head, there; over LeadHead, nowhere.
        let open = slots_of(Some(1), &[1], 2, 10);
        assert_eq!(head_at(&open, 0), None);
        assert_eq!(head_at(&open, 1), None);
        assert_eq!((head_at(&open, 5), head_at(&open, 6)), (Some(3), Some(6)));
    }

    /// EXE5's party read from its schema (two partners, each a navi and ten programs of PartyCust's), and a party
    /// written and read back; a partner's list by its name.
    #[test]
    fn the_party_is_the_schemas() {
        let content = exe5_content();
        let shape = Shape::of(&content).unwrap();
        assert_eq!((shape.room, shape.navi.as_str(), shape.programs.as_str(), shape.collection.as_str(), shape.program_room), (2, "navi", "programs", "partycust_programs", 10));
        let builds = saito_match::build::Builds::of(&content, "exe5").unwrap();
        let mut side = builds.new_build(&content, "protoman").unwrap();
        assert_eq!(partners(&content, &side, &shape), Vec::new());
        let navi = |n: &str| Some(saito_match::ids::navi(&content, "exe5", n).unwrap());
        let program = |n: &str| saito_match::ids::entry(&content, "exe5", "partycust_programs", n).unwrap();
        let party = vec![Partner { navi: navi("toadman"), programs: vec![program("p-spport"), program("p-spport")] }, Partner { navi: navi("protoman"), programs: Vec::new() }];
        assert!(write(&content, &mut side, &shape, &party));
        assert_eq!(partners(&content, &side, &shape), party);
        assert_eq!(tone(&content, program("p-spport")), slint::Color::from_rgb_u8(0xEA, 0xF3, 0xFF));
    }
}
