//! Matches of each game (docs/frontend.md §6), each on its game's content
//! alone (`testing::exe5_content`, `exe6_content`): a match is of one game,
//! its own, and names nothing of another's. EXE5's folder rules on
//! EXE5's sides (its rules' rules/folder,
//! content/exe5/rules/folder), an EXE5 match played a few hundred ticks,
//! EXE5's karma and souls, and a name another game has but the match's
//! hasn't refused as any unknown name is.

use crate::facts::Stated;
use crate::testing::{exe4_content, exe5_content, exe6_content};
use crate::{Match, check_match};
use saito_battle::content::Content;
use saito_battle::rules::Fact;
use saito_content_api::{FormHandle, Registry, Value};
use std::sync::Arc;

mod custom_shortcuts;
mod glitch_fixes;
mod take_backs;

/// A number, a flag and forms of `game` by name, as a side's fact takes
/// them.
fn number(n: i64) -> [Fact<'static>; 1] {
    [Fact::Value(Value::Int(n))]
}

fn flag(on: bool) -> [Fact<'static>; 1] {
    [Fact::Value(Value::Bool(on))]
}

fn forms(content: &Content, game: &str, names: &[&str]) -> Vec<Fact<'static>> {
    names.iter().map(|n| Fact::Value(Value::Def(Registry::Form, crate::ids::form(content, game, n).unwrap().0))).collect()
}

/// EXE5's souls, of both versions: what a side that says nothing has, in
/// the forms' order.
const EXE5_SOULS: [&str; 12] = [
    "colonelsoul",
    "gyrosoul",
    "knightsoul",
    "magnetsoul",
    "meddysoul",
    "napalmsoul",
    "numbersoul",
    "protosoul",
    "searchsoul",
    "shadowsoul",
    "toadsoul",
    "tomahawksoul",
];

/// That two sides' facts are the same but a side's own navi and folder
/// (its navi, its folder, its Regular and tag chips).
fn same_but_navi_and_folder(content: &Content, a: &crate::Facts, b: &crate::Facts) {
    for f in crate::facts::fields(content) {
        if !["navi", "folder", "regular_chip", "tag_chips"].contains(&f.name) {
            assert_eq!(a.get(content, f.name), b.get(content, f.name), "{}", f.name);
        }
    }
}

/// A match file's text of `game`: three rounds on its first link battle
/// stage, and the sides `left` and `right` (each a side's table body).
fn match_text(content: &Content, game: &str, left: &str, right: &str) -> String {
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(content, game)[0]).key);
    let round = format!("[[round]]\nstage = \"{stage}\"\n\n");
    format!("game = \"{game}\"\n\n{}[left]\n{left}\n\n[right]\n{right}\n", round.repeat(3))
}

/// A side's table body: its navi, the folder's chips (each "name code"),
/// and `more` lines (`regular_chip = 3`).
fn side(navi: &str, chips: &[&str], more: &str) -> String {
    let chips: Vec<String> = chips
        .iter()
        .map(|c| {
            let (name, code) = c.rsplit_once(' ').unwrap();
            format!("{{ chip = \"{name}\", code = \"{code}\" }}")
        })
        .collect();
    format!("navi = \"{navi}\"\nfolder = [{}]\n{more}", chips.join(", "))
}

/// The match of `game` these sides make, or every problem its checks find.
fn parse_in(content: &Arc<Content>, game: &str, left: &str, right: &str) -> Result<Match, Vec<String>> {
    crate::parse(content, &match_text(content, game, left, right))
}

/// An EXE5 match of these sides.
fn parse(content: &Arc<Content>, left: &str, right: &str) -> Result<Match, Vec<String>> {
    parse_in(content, "exe5", left, right)
}

/// A folder of reference EXE5 saves (their first): Standard chips only, four
/// copies at most.
const TANGO_EXE5: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "cannon B", "airshot *", "airshot *", "airshot *",
    "vulcan1 D", "vulcan1 D", "vulcan1 D", "minibomb B", "minibomb B", "minibomb L", "minibomb L",
    "sword S", "sword S", "sword S", "sword S", "wideswrd S", "wideswrd S", "busterup *",
    "crakout *", "crakout *", "recov10 A", "recov10 A", "recov10 L", "recov10 L", "areagrab S",
    "attck-10 *", "attck-10 *",
];

/// An EXE6 folder: the live navi's kind of folder, Standard chips.
const EXE6: [&str; 30] = [
    "cannon A", "cannon A", "cannon B", "airshot *", "airshot *", "vulcan1 D", "vulcan1 D",
    "minibomb B", "minibomb B", "minibomb L", "sword S", "sword S", "sword S", "wideswrd S",
    "wideswrd S", "recov10 A", "recov10 A", "recov10 L", "areagrab S", "areagrab S", "spreadr1 L",
    "spreadr1 L", "longswrd S", "longswrd S", "crakshot A", "crakshot A", "widesht P", "widesht P",
    "cannon C", "recov30 L",
];

/// An EXE5 MegaMan with this folder.
fn exe5(chips: &[&str], more: &str) -> String {
    side("megaman", chips, more)
}

/// `chips` with entries `at` replaced by `with`.
fn with(chips: &[&str], at: usize, with: &[&'static str]) -> Vec<String> {
    let mut v: Vec<String> = chips.iter().map(|s| s.to_string()).collect();
    for (i, c) in with.iter().enumerate() {
        v[at + i] = c.to_string();
    }
    v
}

fn refs(v: &[String]) -> Vec<&str> {
    v.iter().map(String::as_str).collect()
}

/// EXE5's MegaMan, by EXE5's rules, with reference EXE5 folder: the
/// rules accept it (the Regular chip too).
#[test]
fn a_exe5_folder_keeps_exe5s_rules() {
    let content = exe5_content();
    let right = exe5(&TANGO_EXE5, "");
    parse(&content, &exe5(&TANGO_EXE5, ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // A Regular chip within the Regular memory (every side's 50 MB: CrakOut's 4).
    parse(&content, &exe5(&TANGO_EXE5, "regular_chip = 21"), &right).unwrap_or_else(|p| panic!("{p:?}"));
    // Three Mega chips, three dark chips (one of each), one Giga chip.
    let megas = with(&TANGO_EXE5, 0, &["blizman-ds B", "cloudmn-ds C", "colonel C", "drksword Z", "darkthnd M", "darkwide T", "bass F"]);
    parse(&content, &exe5(&refs(&megas), ""), &right).unwrap_or_else(|p| panic!("{p:?}"));
}

/// What EXE5's folder rules refuse, each said with its rule.
#[test]
fn exe5s_rules_refuse() {
    let content = exe5_content();
    let right = exe5(&TANGO_EXE5, "");
    let refused = |chips: &[String], more: &str| -> Vec<String> {
        let e = parse(&content, &exe5(&refs(chips), more), &right).expect_err("refused");
        assert!(e.iter().all(|p| p.starts_with("left: folder: ")), "{e:?}");
        e
    };
    let says = |e: &[String], what: &str| assert!(e.iter().any(|p| p.contains(what)), "{what:?} not in {e:?}");
    // A fifth copy of a Standard chip (EXE6 would take five 8 MB Cannons).
    let five = with(&TANGO_EXE5, 4, &["cannon C"]);
    says(&refused(&five, ""), "5 copies of cannon, past 4");
    // Two copies of a Mega chip, of a Giga chip, of a dark chip.
    let two = with(&TANGO_EXE5, 0, &["colonel C", "colonel C", "bass F", "bass F", "drksword Z", "drksword Z"]);
    let e = refused(&two, "");
    says(&e, "2 copies of colonel, past 1");
    says(&e, "2 copies of bass, past 1");
    says(&e, "2 copies of drksword, past 1");
    says(&e, "2 Giga chips, past the navi's 1");
    // Six Mega chips; four dark chips.
    let six = with(&TANGO_EXE5, 0, &["colonel C", "meddy M", "gyroman G", "knightmn K", "larkman S", "gridman F"]);
    says(&refused(&six, ""), "6 Mega chips, past the navi's 5");
    let four = with(&TANGO_EXE5, 0, &["drksword Z", "darkthnd M", "darkwide T", "drklance W"]);
    says(&refused(&four, ""), "4 dark chips, past 3");
    // A chip past the pack (BatCan1), a Program Advance; a code the chip
    // doesn't come in.
    let pack = with(&TANGO_EXE5, 0, &["batcan1 *", "lifesrd *"]);
    let e = refused(&pack, "");
    says(&e, "entry 0: batcan1 is no chip a folder can hold");
    says(&e, "entry 1: lifesrd is no chip a folder can hold");
    let code = with(&TANGO_EXE5, 0, &["cannon Z"]);
    says(&refused(&code, ""), "entry 0: cannon doesn't come in code Z");
    // A Regular chip past the Regular memory (Astroid3 is 56 MB; every
    // side's memory is 50); tag chips, which EXE5's folders haven't.
    says(&refused(&with(&TANGO_EXE5, 0, &["astroid3 C"]), "regular_chip = 0"), "the Regular chip astroid3 is 56 MB, past the navi's 50");
    let e = parse(&content, &exe5(&TANGO_EXE5, "tag_chips = [4, 5]"), &right).expect_err("refused");
    says(&e, "left: no field \"tag_chips\" (a side of exe5 takes");
}

/// A name the match's game hasn't is refused as any unknown name is, the
/// same whether another game has it or nothing does: an EXE6 Cross, navi
/// or chip in an EXE5 match, an EXE5 chip in an EXE6 match, a name written with
/// a game (`exe6:cannon`: no name is written so) and a misspelling alike.
#[test]
fn an_unknown_name_is_refused() {
    let content = exe5_content();
    let says = |e: Vec<String>, what: &str| assert!(e.iter().any(|p| p.starts_with(what)), "{what:?} not in {e:?}");
    let ok = exe5(&TANGO_EXE5, "");
    // An EXE6 navi, Cross (as a soul: EXE5's rules take no Cross list at
    // all) and NaviCust program: none of EXE5's.
    // (EXE5 has a ProtoMan of its own: HeatMan is EXE6's alone.)
    says(parse(&content, &side("heatman", &TANGO_EXE5, ""), &ok).unwrap_err(), "left: navi: no navi \"heatman\" in exe5");
    let crosses = exe5(&TANGO_EXE5, "souls = [\"heatcross\"]");
    says(parse(&content, &crosses, &ok).unwrap_err(), "left: souls: [1]: no form \"heatcross\" in exe5");
    let crosses = exe5(&TANGO_EXE5, "crosses = [\"heatcross\"]");
    says(parse(&content, &crosses, &ok).unwrap_err(), "left: no field \"crosses\" (a side of exe5 takes auto_battle_places, auto_battle_records, chaos_unison,");
    // A chip of EXE6's alone (HeatMan), a qualified name, a misspelling:
    // one error.
    let six = exe6_content();
    assert!(crate::ids::chip(&six, "exe6", "heatman").is_some() && crate::ids::chip(&content, "exe5", "heatman").is_none());
    for name in ["heatman", "exe6:cannon", "exe5:cannon", "canon"] { // (written in full)
        let chips = with(&TANGO_EXE5, 0, &[]);
        let mut chips = refs(&chips);
        let entry = format!("{name} A");
        chips[3] = &entry;
        says(parse(&content, &exe5(&chips, ""), &ok).unwrap_err(), &format!("left: folder: [4]: chip: no chip {name:?} in exe5"));
    }
    // An EXE5 chip (GyroMan) in an EXE6 match.
    assert!(crate::ids::chip(&content, "exe5", "gyroman").is_some() && crate::ids::chip(&six, "exe6", "gyroman").is_none());
    let mut chips = EXE6.to_vec();
    chips[0] = "gyroman G";
    let e = parse_in(&six, "exe6", &side("megaman", &chips, ""), &side("megaman", &EXE6, "")).unwrap_err();
    says(e, "left: folder: [1]: chip: no chip \"gyroman\" in exe6");
    // A game the content hasn't (EXE6's file on EXE5's content, a game no
    // content has).
    let text = match_text(&content, "exe5", &ok, &ok);
    let e = crate::parse(&content, &text.replacen("game = \"exe5\"", "game = \"bn7\"", 1)).unwrap_err();
    says(e, "no game \"bn7\" (the content's are exe5)");
    let e = crate::parse(&six, &text).unwrap_err();
    says(e, "no game \"exe5\" (the content's are exe6)");
}

/// A match's lookups see only its game: its stages, navis, chips (the
/// rules' pool), souls and patch cards are its own, so an EXE5 match's
/// sides, made or picked, hold EXE5's alone and an EXE6 match's EXE6's.
#[test]
fn a_matchs_lookups_only_see_its_game() {
    for (game, content) in [("exe5", exe5_content()), ("exe6", exe6_content())] {
        let of = |key: &str| crate::ids::in_game(&content, game, key);
        assert!(crate::link_battle_stages(&content, game).iter().all(|&s| of(&content.defs.stage(s).key)));
        assert!(crate::navis(&content, game).iter().all(|&n| of(&content.defs.navi(n).key)));
        let m = crate::pick::live(&content, game, 5, None).unwrap();
        assert_eq!(check_match(&content, &m), Vec::<String>::new(), "{game}");
        // (The forms its sides' facts hold: a picked Cross list, the souls
        // a side that says nothing has.)
        let forms = |v: crate::facts::Stated| match v {
            crate::facts::Stated::List(items) => items.into_iter().filter_map(|i| if let crate::facts::Stated::Def(saito_content_api::Registry::Form, Some(h)) = i { Some(h) } else { None }).collect(),
            _ => Vec::new(),
        };
        let held: Vec<u16> = crate::facts::fields(&content).iter().flat_map(|f| forms(m.sides[0].facts.get(&content, f.name).unwrap())).collect();
        assert!(!held.is_empty() && held.iter().all(|&f| of(&content.defs.form(FormHandle(f)).key)), "{game}");
        let mut b = crate::check::start(&content, &m).unwrap();
        let pool = crate::folders::pool(&content, game, &mut b, 0);
        assert!(pool.len() > 100 && pool.iter().all(|&c| of(&content.defs.chip(c).key)), "{game}: {} chips", pool.len());
        for s in &m.sides {
            assert!(of(&content.defs.navi(s.navi(&content)).key) && s.folder(&content).chips().all(|c| of(&content.defs.chip(c.id).key)));
        }
        // Written, it names its game once, and nothing else qualified.
        let text = crate::write(&content, &m);
        assert!(text.contains(&format!("game = \"{game}\"")) && !text.contains("exe5:") && !text.contains("exe6:"), "{text}");
        assert_eq!(crate::parse(&content, &text).unwrap(), m);
    }
    // EXE5's souls aren't EXE6's; EXE6's names are none of EXE5's
    // content's.
    assert_eq!(crate::ids::form(&exe6_content(), "exe6", "protosoul"), None);
    assert!(crate::ids::form(&exe5_content(), "exe5", "protosoul").is_some());
    assert_eq!(crate::ids::form(&exe5_content(), "exe6", "heatcross"), None);
    assert_eq!(crate::ids::games(&exe5_content()), ["exe5"]);
    // A match of another game than the content's makes no match.
    assert!(Match::empty(&exe5_content(), "exe6").is_err());
}

/// Play a match `ticks` ticks: the scenario's players (picking chips at
/// each custom screen, stepping about, firing and using them) on the
/// match's round. The chips each side used, by key, in order.
fn play(content: &Arc<Content>, m: &Match, ticks: usize) -> [Vec<String>; 2] {
    assert_eq!(check_match(content, m), Vec::<String>::new());
    let setup = m.round(content, 0x5EED);
    let tape = saito_battle::scenario::record_on_content(setup.clone(), content.clone(), ticks, 11);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    let mut used: [Vec<String>; 2] = Default::default();
    let mut shown = [None, None];
    for t in &tape {
        b.tick(&t.input, t.events.clone());
        for side in 0..2 {
            let now = b.used_chips[side].map(|u| u.chip);
            if now.is_some() && b.used_chips[side].is_some_and(|u| u.ticks > 0) && now != shown[side] {
                used[side].push(content.defs.chip(now.unwrap()).key.clone());
            }
            shown[side] = now;
        }
    }
    used
}

/// An EXE5 match plays 900 ticks (custom screens, chips used) without the
/// engine stopping, its sides using EXE5's chips.
#[test]
fn a_exe5_match_plays() {
    let content = exe5_content();
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let used = play(&content, &m, 900);
    assert!(used.iter().all(|u| !u.is_empty() && u.iter().all(|k| crate::ids::in_game(&content, "exe5", k))), "{used:?}");
}

/// EXE4's karma (content/exe4/rules/light_dark, docs/design/exe4-map.md
/// §18 item 12): a navi whose value is above 499 (a light one's, and the
/// default 500) closes the hole he stands on, each tick of the fight (his
/// update's intake); a dark one's doesn't. (Every panel of side 0's the stage has is a hole, wherever its
/// navi stands.)
#[test]
fn a_exe4_navi_of_light_closes_a_hole() {
    use saito_battle::Battle;
    use saito_battle::battle::battle_flags;
    let content = exe4_content();
    let hole = content.rules().panels.named("hole").expect("EXE4's hole");
    let holes_closed = |karma: Option<i64>| {
        let mut m = crate::pick::live(&content, "exe4", 3, None).unwrap();
        if let Some(k) = karma {
            m.sides[0].set_fact(&content, "karma", &number(k)).unwrap();
        }
        let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
        let navi = (0..600)
            .find_map(|_| {
                b.tick(&Default::default(), Default::default());
                b.player(0).filter(|&r| b.objects.get(r).collision.is_some())
            })
            .expect("side 0's navi, with its collision data");
        let holes = |b: &Battle| b.field.panels.iter().flatten().filter(|p| p.kind == hole).count();
        for y in 1..=3 {
            for x in 1..=3 {
                b.set_panel_type(x, y, hole);
            }
        }
        let before = holes(&b);
        b.round.flags |= battle_flags::FIGHTING;
        saito_battle::kinds::player::update(&mut b, navi);
        before - holes(&b)
    };
    assert_eq!(holes_closed(Some(1000)), 1, "the light navi's");
    assert_eq!(holes_closed(None), 1, "the default, 500");
    assert_eq!(holes_closed(Some(499)), 0, "a dark one's");
}

/// What a match is, in words: each side's navi and the facts it states,
/// each as its name and value; a fact at its rules' default isn't said.
#[test]
fn a_match_is_described_by_its_facts() {
    let six = exe6_content();
    let mut m = crate::pick::live(&six, "exe6", 1, None).unwrap();
    let side = &mut m.sides[0];
    side.set_fact(&six, "cybeast", &[crate::facts::version_fact(&six, "falzar").unwrap()]).unwrap();
    side.set_fact(&six, "crosses", &forms(&six, "exe6", &["heatcross", "spoutcross"])).unwrap();
    side.set_fact(&six, "beast_out", &flag(false)).unwrap();
    side.set_fact(&six, "bug_frags", &number(9)).unwrap();
    let said = crate::describe(&six, &m, 1, false, 0);
    let line = said.lines().nth(1).unwrap();
    assert_eq!(line, "  ロックマン (you); beast_out: no; bug_frags: 9; crosses: ヒートクロス, アクアクロス; cybeast: Fビースト");
    // (No Crosses is the list's default, which goes unsaid.)
    m.sides[0].set_fact(&six, "crosses", &[]).unwrap();
    assert!(crate::describe(&six, &m, 1, false, 0).lines().nth(1).unwrap().starts_with("  ロックマン (you); beast_out: no; bug_frags: 9; cybeast: "));
    let five = exe5_content();
    let mut m = parse(&five, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    assert_eq!(crate::describe(&five, &m, 1, false, 0).lines().nth(1).unwrap(), "  ロックマン (you)");
    m.sides[0].set_fact(&five, "karma", &number(100)).unwrap();
    m.sides[0].set_fact(&five, "souls", &[]).unwrap();
    assert_eq!(crate::describe(&five, &m, 1, false, 0).lines().nth(1).unwrap(), "  ロックマン (you); karma: 100; souls: none");
}

/// What a tool offers for a side's list of definitions: for the engine's
/// form list, the forms of the navi's own lists (EXE6's ten Crosses, in its
/// versions' order; none for a navi without); else the list the navi that
/// changes form holds under the fact's name (EXE5's twelve souls, in the
/// soul button's order), with whatever else the side's list holds.
#[test]
fn a_list_fact_offers_its_definitions() {
    let six = exe6_content();
    let m = crate::pick::live(&six, "exe6", 1, None).unwrap();
    let field = crate::facts::field(&six, "crosses").unwrap();
    let offered = crate::facts::offered(&six, "exe6", &m.sides[0], &field).unwrap();
    let megaman = six.navi(m.sides[0].navi(&six)).forms.as_ref().unwrap();
    let own: Vec<u16> = megaman.listed("gregar").iter().chain(megaman.listed("falzar")).map(|f| f.0).collect();
    assert_eq!((offered.len(), &offered), (10, &own));
    let mut link = m.sides[0].clone();
    link.set_navi(&six, crate::ids::navi(&six, "exe6", "protoman").unwrap()).unwrap();
    assert_eq!(crate::facts::offered(&six, "exe6", &link, &field), Some(Vec::new()));
    // (Flags and single values are no lists of definitions.)
    for name in ["cybeast", "beast_out", "bug_frags"] {
        assert_eq!(crate::facts::offered(&six, "exe6", &m.sides[0], &crate::facts::field(&six, name).unwrap()), None, "{name}");
    }
    let five = exe5_content();
    let mut m = parse(&five, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let field = crate::facts::field(&five, "souls").unwrap();
    let megaman = five.navi(m.sides[0].navi(&five)).forms.as_ref().unwrap();
    let souls: Vec<u16> = megaman.listed("souls").iter().map(|f| f.0).collect();
    assert_eq!(souls.len(), 12);
    assert_eq!(crate::facts::offered(&five, "exe5", &m.sides[0], &field), Some(souls.clone()));
    // (A form the list doesn't hold, MegaMan's base form, stated all the
    // same: offered after.)
    m.sides[0].set_fact(&five, "souls", &forms(&five, "exe5", &["protosoul", "base"])).unwrap();
    let base = crate::ids::form(&five, "exe5", "base").unwrap().0;
    assert_eq!(crate::facts::offered(&five, "exe5", &m.sides[0], &field).unwrap(), [souls, vec![base]].concat());
}

/// An EXE5 side's karma and souls write to a match file and read back; the
/// karma's default (a fresh save's 500) and an unlisted soul list (every
/// soul) are left out, so old files load unchanged.
#[test]
fn karma_and_souls_write_and_read_back() {
    let content = exe5_content();
    let left = exe5(&TANGO_EXE5, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 100\nsouls = [\"protosoul\", \"colonelsoul\"]\n", 1);
    let m = parse(&content, &left, &exe5(&TANGO_EXE5, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let s = &m.sides[0];
    assert_eq!(s.facts.get(&content, "karma"), Some(Stated::Number(100)));
    let souls = ["protosoul", "colonelsoul"].map(|n| crate::ids::form(&content, "exe5", n).unwrap().0);
    assert_eq!(s.facts.get(&content, "souls").unwrap().defs(), souls, "either version's");
    let text = crate::write(&content, &m);
    assert!(text.contains("karma = 100") && text.contains("souls = [") && text.contains("\"colonelsoul\""), "{text}");
    assert_eq!(crate::parse(&content, &text).unwrap(), m);
    // A side that says nothing of them: the defaults, nothing written.
    let plain = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    same_but_navi_and_folder(&content, &plain.sides[0].facts, &crate::Facts::defaults(&content));
    assert_eq!(plain.sides[0].facts.get(&content, "karma"), Some(Stated::Number(500)));
    let all: Vec<u16> = EXE5_SOULS.map(|n| crate::ids::form(&content, "exe5", n).unwrap().0).to_vec();
    assert_eq!(plain.sides[0].facts.get(&content, "souls").unwrap().defs(), all);
    let text = crate::write(&content, &plain);
    assert!(!text.contains("karma") && !text.contains("souls"), "{text}");
    // No souls at all is stated too (an empty list isn't the default).
    let none = exe5(&TANGO_EXE5, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nsouls = []\n", 1);
    let m = parse(&content, &none, &exe5(&TANGO_EXE5, "")).unwrap();
    assert!(m.sides[0].facts.get(&content, "souls").unwrap().defs().is_empty());
    let text = crate::write(&content, &m);
    assert!(text.contains("souls = []"), "{text}");
    assert_eq!(crate::parse(&content, &text).unwrap(), m);
    // Karma and a soul list under EXE6's rules, which take neither: the
    // facts EXE6 takes are said. (An EXE6 Cross is none of EXE5's forms.)
    let bad = side("megaman", &EXE6, "").replacen("navi = \"megaman\"\n", "navi = \"megaman\"\nkarma = 1200\nsouls = [\"heatcross\"]\n", 1);
    let six = exe6_content();
    let e = parse_in(&six, "exe6", &side("megaman", &EXE6, ""), &bad).unwrap_err();
    let takes = "(a side of exe6 takes beast_out, bug_frags, crosses, cybeast, folder, hp, level, navi, navicust_expansions, navicust_programs, patch_cards, reg_up, regular_chip, sp_times, sun, tag_chips)";
    for p in [format!("right: no field \"karma\" {takes}"), format!("right: no field \"souls\" {takes}")] {
        assert!(e.iter().any(|x| *x == p), "{p:?} not in {e:?}");
    }
    let bad = exe5(&TANGO_EXE5, "souls = [\"heatcross\"]");
    let e = parse(&content, &bad, &exe5(&TANGO_EXE5, "")).unwrap_err();
    assert!(e.iter().any(|p| p.starts_with("left: souls: [1]: no form \"heatcross\" in exe5")), "{e:?}");
    // A value past its field's type, a soul twice, more souls than the list
    // holds.
    for (stated, said) in [
        ("karma = 70000", "left: karma: 70000 is past a u16 (0 to 65535)"),
        ("souls = [\"protosoul\", \"protosoul\"]", "left: souls: ブルースソウル is there twice"),
        ("chaos_unison = \"no\"", "left: chaos_unison: \"no\" is neither true nor false"),
    ] {
        let e = parse(&content, &exe5(&TANGO_EXE5, stated), &exe5(&TANGO_EXE5, "")).unwrap_err();
        assert!(e.contains(&said.to_string()), "{said:?} not in {e:?}");
    }
}

/// What a setup that says nothing has is the rules' own to state (their
/// systems' `setup_defaults`), and a match writes none of it: EXE6's, Beast
/// Out (its Cybeast and its Crosses have no default: a setup that says
/// nothing states neither); EXE5's, every soul of either version, Chaos
/// Unison and a fresh save's karma.
#[test]
fn a_setup_that_says_nothing_has_the_rules_defaults() {
    use saito_battle::custom::PlayerSetup;
    use saito_content_api::{FieldValue, Registry};
    let nothing = PlayerSetup::default();
    let six = exe6_content();
    let (schema, block) = nothing.rules_block(&six).expect("EXE6's rules");
    assert_eq!(block.get(schema, schema.index_of("cybeast").unwrap()), FieldValue::Ref(None), "no Cybeast");
    // (A Cross list left out is empty: no Crosses.)
    let crosses = schema.index_of("crosses").unwrap();
    assert!(block.stated(schema, crosses) && (0..5).all(|k| block.get_elem(schema, crosses, k) == Some(FieldValue::Ref(None))));
    assert_eq!(block.get(schema, schema.index_of("beast_out").unwrap()), FieldValue::Bool(true));
    let five = exe5_content();
    let (schema, block) = nothing.rules_block(&five).expect("EXE5's rules");
    assert!(schema.index_of("version").is_none(), "no version");
    let souls = schema.index_of("souls").unwrap();
    let listed: Vec<_> = (0..12).filter_map(|k| block.get_elem(schema, souls, k)).take_while(|v| *v != FieldValue::Ref(None)).collect();
    let all: Vec<_> = EXE5_SOULS.iter().map(|n| FieldValue::Ref(Some((Registry::Form, crate::ids::form(&five, "exe5", n).unwrap().0)))).collect();
    assert_eq!(listed, all, "every soul the game has, in the forms' order");
    assert_eq!(all.len(), 12);
    assert_eq!(block.get(schema, schema.index_of("chaos_unison").unwrap()), FieldValue::Bool(true));
    // A side that says nothing of them plays with exactly that.
    let plain = parse(&five, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let setup = plain.round(&five, 3);
    let (_, played) = setup.players[0].rules_block(&five).unwrap();
    same_but_navi_and_folder(&five, &crate::facts::Facts::of_block(played), &crate::facts::Facts::of_block(block));
}

/// A round of `m` after `ticks` ticks of nothing pressed.
fn started(content: &Arc<Content>, m: &Match, ticks: usize) -> saito_battle::Battle {
    let mut b = saito_battle::Battle::new(m.round(content, 0x5EED), content.clone());
    for _ in 0..ticks {
        b.tick(&Default::default(), Default::default());
    }
    b
}

/// A dark MegaMan (light/dark value 100) starts with mood 0 and the dark
/// face (the evil one: `megaman-dark`); a fresh save's (500)
/// with mood 128 and his plain face; a light one (1000) at 190.
#[test]
fn a_dark_side_starts_dark() {
    let content = exe5_content();
    let at = |value: Option<u16>| {
        let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        if let Some(v) = value {
            m.sides[0].set_fact(&content, "karma", &number(v as i64)).unwrap();
        }
        let b = started(&content, &m, 40);
        (b.stats[0].mood, saito_battle::kinds::player::emotion_name(&b, 0).to_string())
    };
    assert_eq!(at(Some(100)), (0, "evil".to_string()));
    assert_eq!(at(None), ((500 / 20 + 103) as u8, "normal".to_string()));
    assert_eq!(at(Some(1000)), (190, "normal".to_string()));
    // The face a mood of 0 shows: the base form's dark one, plus ProtoMan (1) and Colonel (2) Hub Style faces.
    let base = content.base_form_for(crate::ids::navi(&content, "exe5", "megaman").unwrap());
    let emotions = &content.rules().emotion;
    let mugshot = content.form(base).mugshot.as_ref().unwrap();
    let face = mugshot.shown(emotions, emotions.by_name("evil").unwrap(), 0);
    assert_eq!(content.assets.handle(saito_content_api::AssetKind::Mugshot, "megaman-dark"), Some(face.0));
    let hub_proto = mugshot.shown(emotions, emotions.by_name("normal").unwrap(), 1);
    assert_eq!(content.assets.handle(saito_content_api::AssetKind::Mugshot, "megaman-hub"), Some(hub_proto.0));
    let hub_colonel = mugshot.shown(emotions, emotions.by_name("normal").unwrap(), 2);
    assert_eq!(content.assets.handle(saito_content_api::AssetKind::Mugshot, "megaman-hub-colonel"), Some(hub_colonel.0));
    // Equipping Bass-Cross MegaMan (protoman vs colonel) sets face_variant 1 vs 2 on battle start (S-110).
    let card = crate::ids::entry(&content, "exe5", "patch_cards", "bass-cross-megaman").unwrap();
    let builds = crate::build::Builds::of(&content, "exe5").unwrap();
    for (variant, expected) in [("protoman", 1u8), ("colonel", 2u8)] {
        let mut side = builds.new_build(&content, variant).unwrap();
        side.set_fact(
            &content,
            "patch_cards",
            &[Fact::Record(vec![
                ("card", Fact::Value(Value::Def(Registry::Entry, card.0))),
                ("variant", Fact::Name(variant)),
            ])],
        )
        .unwrap();
        let face = crate::build::Checked::of(&content, "exe5", &side).face();
        assert_eq!(face, Some(crate::build::Face::Form(base, "normal".into(), expected)), "variant {variant}");
    }
}

/// The soul button (EXE5's rules/souls's) offers only a soul the side has:
/// with every soul (ProtoSoul among them), a Sword picked offers ProtoSoul;
/// with GyroSoul alone, the button is there but the sword's soul isn't
/// offered. With no souls (a save without Soul Unison) there is no button.
#[test]
fn an_unowned_soul_cant_be_chosen() {
    use saito_battle::custom::screen::{Phase, SPECIAL_SLOT, SlotKind, SlotState};
    let content = exe5_content();
    // Swords alone, so the first chip dealt is one (no folder the rules
    // take: the round is played as set up).
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let offered = |souls: Option<&[&str]>| {
        let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        { let mut f = m.sides[0].folder(&content); f.chips = [Some(saito_battle::custom::FolderChip::new(sword, saito_battle::content::ChipCode(18))); 30]; m.sides[0].set_folder(&content, &f).unwrap(); }
        if let Some(names) = souls {
            m.sides[0].set_fact(&content, "souls", &forms(&content, "exe5", names)).unwrap();
        }
        let mut b = saito_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
        let mut last = None;
        for _ in 0..400 {
            let screen = b.custom.sides[0].screen.as_ref().filter(|s| s.phase == Phase::Choosing);
            if let Some(s) = screen
                && s.selected == 1
            {
                let slot = s.slots[SPECIAL_SLOT as usize];
                let soul = matches!(slot.kind, SlotKind::Button { button, .. } if content.defs.button(button).name == "soul");
                return (soul, soul && slot.state == SlotState::Selectable);
            }
            let a = (screen.is_some() && last.is_none()).then_some(saito_battle::ScreenChoice::Pick(0));
            last = a;
            b.tick(&[saito_battle::input::PlayerTick::default().with_choice(a), Default::default()], Default::default());
        }
        panic!("no chip picked");
    };
    assert_eq!(offered(None), (true, true));
    assert_eq!(offered(Some(&[])), (false, false), "no souls, no button");
    assert_eq!(offered(Some(&["gyrosoul"])), (true, false));
    // ProtoSoul alone, or with Team Colonel's ColonelSoul: offered.
    assert_eq!(offered(Some(&["protosoul", "colonelsoul"])), (true, true));
}

/// The soul given for a chip (rules/souls's button and window): the
/// soul takes the Sword's place, first in the selection; B puts the Sword
/// back and offers the soul again; given again and OK, the turn's form is
/// ProtoSoul for 3 turns and the Sword leaves the folder in its place.
#[test]
fn the_soul_takes_the_chips_place() {
    use saito_battle::custom::screen::{Phase, SPECIAL_SLOT, SlotKind, SlotState};
    use saito_battle::input::PlayerTick;
    let content = exe5_content();
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let proto = crate::ids::form(&content, "exe5", "protosoul").unwrap();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    { let mut f = m.sides[0].folder(&content); f.chips = [Some(saito_battle::custom::FolderChip::new(sword, saito_battle::content::ChipCode(18))); 30]; m.sides[0].set_folder(&content, &f).unwrap(); }
    let mut b = saito_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
    let screen = |b: &saito_battle::Battle| b.custom.sides[0].screen.expect("a screen");
    // A tick with `key` pressed, then ticks without until the screen is
    // back to choosing.
    let press = |b: &mut saito_battle::Battle, choice: saito_battle::ScreenChoice| {
        b.tick(&[PlayerTick::default().with_choice(Some(choice)), Default::default()], Default::default());
        for _ in 0..200 {
            b.tick(&[PlayerTick::buttons(0), Default::default()], Default::default());
            if b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing) {
                return;
            }
        }
        panic!("the screen stays out of choosing");
    };
    for _ in 0..400 {
        if b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing) {
            break;
        }
        b.tick(&[PlayerTick::buttons(0), Default::default()], Default::default());
    }
    // The first Sword picked: the soul is on offer.
    press(&mut b, saito_battle::ScreenChoice::Pick(0));
    let s = screen(&b);
    assert_eq!((s.selection(), s.slots[SPECIAL_SLOT as usize].state), (&[0u8][..], SlotState::Selectable));
    assert!(matches!(s.slots[SPECIAL_SLOT as usize].kind, SlotKind::Button { .. }));
    // Given: the soul first in the selection, the Sword's slot still picked.
    press(&mut b, saito_battle::ScreenChoice::Press(SPECIAL_SLOT));
    let s = screen(&b);
    assert_eq!(s.selection(), &[SPECIAL_SLOT][..]);
    assert_eq!((s.slots[0].state, s.slots[SPECIAL_SLOT as usize].state), (SlotState::Selected, SlotState::Selected));
    assert_eq!(s.trade.map(|t| (t.button, t.chip)), Some((SPECIAL_SLOT, 0)));
    // B: the Sword back in its place, the soul on offer again.
    press(&mut b, saito_battle::ScreenChoice::TakeBack);
    let s = screen(&b);
    assert_eq!((s.selection(), s.trade), (&[0u8][..], None));
    assert_eq!(s.slots[SPECIAL_SLOT as usize].state, SlotState::Selectable);
    // Given again, and OK: the turn's form, its turns, the Sword gone.
    press(&mut b, saito_battle::ScreenChoice::Press(SPECIAL_SLOT));
    b.tick(&[PlayerTick::default().with_choice(Some(saito_battle::ScreenChoice::Ok)), Default::default()], Default::default());
    let (hand, transform) = b.custom.sides[0].built.clone().expect("OK built the hand");
    assert_eq!((transform.form, transform.turns, transform.alternate), (Some(proto), 3, false));
    assert!(hand.is_some(), "the soul is a pick");
    let folder = b.custom.sides[0].folder;
    assert_eq!(folder.chips[0], None, "the Sword given up left the folder");
    assert_eq!(folder.count(), 29);
}

/// What a soul keeps of a custom screen in the rules' state (its
/// form's `custom.state`) is as a fresh state has it when the next screen
/// deals, whatever soul the navi is in then: the original zeroes the
/// screen's record as it opens (0x08022CA2). MegaMan in MeddySoul is dealt
/// two capsules and mixes the first into a Sword; his soul has no turns
/// left, so the turn's start takes him back to his base form, and the next
/// screen's deal, out of MeddySoul, leaves no capsule and no mix.
#[test]
fn what_a_soul_keeps_of_a_screen_is_fresh_at_the_next_deal() {
    use saito_battle::Battle;
    use saito_battle::custom::screen::{Phase, SlotKind};
    use saito_battle::input::{PlayerTick, keys};
    use saito_content_api::{FieldValue, Registry};
    let content = exe5_content();
    let sword = crate::ids::chip(&content, "exe5", "sword").unwrap();
    let meddy = crate::ids::form(&content, "exe5", "meddysoul").unwrap();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    { let mut f = m.sides[0].folder(&content); f.chips = [Some(saito_battle::custom::FolderChip::new(sword, saito_battle::content::ChipCode(18))); 30]; m.sides[0].set_folder(&content, &f).unwrap(); }
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    let base = b.stats[0].form;
    // (In MeddySoul from the start, his stats' starting form: the souls
    // part's state is a fresh one, with no turns of the soul.)
    b.stats[0].starting_form = meddy;
    // (A field by its name, wherever the rules' state keeps it: the souls'
    // screen fields are its `screen` record's.)
    let field = |b: &Battle, name: &str| {
        let (schema, state) = b.rules_state(0).expect("EXE5's rules");
        let path = schema.find(name).unwrap().unwrap_or_else(|| panic!("the rules keep no `{name}`"));
        state.get_at(schema.place_of(path))
    };
    let choosing = |b: &Battle, side: usize| {
        let s = &b.custom.sides[side];
        s.in_custom && s.screen.is_some_and(|s| s.phase == Phase::Choosing)
    };
    let tick = |b: &mut Battle, left: u16, right: u16| b.tick(&[PlayerTick::buttons(left), PlayerTick::buttons(right)], Default::default());
    // A tick with the left side's choice, then ticks without until its
    // screen is back to choosing.
    let press = |b: &mut Battle, choice: saito_battle::ScreenChoice| {
        b.tick(&[PlayerTick::default().with_choice(Some(choice)), Default::default()], Default::default());
        for _ in 0..200 {
            tick(b, 0, 0);
            if choosing(b, 0) {
                return;
            }
        }
        panic!("the screen stays out of choosing");
    };
    for _ in 0..400 {
        if choosing(&b, 0) && choosing(&b, 1) {
            break;
        }
        tick(&mut b, 0, 0);
    }
    // The first screen, in MeddySoul: two capsules, in the hand's last two
    // slots.
    assert!(choosing(&b, 0) && choosing(&b, 1));
    assert_eq!(b.stats[0].form, meddy);
    let a_capsule = |v: FieldValue| matches!(v, FieldValue::Ref(Some((Registry::Chip, _))));
    assert!(a_capsule(field(&b, "capsule_1")) && a_capsule(field(&b, "capsule_2")), "MeddySoul's deal picks two capsules");
    let screen = b.custom.sides[0].screen.unwrap();
    for slot in [8, 9] {
        assert!(matches!(screen.slots[slot].kind, SlotKind::Button { button, .. } if content.defs.button(button).name.starts_with("capsule_")));
    }
    // A Sword picked, and the first capsule mixed into it: its sequence's
    // end is in the state.
    press(&mut b, saito_battle::ScreenChoice::Pick(0));
    press(&mut b, saito_battle::ScreenChoice::Press(8));
    assert_eq!((field(&b, "mix_capsule"), field(&b, "mix_step")), (FieldValue::U8(1), FieldValue::U8(24)));
    assert!(b.custom.sides[0].screen.unwrap().slots[0].attached.is_some(), "the capsule is in the Sword");
    // OK on both screens, and the fight: the turn's start reverts a soul
    // with no turns left.
    let ok = PlayerTick::default().with_choice(Some(saito_battle::ScreenChoice::Ok));
    b.tick(&[ok, ok], Default::default());
    for _ in 0..600 {
        if !b.custom.sides[0].in_custom && !b.custom.sides[1].in_custom {
            break;
        }
        tick(&mut b, 0, 0);
    }
    assert!(!b.custom.sides[0].in_custom, "the fight goes on");
    // L each other tick until the gauge is full and the next screen opens.
    for t in 0..6000 {
        if choosing(&b, 0) {
            break;
        }
        tick(&mut b, if t % 2 == 0 { keys::L } else { 0 }, 0);
    }
    assert!(choosing(&b, 0), "the second screen");
    assert_eq!(b.stats[0].form, base, "out of MeddySoul");
    // Its deal left what MeddySoul kept of the first screen as a fresh
    // state has it.
    for name in ["capsule_1", "capsule_2"] {
        assert_eq!(field(&b, name), FieldValue::Ref(None), "{name}");
    }
    for name in ["mix_capsule", "mix_step", "mix_count", "arm_step", "arm_timer"] {
        assert_eq!(field(&b, name), FieldValue::U8(0), "{name}");
    }
    let screen = b.custom.sides[0].screen.unwrap();
    assert!(!matches!(screen.slots[8].kind, SlotKind::Button { .. }), "no capsule's slot out of MeddySoul");
}

/// A side's karma and souls go into its round's setup: the rules'
/// setup holds the karma, the souls (which the
/// soul button offers) and Chaos Unison (on unless the side says).
#[test]
fn karma_and_souls_reach_the_round() {
    let content = exe5_content();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    m.sides[0].set_fact(&content, "karma", &number(300)).unwrap();
    let colonel = crate::ids::form(&content, "exe5", "colonelsoul").unwrap();
    m.sides[0].set_fact(&content, "souls", &forms(&content, "exe5", &["colonelsoul"])).unwrap();
    m.sides[0].set_fact(&content, "chaos_unison", &flag(false)).unwrap();
    let b = started(&content, &m, 1);
    let (schema, block) = b.rules_setup(0).unwrap();
    assert_eq!(block.get(schema, schema.index_of("karma").unwrap()), saito_content_api::FieldValue::U16(300));
    let field = |name: &str| block.get(schema, schema.index_of(name).unwrap());
    let soul = |k: usize| block.get_elem(schema, schema.index_of("souls").unwrap(), k);
    use saito_content_api::{FieldValue, Registry};
    assert_eq!(soul(0), Some(FieldValue::Ref(Some((Registry::Form, colonel.0)))));
    assert_eq!(soul(1), Some(FieldValue::Ref(None)));
    assert_eq!(field("chaos_unison"), FieldValue::Bool(false));
    // An EXE6 match's sides have no souls.
    let six_content = exe6_content();
    let six = crate::pick::live(&six_content, "exe6", 1, None).unwrap();
    let b = started(&six_content, &six, 1);
    let (schema, _) = b.rules_setup(1).unwrap();
    assert!(schema.index_of("souls").is_none());
}

/// What the match's rules and a side's navi take decide the side's own
/// fields: an EXE6 match's side takes its version (its Cybeast), a navi
/// code's level and EXE6's SP times; an EXE5 match's no version, no level,
/// and EXE5's own SP times (its SP navi chips', not EXE6's).
#[test]
fn a_sides_fields_are_its_rules() {
    let (c5, c6) = (exe5_content(), exe6_content());
    let six = crate::pick::live(&c6, "exe6", 3, None).unwrap();
    let five = parse(&c5, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let sp_times = |c: &saito_battle::Content| crate::facts::field(c, "sp_times").is_some();
    assert!(crate::Side::takes_version(&c6) && six.sides[0].takes_level(&c6) && sp_times(&c6));
    assert!(!crate::Side::takes_version(&c5) && !five.sides[0].takes_level(&c5) && sp_times(&c5));
    // A side that says nothing has every SP chip of its game deleted in no
    // time.
    let times = |c: &saito_battle::Content, m: &Match| -> Vec<(String, u16)> { crate::testing::sp_times(c, &m.sides[0].facts) };
    let (t6, t5) = (times(&c6, &six), times(&c5, &five));
    assert_eq!((t6.len(), &t6[0]), (18, &("heatman-sp".to_string(), 0)), "{t6:?}");
    assert_eq!((t5.len(), &t5[0]), (18, &("protomn-sp".to_string(), 0)), "{t5:?}");
}

/// EXE5's rules send both sides' auto battle data as the round is set up
/// (content/exe5/rules/auto_battle/block.luau, 0x0802C7BE): from the
/// battle's RNG, two numbers a round, 84 in all, the same order for both
/// sides' places (each console shuffles its own from the RNG2 they share);
/// a 0 is sent as an entry, an empty place is packed away; the first three
/// places are shuffled among themselves.
#[test]
fn exe5s_rules_send_the_auto_battle_data() {
    use saito_battle::rules::Fact;
    use saito_content_api::{FieldValue, Registry, Value};
    let content = exe5_content();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let chip = |name: &str| Fact::Value(Value::Def(Registry::Chip, crate::ids::chip(&content, "exe5", name).unwrap().0));
    // Its places: a chip in place 2, a 0 in place 4, a chip in place 21, a
    // pattern in place 42; its records none written.
    let mut places: Vec<Fact> = (0..42).map(|_| Fact::Record(Vec::new())).collect();
    places[1] = Fact::Record(vec![("chip", chip("areagrab"))]);
    places[3] = Fact::Record(vec![("zero", Fact::Value(Value::Bool(true)))]);
    places[20] = Fact::Record(vec![("chip", chip("sword"))]);
    places[41] = Fact::Record(vec![("pattern", Fact::Value(Value::Int(1)))]);
    m.sides[0].set_fact(&content, "auto_battle_places", &places).unwrap();
    m.sides[0].set_fact(&content, "auto_battle_records", &[]).unwrap();
    m.sides[1].set_fact(&content, "auto_battle_places", &[]).unwrap();
    // The sent places, by side: a chip, a pattern's number, 0, or empty.
    let sent = |b: &saito_battle::Battle, side: u8| -> Vec<String> {
        let (schema, state) = b.rules_state(side).unwrap();
        let place = schema.place(schema.index_of("auto_battle_places").unwrap());
        (0..state.len_at(place).unwrap())
            .map(|k| {
                let e = place.elem(k).unwrap();
                match (state.get_at(e.field("chip").unwrap()), state.get_at(e.field("pattern").unwrap()), state.get_at(e.field("zero").unwrap())) {
                    (FieldValue::Ref(Some((_, h))), _, _) => crate::ids::local(&content.defs.chip(saito_content_api::ChipHandle(h)).key).to_string(),
                    (_, FieldValue::U8(n), _) if n > 0 => format!("pattern {n}"),
                    (_, _, FieldValue::Bool(true)) => "0".to_string(),
                    _ => "-".to_string(),
                }
            })
            .collect()
    };
    let b = crate::check::start(&content, &m).unwrap();
    let left = sent(&b, 0);
    assert_eq!(left.len(), 4, "{left:?}");
    assert_eq!(left[0], "areagrab", "the first three places among themselves: {left:?}");
    for e in ["0", "sword", "pattern 1"] {
        assert!(left.contains(&e.to_string()), "{e} in {left:?}");
    }
    assert!(sent(&b, 1).is_empty());
    // 84 draws in all, whatever the sides hold.
    let mut rng = saito_battle::Rng::new(m.round(&content, m.seed.unwrap_or(0)).rng);
    for _ in 0..84 {
        rng.next_positive();
    }
    assert_eq!(b.rng, rng, "the send's 84 draws");
    // Both sides' places shuffled alike: the same places, the same order.
    let mut both = m.clone();
    both.sides[1].set_fact(&content, "auto_battle_places", &places).unwrap();
    let b2 = crate::check::start(&content, &both).unwrap();
    assert_eq!((sent(&b2, 0), sent(&b2, 1)), (left.clone(), left), "the same order");
    assert_eq!(b2.rng, rng);
}

/// No soul's chip in EXE4's auto battle data (content/exe4/rules/
/// auto_battle/block.luau): the game's writer places no chip of class 3
/// (0x0800EFA8), and the dark soul's AI playing one starts the soul's
/// change with no soul, AIData +0x10 at the 0xFF the dark soul's intake
/// leaves (0x0800B6D0), which the original takes past its tables by soul
/// (0x0800E05C's jump). Its `validate` says so of each place that holds
/// one, and nothing of a Cannon.
#[test]
fn a_souls_chip_is_no_place_of_exe4s_auto_battle_data() {
    use crate::wrong::Wrong;
    let content = exe4_content();
    let mut m = crate::pick::live(&content, "exe4", 3, None).unwrap();
    let chip = |name: &str| crate::ids::chip(&content, "exe4", name).unwrap();
    let place = |name: &str| Fact::Record(vec![("chip", Fact::Value(Value::Def(Registry::Chip, chip(name).0)))]);
    let mut places: Vec<Fact> = (0..19).map(|_| Fact::Record(Vec::new())).collect();
    places[15] = place("metlsoul");
    places[16] = place("cannon");
    places[18] = place("rollsoul");
    m.sides[0].set_fact(&content, "auto_battle_places", &places).unwrap();
    let said: Vec<(Option<usize>, Option<usize>, Wrong)> = crate::check::problems(&content, &m)
        .into_iter()
        .filter(|p| p.field.as_deref() == Some("auto_battle_places"))
        .map(|p| (p.side, p.entry, p.what))
        .collect();
    let refused = |entry: usize, name: &str| (Some(0), Some(entry), Wrong::ChipNotAllowed { chip: chip(name) });
    assert_eq!(said, [refused(15, "metlsoul"), refused(18, "rollsoul")]);
}

/// The stages a match may name are its rules' (`link_pick.match_stages`):
/// each game's link battle stages, those with the link effect that aren't
/// the random battle's (EXE5's that its pick never reaches among them), in
/// handle order.
#[test]
fn a_match_names_its_rules_link_battle_stages() {
    use saito_battle::setup::effects;
    for (content, game, n) in [(exe6_content(), "exe6", 96), (exe5_content(), "exe5", 94)] {
        let by_effects: Vec<saito_content_api::StageHandle> = (0..content.defs.stages.len() as u16)
            .map(saito_content_api::StageHandle)
            .filter(|&s| {
                let e = content.stage(s).effects;
                e & effects::LINK != 0 && e & effects::RANDOM == 0
            })
            .collect();
        assert_eq!(crate::link_battle_stages(&content, game), by_effects, "{game}");
        assert_eq!(by_effects.len(), n, "{game}");
    }
}

/// EXE4's lava burns a player while the battle is dimmed (0x08013128 has no
/// dimming test), wearing 20 off its mood; its grass heals a wood body every
/// 20 ticks at any HP (0x08012FF2): its rules/panels.luau's hooks.
#[test]
fn exe4s_lava_and_grass_are_its_own() {
    use saito_battle::Battle;
    use saito_battle::battle::battle_flags;
    let content = exe4_content();
    let panels = &content.rules().panels;
    let (lava, grass) = (panels.named("lava").expect("EXE4's lava"), panels.named("grass").expect("EXE4's grass"));
    let battle = || {
        let m = crate::pick::live(&content, "exe4", 3, None).unwrap();
        let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
        let navi = (0..600)
            .find_map(|_| {
                b.tick(&Default::default(), Default::default());
                b.player(0).filter(|&r| b.objects.get(r).collision.is_some())
            })
            .expect("side 0's navi, with its collision data");
        (b, navi)
    };
    let (mut b, navi) = battle();
    let p = b.objects.get(navi).panel;
    b.set_panel_type(p.x, p.y, lava);
    b.round.flags |= battle_flags::DIMMED;
    saito_battle::kinds::common::panel_burn(&mut b, navi, true);
    let c = b.collision.get(b.objects.get(navi).collision.unwrap());
    assert_eq!((c.acc.element_damage[1], c.acc.mood_damage), (50, 20), "a player burns while dimmed");
    assert_eq!(b.field.panel(p.x, p.y).unwrap().kind, panels.roles.normal);

    let (mut b, navi) = battle();
    let p = b.objects.get(navi).panel;
    b.set_panel_type(p.x, p.y, grass);
    let o = b.objects.get_mut(navi);
    o.element = 4;
    (o.hp, o.max_hp) = (5, 100);
    b.round.cycles = [0, 1];
    // (The fight on: the standing effects wait out a pause.)
    b.round.flags |= battle_flags::FIGHTING;
    b.paused = false;
    saito_battle::kinds::player::update(&mut b, navi);
    assert_eq!(b.objects.get(navi).hp, 6, "at 5 HP, on the 20-tick count");
}

/// EXE4's ice pushes (0x0801335A): a move's end on it ORs the push bit of
/// the navi's side and the move's direction into its final hit modifier,
/// which the intake reads after as a push (slide type 1); it starts no slide
/// by rows (EXE4's reactions have none).
#[test]
fn exe4s_ice_pushes() {
    use saito_battle::Battle;
    use saito_battle::battle::battle_flags;
    let content = exe4_content();
    let ice = content.rules().panels.named("ice").expect("EXE4's ice");
    assert_eq!(content.rules().slide_rows, None);
    let m = crate::pick::live(&content, "exe4", 3, None).unwrap();
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    let navi = (0..600)
        .find_map(|_| {
            b.tick(&Default::default(), Default::default());
            b.player(0).filter(|&r| b.objects.get(r).collision.is_some())
        })
        .expect("side 0's navi, with its collision data");
    let p = b.objects.get(navi).panel;
    b.set_panel_type(p.x, p.y, ice);
    let c = b.objects.get(navi).collision.unwrap();
    let d = b.collision.get_mut(c);
    d.f1 |= saito_battle::collision::f1::AFFECTED_BY_ICE | saito_battle::collision::f1::MOVE_COMPLETE;
    // (Side 0's move forward: its push 0x10, a push forward.)
    d.direction = 4;
    b.round.flags |= battle_flags::FIGHTING;
    b.paused = false;
    saito_battle::kinds::player::update(&mut b, navi);
    assert_eq!(b.objects.get(navi).slide_type, 1, "a push's slide");
}

/// docs/design/exe4-map.md §18 item 12: EXE4's pitfall turns normal 190
/// ticks after a type change makes it (its `changed`, 0x08009DC4); a stage's
/// waits, armed (its `start`, 0x08009120), until a grounded body stands on it
/// (its `tick`, 0x0800980E).
#[test]
fn exe4s_pitfall_crumbles() {
    use saito_battle::Battle;
    let content = exe4_content();
    let panels = &content.rules().panels;
    let (pitfall, normal) = (panels.named("pitfall").expect("EXE4's pitfall"), panels.roles.normal);
    let m = crate::pick::live(&content, "exe4", 3, None).unwrap();
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    b.set_panel_type(2, 1, pitfall);
    assert_eq!(b.field.panels[1][2].timer, 190);
    for _ in 0..189 {
        b.tick_panels();
    }
    assert_eq!(b.field.panels[1][2].kind, pitfall);
    b.tick_panels();
    assert_eq!(b.field.panels[1][2].kind, normal, "190 ticks after it was made");
    // Armed, as a stage's: nothing stands on it, so it waits.
    b.set_panel_type(2, 1, pitfall);
    b.field.panels[1][2].timer = 0x8000 | 190;
    for _ in 0..300 {
        b.tick_panels();
    }
    assert_eq!((b.field.panels[1][2].kind, b.field.panels[1][2].timer), (pitfall, 0x8000 | 190));
    // A grounded body stands on it: the count starts that tick.
    b.field.panels[1][2].flags |= saito_battle::field::pflags::BODY_SIDE0;
    b.tick_panels();
    assert_eq!(b.field.panels[1][2].timer, 189);
}

// ---- EXE5's party battle (EXE5 DS's: content/exe5/rules/party) ----------------

/// An EXE5 battle whose left side brings this party (match text of its
/// `party` fact), its custom screens closing at once, ticked to the fight.
fn party_battle(party: &str) -> saito_battle::Battle {
    let content = exe5_content();
    let left = exe5(&TANGO_EXE5, &format!("party = [{party}]"));
    let m = parse(&content, &left, &exe5(&TANGO_EXE5, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let mut setup = m.round(&content, 0x5EED);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);
    b
}

/// Tick with nothing pressed until the battle fights (not paused, no
/// custom screen, undimmed).
fn fight(b: &mut saito_battle::Battle) {
    for _ in 0..2000 {
        if b.fight.state == saito_battle::battle::fight::FIGHTING && !b.paused && !b.is_dimmed() && !b.custom.sides[0].in_custom {
            return;
        }
        b.tick(&Default::default(), Default::default());
    }
    panic!("the battle never fights");
}

/// A tick of side 0 asking to switch to `member`.
fn ask_switch(b: &mut saito_battle::Battle, member: u8) {
    let ask = saito_battle::input::PlayerTick { switch_to: Some(member), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
}

fn navi_key(b: &saito_battle::Battle, side: usize) -> String {
    crate::ids::local(&b.content.defs.navi(b.stats[side].navi).key).to_string()
}

/// The partners join with their fresh stats and their PartyCust programs:
/// the base HP at the side's level 9 (the DS's table) and the programs'
/// bonus, which they enter at; the attack and the chip attack bonus added.
#[test]
fn a_partys_partners_join_with_their_programs() {
    let b = party_battle(r#"{ navi = "protoman", programs = ["p-hp-100", "p-atk-1", "p-chp-30"] }, { navi = "gyroman" }"#);
    let content = exe5_content();
    let p = &b.party[0];
    assert_eq!((p.count, p.active, p.switches), (3, 0, 0));
    assert_eq!(b.party[1].count, 1, "the other side brings none");
    let proto = p.member(1).unwrap();
    assert_eq!(crate::ids::local(&content.defs.navi(proto.navi).key), "protoman");
    assert_eq!((proto.max_base_hp, proto.max_hp, proto.hp, proto.attack), (700, 800, 800, 1));
    assert_eq!(proto.game_stat(&content, "chip_attack_bonus"), Some(saito_content_api::FieldValue::U16(30)));
    let gyro = p.member(2).unwrap();
    assert_eq!((gyro.max_hp, gyro.hp, gyro.attack), (600, 600, 0));
    // Member 0 is the side's stats as the battle started.
    assert_eq!(p.members[0].navi, b.stats[0].navi);
}

/// What a party may be: MegaMan's, of team navis, each once, its programs
/// within its gauge (ProtoMan's 6 blocks).
#[test]
fn a_partys_setup_is_held_to_the_rules() {
    let content = exe5_content();
    let right = exe5(&TANGO_EXE5, "");
    let problems = |left: &str| parse(&content, left, &right).err().unwrap_or_default().join("\n");
    let party = |p: &str| exe5(&TANGO_EXE5, &format!("party = [{p}]"));
    assert_eq!(problems(&party(r#"{ navi = "protoman", programs = ["p-hp-300", "p-hp-50"] }"#)), "");
    assert!(problems(&party(r#"{ navi = "protoman", programs = ["p-hp-300", "p-hp-100"] }"#)).contains("fill 7 slots of its 6"));
    assert!(problems(&party(r#"{ navi = "protoman" }, { navi = "protoman" }"#)).contains("in the party at 1 too"));
    assert!(problems(&party(r#"{ navi = "megaman" }"#)).contains("is no team navi"));
    let team_navi = side("protoman", &TANGO_EXE5, "level = 6\nparty = [{ navi = \"gyroman\" }]");
    assert_eq!(problems(&team_navi), "");
    let self_partner = side("protoman", &TANGO_EXE5, "level = 6\nparty = [{ navi = \"protoman\" }]");
    assert!(problems(&self_partner).contains("is no team navi"));
    let megaman_own = exe5(&TANGO_EXE5, "party_programs = [\"p-hp-50\"]");
    assert!(problems(&megaman_own).contains("takes no PartyCust programs"));
    let team_over = side("protoman", &TANGO_EXE5, "level = 6\nparty_programs = [\"p-hp-300\", \"p-hp-100\"]");
    assert!(problems(&team_over).contains("fill 7 slots of its 6"));
}

/// The in-battle switch (the DS's action 0x59): a request takes the
/// member's block whole and leaves MegaMan's HP in his, dims the battle
/// while it lands and settles, and counts the switch.
#[test]
fn a_switch_in_battle_takes_the_members_block() {
    let mut b = party_battle(r#"{ navi = "protoman", programs = ["p-hp-100"] }"#);
    let p0 = b.player(0).unwrap();
    b.objects.get_mut(p0).hp = 640;
    ask_switch(&mut b, 1);
    let mut dimmed = false;
    for _ in 0..60 {
        dimmed |= b.is_dimmed();
        b.tick(&Default::default(), Default::default());
    }
    assert!(dimmed, "the switch dims the battle");
    assert!(!b.is_dimmed(), "and undims as it settles");
    assert_eq!(navi_key(&b, 0), "protoman");
    let p = &b.party[0];
    assert_eq!((p.active, p.switches), (1, 1));
    assert_eq!(p.members[0].hp, 640, "MegaMan's HP is kept in his block");
    let o = b.objects.get(b.player(0).unwrap());
    assert_eq!((o.hp, o.max_hp), (800, 800));
    assert_eq!(b.stats[0].mood, 0x80, "a team navi enters at the normal mood");
    // And back: MegaMan with his HP.
    ask_switch(&mut b, 0);
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!((navi_key(&b, 0).as_str(), b.party[0].active, b.party[0].switches), ("megaman", 0, 2));
    assert_eq!(b.objects.get(b.player(0).unwrap()).hp, 640);
}

/// The panel's limit (the DS's 3, beside NavChg's): the fourth request is
/// refused.
#[test]
fn the_fourth_switch_is_refused() {
    let mut b = party_battle(r#"{ navi = "protoman" }"#);
    for i in 0..4u8 {
        ask_switch(&mut b, if i % 2 == 0 { 1 } else { 0 });
        for _ in 0..60 {
            b.tick(&Default::default(), Default::default());
        }
    }
    assert_eq!((b.party[0].switches, b.party[0].active), (3, 1));
}

/// A side without partners: a switch request is nothing (its input record
/// keeps none), and the battle goes as one without it.
#[test]
fn a_switch_request_without_partners_is_nothing() {
    let content = exe5_content();
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let mut a = saito_battle::Battle::new(m.round(&content, 7), content.clone());
    let mut b = a.clone();
    for _ in 0..300 {
        a.tick(&Default::default(), Default::default());
        ask_switch(&mut b, 1);
    }
    assert_eq!(b.switch_inputs, [None; 2]);
    assert_eq!(a.digest(), b.digest());
}

/// A switched-in team navi at 0 HP falls back (EXE5's action 0x4C): its
/// block keeps HP 0, MegaMan comes back with his, and the battle goes on.
#[test]
fn a_team_navi_at_zero_hp_falls_back_to_megaman() {
    let mut b = party_battle(r#"{ navi = "protoman" }"#);
    ask_switch(&mut b, 1);
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(navi_key(&b, 0), "protoman");
    let r = b.player(0).unwrap();
    b.objects.get_mut(r).hp = 0;
    for _ in 0..40 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(navi_key(&b, 0), "megaman");
    assert_eq!(b.party[0].active, 0);
    assert_eq!(b.party[0].members[1].hp, 0, "ProtoMan's block is kept at 0 HP");
    assert!(!b.is_battle_over());
    assert_eq!(b.objects.get(b.player(0).unwrap()).hp, b.party[0].members[0].hp);
    // A switch to the member at 0 HP is refused.
    ask_switch(&mut b, 1);
    assert_eq!(b.switch_inputs[0], None);
}

/// A battle that ends with a switched-in team navi on the field: the navi
/// runs the switch knockout (EXE5 DS's action 0x4D, from the battle's end in
/// its idle, 0x02045108): MegaMan comes back with his HP.
#[test]
fn a_battle_ending_with_a_team_navi_on_the_field_brings_megaman_back() {
    let mut b = party_battle(r#"{ navi = "protoman" }"#);
    let r = b.player(0).unwrap();
    b.objects.get_mut(r).hp = 640;
    ask_switch(&mut b, 1);
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(navi_key(&b, 0), "protoman");
    let enemy = b.player(1).unwrap();
    b.objects.get_mut(enemy).hp = 0;
    let mut knocked_out = false;
    for _ in 0..120 {
        b.tick(&Default::default(), Default::default());
        if let Some(r) = b.player(0) {
            knocked_out |= saito_battle::kinds::player::navi_action(&b, r)
                == saito_battle::kinds::player::NaviAction::Engine(saito_battle::kinds::player::EngineAction::SwitchKnockout);
        }
    }
    assert!(b.is_battle_over());
    assert!(knocked_out, "the team navi runs the switch knockout");
    assert_eq!((navi_key(&b, 0).as_str(), b.party[0].active), ("megaman", 0));
    assert_eq!(b.objects.get(b.player(0).unwrap()).hp, 640);
}

/// ToadMan's rescue (with P.Spport): the navi at 0 HP is healed a quarter of
/// its maximum, once a battle; the second time it is deleted. Its HP number,
/// gone at 0 HP, shows again on the other side's console.
#[test]
fn toadman_rescues_once_a_battle() {
    let mut b = party_battle(r#"{ navi = "toadman", programs = ["p-spport"] }"#);
    let r = b.player(0).unwrap();
    let numbered = |b: &saito_battle::Battle, console: usize| b.hp_numbers[console].iter().flatten().any(|n| n.object == r);
    assert!(numbered(&b, 1) && !numbered(&b, 0));
    let max = b.objects.get(r).max_hp;
    b.objects.get_mut(r).hp = 0;
    let mut gone = false;
    for _ in 0..400 {
        b.tick(&Default::default(), Default::default());
        gone |= !numbered(&b, 1);
        if b.objects.get(r).hp != 0 {
            break;
        }
    }
    assert_eq!(b.objects.get(r).hp, max / 4);
    assert!(!b.is_battle_over());
    assert!(gone, "the number goes at 0 HP");
    assert!(numbered(&b, 1) && !numbered(&b, 0), "and shows again as it is healed");
    fight(&mut b);
    b.objects.get_mut(r).hp = 0;
    for _ in 0..200 {
        b.tick(&Default::default(), Default::default());
    }
    assert!(b.is_battle_over(), "the second time, no rescue");
}

/// No P.Spport, no rescue.
#[test]
fn toadman_without_support_rescues_no_one() {
    let mut b = party_battle(r#"{ navi = "toadman" }"#);
    let r = b.player(0).unwrap();
    b.objects.get_mut(r).hp = 0;
    for _ in 0..200 {
        b.tick(&Default::default(), Default::default());
    }
    assert!(b.is_battle_over());
}

/// A team navi's own chip in a party: its navi's base at the level and its
/// P.Chp (StepSwrd, ProtoMan's: 100 at level 9 and 40); a side without the
/// navi in its party, 999 (the DS's finder finds none).
#[test]
fn a_team_navis_own_chip_in_a_party_takes_its_bonus() {
    let b = party_battle(r#"{ navi = "protoman", programs = ["p-chp-40"] }"#);
    let content = exe5_content();
    let stepswrd = crate::ids::chip(&content, "exe5", "stepswrd").unwrap();
    let airforce = crate::ids::chip(&content, "exe5", "airforce").unwrap();
    assert_eq!(b.given.damage(stepswrd, 0), Some(140));
    assert_eq!(b.given.damage(airforce, 0), Some(999));
}

/// The custom screen's switch (EXE5 DS's state 0x40): the request takes the
/// screen to the member (counted, the picks dropped), and OK makes it the
/// turn's switch, which the turn's start takes while paused (the member's
/// block whole, 20 ticks landing, 21 settling).
#[test]
fn a_switch_on_the_custom_screen_is_the_turns() {
    use saito_battle::custom::screen::Phase;
    use saito_battle::input::PlayerTick;
    let content = exe5_content();
    let left = exe5(&TANGO_EXE5, r#"party = [{ navi = "protoman" }]"#);
    let m = parse(&content, &left, &exe5(&TANGO_EXE5, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let mut b = saito_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
    let choosing = |b: &saito_battle::Battle| b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing);
    for _ in 0..400 {
        if choosing(&b) {
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    assert!(choosing(&b));
    // A pick, then the switch: the pick is dropped.
    b.tick(&[PlayerTick::default().with_choice(Some(saito_battle::ScreenChoice::Pick(0))), Default::default()], Default::default());
    b.tick(&Default::default(), Default::default());
    assert_eq!(b.custom.sides[0].screen.unwrap().selected, 1);
    ask_switch(&mut b, 1);
    assert!(matches!(b.custom.sides[0].screen.unwrap().phase, Phase::Switching { member: 1, .. }));
    for _ in 0..100 {
        if choosing(&b) {
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    let s = b.custom.sides[0].screen.unwrap();
    assert_eq!((s.phase, s.member, s.selected), (Phase::Choosing, Some(1), 0));
    assert_eq!(b.party[0].switches, 1, "the screen counts the switch");
    assert_eq!(navi_key(&b, 0), "megaman", "the navi on the field is still MegaMan");
    // A request for the screen's own navi is nothing.
    ask_switch(&mut b, 1);
    assert_eq!(b.custom.sides[0].screen.unwrap().phase, Phase::Choosing);
    // OK: the turn's switch to ProtoMan.
    b.tick(&[PlayerTick::default().with_choice(Some(saito_battle::ScreenChoice::Ok)), Default::default()], Default::default());
    let (_, transform) = b.custom.sides[0].built.clone().expect("OK built the hand");
    assert_eq!(transform.navi_switch.map(|n| crate::ids::local(&content.defs.navi(n).key).to_string()), Some("protoman".into()));
    // The other side's OK; then the turn starts with the paused switch.
    for _ in 0..600 {
        if b.custom.sides[1].built.is_none() && b.custom.sides[1].screen.is_some_and(|s| s.phase == Phase::Choosing) {
            b.tick(&[Default::default(), PlayerTick::default().with_choice(Some(saito_battle::ScreenChoice::Ok))], Default::default());
            continue;
        }
        b.tick(&Default::default(), Default::default());
        if b.party[0].active == 1 && !b.paused {
            break;
        }
    }
    assert_eq!((navi_key(&b, 0).as_str(), b.party[0].active, b.party[0].switches), ("protoman", 1, 1));
    let o = b.objects.get(b.player(0).unwrap());
    assert_eq!((o.hp, o.max_hp), (700, 700));
}

/// The soul button is MegaMan's: the screen shows it by the navi it is for
/// (EXE5 DS's 0x0200AAA0: the screen's navi, its +0x10, is MegaMan), not by
/// the navi on the field. A switch on the screen to a team navi takes it
/// away, and a switch back brings it again.
#[test]
fn the_soul_button_follows_the_screens_navi() {
    use saito_battle::content::ButtonView;
    use saito_battle::custom::screen::{Phase, SPECIAL_SLOT, SlotKind};
    let content = exe5_content();
    let left = exe5(&TANGO_EXE5, r#"party = [{ navi = "protoman" }]"#);
    let m = parse(&content, &left, &exe5(&TANGO_EXE5, "")).unwrap_or_else(|p| panic!("{p:?}"));
    let mut b = saito_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
    let choosing = |b: &saito_battle::Battle| b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing);
    let settle = |b: &mut saito_battle::Battle| {
        for _ in 0..400 {
            if choosing(b) {
                break;
            }
            b.tick(&Default::default(), Default::default());
        }
        assert!(choosing(b));
    };
    let soul_button = |b: &saito_battle::Battle| {
        let s = b.custom.sides[0].screen.unwrap();
        matches!(s.slots[SPECIAL_SLOT as usize].kind, SlotKind::Button { button, .. } if b.content.defs.button(button).view == Some(ButtonView::FormOffer))
    };
    settle(&mut b);
    assert!(soul_button(&b), "MegaMan's screen has the soul button");
    ask_switch(&mut b, 1);
    settle(&mut b);
    assert_eq!(b.custom.sides[0].screen.unwrap().member, Some(1));
    assert!(!soul_button(&b), "ProtoMan's screen has none");
    ask_switch(&mut b, 0);
    settle(&mut b);
    assert_eq!(b.custom.sides[0].screen.unwrap().member, None);
    assert!(soul_button(&b), "MegaMan's again: the button again");
}

/// EXE5 DS's shout (rules/shout), which nothing in saito makes: a worried
/// MegaMan at a quarter of his HP who shouts is healed, charged or
/// barriered once a battle, and put in Full Synchro first (saito's
/// microphone is silent or full, the strongest shout). A silent tick, or a
/// second shout, does nothing.
#[test]
fn a_shout_helps_once_a_battle() {
    use saito_battle::input::PlayerTick;
    let content = exe5_content();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    m.ds_shout_enabled = true;
    let mut setup = m.round(&content, 0x5EED);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);
    let r = b.player(0).unwrap();
    b.objects.get_mut(r).hp = 200;
    b.stats[0].mood = 0x30;
    let shout = |b: &mut saito_battle::Battle, mic_tier: u8| {
        b.tick(&[PlayerTick { mic_tier, ..Default::default() }, Default::default()], Default::default());
    };
    // Silent.
    shout(&mut b, 0);
    assert!(!b.is_dimmed());
    let before = (b.stats[0].attack, b.stats[0].charge, b.stats[0].rapid);
    shout(&mut b, 3);
    assert!(b.is_dimmed(), "the shout dims the battle");
    for _ in 0..200 {
        b.tick(&Default::default(), Default::default());
    }
    assert!(!b.is_dimmed());
    assert_eq!(b.stats[0].mood, 0xFF, "the strongest shout: Full Synchro");
    let helped = b.objects.get(r).hp == 500
        || (b.stats[0].attack, b.stats[0].charge, b.stats[0].rapid) != before
        || b.collision.get(b.objects.get(r).collision.unwrap()).barrier != 0;
    assert!(helped, "healed, charged or barriered");
    // Once a battle.
    b.objects.get_mut(r).hp = 200;
    b.stats[0].mood = 0x30;
    shout(&mut b, 3);
    assert!(!b.is_dimmed());
}

/// EXE5's emotion-swing bug (content/exe5/rules/emotion, 0x080113F8) goes on
/// through its navi's deletion: the navi leaves its side's list of alive
/// actors as its deletion starts (`sub_800A11C`) and its tick still runs,
/// swinging every 60 ticks from its own state, as the original's swing does
/// from the navi's AIData (+0x3A, +0x0B). (The port kept that state by the
/// navi's slot on the list, and stopped the round: "a navi of the player's
/// kind on no slot of its side's list".) A patch card's or a NaviCust's bug
/// a build creator makes brings it.
#[test]
fn a_deleted_navis_emotion_swing_goes_on() {
    use saito_battle::battle::{fight, mode};
    let content = exe5_content();
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let mut b = saito_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
    let step = |b: &mut saito_battle::Battle, ok: bool| {
        let tick = saito_battle::PlayerTick::default().with_choice(ok.then_some(saito_battle::ScreenChoice::Ok));
        let input = saito_battle::TickInput { players: [tick; 2], events: Default::default() };
        b.step(&input);
        assert!(!b.is_stopped(), "the engine stopped: {:?}", b.round_end());
    };
    // To the fight, OK pressed on the first custom screen.
    let mut t = 0;
    while !(b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING) {
        let ok = b.round.mode == mode::CUSTOM && t % 4 == 2;
        step(&mut b, ok);
        t += 1;
        assert!(t < 2000, "no fight");
    }
    // Side 1's navi with the bug, its HP gone: its deletion starts.
    b.stats[1].bugs.emotion = 1;
    let navi = b.player(1).unwrap();
    b.objects.get_mut(navi).hp = 0;
    for _ in 0..3 {
        step(&mut b, false);
    }
    assert_eq!(b.round.alive[1], 0, "the navi left its side's list");
    assert!(b.round.alive_actors[1].iter().all(|s| s.is_none()));
    // Two swings' time: the swing runs from the deleted navi's tick.
    for _ in 0..130 {
        if !b.objects.is_allocated(navi) {
            break;
        }
        step(&mut b, false);
    }
}

/// A barrier visual's sprite (`content/exelib/barriers/visual.luau`, LifeAura's
/// `"200"` digits among them) keeps `look.hflip` equal to each console's
/// `view.mirror` (`false` on console 0, `true` on console 1) so `look.hflip ^
/// view.mirror == false` on both screens in EXE4, EXE5, and EXE6.
#[test]
fn life_aura_visual_stays_unflipped_on_both_consoles() {
    for (content, game, card) in [
        (exe4_content(), "exe4", "078-first-life-aura"),
        (exe5_content(), "exe5", "life-virus"),
        (exe6_content(), "exe6", "famous-custom"),
    ] {
        for side in [0usize, 1usize] {
            let mut m = crate::pick::live(&content, game, 3, None).unwrap();
            let cards = crate::testing::patch_cards(&content, game, card);
            m.sides[side].set_fact(&content, "navi", &[Fact::Value(Value::Def(Registry::Navi, crate::ids::navi(&content, game, "megaman").unwrap().0))]).unwrap();
            m.sides[side].set_fact(&content, "patch_cards", &cards).unwrap();
            let mut b = saito_battle::Battle::new(m.round(&content, 0x5EED), content.clone());
            let mut checked = false;
            for _ in 0..120 {
                b.tick(&Default::default(), Default::default());
                let visuals: Vec<_> = b
                    .objects
                    .in_order()
                    .filter(|&r| b.local_kind_key(r) == "barrier-visual" && b.objects.sprite(r).id.is_some())
                    .collect();
                if let Some(&r) = visuals.first() {
                    let look0 = b.look_for(r, 0);
                    let look1 = b.look_for(r, 1);
                    assert!(!look0.hflip, "{game} side {side}: console 0 look.hflip must be false");
                    assert!(look1.hflip, "{game} side {side}: console 1 look.hflip must be true (cancels view.mirror)");
                    checked = true;
                    break;
                }
            }
            assert!(checked, "{game} side {side}: LifeAura barrier-visual never initialized");
        }
    }
}

/// In EXE5, Wind/Fan gusts (collision `pull` / element 5) do NOT strip barriers
/// or LifeAura, whereas NrthWind (hit flag 0x20) still takes them away.
#[test]
fn exe5_wind_and_fan_do_not_strip_barriers_but_nrthwind_does() {
    use saito_battle::input::{PlayerTick, keys};
    let content = exe5_content();
    let run_chip = |chip_key: &str| -> u8 {
        let mut m = crate::pick::live(&content, "exe5", 3, None).unwrap();
        let chip = crate::ids::chip(&content, "exe5", chip_key).unwrap();
        let cards = crate::testing::patch_cards(&content, "exe5", "life-virus");
        m.sides[1].set_fact(&content, "navi", &[Fact::Value(Value::Def(Registry::Navi, crate::ids::navi(&content, "exe5", "megaman").unwrap().0))]).unwrap();
        m.sides[1].set_fact(&content, "patch_cards", &cards).unwrap();

        let mut setup = m.round(&content, 0x5EED);
        setup.custom_screen_limit = Some(1);
        let mut b = saito_battle::Battle::new(setup, content.clone());
        fight(&mut b);
        let p1 = b.player(1).expect("side 1 navi");
        let c1 = b.objects.get(p1).collision.expect("side 1 collision");
        assert_ne!(b.collision.get(c1).barrier, 0, "side 1 should start with LifeAura");

        b.hands[0].cursor = 0;
        b.hands[0].ids[0] = Some(chip);

        let mut spawned_gust = false;
        for t in 0..260 {
            let input = if t == 0 {
                [PlayerTick { held: keys::A, ..Default::default() }, Default::default()]
            } else {
                Default::default()
            };
            b.tick(&input, Default::default());
            if b.objects.in_order().any(|r| b.local_kind_key(r) == "gust") {
                spawned_gust = true;
            }
        }
        if matches!(chip_key, "wind" | "fan") {
            assert!(spawned_gust, "EXE5 {chip_key} should have spawned gusts");
        }
        b.collision.get(c1).barrier
    };

    assert_ne!(run_chip("wind"), 0, "EXE5 Wind must not strip LifeAura");
    assert_ne!(run_chip("fan"), 0, "EXE5 Fan must not strip LifeAura");
    assert_eq!(run_chip("nrthwind"), 0, "EXE5 NrthWind must still strip LifeAura");
}

/// S-072: In EXE5, if evil MegaMan triggers Dark Soul survival (`dark-survival`)
/// while guarding (e.g. Guard1 / Shield / Reflect), `dark-survival` interrupts the
/// guard action and drops the shield visual; it must also clear `f1::GUARD` so
/// MegaMan does not retain an invisible permanent "shield body" after Dark Soul ends.
#[test]
fn exe5_dark_survival_clears_guard_flag() {
    use saito_battle::collision::f1;
    use saito_battle::input::{PlayerTick, keys};

    let content = exe5_content();
    let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let guard1 = crate::ids::chip(&content, "exe5", "guard1").unwrap();
    m.sides[0].set_fact(&content, "karma", &number(0)).unwrap();

    let mut setup = m.round(&content, 0x5EED);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);

    let p0 = b.player(0).expect("side 0 navi");
    let c0 = b.objects.get(p0).collision.expect("side 0 collision");
    assert_eq!(saito_battle::kinds::player::emotion_name(&b, 0), "evil");

    // Put Guard1 in hand and raise it.
    b.hands[0].cursor = 0;
    b.hands[0].ids[0] = Some(guard1);
    let mut guarding = false;
    for t in 0..20 {
        let input = if t == 0 {
            [PlayerTick { held: keys::A, ..Default::default() }, Default::default()]
        } else {
            Default::default()
        };
        b.tick(&input, Default::default());
        if b.collision.get(c0).f1 & f1::GUARD != 0 {
            guarding = true;
            break;
        }
    }
    assert!(guarding, "Guard1 should set f1::GUARD on evil MegaMan");

    // Place a poison panel under evil MegaMan at 1 HP while guarding so HP drains to 0
    // through the guard and triggers `dark-survival` (`volley`), interrupting `guard1/action`.
    let poison = content.rules().panels.named("poison").expect("EXE5 poison panel");
    let pos = b.objects.get(p0).panel;
    b.set_panel_type(pos.x, pos.y, poison);
    b.objects.get_mut(p0).hp = 1;

    let mut entered_survival = false;
    for _ in 0..30 {
        let a = b.actors.get(b.objects.get(p0).actor.unwrap());
        if let saito_battle::kinds::player::NaviAction::Content(h) = a.navi_action
            && content.defs.action(h).key == "dark-survival"
        {
            entered_survival = true;
            assert_eq!(
                b.collision.get(c0).f1 & f1::GUARD,
                0,
                "dark-survival must clear f1::GUARD when interrupting a guard action"
            );
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    assert!(entered_survival, "lethal damage on evil MegaMan should trigger dark-survival");
    assert_eq!(b.objects.get(p0).hp, 1, "dark-survival should hold MegaMan at 1 HP");
}

/// EXE5DS Sol Cross (`sol-cross-megaman` 70 MB DS-only patch card) ruleset gating and validation:
/// - Allowed in Senior 5DS (`senior_5ds`, `senior_5ds_mixed`, alongside `bass-cross-megaman`)
///   and Masters 5DS (`masters_5ds`, `masters_5ds_mixed`, at 70 MB out of 80 MB)
/// - Rejected as DS-only in GBA Masters (`masters`, `masters_mixed`) and disallowed in all pure GBA rulesets
/// - Regular patch cards remain banned in Senior 5DS (`senior_5ds`, `senior_5ds_mixed`)
/// - Mutually exclusive with `bass-cross-megaman` patch card.
#[test]
fn exe5_sol_cross_ruleset_gating_and_validation() {
    use crate::Wrong;
    use crate::build::{Builds, Checked};
    use crate::ruleset::Rulesets;

    let content = exe5_content();
    let r = Rulesets::of(&content, "exe5").unwrap();
    let builds = Builds::of(&content, "exe5").unwrap();

    let mut side = builds.new_build(&content, "protoman").unwrap();
    side.set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "sol-cross-megaman")).unwrap();

    // Allowed in Senior 5DS and Masters 5DS (single-version and mixed).
    for preset_id in ["senior_5ds", "senior_5ds_mixed", "masters_5ds", "masters_5ds_mixed"] {
        let preset = r.preset(preset_id).unwrap();
        let checked = Checked::under(&content, "exe5", &side, &r, preset);
        assert!(checked.ready(), "Sol Cross must be allowed in {preset_id}, got: {:?}", checked.problems);
    }

    // Bass-Cross MegaMan is also allowed in Senior 5DS, while regular patch cards remain banned.
    let mut fcr_side = builds.new_build(&content, "protoman").unwrap();
    fcr_side.set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "bass-cross-megaman/protoman")).unwrap();
    for preset_id in ["senior_5ds", "senior_5ds_mixed"] {
        let preset = r.preset(preset_id).unwrap();
        let checked = Checked::under(&content, "exe5", &fcr_side, &r, preset);
        assert!(checked.ready(), "Bass-Cross must be allowed in {preset_id}, got: {:?}", checked.problems);
    }
    let mut regular_side = builds.new_build(&content, "protoman").unwrap();
    regular_side.set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "mettaur")).unwrap();
    for preset_id in ["senior_5ds", "senior_5ds_mixed"] {
        let preset = r.preset(preset_id).unwrap();
        let checked = Checked::under(&content, "exe5", &regular_side, &r, preset);
        assert!(
            checked.problems.iter().any(|p| p.field.as_deref() == Some("patch_cards") && matches!(p.what, Wrong::NotAllowed { .. })),
            "Regular patch cards must stay banned in {preset_id}, got: {:?}",
            checked.problems
        );
    }

    // Rejected as DS-only in GBA Masters (where patch cards are otherwise allowed).
    for preset_id in ["masters", "masters_mixed"] {
        let preset = r.preset(preset_id).unwrap();
        let checked = Checked::under(&content, "exe5", &side, &r, preset);
        assert!(
            checked.problems.iter().any(|p| p.field.as_deref() == Some("patch_cards") && p.what == Wrong::DsOnly),
            "Sol Cross must be rejected as DsOnly in {preset_id}, got: {:?}",
            checked.problems
        );
    }

    // Rejected in GBA Senior and Gate rulesets.
    for preset_id in ["senior", "senior_mixed", "gate", "gate_mixed", "gate_party", "gate_party_mixed"] {
        let preset = r.preset(preset_id).unwrap();
        let checked = Checked::under(&content, "exe5", &side, &r, preset);
        assert!(!checked.ready(), "Sol Cross build must be rejected in {preset_id}");
    }

    // Validation: Sol Cross is mutually exclusive with Bass-Cross MegaMan patch card.
    let mut both = builds.new_build(&content, "protoman").unwrap();
    both.set_fact(
        &content,
        crate::facts::PATCH_CARDS,
        &crate::testing::patch_cards(&content, "exe5", "sol-cross-megaman, bass-cross-megaman/protoman"),
    )
    .unwrap();
    let both_checked = Checked::of(&content, "exe5", &both);
    assert!(
        both_checked.problems.iter().any(|p| p.field.as_deref() == Some("patch_cards") && p.what == Wrong::Invalid),
        "Sol Cross + Bass-Cross MegaMan must fail validation with Invalid, got: {:?}",
        both_checked.problems
    );
}

/// EXE5DS Sol Cross stats (+200 HP, SuperArmor, MegaFolder -2, 70 MB), HUD variants, sprite,
/// and weapon 0x6C (`sol_gundelsol`) flat 130-frame charge & full 60-tick no-hold drain
/// (120 HP in shade vs 240 HP in sun).
#[test]
fn exe5_sol_cross_stats_and_gundelsol_charge_shot() {
    use crate::build::{Builds, Checked, Stat};
    use saito_battle::input::{PlayerTick, keys};

    let content = exe5_content();
    let builds = Builds::of(&content, "exe5").unwrap();

    // 1. Stats and Build Overview (70 MB cost, same as Bass-Cross MegaMan).
    let mut side = builds.new_build(&content, "protoman").unwrap();
    side.set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "sol-cross-megaman")).unwrap();
    let mut checked = Checked::of(&content, "exe5", &side);
    assert!(checked.ready(), "{:?}", checked.problems);
    let stats = checked.stats().unwrap();
    assert_eq!((stats.max_hp, stats.hp, stats.super_armor, stats.mega_level), (1200, 1200, true, 3));
    let overview = checked.overview(&content, &side).unwrap();
    assert_eq!(overview.hp, Stat { base: 1000, value: 1200 });
    assert_eq!(overview.mega, Stat { base: 5, value: 3 });
    let budget = overview.budget(crate::facts::PATCH_CARDS).unwrap();
    assert_eq!((budget.count.used, budget.count.limit), (70, 80));

    // 2. Run Sol Cross GunDelSol in shade vs sun (also verifying flat 130-tick charge even with Charge=4).
    let run_drain = |sun_enabled: bool| -> (u16, bool) {
        let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        m.sides[0]
            .set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "sol-cross-megaman"))
            .unwrap();
        m.sun_enabled = sun_enabled;
        let mut setup = m.round(&content, 0x5EED);
        setup.custom_screen_limit = Some(1);
        let mut b = saito_battle::Battle::new(setup, content.clone());
        fight(&mut b);

        // Verify HUD variants, sprite, and absence of HubShade overlay.
        assert!(b.looks[0].face_variant_sol && b.looks[0].name_variant_sol);
        assert!(!b.looks[0].face_variant && !b.looks[0].face_variant_alt && !b.looks[0].name_variant);
        let p0 = b.player(0).expect("side 0 navi");
        let p1 = b.player(1).expect("side 1 navi");
        let sprite_id = b.objects.sprite(p0).id.expect("Sol Cross MegaMan sprite loaded");
        assert_eq!(
            content.assets.name(saito_content_api::AssetKind::Sprite, sprite_id.0),
            Some("solcross-megaman"),
            "Sol Cross MegaMan must load solcross-megaman sprite"
        );
        assert!(b.objects.get(p0).related[1].is_none(), "Sol Cross MegaMan must not wear HubShade overlay");

        // Max out Charge stat to verify charge time stays flat at 130 ticks regardless of Charge+.
        b.stats[0].charge = 4;

        // Step side 0 right from (2, 2) to (3, 2), which is 2 columns ahead of side 1 at (5, 2)
        // — directly in GunDelSol's column!
        b.tick(&[PlayerTick { held: keys::RIGHT, ..Default::default() }, Default::default()], Default::default());
        for _ in 0..10 {
            b.tick(&Default::default(), Default::default());
        }
        assert_eq!(b.objects.get(p0).panel.x, 3);
        assert_eq!(b.objects.get(p1).panel.x, 5);
        let hp_before = b.objects.get(p1).hp;

        // Verify releasing B at 120 ticks (< 130) does NOT fire SolCross GunDelSol even at Charge=4.
        for t in 0..150 {
            let input = if t < 120 {
                [PlayerTick { held: keys::B, ..Default::default() }, Default::default()]
            } else {
                Default::default()
            };
            b.tick(&input, Default::default());
            let a = b.actors.get(b.objects.get(p0).actor.unwrap());
            if let saito_battle::kinds::player::NaviAction::Content(h) = a.navi_action {
                assert_ne!(
                    crate::ids::local(&content.defs.action(h).key),
                    "megaman/sol-gundelsol/action",
                    "Sol Cross GunDelSol must not fire before 130 ticks (even at Charge=4)"
                );
            }
        }
        // Reset opponent HP (the early B release fired a plain buster shot).
        b.objects.get_mut(p1).hp = hp_before;

        // Hold B for 155 ticks (past charge startup + 130-tick flat charge threshold), then release ALL buttons.
        // Because `no_hold = true`, releasing B/A must NOT cut the 60-tick beam short.
        let mut fired_sol_gundelsol = false;
        for t in 0..270 {
            let input = if t < 155 {
                [PlayerTick { held: keys::B, ..Default::default() }, Default::default()]
            } else {
                Default::default()
            };
            b.tick(&input, Default::default());
            let a = b.actors.get(b.objects.get(p0).actor.unwrap());
            if let saito_battle::kinds::player::NaviAction::Content(h) = a.navi_action
                && crate::ids::local(&content.defs.action(h).key) == "megaman/sol-gundelsol/action"
            {
                fired_sol_gundelsol = true;
            }
        }
        let hp_after = b.objects.get(p1).hp;
        (hp_before - hp_after, fired_sol_gundelsol)
    };

    let (shade_drain, shade_fired) = run_drain(false);
    let (sun_drain, sun_fired) = run_drain(true);
    assert!(shade_fired, "Sol Cross charged shot should fire megaman/sol-gundelsol/action in shade");
    assert!(sun_fired, "Sol Cross charged shot should fire megaman/sol-gundelsol/action in sun");
    assert_eq!(shade_drain, 120, "Sol Cross GunDelSol must drain 120 HP (60 x 2) in shade with no_hold");
    assert_eq!(sun_drain, 240, "Sol Cross GunDelSol must drain 240 HP (60 x 4) in sun with no_hold");
}

/// EXE5DS Sol Cross corner Sun gradual +20 chip attack boost:
/// - Ticks +1 every 5 frames on front damaging hand chip, capped at +20
/// - Disabled when `sun_enabled = false` (NO_SUN), while Dark MegaMan's dark chips still work indoors
/// - Continues working in Double Soul (`stats.navi == megaman`)
/// - Pauses while switched to a Team Navi (`stats.navi != megaman`).
#[test]
fn exe5_sol_cross_sun_gradual_attack_boost_and_indoor_isolation() {
    use saito_battle::input::{PlayerTick, keys};

    let content = exe5_content();
    let cannon = crate::ids::chip(&content, "exe5", "cannon").unwrap();
    let protosoul = crate::ids::form(&content, "exe5", "protosoul").unwrap();

    // 1. Outdoor (sun_enabled = true): gradual +1 every 5 ticks up to +20, works in Double Soul, pauses on Team Navi.
    let mut m = parse(
        &content,
        &exe5(&TANGO_EXE5, "party = [{ navi = \"protoman\", programs = [] }]"),
        &exe5(&TANGO_EXE5, ""),
    )
    .unwrap();
    m.sides[0]
        .set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "sol-cross-megaman"))
        .unwrap();
    m.sun_enabled = true;
    let mut setup = m.round(&content, 0x5EED);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);

    assert!(b.objects.in_order().any(|r| b.local_kind_key(r) == "sol-sun"), "sol-sun must spawn outdoors");
    b.hands[0].cursor = 0;
    b.hands[0].ids[0] = Some(cannon);
    b.hands[0].ids[1] = Some(cannon);
    b.hands[0].ids[2] = Some(cannon);
    b.hands[0].attack_bonus = [0; 6];

    // After 10 ticks, bonus should be small and positive (gradual +1 every 5 frames, not instant +20).
    for _ in 0..10 {
        b.tick(&Default::default(), Default::default());
    }
    let early = b.hands[0].attack_bonus[0];
    assert!((1..=4).contains(&early), "expected gradual boost after 10 ticks, got {early}");

    // After 120 more ticks, bonus must cap at exactly +20.
    for _ in 0..120 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(b.hands[0].attack_bonus[0], 20, "Sol Cross Sun bonus must cap at +20");

    // Works in Double Soul (MegaMan still active navi, non-base form).
    b.stats[0].form = protosoul;
    b.hands[0].cursor = 1;
    for _ in 0..120 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(b.hands[0].attack_bonus[1], 20, "Sol Cross Sun must still boost in Double Soul");

    // Switch to Team Navi (ProtoMan, member 1): boost must pause while Team Navi is active.
    ask_switch(&mut b, 1);
    for _ in 0..80 {
        b.tick(&Default::default(), Default::default());
        if navi_key(&b, 0) == "protoman" && !b.is_dimmed() && !b.paused {
            break;
        }
    }
    assert_eq!(navi_key(&b, 0), "protoman", "side 0 should have switched to ProtoMan");
    b.hands[0].cursor = 2;
    b.hands[0].attack_bonus[2] = 0;
    for _ in 0..40 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(b.hands[0].attack_bonus[2], 0, "Sol Cross Sun must pause while switched to a Team Navi");

    // 2. Indoor (sun_enabled = false): no sol-sun spawned, no attack bonus.
    let mut m_indoor = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    m_indoor.sides[0]
        .set_fact(&content, crate::facts::PATCH_CARDS, &crate::testing::patch_cards(&content, "exe5", "sol-cross-megaman"))
        .unwrap();
    m_indoor.sun_enabled = false;
    let mut setup_indoor = m_indoor.round(&content, 0x5EED);
    setup_indoor.custom_screen_limit = Some(1);
    let mut b_in = saito_battle::Battle::new(setup_indoor, content.clone());
    fight(&mut b_in);
    assert!(!b_in.objects.in_order().any(|r| b_in.local_kind_key(r) == "sol-sun"), "sol-sun must not spawn indoors");
    b_in.hands[0].cursor = 0;
    b_in.hands[0].ids[0] = Some(cannon);
    b_in.hands[0].attack_bonus[0] = 0;
    for _ in 0..40 {
        b_in.tick(&Default::default(), Default::default());
    }
    assert_eq!(b_in.hands[0].attack_bonus[0], 0, "indoor Sol Cross must grant 0 sun bonus");

    // 3. Indoor (NO_SUN = 0x0010_0000) in a link battle must NOT fizzle Dark MegaMan's Dark Chips.
    let mut m_dark = parse(&content, &exe5(&TANGO_EXE5, "karma = 0\nhp = 997"), &exe5(&TANGO_EXE5, "")).unwrap();
    m_dark.sun_enabled = false;
    let mut setup_dark = m_dark.round(&content, 0x5EED);
    setup_dark.custom_screen_limit = Some(1);
    let mut b_dark = saito_battle::Battle::new(setup_dark, content.clone());
    fight(&mut b_dark);
    let drksword = crate::ids::chip(&content, "exe5", "drksword").unwrap();
    let p1 = b_dark.player(1).unwrap();
    // Step side 0 right from (2, 2) to (3, 2) so DarkSword (2x3 ahead) reaches side 1 at (5, 2).
    b_dark.tick(&[PlayerTick { held: keys::RIGHT, ..Default::default() }, Default::default()], Default::default());
    for _ in 0..10 {
        b_dark.tick(&Default::default(), Default::default());
    }
    let hp_before = b_dark.objects.get(p1).hp;
    b_dark.hands[0].cursor = 0;
    b_dark.hands[0].ids[0] = Some(drksword);
    for t in 0..60 {
        let input = if t == 0 {
            [PlayerTick { held: keys::A, ..Default::default() }, Default::default()]
        } else {
            Default::default()
        };
        b_dark.tick(&input, Default::default());
    }
    assert!(b_dark.objects.get(p1).hp < hp_before, "Dark MegaMan's DarkSword must still hit indoors (NO_SUN must not collide with DARK_CHIPS_FIZZLE)");
}

/// EXE5DS Danger HUD and 5DS Shout gating:
/// - Pure GBA (`ds_shout_enabled = false`): `is_danger(0)` stays false and shout never triggers
/// - 5DS (`ds_shout_enabled = true`): `is_danger(0)` toggles accurately on/off with Danger conditions
/// - 3-tier volume (`mic_tier = 1 / 2 / 3`): only Tier 3 grants Full Synchro, and each tier applies
///   its corresponding boost magnitude (and shout is once per battle)
/// - `ds_shout_max_only = true`: upgrades `mic_tier = 1` to Tier 3 / Full Synchro.
#[test]
fn exe5_ds_danger_hud_and_shout_gating() {
    use saito_battle::input::PlayerTick;

    let content = exe5_content();
    let start_battle = |ds_shout_enabled: bool, ds_shout_max_only: bool| -> saito_battle::Battle {
        let mut m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
        m.ds_shout_enabled = ds_shout_enabled;
        m.ds_shout_max_only = ds_shout_max_only;
        let mut setup = m.round(&content, 0x5EED);
        setup.custom_screen_limit = Some(1);
        let mut b = saito_battle::Battle::new(setup, content.clone());
        fight(&mut b);
        b
    };

    let put_in_danger = |b: &mut saito_battle::Battle| {
        let p0 = b.player(0).unwrap();
        b.objects.get_mut(p0).hp = 200; // <= 1000 / 4 = 250
        b.stats[0].mood = 32; // worried (< 65, > 0)
        b.tick(&Default::default(), Default::default());
    };

    // 1. Pure GBA (`ds_shout_enabled = false`): Danger HUD stays false and shout never fires.
    let mut b_gba = start_battle(false, false);
    put_in_danger(&mut b_gba);
    assert!(!b_gba.is_danger(0), "pure GBA must never raise Danger HUD flag");
    b_gba.tick(&[PlayerTick { mic_tier: 3, ..Default::default() }, Default::default()], Default::default());
    assert!(!b_gba.objects.in_order().any(|r| b_gba.local_kind_key(r) == "shout"), "pure GBA must never trigger shout");
    assert!(!b_gba.is_danger(0));

    // 2. 5DS Danger HUD toggles accurately.
    let mut b_hud = start_battle(true, false);
    assert!(!b_hud.is_danger(0), "full HP must not be in Danger");
    let p0 = b_hud.player(0).unwrap();
    b_hud.objects.get_mut(p0).hp = 200;
    b_hud.stats[0].mood = 0x80; // normal, not worried
    b_hud.tick(&Default::default(), Default::default());
    assert!(!b_hud.is_danger(0), "normal mood must not be in Danger");
    b_hud.stats[0].mood = 32; // worried
    b_hud.tick(&Default::default(), Default::default());
    assert!(b_hud.is_danger(0), "low HP + worried must enter Danger");
    b_hud.objects.get_mut(p0).hp = 500; // above 1/4 HP
    b_hud.tick(&Default::default(), Default::default());
    assert!(!b_hud.is_danger(0), "healing above 1/4 HP must exit Danger");

    // 3. 3-tier volume classification (`mic_tier = 1, 2, 3`) and `ds_shout_max_only = true`.
    let run_shout = |ds_shout_max_only: bool, mic_tier: u8| -> (u8, u16, u8, bool) {
        let mut b = start_battle(true, ds_shout_max_only);
        // Set barrier & high buster levels so ROWS[4] heavily favors HEAL (entries 0..13), making
        // the heal amount (100 / 200 / 300) cleanly observable on seed 0x5EED.
        b.stats[0].attack = 4;
        b.stats[0].charge = 4;
        b.stats[0].rapid = 4;
        put_in_danger(&mut b);
        assert!(b.is_danger(0), "must be in Danger before shouting");
        let p0 = b.player(0).unwrap();
        let hp_before = b.objects.get(p0).hp;
        b.tick(&[PlayerTick { mic_tier, ..Default::default() }, Default::default()], Default::default());
        let spawned = b.objects.in_order().any(|r| b.local_kind_key(r) == "shout");
        assert!(spawned, "shout object must spawn on mic_tier={mic_tier}");
        assert!(!b.is_danger(0), "Danger HUD flag must clear immediately once shout triggers");
        for _ in 0..140 {
            b.tick(&Default::default(), Default::default());
        }
        let healed = b.objects.get(p0).hp - hp_before;
        let mood = b.stats[0].mood;
        let atk = b.stats[0].attack;
        // Verify once-per-battle limit: dropping back to Danger and shouting again does nothing.
        b.objects.get_mut(p0).hp = 100;
        b.stats[0].mood = 32;
        b.tick(&Default::default(), Default::default());
        let danger_after_spent = b.is_danger(0);
        b.tick(&[PlayerTick { mic_tier: 3, ..Default::default() }, Default::default()], Default::default());
        assert!(!b.objects.in_order().any(|r| b.local_kind_key(r) == "shout"), "shout must only trigger once per battle");
        (mood, healed, atk, danger_after_spent)
    };

    let (mood1, healed1, _atk1, spent1) = run_shout(false, 1);
    let (mood2, healed2, _atk2, spent2) = run_shout(false, 2);
    let (mood3, healed3, _atk3, spent3) = run_shout(false, 3);
    let (mood_max, healed_max, _atk_max, spent_max) = run_shout(true, 1);

    assert!(!spent1 && !spent2 && !spent3 && !spent_max, "Danger must stay false after shout is spent");
    assert_ne!(mood1, 0xFF, "Tier 1 shout must NOT grant Full Synchro");
    assert_ne!(mood2, 0xFF, "Tier 2 shout must NOT grant Full Synchro");
    assert_eq!(mood3, 0xFF, "Tier 3 shout MUST grant Full Synchro");
    assert_eq!(mood_max, 0xFF, "ds_shout_max_only must upgrade Tier 1 shout to Full Synchro");
    assert_eq!((healed1, healed2, healed3, healed_max), (100, 200, 300, 300), "Tier 1/2/3/MaxOnly heal amounts must be 100/200/300/300");
}

/// EXE5DS StatusGuard scoping & pure GBA isolation:
/// - TomahawkMan: takes hit statuses in pure GBA (`ds_shout_enabled = false`), immune in 5DS (`true`).
/// - TomahawkSoul & MegaMan with a StatusGuard patch card (`woodman`): immune in both GBA and 5DS.
/// - MegaMan with a plain WoodBody patch card without StatusGuard (`popper`) and base MegaMan:
///   take hit statuses in both GBA and 5DS.
#[test]
fn exe5_ds_status_guard_scoped_and_isolated_from_gba() {
    use saito_battle::collision::timer;
    use saito_battle::content::StatusRole;

    let content = exe5_content();
    let paralyze = content.defs.roles().status(StatusRole::CounterParalysis);
    let tomahawksoul = crate::ids::form(&content, "exe5", "tomahawksoul").unwrap();

    let test_paralyzed = |side_toml: &str, ds_rules: bool, force_form: Option<saito_content_api::FormHandle>| -> bool {
        let mut m = parse(&content, side_toml, &exe5(&TANGO_EXE5, "")).unwrap();
        m.ds_shout_enabled = ds_rules;
        let mut setup = m.round(&content, 0x5EED);
        setup.custom_screen_limit = Some(1);
        let mut b = saito_battle::Battle::new(setup, content.clone());
        fight(&mut b);
        if let Some(form) = force_form {
            b.stats[0].form = form;
        }
        let p0 = b.player(0).unwrap();
        let cid = b.objects.get(p0).collision.unwrap();
        b.collision.get_mut(cid).status_final = Some(paralyze);
        b.tick(&Default::default(), Default::default());
        b.collision.get(cid).status_timers[timer::PARALYZE] > 0
    };

    let tomahawkman = side("tomahawkman", &TANGO_EXE5, "");
    let status_guard_megaman = exe5(&TANGO_EXE5, "patch_cards = [{ card = \"woodman\" }]");
    let woodbody_only_megaman = exe5(&TANGO_EXE5, "patch_cards = [{ card = \"popper\" }]");
    let base_megaman = exe5(&TANGO_EXE5, "");

    assert!(test_paralyzed(&tomahawkman, false, None), "GBA TomahawkMan is not status-immune");
    assert!(!test_paralyzed(&tomahawkman, true, None), "5DS TomahawkMan has StatusGuard");
    assert!(!test_paralyzed(&base_megaman, false, Some(tomahawksoul)), "GBA TomahawkSoul has StatusGuard");
    assert!(!test_paralyzed(&base_megaman, true, Some(tomahawksoul)), "5DS TomahawkSoul has StatusGuard");
    assert!(!test_paralyzed(&status_guard_megaman, false, None), "GBA StatusGuard patch card grants immunity");
    assert!(!test_paralyzed(&status_guard_megaman, true, None), "5DS StatusGuard patch card grants immunity");
    assert!(test_paralyzed(&woodbody_only_megaman, false, None), "GBA plain WoodBody card has no StatusGuard");
    assert!(test_paralyzed(&woodbody_only_megaman, true, None), "5DS plain WoodBody card without StatusGuard does not gain immunity");
    assert!(test_paralyzed(&base_megaman, false, None), "GBA base MegaMan takes status");
    assert!(test_paralyzed(&base_megaman, true, None), "5DS base MegaMan takes status");
}

/// EXE5DS solo Team Navi dark-only chip restriction (JP `0x020070AC` / US `0x020070FC`) vs pure GBA:
/// - In pure GBA (`ds_shout_enabled = false`), a solo Team Navi (ProtoMan, no partners) may use dark-only
///   chips such as Anubis (`megaman = "dark"`).
/// - In 5DS (`ds_shout_enabled = true`), solo Team Navis are blocked from dark-only chips (become `invalid`).
#[test]
fn exe5_ds_solo_team_navi_dark_chip_restriction_isolated_from_gba() {
    use saito_battle::input::{PlayerTick, keys};

    let content = exe5_content();
    let anubis = crate::ids::chip(&content, "exe5", "anubis").unwrap();
    let protoman = side("protoman", &TANGO_EXE5, "");

    let spawns_anubis = |ds_rules: bool| -> bool {
        let mut m = parse(&content, &protoman, &exe5(&TANGO_EXE5, "")).unwrap();
        m.ds_shout_enabled = ds_rules;
        let mut setup = m.round(&content, 0x5EED);
        setup.custom_screen_limit = Some(1);
        let mut b = saito_battle::Battle::new(setup, content.clone());
        fight(&mut b);
        b.hands[0].ids = [Some(anubis), None, None, None, None, None];
        b.hands[0].cursor = 0;
        b.tick(&[PlayerTick::buttons(keys::A), PlayerTick::buttons(0)], Default::default());
        for _ in 0..80 {
            b.tick(&Default::default(), Default::default());
            if b.objects.in_order().any(|r| b.local_kind_key(r).contains("anubis")) {
                return true;
            }
        }
        false
    };

    assert!(spawns_anubis(false), "GBA solo Team Navi may use dark-only chips (Anubis)");
    assert!(!spawns_anubis(true), "5DS solo Team Navi is blocked from dark-only chips");
}
