//! The battle's music (docs/app.md §1, "The battle's music"): its rounds' backgrounds', or a battle theme the
//! player picked, of any game whose pack is there. The user: "you should also allow for selectable battle themes
//! (bn4 virus battle, bn4 boss battle, bn4 tournament battle, bn4 vs duo, bn5 virus battle, bn5 boss battle, bn5 vs
//! nebula gray, bn6 virus battle, bn6 boss battle, bn6 vs cybeast, bn6 crossover battle)", picked "beside the arena,
//! each their own"; then "can you make backgrounds paired with music?", each background with the theme its game's
//! battles on it start, everywhere a pick doesn't say otherwise.
//!
//! What the themes are is each game's content: its rules' `battle_themes`, songs of its game's sound, and their names in
//! its locales (`battle_themes`, by the song's asset name); a background's is its definition's `music`
//! (`Content::music_of_background`), one of its game's themes. This names no song and no game. A theme is offered
//! once its game's parts are read (its content and its sound, `Games::read`: no runtime is needed), whichever
//! game is selected. The pick is one, the player's own, Practice's for one visit (`practice::Visit`: the user,
//! "going into practice should reset all the arena settings"): the practice menu shows and changes that one,
//! and its own sessions play it; a match, a replay and its video play the backgrounds'. It is presentation: the
//! battle, its cues, a replay and its rollback are the same whatever plays (`saito_audio::with_theme`,
//! `saito_audio::RoundMusic`).

mod titles;

use crate::games::{Games, Ready};
use crate::settings::FavoriteBgmMode;
use crate::shell::Shell;
use saito_audio::RoundMusic;
use saito_battle::content::BackgroundId;
use saito_frontend::player::{RoundContext, RoundLook};
use saito_frontend::preload::SoundParts;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Whether EXE5 default / background / escalating battle music should prefer
/// the arranged EXE5DS `SDAT` counterparts when an EXE5DS ROM is loaded.
static PREFER_DS_MUSIC: AtomicBool = AtomicBool::new(true);

pub fn prefer_ds_music() -> bool {
    PREFER_DS_MUSIC.load(Ordering::Relaxed)
}

pub fn set_prefer_ds_music(enabled: bool) {
    PREFER_DS_MUSIC.store(enabled, Ordering::Relaxed);
}

/// A battle's music.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Music {
    /// Each round's background's ([`round_music`]): what it plays without a pick.
    #[default]
    Background,
    /// A battle theme: game `game`'s theme of song `song` (its asset name).
    Theme { game: String, song: String },
}

/// What a list of the battle's music offers: the backgrounds' first, then the themes of each game whose parts
/// are read (`Games::read`), the games in the app's order and each game's in its own. A game whose pack isn't
/// there offers none.
pub fn offered(games: &Games) -> Vec<Music> {
    let themes = games.list.iter().filter_map(|g| games.read(&g.id).map(|ready| (g.id.clone(), ready)));
    std::iter::once(Music::Background)
        .chain(themes.flat_map(|(game, ready)| {
            ready.loaded.battle_themes().into_iter().map(move |song| Music::Theme { game: game.clone(), song: song.to_string() }).collect::<Vec<_>>()
        }))
        .collect()
}

/// Where `music` is in `offered`: a pick not offered (its game not read yet) shows as the backgrounds', the
/// first.
pub fn place(offered: &[Music], music: &Music) -> usize {
    offered.iter().position(|m| m == music).unwrap_or(0)
}

thread_local! {
    /// The themes' names in the language shown, by game and song: [`name_themes`]'.
    static NAMES: RefCell<HashMap<(String, String), String>> = RefCell::new(HashMap::new());
    /// Active UI language code last loaded by [`name_themes`].
    static NAMES_LANG: RefCell<String> = RefCell::new(String::from("en"));
}

/// Read the battle themes' names of `games` in `lang`, for [`name`]: from the content's locales alone, at
/// start-up and when the language changes (`Shell::read_game_names`).
pub fn name_themes<'a>(games: impl IntoIterator<Item = &'a str>, lang: &str) {
    let mut names = HashMap::new();
    for game in games {
        let Some(dir) = crate::contents::dir(game) else { continue };
        for (song, name) in saito_content::locale::text_table(&dir, game, lang, saito_content::locale::BATTLE_THEMES) {
            names.insert((game.to_string(), song), name);
        }
    }
    NAMES.with(|n| *n.borrow_mut() = names);
    NAMES_LANG.with(|l| *l.borrow_mut() = lang.to_string());
}

/// Localized display label for optional EXE5DS `SDAT` battle themes.
fn ds_theme_name(song: &str, lang: &str) -> Option<&'static str> {
    match lang {
        "zh-Hans" => match song {
            "ds-virus-battle" => Some("EXE5DS 病毒战"),
            "ds-boss-battle" => Some("EXE5DS 头目战"),
            "ds-final-battle" => Some("EXE5DS VS 星云灰暗"),
            "ds-you-are-not-alone" => Some("EXE5DS 你并不孤单"),
            "ds-liberate" => Some("EXE5DS 解放任务"),
            "ds-exe1-battle" => Some("EXE5DS EXE1 战斗重编"),
            "ds-exe2-battle" => Some("EXE5DS EXE2 战斗重编"),
            "ds-exe3-battle" => Some("EXE5DS EXE3 战斗重编"),
            "ds-exe4-battle" => Some("EXE5DS EXE4 战斗重编"),
            "ds-exe45-battle" => Some("EXE5DS EXE4.5 战斗重编"),
            _ => None,
        },
        "ja" => match song {
            "ds-virus-battle" => Some("EXE5DS ウイルスバトル"),
            "ds-boss-battle" => Some("EXE5DS ボスバトル"),
            "ds-final-battle" => Some("EXE5DS VS ネビュラグレイ"),
            "ds-you-are-not-alone" => Some("EXE5DS キミはひとりじゃない"),
            "ds-liberate" => Some("EXE5DS リベレートミッション"),
            "ds-exe1-battle" => Some("EXE5DS EXE1 バトルアレンジ"),
            "ds-exe2-battle" => Some("EXE5DS EXE2 バトルアレンジ"),
            "ds-exe3-battle" => Some("EXE5DS EXE3 バトルアレンジ"),
            "ds-exe4-battle" => Some("EXE5DS EXE4 バトルアレンジ"),
            "ds-exe45-battle" => Some("EXE5DS EXE4.5 バトルアレンジ"),
            _ => None,
        },
        "ko" => match song {
            "ds-virus-battle" => Some("EXE5DS 바이러스 배틀"),
            "ds-boss-battle" => Some("EXE5DS 보스 배틀"),
            "ds-final-battle" => Some("EXE5DS VS 네뷸라 그레이"),
            "ds-you-are-not-alone" => Some("EXE5DS 너는 혼자가 아니야"),
            "ds-liberate" => Some("EXE5DS 리버레이트 미션"),
            "ds-exe1-battle" => Some("EXE5DS EXE1 배틀 어레인지"),
            "ds-exe2-battle" => Some("EXE5DS EXE2 배틀 어레인지"),
            "ds-exe3-battle" => Some("EXE5DS EXE3 배틀 어레인지"),
            "ds-exe4-battle" => Some("EXE5DS EXE4 배틀 어레인지"),
            "ds-exe45-battle" => Some("EXE5DS EXE4.5 배틀 어레인지"),
            _ => None,
        },
        _ => match song {
            "ds-virus-battle" => Some("BN5DS Virus Battle"),
            "ds-boss-battle" => Some("BN5DS Boss Battle"),
            "ds-final-battle" => Some("BN5DS VS Nebula Gray"),
            "ds-you-are-not-alone" => Some("BN5DS You're Not Alone"),
            "ds-liberate" => Some("BN5DS Liberation Mission"),
            "ds-exe1-battle" => Some("BN5DS BN1 Battle Remix"),
            "ds-exe2-battle" => Some("BN5DS BN2 Battle Remix"),
            "ds-exe3-battle" => Some("BN5DS BN3 Battle Remix"),
            "ds-exe4-battle" => Some("BN5DS BN4 Battle Remix"),
            "ds-exe45-battle" => Some("BN5DS BN4.5 Battle Remix"),
            _ => None,
        },
    }
}

/// Canonical track key `"<game>/<song>"` used for persisted favorites and menu BGM (`S-135`).
pub fn track_key(game: &str, song: &str) -> String {
    format!("{game}/{song}")
}

/// Split a canonical track key `"<game>/<song>"` into `(game, song)`.
pub fn parse_track_key(key: &str) -> Option<(&str, &str)> {
    let (game, song) = key.split_once('/')?;
    (!game.is_empty() && !song.is_empty() && !game.contains('/') && !song.contains('/')).then_some((game, song))
}

/// Whether `(game, song)` is a known non-looping short jingle (preview-only; never eligible for
/// battle rotation or menu BGM).
pub fn is_known_non_looping(game: &str, song: &str) -> bool {
    if let Some(ds) = saito_extract::EXE5DS_BGM_TRACKS.iter().find(|t| t.key == song) {
        return !ds.looping;
    }
    if let Some(gba) = saito_extract::gba_bgm_track(game, song) {
        return !gba.looping;
    }
    false
}

/// Whether canonical key `"<game>/<song>"` is syntactically valid and not a known non-looping jingle.
pub fn is_eligible_loop_key(key: &str) -> bool {
    let Some((game, song)) = parse_track_key(key) else {
        return false;
    };
    !is_known_non_looping(game, song)
}

/// One track entry shown in the Settings → Audio music library (`S-135`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub key: String,
    pub game: String,
    pub song: String,
    pub is_ds: bool,
    pub looping: bool,
    pub is_battle_theme: bool,
}

/// Complete BGM catalog across every loaded game (in app order; EXE5 GBA tracks followed by
/// EXE5DS tracks when an EXE5DS ROM is attached).
pub fn catalog(games: &Games) -> Vec<CatalogEntry> {
    let mut out = Vec::new();
    for g in &games.list {
        let Some(ready) = games.read(&g.id) else { continue };
        for track in ready.loaded.catalog_tracks() {
            out.push(CatalogEntry {
                key: track_key(&g.id, &track.song),
                game: g.id.clone(),
                song: track.song,
                is_ds: track.is_ds,
                looping: track.looping,
                is_battle_theme: track.is_battle_theme,
            });
        }
    }
    out
}

/// Display badge for a track's game (`"EXE5DS"` / `"BN5DS"` for DS tracks, else `Shell::game_name`).
pub fn game_badge(game: &str, is_ds: bool) -> String {
    if is_ds {
        let lang = NAMES_LANG.with(|l| l.borrow().clone());
        return match lang.as_str() {
            "zh-Hans" | "ja" | "ko" => "EXE5DS".to_string(),
            _ => "BN5DS".to_string(),
        };
    }
    Shell::game_name(game)
}

/// Localized track title alone (without the game prefix), for the music library list.
pub fn track_title(game: &str, song: &str) -> String {
    let lang = NAMES_LANG.with(|l| l.borrow().clone());
    if let Some(title) = titles::track_title(game, song, &lang) {
        return title.to_string();
    }
    NAMES
        .with(|n| n.borrow().get(&(game.to_string(), song.to_string())).cloned())
        .unwrap_or_else(|| song.to_string())
}

/// Localized full track display name with game prefix (`"EXE6 秋原镇"`, `"EXE5DS 解放任务"`).
pub fn track_full_name(game: &str, song: &str) -> String {
    let is_ds = song.starts_with("ds-");
    format!("{} {}", game_badge(game, is_ds), track_title(game, song))
}

/// Resolve any catalog track (including non-looping short jingles) for preview playback.
pub fn resolve_preview(games: &Games, key: &str) -> Option<(saito_audio::Theme, bool)> {
    let (game, song) = parse_track_key(key)?;
    games.read(game)?.loaded.preview_track(song)
}

/// Resolve a looping catalog track for custom main-menu BGM playback.
pub fn resolve_menu_bgm(games: &Games, key: &str) -> Option<saito_audio::Theme> {
    let (game, song) = parse_track_key(key)?;
    games.read(game)?.loaded.theme(song)
}

/// A theme's name as a list shows it, in the language shown: its game's name, then the theme's ("BN4 Tournament
/// Battle", "EXE4 トーナメントバトル"); a theme its locales don't name is its song's asset name. The backgrounds'
/// is `own`, the UI's words for it.
pub fn name(music: &Music, own: &str) -> String {
    match music {
        Music::Background => own.to_string(),
        Music::Theme { game, song } => {
            if let Some(ds_label) = NAMES_LANG.with(|l| ds_theme_name(song, &l.borrow())) {
                return ds_label.to_string();
            }
            if let Some(theme) = NAMES.with(|n| n.borrow().get(&(game.clone(), song.clone())).cloned()) {
                return format!("{} {theme}", Shell::game_name(game));
            }
            let lang = NAMES_LANG.with(|l| l.borrow().clone());
            if let Some(title) = titles::track_title(game, song, &lang) {
                return format!("{} {title}", game_badge(game, song.starts_with("ds-")));
            }
            format!("{} {song}", Shell::game_name(game))
        }
    }
}

// ---- The practice menu's Music row ---------------------------------------------------------------------------

/// What the select's list offers, and the place of the pick in it.
fn list(shell: &Shell) -> (Vec<Music>, usize) {
    let offered = offered(&shell.games);
    let at = place(&offered, &shell.battle_music());
    (offered, at)
}

/// The UI's words for the backgrounds' music: the list's first choice.
fn own_words(shell: &Shell) -> String {
    shell.ui().global::<saito_ui::Shell>().get_background_music().to_string()
}

/// The pick as a select shows it (one not offered now shows as the backgrounds', which it plays as).
pub fn shown(shell: &Shell) -> String {
    let (offered, at) = list(shell);
    name(&offered[at], &own_words(shell))
}

/// Open the select's list under its control `id`, `index` (the screen hears the choice as `choose`).
pub fn open(shell: &mut Shell, id: &str, index: i32) {
    let (offered, at) = list(shell);
    let own = own_words(shell);
    let names = offered.iter().map(|m| name(m, &own)).collect();
    shell.pick(id, index, names, at);
}

/// Step the pick through the list by `by`, around from its end.
pub fn step(shell: &mut Shell, by: i32) {
    let (offered, at) = list(shell);
    let next = (at as i32 + by).rem_euclid(offered.len() as i32) as usize;
    shell.set_battle_music(&offered[next]);
}

/// The list's choice `choice` is the pick.
pub fn choose(shell: &mut Shell, choice: i32) {
    let (offered, _) = list(shell);
    if let Some(music) = offered.get(choice.max(0) as usize) {
        shell.set_battle_music(music);
    }
}

/// Play the pick in the battle the shell runs from now on, in place of the music it had (the user: "also you
/// should be able to change the bgm in the practice menu"): its music, if the battle's own is playing, starts
/// again at once as the new song (`Player::set_sound_parts`); the result's music, or a round's end, stays, and
/// the next round starts with it. Presentation alone, as the pick is.
pub fn retheme(shell: &mut Shell) {
    let Some(game) = shell.battle.as_ref().map(|b| b.game.clone()) else { return };
    let Some(ready) = shell.games.read(&game) else { return };
    let music = shell.battle_music();
    if music == Music::Background
        && let Some(pres) = practice_favorites_presentation(
            &shell.games,
            &ready,
            shell.settings.favorite_battle_bgm,
            &shell.settings.favorite_music,
            next_practice_fav_seed(),
        )
    {
        if let Some(b) = &mut shell.battle {
            b.stage.player.set_round_presentation(Some(pres.resolver));
        }
        return;
    }
    let Some(parts) = sound(&shell.games, &ready, &music) else { return };
    let rounds = round_music(&shell.games, &ready, &music);
    if let Some(b) = &mut shell.battle {
        b.stage.player.set_round_presentation(None);
        match rounds {
            Some(rounds) => b.stage.player.set_round_music(rounds),
            None => b.stage.player.set_sound_parts(parts),
        }
    }
}

/// Global round-advancing cursor for [`FavoriteBgmMode::Sequential`].
static FAVORITE_SEQ_CURSOR: AtomicUsize = AtomicUsize::new(0);

/// Counter used to vary random favorite picks across practice sessions.
static PRACTICE_FAV_NONCE: AtomicUsize = AtomicUsize::new(1);

pub fn next_practice_fav_seed() -> u32 {
    let n = PRACTICE_FAV_NONCE.fetch_add(1, Ordering::Relaxed) as u32;
    n.wrapping_mul(0x9E37_79B9) ^ 0x517C_C1B7
}

/// Reset the sequential favorite rotation cursor (used by deterministic unit tests).
#[cfg(test)]
pub fn reset_favorite_cursor() {
    FAVORITE_SEQ_CURSOR.store(0, Ordering::Relaxed);
}

/// Resolve all currently available, looping favorite tracks as `(game, song, SoundParts)` in
/// the player's saved favorite order. Unavailable games or non-looping entries are silently skipped.
pub fn resolve_favorites(games: &Games, ready: &Ready, favorites: &[String]) -> Vec<(String, String, SoundParts)> {
    let Some(sound) = ready.loaded.sound.as_ref() else {
        return Vec::new();
    };
    let content = ready.content();
    let mut out = Vec::new();
    for key in favorites {
        let Some((game, song)) = parse_track_key(key) else { continue };
        let theme = games
            .read(game)
            .and_then(|of| of.loaded.theme(song))
            .or_else(|| (game == content.game()).then(|| ready.loaded.theme(song)).flatten());
        if let Some(theme) = theme {
            out.push((game.to_string(), song.to_string(), sound.parts(content, Some(&theme))));
        }
    }
    out
}

/// Choose the index into a non-empty favorite pool of length `pool_len` for one round.
///
/// - `Sequential`: advances a global cursor by 1 every round, wrapping around.
/// - `Random`: picks pseudo-randomly from `(seed, round)` and avoids repeating `last_pick`
///   whenever `pool_len > 1`.
pub fn pick_favorite_index(mode: FavoriteBgmMode, pool_len: usize, seed: u32, round: u32, last_pick: Option<usize>) -> usize {
    debug_assert!(pool_len > 0);
    if pool_len <= 1 {
        return 0;
    }
    match mode {
        FavoriteBgmMode::Sequential => FAVORITE_SEQ_CURSOR.fetch_add(1, Ordering::Relaxed) % pool_len,
        FavoriteBgmMode::Random | FavoriteBgmMode::Off => {
            let mut picks = saito_match::pick::Picks::new(seed ^ 0x4641_5652 ^ round.wrapping_mul(0x9E37_79B1));
            let mut idx = picks.below(pool_len);
            if Some(idx) == last_pick {
                let offset = 1 + picks.below(pool_len - 1);
                idx = (idx + offset) % pool_len;
            }
            idx
        }
    }
}

/// Build a per-round presentation resolver for Practice mode when Favorite Battle BGM is active:
/// keeps the battle's chosen background intact (`background: None`) while switching to a favorite
/// track every round.
pub fn practice_favorites_presentation(
    games: &Games,
    ready: &Ready,
    mode: FavoriteBgmMode,
    favorites: &[String],
    seed: u32,
) -> Option<crate::battle::BattlePresentation> {
    if mode == FavoriteBgmMode::Off {
        return None;
    }
    let pool = resolve_favorites(games, ready, favorites);
    if pool.is_empty() {
        return None;
    }
    let mut last_pick: Option<usize> = None;
    let resolver = Box::new(move |ctx: RoundContext| {
        let idx = pick_favorite_index(mode, pool.len(), seed, ctx.round as u32, last_pick);
        last_pick = Some(idx);
        RoundLook { background: None, sound: Some(pool[idx].2.clone()) }
    });
    Some(crate::battle::BattlePresentation {
        extra_backgrounds: Vec::new(),
        first_background: None,
        resolver,
    })
}

/// The sound of a battle of game `ready` with `music` (as the battle's preload takes it): a theme's song, with
/// its own game's bank, in place of the game's own music where the theme's game is read and has it; else the
/// game's own (which the backgrounds' music, [`round_music`], plays in place of round by round). None: the game
/// has no sound loaded.
pub fn sound(games: &Games, ready: &Ready, music: &Music) -> Option<SoundParts> {
    let sound = ready.loaded.sound.as_ref()?;
    let theme = match music {
        Music::Background => prefer_ds_music()
            .then(|| ready.loaded.ds_counterpart("virus-battle"))
            .flatten()
            .and_then(|ds| ready.loaded.theme(ds)),
        Music::Theme { game, song } => games.read(game).and_then(|of| of.loaded.theme(song)),
    };
    Some(sound.parts(ready.content(), theme.as_ref()))
}

/// The music by the round's background of a battle of game `ready` with `music` (the backgrounds'; none for a
/// theme, which plays every round): each background a round of the game may be drawn on (the game's own and the
/// pool's) with its music in place of the game's own, its definition's (`Content::music_of_background`, its
/// game's content, the battle's for one of its own), with its game's bank. A background whose game isn't read,
/// or whose definition names no music, plays the game's own. None: the game has no sound loaded.
pub fn round_music(games: &Games, ready: &Ready, music: &Music) -> Option<Arc<RoundMusic>> {
    if *music != Music::Background {
        return None;
    }
    let sound = ready.loaded.sound.as_ref()?;
    let content = ready.content();
    let use_ds = prefer_ds_music();
    let own_theme = use_ds
        .then(|| ready.loaded.ds_counterpart("virus-battle"))
        .flatten()
        .and_then(|ds| ready.loaded.theme(ds));
    let own = sound.parts(content, own_theme.as_ref());
    // (Each theme's parts once, by its game and song.)
    let mut made: HashMap<(String, String), SoundParts> = HashMap::new();
    let mut by_background = HashMap::new();
    for id in (0..=u16::MAX).map(BackgroundId).map_while(|id| content.background_of(id).map(|_| id)) {
        let Some((game, name)) = content.background_of(id) else { continue };
        // (One of the battle's own game is read from its content, which may be a replay's revision.)
        let other = (game != content.game()).then(|| games.read(game)).flatten();
        let of = match &other {
            Some(other) => &other.loaded,
            None if game == content.game() => &ready.loaded,
            None => continue,
        };
        let Some(raw_song) = of.game.content.music_of_background(name) else { continue };
        let song = if use_ds { of.ds_counterpart(raw_song).unwrap_or(raw_song) } else { raw_song };
        let key = (game.to_string(), song.to_string());
        if !made.contains_key(&key) {
            let Some(theme) = of.theme(song) else { continue };
            made.insert(key.clone(), sound.parts(content, Some(&theme)));
        }
        by_background.insert(id, made[&key].clone());
    }
    Some(Arc::new(RoundMusic { own, by_background }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pick no longer offered shows as the backgrounds'.
    #[test]
    fn a_pick_not_offered_is_the_games_own() {
        let offered = [Music::Background, Music::Theme { game: "exe6".into(), song: "boss-battle".into() }];
        assert_eq!(place(&offered, &offered[1]), 1);
        assert_eq!(place(&offered, &Music::Theme { game: "exe4".into(), song: "virus-battle".into() }), 0);
    }

    use saito_battle::Content;
    use saito_frontend::game::{Game, Loaded, Sound, Enhancements};
    use std::sync::Arc;

    /// Game `content` as it is read (content/'s, its assets made up: each numbered by its handle): its sound a
    /// bank with a song at each of its sounds' numbers (the library's testing bank's songs), its battle's music and
    /// its themes held notes on the music player, every other a short note on another.
    fn loaded(content: Arc<Content>) -> Loaded {
        let mut bank = (*saito_frontend::testing::bank(&content)).clone();
        // (Sound 0's song is a held note on the music player, sound 1's a short one on player 1.)
        let (held, short) = (bank.songs[0].clone(), bank.songs[1].clone());
        let assets = saito_frontend::testing::numbered_by_handle(&content);
        let songs = saito_audio::Songs::of(&assets);
        // (The sounds by name: those the content names at their handles' numbers, the themes it doesn't name past
        // them, as the game's assets number them all.)
        let mut index = saito_content_api::AssetIndex::default();
        for (h, name) in content.assets.sounds.iter().enumerate() {
            index.sounds.insert(name.clone(), h as u16);
        }
        for theme in content.battle_themes() {
            let n = index.sounds.len() as u16;
            index.sounds.entry(theme.clone()).or_insert(n);
        }
        let music: Vec<u16> =
            content.battle_music().into_iter().map(|m| m.0).chain(content.battle_themes().iter().filter_map(|t| index.sounds.get(t).copied())).collect();
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

    fn games(ids: &[&str]) -> Games {
        let content = |id: &str| match id {
            "exe4" => saito_match::testing::exe4_content(),
            "exe5" => saito_match::testing::exe5_content(),
            _ => saito_match::testing::exe6_content(),
        };
        Games::read_of(ids.iter().map(|&id| (id, loaded(content(id)))).collect())
    }

    /// The list is the backgrounds', then each read game's themes as its content states them, by the user's names:
    /// the game's name as shown in the language, then the theme's. A game not read offers none.
    #[test]
    fn the_themes_are_the_games_in_their_order() {
        let all = games(&["exe4", "exe5", "exe6"]);
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "ja");
        let names: Vec<String> = offered(&all).iter().map(|m| name(m, "By background")).collect();
        assert_eq!(
            names,
            [
                "By background",
                "EXE4 ウイルスバトル",
                "EXE4 ボスバトル",
                "EXE4 トーナメントバトル",
                "EXE4 VS デューオ",
                "EXE5 ウイルスバトル",
                "EXE5 ボスバトル",
                "EXE5 VS ネビュラグレイ",
                "EXE6 ウイルスバトル",
                "EXE6 ボスバトル",
                "EXE6 VS 電脳獣",
                "EXE6 クロスオーバーバトル"
            ]
        );
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "zh-Hans");
        let zh: Vec<String> = offered(&all).iter().skip(1).map(|m| name(m, "")).collect();
        assert!(zh[0].starts_with("EXE4 "));
        assert!(zh[10].starts_with("EXE6 "));
        let some = games(&["exe6"]);
        assert_eq!(offered(&some).len(), 1 + 4, "EXE4's and EXE5's aren't read");
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "ja");
    }

    /// What a theme plays in place of: the songs each game's round starts, a link battle's (EXE4's a single
    /// battle's and a set's) and its stages' own.
    #[test]
    fn the_battles_own_music() {
        let names = |content: Arc<Content>| {
            let sounds = content.assets.names(saito_content_api::AssetKind::Sound).into_iter().map(String::from).collect::<Vec<_>>();
            content.battle_music().iter().map(|m| sounds[m.0 as usize].clone()).collect::<Vec<_>>()
        };
        assert_eq!(names(saito_match::testing::exe4_content()), ["boss-battle", "virus-battle"]);
        assert_eq!(names(saito_match::testing::exe5_content()), ["virus-battle"]);
        assert_eq!(names(saito_match::testing::exe6_content()), ["virus-battle"]);
    }

    /// A battle plays the picked theme in place of its game's own music: a BN4 theme in a BN6 battle on a
    /// driver of BN4's bank (its song, as BN4's pack numbers it), BN6's own theme on BN6's; a pick whose game
    /// isn't read, the game's own. The battle is the same tick for tick whatever plays.
    #[test]
    fn a_battle_plays_the_picked_theme() {
        use saito_audio::{MUSIC_PLAYER, m4a::SongId};
        use saito_audio::BankId;
        let all = games(&["exe4", "exe5", "exe6"]);
        let six = all.read("exe6").unwrap();
        let content = six.content().clone();
        let m = saito_match::pick::live(&content, "exe6", 11, None).unwrap();
        // The music each driver plays once the round has started, and the battle's digest.
        let play = |music: &Music| {
            let (banks, songs) = sound(&all, &six, music).unwrap();
            let packs = banks.len();
            let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
            let set = saito_match::Set::new(content.clone(), m.round(&content, 11), folders);
            let driver = saito_frontend::driver::LivePlayer::against(set, Default::default(), 11);
            let renderer = saito_frontend::testing::graphics(&content, 0x7FFF).renderer(Enhancements::NONE, None);
            let mut player = saito_frontend::Player::with(renderer, None, Some(saito_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver));
            for _ in 0..30 {
                player.tick(0);
            }
            let audio = player.audio().unwrap();
            let playing: Vec<Option<SongId>> = (0..packs)
                .map(|p| audio.driver_of(BankId(p as u8)).unwrap().player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song()))
                .collect();
            (playing, player.battle().digest())
        };
        // (A song by its game's number, in the made-up assets.)
        let number = |game: &str, song: &str| SongId(all.read(game).unwrap().loaded.game.index.number(saito_content_api::AssetKind::Sound, song).unwrap());
        let own = SongId(content.defs.roles().link_battle_music(true).0);
        let (playing, digest) = play(&Music::Background);
        assert_eq!(playing, [Some(own)], "BN6's own");
        let theme = |game: &str, song: &str| Music::Theme { game: game.into(), song: song.into() };
        let (playing, with_theme) = play(&theme("exe4", "tournament-battle"));
        assert_eq!(playing, [None, Some(number("exe4", "tournament-battle"))], "BN4's, on its own bank's driver");
        assert_eq!(with_theme, digest, "the battle is the same");
        let (playing, _) = play(&theme("exe6", "final-battle"));
        assert_eq!(playing, [Some(number("exe6", "final-battle"))], "BN6's own theme, on BN6's driver");
        let (playing, _) = play(&theme("exe6", "winner-1"));
        assert_eq!(playing, [Some(own)], "no theme of that song: the game's own");
        let some = games(&["exe6"]);
        let six = some.read("exe6").unwrap();
        let (banks, songs) = sound(&some, &six, &theme("exe4", "tournament-battle")).unwrap();
        assert_eq!((banks.len(), songs), (1, six.loaded.sound.as_ref().unwrap().songs.clone()), "BN4 not read: the game's own");
    }

    /// The practice menu's Music changes the battle's music as it runs (`retheme`'s `Player::set_sound_parts`):
    /// the new song at once, on its own game's driver; the game's own again on the battle's; the battle the same.
    #[test]
    fn a_battle_changes_its_music_as_it_runs() {
        use saito_audio::{MUSIC_PLAYER, m4a::SongId};
        use saito_audio::BankId;
        let all = games(&["exe4", "exe5", "exe6"]);
        let six = all.read("exe6").unwrap();
        let content = six.content().clone();
        let m = saito_match::pick::live(&content, "exe6", 11, None).unwrap();
        let player = |music: &Music| {
            let (banks, songs) = sound(&all, &six, music).unwrap();
            let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
            let set = saito_match::Set::new(content.clone(), m.round(&content, 11), folders);
            let driver = saito_frontend::driver::LivePlayer::against(set, Default::default(), 11);
            let renderer = saito_frontend::testing::graphics(&content, 0x7FFF).renderer(Enhancements::NONE, None);
            saito_frontend::Player::with(renderer, None, Some(saito_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver))
        };
        let playing = |p: &saito_frontend::Player| -> Vec<Option<SongId>> {
            let audio = p.audio().unwrap();
            (0..3).map_while(|n| audio.driver_of(BankId(n))).map(|d| d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song())).collect()
        };
        let number = |game: &str, song: &str| SongId(all.read(game).unwrap().loaded.game.index.number(saito_content_api::AssetKind::Sound, song).unwrap());
        let own = number("exe6", "virus-battle");
        let theme = Music::Theme { game: "exe5".into(), song: "final-battle".into() };
        let (mut changed, mut straight) = (player(&Music::Background), player(&Music::Background));
        for _ in 0..30 {
            changed.tick(0);
            straight.tick(0);
        }
        assert_eq!(playing(&changed), [Some(own)]);
        changed.set_sound_parts(sound(&all, &six, &theme).unwrap());
        for _ in 0..2 {
            changed.tick(0);
            straight.tick(0);
        }
        assert_eq!(playing(&changed), [None, Some(number("exe5", "final-battle"))], "BN5's, at once");
        changed.set_sound_parts(sound(&all, &six, &Music::Background).unwrap());
        for _ in 0..2 {
            changed.tick(0);
            straight.tick(0);
        }
        assert_eq!(playing(&changed), [Some(own), None], "the game's own again, from its start");
        assert_eq!(changed.battle().digest(), straight.battle().digest(), "the battle the same");
    }

    /// Without a pick, each round plays its background's music (`round_music`): the theme its definition names,
    /// on its own game's driver (BN6's Graveyard its Crossover Battle on BN6's, BN4's Duo its vs Duo on BN4's). The
    /// battle is the same whatever plays.
    #[test]
    fn a_round_plays_its_backgrounds_music() {
        use saito_audio::{MUSIC_PLAYER, m4a::SongId};
        use saito_audio::BankId;
        let all = games(&["exe4", "exe5", "exe6"]);
        let six = all.read("exe6").unwrap();
        let content = six.content().clone();
        let number = |game: &str, song: &str| SongId(all.read(game).unwrap().loaded.game.index.number(saito_content_api::AssetKind::Sound, song).unwrap());
        // The music each driver plays once the round on `background` has started, and the battle's digest.
        let play = |background: &str, music: &Music| {
            let mut m = saito_match::pick::live(&content, "exe6", 11, None).unwrap();
            m.rounds.iter_mut().for_each(|r| r.background = Some(background.into()));
            let (banks, songs) = sound(&all, &six, music).unwrap();
            let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
            let set = saito_match::Set::new(content.clone(), m.round(&content, 11), folders);
            let driver = saito_frontend::driver::LivePlayer::against(set, Default::default(), 11);
            let renderer = saito_frontend::testing::graphics(&content, 0x7FFF).renderer(Enhancements::NONE, None);
            let mut player = saito_frontend::Player::with(renderer, None, Some(saito_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver));
            if let Some(rounds) = round_music(&all, &six, music) {
                player.set_round_music(rounds);
            }
            for _ in 0..30 {
                player.tick(0);
            }
            let audio = player.audio().unwrap();
            let playing: Vec<Option<SongId>> = (0..3)
                .map_while(|p| audio.driver_of(BankId(p)))
                .map(|d| d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song()))
                .collect();
            (playing, player.battle().digest())
        };
        let (playing, digest) = play("graveyard", &Music::Background);
        assert_eq!(playing, [Some(number("exe6", "crossover-battle"))], "BN6's Graveyard: its Crossover Battle");
        let (playing, _) = play("exe4:duo", &Music::Background);
        assert_eq!(playing, [None, Some(number("exe4", "final-battle"))], "BN4's Duo: its vs Duo, on BN4's driver");
        let (playing, _) = play("comp", &Music::Background);
        assert_eq!(playing, [Some(number("exe6", "virus-battle"))], "BN6's Comp: its Virus Battle");
        let (playing, picked) = play("graveyard", &Music::Theme { game: "exe5".into(), song: "boss-battle".into() });
        assert_eq!(playing, [None, Some(number("exe5", "boss-battle"))], "a theme picked plays whatever the background");
        assert_eq!(picked, digest, "the battle is the same");
        assert_eq!(round_music(&all, &six, &Music::Theme { game: "exe5".into(), song: "boss-battle".into() }).map(|_| ()), None);
    }

    /// Every background of every game names its music, one of its game's battle themes.
    #[test]
    fn every_background_has_its_music() {
        for content in [saito_match::testing::exe4_content(), saito_match::testing::exe5_content(), saito_match::testing::exe6_content()] {
            let game = content.game().to_string();
            for (of, name) in content.background_pool.iter().filter(|(g, _)| *g == game) {
                let music = content.music_of_background(name).unwrap_or_else(|| panic!("{of}:{name} names no music"));
                assert!(content.battle_themes().iter().any(|t| t == music), "{of}:{name}: {music} is a battle theme");
            }
        }
    }

    /// Practice's Background changed while the battle is held under its menu (`Player::set_backgrounds`): the
    /// music by background switches at once, the new background's song from its start, as the next round's
    /// start would; the battle doesn't step.
    #[test]
    fn a_background_changed_mid_round_changes_the_music_at_once() {
        use saito_audio::{MUSIC_PLAYER, m4a::SongId};
        use saito_audio::BankId;
        let all = games(&["exe4", "exe5", "exe6"]);
        let six = all.read("exe6").unwrap();
        let content = six.content().clone();
        let number = |song: &str| SongId(six.loaded.game.index.number(saito_content_api::AssetKind::Sound, song).unwrap());
        let mut m = saito_match::pick::live(&content, "exe6", 11, None).unwrap();
        let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
        let mut set_on = |background: &str| {
            m.rounds.iter_mut().for_each(|r| r.background = Some(background.into()));
            saito_match::Set::new(content.clone(), m.round(&content, 11), folders.clone())
        };
        let (comp, graveyard) = (set_on("comp"), set_on("graveyard"));
        let (banks, songs) = sound(&all, &six, &Music::Background).unwrap();
        let renderer = saito_frontend::testing::graphics(&content, 0x7FFF).renderer(Enhancements::NONE, None);
        let driver = saito_frontend::driver::LivePlayer::against(comp, Default::default(), 11);
        let mut player = saito_frontend::Player::with(renderer, None, Some(saito_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver));
        player.set_round_music(round_music(&all, &six, &Music::Background).unwrap());
        for _ in 0..30 {
            player.tick(0);
        }
        let playing = |p: &saito_frontend::Player| p.audio().unwrap().driver_of(BankId(0)).unwrap().player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song());
        assert_eq!(playing(&player), Some(number("virus-battle")), "BN6's Comp: its Virus Battle");
        let ticks = player.ticks();
        assert!(player.set_held(true));
        assert!(player.set_backgrounds(graveyard));
        for _ in 0..2 {
            player.tick(0);
        }
        assert_eq!(playing(&player), Some(number("crossover-battle")), "the Graveyard's Crossover Battle, held");
        assert_eq!(player.ticks(), ticks, "held: the battle didn't step");
    }

    /// When an EXE5DS SDAT bank is attached to EXE5, all 10 DS battle themes
    /// appear in `offered()`, localize cleanly, play via SDAT alongside GBA SFX,
    /// and `set_prefer_ds_music` controls whether `By background` defaults to DS
    /// or GBA tracks.
    #[test]
    fn exe5ds_themes_offer_localize_and_play_via_sdat() {
        use saito_audio::{BankId, sdat};

        let samples: Vec<i16> = (0..64)
            .map(|i| (((i as f64 / 64.0) * std::f64::consts::TAU).sin() * 20000.0) as i16)
            .collect();
        let wave = sdat::Wave { samples, sample_rate: 16384, looped: true, loop_start: 0, loop_end: 64 };
        let inst = sdat::Instrument::Single(sdat::NoteDef {
            kind: sdat::NoteKind::Pcm { swav_id: 0, swar_slot: 0 },
            base_note: 60,
            attack: 127,
            decay: 127,
            sustain: 127,
            release: 80,
            pan: 64,
        });
        let sseq = vec![0xE1, 144, 0, 0x81, 0, 0xD4, 0, 60, 100, 24, 0xFC];
        let mut ds_songs = vec![None; 42];
        for &(_, seq_id, _) in saito_extract::EXE5DS_BATTLE_THEMES {
            ds_songs[seq_id as usize] = Some(sdat::Song {
                data: sseq.clone(),
                bank: 0,
                volume: 100,
                channel_priority: 64,
                player_priority: 64,
                player: 31,
            });
        }
        let ds_bank = Arc::new(sdat::SoundBank {
            songs: ds_songs,
            banks: vec![Some(sdat::InstrumentBank {
                wave_arcs: [Some(0), None, None, None],
                instruments: vec![inst],
            })],
            wave_arcs: vec![Some(sdat::WaveArchive { waves: vec![wave] })],
        });

        let mut exe5 = loaded(saito_match::testing::exe5_content());
        exe5.game.ds_sound = Some(ds_bank.clone());
        exe5.sound.as_mut().unwrap().ds_bank = Some(ds_bank);

        let all = Games::read_of(vec![("exe5", exe5)]);
        let five = all.read("exe5").unwrap();

        let list = offered(&all);
        assert_eq!(list.len(), 1 + 3 + saito_extract::EXE5DS_BATTLE_THEMES.len());

        crate::shell::Shell::read_game_names(&["exe5"], "zh-Hans");
        assert_eq!(name(&Music::Theme { game: "exe5".into(), song: "ds-virus-battle".into() }, ""), "EXE5DS 病毒战");
        assert_eq!(name(&Music::Theme { game: "exe5".into(), song: "ds-you-are-not-alone".into() }, ""), "EXE5DS 你并不孤单");
        assert_eq!(name(&Music::Theme { game: "exe5".into(), song: "ds-exe45-battle".into() }, ""), "EXE5DS EXE4.5 战斗重编");
        crate::shell::Shell::read_game_names(&["exe5"], "ja");
        assert_eq!(name(&Music::Theme { game: "exe5".into(), song: "ds-virus-battle".into() }, ""), "EXE5DS ウイルスバトル");
        assert_eq!(name(&Music::Theme { game: "exe5".into(), song: "ds-you-are-not-alone".into() }, ""), "EXE5DS キミはひとりじゃない");

        let content = five.content().clone();
        let m = saito_match::pick::live(&content, "exe5", 11, None).unwrap();
        let play_once = |music: &Music| {
            let (banks, songs) = sound(&all, &five, music).unwrap();
            let folders = [0, 1].map(|side| m.sides[side].folder(&content).saved().unwrap());
            let set = saito_match::Set::new(content.clone(), m.round(&content, 11), folders);
            let driver = saito_frontend::driver::LivePlayer::against(set, Default::default(), 11);
            let renderer = saito_frontend::testing::graphics(&content, 0x7FFF).renderer(Enhancements::NONE, None);
            let mut player = saito_frontend::Player::with(renderer, None, Some(saito_audio::BattleAudio::with_banks(banks, songs)), Box::new(driver));
            for _ in 0..30 {
                player.tick(0);
            }
            let audio = player.audio().unwrap();
            (audio.playing_music_of(BankId(0)), audio.playing_music_of(BankId(1)))
        };

        assert_eq!(play_once(&Music::Theme { game: "exe5".into(), song: "ds-you-are-not-alone".into() }), (None, Some(14)));

        let prev = prefer_ds_music();
        set_prefer_ds_music(true);
        assert_eq!(play_once(&Music::Background), (None, Some(21)), "DS Virus Battle (SEQ 21) when prefer_ds_music=true");
        set_prefer_ds_music(false);
        let own_gba = five.loaded.game.index.number(saito_content_api::AssetKind::Sound, "virus-battle").unwrap();
        assert_eq!(play_once(&Music::Background), (Some(own_gba), None), "GBA Virus Battle when prefer_ds_music=false");
        set_prefer_ds_music(prev);
    }

    /// Full BGM catalog, loop filtering (short jingles preview-only), localization, and per-round
    /// favorite rotation (`S-135`).
    #[test]
    fn full_catalog_loop_filtering_and_favorite_rotation() {
        let all = games(&["exe4", "exe5", "exe6"]);
        // Verify non-looping jingle classification across all games.
        assert!(is_known_non_looping("exe4", "transmission"));
        assert!(is_known_non_looping("exe4", "game-over"));
        assert!(is_known_non_looping("exe4", "winner"));
        assert!(is_known_non_looping("exe5", "liberate-boss-fanfare"));
        assert!(is_known_non_looping("exe5", "1turn-delete"));
        assert!(is_known_non_looping("exe5", "ds-1turn-delete"));
        assert!(is_known_non_looping("exe6", "winner-1"));
        assert!(!is_known_non_looping("exe6", "acdc-town"));
        assert!(!is_known_non_looping("exe5", "ds-liberate"));

        // Non-looping jingles are allowed in preview_track but rejected by theme() / resolve_menu_bgm().
        assert!(resolve_preview(&all, "exe6/winner-1").is_some());
        assert!(resolve_menu_bgm(&all, "exe6/winner-1").is_none());

        // Localized Legacy Collection track titles exist for every catalog track in all four languages,
        // preserving untranslated English titles (e.g. Theme of ROCKMAN EXE4, Invisible Wing, Battle Field).
        for lang in ["en", "ja", "zh-Hans", "ko"] {
            crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], lang);
            for game in ["exe4", "exe5", "exe6"] {
                for track in saito_extract::gba_bgm_tracks(game) {
                    let title = track_title(game, track.key);
                    assert!(!title.is_empty(), "{lang} {game}/{} empty", track.key);
                    assert_ne!(title, track.key, "{lang} {game}/{} fell back to raw key", track.key);
                }
            }
            for track in saito_extract::EXE5DS_BGM_TRACKS {
                let title = track_title("exe5", track.key);
                assert!(!title.is_empty(), "{lang} exe5/{} empty", track.key);
                assert_ne!(title, track.key, "{lang} exe5/{} fell back to raw key", track.key);
            }
        }
        crate::shell::Shell::read_game_names(&["exe4", "exe5", "exe6"], "zh-Hans");
        assert_eq!(track_title("exe4", "title-screen"), "Theme of ROCKMAN EXE 4");
        assert_eq!(track_title("exe4", "acdc-town"), "Hometown");
        assert_eq!(track_title("exe4", "elec-tower-comp"), "Invisible Wing");
        assert_eq!(track_title("exe4", "undernet"), "Heel's Parade");
        assert_eq!(track_title("exe4", "final-battle"), "对决迪欧");
        assert_eq!(track_title("exe5", "title-screen"), "Theme of ROCKMAN EXE 5");
        assert_eq!(track_title("exe5", "main-comp"), "Seek a Cord");
        assert_eq!(track_title("exe5", "castle-comp"), "Castle of Ninja!");
        assert_eq!(track_title("exe5", "virus-battle"), "来对战吧!");
        assert_eq!(track_title("exe5", "final-battle"), "对决星云灰暗");
        assert_eq!(track_title("exe6", "title-screen"), "Theme of ROCKMAN EXE 6");
        assert_eq!(track_title("exe6", "virus-battle"), "Battle Field");
        assert_eq!(track_title("exe6", "pavilion"), "秘密总部");
        assert_eq!(track_title("exe6", "pavilion-comp"), "Two of Braves");
        assert_eq!(track_title("exe6", "final-battle"), "决战、电脑兽!");
        assert_eq!(track_title("exe6", "school"), "才叶学园");

        // Sequential rotation advances round-by-round wrapping cleanly.
        reset_favorite_cursor();
        let seq: Vec<usize> = (0..5)
            .map(|r| pick_favorite_index(FavoriteBgmMode::Sequential, 3, 42, r, None))
            .collect();
        assert_eq!(seq, vec![0, 1, 2, 0, 1]);

        // Random rotation never repeats consecutive rounds when pool has 2+ tracks.
        let mut last = None;
        for r in 0..20 {
            let next = pick_favorite_index(FavoriteBgmMode::Random, 3, 12345, r, last);
            assert!(next < 3);
            assert_ne!(Some(next), last, "round {r} must not repeat previous favorite");
            last = Some(next);
        }
    }
}
