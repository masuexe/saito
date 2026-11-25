//! Regression tests for destructive, gameplay, and visual glitch fixes across EXE4, EXE5, and EXE6.

use super::*;
use saito_battle::battle::battle_flags;
use saito_battle::dimming::DimmingState;
use saito_battle::input::{PlayerTick, keys};

/// Helper: start a live battle for `game` and advance to fighting mode, clearing random NaviCust supports.
fn start_fight(content: &Arc<Content>, game: &str) -> saito_battle::Battle {
    let mut m = crate::pick::live(content, game, 1, None).unwrap();
    let megaman = crate::ids::navi(content, game, "megaman").unwrap();
    for s in &mut m.sides {
        s.set_fact(content, "navi", &[Fact::Value(Value::Def(Registry::Navi, megaman.0))]).unwrap();
    }
    let mut setup = m.round(content, 0x1234);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);
    for side in 0..2 {
        b.stats[side].support = None;
        b.stats[side].bugs = Default::default();
    }
    b
}

/// Advance until no dimming is active (at least one tick).
fn wait_undimmed(b: &mut saito_battle::Battle) {
    for _ in 0..300 {
        b.tick(&Default::default(), Default::default());
        if !b.is_dimmed() && b.dimming.iter().all(|d| d.state == DimmingState::Idle) {
            break;
        }
    }
    assert!(!b.is_dimmed(), "dimming did not finish in 300 ticks");
}

/// Put a single chip into `side`'s hand at cursor 0 with explicit base damage and Atk+ bonus.
fn give_chip(b: &mut saito_battle::Battle, side: usize, chip: saito_content_api::ChipHandle, damage: u16, bonus: u16) {
    b.hands[side].cursor = 0;
    b.hands[side].ids[0] = Some(chip);
    b.hands[side].ids[1] = None;
    b.hands[side].damage[0] = damage;
    b.hands[side].attack_bonus[0] = bonus;
    b.hands[side].charge_bonus[0] = 0;
    b.hands[side].modifiers[0] = 0;
}

/// EXE4: Full Synchro (2x) Roll hitting opponent's AntiRecv must NOT deal 49,152+ instant-kill damage.
#[test]
fn exe4_roll_2x_anti_recovery_does_not_instant_kill() {
    let content = exe4_content();
    let mut m = crate::pick::live(&content, "exe4", 1, None).unwrap();
    let megaman = crate::ids::navi(&content, "exe4", "megaman").unwrap();
    for s in &mut m.sides {
        s.set_fact(&content, "navi", &[Fact::Value(Value::Def(Registry::Navi, megaman.0))]).unwrap();
        s.set_fact(&content, "karma", &number(500)).unwrap();
    }
    let mut setup = m.round(&content, 0x1234);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);
    for side in 0..2 {
        b.stats[side].support = None;
        b.stats[side].bugs = Default::default();
    }
    let roll = crate::ids::chip(&content, "exe4", "roll").unwrap();
    let antirecv = crate::ids::chip(&content, "exe4", "antirecv").unwrap();

    // Side 1 sets AntiRecv; Side 0 enters Full Synchro (mood 0xFF) and uses Roll (base 20).
    give_chip(&mut b, 1, antirecv, 0, 0);
    b.tick(&[Default::default(), PlayerTick { held: keys::A, ..Default::default() }], Default::default());
    wait_undimmed(&mut b);
    assert_eq!(b.linked[1].chip, Some(antirecv), "side 1 should have AntiRecv armed");

    let p0 = b.player(0).unwrap();
    let pos0 = b.objects.get(p0).panel;
    let normal = content.rules().panels.named("normal").unwrap();
    b.set_panel_type(pos0.x, pos0.y, normal);
    let start_hp = b.objects.get(p0).hp;
    assert!(start_hp >= 500);
    b.stats[0].mood = 0xFF; // Full Synchro -> 0x8000 bit set on Roll
    give_chip(&mut b, 0, roll, 20, 0);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());
    wait_undimmed(&mut b);

    let end_hp = b.objects.get(p0).hp;
    let taken = start_hp - end_hp;
    assert_eq!(taken, 60, "2x Roll1 vs AntiRecv should deal 60 damage (not 49,182 instant kill), got {taken}");
}

/// EXE4: MetlGer1 with Atk+10 must apply the +10 bonus to the spawned gear's damage.
#[test]
fn exe4_metalgear_applies_plus_chip_bonus() {
    let content = exe4_content();
    let mut b = start_fight(&content, "exe4");
    let metlger1 = crate::ids::chip(&content, "exe4", "metlger1").unwrap();

    give_chip(&mut b, 0, metlger1, 100, 10);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());

    let mut gear_damage = None;
    for _ in 0..200 {
        b.tick(&Default::default(), Default::default());
        for r in b.objects.in_order() {
            if b.local_kind_key(r) == "gear" {
                gear_damage = Some(b.objects.get(r).damage);
                break;
            }
        }
        if gear_damage.is_some() {
            break;
        }
    }
    assert_eq!(gear_damage, Some(110), "MetlGer1 (100) + Atk+10 should spawn a 110-damage gear");
}

/// EXE5: ResetBom (Uninstall) swallowed by AntiDmg must NOT strip NaviCust body programs.
#[test]
fn exe5_antidmg_blocks_resetbom_uninstall() {
    let content = exe5_content();
    let mut b = start_fight(&content, "exe5");
    let antidmg = crate::ids::chip(&content, "exe5", "antidmg").unwrap();
    let resetbom = crate::ids::chip(&content, "exe5", "resetbom").unwrap();

    // Give side 1 SuperArmor + AirShoes + FloatShoes and clear any barrier.
    b.stats[1].super_armor = true;
    b.stats[1].air_shoes = true;
    b.stats[1].float_shoes = true;

    // Position players 3 columns apart on the same row so ResetBom lands directly on side 1.
    let p0 = b.player(0).unwrap();
    let p1 = b.player(1).unwrap();
    for (p, x, y) in [(p0, 2, 2), (p1, 5, 2)] {
        b.objects.get_mut(p).panel = saito_battle::object::PanelPos { x, y };
        let (cx, cy) = saito_battle::kinds::player::panel_coordinates(x, y);
        b.objects.get_mut(p).pos = saito_battle::object::Vec3 { x: cx, y: cy, z: 0 };
        if let Some(c) = b.objects.get(p).collision {
            b.collision.get_mut(c).panel = saito_battle::object::PanelPos { x, y };
            b.collision.get_mut(c).barrier = 0;
        }
    }

    // Side 1 arms AntiDmg.
    give_chip(&mut b, 1, antidmg, 100, 0);
    b.tick(&[Default::default(), PlayerTick { held: keys::A, ..Default::default() }], Default::default());
    wait_undimmed(&mut b);
    assert_eq!(b.linked[1].chip, Some(antidmg));

    // Side 0 throws ResetBom (damage 100) at side 1.
    give_chip(&mut b, 0, resetbom, 100, 0);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());
    for _ in 0..120 {
        b.tick(&Default::default(), Default::default());
    }

    assert!(b.linked[1].chip.is_none(), "AntiDmg should have triggered on ResetBom");
    assert!(b.stats[1].super_armor, "AntiDmg-blocked ResetBom must not strip SuperArmor");
    assert!(b.stats[1].air_shoes, "AntiDmg-blocked ResetBom must not strip AirShoes");
}

/// EXE5: DarkInvis / DarkSoul auto-battle drive ignores manual B button presses (no Controllable Dark Soul).
#[test]
fn exe5_darkinvis_ignores_manual_buster_input() {
    let content = exe5_content();
    let mut b = start_fight(&content, "exe5");
    let p0 = b.player(0).unwrap();
    let aid = b.objects.get(p0).actor.unwrap();
    b.actors.get_mut(aid).status |= saito_battle::actor::status::NO_CHARGE;
    b.actors.get_mut(aid).no_charge_timer = 300;

    // Press and release B; no BUSTER request should be raised while NO_CHARGE is set.
    b.tick(&[PlayerTick { held: keys::B, ..Default::default() }, Default::default()], Default::default());
    b.tick(&[Default::default(), Default::default()], Default::default());
    let req = b.actors.get(aid).requests;
    assert_eq!(
        req & (saito_battle::actor::request::BUSTER | saito_battle::actor::request::CHARGED_SHOT),
        0,
        "manual B button must not fire buster during EXE5 NO_CHARGE drive"
    );
}

/// EXE5: Custom HP Bug (DarkCirc bug code 0x54, arg 50) must not wrap modulo 256 (300 -> 44).
#[test]
fn exe5_custom_hp_bug_does_not_wrap_at_256() {
    let content = exe5_content();
    let mut b = start_fight(&content, "exe5");
    for _ in 0..6 {
        saito_battle::kinds::player::take_navi_bug(&mut b, 0, (50u16 << 8) | 0x54);
    }
    assert_eq!(b.stats[0].bugs.custom_damage, 300, "6x 50 custom HP bug must be 300, not 44");
}

/// EXE6: Roll triggering AntiRecv must set `no_cut_in = true`, matching non-dimming recovery chips.
#[test]
fn exe6_roll_anti_recovery_sets_no_cut_in() {
    let content = exe6_content();
    let mut b = start_fight(&content, "exe6");
    let roll = crate::ids::chip(&content, "exe6", "roll").unwrap();
    let antirecv = crate::ids::chip(&content, "exe6", "antirecv").unwrap();

    give_chip(&mut b, 1, antirecv, 0, 0);
    b.tick(&[Default::default(), PlayerTick { held: keys::A, ..Default::default() }], Default::default());
    wait_undimmed(&mut b);
    assert_eq!(b.linked[1].chip, Some(antirecv));

    give_chip(&mut b, 0, roll, 20, 0);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());

    let mut saw_no_cut_in = false;
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
        if b.dimming[0].state != DimmingState::Idle {
            assert!(b.dimming[0].no_cut_in, "Roll-triggered AntiRecv must set no_cut_in = true");
            saw_no_cut_in = true;
            break;
        }
    }
    assert!(saw_no_cut_in, "Roll should have triggered AntiRecv dimming");
}

/// EXE6: DoubleHero with 31/32 Actor slots occupied must abort cleanly instead of panicking.
#[test]
fn exe6_doublehero_full_actor_pool_does_not_crash() {
    let content = exe6_content();
    let mut b = start_fight(&content, "exe6");
    let dblhero = crate::ids::chip(&content, "exe6", "dblhero").unwrap();

    // Fill actor pool to 31/32 so MegaMan's clone takes the last slot (32/32) and ProtoMan fails to spawn.
    let mut filled = 0;
    let p0 = b.player(0).unwrap();
    let sprite = b.objects.sprite(p0).id.unwrap();
    while b.objects.in_order().filter(|r| matches!(r.pool, saito_battle::object::Pool::Actor)).count() < 31 {
        let Some(_) = saito_battle::kinds::idle_overlay::spawn(&mut b, p0, sprite) else { break };
        filled += 1;
    }
    assert!(filled > 0);

    give_chip(&mut b, 0, dblhero, 60, 0);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());
    for _ in 0..180 {
        b.tick(&Default::default(), Default::default());
    }
    assert!(!b.is_dimmed(), "DoubleHero should cleanly undim and exit when Actor pool is full");
}

/// EXE6: TomahawkMan and Puffy patch card BubbleSpread must clear `extra` so Atk+10 does not boost B-charge.
#[test]
fn exe6_tomahawkman_and_puffy_b_charge_clear_extra() {
    let content = exe6_content();
    // 1) TomahawkMan
    let mut m = crate::pick::live(&content, "exe6", 1, None).unwrap();
    let tmhk = crate::ids::navi(&content, "exe6", "tomahawkman").unwrap();
    m.sides[0].set_fact(&content, "navi", &[Fact::Value(Value::Def(Registry::Navi, tmhk.0))]).unwrap();
    let mut setup = m.round(&content, 0x1234);
    setup.custom_screen_limit = Some(1);
    let mut b = saito_battle::Battle::new(setup, content.clone());
    fight(&mut b);
    let p0 = b.player(0).unwrap();
    let aid = b.objects.get(p0).actor.unwrap();
    b.actors.get_mut(aid).attack.extra = 30;
    b.actors.get_mut(aid).requests |= saito_battle::actor::request::CHARGED_SHOT;
    b.tick(&Default::default(), Default::default());
    assert_eq!(b.actors.get(aid).attack.extra, 0, "TomahawkMan charge setup must clear extra");

    // 2) Puffy patch card (BubbleSpread)
    let mut m2 = crate::pick::live(&content, "exe6", 1, None).unwrap();
    let mm = crate::ids::navi(&content, "exe6", "megaman").unwrap();
    let puffy = crate::testing::patch_cards(&content, "exe6", "puffy");
    m2.sides[0].set_fact(&content, "navi", &[Fact::Value(Value::Def(Registry::Navi, mm.0))]).unwrap();
    m2.sides[0].set_fact(&content, "patch_cards", &puffy).unwrap();
    let mut setup2 = m2.round(&content, 0x1234);
    setup2.custom_screen_limit = Some(1);
    let mut b2 = saito_battle::Battle::new(setup2, content.clone());
    fight(&mut b2);
    let p0_2 = b2.player(0).unwrap();
    let aid2 = b2.objects.get(p0_2).actor.unwrap();
    b2.actors.get_mut(aid2).attack.extra = 30;
    b2.actors.get_mut(aid2).requests |= saito_battle::actor::request::CHARGED_SHOT;
    b2.tick(&Default::default(), Default::default());
    assert_eq!(b2.actors.get(aid2).attack.extra, 0, "Puffy BubbleSpread charge setup must clear extra");
}

/// EXE6: Cross break finishing while a dimming controller (e.g. ElementTrap) is active must NOT clear DIMMED.
#[test]
fn exe6_cross_break_preserves_dimmed_flag_when_dimming_active() {
    let content = exe6_content();
    let mut b = start_fight(&content, "exe6");
    let p0 = b.player(0).unwrap();
    let aid = b.objects.get(p0).actor.unwrap();
    let heat = crate::ids::form(&content, "exe6", "heatcross").unwrap();
    b.stats[0].form = heat;
    b.actors.get_mut(aid).requests |= saito_battle::actor::request::WEAKNESS_HIT;
    b.dimming[1].state = DimmingState::Running;
    b.set_flags(battle_flags::DIMMED);

    for _ in 0..40 {
        b.tick(&Default::default(), Default::default());
    }
    assert!(
        b.is_dimmed(),
        "break_form must not clear battle_flags::DIMMED while a dimming controller is active"
    );
}

/// EXE5: Dying VDoll (killed while dimmed) + Poltergeist must finish cleanly without softlocking.
#[test]
fn exe5_vdoll_poltergeist_does_not_softlock() {
    let content = exe5_content();
    let mut b = start_fight(&content, "exe5");
    let vdoll = crate::ids::chip(&content, "exe5", "vdoll").unwrap();
    let poltrgst = crate::ids::chip(&content, "exe5", "poltrgst").unwrap();

    // Place VDoll on field.
    give_chip(&mut b, 0, vdoll, 0, 0);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());
    for _ in 0..60 {
        b.tick(&Default::default(), Default::default());
    }
    let doll = b
        .objects
        .in_order()
        .find(|&r| b.local_kind_key(r) == "vdoll/doll")
        .expect("VDoll should be on field");
    // Simulate VDoll being killed during dimming (HP=0, DEAD flag, Action::Destroyed=2, waiting for undim).
    b.objects.get_mut(doll).hp = 0;
    b.objects.get_mut(doll).action = 2;
    if let Some(c) = b.objects.get(doll).collision {
        b.collision.get_mut(c).f1 |= saito_battle::collision::f1::DEAD;
    }

    // Now side 1 fires Poltergeist; it must NOT lock up waiting on the dead VDoll.
    give_chip(&mut b, 1, poltrgst, 200, 0);
    b.tick(&[Default::default(), PlayerTick { held: keys::A, ..Default::default() }], Default::default());
    wait_undimmed(&mut b);
}

/// EXE6: Once AntiDmg triggers (e.g. during a dimming attack), overwriting or clearing
/// `linked[side]` before the counter starts must not override or zero the counter's damage.
#[test]
fn exe6_trap_damage_override_uses_triggered_snapshot() {
    let content = exe6_content();
    let mut b = start_fight(&content, "exe6");
    let antidmg = crate::ids::chip(&content, "exe6", "antidmg").unwrap();
    let protoman = crate::ids::chip(&content, "exe6", "protoman").unwrap();

    // Clear barriers on side 1.
    let p1 = b.player(1).unwrap();
    if let Some(c) = b.objects.get(p1).collision {
        b.collision.get_mut(c).barrier = 0;
    }

    give_chip(&mut b, 1, antidmg, 100, 0);
    b.tick(&[Default::default(), PlayerTick { held: keys::A, ..Default::default() }], Default::default());
    wait_undimmed(&mut b);
    assert_eq!(b.linked[1].chip, Some(antidmg));

    // Side 0 uses ProtoMan (dimming attack) so AntiDmg triggers while dimmed and snapshots
    // `linked[1]` before `reactive::counter` can run after undim.
    give_chip(&mut b, 0, protoman, 130, 0);
    b.tick(&[PlayerTick { held: keys::A, ..Default::default() }, Default::default()], Default::default());

    let aid1 = b.objects.get(p1).actor.unwrap();
    let mut snapshot_tested = false;
    for _ in 0..240 {
        b.tick(&Default::default(), Default::default());
        if b.triggered_linked[1].is_some() {
            // Simulate Cursor breaking or a new trap overwriting `linked[1]` before counter runs.
            b.clear_linked(1);
            snapshot_tested = true;
            break;
        }
    }
    assert!(snapshot_tested, "AntiDmg should snapshot linked[1] on trigger");

    // Advance until dimming ends and the counter executes; verify it kept 100 damage rather than 0.
    let mut counter_dmg = None;
    for _ in 0..120 {
        b.tick(&Default::default(), Default::default());
        let a = b.actors.get(aid1);
        if a.attack.damage == 100 {
            counter_dmg = Some(a.attack.damage);
            break;
        }
    }
    assert_eq!(counter_dmg, Some(100), "triggered AntiDmg counter must keep 100 damage after linked[1] cleared");
}
