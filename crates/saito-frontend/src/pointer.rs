//! The custom screen by pointer (docs/design/custom-screen-actions.md §5):
//! the mouse's and the fingers' events on the battle's picture, in the
//! frame's pixels, made into the local player's layer's gestures
//! (`saito_battle::custom::layer::Gesture`) by what the last frame drew
//! where (`saito_render::Renderer::target_at`).
//!
//! The mouse and a finger are one: over a target (the mouse moved onto it,
//! a press on it), the cursor goes there; off the targets (the mouse moved
//! on none, or off the picture), the cursor is on nothing until the pointer
//! is over a slot again or a key brings it back; let
//! go on the target it was pressed on, it chooses what it is (sliding off
//! cancels, as a button does); held still on it for [`LONG_PRESS`] ticks,
//! it describes it instead, as a right-click does. A click or a tap on no
//! target is still a click: it closes a description, as a key does, and the
//! form list's window, as B does (its targets span it). A finger only moves
//! pressed, so it never hovers. Only what a gesture chooses goes in the
//! player's tick (docs/design/custom-screen-actions.md §3): the cursor's
//! moves are the player's view alone.

use saito_battle::custom::layer::{DragSource, DropTarget, Gesture, Target};
use saito_render::textlayer::Rect;

/// The ticks a press stays down on a target before it describes it (half
/// a second).
pub const LONG_PRESS: u32 = 30;
/// Pointer movement distance (in frame pixels) required to begin dragging on desktop.
pub const DRAG_THRESHOLD: f32 = 5.0;
/// Leftward horizontal drag distance (in frame pixels) required to begin peeking behind the custom screen.
pub const PEEK_THRESHOLD: f32 = 6.0;
/// Maximum leftward peek scroll distance (in frame pixels), matching `saito_render::custom::SLIDE`.
pub const PEEK_MAX: f32 = saito_render::custom::SLIDE as f32;
/// Pixels per tick the custom screen slides back after releasing a peek drag.
pub const PEEK_RETURN_STEP: f32 = 16.0;

/// Desktop drag capabilities & layout metadata computed from the current custom screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DragContext {
    pub enabled: bool,
    pub can_peek: bool,
    pub picks: u8,
    pub locked_first: bool,
    pub draggable_slots: u16,
    pub capsule_slots: u16,
    pub drop_buttons: u16,
    pub column_rect: Option<Rect>,
}

/// Live drag state exposed to `Player` for rendering the translucent ghost & drop visual.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveDrag {
    pub from: DragSource,
    pub x: i32,
    pub y: i32,
    pub to: DropTarget,
}

/// What a pointer did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    /// Pressed (a button, a finger down).
    Down,
    /// Let go.
    Up,
    /// Moved, pressed or not.
    Moved,
    /// The system took it (a gesture of its own, the window lost it): what
    /// it began is forgotten.
    Cancel,
    /// It left the picture.
    Left,
}

/// Which of the mouse's buttons (a finger is the primary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other,
}

/// A pointer's event on the battle's picture: `x` and `y` in the frame's
/// pixels (`Frame::size`, from its top left).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerEvent {
    pub kind: PointerKind,
    pub button: PointerButton,
    pub x: f32,
    pub y: f32,
}

/// A press of the primary button or a finger, until it is let go.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Press {
    target: Option<Target>,
    /// The ticks it has been down, and whether it has left its target.
    held: u32,
    off: bool,
    /// A long press described its target: its letting go chooses nothing
    /// unless a desktop drag began afterwards.
    described: bool,
    start: (f32, f32),
    drag_source: Option<DragSource>,
    dragging: bool,
    peeking: bool,
    peek_origin_x: f32,
    drop: DropTarget,
}

/// The pointer's state on the custom screen: the target it was last over,
/// the press under way, and where the pointer is (frame pixels; none once
/// it leaves the picture), for the host's hand cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenPointer {
    over: Option<Target>,
    press: Option<Press>,
    at: Option<(i32, i32)>,
    hovering: bool,
    peek_scroll: f32,
}

fn resolve_drop(x: f32, y: f32, from: DragSource, ctx: &DragContext, at: &impl Fn(f32, f32) -> Option<Target>) -> DropTarget {
    if let Some(col) = ctx.column_rect {
        let (cx0, cy0) = (col.x as f32, col.y as f32);
        let (cx1, cy1) = (cx0 + col.w as f32, cy0 + col.h as f32);
        if x >= cx0 && x < cx1 && y >= cy0 && y < cy1 {
            let rel_y = y - cy0;
            let row = (rel_y / 16.0).floor().clamp(0.0, 4.0) as u8;
            let frac = rel_y - (row as f32 * 16.0);
            let is_capsule = matches!(from, DragSource::Slot(s) if s < 16 && (ctx.capsule_slots & (1 << s)) != 0);
            return if row < ctx.picks {
                if is_capsule {
                    DropTarget::ReplaceOrSwap(row)
                } else if frac < 4.0 {
                    DropTarget::InsertAt(row)
                } else if frac >= 12.0 {
                    DropTarget::InsertAt(row + 1)
                } else {
                    DropTarget::ReplaceOrSwap(row)
                }
            } else if is_capsule {
                DropTarget::Outside
            } else {
                DropTarget::InsertAt(ctx.picks)
            };
        }
    }
    if let Some(Target::Slot(s)) = at(x, y)
        && s < 16
        && (ctx.drop_buttons & (1 << s)) != 0
    {
        return DropTarget::Button(s);
    }
    DropTarget::Outside
}

impl ScreenPointer {
    /// An event, `at` saying what is under a place of the frame: the
    /// gesture it makes, if it makes one.
    pub fn event(&mut self, e: PointerEvent, at: impl Fn(f32, f32) -> Option<Target>) -> Option<Gesture> {
        self.event_with_drag(e, at, &DragContext::default())
    }

    /// An event processed with desktop drag context (`ctx`).
    pub fn event_with_drag(
        &mut self,
        e: PointerEvent,
        at: impl Fn(f32, f32) -> Option<Target>,
        ctx: &DragContext,
    ) -> Option<Gesture> {
        let under = at(e.x, e.y);
        self.at = match e.kind {
            PointerKind::Left | PointerKind::Cancel => None,
            _ => Some((e.x.floor() as i32, e.y.floor() as i32)),
        };
        match (e.kind, e.button) {
            // (Over its target again, where the mouse already was: a finger
            // lifted since, a key moved the cursor. Only targets that hold a
            // cursor; a tap on the form tab or exit label never puts the
            // cursor away on a touch screen that doesn't hover.)
            (PointerKind::Down, PointerButton::Primary) => {
                let drag_source = if ctx.enabled {
                    match under {
                        Some(Target::Slot(s)) if s < 16 && (ctx.draggable_slots & (1 << s)) != 0 => Some(DragSource::Slot(s)),
                        Some(Target::Pick(k)) if k < ctx.picks => Some(DragSource::Pick(k)),
                        _ => None,
                    }
                } else {
                    None
                };
                self.press = Some(Press {
                    target: under,
                    held: 0,
                    off: false,
                    described: false,
                    start: (e.x, e.y),
                    drag_source,
                    dragging: false,
                    peeking: false,
                    peek_origin_x: e.x,
                    drop: DropTarget::Outside,
                });
                self.over = under;
                under.filter(|t| matches!(t, Target::Slot(_) | Target::Pick(_) | Target::Form(_))).map(Gesture::Over)
            }
            (PointerKind::Down, PointerButton::Secondary) => Some(under.map_or(Gesture::TakeBack, Gesture::Describe)),
            (PointerKind::Down, PointerButton::Middle) => Some(Gesture::QuickUnite(under)),
            (PointerKind::Down, PointerButton::Other) => None,
            (PointerKind::Up, PointerButton::Primary) => {
                let press = self.press.take()?;
                if press.peeking {
                    return None;
                }
                if press.dragging {
                    let from = press.drag_source?;
                    let to = resolve_drop(e.x, e.y, from, ctx, &at);
                    return Some(Gesture::Drop { from, to });
                }
                (!press.described && !press.off && under == press.target).then_some(Gesture::Click(under))
            }
            (PointerKind::Up, _) => None,
            // (A press follows nothing until past DRAG_THRESHOLD on desktop: once dragging,
            // it tracks drop hotzones rather than cancelling on target exit.)
            (PointerKind::Moved, _) => match &mut self.press {
                Some(press) => {
                    if press.peeking {
                        self.peek_scroll = (press.peek_origin_x - e.x).clamp(0.0, PEEK_MAX);
                    } else if !press.dragging {
                        let dx = e.x - press.start.0;
                        let dy = e.y - press.start.1;
                        if let Some(src) = press.drag_source
                            && dx.hypot(dy) >= DRAG_THRESHOLD
                        {
                            press.dragging = true;
                            press.off = true;
                            press.drop = resolve_drop(e.x, e.y, src, ctx, &at);
                            return Some(Gesture::DragBegin(src));
                        }
                        if press.drag_source.is_none() && ctx.can_peek && dx <= -PEEK_THRESHOLD && dx.abs() > dy.abs() {
                            press.peeking = true;
                            press.off = true;
                            press.peek_origin_x = press.start.0 - PEEK_THRESHOLD + self.peek_scroll;
                            self.peek_scroll = (press.peek_origin_x - e.x).clamp(0.0, PEEK_MAX);
                            return None;
                        }
                        press.off |= under != press.target;
                    } else if let Some(src) = press.drag_source {
                        press.drop = resolve_drop(e.x, e.y, src, ctx, &at);
                    }
                    None
                }
                None => {
                    self.hovering = true;
                    self.hover(under)
                }
            },
            (PointerKind::Cancel, _) => {
                self.press = None;
                self.hovering = false;
                None
            }
            (PointerKind::Left, _) => {
                if let Some(press) = &mut self.press {
                    press.off = true;
                    if press.dragging || press.peeking {
                        if press.dragging {
                            press.drop = DropTarget::Outside;
                        }
                        self.hovering = false;
                        return None;
                    }
                }
                self.hovering = false;
                self.over.take().map(|_| Gesture::Away)
            }
        }
    }

    /// Re-evaluate a hovering mouse pointer against the latest drawn targets
    /// (for example when a panel like EXE6's Cross list opens under a stationary
    /// mouse cursor). Never emits events for touchscreens that never hover.
    pub fn sync_hover(&mut self, at: impl Fn(f32, f32) -> Option<Target>) -> Option<Gesture> {
        if !self.hovering || self.press.is_some() {
            return None;
        }
        let (x, y) = self.at?;
        let under = at(x as f32, y as f32);
        if under == self.over {
            return None;
        }
        self.over = under;
        Some(under.map_or(Gesture::Away, Gesture::Over))
    }

    /// Currently active desktop drag operation for visual overlay rendering.
    pub fn active_drag(&self) -> Option<ActiveDrag> {
        let press = self.press.filter(|p| p.dragging)?;
        let from = press.drag_source?;
        let (x, y) = self.at?;
        Some(ActiveDrag { from, x, y, to: press.drop })
    }

    /// Cancel an in-progress drag (for example when leaving `Phase::Choosing`).
    pub fn cancel_drag(&mut self) {
        if self.press.is_some_and(|p| p.dragging) {
            self.press = None;
        }
    }

    /// Current leftward peek scroll of the custom screen (in frame pixels, `0..=PEEK_MAX`).
    pub fn peek_scroll(&self) -> u32 {
        self.peek_scroll.round() as u32
    }

    /// Whether a peek drag is currently held down.
    pub fn is_peeking(&self) -> bool {
        self.press.is_some_and(|p| p.peeking)
    }

    /// Cancel an in-progress peek drag and reset any residual peek scroll.
    pub fn cancel_peek(&mut self) {
        if self.press.is_some_and(|p| p.peeking) {
            self.press = None;
        }
        self.peek_scroll = 0.0;
    }

    /// The mouse moved over `under`: over it, if it is another target than
    /// the one it was over; off the targets, away, at each move (a key may
    /// have brought the cursor back since the last).
    fn hover(&mut self, under: Option<Target>) -> Option<Gesture> {
        if under.is_some() && under == self.over {
            return None;
        }
        self.over = under;
        Some(under.map_or(Gesture::Away, Gesture::Over))
    }

    /// A tick of the battle's: a press held still on a target long enough
    /// describes it, and a released peek drag slides smoothly back to 0.
    pub fn tick(&mut self) -> Option<Gesture> {
        if self.press.is_none_or(|p| !p.peeking) && self.peek_scroll > 0.0 {
            self.peek_scroll = (self.peek_scroll - PEEK_RETURN_STEP).max(0.0);
        }
        let press = self.press.as_mut()?;
        press.held = press.held.saturating_add(1);
        if !press.dragging && !press.peeking && !press.off && !press.described && press.held >= LONG_PRESS {
            press.described = true;
            return press.target.map(Gesture::Describe);
        }
        None
    }

    /// Where the pointer is on the picture, in frame pixels (none off the
    /// picture), for the host's hand cursor.
    pub fn at(&self) -> Option<(i32, i32)> {
        self.at
    }

    /// Forget what is under way (a new round, the battle held).
    pub fn clear(&mut self) {
        *self = ScreenPointer::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHIP: Target = Target::Slot(1);
    const OK: Target = Target::Slot(10);

    /// Slot 1 left of x 100, OK right of it; nothing below y 50.
    fn at(x: f32, y: f32) -> Option<Target> {
        (y < 50.0).then_some(if x < 100.0 { CHIP } else { OK })
    }

    fn e(kind: PointerKind, button: PointerButton, x: f32, y: f32) -> PointerEvent {
        PointerEvent { kind, button, x, y }
    }

    fn p(kind: PointerKind, x: f32, y: f32) -> PointerEvent {
        e(kind, PointerButton::Primary, x, y)
    }

    /// The pointer's place, for the hand cursor: where it last was on the
    /// picture; none once it leaves.
    #[test]
    fn the_pointer_is_where_it_last_was() {
        let mut s = ScreenPointer::default();
        s.event(p(PointerKind::Moved, 20.5, 10.0), at);
        assert_eq!(s.at(), Some((20, 10)));
        s.event(p(PointerKind::Down, 120.0, 10.0), at);
        assert_eq!(s.at(), Some((120, 10)));
        s.event(p(PointerKind::Left, 0.0, 0.0), at);
        assert_eq!(s.at(), None);
    }

    /// The mouse over a target moves the cursor there, once as it comes
    /// onto it; off the targets, away, at each move; off the picture from
    /// one, away.
    #[test]
    fn the_mouse_hovers() {
        let mut s = ScreenPointer::default();
        assert_eq!(s.event(p(PointerKind::Moved, 10.0, 10.0), at), Some(Gesture::Over(CHIP)));
        assert_eq!(s.event(p(PointerKind::Moved, 20.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Moved, 120.0, 10.0), at), Some(Gesture::Over(OK)));
        assert_eq!(s.event(p(PointerKind::Moved, 10.0, 90.0), at), Some(Gesture::Away));
        assert_eq!(s.event(p(PointerKind::Moved, 11.0, 90.0), at), Some(Gesture::Away));
        assert_eq!(s.event(p(PointerKind::Moved, 120.0, 10.0), at), Some(Gesture::Over(OK)));
        assert_eq!(s.event(p(PointerKind::Left, 0.0, 0.0), at), Some(Gesture::Away));
        assert_eq!(s.event(p(PointerKind::Left, 0.0, 0.0), at), None);
        // Pressed, it follows nothing: no hover until it is let go.
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Over(CHIP)));
        assert_eq!(s.event(p(PointerKind::Moved, 120.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Up, 120.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Moved, 121.0, 10.0), at), Some(Gesture::Over(OK)));
    }

    /// Down on a target puts the cursor there (where it already hovers too),
    /// up on the same target chooses; sliding off cancels; on nothing, a
    /// click on nothing; right-click or a long press describes.
    #[test]
    fn a_press_moves_the_cursor_and_its_letting_go_chooses() {
        let mut s = ScreenPointer::default();
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Over(CHIP)));
        assert_eq!(s.tick(), None);
        assert_eq!(s.event(p(PointerKind::Moved, 12.0, 10.0), at), None);
        assert_eq!(s.event(p(PointerKind::Up, 12.0, 10.0), at), Some(Gesture::Click(Some(CHIP))));
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 10.0), at), Some(Gesture::Over(CHIP)));
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), Some(Gesture::Click(Some(CHIP))));
        assert_eq!(s.event(p(PointerKind::Down, 120.0, 10.0), at), Some(Gesture::Over(OK)));
        assert_eq!(s.event(p(PointerKind::Up, 121.0, 11.0), at), Some(Gesture::Click(Some(OK))));
        // Down on one, up on another; slid off and back; off the picture: nothing chosen.
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Moved, 120.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 120.0, 10.0), at), None);
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Moved, 10.0, 90.0), at);
        s.event(p(PointerKind::Moved, 10.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Left, 0.0, 0.0), at), Some(Gesture::Away));
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
        // On nothing: a click on nothing.
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 90.0), at), None);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 90.0), at), Some(Gesture::Click(None)));
        // Right-click describes.
        assert_eq!(s.event(e(PointerKind::Down, PointerButton::Secondary, 10.0, 10.0), at), Some(Gesture::Describe(CHIP)));
        assert_eq!(s.event(e(PointerKind::Up, PointerButton::Secondary, 10.0, 10.0), at), None);
        // Held: it describes once, and its letting go chooses nothing.
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        let described: Vec<_> = (0..2 * LONG_PRESS).filter_map(|_| s.tick()).collect();
        assert_eq!(described, [Gesture::Describe(CHIP)]);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
        // Cancelled by the system: nothing.
        s.event(p(PointerKind::Down, 10.0, 10.0), at);
        s.event(p(PointerKind::Cancel, 10.0, 10.0), at);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), at), None);
    }

    /// Tapping the form list's tab or exit label (without hovering first, as
    /// on a touch screen) does not emit `Over` on press (which would put the
    /// cursor away and blank the chip window), while hovering with a mouse
    /// still does.
    #[test]
    fn tapping_form_tab_does_not_put_cursor_away() {
        let tab = |_: f32, _: f32| Some(Target::FormTab);
        let mut s = ScreenPointer::default();
        assert_eq!(s.event(p(PointerKind::Down, 10.0, 10.0), tab), None);
        assert_eq!(s.event(p(PointerKind::Up, 10.0, 10.0), tab), Some(Gesture::Click(Some(Target::FormTab))));
        let mut hovered = ScreenPointer::default();
        assert_eq!(hovered.event(p(PointerKind::Moved, 10.0, 10.0), tab), Some(Gesture::Over(Target::FormTab)));
    }

    #[test]
    fn blank_right_click_takes_back_and_middle_click_quick_unites() {
        let mut s = ScreenPointer::default();
        assert_eq!(s.event(e(PointerKind::Down, PointerButton::Secondary, 10.0, 90.0), at), Some(Gesture::TakeBack));
        assert_eq!(s.event(e(PointerKind::Down, PointerButton::Secondary, 10.0, 10.0), at), Some(Gesture::Describe(CHIP)));
        assert_eq!(s.event(e(PointerKind::Down, PointerButton::Middle, 10.0, 10.0), at), Some(Gesture::QuickUnite(Some(CHIP))));
        assert_eq!(s.event(e(PointerKind::Down, PointerButton::Middle, 10.0, 90.0), at), Some(Gesture::QuickUnite(None)));
        assert_eq!(s.event(e(PointerKind::Up, PointerButton::Middle, 10.0, 10.0), at), None);
    }

    #[test]
    fn sync_hover_picks_up_newly_opened_targets_for_mouse_only() {
        let mut list_open = false;
        let dyn_at = |open: bool| move |x: f32, y: f32| {
            if open && (20.0..80.0).contains(&x) && (20.0..40.0).contains(&y) {
                Some(Target::Form(0))
            } else {
                None
            }
        };
        let mut touch = ScreenPointer::default();
        assert_eq!(touch.event(p(PointerKind::Down, 40.0, 30.0), dyn_at(list_open)), None);
        assert_eq!(touch.event(p(PointerKind::Up, 40.0, 30.0), dyn_at(list_open)), Some(Gesture::Click(None)));
        list_open = true;
        assert_eq!(touch.sync_hover(dyn_at(list_open)), None, "touch must not hover after release");

        let mut mouse = ScreenPointer::default();
        list_open = false;
        assert_eq!(mouse.event(p(PointerKind::Moved, 40.0, 30.0), dyn_at(list_open)), Some(Gesture::Away));
        assert_eq!(mouse.sync_hover(dyn_at(list_open)), None);
        list_open = true;
        assert_eq!(mouse.sync_hover(dyn_at(list_open)), Some(Gesture::Over(Target::Form(0))));
        assert_eq!(mouse.sync_hover(dyn_at(list_open)), None);
    }

    #[test]
    fn desktop_drag_threshold_and_column_hotzones() {
        let col = Rect::new(100, 16, 24, 80); // rows 0..5, 16px each: row 0 = y 16..32, row 1 = y 32..48
        let ctx = DragContext {
            enabled: true,
            can_peek: true,
            picks: 2,
            locked_first: false,
            draggable_slots: 1 << 1,
            capsule_slots: 0,
            drop_buttons: 1 << 8,
            column_rect: Some(col),
        };
        let drag_at = |x: f32, y: f32| {
            if (8.0..24.0).contains(&x) && (104.0..128.0).contains(&y) {
                Some(Target::Slot(1))
            } else if (80.0..96.0).contains(&x) && (104.0..128.0).contains(&y) {
                Some(Target::Slot(8))
            } else if (100.0..124.0).contains(&x) && (16.0..32.0).contains(&y) {
                Some(Target::Pick(0))
            } else if (100.0..124.0).contains(&x) && (32.0..48.0).contains(&y) {
                Some(Target::Pick(1))
            } else {
                None
            }
        };

        // Small jitter (< 5px) within slot remains a normal click.
        let mut s = ScreenPointer::default();
        assert_eq!(s.event_with_drag(p(PointerKind::Down, 12.0, 110.0), drag_at, &ctx), Some(Gesture::Over(Target::Slot(1))));
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 14.0, 112.0), drag_at, &ctx), None);
        assert_eq!(s.active_drag(), None);
        assert_eq!(s.event_with_drag(p(PointerKind::Up, 14.0, 112.0), drag_at, &ctx), Some(Gesture::Click(Some(Target::Slot(1)))));

        // Moving >= 5px begins dragging and suppresses click. Test all column hotzones:
        for (y, want) in [
            (17.0, DropTarget::InsertAt(0)),       // row 0 top 25% (16..20)
            (24.0, DropTarget::ReplaceOrSwap(0)),  // row 0 middle 50% (20..28)
            (29.0, DropTarget::InsertAt(1)),       // row 0 bottom 25% (28..32)
            (40.0, DropTarget::ReplaceOrSwap(1)),  // row 1 middle 50%
            (60.0, DropTarget::InsertAt(2)),       // empty row below picks -> append
        ] {
            assert_eq!(s.event_with_drag(p(PointerKind::Down, 12.0, 110.0), drag_at, &ctx), Some(Gesture::Over(Target::Slot(1))));
            assert_eq!(s.event_with_drag(p(PointerKind::Moved, 112.0, y), drag_at, &ctx), Some(Gesture::DragBegin(DragSource::Slot(1))));
            assert!(s.active_drag().is_some());
            assert_eq!(
                s.event_with_drag(p(PointerKind::Up, 112.0, y), drag_at, &ctx),
                Some(Gesture::Drop { from: DragSource::Slot(1), to: want })
            );
        }

        // Drop onto special button & outside.
        s.event_with_drag(p(PointerKind::Down, 12.0, 110.0), drag_at, &ctx);
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 85.0, 112.0), drag_at, &ctx), Some(Gesture::DragBegin(DragSource::Slot(1))));
        assert_eq!(
            s.event_with_drag(p(PointerKind::Up, 85.0, 112.0), drag_at, &ctx),
            Some(Gesture::Drop { from: DragSource::Slot(1), to: DropTarget::Button(8) })
        );

        s.event_with_drag(p(PointerKind::Down, 112.0, 24.0), drag_at, &ctx);
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 200.0, 10.0), drag_at, &ctx), Some(Gesture::DragBegin(DragSource::Pick(0))));
        assert_eq!(
            s.event_with_drag(p(PointerKind::Up, 200.0, 10.0), drag_at, &ctx),
            Some(Gesture::Drop { from: DragSource::Pick(0), to: DropTarget::Outside })
        );
    }

    #[test]
    fn long_press_describe_transitions_cleanly_into_drag() {
        let ctx = DragContext {
            enabled: true,
            can_peek: true,
            picks: 1,
            locked_first: false,
            draggable_slots: 1 << 1,
            capsule_slots: 0,
            drop_buttons: 0,
            column_rect: Some(Rect::new(100, 16, 24, 80)),
        };
        let mut s = ScreenPointer::default();
        s.event_with_drag(p(PointerKind::Down, 10.0, 10.0), at, &ctx);
        let described: Vec<_> = (0..LONG_PRESS).filter_map(|_| s.tick()).collect();
        assert_eq!(described, [Gesture::Describe(CHIP)]);
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 112.0, 60.0), at, &ctx), Some(Gesture::DragBegin(DragSource::Slot(1))));
        assert_eq!(s.tick(), None, "must not re-describe once dragging");
        assert_eq!(
            s.event_with_drag(p(PointerKind::Up, 112.0, 60.0), at, &ctx),
            Some(Gesture::Drop { from: DragSource::Slot(1), to: DropTarget::InsertAt(1) })
        );
    }

    #[test]
    fn mobile_touch_does_not_trigger_drag() {
        let ctx = DragContext { enabled: false, ..DragContext::default() };
        let mut s = ScreenPointer::default();
        s.event_with_drag(p(PointerKind::Down, 10.0, 10.0), at, &ctx);
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 120.0, 10.0), at, &ctx), None);
        assert_eq!(s.active_drag(), None);
        assert_eq!(s.event_with_drag(p(PointerKind::Up, 120.0, 10.0), at, &ctx), None);
    }

    #[test]
    fn leftward_drag_peeks_custom_screen_and_springs_back_on_release() {
        let open = DragContext { enabled: true, can_peek: true, draggable_slots: 1 << 1, ..DragContext::default() };
        let mut s = ScreenPointer::default();

        // Leftward drag on blank area follows pointer smoothly and clamps to PEEK_MAX.
        assert_eq!(s.event_with_drag(p(PointerKind::Down, 100.0, 60.0), at, &open), None);
        assert_eq!(s.peek_scroll(), 0);
        // Below PEEK_THRESHOLD: does not start peeking yet.
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 96.0, 60.0), at, &open), None);
        assert!(!s.is_peeking());
        assert_eq!(s.peek_scroll(), 0);
        // Past PEEK_THRESHOLD (6px): enters peeking and tracks distance smoothly.
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 64.0, 62.0), at, &open), None);
        assert!(s.is_peeking());
        assert_eq!(s.peek_scroll(), 30);
        // Dragging further left clamps at PEEK_MAX (120).
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, -50.0, 62.0), at, &open), None);
        assert_eq!(s.peek_scroll(), 120);
        // Dragging back right while still holding reduces peek_scroll.
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 44.0, 62.0), at, &open), None);
        assert_eq!(s.peek_scroll(), 50);
        // Releasing suppresses Click and lets tick() slide smoothly back to 0.
        assert_eq!(s.event_with_drag(p(PointerKind::Up, 44.0, 62.0), at, &open), None);
        assert!(!s.is_peeking());
        assert_eq!(s.peek_scroll(), 50);
        assert_eq!(s.tick(), None);
        assert_eq!(s.peek_scroll(), 34);
        for _ in 0..4 {
            s.tick();
        }
        assert_eq!(s.peek_scroll(), 0);

        // Mostly vertical drag does not trigger peek (releasing still acts as a normal blank click).
        assert_eq!(s.event_with_drag(p(PointerKind::Down, 80.0, 60.0), at, &open), None);
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 65.0, 100.0), at, &open), None);
        assert!(!s.is_peeking());
        assert_eq!(s.event_with_drag(p(PointerKind::Up, 65.0, 100.0), at, &open), Some(Gesture::Click(None)));

        // Dragging a draggable chip slot leftward still starts chip drag instead of peeking.
        assert_eq!(s.event_with_drag(p(PointerKind::Down, 10.0, 10.0), at, &open), Some(Gesture::Over(CHIP)));
        assert_eq!(s.event_with_drag(p(PointerKind::Moved, 0.0, 10.0), at, &open), Some(Gesture::DragBegin(DragSource::Slot(1))));
        assert!(!s.is_peeking());
        assert_eq!(s.peek_scroll(), 0);
    }
}
