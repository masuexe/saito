//! The selected game's look (docs/app.md §10; docs/design/ui-themes.md): the theme's colors, PET/terminal
//! style flag, optional custom backdrop/fonts, and the ground behind the menus.
//!
//! Driven by the active theme (`crate::themes::ResolvedTheme`: built-in `pet-hd`, built-in `terminal` which
//! reads each game's `content/<game>/look.toml`, or any user `.saitotheme` plugin). Switching themes updates
//! `Theme` immediately without restarting; switching games sweeps the new game's ground across the window and
//! cross-fades the colors over `Theme.settle`.

use saito_ui::{AppWindow, Ground, Look, PatternPaths, Theme};
use slint::{Color, ComponentHandle, Image, Model, ModelRc, VecModel};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;
use web_time::Instant;

/// The most grounds there are at once: a change over a change over a change; one more drops the oldest.
const MOST_GROUNDS: usize = 4;

/// How long after the last ground's sweep began those under it are dropped: its sweep, and a little for the
/// frame its clock starts in.
const SWEPT: Duration = Duration::from_millis(60);

/// The looks, active UI theme, and any game-change transition under way.
pub struct Looks {
    /// Content `look.toml` reads per game (for the built-in `terminal` theme and plugins extending it).
    content_by_game: HashMap<String, (PathBuf, Option<Look>)>,
    /// The active resolved theme.
    active: crate::themes::ResolvedTheme,
    /// The look shown, or being changed to.
    target: Look,
    /// The colors' cross-fade under way: since when.
    fading: Option<Instant>,
    /// A change's ground, to come in at the next frame.
    coming: Option<Ground>,
    /// The grounds the window shows, the last on top, and when each came.
    grounds: Rc<VecModel<Ground>>,
    came: Vec<Instant>,
}

impl Looks {
    /// Load every game's content look and activate `theme_id` (defaulting to `pet-hd` if unknown).
    pub fn new<'g>(ui: &AppWindow, games: impl IntoIterator<Item = &'g str>, theme_id: &str) -> Looks {
        let mut content_by_game = HashMap::new();
        for game in games {
            if let Some(dir) = crate::contents::dir(game) {
                let look = read_content(&dir, game);
                content_by_game.insert(game.to_string(), (dir, look));
            }
        }
        let terminal_games = terminal_map(&content_by_game);
        let active = crate::themes::resolve_in(&crate::paths::themes(), theme_id, &terminal_games);
        apply_theme_globals(ui, &active);
        let plain = active.default_look.clone();
        let theme = ui.global::<Theme>();
        theme.set_was(plain.clone());
        theme.set_look(plain.clone());
        theme.set_fade(1.0);
        let grounds = Rc::new(VecModel::from(vec![Ground { look: plain.clone(), from: 0 }]));
        ui.global::<saito_ui::Shell>().set_grounds(ModelRc::from(grounds.clone()));
        Looks { content_by_game, active, target: plain, fading: None, coming: None, grounds, came: vec![Instant::now()] }
    }

    /// Hot-switch the active UI theme and immediately re-apply `current_game`'s look.
    pub fn set_theme(&mut self, ui: &AppWindow, theme_id: &str, current_game: Option<&str>) {
        let terminal_games = terminal_map(&self.content_by_game);
        self.active = crate::themes::resolve_in(&crate::paths::themes(), theme_id, &terminal_games);
        apply_theme_globals(ui, &self.active);
        let look = self.look_of_opt(current_game);
        let theme = ui.global::<Theme>();
        theme.set_was(look.clone());
        theme.set_look(look.clone());
        theme.set_fade(1.0);
        self.target = look.clone();
        self.fading = None;
        self.coming = None;
        self.grounds.set_vec(vec![Ground { look, from: 0 }]);
        self.came = vec![Instant::now()];
    }

    /// The active theme's resolved ID.
    pub fn theme_id(&self) -> &str {
        &self.active.id
    }

    /// Show game `game`'s look, its ground sweeping in from side `from` (-1 the left, 1 the right); `from` 0, or
    /// motion turned off, and it is there at once.
    pub fn show(&mut self, ui: &AppWindow, game: Option<&str>, from: i32) {
        let look = self.look_of_opt(game);
        if look == self.target {
            return;
        }
        let theme = ui.global::<Theme>();
        let at_once = from == 0 || theme.get_settle() <= 0;
        let shown = blend(&theme.get_look(), &theme.get_was(), theme.get_fade());
        theme.set_was(shown);
        theme.set_look(look.clone());
        theme.set_fade(if at_once { 1.0 } else { 0.0 });
        self.fading = None;
        self.target = look.clone();
        if at_once {
            self.coming = None;
            self.grounds.set_vec(vec![Ground { look, from: 0 }]);
            self.came = vec![Instant::now()];
        } else {
            self.coming = Some(Ground { look, from: from.signum() });
        }
    }

    /// A frame at `now`, before it is drawn: a change's ground comes in and its colors fade.
    pub fn frame(&mut self, ui: &AppWindow, now: Instant) {
        let theme = ui.global::<Theme>();
        let settle = Duration::from_millis(theme.get_settle().max(0) as u64);
        if let Some(ground) = self.coming.take() {
            self.grounds.push(ground);
            self.came.push(now);
            while self.grounds.row_count() > MOST_GROUNDS {
                self.grounds.remove(0);
                self.came.remove(0);
            }
            self.fading = Some(now);
        }
        if let Some(at) = self.fading {
            let linear = if settle.is_zero() { 1.0 } else { (now.saturating_duration_since(at).as_secs_f32() / settle.as_secs_f32()).min(1.0) };
            theme.set_fade(1.0 - (1.0 - linear).powi(3));
            if linear >= 1.0 {
                self.fading = None;
            }
        }
        if self.grounds.row_count() > 1 && self.came.last().is_some_and(|at| now.saturating_duration_since(*at) >= settle + SWEPT) {
            while self.grounds.row_count() > 1 {
                self.grounds.remove(0);
                self.came.remove(0);
            }
        }
    }

    /// How many grounds the window has (the screenshot pass's report).
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn grounds(&self) -> usize {
        self.grounds.row_count()
    }

    fn look_of_opt(&mut self, game: Option<&str>) -> Look {
        let Some(game) = game else {
            return self.active.default_look.clone();
        };
        // Refresh content `look.toml` if a game's content folder changed on disk.
        if let Some(dir) = crate::contents::dir(game)
            && self.content_by_game.get(game).is_none_or(|(read_from, _)| *read_from != dir)
        {
            let look = read_content(&dir, game);
            self.content_by_game.insert(game.to_string(), (dir, look));
            if self.active.id == crate::themes::TERMINAL_THEME {
                let terminal_games = terminal_map(&self.content_by_game);
                self.active = crate::themes::resolve_in(&crate::paths::themes(), &self.active.id, &terminal_games);
            }
        }
        self.active.games.get(game).cloned().unwrap_or_else(|| self.active.default_look.clone())
    }
}

fn apply_theme_globals(ui: &AppWindow, active: &crate::themes::ResolvedTheme) {
    let theme = ui.global::<Theme>();
    theme.set_pet(active.pet);
    theme.set_backdrop_image(active.assets.backdrop_image.clone().unwrap_or_else(Image::default));
    theme.set_backdrop_image_opacity(if active.assets.backdrop_image.is_some() { active.assets.backdrop_image_opacity } else { 0.0 });
    theme.set_custom_display_font(active.assets.display_font.as_deref().unwrap_or("").into());
    theme.set_custom_body_font(active.assets.body_font.as_deref().unwrap_or("").into());
    theme.set_custom_mono_font(active.assets.mono_font.as_deref().unwrap_or("").into());
}

fn terminal_map(by_game: &HashMap<String, (PathBuf, Option<Look>)>) -> HashMap<String, Look> {
    by_game.iter().filter_map(|(g, (_, l))| l.as_ref().map(|look| (g.clone(), look.clone()))).collect()
}

fn read_content(dir: &Path, game: &str) -> Option<Look> {
    match saito_content::look::of(dir, game) {
        Ok(look) => look.as_ref().map(crate::themes::terminal_look_from_content),
        Err(why) => {
            crate::errors::log(&format!("reading {game}'s look"), &why);
            None
        }
    }
}

/// `new`'s colors `fade` of the way from `old`'s, as the theme blends them (`Theme.blend`); the rest is `new`'s.
fn blend(new: &Look, old: &Look, fade: f32) -> Look {
    if fade >= 1.0 {
        return new.clone();
    }
    let mix = |a: Color, b: Color| a.mix(&b, fade);
    Look {
        ground_top: mix(new.ground_top, old.ground_top),
        ground_bottom: mix(new.ground_bottom, old.ground_bottom),
        panel: mix(new.panel, old.panel),
        raised: mix(new.raised, old.raised),
        line: mix(new.line, old.line),
        inset: mix(new.inset, old.inset),
        text: mix(new.text, old.text),
        muted: mix(new.muted, old.muted),
        dim: mix(new.dim, old.dim),
        cursor: mix(new.cursor, old.cursor),
        action: mix(new.action, old.action),
        action_ink: mix(new.action_ink, old.action_ink),
        frame_stroke: mix(new.frame_stroke, old.frame_stroke),
        btn_shadow: mix(new.btn_shadow, old.btn_shadow),
        active_top: mix(new.active_top, old.active_top),
        active_bot: mix(new.active_bot, old.active_bot),
        active_border: mix(new.active_border, old.active_border),
        active_shadow: mix(new.active_shadow, old.active_shadow),
        slot_bg: mix(new.slot_bg, old.slot_bg),
        slot_border: mix(new.slot_border, old.slot_border),
        bar_start: mix(new.bar_start, old.bar_start),
        bar_end: mix(new.bar_end, old.bar_end),
        ..new.clone()
    }
}

// ---- The patterns' paths ------------------------------------------------------------------------------------

const HEXAGON_TILE: (f32, f32) = (42.0, 72.75);
const GRID_TILE: (f32, f32) = (92.0, 53.116);
const STAR_TILE: (f32, f32) = (292.0, 244.0);
/// Clean 45-degree PET diagonal grid tile (44×44 logical units).
const PET_GRID_TILE: (f32, f32) = (44.0, 44.0);

/// Every pattern's paths over a window of `width` by `height` units and a tile more each way.
pub fn patterns(width: i32, height: i32) -> PatternPaths {
    let [stars, far_stars, bright_stars] = starfield(width, height);
    PatternPaths {
        hexagons: hexagons(width, height).into(),
        isometric_grid: isometric_grid(width, height).into(),
        pet_grid: pet_grid(width, height).into(),
        stars: stars.into(),
        far_stars: far_stars.into(),
        bright_stars: bright_stars.into(),
    }
}

/// Crisp 45-degree diagonal PET grid: repeats seamlessly on `PET_GRID_TILE` (44×44).
fn pet_grid(width: i32, height: i32) -> String {
    let (w, h) = (width as f32 + PET_GRID_TILE.0, height as f32 + PET_GRID_TILE.1);
    let step = PET_GRID_TILE.0;
    let lines = ((w + h) / step).ceil() as i32;
    let mut path = String::new();
    for k in 0..=lines {
        let d = k as f32 * step;
        // Falling 45° (x - y = const) and rising 45° (x + y = const) across the bounding box.
        let _ = write!(path, "M0 {:.1}L{w:.1} {:.1}M0 {:.1}L{w:.1} {:.1}", d - w, d, d, d - w);
    }
    path
}

/// The field of hexagons: each tile holds one hexagon and the stem to the one under it, and draws the edges its
/// neighbors don't.
fn hexagons(width: i32, height: i32) -> String {
    let (columns, rows) = ((width as f32 / HEXAGON_TILE.0).ceil() as i32 + 1, (height as f32 / HEXAGON_TILE.1).ceil() as i32 + 1);
    let mut path = String::new();
    for row in 0..rows {
        for column in 0..columns {
            let (x, y) = (column as f32 * HEXAGON_TILE.0, row as f32 * HEXAGON_TILE.1);
            let _ = write!(
                path,
                "M{:.2} {:.2}V{:.2}L{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}L{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}V{:.2}",
                x,
                y + 36.38,
                y + 12.12,
                x + 21.0,
                y,
                x + 42.0,
                y + 12.12,
                x,
                y + 36.38,
                x + 21.0,
                y + 48.5,
                x + 42.0,
                y + 36.38,
                x + 21.0,
                y + 48.5,
                y + 72.75,
            );
        }
    }
    path
}

/// The isometric grid: lines falling and rising at 30 degrees.
fn isometric_grid(width: i32, height: i32) -> String {
    let (w, h) = (width as f32 + GRID_TILE.0, height as f32 + GRID_TILE.1);
    let drop = w * 30f32.to_radians().tan();
    let lines = ((h + drop) / GRID_TILE.1).ceil() as i32;
    let mut path = String::new();
    for k in 0..=lines {
        let at = k as f32 * GRID_TILE.1;
        let _ = write!(path, "M0 {:.2}L{w:.2} {:.2}M0 {:.2}L{w:.2} {:.2}", at - drop, at, at, at - drop);
    }
    path
}

/// The starfield's three layers, each a lattice of dots.
fn starfield(width: i32, height: i32) -> [String; 3] {
    let (w, h) = (width as f32 + STAR_TILE.0, height as f32 + STAR_TILE.1);
    let layer = |per_tile: f32, offset: (f32, f32), radius: f32| {
        let step = (STAR_TILE.0 / per_tile, STAR_TILE.1 / per_tile);
        let mut path = String::new();
        let mut y = offset.1;
        while y < h {
            let mut x = offset.0;
            while x < w {
                let _ = write!(path, "M{:.2} {y:.2}a{radius} {radius} 0 1 0 {d} 0a{radius} {radius} 0 1 0 -{d} 0Z", x - radius, d = radius * 2.0);
                x += step.0;
            }
            y += step.1;
        }
        path
    };
    [layer(4.0, (0.0, 0.0), 1.2), layer(3.0, (31.0, 17.0), 1.2), layer(2.0, (55.0, 44.0), 1.55)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_ui::PatternKind;

    #[test]
    fn the_patterns_cover_the_window_and_repeat_on_their_tiles() {
        assert!((GRID_TILE.0 * 30f32.to_radians().tan() - GRID_TILE.1).abs() < 0.01);
        assert!((GRID_TILE.1 * 30f32.to_radians().cos() - 46.0).abs() < 0.01, "the lines are 46 apart");
        let grid = isometric_grid(1280, 800);
        let moves = grid.matches('M').count();
        assert!(moves >= 2 * ((800.0 + 53.116 + 1372.0 * 0.5774) / 53.116) as usize, "{moves} lines");
        let pet = pet_grid(1280, 800);
        assert!(pet.matches('M').count() >= 80);
        let [stars, far, bright] = starfield(1280, 800);
        for (layer, per_tile) in [(&stars, 4.0), (&far, 3.0), (&bright, 2.0)] {
            let step = STAR_TILE.0 / per_tile;
            assert!((step * per_tile - STAR_TILE.0).abs() < 0.001);
            let dots = layer.matches('Z').count();
            let across = ((1280.0 + STAR_TILE.0) / step).floor() as usize;
            assert!(dots >= across * ((800.0 + STAR_TILE.1) / (STAR_TILE.1 / per_tile)).floor() as usize, "{dots} dots");
        }
        assert!(hexagons(1280, 800).starts_with("M0.00 36.38V12.12L21.00 0.00L42.00 12.12"));
    }

    #[test]
    fn each_game_reads_its_look_from_its_content() {
        for game in ["exe4", "exe5", "exe6"] {
            let dir = crate::contents::dir(game).unwrap_or_else(|| panic!("{game} has no content"));
            assert!(read_content(&dir, game).is_some(), "{game}'s look");
        }
    }

    #[test]
    fn a_look_blends_from_the_one_before() {
        let mut old = Look { pattern: PatternKind::Hexagons, ..Default::default() };
        old.panel = Color::from_rgb_u8(0, 0, 0);
        old.frame_stroke = Color::from_rgb_u8(0, 0, 0);
        let mut new = Look { pattern: PatternKind::Starfield, pattern_strength: 0.22, ..Default::default() };
        new.panel = Color::from_rgb_u8(200, 100, 0);
        new.frame_stroke = Color::from_rgb_u8(100, 200, 50);
        assert_eq!(blend(&new, &old, 1.0), new);
        let half = blend(&new, &old, 0.5);
        assert_eq!((half.panel.red(), half.panel.green(), half.panel.blue()), (100, 50, 0));
        assert_eq!((half.frame_stroke.red(), half.frame_stroke.green(), half.frame_stroke.blue()), (50, 100, 25));
        assert_eq!((half.pattern, half.pattern_strength), (PatternKind::Starfield, 0.22));
    }
}
