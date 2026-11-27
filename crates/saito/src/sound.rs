//! The sound: the audio device, the battle's samples to it, the menus' sounds and the title music.
//!
//! The menus' sounds are the selected game's own: its custom screen's cursor, pick and taking back, its
//! refusal, and the sound its battles start with as the screen breaks into one (the jack-in's), as its rules'
//! sound roles name them (the app names no sound of a game's graphics). The title music is the
//! game's too, the song its rules' music role `title_screen` names, played across the menu screens, faded out as a
//! battle starts, and crossfaded with the next game's as the game changes (docs/design/app-ui.md §6).
//!
//! The device gets one stream at the battle's rate. While a battle runs, the menus' sounds are ticked with
//! its ticks and added to its samples; with none running they run on their own clock at the same rate
//! (docs/app.md §1, "The sound").

use saito_audio::{BankId, BankSong, BattleAudio, Output, Songs, SoundCue, SoundId, Theme};
use saito_battle::content::{MusicRole, SoundRole};
use saito_frontend::game::Loaded;
use saito_frontend::player::FRAME_RATE;
use std::time::Duration;
use web_time::Instant;

/// How long the title music takes to fade out, and to fade in.
const FADE: f32 = 0.9;
const FADE_IN: f32 = 0.45;

/// The device's level at full volume (the GBA's full scale is loud).
const FULL: f32 = 0.8;

/// A menu sound, by what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// The cursor moved.
    Cursor,
    /// Something was chosen.
    Pick,
    /// Back.
    Back,
    /// It can't be done.
    Refused,
    /// A battle is entered (the jack-in): the game's own encounter sound, where its content names one.
    JackIn,
}

/// The selected game's sound for the menus: its effects (the cues of its roles) and its title music, each on a
/// driver of its own so their levels are set apart.
struct Menu {
    effects: BattleAudio,
    cursor: Option<SoundCue>,
    pick: Option<SoundCue>,
    back: Option<SoundCue>,
    refused: Option<SoundCue>,
    jack_in: Option<SoundCue>,
    music: BattleAudio,
    title: Option<SoundCue>,
    /// The title music is playing (or fading out).
    playing: bool,
}

/// Active single-track preview inside the Settings → Audio music library (`S-135`).
struct Preview {
    key: String,
    audio: BattleAudio,
    looping: bool,
    frames: u32,
}

/// Custom main-menu BGM selected by the user (`S-135`), overriding the selected game's title theme.
struct CustomMenuMusic {
    key: String,
    audio: BattleAudio,
    cue: SoundCue,
    playing: bool,
}

fn theme_audio(theme: Theme) -> (BattleAudio, SoundCue) {
    let songs = Songs(vec![BankSong { bank: BankId(0), song: theme.song }]);
    (BattleAudio::new(theme.bank, songs), SoundCue::Music(SoundId(0)))
}

pub struct Sound {
    out: Option<Output>,
    /// True for [`Sound::silent`] (screenshot / headless mode), which must never open an audio device.
    silent: bool,
    /// Backoff timestamp before retrying [`Output::open`] after a stream error or failed reopen.
    reopen_after: Option<Instant>,
    menu: Option<Menu>,
    /// Optional custom main-menu BGM overriding `menu.music` across menu screens.
    custom_menu: Option<CustomMenuMusic>,
    /// Optional single-track preview player (suppresses menu music while active).
    preview: Option<Preview>,
    /// Which game's sounds the menus have.
    game: Option<String>,
    /// The levels, 0 to 1: the music's, and the effects' (the battle's sound is at the effects').
    music: f32,
    effects: f32,
    /// The title music is wanted (a menu screen shows), and how much of it there is as it fades (0 to 1).
    wanted: bool,
    fade: f32,
    /// The game before's title music, fading out under this one's, and how much of it is left.
    leaving: Option<(BattleAudio, f32)>,
    faded: Vec<[f32; 2]>,
    preview_buf: Vec<[f32; 2]>,
    last: Option<Instant>,
    owed: f64,
    mix: Vec<[f32; 2]>,
    tune: Vec<[f32; 2]>,
    /// `SAITO_PLAY_STATS`: since when, and the loudest sample of the title music and of the whole stream since.
    stats: Option<(Instant, f32, f32)>,
}

impl Sound {
    pub fn open() -> Sound {
        let out = match Output::open() {
            Ok(o) => {
                o.set_volume(FULL);
                Some(o)
            }
            Err(e) => {
                crate::errors::log("opening the sound device", &e);
                None
            }
        };
        Sound {
            out,
            silent: false,
            reopen_after: None,
            stats: std::env::var_os("SAITO_PLAY_STATS").map(|_| (Instant::now(), 0.0, 0.0)),
            ..Sound::silent()
        }
    }

    /// Drop any stale/disconnected stream and open a fresh output device, restoring full volume.
    fn reopen(&mut self, now: Instant) {
        if self.silent {
            return;
        }
        self.out = None;
        match Output::open() {
            Ok(o) => {
                o.set_volume(FULL);
                self.out = Some(o);
                self.reopen_after = None;
            }
            Err(e) => {
                crate::errors::log("reopening the sound device", &e);
                self.reopen_after = Some(now + Duration::from_secs(1));
            }
        }
    }

    /// The player clicked or pressed a key: a browser starts a page's sound only then, so the device is started
    /// again (where it runs already, nothing changes).
    #[cfg(target_arch = "wasm32")]
    pub fn resume(&self) {
        if let Some(out) = &self.out {
            out.resume();
        }
    }

    /// The app went to the background (Android's: it would play on there): the device stops till the app comes
    /// back (`crate::lifecycle::resumed`, in [`Sound::frame`]).
    #[cfg(target_os = "android")]
    pub fn pause(&self) {
        if let Some(out) = &self.out {
            out.pause();
        }
    }

    /// No device: nothing is heard (the screenshot pass).
    pub fn silent() -> Sound {
        Sound {
            out: None,
            silent: true,
            reopen_after: None,
            menu: None,
            custom_menu: None,
            preview: None,
            game: None,
            music: 0.8,
            effects: 1.0,
            wanted: false,
            fade: 0.0,
            leaving: None,
            faded: Vec::new(),
            preview_buf: Vec::new(),
            last: None,
            owed: 0.0,
            mix: Vec::new(),
            tune: Vec::new(),
            stats: None,
        }
    }

    /// The sound queued for the device, in seconds.
    pub fn queued(&self) -> f64 {
        self.out.as_ref().map_or(0.0, Output::buffered)
    }

    /// The levels, each 0 to 100.
    pub fn set_levels(&mut self, music: u32, effects: u32) {
        self.music = music.min(100) as f32 / 100.0;
        self.effects = effects.min(100) as f32 / 100.0;
    }

    /// The levels a battle's sound is to be given (`Player::set_sound_levels`, a viewer's): its music's and
    /// its effects', against the louder of the two, which is what its samples are played at here. (The game's
    /// driver mixes the two with eight bits: turning one down inside it is coarser than turning samples down,
    /// so only the difference between the two is the driver's.)
    pub fn battle_levels(&self) -> (f32, f32) {
        let most = self.music.max(self.effects);
        if most <= 0.0 { (0.0, 0.0) } else { (self.music / most, self.effects / most) }
    }

    /// The menus' sounds and music from `game` (its id), if they aren't its already.
    pub fn use_game(&mut self, id: &str, game: &Loaded) {
        if self.game.as_deref() == Some(id) {
            return;
        }
        let Some(sound) = &game.sound else { return };
        let content = &game.game.content;
        let roles = content.defs.roles();
        let cue = |role| roles.try_sound(role).map(SoundCue::Effect);
        // The music of the game before fades out under this one's (unless a custom menu BGM is pinned).
        if self.custom_menu.is_none() {
            if let Some(old) = self.menu.take().filter(|m| m.playing) {
                self.leaving = Some((old.music, self.fade));
            }
            self.fade = 0.0;
        }
        let title = roles.try_music(MusicRole::TitleScreen).map(SoundCue::Music);
        self.menu = Some(Menu {
            effects: BattleAudio::new(sound.bank.clone(), sound.songs.clone()),
            cursor: cue(SoundRole::CustomCursor),
            pick: cue(SoundRole::CustomPick),
            back: cue(SoundRole::CustomBack),
            refused: cue(SoundRole::Refused),
            jack_in: cue(SoundRole::Encounter),
            music: BattleAudio::new(sound.bank.clone(), sound.songs.clone()),
            title,
            playing: false,
        });
        self.game = Some(id.to_string());
    }

    /// Set or clear a custom main-menu BGM (`key`: `"<game>/<song>"`, `theme`: resolved looping track).
    /// Passing `None` restores the selected game's default title screen music with a smooth crossfade.
    pub fn set_custom_menu_music(&mut self, key: Option<&str>, theme: Option<Theme>) {
        match (key, theme) {
            (Some(k), Some(theme)) => {
                if self.custom_menu.as_ref().is_some_and(|c| c.key == k) {
                    return;
                }
                if let Some(old) = self.custom_menu.take().filter(|c| c.playing) {
                    self.leaving = Some((old.audio, self.fade));
                } else if let Some(m) = self.menu.as_mut()
                    && m.playing
                {
                    self.leaving = Some((m.music.clone(), self.fade));
                    m.music.handle(&[SoundCue::StopMusic]);
                    m.playing = false;
                }
                self.fade = 0.0;
                let (audio, cue) = theme_audio(theme);
                self.custom_menu = Some(CustomMenuMusic { key: k.to_string(), audio, cue, playing: false });
            }
            _ => {
                if let Some(old) = self.custom_menu.take() {
                    if old.playing {
                        self.leaving = Some((old.audio, self.fade));
                    }
                    self.fade = 0.0;
                }
            }
        }
    }

    /// Key of the currently installed custom menu BGM, if any.
    #[cfg(test)]
    pub fn custom_menu_music_key(&self) -> Option<&str> {
        self.custom_menu.as_ref().map(|c| c.key.as_str())
    }

    /// Start previewing a single track (`key`: `"<game>/<song>"`, `theme`: M4A or SDAT).
    /// Menu music smoothly fades out while preview is active.
    pub fn start_preview(&mut self, key: &str, theme: Theme, looping: bool) {
        let (mut audio, cue) = theme_audio(theme);
        audio.handle(&[cue]);
        self.preview = Some(Preview { key: key.to_string(), audio, looping, frames: 0 });
    }

    /// Stop any active track preview (menu music fades back in if wanted).
    pub fn stop_preview(&mut self) {
        self.preview = None;
    }

    /// Key of the track currently previewing, if any.
    pub fn preview_key(&self) -> Option<&str> {
        self.preview.as_ref().map(|p| p.key.as_str())
    }

    pub fn play(&mut self, cue: Cue) {
        let Some(m) = &mut self.menu else { return };
        let cue = match cue {
            Cue::Cursor => m.cursor,
            Cue::Pick => m.pick,
            Cue::Back => m.back,
            Cue::Refused => m.refused,
            Cue::JackIn => m.jack_in,
        };
        if let Some(cue) = cue {
            m.effects.handle(&[cue]);
        }
    }

    /// Whether the title music should play: on the menu screens, not in a battle or on the cards before one.
    /// It starts from its beginning, and stops by fading out.
    pub fn want_music(&mut self, wanted: bool) {
        self.wanted = wanted;
    }

    /// A frame at `now`, which ran `ticks` of a battle that made `battle`'s samples (none: no battle runs, or
    /// it is paused): the menus' sounds and the title music are ticked with the battle's ticks, their samples
    /// added to the battle's where they fall; with no battle running, on their own clock at the same rate.
    /// What there is goes to the device.
    pub fn frame(&mut self, now: Instant, ticks: Option<u32>, battle: &mut Vec<[f32; 2]>) {
        // (A phone's app back from the background, or its sound given back after a call: the device stopped then,
        // and is started again — or recreated if Android disconnected/reclaimed the stream while backgrounded.)
        if crate::lifecycle::resumed() {
            let force_reopen = crate::lifecycle::take_reopen_audio();
            if force_reopen || self.out.as_ref().is_none_or(|out| !out.resume()) {
                self.reopen(now);
            }
        } else if !self.silent
            && self.reopen_after.is_none_or(|at| now >= at)
            && self.out.as_ref().is_some_and(Output::is_broken)
        {
            self.reopen(now);
        } else if !self.silent
            && self.out.is_none()
            && self.reopen_after.is_some_and(|at| now >= at)
        {
            self.reopen(now);
        }
        let elapsed = self.last.map_or(Duration::ZERO, |l| now.saturating_duration_since(l)).min(Duration::from_millis(250));
        self.last = Some(now);
        let due = match ticks {
            Some(n) => {
                self.owed = 0.0;
                n
            }
            None => {
                self.owed += elapsed.as_secs_f64() * FRAME_RATE;
                let due = self.owed.floor();
                self.owed -= due;
                due as u32
            }
        };
        self.mix.clear();
        self.tune.clear();
        self.faded.clear();
        self.preview_buf.clear();
        let menu_wanted = self.wanted && self.preview.is_none();
        let mut fade = (self.fade, self.fade);
        if let Some(m) = &mut self.menu {
            for _ in 0..due {
                m.effects.tick(&mut self.mix);
            }
        }
        if let Some(custom) = &mut self.custom_menu {
            if menu_wanted && !custom.playing {
                custom.audio.handle(&[custom.cue]);
                custom.playing = true;
            }
            if menu_wanted {
                self.fade = (self.fade + elapsed.as_secs_f32() / FADE_IN).min(1.0);
            } else if custom.playing {
                self.fade = (self.fade - elapsed.as_secs_f32() / FADE).max(0.0);
                if self.fade == 0.0 {
                    custom.audio.handle(&[SoundCue::StopMusic]);
                    custom.playing = false;
                }
            }
            fade.1 = self.fade;
            for _ in 0..due {
                custom.audio.tick(&mut self.tune);
            }
        } else if let Some(m) = &mut self.menu {
            // The title music: started (from nothing, fading in) when it is wanted, faded out and then stopped
            // when it isn't.
            let (music, cue, playing) = (&mut m.music, m.title, &mut m.playing);
            if menu_wanted && !*playing {
                if let Some(cue) = cue {
                    music.handle(&[cue]);
                    *playing = true;
                }
            }
            if menu_wanted {
                self.fade = (self.fade + elapsed.as_secs_f32() / FADE_IN).min(1.0);
            } else if *playing {
                self.fade = (self.fade - elapsed.as_secs_f32() / FADE).max(0.0);
                if self.fade == 0.0 {
                    music.handle(&[SoundCue::StopMusic]);
                    *playing = false;
                }
            }
            fade.1 = self.fade;
            for _ in 0..due {
                music.tick(&mut self.tune);
            }
        }
        if let Some(preview) = &mut self.preview {
            for _ in 0..due {
                preview.audio.tick(&mut self.preview_buf);
                preview.frames = preview.frames.saturating_add(1);
            }
            // Non-looping short tracks stop automatically once their sequence completes.
            if !preview.looping && preview.frames > 2 && preview.audio.playing_music_of(BankId(0)).is_none() {
                self.preview = None;
            }
        }
        // The game before's music, on its way out.
        let mut left = (0.0, 0.0);
        if let Some((music, level)) = &mut self.leaving {
            left.0 = *level;
            *level = (*level - elapsed.as_secs_f32() / FADE).max(0.0);
            left.1 = *level;
            for _ in 0..due {
                music.tick(&mut self.faded);
            }
            if *level == 0.0 {
                self.leaving = None;
            }
        }
        // (The battle's own sound: its music and its effects are set apart inside it, `battle_levels`.)
        let most = self.music.max(self.effects);
        for s in battle.iter_mut() {
            s[0] *= most;
            s[1] *= most;
        }
        if ticks.is_none() {
            battle.clear();
            let len = self.mix.len().max(self.tune.len()).max(self.preview_buf.len()).max(self.faded.len());
            battle.resize(len, [0.0; 2]);
        }
        // (Added sample for sample; a sample the menus' drivers made past the battle's frames is let go, so the
        // stream keeps the battle's rate. The music's level goes from the last frame's to this one's across
        // the frame, so a fade has no steps.)
        let count = battle.len().max(1) as f32;
        for (i, b) in battle.iter_mut().enumerate() {
            let effect = self.mix.get(i).copied().unwrap_or_default();
            let tune = self.tune.get(i).copied().unwrap_or_default();
            let old = self.faded.get(i).copied().unwrap_or_default();
            let prev = self.preview_buf.get(i).copied().unwrap_or_default();
            let along = i as f32 / count;
            let level = self.music * (fade.0 + (fade.1 - fade.0) * along);
            let leaving = self.music * (left.0 + (left.1 - left.0) * along);
            b[0] += effect[0] * self.effects + tune[0] * level + old[0] * leaving + prev[0] * self.music;
            b[1] += effect[1] * self.effects + tune[1] * level + old[1] * leaving + prev[1] * self.music;
        }
        // (`SAITO_PLAY_STATS`: what can't be seen in a screenshot, said each second.)
        if let Some((since, tune, stream)) = &mut self.stats {
            let peak = |samples: &[[f32; 2]], from: f32| samples.iter().fold(from, |p, s| p.max(s[0].abs()).max(s[1].abs()));
            (*tune, *stream) = (peak(&self.tune, *tune), peak(battle, *stream));
            if now.saturating_duration_since(*since) >= Duration::from_secs(1) {
                eprintln!("sound stats: the title music at {:.2} of its level (its samples' peak {tune:.3}); the stream's peak {stream:.3}", self.fade);
                (*since, *tune, *stream) = (now, 0.0, 0.0);
            }
        }
        if let Some(out) = &self.out
            && !battle.is_empty()
        {
            out.queue(battle);
        }
        battle.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saito_audio::Songs;
    use saito_audio::m4a::SoundBank;
    use saito_audio::m4a::bank::MixerConfig;
    use saito_battle::sound::SoundId;
    use std::sync::Arc;

    fn dummy_sound() -> Sound {
        let bank = Arc::new(SoundBank {
            mixer: MixerConfig { mix_rate: 13379, ds_channels: 8, master_volume: 15, reverb: 0, dac_resolution: 2 },
            players: Vec::new(),
            songs: Vec::new(),
            voicegroups: Vec::new(),
            key_maps: Vec::new(),
            samples: Vec::new(),
            waves: Vec::new(),
        });
        let songs = Songs::numbers(1);
        let mut s = Sound::silent();
        s.menu = Some(Menu {
            effects: BattleAudio::new(bank.clone(), songs.clone()),
            cursor: None,
            pick: None,
            back: None,
            refused: None,
            jack_in: None,
            music: BattleAudio::new(bank, songs),
            title: Some(SoundCue::Music(SoundId(0))),
            playing: false,
        });
        s
    }

    #[test]
    fn title_music_stays_suppressed_across_zero_tick_spectate_frames() {
        let mut s = dummy_sound();
        let mut t = Instant::now();
        let mut battle = Vec::new();
        // Title music fades up while in menus (`want_music(true)`).
        s.want_music(true);
        for _ in 0..30 {
            t += Duration::from_millis(16);
            s.frame(t, None, &mut battle);
        }
        assert!(s.fade > 0.9);
        assert!(s.menu.as_ref().unwrap().playing);

        // Entering spectate suppresses title music even on frames with `ticks == Some(0)`.
        s.want_music(false);
        for i in 0..80 {
            t += Duration::from_millis(16);
            let ticks = if i % 2 == 0 { Some(0) } else { Some(1) };
            s.frame(t, ticks, &mut battle);
        }
        assert_eq!(s.fade, 0.0);
        assert!(!s.menu.as_ref().unwrap().playing);
    }

    #[test]
    fn preview_suppresses_menu_music_and_custom_menu_bgm_activates() {
        let mut s = dummy_sound();
        let mut t = Instant::now();
        let mut battle = Vec::new();
        let bank = Arc::new(SoundBank {
            mixer: MixerConfig { mix_rate: 13379, ds_channels: 8, master_volume: 15, reverb: 0, dac_resolution: 2 },
            players: Vec::new(),
            songs: Vec::new(),
            voicegroups: Vec::new(),
            key_maps: Vec::new(),
            samples: Vec::new(),
            waves: Vec::new(),
        });

        s.want_music(true);
        s.set_custom_menu_music(Some("exe6/acdc-town"), Some(Theme::m4a(bank.clone(), 0)));
        assert_eq!(s.custom_menu_music_key(), Some("exe6/acdc-town"));
        for _ in 0..30 {
            t += Duration::from_millis(16);
            s.frame(t, None, &mut battle);
        }
        assert!(s.fade > 0.9);

        // Starting a preview fades out custom menu music cleanly.
        s.start_preview("exe6/virus-battle", Theme::m4a(bank.clone(), 0), true);
        assert_eq!(s.preview_key(), Some("exe6/virus-battle"));
        for _ in 0..80 {
            t += Duration::from_millis(16);
            s.frame(t, None, &mut battle);
        }
        assert_eq!(s.fade, 0.0, "menu music suppressed while previewing");

        // Stopping preview restores menu music fade-in.
        s.stop_preview();
        assert_eq!(s.preview_key(), None);
        for _ in 0..30 {
            t += Duration::from_millis(16);
            s.frame(t, None, &mut battle);
        }
        assert!(s.fade > 0.9, "menu music fades back in after stopping preview");

        // Non-looping empty/finished preview auto-stops after initial frames.
        s.start_preview("exe6/transmission", Theme::m4a(bank, 0), false);
        for _ in 0..5 {
            t += Duration::from_millis(16);
            s.frame(t, None, &mut battle);
        }
        assert_eq!(s.preview_key(), None, "finished non-looping jingle stops preview automatically");

        s.set_custom_menu_music(None, None);
        assert_eq!(s.custom_menu_music_key(), None);
    }
}
