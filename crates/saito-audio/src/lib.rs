//! Battle audio: plays saito-battle's sound cues with the game's own sound
//! driver (the `m4a` crate) and sound data (an `m4a::SoundBank`, which
//! saito-extract extracts from the user's ROMs).
//!
//! - [`Songs`]: what each of the engine's sounds (a handle over the
//!   content's sounds) is in a bank's song table; [`Songs::cue`] turns a
//!   cue's sounds into songs.
//! - [`Theme`] and [`with_theme`]: a battle's music in place of its game's
//!   own, a song of any game's played with that game's bank.
//! - [`RoundMusic`]: a battle's music by its round's background, which
//!   [`BattleAudio::round`] switches to as a round starts.
//! - [`SoundCalls`]: what the game's sound functions ask of the driver for
//!   each cue in songs (EXE6's wrappers: `PlayMusic`'s current-music check,
//!   the pinch effect's pitch and tempo, ...), as [`Request`]s.
//! - [`BattleAudio`]: cues in, samples out, a frame at a time, with the
//!   game's timing (calls queue up and run on the next frame), its Direct
//!   Sound mixed with ipatix's HQ mixer's techniques (`m4a::Mixer::Hq`, the
//!   driver's default).
//! - [`AudioOut`] (feature `playback`): the same on the default output device;
//!   [`Output`] is that device alone, playing samples it is given.
//! - [`wav`]: writing rendered audio.
//!
//! A frontend drives it once per battle tick:
//!
//! ```ignore
//! battle.tick(&input, events);
//! audio.handle(battle.sound_cues());
//! audio.tick();
//! ```
//!
//! Under rollback netplay a frontend feeds each simulated tick's cues to a
//! [`saito_battle::cues::CueTracker`] instead and hands its actions to
//! [`BattleAudio::handle_actions`]: confirmed cues play once, and cues
//! played on a wrong prediction are stopped or undone.

#![forbid(unsafe_code)]

use m4a::{Driver, Mixer, PlayerId, SongId, SoundBank};
use std::sync::Arc;

pub use saito_battle::cues::CueAction;
pub use saito_battle::sound::{SoundCue, SoundId};

/// The game's "no music" song: `PlayMusic` of it stops the music (its
/// battle settings name it for a battle without music). A song, not a
/// sound handle: a cue's sounds are songs once [`Songs::cue`] has them.
pub const NO_MUSIC: SoundId = SoundId(0x63);

/// A bank among a battle's (`BattleAudio::with_banks`), by its place: the
/// game's own first, a battle theme's of another game after it ([`with_theme`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BankId(pub u8);

impl BankId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A song of one of a battle's banks: the bank, and the song's number in
/// its song table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BankSong {
    pub bank: BankId,
    pub song: u16,
}

/// What the engine's sounds are in the battle's banks' song tables
/// (docs/design/rules-in-luau.md §7.4: the engine knows a sound by its
/// handle), by handle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Songs(pub Vec<BankSong>);

impl Songs {
    /// A content's sounds' songs in its game's bank (`assets`: what the
    /// game's assets hold of the content's), the bank the first.
    pub fn of(assets: &saito_battle::content::AssetNumbers) -> Songs {
        Songs(assets.sounds.iter().map(|&song| BankSong { bank: BankId::default(), song }).collect())
    }

    /// Songs by their own numbers (sound `n` is song `n` of the one bank):
    /// for tools that play a bank's songs, not the engine's sounds.
    pub fn numbers(count: usize) -> Songs {
        Songs((0..count).map(|n| BankSong { bank: BankId::default(), song: n as u16 }).collect())
    }

    /// Sound `id`'s song.
    pub fn song(&self, id: SoundId) -> SoundId {
        SoundId(self.0.get(id.0 as usize).unwrap_or_else(|| panic!("no sound has handle {}", id.0)).song)
    }

    /// These songs with the sounds `music` (a battle's own music: its
    /// content's `battle_music`) playing song `to` in their place.
    pub fn with_music(&self, music: &[SoundId], to: BankSong) -> Songs {
        let mut songs = self.clone();
        for &m in music {
            if let Some(s) = songs.0.get_mut(m.0 as usize) {
                *s = to;
            }
        }
        songs
    }

    /// `cue` with its sounds as songs: what [`SoundCalls`] takes.
    pub fn cue(&self, cue: SoundCue) -> SoundCue {
        match cue {
            SoundCue::Effect(id) => SoundCue::Effect(self.song(id)),
            SoundCue::Music(id) => SoundCue::Music(self.song(id)),
            other => other,
        }
    }
}
pub use m4a;
pub use sdat;

/// A sound bank playable by [`BattleAudio`]: either a GBA M4A sound bank or an
/// NDS NitroComposer (`SDAT`) sound bank.
#[derive(Clone)]
pub enum AudioBank {
    M4a(Arc<SoundBank>),
    Sdat(Arc<sdat::SoundBank>),
}

impl AudioBank {
    /// Whether two handles refer to the exact same underlying sound bank.
    pub fn ptr_eq(&self, other: &AudioBank) -> bool {
        match (self, other) {
            (AudioBank::M4a(a), AudioBank::M4a(b)) => Arc::ptr_eq(a, b),
            (AudioBank::Sdat(a), AudioBank::Sdat(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl From<Arc<SoundBank>> for AudioBank {
    fn from(bank: Arc<SoundBank>) -> Self {
        AudioBank::M4a(bank)
    }
}

impl From<Arc<sdat::SoundBank>> for AudioBank {
    fn from(bank: Arc<sdat::SoundBank>) -> Self {
        AudioBank::Sdat(bank)
    }
}

/// A battle's banks and its songs, as [`BattleAudio::with_banks`] and
/// [`BattleAudio::set_songs`] take them.
pub type Parts = (Vec<AudioBank>, Songs);

/// A battle's music by its round's background (docs/app.md §1, "The
/// battle's music"): the parts a round plays (each the game's own with its
/// background's music in place of its own music, [`with_theme`]'s), by
/// the background it is drawn on; a round on a background it has none for
/// plays `own`.
pub struct RoundMusic {
    pub own: Parts,
    pub by_background: std::collections::HashMap<saito_battle::content::BackgroundId, Parts>,
}

impl RoundMusic {
    /// What a round drawn on `background` plays.
    pub fn parts(&self, background: saito_battle::content::BackgroundId) -> &Parts {
        self.by_background.get(&background).unwrap_or(&self.own)
    }
}

/// A song of a game's, with that game's bank: a battle's music in place of
/// its game's own (a battle theme a player picked, of the battle's game or
/// another's). The song plays on a driver of its own bank, so with its own
/// game's voice groups and samples, and the battle stays its game's: its
/// effects and its result's music are its game's bank's, as before.
#[derive(Clone)]
pub struct Theme {
    pub bank: AudioBank,
    /// Its number in the bank's song table.
    pub song: u16,
}

impl Theme {
    pub fn m4a(bank: Arc<SoundBank>, song: u16) -> Theme {
        Theme { bank: AudioBank::M4a(bank), song }
    }

    pub fn sdat(bank: Arc<sdat::SoundBank>, song: u16) -> Theme {
        Theme { bank: AudioBank::Sdat(bank), song }
    }
}

/// A battle's sound, its `banks` (by [`BankId`]) and its `songs`, with
/// `theme`'s song as its music in place of its own (`music`: the sounds
/// that are its own music, `Content::battle_music`): the theme's bank one
/// more, unless it is one of `banks` already (a theme of the battle's
/// own game, which plays on its game's driver as the game's own music does).
pub fn with_theme(banks: impl IntoIterator<Item = impl Into<AudioBank>>, songs: &Songs, music: &[SoundId], theme: &Theme) -> Parts {
    let mut banks: Vec<AudioBank> = banks.into_iter().map(Into::into).collect();
    let at = match banks.iter().position(|b| b.ptr_eq(&theme.bank)) {
        Some(p) => p,
        None => {
            banks.push(theme.bank.clone());
            banks.len() - 1
        }
    };
    let to = BankSong { bank: BankId(at as u8), song: theme.song };
    (banks, songs.with_music(music, to))
}

pub mod mic;
#[cfg(feature = "playback")]
mod output;
pub mod wav;

pub use mic::MicMonitor;
#[cfg(feature = "playback")]
pub use output::{AudioOut, MAX_RATE_SHIFT, Output, OutputError, OutputStats, RESUME_FILL, TARGET_FILL};

/// The music player EXE6's background music plays on.
pub const MUSIC_PLAYER: PlayerId = PlayerId(31);
/// Sound calls the game queues per frame (`sound_8000808` drops the rest).
pub const QUEUE_LIMIT: usize = 32;
/// Output rate, Hz.
pub const SAMPLE_RATE: u32 = m4a::OUT_RATE;
/// Frames (battle ticks) per second.
pub const FPS: f64 = m4a::FPS;

/// A call into the sound driver, as the game queues it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Request {
    /// `m4aSongNumStart`.
    Start(SongId),
    /// `m4aSongNumStop`: stop a song if its player is still playing it
    /// (only to take back a cue played on a wrong prediction).
    Stop(SongId),
    /// `m4aMPlayAllStop`.
    StopAll,
    /// `m4aMPlayTempoControl` (0x100 = as written).
    Tempo { player: PlayerId, tempo: u16 },
    /// `m4aMPlayPitchControl` (1/256 semitone) on the tracks of a bit mask.
    Pitch { player: PlayerId, tracks: u16, pitch: i16 },
    /// `m4aMPlayVolumeControl` (0x100 = full) on the tracks of a bit mask.
    Volume { player: PlayerId, tracks: u16, volume: u16 },
}

impl Request {
    /// Run it on a driver.
    pub fn apply(self, driver: &mut Driver) {
        match self {
            Request::Start(id) => {
                driver.start(id);
            }
            Request::Stop(id) => driver.stop_song(id),
            Request::StopAll => driver.stop_all(),
            Request::Tempo { player, tempo } => driver.set_tempo(player, tempo),
            Request::Pitch { player, tracks, pitch } => driver.set_pitch(player, tracks, pitch),
            Request::Volume { player, tracks, volume } => driver.set_volume(player, tracks, volume),
        }
    }
}

/// EXE6's sound functions: each cue as the driver calls the game queues.
#[derive(Clone, Debug)]
pub struct SoundCalls {
    /// The music `PlayMusic` last started (GameState's BGMusicIndicator, a
    /// byte there: every game's music is a song under 0x100; held whole
    /// here, so a bank of more songs is no other; 0xFFFF: none).
    music: u16,
    /// What it was before the last change (to undo a canceled change).
    previous_music: u16,
}

impl Default for SoundCalls {
    fn default() -> SoundCalls {
        SoundCalls { music: NO_INDICATOR, previous_music: NO_INDICATOR }
    }
}

const NO_INDICATOR: u16 = 0xFFFF;

impl SoundCalls {
    pub fn new() -> SoundCalls {
        SoundCalls::default()
    }

    /// The driver calls for a cue (its sounds as songs: [`Songs::cue`]),
    /// appended to `out`.
    pub fn requests(&mut self, cue: SoundCue, out: &mut Vec<Request>) {
        match cue {
            // PlaySoundEffect: m4aSongNumStart through the queue.
            SoundCue::Effect(id) => out.push(Request::Start(SongId(id.0))),
            // PlayMusic: nothing if it's the music already.
            SoundCue::Music(id) => {
                if id.0 == self.music {
                    return;
                }
                self.previous_music = self.music;
                self.music = id.0;
                out.push(if id == NO_MUSIC { Request::StopAll } else { Request::Start(SongId(id.0)) });
            }
            // musicGameState_8000784.
            SoundCue::StopMusic => {
                self.previous_music = self.music;
                self.music = NO_INDICATOR;
                out.push(Request::StopAll);
            }
            // sub_8009158: a semitone up at 0x11A/0x100 tempo, or back.
            SoundCue::Pinch(on) => {
                let (pitch, tempo) = if on { (0x100, 0x11A) } else { (0, 0x100) };
                out.push(Request::Pitch { player: MUSIC_PLAYER, tracks: 0xFFFF, pitch });
                out.push(Request::Tempo { player: MUSIC_PLAYER, tempo });
            }
            // sub_8000642: m4aMPlayVolumeControl on all of its tracks.
            SoundCue::Volume { player, volume } => out.push(Request::Volume { player: PlayerId(player), tracks: 0xFFFF, volume }),
        }
    }
}

impl SoundCalls {
    /// The driver calls that take back a cue played on a wrong prediction
    /// (rollback netplay), appended to `out`: a sound effect stops if it
    /// is still playing, a music change goes back to the music before it
    /// (from its start), the pinch effect is switched back.
    pub fn cancel(&mut self, cue: SoundCue, out: &mut Vec<Request>) {
        match cue {
            SoundCue::Effect(id) => out.push(Request::Stop(SongId(id.0))),
            SoundCue::Music(id) if id.0 == self.music => self.restore_previous_music(out),
            SoundCue::StopMusic if self.music == NO_INDICATOR => self.restore_previous_music(out),
            SoundCue::Pinch(on) => self.requests(SoundCue::Pinch(!on), out),
            // A later change replaced it already; the custom screen's
            // volume can't be taken back.
            SoundCue::Music(_) | SoundCue::StopMusic | SoundCue::Volume { .. } => {}
        }
    }

    /// The music moved to another bank's player: this one's song stops
    /// (that alone: m4aSongNumStop, the bank's effects playing on), and is
    /// what a taken-back move starts again (`cancel` of a `StopMusic`).
    fn stop_moved(&mut self, out: &mut Vec<Request>) {
        self.previous_music = std::mem::replace(&mut self.music, NO_INDICATOR);
        if self.previous_music != NO_INDICATOR && self.previous_music != NO_MUSIC.0 {
            out.push(Request::Stop(SongId(self.previous_music)));
        }
    }

    /// Take back music `id`, which moved the music here from another bank's
    /// player (it goes back there): it stops, and the music before it here
    /// is this player's again, without starting.
    fn take_back_moved(&mut self, id: SoundId, out: &mut Vec<Request>) {
        self.music = std::mem::replace(&mut self.previous_music, self.music);
        out.push(if id == NO_MUSIC { Request::StopAll } else { Request::Stop(SongId(id.0)) });
    }

    fn restore_previous_music(&mut self, out: &mut Vec<Request>) {
        let music = std::mem::replace(&mut self.music, self.previous_music);
        self.previous_music = music;
        out.push(match self.music {
            NO_INDICATOR => Request::StopAll,
            m if m == NO_MUSIC.0 => Request::StopAll,
            m => Request::Start(SongId(m)),
        });
    }
}

/// The game's sound, fed with cues and rendered a frame at a time: a
/// driver for each of its banks (docs/design/rules-in-luau.md §7.4: a song
/// plays with its own bank's instruments; two banks never mix inside one
/// player), their outputs added.
///
/// A copy of it is the sound as it stands, to come back to: fed the same
/// cues from there, it gives the same samples (the banks are shared, the
/// rest is the drivers' state). A replay viewer keeps one with each
/// snapshot of the battle, so a seek lands in the sound that playing
/// straight there makes.
#[derive(Clone)]
pub struct BattleAudio {
    banks: Vec<BankSound>,
    songs: Songs,
    /// The bank whose music player plays the battle's music, once some
    /// music has started.
    music: Option<usize>,
    /// The bank the last music change moved the music from (it stopped
    /// that bank's), to move it back there if the change is taken back.
    moved_from: Option<usize>,
    /// The battle's music as its cues name it (a sound's handle; none:
    /// stopped), and what it was before the last change: what a change of
    /// the songs ([`BattleAudio::set_songs`]) starts again as its new song.
    heard: Option<SoundId>,
    heard_before: Option<SoundId>,
    /// The low-HP music's switch is on (`Pinch`).
    pinch: bool,
    /// The host's levels ([`BattleAudio::set_levels`]), for a bank added
    /// later.
    levels: (f32, f32),
    /// How the drivers mix ([`BattleAudio::set_mixer`]), for a bank added later.
    mixer: Mixer,
    mix: Vec<[f32; 2]>,
    /// The music by the round's background ([`BattleAudio::set_round_music`]),
    /// and the background whose parts play now.
    round_music: Option<Arc<RoundMusic>>,
    round: Option<saito_battle::content::BackgroundId>,
    /// Optional presentation-only NDS `SSAR` battle voice player (mixed at the
    /// host's effects level).
    voices: Option<sdat::VoicePlayer>,
}

fn apply_sdat(d: &mut sdat::Driver, r: Request) {
    match r {
        Request::Start(id) => d.start(sdat::SongId(id.0)),
        Request::Stop(id) => d.stop_song(sdat::SongId(id.0)),
        Request::StopAll => d.stop_all(),
        Request::Tempo { player, tempo } if player == MUSIC_PLAYER => d.set_tempo(tempo),
        Request::Pitch { player, pitch, .. } if player == MUSIC_PLAYER => d.set_pitch(pitch),
        Request::Volume { player, volume, .. } if player == MUSIC_PLAYER => d.set_volume(volume),
        Request::Tempo { .. } | Request::Pitch { .. } | Request::Volume { .. } => {}
    }
}

/// One bank's sound driver (GBA M4A or NDS NitroComposer SDAT).
#[derive(Clone)]
enum BankDriver {
    M4a(Driver),
    Sdat(sdat::Driver),
}

impl BankDriver {
    fn new(bank: AudioBank) -> Self {
        match bank {
            AudioBank::M4a(b) => BankDriver::M4a(Driver::new(b)),
            AudioBank::Sdat(b) => BankDriver::Sdat(sdat::Driver::new(b)),
        }
    }

    fn matches(&self, bank: &AudioBank) -> bool {
        match (self, bank) {
            (BankDriver::M4a(d), AudioBank::M4a(b)) => Arc::ptr_eq(d.bank(), b),
            (BankDriver::Sdat(d), AudioBank::Sdat(b)) => Arc::ptr_eq(d.bank(), b),
            _ => false,
        }
    }

    fn set_levels(&mut self, (music, effects): (f32, f32)) {
        let level = |l: f32| (l.clamp(0.0, 1.0) * 256.0).round() as u16;
        match self {
            BankDriver::M4a(driver) => {
                for player in 0..driver.players().len() {
                    let id = PlayerId(player as u8);
                    driver.set_level(id, level(if id == MUSIC_PLAYER { music } else { effects }));
                }
            }
            BankDriver::Sdat(driver) => {
                driver.set_level(level(music));
            }
        }
    }

    fn set_mixer(&mut self, mixer: Mixer) {
        if let BankDriver::M4a(driver) = self {
            driver.set_mixer(mixer);
        }
    }
}

/// One bank's driver and the calls queued for it.
#[derive(Clone)]
struct BankSound {
    driver: BankDriver,
    calls: SoundCalls,
    queue: Vec<Request>,
}

impl BattleAudio {
    /// The sound of `bank`, playing the engine's sounds as `songs` says
    /// (one bank's).
    pub fn new(bank: impl Into<AudioBank>, songs: Songs) -> BattleAudio {
        BattleAudio::with_banks([bank.into()], songs)
    }

    /// The sound of several banks, `banks` by `BankId`.
    pub fn with_banks(banks: impl IntoIterator<Item = impl Into<AudioBank>>, songs: Songs) -> BattleAudio {
        let banks: Vec<BankSound> = banks
            .into_iter()
            .map(|b| BankSound { driver: BankDriver::new(b.into()), calls: SoundCalls::new(), queue: Vec::new() })
            .collect();
        assert!(!banks.is_empty(), "a bank");
        let (music, moved_from, heard, heard_before) = (None, None, None, None);
        BattleAudio {
            banks,
            songs,
            music,
            moved_from,
            heard,
            heard_before,
            pinch: false,
            levels: (1.0, 1.0),
            mixer: Mixer::Hq,
            mix: Vec::new(),
            round_music: None,
            round: None,
            voices: None,
        }
    }

    /// The bank a cue plays on, and the cue in its songs: a sound's own
    /// bank; what changes the music, the music's bank (the first's before
    /// any).
    fn route(&self, cue: SoundCue) -> (usize, SoundCue) {
        let bank_of = |id: SoundId| {
            let a = self.songs.0.get(id.0 as usize).unwrap_or_else(|| panic!("no sound has handle {}", id.0));
            a.bank.index().min(self.banks.len() - 1)
        };
        match cue {
            SoundCue::Effect(id) | SoundCue::Music(id) => (bank_of(id), self.songs.cue(cue)),
            other => (self.music.unwrap_or(0), other),
        }
    }

    /// Queue `requests` for the driver of the bank at `at`.
    fn queue(&mut self, at: usize, requests: Vec<Request>) {
        let q = &mut self.banks[at].queue;
        for r in requests {
            if q.len() < QUEUE_LIMIT {
                q.push(r);
            }
        }
    }

    /// Play `cue`: a music change on another bank's player than the music's
    /// stops that one first; stopping the music stops every bank's; a
    /// player's volume is set in every bank (as one driver's would be: the
    /// music's bank has the music player's to set, the effects' the others').
    fn play(&mut self, cue: SoundCue) {
        match cue {
            SoundCue::StopMusic => self.heard_before = std::mem::take(&mut self.heard),
            SoundCue::Pinch(on) => self.pinch = on,
            _ => {}
        }
        if let SoundCue::StopMusic | SoundCue::Volume { .. } = cue {
            for p in 0..self.banks.len() {
                let mut r = Vec::new();
                self.banks[p].calls.requests(cue, &mut r);
                self.queue(p, r);
            }
            return;
        }
        let handle = cue;
        let (at, cue) = self.route(cue);
        if let (SoundCue::Music(id), SoundCue::Music(handle)) = (cue, handle) {
            // (PlayMusic of the music already playing changes nothing.)
            if id.0 != self.banks[at].calls.music {
                self.heard_before = self.heard.replace(handle);
                // Another bank's music player stops its song (that alone: the
                // bank's effects play on).
                self.moved_from = self.music.filter(|&m| m != at);
                if let Some(old) = self.moved_from {
                    let mut r = Vec::new();
                    self.banks[old].calls.stop_moved(&mut r);
                    self.queue(old, r);
                }
            }
            self.music = Some(at);
        }
        let mut r = Vec::new();
        self.banks[at].calls.requests(cue, &mut r);
        self.queue(at, r);
    }

    /// Queue a tick's cues; they run at the start of the next frame, as
    /// the game's queued sound calls do.
    pub fn handle(&mut self, cues: &[SoundCue]) {
        for &cue in cues {
            self.play(cue);
        }
    }

    /// Queue rollback cue actions (see [`saito_battle::cues`]): plays as
    /// [`handle`](Self::handle) does, and cancels of cues played on a
    /// wrong prediction.
    pub fn handle_actions(&mut self, actions: impl IntoIterator<Item = CueAction>) {
        for action in actions {
            match action {
                CueAction::Play(cue) => self.play(cue),
                CueAction::Cancel(cue) => {
                    let (at, cue) = self.route(cue);
                    let mut r = Vec::new();
                    // (What the music was, as the cues name it, is again what it
                    // was where the change is taken back; the low-HP switch, the
                    // other way.)
                    let taken_back = match cue {
                        SoundCue::Music(id) => self.banks[at].calls.music == id.0,
                        SoundCue::StopMusic => self.banks[at].calls.music == NO_INDICATOR,
                        _ => false,
                    };
                    if taken_back {
                        std::mem::swap(&mut self.heard, &mut self.heard_before);
                    }
                    if let SoundCue::Pinch(on) = cue {
                        self.pinch = !on;
                    }
                    // A music change that moved the music here from another
                    // bank's player moves it back: this bank's song stops,
                    // and the other's starts again from its start (as a
                    // change taken back on one player does).
                    if let (SoundCue::Music(id), Some(from)) = (cue, self.moved_from)
                        && self.music == Some(at)
                        && self.banks[at].calls.music == id.0
                    {
                        self.banks[at].calls.take_back_moved(id, &mut r);
                        self.queue(at, r);
                        let mut back = Vec::new();
                        self.banks[from].calls.cancel(SoundCue::StopMusic, &mut back);
                        self.queue(from, back);
                        self.music = Some(from);
                        self.moved_from = None;
                    } else {
                        self.banks[at].calls.cancel(cue, &mut r);
                        self.queue(at, r);
                    }
                }
            }
        }
    }

    /// The host's levels, each 0 to 1 (1: as the game plays it): the
    /// music's, which is the music player's notes, and the effects', which
    /// is every other player's ([`m4a::Driver::set_level`]: what sounds is
    /// the same, only how loud each is mixed). A host's "Music" and "Sound
    /// effects". They are the drivers' state: a copy kept before a change
    /// has the levels it was kept with.
    pub fn set_levels(&mut self, music: f32, effects: f32) {
        self.levels = (music, effects);
        for p in &mut self.banks {
            p.driver.set_levels(self.levels);
        }
        if let Some(voices) = &mut self.voices {
            voices.set_level((effects.clamp(0.0, 1.0) * 256.0).round() as u16);
        }
    }

    /// Attach or detach an optional NDS `SSAR` voice bank for presentation-only
    /// battle voice playback (mixed at the host's effects level).
    pub fn set_voice_bank(&mut self, bank: Option<Arc<sdat::VoiceBank>>) {
        self.voices = bank.map(|b| {
            let mut player = sdat::VoicePlayer::new(b);
            player.set_level((self.levels.1.clamp(0.0, 1.0) * 256.0).round() as u16);
            player
        });
    }

    /// Trigger a presentation-only voice clip `clip_id` on `channel`
    /// (`0` = operator/Lan, `1` = Navi). No-op if no voice bank is attached.
    pub fn play_voice(&mut self, clip_id: u16, channel: usize) {
        if let Some(voices) = &mut self.voices {
            voices.play(clip_id, channel);
        }
    }

    /// Stop any currently playing battle voice clips.
    pub fn stop_voices(&mut self) {
        if let Some(voices) = &mut self.voices {
            voices.stop_all();
        }
    }

    /// Mix every bank's Direct Sound as `mixer` says from the next frame on
    /// ([`m4a::Driver::set_mixer`]; [`Mixer::Hq`] at first). The driver's
    /// own mix, [`Mixer::Original`], is the game's to the byte.
    pub fn set_mixer(&mut self, mixer: Mixer) {
        self.mixer = mixer;
        for p in &mut self.banks {
            p.driver.set_mixer(mixer);
        }
    }

    /// Play the battle's sounds as `songs` says from now on, with `banks`
    /// (by the `BankId`s `songs` names): a host's change of the battle's
    /// music while it runs (a battle theme picked, or the game's own again:
    /// [`with_theme`]'s parts, or the game's own parts). A bank playing
    /// already is that one (found by its bank); another is one more, at
    /// the host's levels. Where the battle's music playing now is another
    /// song by them, that one starts in its place, from its beginning, on its
    /// own bank's player (the song before stopped, and the low-HP switch kept
    /// on where it is); the result's music, or music stopped, stays as it is,
    /// and the next round's start plays the new song.
    pub fn set_songs(&mut self, banks: impl IntoIterator<Item = impl Into<AudioBank>>, songs: Songs) {
        let banks: Vec<AudioBank> = banks.into_iter().map(Into::into).collect();
        let places: Vec<usize> = banks.iter().map(|bank| self.place_of(bank)).collect();
        let place = |s: BankSong| BankSong { bank: BankId(places.get(s.bank.index()).copied().unwrap_or(0) as u8), song: s.song };
        let songs = Songs(songs.0.into_iter().map(place).collect());
        let playing = self.heard.map(|h| (h, self.songs.0.get(h.0 as usize).copied()));
        self.songs = songs;
        if let Some((h, was)) = playing
            && self.songs.0.get(h.0 as usize).copied() != was
        {
            self.play(SoundCue::Music(h));
            if self.pinch {
                self.play(SoundCue::Pinch(true));
            }
        }
    }

    /// Play the battle's music by its round's background from now on
    /// (`music`), or none: the songs stay as they are. A host tells it each
    /// tick's background ([`BattleAudio::round`]), and the next it is told
    /// switches to that one's parts.
    pub fn set_round_music(&mut self, music: Option<Arc<RoundMusic>>) {
        self.round_music = music;
        self.round = None;
    }

    /// The cues handled next are of a round drawn on `background`: where it
    /// isn't the one before, the battle's songs are its parts from now on
    /// ([`BattleAudio::set_songs`]; a round's music starts with its first
    /// tick, so a host tells it before handling a tick's cues). Nothing
    /// without music by background.
    pub fn round(&mut self, background: saito_battle::content::BackgroundId) {
        if self.round == Some(background) {
            return;
        }
        let Some(music) = self.round_music.clone() else { return };
        self.round = Some(background);
        let (banks, songs) = music.parts(background).clone();
        self.set_songs(banks, songs);
    }

    /// The place of `bank` among the battle's: one playing already, else one
    /// more, at the host's levels.
    fn place_of(&mut self, bank: &AudioBank) -> usize {
        if let Some(p) = self.banks.iter().position(|p| p.driver.matches(bank)) {
            return p;
        }
        let mut driver = BankDriver::new(bank.clone());
        driver.set_mixer(self.mixer);
        driver.set_levels(self.levels);
        self.banks.push(BankSound { driver, calls: SoundCalls::new(), queue: Vec::new() });
        self.banks.len() - 1
    }

    /// One frame: each driver's VBlank (sequencers and mix), then its
    /// queued calls (the game's main loop runs them next). Appends the
    /// frame's samples (about 549 at 32768 Hz, stereo, +-1.0; every
    /// bank's added) to `out`.
    pub fn tick(&mut self, out: &mut Vec<[f32; 2]>) {
        let start = out.len();
        let mut frame_len = 0usize;
        for (i, p) in self.banks.iter_mut().enumerate() {
            if i == 0 {
                match &mut p.driver {
                    BankDriver::M4a(d) => {
                        d.step_frame();
                        for r in std::mem::take(&mut p.queue) {
                            r.apply(d);
                        }
                        d.take_output(out);
                    }
                    BankDriver::Sdat(d) => {
                        d.step_frame();
                        for r in std::mem::take(&mut p.queue) {
                            apply_sdat(d, r);
                        }
                        d.take_output(out);
                    }
                }
                frame_len = out.len() - start;
            } else {
                self.mix.clear();
                match &mut p.driver {
                    BankDriver::M4a(d) => {
                        d.step_frame();
                        for r in std::mem::take(&mut p.queue) {
                            r.apply(d);
                        }
                        d.take_output(&mut self.mix);
                    }
                    BankDriver::Sdat(d) => {
                        d.render_samples(frame_len, &mut self.mix);
                        for r in std::mem::take(&mut p.queue) {
                            apply_sdat(d, r);
                        }
                    }
                }
                for (o, m) in out[start..].iter_mut().zip(&self.mix) {
                    o[0] = (o[0] + m[0]).clamp(-1.0, 1.0);
                    o[1] = (o[1] + m[1]).clamp(-1.0, 1.0);
                }
            }
        }
        if let Some(voices) = &mut self.voices {
            voices.mix_into(&mut out[start..]);
        }
    }

    /// The first bank's GBA M4A driver.
    pub fn driver(&self) -> &Driver {
        match &self.banks[0].driver {
            BankDriver::M4a(d) => d,
            BankDriver::Sdat(_) => panic!("bank 0 is an SDAT bank, not M4A"),
        }
    }

    pub fn driver_mut(&mut self) -> &mut Driver {
        match &mut self.banks[0].driver {
            BankDriver::M4a(d) => d,
            BankDriver::Sdat(_) => panic!("bank 0 is an SDAT bank, not M4A"),
        }
    }

    /// Bank `bank`'s GBA M4A driver (`None` if out of bounds or an SDAT bank).
    pub fn driver_of(&self, bank: BankId) -> Option<&Driver> {
        match &self.banks.get(bank.index())?.driver {
            BankDriver::M4a(d) => Some(d),
            BankDriver::Sdat(_) => None,
        }
    }

    /// Bank `bank`'s NDS SDAT driver (`None` if out of bounds or an M4A bank).
    pub fn sdat_driver_of(&self, bank: BankId) -> Option<&sdat::Driver> {
        match &self.banks.get(bank.index())?.driver {
            BankDriver::Sdat(d) => Some(d),
            BankDriver::M4a(_) => None,
        }
    }

    /// The song currently playing on `bank`'s music player (works uniformly for
    /// both M4A and SDAT banks).
    pub fn playing_music_of(&self, bank: BankId) -> Option<u16> {
        match &self.banks.get(bank.index())?.driver {
            BankDriver::M4a(d) => d.player(MUSIC_PLAYER).filter(|m| m.is_playing()).and_then(|m| m.song()).map(|s| s.0),
            BankDriver::Sdat(d) => d.song().map(|s| s.0),
        }
    }
}

#[cfg(test)]
mod tests;
