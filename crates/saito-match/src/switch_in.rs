//! A team navi switched in mid-fight (EXE5's party battle: the DS's action
//! 0x59) is the navi it is when it starts the battle: what its init, its
//! hooks and its switch's tail give it.

use crate::double_attacks::fighting;
use crate::testing::exe5_content;
use saito_battle::Battle;
use saito_battle::actor::status;
use saito_battle::collision::f1;
use saito_battle::input::PlayerTick;

/// Side 0 asks to switch to `member` (out of the normal mood: no Double
/// Attack), then 80 ticks of nothing: the switch lands and settles.
fn switch_to(b: &mut Battle, member: u8) {
    b.stats[0].mood = 0x80;
    let ask = PlayerTick { switch_to: Some(member), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    for _ in 0..80 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(b.party[0].active, member, "the switch landed");
}

fn navi_status(b: &Battle) -> u32 {
    let a = b.objects.get(b.player(0).unwrap()).actor.expect("the navi's actor");
    b.actors.get(a).status
}

fn flags(b: &Battle) -> u32 {
    let c = b.objects.get(b.player(0).unwrap()).collision.expect("the navi's collision");
    b.collision.get(c).f1
}

/// Side 0's navi's panel turned to `kind`, then 10 ticks.
fn panel_under(b: &mut Battle, kind: &str) {
    let c = b.objects.get(b.player(0).unwrap()).collision.expect("the navi's collision");
    let p = b.collision.get(c).panel;
    let t = b.content.rules().panels.named(kind).unwrap_or_else(|| panic!("EXE5's {kind}"));
    b.set_panel_type(p.x, p.y, t);
    for _ in 0..10 {
        b.tick(&Default::default(), Default::default());
    }
}

fn sea_under(b: &mut Battle) {
    panel_under(b, "sea");
}

/// ToadMan dives (his post-init hook; the switch's tail clears the bit and
/// his entry in its table sets it again, 0x080119A0, the DS's 0x0206E994):
/// switched in, he goes under on sea, as he does when he starts the battle
/// (the DS's G-OTOSHI recording, frame 852). MegaMan switched back in
/// doesn't.
#[test]
fn toadman_switched_in_dives_on_sea() {
    let content = exe5_content();
    let mut b = fighting(&content, &["toadman"]);
    switch_to(&mut b, 1);
    assert_ne!(navi_status(&b) & status::DIVES, 0, "ToadMan dives");
    sea_under(&mut b);
    assert_ne!(flags(&b) & f1::BUBBLED, 0, "under the sea's surface");
    // (Switched out from under the surface, on the sea.)
    switch_to(&mut b, 0);
    assert_eq!(navi_status(&b) & status::DIVES, 0, "MegaMan doesn't dive");
    sea_under(&mut b);
    assert_eq!(flags(&b) & f1::BUBBLED, 0, "MegaMan stands on the sea");
    let ripple = b.content.defs.kind_by_key("dive-ripple").expect("EXE5's dive ripple");
    assert!(b.objects.in_order().all(|q| b.objects.get(q).kind != ripple), "and no ripple is left over him");
}

/// The navi `navi` starting the battle (side 0's own navi), fighting.
fn starting_as(content: &std::sync::Arc<saito_battle::Content>, navi: &str) -> Battle {
    let megaman = crate::double_attacks::side(&[]);
    let left = megaman.replacen("navi = \"megaman\"", &format!("navi = \"{navi}\""), 1).replace("party = []\n", "");
    fighting_sides(content, &left, &megaman)
}

/// A round of the sides `left` and `right` (a match file's), both pressing
/// through their custom screens until the fight runs.
fn fighting_sides(content: &std::sync::Arc<saito_battle::Content>, left: &str, right: &str) -> Battle {
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(content, "exe5")[0]).key);
    let text = format!("game = \"exe5\"\n\n[[round]]\nstage = \"{stage}\"\n\n[left]\n{left}\n\n[right]\n{right}\n");
    let m = crate::parse(content, &text).unwrap_or_else(|p| panic!("{p:?}"));
    let mut b = Battle::new(m.round(content, 0x5EED), content.clone());
    let mut last = [None; 2];
    for _ in 0..3000 {
        if b.round.mode == saito_battle::battle::mode::FIGHTING && b.fight.state == saito_battle::battle::fight::FIGHTING {
            return b;
        }
        let input = [0, 1].map(|side| {
            last[side] = saito_battle::scenario::custom_choice(&b, side, last[side]);
            PlayerTick::buttons(0).with_choice(last[side])
        });
        b.tick(&input, Default::default());
    }
    panic!("the fight never started");
}

/// The navi's state as it stands, beside what a switch leaves on it (its
/// status bit `SWITCHED`): its identity, sprite look, collision, statuses,
/// weapons and element, and the objects it owns.
fn state(b: &Battle) -> Vec<String> {
    let r = b.player(0).unwrap();
    let o = b.objects.get(r);
    let a = b.actors.get(o.actor.unwrap());
    let owned = b
        .objects
        .in_order()
        .filter(|&q| q != r && b.objects.get(q).related.contains(&Some(r)))
        .map(|q| b.content.defs.kind(b.objects.get(q).kind).key.to_string())
        .collect::<Vec<_>>();
    vec![
        format!("identity {:?}", o.identity),
        // (Its look, but how long its animation has played, which time alone tells apart.)
        format!("look {:?}", saito_battle::object::sprite::Look { part_palette: b.objects.sprite(r).look.part_palette.map(|p| saito_battle::object::sprite::Place { steps: 0, ..p }), ..b.objects.sprite(r).look }),
        format!("collision {:?}", b.collision.get(o.collision.unwrap())),
        format!("status {:#x}", a.status & !status::SWITCHED),
        format!("weapons {:?}", (a.buster, a.charge_shot, a.a_charge, a.alt_a_charge, a.back_special, a.ai_index, a.identity)),
        format!("element {}", b.stats[0].element),
        format!("owned {owned:?}"),
    ]
}

/// The script both battles play from where they are compared: tap B (the
/// buster), hold B to a full charge and let go (the charge shot), step up
/// and back, tap A (the hand's first chip): each tick's buttons.
fn script() -> Vec<u16> {
    use saito_battle::input::keys;
    let mut s = Vec::new();
    let hold = |key: u16, ticks: usize, s: &mut Vec<u16>| {
        s.extend(std::iter::repeat_n(key, ticks));
        s.extend(std::iter::repeat_n(0, 40));
    };
    hold(keys::B, 2, &mut s);
    hold(keys::B, 240, &mut s);
    s.extend(std::iter::repeat_n(0, 120));
    hold(keys::UP, 2, &mut s);
    hold(keys::DOWN, 2, &mut s);
    hold(keys::A, 2, &mut s);
    s.extend(std::iter::repeat_n(0, 200));
    s
}

/// What a navi did through the script: the kinds it spawned (in order of
/// first appearance), its sounds and the ticks the opponent was hit on.
/// (Not the damage: a party's team navi deals the DS's, by its party's
/// level, lib/navi_level.)
fn plays(b: &mut Battle) -> (Vec<String>, Vec<String>, Vec<usize>) {
    let opponent = |b: &Battle| b.objects.get(b.player(1).unwrap()).hp;
    let mut last = opponent(b);
    let (mut kinds, mut sounds, mut hits) = (Vec::new(), Vec::new(), Vec::new());
    for (i, held) in script().into_iter().enumerate() {
        b.tick(&[PlayerTick::buttons(held), Default::default()], Default::default());
        for q in b.objects.in_order() {
            let key = b.content.defs.kind(b.objects.get(q).kind).key.to_string();
            if !kinds.contains(&key) {
                kinds.push(key);
            }
        }
        sounds.extend(b.sound_cues_for(0).iter().map(|c| format!("{c:?}")));
        if opponent(b) != last {
            hits.push(i);
            last = opponent(b);
        }
    }
    (kinds, sounds, hits)
}

const TEAM_NAVIS: [&str; 12] = [
    "protoman", "gyroman", "knightman", "magnetman", "meddy", "napalmman", "numberman", "searchman", "shadowman", "toadman",
    "tomahawkman", "colonel",
];

/// Each team navi switched in mid-fight is the navi it is when it starts
/// the battle (its look, collision, statuses, weapons and the objects it
/// wears), plays as it does (the same buster, charge shot, steps and chip:
/// the same objects, sounds and hits), and takes the sea and a broken
/// panel under it as it does; switched in from another team navi, too.
/// MegaMan switched back in is MegaMan as he started.
#[test]
fn a_team_navi_switched_in_is_as_at_the_start() {
    let content = exe5_content();
    let mut wrong = Vec::new();
    let mut megaman = fighting(&content, &[]);
    for _ in 0..162 {
        megaman.tick(&Default::default(), Default::default());
    }
    let mut megaman_on_broken = megaman.clone();
    panel_under(&mut megaman_on_broken, "broken");
    for _ in 0..81 {
        megaman_on_broken.tick(&Default::default(), Default::default());
    }
    for navi in TEAM_NAVIS {
        let mut a = starting_as(&content, navi);
        for _ in 0..81 {
            a.tick(&Default::default(), Default::default());
        }
        let mut s = fighting(&content, &[navi]);
        switch_to(&mut s, 1);
        if state(&a) != state(&s) {
            wrong.push(format!("{navi}'s state:\n  at the start {:?}\n  switched in  {:?}", state(&a), state(&s)));
        }
        let (at_start, switched) = (plays(&mut a), plays(&mut s));
        if at_start != switched {
            wrong.push(format!("{navi}'s play:\n  at the start {at_start:?}\n  switched in  {switched:?}"));
        }
        // What the panel under it does to it (ToadMan dives in the sea,
        // GyroMan hovers over a broken panel).
        for kind in ["sea", "broken", "normal"] {
            panel_under(&mut a, kind);
            panel_under(&mut s, kind);
            if state(&a) != state(&s) {
                wrong.push(format!("{navi} on {kind}:\n  at the start {:?}\n  switched in  {:?}", state(&a), state(&s)));
            }
        }
        // From another team navi (the next in the list) rather than MegaMan.
        let other = TEAM_NAVIS[(TEAM_NAVIS.iter().position(|&n| n == navi).unwrap() + 1) % TEAM_NAVIS.len()];
        let mut from_other = fighting(&content, &[other, navi]);
        switch_to(&mut from_other, 1);
        switch_to(&mut from_other, 2);
        let mut a = starting_as(&content, navi);
        for _ in 0..161 {
            a.tick(&Default::default(), Default::default());
        }
        if state(&a) != state(&from_other) {
            wrong.push(format!("{navi} from {other}:\n  at the start {:?}\n  switched in  {:?}", state(&a), state(&from_other)));
        }
        let mut back = fighting(&content, &[navi]);
        switch_to(&mut back, 1);
        switch_to(&mut back, 0);
        if state(&back) != state(&megaman) {
            wrong.push(format!("MegaMan back from {navi}:\n  at the start {:?}\n  back  {:?}", state(&megaman), state(&back)));
        }
        // Switched out over a broken panel (GyroMan hovering).
        let mut back = fighting(&content, &[navi]);
        switch_to(&mut back, 1);
        panel_under(&mut back, "broken");
        switch_to(&mut back, 0);
        if state(&back) != state(&megaman_on_broken) {
            wrong.push(format!(
                "MegaMan back from {navi} on a broken panel:\n  standing there {:?}\n  back  {:?}",
                state(&megaman_on_broken),
                state(&back)
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// GyroMan switched in hovers over a broken panel (his `tick`, 0x080F0A10),
/// as he does when he starts the battle; MegaMan switched in there doesn't
/// (the settle clears the bit, 0x0202C8F8).
#[test]
fn gyroman_switched_in_hovers_over_a_broken_panel() {
    let content = exe5_content();
    let mut b = fighting(&content, &["gyroman"]);
    switch_to(&mut b, 1);
    panel_under(&mut b, "broken");
    assert_ne!(navi_status(&b) & status::HOVERING, 0, "GyroMan hovers");
    switch_to(&mut b, 0);
    assert_eq!(navi_status(&b) & status::HOVERING, 0, "MegaMan doesn't");
}

/// A dark MegaMan switches in battle as a light one does: EXE5 DS's switch
/// paths read no emotion and no light/dark value (the input handler's
/// request, 0x0203C5F6, and 0x0200E064; the bottom screen's panel,
/// 0x02036F80; action 0x59, 0x0202C67C), only its Double Attacks and
/// supports do. MegaMan of a dark value (evil from his start: mood 0) uses
/// DarkPlus (a dark chip that doesn't dim, its cost the step bug), is still
/// evil, switches to ProtoMan, and ProtoMan back to him.
#[test]
fn a_dark_megaman_switches() {
    use saito_battle::input::keys;
    use saito_battle::kinds::player::emotion_name;
    let content = exe5_content();
    let left = crate::double_attacks::side(&["protoman"]) + "karma = 0\n";
    let mut b = fighting_sides(&content, &left, &crate::double_attacks::side(&[]));
    assert_eq!(emotion_name(&b, 0), "evil", "a dark value starts him evil");
    b.hands[0].ids[0] = crate::ids::chip(&content, "exe5", "darkplus");
    b.hands[0].cursor = 0;
    b.tick(&[PlayerTick::buttons(keys::A), Default::default()], Default::default());
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!((b.hands[0].cursor, b.stats[0].bugs.processing), (1, 1), "DarkPlus used, its step bug his");
    assert_eq!(emotion_name(&b, 0), "evil");
    let ask = |b: &mut Battle, member: u8| {
        b.tick(&[PlayerTick { switch_to: Some(member), ..Default::default() }, Default::default()], Default::default());
        for _ in 0..80 {
            b.tick(&Default::default(), Default::default());
        }
    };
    ask(&mut b, 1);
    assert_eq!((b.party[0].active, b.party[0].switches), (1, 1), "the dark MegaMan switched to ProtoMan");
    ask(&mut b, 0);
    assert_eq!((b.party[0].active, b.party[0].switches), (0, 2), "and back to him");
    assert_eq!(emotion_name(&b, 0), "evil", "his mood kept in his block");
}

/// A switch to a member whose support acts (EXE5 DS's busy flag, the
/// party block's +0xC + member, 0x02004B80) is refused in battle with the
/// refusal's sound on both consoles (0x0203C62E: the navi's input, which
/// each console runs). Benched NapalmMan with P.Spport comes out to fire;
/// asked for meanwhile, he isn't switched in.
#[test]
fn a_switch_to_a_supporting_member_is_refused() {
    use saito_battle::content::SoundRole;
    let content = exe5_content();
    let megaman = crate::double_attacks::side(&[]);
    let left = megaman.replace("party = []\n", "party = [{ navi = \"napalmman\", programs = [\"p-spport\"] }]\n");
    let mut b = fighting_sides(&content, &left, &megaman);
    let support = b.content.defs.kind_by_key("napalmman/support").expect("NapalmMan's support");
    let out = |b: &Battle| b.objects.in_order().any(|q| b.objects.get(q).kind == support);
    for _ in 0..600 {
        if out(&b) && !b.is_dimmed() {
            break;
        }
        b.tick(&Default::default(), Default::default());
    }
    assert!(out(&b), "NapalmMan came out to support");
    b.tick(&[PlayerTick { switch_to: Some(1), ..Default::default() }, Default::default()], Default::default());
    let refused = saito_battle::SoundCue::Effect(b.roles().sound(SoundRole::Refused));
    assert!(out(&b), "still out as the request came");
    assert!([0, 1].iter().all(|&s| b.sound_cues_for(s).contains(&refused)), "the refusal heard on both consoles");
    for _ in 0..80 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!((b.party[0].active, b.party[0].switches), (0, 0), "no switch");
}

/// KnightMan's guard request and `guarded` state are cleared whenever the active
/// navi switches (whether into another partner or into KnightMan himself), so
/// neither the guard action nor the support actor sticks after switching.
#[test]
fn knightman_guard_does_not_stick_across_navi_switch() {
    use saito_battle::actor::request;
    use saito_battle::input::keys;
    let content = exe5_content();
    let guard_actor = content.defs.kind_by_key("knightman/guard").expect("KnightMan's guard");

    // 1. Move MegaMan to his area's front edge, trigger `guard.action`, then switch to ProtoMan while guarded.
    let mut b = fighting(&content, &["knightman", "protoman"]);
    for _ in 0..16 {
        b.tick(&[PlayerTick::buttons(keys::RIGHT), Default::default()], Default::default());
    }
    {
        let r = b.player(0).unwrap();
        let a = b.objects.get(r).actor.unwrap();
        b.actors.get_mut(a).requests |= request::GUARD;
    }
    for _ in 0..10 {
        b.tick(&Default::default(), Default::default());
    }
    assert!(b.objects.in_order().any(|q| b.objects.get(q).kind == guard_actor), "guard actor spawned");
    switch_to(&mut b, 2);
    // Allow warp-out animation of the dismissed support actor to complete.
    for _ in 0..30 {
        b.tick(&Default::default(), Default::default());
    }
    let r = b.player(0).unwrap();
    let a = b.objects.get(r).actor.unwrap();
    assert_eq!(b.actors.get(a).requests & request::GUARD, 0, "guard request cleared after switch");
    assert!(!b.objects.in_order().any(|q| b.objects.get(q).kind == guard_actor), "guard actor warped out after switch");
    // ProtoMan can move freely (not trapped in guard).
    let y_before = b.collision.get(b.objects.get(r).collision.unwrap()).panel.y;
    let target_key = if y_before == 0 { keys::DOWN } else { keys::UP };
    for _ in 0..16 {
        b.tick(&[PlayerTick::buttons(target_key), Default::default()], Default::default());
    }
    let y_after = b.collision.get(b.objects.get(b.player(0).unwrap()).collision.unwrap()).panel.y;
    assert_ne!(y_after, y_before, "switched navi can move after guard interrupted");

    // 2. Pending `request::GUARD` at switch time into KnightMan himself must not stick.
    let mut b = fighting(&content, &["knightman"]);
    {
        let r = b.player(0).unwrap();
        let a = b.objects.get(r).actor.unwrap();
        b.actors.get_mut(a).requests |= request::GUARD;
    }
    b.stats[0].mood = 0x80;
    let ask = PlayerTick { switch_to: Some(1), ..Default::default() };
    b.tick(&[ask, Default::default()], Default::default());
    for _ in 0..80 {
        b.tick(&Default::default(), Default::default());
    }
    assert_eq!(b.party[0].active, 1, "switched to KnightMan");
    let r = b.player(0).unwrap();
    let a = b.objects.get(r).actor.unwrap();
    assert_eq!(b.actors.get(a).requests & request::GUARD, 0, "guard request cleared on KnightMan switch");
    assert!(!b.objects.in_order().any(|q| b.objects.get(q).kind == guard_actor), "KnightMan never spawns a self-guard actor");
}

fn finish_custom_to_fight(b: &mut Battle) {
    let mut last = [None; 2];
    for _ in 0..3000 {
        if b.round.mode == saito_battle::battle::mode::FIGHTING
            && b.fight.state == saito_battle::battle::fight::FIGHTING
            && !b.is_dimmed()
        {
            return;
        }
        let input = [0, 1].map(|side| {
            last[side] = saito_battle::scenario::custom_choice(b, side, last[side]);
            PlayerTick::buttons(0).with_choice(last[side])
        });
        b.tick(&input, Default::default());
    }
    panic!("custom screen never returned to fight");
}

fn open_custom_screen(b: &mut Battle) {
    use saito_battle::custom::screen::Phase;
    use saito_battle::input::keys;
    for t in 0..600 {
        if b.round.mode == saito_battle::battle::mode::CUSTOM
            && b.custom.sides.iter().all(|s| {
                s.screen.as_ref().is_some_and(|scr| scr.phase == Phase::Choosing)
            })
        {
            return;
        }
        let press_l = if b.round.mode == saito_battle::battle::mode::FIGHTING {
            b.gauge.value = saito_battle::hud::CustomGauge::FULL;
            if t % 2 == 1 { keys::L } else { 0 }
        } else {
            0
        };
        b.tick(&[PlayerTick::buttons(press_l), Default::default()], Default::default());
    }
    panic!("custom screen never reached Choosing (mode={:?})", b.round.mode);
}

fn custom_switch_side0(b: &mut Battle, member: u8) {
    let expected_navi = b.party[0].member(member).expect("party member").navi;
    for tick in 0..60 {
        let ask = if tick == 0 {
            PlayerTick { switch_to: Some(member), ..Default::default() }
        } else {
            PlayerTick::default()
        };
        b.tick(&[ask, Default::default()], Default::default());
        let expected_window = (expected_navi != b.stats[0].navi).then_some(expected_navi);
        if b.window_navis[0] == expected_window
            && b.custom.sides[0]
                .screen
                .as_ref()
                .is_some_and(|scr| matches!(scr.phase, saito_battle::custom::Phase::Choosing))
        {
            return;
        }
    }
    panic!("custom screen switch to member {member} did not complete");
}

fn wait_in_fight_switch_settled(b: &mut Battle, expected_member: u8) {
    for _ in 0..120 {
        b.tick(&Default::default(), Default::default());
        let idle = b
            .player(0)
            .is_some_and(|r| saito_battle::kinds::player::running_content_action(b, r).is_none());
        if b.party[0].active == expected_member && !b.is_dimmed() && idle {
            return;
        }
    }
    panic!("switch to member {expected_member} never settled");
}

/// Evil MegaMan (`karma = 0`) keeps `mood == 0` in `members[0]` at round start and
/// across custom-screen switches (same-turn and cross-turn), in-battle switches,
/// teammate KO take-back, and Light MegaMan at `mood == 0`.
#[test]
fn evil_megaman_party_switch_paths_preserve_evil_mood_and_palette() {
    use saito_battle::input::keys;
    use saito_battle::kinds::player::emotion_name;

    let content = exe5_content();
    let light_side = crate::double_attacks::side(&["searchman", "protoman"]) + "karma = 500\n";
    let light_b = fighting_sides(&content, &light_side, &crate::double_attacks::side(&[]));
    let light_palette = light_b.objects.sprite(light_b.player(0).unwrap()).look.palette;

    // 1. Initial spawn with karma = 0 syncs both b.stats[0].mood and members[0].mood to 0.
    let stage = crate::ids::local(&content.defs.stage(crate::link_battle_stages(&content, "exe5")[0]).key);
    let evil_side = crate::double_attacks::side(&["searchman", "protoman"]) + "karma = 0\n";
    let text = format!(
        "game = \"exe5\"\n\n[[round]]\nstage = \"{stage}\"\n\n[left]\n{evil_side}\n\n[right]\n{}\n",
        crate::double_attacks::side(&[])
    );
    let m = crate::parse(&content, &text).unwrap_or_else(|p| panic!("{p:?}"));
    let new_evil_battle = || Battle::new(m.round(&content, 0x5EED), content.clone());
    let assert_darkplus_works = |b: &mut Battle| {
        b.hands[0].ids[0] = crate::ids::chip(&content, "exe5", "darkplus");
        b.hands[0].cursor = 0;
        b.tick(&[PlayerTick::buttons(keys::A), Default::default()], Default::default());
        for _ in 0..40 {
            b.tick(&Default::default(), Default::default());
        }
        assert_eq!(b.stats[0].bugs.processing, 1, "dark-only chip still works after switching back");
    };

    // 1. Turn 1 initial spawn + same-turn custom-screen switch (MegaMan -> SearchMan -> MegaMan).
    let mut b = new_evil_battle();
    open_custom_screen(&mut b);
    assert_eq!(b.stats[0].mood, 0, "evil MegaMan starts with live mood 0");
    assert_eq!(b.party[0].members[0].mood, 0, "members[0].mood must also start at 0 before first switch");
    custom_switch_side0(&mut b, 1);
    custom_switch_side0(&mut b, 0);
    assert_eq!(b.window_navis[0], None);
    finish_custom_to_fight(&mut b);

    let r0 = b.player(0).unwrap();
    let evil_palette = b.objects.sprite(r0).look.palette;
    assert_ne!(evil_palette, light_palette, "evil MegaMan uses dark palette");
    assert_eq!((b.stats[0].mood, b.party[0].members[0].mood), (0, 0));
    assert_eq!(emotion_name(&b, 0), "evil");

    // 2. Cross-turn custom-screen switch: MegaMan -> SearchMan (Turn 1) -> MegaMan (Turn 2).
    let mut b = new_evil_battle();
    open_custom_screen(&mut b);
    custom_switch_side0(&mut b, 1);
    let searchman = b.party[0].members[1].navi;
    let megaman = b.party[0].members[0].navi;
    assert_eq!(b.window_navis[0], Some(searchman));
    finish_custom_to_fight(&mut b);
    assert_eq!((b.party[0].active, b.stats[0].navi, b.stats[0].mood), (1, searchman, 0x80));
    assert_eq!(b.party[0].members[0].mood, 0);

    open_custom_screen(&mut b);
    custom_switch_side0(&mut b, 0);
    assert_eq!(b.window_navis[0], Some(megaman), "custom screen previews MegaMan while SearchMan is live");
    assert_eq!(b.party[0].members[0].mood, 0, "benched MegaMan keeps mood 0");
    finish_custom_to_fight(&mut b);
    assert_eq!(b.party[0].active, 0);
    assert_eq!((b.stats[0].navi, b.stats[0].mood, b.party[0].members[0].mood), (megaman, 0, 0));
    assert_eq!(emotion_name(&b, 0), "evil");
    assert_eq!(b.objects.sprite(b.player(0).unwrap()).look.palette, evil_palette);
    assert_darkplus_works(&mut b);

    // 3. In-battle switch MegaMan -> SearchMan -> MegaMan, and 4. Teammate KO take_back
    //    (within EXE5's 3-switch limit per battle).
    let mut b = new_evil_battle();
    open_custom_screen(&mut b);
    finish_custom_to_fight(&mut b);
    b.tick(&[PlayerTick { switch_to: Some(1), ..Default::default() }, Default::default()], Default::default());
    wait_in_fight_switch_settled(&mut b, 1);
    assert_eq!((b.stats[0].navi, b.stats[0].mood, b.party[0].members[0].mood), (searchman, 0x80, 0));

    b.tick(&[PlayerTick { switch_to: Some(0), ..Default::default() }, Default::default()], Default::default());
    wait_in_fight_switch_settled(&mut b, 0);
    assert_eq!((b.stats[0].navi, b.stats[0].mood), (megaman, 0));
    assert_eq!(emotion_name(&b, 0), "evil");
    assert_eq!(b.objects.sprite(b.player(0).unwrap()).look.palette, evil_palette);
    assert_darkplus_works(&mut b);

    // 4. Teammate KO take_back: switch to SearchMan (switch #3), KO him with opponent Cannon, verify Evil MegaMan returns.
    b.tick(&[PlayerTick { switch_to: Some(1), ..Default::default() }, Default::default()], Default::default());
    wait_in_fight_switch_settled(&mut b, 1);
    let sr = b.player(0).unwrap();
    b.objects.get_mut(sr).hp = 10;
    // Align opponent vertically and fire Cannon.
    let sy = b.objects.get(sr).panel.y;
    let enemy = b.player(1).unwrap();
    {
        let e = b.objects.get_mut(enemy);
        e.panel.y = sy;
        e.future_panel.y = sy;
    }
    saito_battle::kinds::common::set_coordinates_from_panels(&mut b, enemy);
    b.hands[1].ids[0] = crate::ids::chip(&content, "exe5", "cannon");
    b.hands[1].damage[0] = 40;
    b.hands[1].cursor = 0;
    b.tick(&[Default::default(), PlayerTick::buttons(keys::A)], Default::default());
    for _ in 0..200 {
        b.tick(&Default::default(), Default::default());
        if b.party[0].active == 0 && !b.is_dimmed() {
            break;
        }
    }
    assert_eq!(b.party[0].active, 0, "KO take_back returned MegaMan");
    assert_eq!((b.stats[0].navi, b.stats[0].mood), (megaman, 0));
    assert_eq!(emotion_name(&b, 0), "evil");
    assert_eq!(b.objects.sprite(b.player(0).unwrap()).look.palette, evil_palette);

    // 5. Light MegaMan whose mood reached 0 keeps mood 0 across partner switch and return.
    let mut light_b = fighting_sides(&content, &light_side, &crate::double_attacks::side(&[]));
    light_b.stats[0].mood = 0;
    assert_eq!(emotion_name(&light_b, 0), "evil");
    light_b.tick(&[PlayerTick { switch_to: Some(1), ..Default::default() }, Default::default()], Default::default());
    wait_in_fight_switch_settled(&mut light_b, 1);
    assert_eq!(light_b.party[0].members[0].mood, 0);
    light_b.tick(&[PlayerTick { switch_to: Some(0), ..Default::default() }, Default::default()], Default::default());
    wait_in_fight_switch_settled(&mut light_b, 0);
    assert_eq!(light_b.stats[0].mood, 0);
    assert_eq!(emotion_name(&light_b, 0), "evil");
}

