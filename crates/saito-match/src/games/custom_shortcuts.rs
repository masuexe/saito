//! Stage 2 integration tests against real EXE4, EXE5, and EXE6 content:
//! hover CrossSelect, blank right-click TakeBack, and middle-click Soul/Double/Chaos Unison.

use super::*;
use saito_battle::custom::layer::{Gesture, ListStage, Sub, Target};
use saito_battle::custom::look::ScreenSound;
use saito_battle::custom::screen::{Phase, SPECIAL_SLOT, Screen, SlotKind, SlotState};
use saito_battle::custom::{FolderChip, Lockstep};
use saito_battle::input::PlayerTick;
use saito_battle::{Battle, ScreenChoice};
use saito_battle::content::ChipCode;

const A: u8 = 0;
const S: u8 = 18;
const Z: u8 = 25;
const STAR: u8 = 26;

fn screen(b: &Battle) -> &Screen {
    b.custom.sides[0].screen.as_ref().expect("side 0's screen")
}

fn step_idle(b: &mut Battle, lockstep: &mut Lockstep) {
    let idle = [PlayerTick::default(), PlayerTick::default()];
    lockstep.tick(b, &idle, Default::default());
}

fn step_choice(b: &mut Battle, lockstep: &mut Lockstep, choice: ScreenChoice) {
    let input = [PlayerTick::default().with_choice(Some(choice)), PlayerTick::default()];
    lockstep.tick(b, &input, Default::default());
    for _ in 0..120 {
        if screen(b).phase == Phase::Choosing {
            return;
        }
        step_idle(b, lockstep);
    }
}

fn step_until_choosing(b: &mut Battle) -> Lockstep {
    let mut lockstep = Lockstep::new(b);
    for _ in 0..600 {
        if b.custom.sides[0].screen.is_some_and(|s| s.phase == Phase::Choosing) {
            return lockstep;
        }
        step_idle(b, &mut lockstep);
    }
    panic!("no custom screen");
}

fn point_and_settle(b: &mut Battle, lockstep: &mut Lockstep, g: Gesture) {
    lockstep.layers[0].point(g);
    let mut choosing_streak = 0;
    for _ in 0..300 {
        step_idle(b, lockstep);
        if screen(b).phase == Phase::Choosing {
            choosing_streak += 1;
            if choosing_streak >= 12 {
                return;
            }
        } else {
            choosing_streak = 0;
        }
    }
    panic!("screen did not settle in Choosing after {g:?}");
}

fn deal(b: &mut Battle, content: &Content, game: &str, slot: u8, chip: &str, code: u8) {
    let SlotKind::Chip { index, .. } = screen(b).slots[slot as usize].kind else { panic!("slot {slot} holds no dealt chip") };
    let c = crate::ids::chip(content, game, chip).unwrap();
    b.custom.sides[0].folder.chips[index as usize] = Some(FolderChip::new(c, ChipCode(code)));
}

#[test]
fn exe4_middle_click_quick_unites_and_refuses_ineligible_chips() {
    let content = exe4_content();
    let protosoul = crate::ids::form(&content, "exe4", "protosoul").unwrap();

    let new_battle = |regular: bool, evil: bool| {
        let mut m = crate::pick::live(&content, "exe4", 3, None).unwrap();
        let cannon = crate::ids::chip(&content, "exe4", "cannon").unwrap();
        let mut f = m.sides[0].folder(&content);
        f.chips = [Some(FolderChip::new(cannon, ChipCode(A))); 30];
        if regular {
            f.regular = Some(0);
        }
        m.sides[0].set_folder(&content, &f).unwrap();
        m.sides[0].set_fact(&content, "karma", &number(if evil { 100 } else { 500 })).unwrap();
        m.sides[0].set_fact(&content, "double_soul", &flag(true)).unwrap();
        m.sides[0].set_fact(&content, "souls", &forms(&content, "exe4", &["protosoul"])).unwrap();
        Battle::new(m.round(&content, 0x5EED), content.clone())
    };

    // Greyed-out Sword S in slot 1 while Cannon A (slot 2) is picked -> temporary Unpick, Unite, restore Cannon A.
    let mut b = new_battle(true, false);
    let mut ls = step_until_choosing(&mut b);
    deal(&mut b, &content, "exe4", 0, "sword", S); // slot 0 is Regular -> ineligible
    deal(&mut b, &content, "exe4", 1, "sword", S); // slot 1 is Sword S -> eligible greyed when Cannon A picked
    step_choice(&mut b, &mut ls, ScreenChoice::Pick(2));
    assert_eq!(screen(&b).slots[1].state, SlotState::Unavailable);

    // Regular chip on slot 0 -> Refused, keeps slot 2 picked.
    ls.layers[0].point(Gesture::QuickUnite(Some(Target::Slot(0))));
    step_idle(&mut b, &mut ls);
    assert!(ls.layers[0].heard(&b).iter().any(|c| matches!(c, saito_battle::sound::SoundCue::Effect(_))));
    assert_eq!(screen(&b).selection(), [2]);

    // Greyed-out Sword S on slot 1 -> succeeds and restores Cannon A at slot 2.
    point_and_settle(&mut b, &mut ls, Gesture::QuickUnite(Some(Target::Slot(1))));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 2]);
    assert_eq!(b.offer(0).and_then(|o| o.form), Some(protosoul));

    // Blank right-click -> TakeBack removes Cannon A (slot 2).
    ls.layers[0].point(Gesture::TakeBack);
    step_idle(&mut b, &mut ls);
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT]);

    // Worried mood hides UNITE button -> middle-click on eligible chip plays Refused.
    let mut b_worried = new_battle(false, true);
    let mut ls_w = step_until_choosing(&mut b_worried);
    deal(&mut b_worried, &content, "exe4", 1, "sword", S);
    assert!(matches!(screen(&b_worried).slots[SPECIAL_SLOT as usize].kind, SlotKind::Hidden | SlotKind::Empty));
    ls_w.layers[0].point(Gesture::QuickUnite(Some(Target::Slot(1))));
    step_idle(&mut b_worried, &mut ls_w);
    let refused_id = b_worried.screen_sound(ScreenSound::Refused);
    assert!(refused_id.is_some_and(|id| ls_w.layers[0].heard(&b_worried).contains(&saito_battle::sound::SoundCue::Effect(id))));
}

#[test]
fn exe5_middle_click_supports_chaos_unison_five_picks_and_meddysoul_capsules() {
    let content = exe5_content();
    let protosoul = crate::ids::form(&content, "exe5", "protosoul").unwrap();
    let meddysoul = crate::ids::form(&content, "exe5", "meddysoul").unwrap();
    let cannon = crate::ids::chip(&content, "exe5", "cannon").unwrap();

    // 1. 5 picks full: middle-click on left tray slot 5 plays Refused; middle-click on non-last pick 1 unites and restores other 4!
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    b.stats[0].custom_level = 6;
    let mut ls = step_until_choosing(&mut b);
    for i in 0..6u8 {
        deal(&mut b, &content, "exe5", i, "airshot", STAR);
    }
    deal(&mut b, &content, "exe5", 1, "sword", S);
    for i in 0..5u8 {
        step_choice(&mut b, &mut ls, ScreenChoice::Pick(i));
    }
    assert_eq!(screen(&b).selection(), [0, 1, 2, 3, 4]);

    ls.layers[0].point(Gesture::QuickUnite(Some(Target::Slot(5))));
    step_idle(&mut b, &mut ls);
    let refused_id = b.screen_sound(ScreenSound::Refused).unwrap();
    assert!(ls.layers[0].heard(&b).contains(&saito_battle::sound::SoundCue::Effect(refused_id)));
    assert_eq!(screen(&b).selection(), [0, 1, 2, 3, 4]);

    // Middle-click pick 1 (Sword S): pops 4, 3, 2, unites Sword S, restores 2, 3, 4!
    point_and_settle(&mut b, &mut ls, Gesture::QuickUnite(Some(Target::Pick(1))));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 0, 2, 3, 4]);
    assert_eq!(b.offer(0).map(|o| (o.form, o.alternate)), Some((Some(protosoul), false)));

    // 2. Chaos Unison with DrkSword Z while Cannon A is picked.
    let mut b_chaos = Battle::new(m.round(&content, 0x5EED), content.clone());
    let mut ls_c = step_until_choosing(&mut b_chaos);
    deal(&mut b_chaos, &content, "exe5", 0, "cannon", A);
    deal(&mut b_chaos, &content, "exe5", 1, "drksword", Z);
    step_choice(&mut b_chaos, &mut ls_c, ScreenChoice::Pick(0));
    point_and_settle(&mut b_chaos, &mut ls_c, Gesture::QuickUnite(Some(Target::Slot(1))));
    assert_eq!(screen(&b_chaos).selection(), [SPECIAL_SLOT, 0]);
    assert_eq!(b_chaos.offer(0).map(|o| (o.form, o.alternate)), Some((Some(protosoul), true)));

    // Already used Chaos ProtoSoul -> trying another DrkSword Z on slot 2 is Refused.
    // (Commit turn via OK, open 2nd custom screen? Quick check within same screen: UNISON already selected -> slot 2 Refused.)
    deal(&mut b_chaos, &content, "exe5", 2, "drksword", Z);
    ls_c.layers[0].point(Gesture::QuickUnite(Some(Target::Slot(2))));
    step_idle(&mut b_chaos, &mut ls_c);
    assert!(ls_c.layers[0].heard(&b_chaos).contains(&saito_battle::sound::SoundCue::Effect(refused_id)));

    // 3. MeddySoul: attached pick is ineligible, non-last pick preserves trailing capsule!
    let mut f = m.sides[0].folder(&content);
    f.chips = [Some(FolderChip::new(cannon, ChipCode(A))); 30];
    let mut m_meddy = m.clone();
    m_meddy.sides[0].set_folder(&content, &f).unwrap();
    let mut b_meddy = Battle::new(m_meddy.round(&content, 0x5EED), content.clone());
    b_meddy.stats[0].starting_form = meddysoul;
    let mut ls_m = step_until_choosing(&mut b_meddy);
    deal(&mut b_meddy, &content, "exe5", 0, "sword", S);
    deal(&mut b_meddy, &content, "exe5", 1, "airshot", STAR);
    step_choice(&mut b_meddy, &mut ls_m, ScreenChoice::Pick(0));
    step_choice(&mut b_meddy, &mut ls_m, ScreenChoice::Pick(1));
    step_choice(&mut b_meddy, &mut ls_m, ScreenChoice::Press(8)); // capsule on pick 1 (slot 1)
    assert!(screen(&b_meddy).slots[1].attached == Some(8));

    // Middle-clicking pick 1 (attached) -> Refused.
    ls_m.layers[0].point(Gesture::QuickUnite(Some(Target::Pick(1))));
    step_idle(&mut b_meddy, &mut ls_m);
    assert!(ls_m.layers[0].heard(&b_meddy).contains(&saito_battle::sound::SoundCue::Effect(refused_id)));

    // Middle-clicking pick 0 (Sword S, unattached) pops pick 1 (and detaches capsule 8), unites pick 0, restores pick 1 AND capsule 8!
    point_and_settle(&mut b_meddy, &mut ls_m, Gesture::QuickUnite(Some(Target::Pick(0))));
    assert_eq!(screen(&b_meddy).selection(), [SPECIAL_SLOT, 1]);
    assert_eq!(screen(&b_meddy).slots[1].attached, Some(8));
}

#[test]
fn exe6_hover_cross_select_and_silent_middle_click() {
    let content = exe6_content();
    let falzar = "cybeast = \"falzar-beast\"";
    let mut m = parse_in(&content, "exe6", &side("megaman", &EXE6, falzar), &side("megaman", &EXE6, falzar)).unwrap();
    m.sides[0].set_fact(&content, "crosses", &forms(&content, "exe6", &["heatcross", "eleccross"])).unwrap();
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    let mut ls = step_until_choosing(&mut b);

    // Middle-click in EXE6 is completely silent and does not change selection.
    ls.layers[0].point(Gesture::QuickUnite(Some(Target::Slot(0))));
    step_idle(&mut b, &mut ls);
    assert!(ls.layers[0].heard(&b).is_empty(), "EXE6 middle-click must be silent");
    assert!(screen(&b).selection().is_empty());

    // Hovering FormTab opens Cross list automatically.
    ls.layers[0].point(Gesture::Over(Target::FormTab));
    for _ in 0..20 {
        step_idle(&mut b, &mut ls);
    }
    assert!(matches!(ls.layers[0].sub, Sub::List(ListStage::Up(_))), "{:?}", ls.layers[0].sub);

    // Hovering a form keeps it open and moves list cursor.
    ls.layers[0].point(Gesture::Over(Target::Form(1)));
    for _ in 0..6 {
        step_idle(&mut b, &mut ls);
    }
    assert!(matches!(ls.layers[0].sub, Sub::List(ListStage::Up(_))));
    assert_eq!(ls.layers[0].list_cursor, 1);

    // Moving off the list auto-closes it back to Sub::None.
    ls.layers[0].point(Gesture::Away);
    for _ in 0..20 {
        step_idle(&mut b, &mut ls);
    }
    assert_eq!(ls.layers[0].sub, Sub::None);

    // Clicking FormTab opens it WITHOUT auto-closing even when mouse is away.
    ls.layers[0].point(Gesture::Click(Some(Target::FormTab)));
    for _ in 0..24 {
        step_idle(&mut b, &mut ls);
    }
    assert!(matches!(ls.layers[0].sub, Sub::List(ListStage::Up(_))));
}

#[test]
fn exe5_drag_drops_unison_arm_change_and_meddysoul_capsules() {
    use saito_battle::custom::layer::{DragSource, DropTarget};
    let content = exe5_content();
    let protosoul = crate::ids::form(&content, "exe5", "protosoul").unwrap();
    let colonelsoul = crate::ids::form(&content, "exe5", "colonelsoul").unwrap();
    let meddysoul = crate::ids::form(&content, "exe5", "meddysoul").unwrap();
    let cannon = crate::ids::chip(&content, "exe5", "cannon").unwrap();

    // 1. Drag non-last Sword S pick onto UNISON button -> ProtoSoul + restores Cannon A.
    let m = parse(&content, &exe5(&TANGO_EXE5, ""), &exe5(&TANGO_EXE5, "")).unwrap();
    let mut b_unison = Battle::new(m.round(&content, 0x5EED), content.clone());
    let mut ls_u = step_until_choosing(&mut b_unison);
    deal(&mut b_unison, &content, "exe5", 0, "sword", S);
    deal(&mut b_unison, &content, "exe5", 1, "airshot", STAR);
    step_choice(&mut b_unison, &mut ls_u, ScreenChoice::Pick(0));
    step_choice(&mut b_unison, &mut ls_u, ScreenChoice::Pick(1));
    point_and_settle(&mut b_unison, &mut ls_u, Gesture::Drop { from: DragSource::Pick(0), to: DropTarget::Button(SPECIAL_SLOT) });
    assert_eq!(screen(&b_unison).selection(), [SPECIAL_SLOT, 1]);
    assert_eq!(b_unison.offer(0).and_then(|o| o.form), Some(protosoul));

    // 2. ColonelSoul: drag eligible Cannon A onto Arm Change holds it; ineligible Sword S refused; drag held button outside releases it.
    let mut f = m.sides[0].folder(&content);
    f.chips = [Some(FolderChip::new(cannon, ChipCode(A))); 30];
    let mut m_col = m.clone();
    m_col.sides[0].set_folder(&content, &f).unwrap();
    let mut b_col = Battle::new(m_col.round(&content, 0x5EED), content.clone());
    b_col.stats[0].starting_form = colonelsoul;
    let mut ls_arm = step_until_choosing(&mut b_col);
    deal(&mut b_col, &content, "exe5", 0, "sword", S);
    deal(&mut b_col, &content, "exe5", 1, "cannon", A);
    deal(&mut b_col, &content, "exe5", 2, "vulcan1", STAR);
    deal(&mut b_col, &content, "exe5", 3, "cannon", A);

    // Sword S is sword family (not null) -> dropping onto Arm Change plays Refused.
    ls_arm.layers[0].point(Gesture::Drop { from: DragSource::Slot(0), to: DropTarget::Button(8) });
    step_idle(&mut b_col, &mut ls_arm);
    let refused_id = b_col.screen_sound(ScreenSound::Refused).unwrap();
    assert!(ls_arm.layers[0].heard(&b_col).contains(&saito_battle::sound::SoundCue::Effect(refused_id)));
    assert_eq!(screen(&b_col).hold, None);

    // Non-last pick Cannon A (`Pick(0)` before `Vulcan1 *` at `Pick(1)`) dragged onto Arm Change cell 9 -> holds Cannon A & restores Vulcan1 *!
    step_choice(&mut b_col, &mut ls_arm, ScreenChoice::Pick(1));
    step_choice(&mut b_col, &mut ls_arm, ScreenChoice::Pick(2));
    point_and_settle(&mut b_col, &mut ls_arm, Gesture::Drop { from: DragSource::Pick(0), to: DropTarget::Button(9) });
    assert_eq!(screen(&b_col).hold.map(|h| h.chip), Some(1));
    assert_eq!(screen(&b_col).selection(), [2]);

    // While Arm Change already holds Cannon A (`slot 1`), dragging ineligible Sword S (`slot 0`) plays Refused and preserves Cannon A.
    ls_arm.layers[0].point(Gesture::Drop { from: DragSource::Slot(0), to: DropTarget::Button(8) });
    step_idle(&mut b_col, &mut ls_arm);
    assert!(ls_arm.layers[0].heard(&b_col).contains(&saito_battle::sound::SoundCue::Effect(refused_id)));
    assert_eq!(screen(&b_col).hold.map(|h| h.chip), Some(1));
    assert_eq!(screen(&b_col).selection(), [2]);

    // Dragging eligible left-tray chip (`slot 3`, Cannon A) onto occupied Arm Change overwrites held chip and returns `slot 1` cleanly to tray (`puts_back == true`).
    point_and_settle(&mut b_col, &mut ls_arm, Gesture::Drop { from: DragSource::Slot(3), to: DropTarget::Button(8) });
    assert_eq!(screen(&b_col).hold.map(|h| h.chip), Some(3));
    assert_eq!(screen(&b_col).selection(), [2]);
    assert_eq!(screen(&b_col).slots[1].state, SlotState::Selectable);

    // Dragging eligible right-column pick (`Pick(0)` -> `slot 2`, Vulcan1 *) onto occupied Arm Change overwrites held chip cleanly.
    point_and_settle(&mut b_col, &mut ls_arm, Gesture::Drop { from: DragSource::Pick(0), to: DropTarget::Button(9) });
    assert_eq!(screen(&b_col).hold.map(|h| h.chip), Some(2));
    assert!(screen(&b_col).selection().is_empty());
    assert_eq!(screen(&b_col).slots[3].state, SlotState::Selectable);

    // Dragging held Arm Change button outside releases `slot 2` back into picks `[2]`.
    ls_arm.layers[0].point(Gesture::Drop { from: DragSource::Slot(8), to: DropTarget::Outside });
    step_idle(&mut b_col, &mut ls_arm);
    assert_eq!(screen(&b_col).hold, None);
    assert_eq!(screen(&b_col).selection(), [2]);

    // Overwrite when old held chip is incompatible with current selection (`puts_back == false`).
    let mut b_arm_incompat = Battle::new(m_col.round(&content, 0x5EED), content.clone());
    b_arm_incompat.stats[0].starting_form = colonelsoul;
    let mut ls_ai = step_until_choosing(&mut b_arm_incompat);
    deal(&mut b_arm_incompat, &content, "exe5", 0, "sword", S);
    deal(&mut b_arm_incompat, &content, "exe5", 1, "cannon", A);
    deal(&mut b_arm_incompat, &content, "exe5", 2, "vulcan1", STAR);
    step_choice(&mut b_arm_incompat, &mut ls_ai, ScreenChoice::Pick(1));
    step_choice(&mut b_arm_incompat, &mut ls_ai, ScreenChoice::Press(8)); // holds Cannon A (`slot 1`)
    step_choice(&mut b_arm_incompat, &mut ls_ai, ScreenChoice::Pick(0)); // picks Sword S (`slot 0`, incompatible with Cannon A)
    assert_eq!(screen(&b_arm_incompat).slots[2].state, SlotState::Selectable);
    point_and_settle(&mut b_arm_incompat, &mut ls_ai, Gesture::Drop { from: DragSource::Slot(2), to: DropTarget::Button(8) });
    assert_eq!(screen(&b_arm_incompat).hold.map(|h| h.chip), Some(2));
    assert_eq!(screen(&b_arm_incompat).selection(), [0]);
    assert_eq!(screen(&b_arm_incompat).slots[1].state, SlotState::Unavailable); // freed Cannon A stays in tray (code A vs Sword S)

    // 3. MeddySoul: bidirectional capsule <-> chip drops, including capsule onto non-last pick.
    let mut b_med = Battle::new(m_col.round(&content, 0x5EED), content.clone());
    b_med.stats[0].starting_form = meddysoul;
    let mut ls_m = step_until_choosing(&mut b_med);
    // Drag selectable chip Slot(0) directly onto Capsule 1 (`Button(8)`).
    point_and_settle(&mut b_med, &mut ls_m, Gesture::Drop { from: DragSource::Slot(0), to: DropTarget::Button(8) });
    assert_eq!(screen(&b_med).selection(), [0]);
    assert_eq!(screen(&b_med).slots[0].attached, Some(8));

    // Pick Slot(1) and Slot(2), then drag Capsule 2 (`Slot(9)`) directly onto non-last `Pick(1)` -> pops Pick(2), attaches Capsule 2 to Slot(1), restores Slot(2)!
    step_choice(&mut b_med, &mut ls_m, ScreenChoice::Pick(1));
    step_choice(&mut b_med, &mut ls_m, ScreenChoice::Pick(2));
    point_and_settle(&mut b_med, &mut ls_m, Gesture::Drop { from: DragSource::Slot(9), to: DropTarget::ReplaceOrSwap(1) });
    assert_eq!(screen(&b_med).selection(), [0, 1, 2]);
    assert_eq!(screen(&b_med).slots[0].attached, Some(8));
    assert_eq!(screen(&b_med).slots[1].attached, Some(9));
}

#[test]
fn exe6_drag_beast_out_locked_slot_zero_and_outside_unpick() {
    use saito_battle::custom::layer::{DragSource, DropTarget};
    let content = exe6_content();
    let falzar = "cybeast = \"falzar-beast\"";
    let mut m = parse_in(&content, "exe6", &side("megaman", &EXE6, falzar), &side("megaman", &EXE6, falzar)).unwrap();
    m.sides[0].set_fact(&content, "beast_out", &flag(true)).unwrap();
    let mut b = Battle::new(m.round(&content, 0x5EED), content.clone());
    let mut ls = step_until_choosing(&mut b);

    // Pick chips first, then Beast Out -> Beast Out locks into selection[0].
    deal(&mut b, &content, "exe6", 0, "cannon", A);
    deal(&mut b, &content, "exe6", 1, "cannon", A);
    deal(&mut b, &content, "exe6", 2, "cannon", A);
    step_choice(&mut b, &mut ls, ScreenChoice::Pick(0));
    step_choice(&mut b, &mut ls, ScreenChoice::Pick(1));
    step_choice(&mut b, &mut ls, ScreenChoice::Press(SPECIAL_SLOT));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 0, 1]);

    // Locked slot 0 refuses insert before 0 and swap with 0.
    ls.layers[0].point(Gesture::Drop { from: DragSource::Slot(2), to: DropTarget::InsertAt(0) });
    step_idle(&mut b, &mut ls);
    let refused_id = b.screen_sound(ScreenSound::Refused).unwrap();
    assert!(ls.layers[0].heard(&b).contains(&saito_battle::sound::SoundCue::Effect(refused_id)));
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 0, 1]);

    ls.layers[0].point(Gesture::Drop { from: DragSource::Pick(0), to: DropTarget::ReplaceOrSwap(1) });
    step_idle(&mut b, &mut ls);
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 0, 1]);

    // Swapping unlocked Pick(1) and Pick(2) succeeds.
    point_and_settle(&mut b, &mut ls, Gesture::Drop { from: DragSource::Pick(1), to: DropTarget::ReplaceOrSwap(2) });
    assert_eq!(screen(&b).selection(), [SPECIAL_SLOT, 1, 0]);

    // Dragging Beast Out (`Pick(0)`) outside unpicks Beast Out cleanly.
    point_and_settle(&mut b, &mut ls, Gesture::Drop { from: DragSource::Pick(0), to: DropTarget::Outside });
    assert_eq!(screen(&b).selection(), [1, 0]);
    assert_eq!(screen(&b).form, None);
}
