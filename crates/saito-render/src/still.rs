//! Still pictures of a battle's parts on their own, for a host's pictures
//! of them (Practice's grids of its arenas and of the background pool:
//! docs/app.md §15). A round's whole picture is
//! [`crate::Renderer::render`]'s.

use crate::audit::Problems;
use crate::compose::{BACKDROP_DEPTH, Fades, HEIGHT, Layer, WIDTH, compose_with_depth, to_rgb};
use crate::objects::SpriteList;
use crate::graphics::GameGraphics;
use crate::stage::{Stage, StageClock};
use saito_battle::Battle;
use saito_battle::content::{BackgroundId, Content};
use saito_battle::object::{ObjectRef, flags};

/// Background `id` as a round's first frame shows it (its scroll and its
/// animations at their start), `w` by `h` pixels from the screen's top
/// left, each `0x00RRGGBB`: drawn as [`crate::Renderer::render`] draws the
/// background layer, from its own game's graphics (another game's from that game's:
/// the background pool's), the backdrop where it is clear. None: no pack
/// here has it.
///
/// For animated backgrounds whose frame 0 is completely solid/blank (such as
/// EXE1's Comp, Power Plant lightning flash, and Masa's Fish Stand), probes the
/// animation's frame boundaries and returns the first non-uniform still frame so
/// preview thumbnails depict the background's motif.
pub fn background(graphics: &GameGraphics, c: &Content, id: BackgroundId, w: usize, h: usize) -> Option<Vec<u32>> {
    let graphics = graphics.view();
    let background = graphics.background(c, id)?;
    let draw_at = |tick: u32| {
        let clock = StageClock { background: tick, field: 0 };
        let stage = Stage::new(&graphics, c, Some(background), clock, false);
        let backdrop = stage.palettes[0][0];
        stage.draw_background_wide((0, 0), (w, h), backdrop, |c| c)
    };
    let is_uniform = |pixels: &[u16]| pixels.first().is_none_or(|&first| pixels.iter().all(|&p| p == first));

    let mut chosen = draw_at(0);
    if is_uniform(&chosen) && !background.anims.is_empty() {
        let mut ticks = Vec::new();
        for anim in &background.anims {
            let mut acc = 0u32;
            for frame in anim.frames.iter().take(24) {
                acc = acc.saturating_add(u32::from(frame.delay.max(1)));
                ticks.push(acc);
            }
        }
        ticks.sort_unstable();
        ticks.dedup();
        for tick in ticks {
            let candidate = draw_at(tick);
            if !is_uniform(&candidate) {
                chosen = candidate;
                break;
            }
        }
    }
    Some(chosen.into_iter().map(to_rgb).collect())
}

/// Battle `b`'s field as it is, its panels and the objects `keep` on them
/// (an arena's picture's: the two navis standing, each with its own shadow
/// under it, and the objects the arena placed as the round started), and
/// nothing else: no background, no other object, no HUD, no fade. The screen's 240 by 160,
/// each pixel `0xAARRGGBB`, clear (0) where nothing is drawn. The field's
/// panels as [`crate::Renderer::render`] draws them, from the game's graphics,
/// seen from the local side; `keep`'s sprites as it draws them.
pub fn field(graphics: &GameGraphics, b: &Battle, keep: &[ObjectRef]) -> Vec<u32> {
    let graphics = graphics.view();
    let mut problems = Problems::default();
    let mut view = crate::Renderer::view(b);
    view.fade = Default::default();
    // (The others' sprites are left out as a sprite that doesn't animate is.)
    let mut only = b.clone();
    let others: Vec<ObjectRef> = only.objects.in_order().filter(|r| !keep.contains(r)).collect();
    for r in others {
        only.objects.get_mut(r).flags |= flags::NO_SPRITE_UPDATE;
    }
    let stage = Stage::new(&graphics, &b.content, None, StageClock::of(b), false);
    let mut layer = Layer::new(2, 2);
    stage.draw_field(&only, &mut layer, b.setup.local_side, &view, &mut problems);
    let mut list = SpriteList::default();
    crate::objects::queue_objects(&only, &graphics, &view, &[b.setup.local_side], &mut list, &mut problems, false, false, false);
    let parts = list.into_parts();
    let (pixels, depth) = compose_with_depth(0, &[&layer], &parts, Fades::default());
    let drawn = pixels.iter().zip(&depth).map(|(&c, &d)| if d == BACKDROP_DEPTH { 0 } else { 0xFF00_0000 | to_rgb(c) });
    debug_assert_eq!(pixels.len(), WIDTH * HEIGHT);
    drawn.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_assets::{AnimTarget, Background, Bundle, GfxAnim, GfxAnimFrame, MapEntry, Tiles};
    use saito_battle::content::testing;
    use std::sync::Arc;

    /// An animated background whose frame 0 is solid (like EXE1's Power Plant lightning flash)
    /// probes its animation frame boundaries so the preview thumbnail shows non-uniform art.
    #[test]
    fn still_background_probes_past_uniform_frame_zero() {
        let mut c = testing::build();
        c.define().unwrap();
        let bg_id = c.stage_background().unwrap();
        let assets = testing::asset_numbers(&c);
        let num = assets.number(saito_content_api::AssetKind::Background, bg_id.0).unwrap() as usize;

        // Tile 0: half color 1, half color 2.
        let mut tile = vec![1u8; Tiles::TILE];
        for px in tile.iter_mut().take(32) {
            *px = 2;
        }
        let bg = Background {
            tiles: Tiles { pixels: tile },
            first_tile: 0,
            map: vec![MapEntry { tile: 0, hflip: false, vflip: false, palette: 0 }; 4],
            map_width: 2,
            map_height: 2,
            palette: Some([0x0400; 16]),
            anims: vec![GfxAnim {
                target: AnimTarget::Palettes { first: 0, count: 1 },
                frames: vec![
                    GfxAnimFrame { palettes: vec![[0x0400; 16]], delay: 180, ..Default::default() },
                    GfxAnimFrame {
                        palettes: vec![std::array::from_fn(|i| if i == 2 { 0x7FFF } else { 0x0400 })],
                        delay: 4,
                        ..Default::default()
                    },
                ],
                repeat_from: Some(0),
            }],
            ..Background::default()
        };

        let mut bundle = Bundle::default();
        bundle.backgrounds = vec![None; num + 1];
        bundle.backgrounds[num] = Some(bg);
        let held = GameGraphics::new(Arc::new(bundle)).with_numbers(Arc::new(assets));

        let rgb = background(&held, &c, bg_id, 16, 16).unwrap();
        assert!(rgb.contains(&0x00FF_FFFF), "thumbnail should probe to the non-uniform animated frame");
        assert!(rgb.iter().any(|&p| p != rgb[0]), "thumbnail should not remain solid");
    }
}
