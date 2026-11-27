//! Client-local online match Stage (background) & BGM presentation configuration (`S-008`).
//!
//! All modes operate strictly on the local frontend presentation layer (`Renderer` background
//! override + `BattleAudio` song/bank selection) and never alter `BattleSettings`, simulation
//! state, RNG, or netplay wire compatibility.

use crate::battle::BattlePresentation;
use crate::games::{Games, Ready};
use crate::music::{self, Music};
use crate::screens::practice;
use crate::settings::FavoriteBgmMode;
use saito_battle::Content;
use saito_battle::content::{BackgroundId, pool_key};
use saito_frontend::player::{RoundContext, RoundLook};
use saito_frontend::preload::SoundParts;
use saito_match::pick::Picks;
use std::collections::BTreeMap;

/// Global favorite battle BGM configuration applied in Mode A (`S-135`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FavoriteBgmConfig<'a> {
    pub mode: FavoriteBgmMode,
    pub favorites: &'a [String],
}

const REMEMBERED_STAGE_MODE: &str = "play.stage-mode";
const REMEMBERED_STAGE_ROUNDS: &str = "play.stage-rounds";

/// Background prefixes/keys reserved for boss/final-stage or postgame encounters, excluded from
/// the "normal" tier in [`StageMode::Escalating`].
const NON_REGULAR_BACKGROUNDS: &[&str] = &[
    "black-earth",
    "darksoul",
    "duo",
    "meteor-comp",
    "factory-comp",
    "soulserv-comp",
    "nebula-area",
    "nebulagray",
    "copybot-comp",
    "underground",
    "graveyard",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum StageMode {
    #[default]
    Default,
    Random,
    Escalating,
    Custom,
}

impl StageMode {
    pub fn to_int(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::Random => 1,
            Self::Escalating => 2,
            Self::Custom => 3,
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Random => "random",
            Self::Escalating => "escalating",
            Self::Custom => "custom",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key.trim() {
            "random" => Self::Random,
            "escalating" => Self::Escalating,
            "custom" => Self::Custom,
            _ => Self::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EscalationTier {
    Normal,
    SingleMatchPoint,
    DoubleMatchPoint,
}

/// Computes the escalating intensity tier from the current score and match target.
///
/// A 1-win match (`first_to <= 1`) is immediately both players' match point, so it starts at
/// [`EscalationTier::DoubleMatchPoint`].
pub(crate) fn escalation_tier(wins: u8, losses: u8, first_to: u8) -> EscalationTier {
    if first_to <= 1 {
        return EscalationTier::DoubleMatchPoint;
    }
    let match_point = first_to - 1;
    match (wins >= match_point, losses >= match_point) {
        (true, true) => EscalationTier::DoubleMatchPoint,
        (true, false) | (false, true) => EscalationTier::SingleMatchPoint,
        (false, false) => EscalationTier::Normal,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum RoundBg {
    #[default]
    Random,
    /// Preserve the round's native `SMTC` background (used when recording favorite-only BGM overrides).
    Default,
    /// Canonical `<game>:<background>` pool key.
    Named(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum RoundBgm {
    #[default]
    ByBackground,
    Random,
    Theme {
        game: String,
        song: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RoundCustom {
    pub bg: RoundBg,
    pub bgm: RoundBgm,
}

impl RoundCustom {
    pub fn is_default(&self) -> bool {
        self.bg == RoundBg::Random && self.bgm == RoundBgm::ByBackground
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StageConfig {
    pub mode: StageMode,
    pub rounds: Vec<RoundCustom>,
}

impl StageConfig {
    pub fn load(remembered: &BTreeMap<String, String>) -> Self {
        let mode = remembered.get(REMEMBERED_STAGE_MODE).map(|s| StageMode::from_key(s)).unwrap_or_default();
        let max_rounds = saito_match::rounds_for(saito_match::MAX_FIRST_TO);
        let rounds = remembered
            .get(REMEMBERED_STAGE_ROUNDS)
            .map(|raw| raw.split(';').take(max_rounds).map(decode_round).collect())
            .unwrap_or_default();
        Self { mode, rounds }
    }

    pub fn save(&self, remembered: &mut BTreeMap<String, String>) {
        if self.mode == StageMode::Default {
            remembered.remove(REMEMBERED_STAGE_MODE);
        } else {
            remembered.insert(REMEMBERED_STAGE_MODE.into(), self.mode.as_key().into());
        }
        let used_len = self.rounds.iter().rposition(|r| !r.is_default()).map_or(0, |last| last + 1);
        if used_len == 0 {
            remembered.remove(REMEMBERED_STAGE_ROUNDS);
        } else {
            let encoded: Vec<String> = self.rounds[..used_len].iter().map(encode_round).collect();
            remembered.insert(REMEMBERED_STAGE_ROUNDS.into(), encoded.join(";"));
        }
    }

    pub fn ensure_rounds(&mut self, count: usize) {
        if self.rounds.len() < count {
            self.rounds.resize_with(count, RoundCustom::default);
        }
    }

    pub fn round_bg(&self, index: usize) -> &RoundBg {
        self.rounds.get(index).map_or(&RoundBg::Random, |r| &r.bg)
    }

    pub fn round_bgm(&self, index: usize) -> &RoundBgm {
        self.rounds.get(index).map_or(&RoundBgm::ByBackground, |r| &r.bgm)
    }

    pub fn set_round_bg(&mut self, index: usize, bg: RoundBg) {
        self.ensure_rounds(index + 1);
        self.rounds[index].bg = bg;
    }

    pub fn set_round_bgm(&mut self, index: usize, bgm: RoundBgm) {
        self.ensure_rounds(index + 1);
        self.rounds[index].bgm = bgm;
    }

    pub fn reset_custom(&mut self) {
        self.rounds.clear();
    }

    /// Encode non-default stage/BGM configuration for recording in a replay's `SINF` section (`S-088`).
    /// Returns an empty string for [`StageMode::Default`] so default replays carry no extra metadata.
    #[cfg(test)]
    pub fn encode_presentation(&self) -> String {
        match self.mode {
            StageMode::Default => String::new(),
            StageMode::Random => "v1|random".into(),
            StageMode::Escalating => "v1|escalating".into(),
            StageMode::Custom => {
                let used_len = self.rounds.iter().rposition(|r| !r.is_default()).map_or(0, |last| last + 1);
                if used_len == 0 {
                    "v1|custom|".into()
                } else {
                    let encoded: Vec<String> = self.rounds[..used_len].iter().map(encode_round).collect();
                    format!("v1|custom|{}", encoded.join(";"))
                }
            }
        }
    }

    /// Decode a recorded replay presentation string produced by [`StageConfig::encode_presentation`].
    pub fn decode_presentation(raw: &str) -> Option<StageConfig> {
        let rest = raw.strip_prefix("v1|")?;
        if rest == "random" {
            return Some(StageConfig { mode: StageMode::Random, rounds: Vec::new() });
        }
        if rest == "escalating" {
            return Some(StageConfig { mode: StageMode::Escalating, rounds: Vec::new() });
        }
        if let Some(rounds_raw) = rest.strip_prefix("custom|") {
            let max_rounds = saito_match::rounds_for(saito_match::MAX_FIRST_TO);
            let rounds = if rounds_raw.is_empty() {
                Vec::new()
            } else {
                rounds_raw.split(';').take(max_rounds).map(decode_round).collect()
            };
            return Some(StageConfig { mode: StageMode::Custom, rounds });
        }
        None
    }

    /// Build the client-local [`BattlePresentation`] attached when an online match starts loading.
    pub fn build_presentation(&self, games: &Games, ready: &Ready, seed: u32, first_to: u8) -> BattlePresentation {
        self.build_resolved(games, ready, seed, first_to, FavoriteBgmConfig::default()).0
    }

    /// Build the client-local [`BattlePresentation`] for a live match together with the exact
    /// deterministic presentation string to record in the replay's `SINF` section (`S-088`, `S-129`).
    ///
    /// For [`StageMode::Default`], returns `(None, String::new())` so live matches use the native
    /// `SMTC` round backgrounds and paired music — matching replay playback, thumbnails, and the
    /// menu demo identically.
    #[cfg(test)]
    pub fn resolve_presentation(&self, games: &Games, ready: &Ready, seed: u32, first_to: u8) -> (Option<BattlePresentation>, String) {
        self.resolve_presentation_with_favorites(games, ready, seed, first_to, FavoriteBgmConfig::default())
    }

    /// Like [`StageConfig::resolve_presentation`], applying the player's global favorite battle BGM
    /// setting in Mode A (`S-135`): when enabled and at least one favorited track is available,
    /// each round automatically switches to a favorite track (while `StageMode::Custom` still
    /// respects explicitly chosen single-track round BGMs).
    pub fn resolve_presentation_with_favorites(
        &self,
        games: &Games,
        ready: &Ready,
        seed: u32,
        first_to: u8,
        fav: FavoriteBgmConfig<'_>,
    ) -> (Option<BattlePresentation>, String) {
        let fav_active = fav.mode != FavoriteBgmMode::Off && !music::resolve_favorites(games, ready, fav.favorites).is_empty();
        if self.mode == StageMode::Default && !fav_active {
            return (None, String::new());
        }
        let (pres, recorded) = self.build_resolved(games, ready, seed, first_to, fav);
        (Some(pres), recorded)
    }

    fn build_resolved(&self, games: &Games, ready: &Ready, seed: u32, first_to: u8, fav: FavoriteBgmConfig<'_>) -> (BattlePresentation, String) {
        let content = ready.content().clone();
        let max_rounds = saito_match::rounds_for(first_to.clamp(1, saito_match::MAX_FIRST_TO));
        let own_sound = music::sound(games, ready, &Music::Background);
        let fav_pool = if fav.mode != FavoriteBgmMode::Off {
            music::resolve_favorites(games, ready, fav.favorites)
        } else {
            Vec::new()
        };

        match self.mode {
            StageMode::Default => {
                if !fav_pool.is_empty() {
                    let mut pre: Vec<SoundParts> = Vec::with_capacity(max_rounds);
                    let mut recorded_rounds: Vec<RoundCustom> = Vec::with_capacity(max_rounds);
                    let mut last_pick = None;
                    for r in 0..max_rounds {
                        let idx = music::pick_favorite_index(fav.mode, fav_pool.len(), seed, r as u32, last_pick);
                        last_pick = Some(idx);
                        let (g, s, parts) = &fav_pool[idx];
                        pre.push(parts.clone());
                        recorded_rounds.push(RoundCustom {
                            bg: RoundBg::Default,
                            bgm: RoundBgm::Theme { game: g.clone(), song: s.clone() },
                        });
                    }
                    let recorded = encode_resolved_custom(&recorded_rounds);
                    let fav_mode = fav.mode;
                    let resolver = Box::new(move |ctx: RoundContext| {
                        let sound = pre.get(ctx.round as usize).cloned().or_else(|| {
                            let idx = music::pick_favorite_index(fav_mode, fav_pool.len(), seed, ctx.round as u32, None);
                            Some(fav_pool[idx].2.clone())
                        });
                        RoundLook { background: Some(ctx.default_background), sound }
                    });
                    return (BattlePresentation { extra_backgrounds: Vec::new(), first_background: None, resolver }, recorded);
                }
                let round_music = music::round_music(games, ready, &Music::Background);
                let pre_sound = own_sound.clone();
                let resolver = Box::new(move |ctx: RoundContext| {
                    let sound = round_music
                        .as_ref()
                        .map(|rm| rm.parts(ctx.default_background).clone())
                        .or_else(|| pre_sound.clone());
                    RoundLook { background: Some(ctx.default_background), sound }
                });
                (BattlePresentation { extra_backgrounds: Vec::new(), first_background: None, resolver }, String::new())
            }
            StageMode::Random => {
                let pool_bgs = available_pool_backgrounds(&content, &ready.loaded.game.others);
                let themes = available_themes_named(games, ready);
                let mut bg_picks = Picks::new(seed ^ 0x5241_4E44);
                let mut bgm_picks = Picks::new(seed ^ 0x4247_4D52);
                let mut pre: Vec<RoundLook> = Vec::with_capacity(max_rounds);
                let mut recorded_rounds: Vec<RoundCustom> = Vec::with_capacity(max_rounds);
                let mut last_fav = None;
                for r in 0..max_rounds {
                    let bg = (!pool_bgs.is_empty()).then(|| pool_bgs[bg_picks.below(pool_bgs.len())]);
                    let picked_theme = if !fav_pool.is_empty() {
                        let idx = music::pick_favorite_index(fav.mode, fav_pool.len(), seed, r as u32, last_fav);
                        last_fav = Some(idx);
                        Some(&fav_pool[idx])
                    } else {
                        (!themes.is_empty()).then(|| &themes[bgm_picks.below(themes.len())])
                    };
                    let sound = picked_theme.map(|(_, _, parts)| parts.clone()).or_else(|| own_sound.clone());
                    pre.push(RoundLook { background: bg, sound });
                    recorded_rounds.push(RoundCustom {
                        bg: bg
                            .and_then(|id| content.background_of(id))
                            .map_or(RoundBg::Random, |(g, n)| RoundBg::Named(pool_key(g, n))),
                        bgm: picked_theme.map_or(RoundBgm::ByBackground, |(g, s, _)| RoundBgm::Theme {
                            game: g.clone(),
                            song: s.clone(),
                        }),
                    });
                }
                let recorded = encode_resolved_custom(&recorded_rounds);
                let first_background = pre.first().and_then(|l| l.background);
                let extra_backgrounds: Vec<BackgroundId> = pre.iter().filter_map(|l| l.background).collect();
                let fav_mode = fav.mode;
                let resolver = Box::new(move |ctx: RoundContext| {
                    if let Some(look) = pre.get(ctx.round as usize) {
                        return look.clone();
                    }
                    let bg = (!pool_bgs.is_empty()).then(|| {
                        let mut p = Picks::new(seed ^ 0x5241_4E44 ^ ((ctx.round as u32) << 16));
                        pool_bgs[p.below(pool_bgs.len())]
                    });
                    let sound = if !fav_pool.is_empty() {
                        let idx = music::pick_favorite_index(fav_mode, fav_pool.len(), seed, ctx.round as u32, None);
                        Some(fav_pool[idx].2.clone())
                    } else if !themes.is_empty() {
                        let mut p = Picks::new(seed ^ 0x4247_4D52 ^ ((ctx.round as u32) << 16));
                        Some(themes[p.below(themes.len())].2.clone())
                    } else {
                        own_sound.clone()
                    };
                    RoundLook { background: bg, sound }
                });
                (BattlePresentation { extra_backgrounds, first_background, resolver }, recorded)
            }
            StageMode::Escalating => {
                if !fav_pool.is_empty() {
                    let mut per_round: Vec<(String, String, SoundParts)> = Vec::with_capacity(max_rounds);
                    let mut last_fav = None;
                    for r in 0..max_rounds {
                        let idx = music::pick_favorite_index(fav.mode, fav_pool.len(), seed, r as u32, last_fav);
                        last_fav = Some(idx);
                        per_round.push(fav_pool[idx].clone());
                    }
                    let recorded_tracks: Vec<String> = per_round
                        .iter()
                        .map(|(g, s, _)| format!("{}/{}", g.replace(['/', '|', ';'], ""), s.replace(['/', '|', ';'], "")))
                        .collect();
                    let recorded = format!("v3|escalating-fav|{}", recorded_tracks.join(";"));
                    let pres = build_escalating_favorites_presentation(games, ready, seed, first_to, per_round);
                    return (pres, recorded);
                }
                let songs = escalating_song_names(ready, None);
                let pres = build_escalating_presentation(games, ready, seed, first_to, &songs);
                let recorded = format!("v2|escalating|{}|{}|{}", songs[0], songs[1], songs[2]);
                (pres, recorded)
            }
            StageMode::Custom => {
                let pool_bgs = available_pool_backgrounds(&content, &ready.loaded.game.others);
                let themes = available_themes_named(games, ready);
                let round_music = music::round_music(games, ready, &Music::Background);
                let mut bg_picks = Picks::new(seed ^ 0x4355_5354);
                let mut bgm_picks = Picks::new(seed ^ 0x4342_474D);
                let mut pre: Vec<RoundLook> = Vec::with_capacity(max_rounds);
                let mut recorded_rounds: Vec<RoundCustom> = Vec::with_capacity(max_rounds);
                let mut last_fav = None;

                for r in 0..max_rounds {
                    let rand_bg = (!pool_bgs.is_empty()).then(|| pool_bgs[bg_picks.below(pool_bgs.len())]);
                    let rand_theme = (!themes.is_empty()).then(|| &themes[bgm_picks.below(themes.len())]);
                    let cfg = self.rounds.get(r).cloned().unwrap_or_default();

                    let bg = match &cfg.bg {
                        RoundBg::Default => None,
                        RoundBg::Random => rand_bg,
                        RoundBg::Named(canonical) => resolve_background(&content, &ready.loaded.game.others, canonical).or(rand_bg),
                    };
                    let (sound, recorded_bgm) = match &cfg.bgm {
                        RoundBgm::Theme { game, song } => (
                            resolve_theme(games, ready, game, song).or_else(|| own_sound.clone()),
                            RoundBgm::Theme { game: game.clone(), song: song.clone() },
                        ),
                        RoundBgm::ByBackground | RoundBgm::Random if !fav_pool.is_empty() => {
                            let idx = music::pick_favorite_index(fav.mode, fav_pool.len(), seed, r as u32, last_fav);
                            last_fav = Some(idx);
                            let (g, s, parts) = &fav_pool[idx];
                            (Some(parts.clone()), RoundBgm::Theme { game: g.clone(), song: s.clone() })
                        }
                        RoundBgm::ByBackground => (
                            bg.and_then(|id| round_music.as_ref().map(|rm| rm.parts(id).clone()))
                                .or_else(|| own_sound.clone()),
                            RoundBgm::ByBackground,
                        ),
                        RoundBgm::Random => (
                            rand_theme.map(|(_, _, parts)| parts.clone()).or_else(|| own_sound.clone()),
                            rand_theme.map_or(RoundBgm::ByBackground, |(g, s, _)| RoundBgm::Theme {
                                game: g.clone(),
                                song: s.clone(),
                            }),
                        ),
                    };
                    pre.push(RoundLook { background: bg, sound });
                    recorded_rounds.push(RoundCustom {
                        bg: if cfg.bg == RoundBg::Default {
                            RoundBg::Default
                        } else {
                            bg.and_then(|id| content.background_of(id))
                                .map_or_else(|| cfg.bg.clone(), |(g, n)| RoundBg::Named(pool_key(g, n)))
                        },
                        bgm: recorded_bgm,
                    });
                }

                let recorded = encode_resolved_custom(&recorded_rounds);
                let first_background = pre.first().and_then(|l| l.background);
                let extra_backgrounds: Vec<BackgroundId> = pre.iter().filter_map(|l| l.background).collect();
                let resolver = Box::new(move |ctx: RoundContext| {
                    pre.get(ctx.round as usize)
                        .cloned()
                        .map(|mut look| {
                            if look.background.is_none() {
                                look.background = Some(ctx.default_background);
                            }
                            look
                        })
                        .unwrap_or_else(|| RoundLook {
                            background: Some(ctx.default_background),
                            sound: own_sound.clone(),
                        })
                });
                (BattlePresentation { extra_backgrounds, first_background, resolver }, recorded)
            }
        }
    }
}

fn encode_resolved_custom(rounds: &[RoundCustom]) -> String {
    let used_len = rounds.iter().rposition(|r| !r.is_default()).map_or(0, |last| last + 1);
    if used_len == 0 {
        "v1|custom|".into()
    } else {
        let encoded: Vec<String> = rounds[..used_len].iter().map(encode_round).collect();
        format!("v1|custom|{}", encoded.join(";"))
    }
}

fn escalating_song_names(ready: &Ready, pinned: Option<[&str; 3]>) -> [String; 3] {
    if let Some([a, b, c]) = pinned {
        return [a.to_string(), b.to_string(), c.to_string()];
    }
    let pick = |gba_song: &str| {
        if music::prefer_ds_music() {
            ready.loaded.ds_counterpart(gba_song).unwrap_or(gba_song).to_string()
        } else {
            gba_song.to_string()
        }
    };
    [pick("virus-battle"), pick("boss-battle"), pick("final-battle")]
}

fn build_escalating_presentation(
    games: &Games,
    ready: &Ready,
    seed: u32,
    first_to: u8,
    songs: &[String; 3],
) -> BattlePresentation {
    let content = ready.content().clone();
    let own_sound = music::sound(games, ready, &Music::Background);
    let normal_bgs = escalating_backgrounds(&content, EscalationTier::Normal);
    let single_mp_bgs = escalating_backgrounds(&content, EscalationTier::SingleMatchPoint);
    let double_mp_bgs = escalating_backgrounds(&content, EscalationTier::DoubleMatchPoint);
    let normal_bgm = theme_for_game(games, ready, &songs[0]).or_else(|| own_sound.clone());
    let single_mp_bgm = theme_for_game(games, ready, &songs[1]).or_else(|| own_sound.clone());
    let double_mp_bgm = theme_for_game(games, ready, &songs[2]).or_else(|| own_sound.clone());

    let pick_from = |round: u8, list: &[BackgroundId]| -> Option<BackgroundId> {
        if list.is_empty() {
            return None;
        }
        let mut p = Picks::new(seed ^ 0x4553_4341 ^ ((round as u32) << 8));
        Some(list[p.below(list.len())])
    };

    let first_tier = escalation_tier(0, 0, first_to);
    let first_background = match first_tier {
        EscalationTier::Normal => pick_from(0, &normal_bgs),
        EscalationTier::SingleMatchPoint => pick_from(0, &single_mp_bgs),
        EscalationTier::DoubleMatchPoint => pick_from(0, &double_mp_bgs),
    };
    let mut extra_backgrounds = Vec::new();
    for id in normal_bgs.iter().chain(&single_mp_bgs).chain(&double_mp_bgs).copied() {
        if !extra_backgrounds.contains(&id) {
            extra_backgrounds.push(id);
        }
    }
    let resolver = Box::new(move |ctx: RoundContext| {
        let tier = escalation_tier(ctx.wins, ctx.losses, ctx.first_to);
        let (bgs, bgm) = match tier {
            EscalationTier::Normal => (&normal_bgs, &normal_bgm),
            EscalationTier::SingleMatchPoint => (&single_mp_bgs, &single_mp_bgm),
            EscalationTier::DoubleMatchPoint => (&double_mp_bgs, &double_mp_bgm),
        };
        let mut p = Picks::new(seed ^ 0x4553_4341 ^ ((ctx.round as u32) << 8));
        let background = (!bgs.is_empty()).then(|| bgs[p.below(bgs.len())]);
        RoundLook { background, sound: bgm.clone() }
    });
    BattlePresentation { extra_backgrounds, first_background, resolver }
}

fn build_escalating_favorites_presentation(
    games: &Games,
    ready: &Ready,
    seed: u32,
    first_to: u8,
    per_round: Vec<(String, String, SoundParts)>,
) -> BattlePresentation {
    let content = ready.content().clone();
    let own_sound = music::sound(games, ready, &Music::Background);
    let normal_bgs = escalating_backgrounds(&content, EscalationTier::Normal);
    let single_mp_bgs = escalating_backgrounds(&content, EscalationTier::SingleMatchPoint);
    let double_mp_bgs = escalating_backgrounds(&content, EscalationTier::DoubleMatchPoint);

    let pick_from = |round: u8, list: &[BackgroundId]| -> Option<BackgroundId> {
        if list.is_empty() {
            return None;
        }
        let mut p = Picks::new(seed ^ 0x4553_4341 ^ ((round as u32) << 8));
        Some(list[p.below(list.len())])
    };

    let first_tier = escalation_tier(0, 0, first_to);
    let first_background = match first_tier {
        EscalationTier::Normal => pick_from(0, &normal_bgs),
        EscalationTier::SingleMatchPoint => pick_from(0, &single_mp_bgs),
        EscalationTier::DoubleMatchPoint => pick_from(0, &double_mp_bgs),
    };
    let mut extra_backgrounds = Vec::new();
    for id in normal_bgs.iter().chain(&single_mp_bgs).chain(&double_mp_bgs).copied() {
        if !extra_backgrounds.contains(&id) {
            extra_backgrounds.push(id);
        }
    }
    let resolver = Box::new(move |ctx: RoundContext| {
        let tier = escalation_tier(ctx.wins, ctx.losses, ctx.first_to);
        let bgs = match tier {
            EscalationTier::Normal => &normal_bgs,
            EscalationTier::SingleMatchPoint => &single_mp_bgs,
            EscalationTier::DoubleMatchPoint => &double_mp_bgs,
        };
        let mut p = Picks::new(seed ^ 0x4553_4341 ^ ((ctx.round as u32) << 8));
        let background = (!bgs.is_empty()).then(|| bgs[p.below(bgs.len())]);
        let sound = if !per_round.is_empty() {
            Some(per_round[ctx.round as usize % per_round.len()].2.clone())
        } else {
            own_sound.clone()
        };
        RoundLook { background, sound }
    });
    BattlePresentation { extra_backgrounds, first_background, resolver }
}

fn decode_v3_escalating_fav(raw: &str) -> Option<Vec<(String, String)>> {
    let rest = raw.strip_prefix("v3|escalating-fav|")?;
    let tracks: Vec<(String, String)> = rest
        .split(';')
        .filter_map(|item| {
            let (g, s) = item.split_once('/')?;
            (!g.is_empty() && !s.is_empty()).then(|| (g.to_string(), s.to_string()))
        })
        .collect();
    (!tracks.is_empty()).then_some(tracks)
}

fn decode_v2_escalating(raw: &str) -> Option<[&str; 3]> {
    let rest = raw.strip_prefix("v2|escalating|")?;
    let mut parts = rest.split('|');
    let a = parts.next().filter(|s| !s.is_empty())?;
    let b = parts.next().filter(|s| !s.is_empty())?;
    let c = parts.next().filter(|s| !s.is_empty())?;
    if parts.next().is_some() {
        return None;
    }
    Some([a, b, c])
}

/// Rebuild the [`BattlePresentation`] recorded in `replay.info.presentation` (`S-088`, `S-129`), if any.
pub(crate) fn replay_presentation(games: &Games, ready: &Ready, replay: &saito_frontend::replay::Replay) -> Option<BattlePresentation> {
    let raw = replay.info.presentation.as_str();
    if raw.is_empty() {
        return None;
    }
    let m = saito_match::binary::read_match(ready.content(), &replay.match_bytes).ok()?;
    let seed = m.seed.unwrap_or(0);
    if let Some(tracks) = decode_v3_escalating_fav(raw) {
        let own_sound = music::sound(games, ready, &Music::Background);
        let per_round: Vec<(String, String, SoundParts)> = tracks
            .into_iter()
            .filter_map(|(g, s)| {
                let parts = resolve_theme(games, ready, &g, &s).or_else(|| own_sound.clone())?;
                Some((g, s, parts))
            })
            .collect();
        return Some(build_escalating_favorites_presentation(games, ready, seed, m.first_to(), per_round));
    }
    if let Some(pinned) = decode_v2_escalating(raw) {
        let songs = escalating_song_names(ready, Some(pinned));
        return Some(build_escalating_presentation(games, ready, seed, m.first_to(), &songs));
    }
    let cfg = StageConfig::decode_presentation(raw)?;
    Some(cfg.build_presentation(games, ready, seed, m.first_to()))
}

/// Resolve Round 0's presentation background for a replay still preview without touching audio (`S-129`).
pub(crate) fn replay_first_background(ready: &Ready, presentation: &str, match_bytes: &[u8]) -> Option<BackgroundId> {
    if presentation.is_empty() {
        return None;
    }
    let content = ready.content();
    let m = saito_match::binary::read_match(content, match_bytes).ok()?;
    let seed = m.seed.unwrap_or(0);
    let first_to = m.first_to();
    if decode_v2_escalating(presentation).is_some() || decode_v3_escalating_fav(presentation).is_some() {
        let tier = escalation_tier(0, 0, first_to);
        let list = escalating_backgrounds(content, tier);
        if list.is_empty() {
            return None;
        }
        let mut p = Picks::new(seed ^ 0x4553_4341);
        return Some(list[p.below(list.len())]);
    }
    let cfg = StageConfig::decode_presentation(presentation)?;
    match cfg.mode {
        StageMode::Default => None,
        StageMode::Random => {
            let pool_bgs = available_pool_backgrounds(content, &ready.loaded.game.others);
            let mut bg_picks = Picks::new(seed ^ 0x5241_4E44);
            (!pool_bgs.is_empty()).then(|| pool_bgs[bg_picks.below(pool_bgs.len())])
        }
        StageMode::Escalating => {
            let tier = escalation_tier(0, 0, first_to);
            let list = escalating_backgrounds(content, tier);
            let mut p = Picks::new(seed ^ 0x4553_4341);
            (!list.is_empty()).then(|| list[p.below(list.len())])
        }
        StageMode::Custom => {
            let pool_bgs = available_pool_backgrounds(content, &ready.loaded.game.others);
            let mut bg_picks = Picks::new(seed ^ 0x4355_5354);
            let rand_bg = (!pool_bgs.is_empty()).then(|| pool_bgs[bg_picks.below(pool_bgs.len())]);
            match cfg.rounds.first().map(|r| &r.bg) {
                Some(RoundBg::Default) => None,
                Some(RoundBg::Named(canonical)) => resolve_background(content, &ready.loaded.game.others, canonical).or(rand_bg),
                _ => rand_bg,
            }
        }
    }
}

/// Canonicalizes a background selection index from `practice::background_list(content, game)`
/// (where 0 = Random, 1.. = list entry) into a [`RoundBg`].
pub(crate) fn round_bg_from_list_index(content: &Content, game: &str, backgrounds: &[String], index: usize) -> RoundBg {
    let Some(written) = index.checked_sub(1).and_then(|i| backgrounds.get(i)) else {
        return RoundBg::Random;
    };
    match practice::pool_background_parts(content, game, written) {
        Some((of, name)) => RoundBg::Named(pool_key(of, name)),
        None => RoundBg::Named(pool_key(game, written)),
    }
}

/// Returns the picker list index (0 = Random, 1.. = list entry) for a stored [`RoundBg`].
pub(crate) fn list_index_for_round_bg(content: &Content, game: &str, backgrounds: &[String], bg: &RoundBg) -> usize {
    let RoundBg::Named(canonical) = bg else {
        return 0;
    };
    let Some((target_game, target_name)) = split_bg_key(canonical, game) else {
        return 0;
    };
    backgrounds
        .iter()
        .position(|written| practice::pool_background_parts(content, game, written) == Some((target_game, target_name)))
        .map_or(0, |i| i + 1)
}

/// Formats a [`RoundBg`] for UI display.
pub(crate) fn round_bg_label(bg: &RoundBg, random_label: &str) -> String {
    match bg {
        RoundBg::Random | RoundBg::Default => random_label.to_string(),
        RoundBg::Named(canonical) => match canonical.split_once(':') {
            Some((game, name)) => crate::backgrounds::name(game, name),
            None => canonical.clone(),
        },
    }
}

/// Offered BGM choices for [`StageMode::Custom`]: `[ByBackground, Random, ...themes]`.
pub(crate) fn bgm_choices(games: &Games) -> Vec<RoundBgm> {
    let mut out = vec![RoundBgm::ByBackground, RoundBgm::Random];
    for m in music::offered(games) {
        if let Music::Theme { game, song } = m {
            out.push(RoundBgm::Theme { game, song });
        }
    }
    out
}

/// Index into [`bgm_choices`], defaulting to `0` (`ByBackground`) if an external theme is unavailable.
pub(crate) fn bgm_choice_index(choices: &[RoundBgm], bgm: &RoundBgm) -> usize {
    choices.iter().position(|c| c == bgm).unwrap_or(0)
}

/// Formats a [`RoundBgm`] for UI display.
pub(crate) fn round_bgm_label(bgm: &RoundBgm, by_bg_label: &str, random_label: &str) -> String {
    match bgm {
        RoundBgm::ByBackground => by_bg_label.to_string(),
        RoundBgm::Random => random_label.to_string(),
        RoundBgm::Theme { game, song } => music::name(&Music::Theme { game: game.clone(), song: song.clone() }, ""),
    }
}

fn encode_round(round: &RoundCustom) -> String {
    let bg = match &round.bg {
        RoundBg::Random => String::new(),
        RoundBg::Default => "default".into(),
        RoundBg::Named(key) => key.replace(['|', ';'], ""),
    };
    let bgm = match &round.bgm {
        RoundBgm::ByBackground => String::new(),
        RoundBgm::Random => "random".into(),
        RoundBgm::Theme { game, song } => format!("{}/{}", game.replace(['/', '|', ';'], ""), song.replace(['/', '|', ';'], "")),
    };
    format!("{bg}|{bgm}")
}

fn decode_round(raw: &str) -> RoundCustom {
    let (bg_raw, bgm_raw) = raw.split_once('|').unwrap_or((raw, ""));
    let bg = match bg_raw {
        "" => RoundBg::Random,
        "default" => RoundBg::Default,
        other => RoundBg::Named(other.to_string()),
    };
    let bgm = match bgm_raw {
        "" => RoundBgm::ByBackground,
        "random" => RoundBgm::Random,
        other => match other.split_once('/') {
            Some((game, song)) if !game.is_empty() && !song.is_empty() => RoundBgm::Theme {
                game: game.to_string(),
                song: song.to_string(),
            },
            _ => RoundBgm::ByBackground,
        },
    };
    RoundCustom { bg, bgm }
}

fn split_bg_key<'a>(canonical: &'a str, fallback_game: &'a str) -> Option<(&'a str, &'a str)> {
    match canonical.split_once(':') {
        Some((game, name)) if !game.is_empty() && !name.is_empty() => Some((game, name)),
        None if !canonical.is_empty() => Some((fallback_game, canonical)),
        _ => None,
    }
}

fn resolve_background(content: &Content, others: &saito_render::graphics::OtherBackgrounds, canonical: &str) -> Option<BackgroundId> {
    let (game, name) = split_bg_key(canonical, content.game())?;
    let id = content.background_handle(game, name)?;
    if content.is_own_background(id) || others.game(game).is_some() {
        Some(id)
    } else {
        None
    }
}

fn available_pool_backgrounds(content: &Content, others: &saito_render::graphics::OtherBackgrounds) -> Vec<BackgroundId> {
    let pool: Vec<BackgroundId> = content
        .pool_backgrounds()
        .into_iter()
        .filter(|&id| content.is_own_background(id) || content.background_of(id).is_some_and(|(g, _)| others.game(g).is_some()))
        .collect();
    if !pool.is_empty() {
        return pool;
    }
    let game_pool = content.game_pool_backgrounds();
    if !game_pool.is_empty() {
        game_pool
    } else {
        default_link_backgrounds(content)
    }
}

fn default_link_backgrounds(content: &Content) -> Vec<BackgroundId> {
    let bgs = content.link_battle_backgrounds();
    if !bgs.is_empty() {
        bgs
    } else {
        content.stage_background().into_iter().collect()
    }
}

fn available_themes_named(games: &Games, ready: &Ready) -> Vec<(String, String, SoundParts)> {
    let mut parts = Vec::new();
    for m in music::offered(games) {
        if let Music::Theme { game, song } = &m
            && let Some(p) = music::sound(games, ready, &m)
        {
            parts.push((game.clone(), song.clone(), p));
        }
    }
    if parts.is_empty()
        && let Some(sound) = ready.loaded.sound.as_ref()
    {
        let own_game = ready.content().game().to_string();
        for song in ready.loaded.battle_themes() {
            if let Some(theme) = ready.loaded.theme(song) {
                parts.push((own_game.clone(), song.to_string(), sound.parts(ready.content(), Some(&theme))));
            }
        }
    }
    parts
}

fn resolve_theme(games: &Games, ready: &Ready, game: &str, song: &str) -> Option<SoundParts> {
    if let Some(parts) = music::sound(games, ready, &Music::Theme { game: game.to_string(), song: song.to_string() }) {
        // Note: `music::sound` returns game's own sound when `games.read(game)` is None; prefer `ready` directly if same game.
        if games.read(game).is_some() {
            return Some(parts);
        }
    }
    if game == ready.content().game()
        && let (Some(sound), Some(theme)) = (ready.loaded.sound.as_ref(), ready.loaded.theme(song))
    {
        return Some(sound.parts(ready.content(), Some(&theme)));
    }
    None
}

fn theme_for_game(games: &Games, ready: &Ready, song: &str) -> Option<SoundParts> {
    resolve_theme(games, ready, ready.content().game(), song)
}

fn escalating_backgrounds(content: &Content, tier: EscalationTier) -> Vec<BackgroundId> {
    let game = content.game();
    let named = |names: &[&str]| -> Vec<BackgroundId> { names.iter().filter_map(|n| content.background_handle(game, n)).collect() };

    let picked = match tier {
        EscalationTier::DoubleMatchPoint => {
            let specific = match game {
                "exe4" => named(&["duo"]),
                "exe5" => named(&["nebulagray"]),
                "exe6" => named(&["copybot-comp"]),
                _ => Vec::new(),
            };
            if !specific.is_empty() {
                specific
            } else {
                own_backgrounds_with_music(content, "final-battle", false)
            }
        }
        EscalationTier::SingleMatchPoint => {
            let specific = match game {
                "exe4" => named(&["meteor-comp"]),
                "exe5" => named(&["factory-comp", "soulserv-comp"]),
                "exe6" => named(&["underground"]),
                _ => Vec::new(),
            };
            if !specific.is_empty() {
                specific
            } else {
                own_backgrounds_with_music(content, "boss-battle", false)
            }
        }
        EscalationTier::Normal => {
            let regular = own_backgrounds_with_music(content, "virus-battle", true);
            if !regular.is_empty() {
                regular
            } else {
                own_backgrounds_with_music(content, "virus-battle", false)
            }
        }
    };

    if !picked.is_empty() {
        picked
    } else {
        default_link_backgrounds(content)
    }
}

fn own_backgrounds_with_music(content: &Content, song: &str, exclude_non_regular: bool) -> Vec<BackgroundId> {
    let game = content.game();
    content
        .background_pool
        .iter()
        .filter(|(g, name)| {
            *g == game
                && content.music_of_background(name) == Some(song)
                && (!exclude_non_regular || !NON_REGULAR_BACKGROUNDS.contains(name))
        })
        .filter_map(|(g, name)| content.background_handle(g, name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::Games;
    use saito_audio::{BankId, MUSIC_PLAYER, m4a::SongId};
    use saito_battle::Content;
    use saito_frontend::driver::LivePlayer;
    use saito_frontend::game::{Enhancements, Game, Loaded, Sound};
    use std::sync::Arc;

    fn loaded_game(content: Arc<Content>) -> Loaded {
        let mut bank = (*saito_frontend::testing::bank(&content)).clone();
        let (held, short) = (bank.songs[0].clone(), bank.songs[1].clone());
        let assets = saito_frontend::testing::numbered_by_handle(&content);
        let songs = saito_audio::Songs::of(&assets);
        let mut index = saito_content_api::AssetIndex::default();
        for (h, name) in content.assets.sounds.iter().enumerate() {
            index.sounds.insert(name.clone(), h as u16);
        }
        for theme in content.battle_themes() {
            let n = index.sounds.len() as u16;
            index.sounds.entry(theme.clone()).or_insert(n);
        }
        let music: Vec<u16> = content
            .battle_music()
            .into_iter()
            .map(|m| m.0)
            .chain(content.battle_themes().iter().filter_map(|t| index.sounds.get(t).copied()))
            .collect();
        let numbers = songs.0.iter().map(|s| s.song as usize + 1).max().unwrap_or(0).max(index.sounds.len());
        bank.songs = vec![short; numbers];
        for n in &music {
            bank.songs[*n as usize] = held.clone();
        }
        let bank = Arc::new(bank);
        Loaded {
            game: Game {
                name: content.game().to_string(),
                content: content.clone(),
                dir: saito_content::index::content(),
                graphics: Default::default(),
                sound: bank.clone(),
                ds_sound: None,
                ds_voices: None,
                numbers: Arc::new(assets),
                index: Arc::new(index),
                others: Default::default(),
                report: Default::default(),
            },
            graphics: saito_frontend::testing::graphics(&content, 0x7FFF),
            enhancements: Enhancements::NONE,
            font: None,
            sound: Some(Sound { bank, ds_bank: None, ds_voices: None, songs, report: Default::default() }),
        }
    }

    fn all_games() -> Games {
        Games::read_of(vec![
            ("exe4", loaded_game(saito_match::testing::exe4_content())),
            ("exe5", loaded_game(saito_match::testing::exe5_content())),
            ("exe6", loaded_game(saito_match::testing::exe6_content())),
        ])
    }

    fn song_num(games: &Games, game: &str, song: &str) -> u16 {
        games.read(game).unwrap().loaded.game.index.number(saito_content_api::AssetKind::Sound, song).unwrap()
    }

    fn resolved_song(sound: &Option<SoundParts>, content: &Content) -> Option<(usize, u16)> {
        let (banks, songs) = sound.as_ref()?;
        let handle = content.defs.roles().link_battle_music(true);
        let bank_song = songs.0.get(handle.0 as usize)?;
        let bank = banks.get(bank_song.bank.index())?;
        let _ = bank;
        Some((bank_song.bank.index(), bank_song.song))
    }

    #[test]
    fn config_persistence_roundtrips_and_trims_defaults() {
        let mut remembered = BTreeMap::new();
        let mut cfg = StageConfig {
            mode: StageMode::Custom,
            rounds: vec![
                RoundCustom {
                    bg: RoundBg::Named("exe6:graveyard".into()),
                    bgm: RoundBgm::Theme { game: "exe4".into(), song: "tournament-battle".into() },
                },
                RoundCustom::default(),
                RoundCustom { bg: RoundBg::Random, bgm: RoundBgm::Random },
                RoundCustom::default(),
            ],
        };
        cfg.save(&mut remembered);
        assert_eq!(remembered.get(REMEMBERED_STAGE_MODE).map(String::as_str), Some("custom"));
        assert!(remembered.contains_key(REMEMBERED_STAGE_ROUNDS));

        let loaded = StageConfig::load(&remembered);
        assert_eq!(loaded.mode, StageMode::Custom);
        assert_eq!(loaded.rounds.len(), 3, "trailing defaults trimmed, middle default kept");
        assert_eq!(loaded.rounds[0], cfg.rounds[0]);
        assert_eq!(loaded.rounds[1], RoundCustom::default());
        assert_eq!(loaded.rounds[2], cfg.rounds[2]);

        cfg.mode = StageMode::Default;
        cfg.reset_custom();
        cfg.save(&mut remembered);
        assert!(!remembered.contains_key(REMEMBERED_STAGE_MODE));
        assert!(!remembered.contains_key(REMEMBERED_STAGE_ROUNDS));

        // Malformed saved strings degrade gracefully without panicking.
        remembered.insert(REMEMBERED_STAGE_MODE.into(), "bogus".into());
        remembered.insert(REMEMBERED_STAGE_ROUNDS.into(), "|bad;exe5:comp|random;;|/onlysong".into());
        let bad = StageConfig::load(&remembered);
        assert_eq!(bad.mode, StageMode::Default);
        assert_eq!(bad.rounds[0], RoundCustom::default());
        assert_eq!(bad.rounds[1], RoundCustom { bg: RoundBg::Named("exe5:comp".into()), bgm: RoundBgm::Random });
        assert_eq!(bad.rounds[2], RoundCustom::default());
        assert_eq!(bad.rounds[3], RoundCustom::default());
    }

    #[test]
    fn escalation_tiers_match_score_and_bo1_rule() {
        // Single battle (FT1) -> immediately treated as double match point.
        assert_eq!(escalation_tier(0, 0, 1), EscalationTier::DoubleMatchPoint);

        // Triple battle (FT2).
        assert_eq!(escalation_tier(0, 0, 2), EscalationTier::Normal);
        assert_eq!(escalation_tier(1, 0, 2), EscalationTier::SingleMatchPoint);
        assert_eq!(escalation_tier(0, 1, 2), EscalationTier::SingleMatchPoint);
        assert_eq!(escalation_tier(1, 1, 2), EscalationTier::DoubleMatchPoint);

        // Best of 5 (FT3).
        assert_eq!(escalation_tier(0, 0, 3), EscalationTier::Normal);
        assert_eq!(escalation_tier(1, 0, 3), EscalationTier::Normal);
        assert_eq!(escalation_tier(1, 1, 3), EscalationTier::Normal);
        assert_eq!(escalation_tier(2, 0, 3), EscalationTier::SingleMatchPoint);
        assert_eq!(escalation_tier(2, 1, 3), EscalationTier::SingleMatchPoint);
        assert_eq!(escalation_tier(0, 2, 3), EscalationTier::SingleMatchPoint);
        assert_eq!(escalation_tier(2, 2, 3), EscalationTier::DoubleMatchPoint);
    }

    #[test]
    fn default_and_random_modes_pick_expected_pools() {
        let games = all_games();
        let ready = games.read("exe6").unwrap();
        let content = ready.content();
        let link_set = content.link_battle_backgrounds();
        let default_bg = content.stage_background().unwrap();

        let cfg = StageConfig { mode: StageMode::Default, ..Default::default() };
        let mut pres = cfg.build_presentation(&games, &ready, 42, 2);
        let look0 = (pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert!(link_set.contains(&look0.background.unwrap()));
        let virus_song = song_num(&games, "exe6", "virus-battle");
        assert_eq!(resolved_song(&look0.sound, content), Some((0, virus_song)));

        let random_cfg = StageConfig { mode: StageMode::Random, ..Default::default() };
        let mut rpres = random_cfg.build_presentation(&games, &ready, 99, 2);
        let r0 = (rpres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert!(r0.background.is_some());
        assert!(r0.sound.is_some());
    }

    #[test]
    fn escalating_mode_picks_authentic_stages_and_bgms_for_exe4_5_6() {
        let games = all_games();
        let cfg = StageConfig { mode: StageMode::Escalating, ..Default::default() };

        for (game, mp_allowed, final_bg) in [
            ("exe4", vec!["meteor-comp"], "duo"),
            ("exe5", vec!["factory-comp", "soulserv-comp"], "nebulagray"),
            ("exe6", vec!["underground"], "copybot-comp"),
        ] {
            let ready = games.read(game).unwrap();
            let content = ready.content();
            let default_bg = content.stage_background().unwrap();

            // FT1 -> starts on Final Boss directly.
            let mut bo1 = cfg.build_presentation(&games, &ready, 7, 1);
            let bo1_name = content.background_of(bo1.first_background.unwrap()).unwrap().1.to_string();
            assert_eq!(bo1_name, final_bg, "{game} FT1 first background");
            let bo1_look = (bo1.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 1, first_to: 1, default_background: default_bg });
            assert_eq!(resolved_song(&bo1_look.sound, content), Some((0, song_num(&games, game, "final-battle"))));

            // FT2 (Bo3) progression: 0-0 -> Virus+Normal, 1-0 -> Boss+FinalDungeon, 1-1 -> FinalBoss.
            let mut bo3 = cfg.build_presentation(&games, &ready, 7, 2);
            let normal = (bo3.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
            let normal_name = content.background_of(normal.background.unwrap()).unwrap().1;
            assert!(!NON_REGULAR_BACKGROUNDS.contains(&normal_name), "{game} normal skipped {normal_name}");
            assert_eq!(content.music_of_background(normal_name), Some("virus-battle"), "{game} normal music");
            assert_eq!(resolved_song(&normal.sound, content), Some((0, song_num(&games, game, "virus-battle"))));

            let mp = (bo3.resolver)(RoundContext { round: 1, wins: 1, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
            let mp_name = content.background_of(mp.background.unwrap()).unwrap().1;
            assert!(mp_allowed.contains(&mp_name), "{game} single MP bg was {mp_name}");
            assert_eq!(resolved_song(&mp.sound, content), Some((0, song_num(&games, game, "boss-battle"))));

            let dmp = (bo3.resolver)(RoundContext { round: 2, wins: 1, losses: 1, max_rounds: 3, first_to: 2, default_background: default_bg });
            assert_eq!(content.background_of(dmp.background.unwrap()).unwrap().1, final_bg);
            assert_eq!(resolved_song(&dmp.sound, content), Some((0, song_num(&games, game, "final-battle"))));
        }
    }

    #[test]
    fn custom_mode_resolves_per_round_background_and_bgm() {
        let games = all_games();
        let ready = games.read("exe6").unwrap();
        let content = ready.content();
        let default_bg = content.stage_background().unwrap();

        let cfg = StageConfig {
            mode: StageMode::Custom,
            rounds: vec![
                RoundCustom {
                    bg: RoundBg::Named("exe6:graveyard".into()),
                    bgm: RoundBgm::ByBackground,
                },
                RoundCustom {
                    bg: RoundBg::Named("exe6:comp".into()),
                    bgm: RoundBgm::Theme { game: "exe4".into(), song: "tournament-battle".into() },
                },
                RoundCustom { bg: RoundBg::Random, bgm: RoundBgm::Random },
            ],
        };
        let mut pres = cfg.build_presentation(&games, &ready, 15, 2);

        let r0 = (pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert_eq!(content.background_of(r0.background.unwrap()), Some(("exe6", "graveyard")));
        assert_eq!(resolved_song(&r0.sound, content), Some((0, song_num(&games, "exe6", "crossover-battle"))));

        let r1 = (pres.resolver)(RoundContext { round: 1, wins: 1, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert_eq!(content.background_of(r1.background.unwrap()), Some(("exe6", "comp")));
        assert_eq!(resolved_song(&r1.sound, content), Some((1, song_num(&games, "exe4", "tournament-battle"))));

        let r2 = (pres.resolver)(RoundContext { round: 2, wins: 1, losses: 1, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert!(r2.background.is_some());
        assert!(r2.sound.is_some());
    }

    #[test]
    fn custom_stage_presentation_never_alters_battle_digest_and_switches_music() {
        let games = all_games();
        let six = games.read("exe6").unwrap();
        let content = six.content().clone();
        let m = saito_match::pick::live(&content, "exe6", 11, None).unwrap();
        let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
        let set = saito_match::Set::new(content.clone(), m.round(&content, 11), folders);

        let make_player = |with_override: bool| {
            let (banks, songs) = music::sound(&games, &six, &Music::Background).unwrap();
            let driver = LivePlayer::against(set.clone(), Default::default(), 11);
            let renderer = saito_frontend::testing::graphics(&content, 0x7FFF).renderer(Enhancements::NONE, None);
            let mut p = saito_frontend::Player::with(
                renderer,
                None,
                Some(saito_audio::BattleAudio::with_banks(banks, songs)),
                Box::new(driver),
            );
            if with_override {
                let cfg = StageConfig {
                    mode: StageMode::Custom,
                    rounds: vec![RoundCustom {
                        bg: RoundBg::Named("exe6:graveyard".into()),
                        bgm: RoundBgm::Theme { game: "exe4".into(), song: "tournament-battle".into() },
                    }],
                };
                let pres = cfg.build_presentation(&games, &six, 11, 2);
                p.renderer().set_background_override(pres.first_background);
                p.set_round_presentation(Some(pres.resolver));
            }
            for _ in 0..30 {
                assert!(p.tick(0));
            }
            p
        };

        let plain = make_player(false);
        let mut custom = make_player(true);
        assert_eq!(custom.battle().digest(), plain.battle().digest(), "digest identical with local stage/bgm override");
        assert_eq!(
            custom.renderer().background_override().and_then(|id| content.background_of(id)),
            Some(("exe6", "graveyard"))
        );
        let audio = custom.audio().unwrap();
        let playing: Vec<Option<SongId>> = (0..2)
            .map(|p| audio.driver_of(BankId(p)).unwrap().player(MUSIC_PLAYER).filter(|pl| pl.is_playing()).and_then(|pl| pl.song()))
            .collect();
        assert_eq!(playing, [None, Some(SongId(song_num(&games, "exe4", "tournament-battle")))]);
    }

    #[test]
    fn replay_presentation_roundtrips_through_replay_info() {
        let cases = [
            (StageConfig { mode: StageMode::Default, rounds: vec![] }, false),
            (StageConfig { mode: StageMode::Random, rounds: vec![] }, true),
            (StageConfig { mode: StageMode::Escalating, rounds: vec![] }, true),
            (
                StageConfig {
                    mode: StageMode::Custom,
                    rounds: vec![
                        RoundCustom {
                            bg: RoundBg::Named("exe6:graveyard".into()),
                            bgm: RoundBgm::Theme { game: "exe4".into(), song: "tournament-battle".into() },
                        },
                        RoundCustom { bg: RoundBg::Random, bgm: RoundBgm::Random },
                    ],
                },
                true,
            ),
        ];
        for (cfg, non_empty) in cases {
            let encoded = cfg.encode_presentation();
            assert_eq!(!encoded.is_empty(), non_empty, "{cfg:?}");
            if non_empty {
                let decoded = StageConfig::decode_presentation(&encoded).expect("decode");
                assert_eq!(decoded, cfg);
            } else {
                assert_eq!(StageConfig::decode_presentation(&encoded), None);
            }
        }
    }

    #[test]
    fn resolved_random_presentation_stays_stable_when_loaded_games_change() {
        let full_games = all_games();
        let exe5_only = Games::read_of(vec![("exe5", loaded_game(saito_match::testing::exe5_content()))]);

        let ready_full = full_games.read("exe5").unwrap();
        let ready_only = exe5_only.read("exe5").unwrap();
        let content = ready_only.content();
        let m = saito_match::pick::live(content, "exe5", 77, None).unwrap();
        let match_bytes = saito_match::binary::match_bytes(content, &m).unwrap();
        let default_bg = m.round(content, 77).settings.background;

        // Default mode produces None presentation and empty recorded metadata so live matches == replays.
        let default_cfg = StageConfig { mode: StageMode::Default, rounds: vec![] };
        let (default_pres, default_rec) = default_cfg.resolve_presentation(&exe5_only, &ready_only, 77, m.first_to());
        assert!(default_pres.is_none());
        assert!(default_rec.is_empty());

        // Record a Random match while only exe5 is loaded.
        let random_cfg = StageConfig { mode: StageMode::Random, rounds: vec![] };
        let (live_pres, recorded) = random_cfg.resolve_presentation(&exe5_only, &ready_only, 77, m.first_to());
        let mut live_pres = live_pres.expect("random presentation");
        assert!(recorded.starts_with("v1|custom|"), "recorded frozen rounds: {recorded}");
        let live_r0 = (live_pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: m.first_to(), default_background: default_bg });

        // Replay the recorded presentation later when exe4 + exe5 + exe6 are ALL loaded:
        // Round 0 background and BGM must match the live match identically!
        let decoded = StageConfig::decode_presentation(&recorded).expect("decode frozen random");
        let mut replay_pres = decoded.build_presentation(&full_games, &ready_full, 77, m.first_to());
        let replay_r0 = (replay_pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: m.first_to(), default_background: default_bg });
        assert_eq!(replay_r0.background, live_r0.background);
        assert_eq!(resolved_song(&replay_r0.sound, content), resolved_song(&live_r0.sound, content));

        // Still helper matches replay viewer's Round 0 background for both frozen and legacy v1|random strings.
        assert_eq!(replay_first_background(&ready_full, &recorded, &match_bytes), replay_pres.first_background);
        let legacy_pres = StageConfig { mode: StageMode::Random, rounds: vec![] }.build_presentation(&full_games, &ready_full, 77, m.first_to());
        assert_eq!(replay_first_background(&ready_full, "v1|random", &match_bytes), legacy_pres.first_background);
    }

    #[test]
    fn favorite_battle_bgm_mode_a_switches_per_round_and_roundtrips_replay() {
        let games = all_games();
        let ready = games.read("exe6").unwrap();
        let content = ready.content();
        let m = saito_match::pick::live(content, "exe6", 55, None).unwrap();
        let match_bytes = saito_match::binary::match_bytes(content, &m).unwrap();
        let default_bg = m.round(content, 55).settings.background;

        let favs = vec![
            "exe4/tournament-battle".to_string(),
            "exe5/final-battle".to_string(),
            "exe6/crossover-battle".to_string(),
        ];

        // 1. Default stage + Sequential favorites: keeps native background, switches track every round.
        music::reset_favorite_cursor();
        let default_cfg = StageConfig { mode: StageMode::Default, rounds: vec![] };
        let (pres, recorded) = default_cfg.resolve_presentation_with_favorites(
            &games,
            &ready,
            55,
            2,
            FavoriteBgmConfig { mode: FavoriteBgmMode::Sequential, favorites: &favs },
        );
        let mut pres = pres.expect("favorites override default presentation");
        assert!(recorded.starts_with("v1|custom|default|exe4/tournament-battle;default|exe5/final-battle;default|exe6/crossover-battle"), "recorded: {recorded}");
        let r0 = (pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        let r1 = (pres.resolver)(RoundContext { round: 1, wins: 1, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        let r2 = (pres.resolver)(RoundContext { round: 2, wins: 1, losses: 1, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert_eq!(r0.background, Some(default_bg));
        assert_eq!(r1.background, Some(default_bg));
        assert_eq!(resolved_song(&r0.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe4", "tournament-battle")));
        assert_eq!(resolved_song(&r1.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe5", "final-battle")));
        assert_eq!(resolved_song(&r2.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe6", "crossover-battle")));
        assert_eq!(replay_first_background(&ready, &recorded, &match_bytes), None, "RoundBg::Default preserves native background");

        // 2. Custom stage + Favorites (Mode A): explicit round theme is respected; unspecified rounds use favorites.
        music::reset_favorite_cursor();
        let custom_cfg = StageConfig {
            mode: StageMode::Custom,
            rounds: vec![
                RoundCustom {
                    bg: RoundBg::Named("exe6:graveyard".into()),
                    bgm: RoundBgm::Theme { game: "exe6".into(), song: "boss-battle".into() },
                },
                RoundCustom { bg: RoundBg::Random, bgm: RoundBgm::ByBackground },
            ],
        };
        let (c_pres, _) = custom_cfg.resolve_presentation_with_favorites(
            &games,
            &ready,
            55,
            2,
            FavoriteBgmConfig { mode: FavoriteBgmMode::Sequential, favorites: &favs },
        );
        let mut c_pres = c_pres.expect("custom presentation");
        let cr0 = (c_pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        let cr1 = (c_pres.resolver)(RoundContext { round: 1, wins: 1, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert_eq!(resolved_song(&cr0.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe6", "boss-battle")), "explicit round theme wins");
        assert_eq!(resolved_song(&cr1.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe4", "tournament-battle")), "unspecified round uses favorite");

        // 3. Escalating stage + Favorites: escalating backgrounds + per-round favorite BGM, encoded as v3|escalating-fav|.
        music::reset_favorite_cursor();
        let esc_cfg = StageConfig { mode: StageMode::Escalating, rounds: vec![] };
        let (e_pres, e_rec) = esc_cfg.resolve_presentation_with_favorites(
            &games,
            &ready,
            55,
            2,
            FavoriteBgmConfig { mode: FavoriteBgmMode::Sequential, favorites: &favs },
        );
        let mut e_pres = e_pres.expect("escalating presentation");
        assert!(e_rec.starts_with("v3|escalating-fav|exe4/tournament-battle;exe5/final-battle;exe6/crossover-battle"), "esc recorded: {e_rec}");
        let er0 = (e_pres.resolver)(RoundContext { round: 0, wins: 0, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        let er1 = (e_pres.resolver)(RoundContext { round: 1, wins: 1, losses: 0, max_rounds: 3, first_to: 2, default_background: default_bg });
        assert_eq!(resolved_song(&er0.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe4", "tournament-battle")));
        assert_eq!(resolved_song(&er1.sound, content).map(|(_, s)| s), Some(song_num(&games, "exe5", "final-battle")));
        assert_eq!(replay_first_background(&ready, &e_rec, &match_bytes), e_pres.first_background);
    }
}
