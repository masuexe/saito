//! Spectator mode for Play:
//! - [`HostPublisher`]: streams settled ticks from the room host's match to the
//!   signaling server's `/spectate/host/<CODE>` relay without interfering with
//!   P1/P2 WebRTC rollback netplay.
//! - [`WatchSession`]: connects to `/spectate/watch/<CODE>`, follows a room
//!   across lobby waiting and multiple sets, handles mid-match catch-up, and
//!   renders live frames & audio through `saito_frontend::viewer::Viewer`.
//! - [`DirectoryClient`]: subscribes to `/spectate/list` for the public live
//!   matches list.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use saito_battle::Content;
use saito_battle::content::BackgroundId;
use saito_battle::setup::{BattleSettings, SetScore};
use saito_content_api::StageHandle;
use saito_frontend::game::{Enhancements, Loaded, Sound};
use saito_frontend::lobby::Agreement;
use saito_frontend::replay::{RecordedRoundStart, Recorder};
use saito_frontend::viewer::Viewer;
use saito_match::RoundStart;
use saito_replay::{Header, Tick, TickDecoder};
use saito_signaling::spectate::{
    HostEvent, RoomMeta, RoomStage, RoomSummary, RoundCheckpoint, SetSnapshot, SpectateDirectory, SpectateHost,
    SpectateWatcher, Visibility, WatcherEvent, base64_decode,
};
use web_time::Instant;

use crate::shell::Shell;
use crate::stage::Surface;

/// How often the host flushes pending ticks to the relay (even if fewer than
/// `BATCH_TICKS` accumulated).
const FLUSH_INTERVAL: Duration = Duration::from_millis(200);
/// Batch size before flushing immediately.
const BATCH_TICKS: u32 = 12;
/// How many silent ticks a mid-match spectator runs per UI frame while catching up.
const CATCHUP_TICKS_PER_FRAME: u32 = 60;
/// Lookahead margin (ticks) kept during catch-up so custom screen cursor layers stay exact.
const CATCHUP_MARGIN: u32 = 6;
/// Enter silent catch-up mode when more than this many ticks behind live.
const CATCHUP_START_LAG: u32 = 90;
/// Exit silent catch-up mode once within this many ticks of live.
const CATCHUP_END_LAG: u32 = 18;
/// Supported anti-ghosting delay presets (seconds).
pub const DELAY_PRESETS: [u16; 4] = [0, 5, 10, 15];

// ---- Host publisher ------------------------------------------------------------

/// `saito-replay` tick control bits (mirrors `saito_replay::ROUND_END` / `SET_END`).
const REPLAY_ROUND_END_BIT: u8 = 1 << 3;
const REPLAY_SET_END_BIT: u8 = 1 << 4;

#[derive(Default)]
struct SinkState {
    header_done: bool,
    header: Option<Vec<u8>>,
    /// Complete chunks split cleanly on round boundaries before `pending`.
    completed: VecDeque<(u32, Vec<u8>)>,
    /// Accumulated tick bytes since the last drain.
    pending: Vec<u8>,
    /// Number of complete tick records in `pending`.
    pending_ticks: u32,
    /// Bytes written inside the current `write_all` call before `flush()`.
    current_record: Vec<u8>,
}

struct SpectateSink {
    state: Arc<Mutex<SinkState>>,
}

impl Write for SpectateSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(mut s) = self.state.lock() {
            s.current_record.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut s) = self.state.lock() {
            if s.current_record.is_empty() {
                return Ok(());
            }
            let bytes = std::mem::take(&mut s.current_record);
            if !s.header_done {
                s.header_done = true;
                s.header = Some(bytes);
            } else {
                let round_boundary = (bytes[0] & REPLAY_ROUND_END_BIT) != 0 && (bytes[0] & REPLAY_SET_END_BIT) == 0;
                s.pending.extend_from_slice(&bytes);
                s.pending_ticks += 1;
                if round_boundary {
                    let count = s.pending_ticks;
                    let chunk = std::mem::take(&mut s.pending);
                    s.pending_ticks = 0;
                    s.completed.push_back((count, chunk));
                }
            }
        }
        Ok(())
    }
}

/// Host-side spectator publisher attached to an invite room.
pub struct HostPublisher {
    host: SpectateHost,
    visibility: Visibility,
    delay_secs: u16,
    spectators: u32,
    set_id: u32,
    sent_ticks: u32,
    active_sink: Option<Arc<Mutex<SinkState>>>,
    pending_checkpoints: VecDeque<RoundCheckpoint>,
    last_flush: Instant,
    last_meta: Option<RoomMeta>,
}

impl HostPublisher {
    pub fn start(server: &str, code: &str, peer: &str, visibility: Visibility, delay_secs: u16) -> HostPublisher {
        HostPublisher {
            host: SpectateHost::connect(server, code, peer),
            visibility,
            delay_secs,
            spectators: 0,
            set_id: 0,
            sent_ticks: 0,
            active_sink: None,
            pending_checkpoints: VecDeque::new(),
            last_flush: Instant::now(),
            last_meta: None,
        }
    }

    pub fn visibility(&self) -> Visibility {
        self.visibility
    }

    pub fn set_visibility(&mut self, visibility: Visibility) {
        self.visibility = visibility;
    }

    pub fn delay_secs(&self) -> u16 {
        self.delay_secs
    }

    pub fn set_delay_secs(&mut self, secs: u16) {
        self.delay_secs = secs;
    }

    pub fn spectators(&self) -> u32 {
        self.spectators
    }

    /// Create a `Recorder` whose header and settled ticks stream directly to this publisher.
    pub fn make_recorder(&mut self, content: &Arc<Content>, agreed: &Agreement, when: u64) -> Option<Recorder> {
        self.set_id = self.set_id.wrapping_add(1).max(1);
        self.sent_ticks = 0;
        self.pending_checkpoints.clear();
        let state = Arc::new(Mutex::new(SinkState::default()));
        let sink = SpectateSink { state: state.clone() };
        match Recorder::of_agreement(Box::new(sink), content, agreed, when) {
            Ok(recorder) => {
                self.active_sink = Some(state);
                self.last_flush = Instant::now();
                Some(recorder)
            }
            Err(why) => {
                crate::errors::log("starting spectator stream recorder", &why);
                None
            }
        }
    }

    /// Queue a round >= 2 transition checkpoint so late-joining spectators start at the new round.
    pub fn push_round_start(&mut self, rs: &RecordedRoundStart) {
        self.pending_checkpoints.push_back(RoundCheckpoint {
            from_tick: rs.from_tick,
            stage: rs.start.settings.stage.0 as u32,
            background: rs.start.settings.background.0,
            effects: rs.start.settings.effects,
            wins: rs.start.score.wins,
            losses: rs.start.score.losses,
            round: rs.start.score.round,
            max_combo: rs.start.score.max_combo,
            console_rng: rs.start.console_rng,
            console_frames: rs.start.console_frames,
            prev_inputs: rs.prev_inputs,
            prev_delays: rs.prev_delays,
        });
    }

    /// Flush any remaining ticks and end the active set on the relay.
    pub fn end_set(&mut self, reason: &str) {
        self.flush_sink(true);
        self.pending_checkpoints.clear();
        if self.active_sink.take().is_some() {
            self.host.end_set(self.set_id, reason);
        }
    }

    /// Poll the host socket, send metadata updates on change, and flush batched ticks.
    pub fn poll(&mut self, now: Instant, mut meta: RoomMeta) {
        meta.visibility = self.visibility;
        meta.delay_secs = self.delay_secs;
        if self.last_meta.as_ref() != Some(&meta) {
            self.last_meta = Some(meta.clone());
            self.host.update_meta(meta);
        }
        self.flush_sink(now.saturating_duration_since(self.last_flush) >= FLUSH_INTERVAL);
        while let Some(ev) = self.host.poll(now) {
            match ev {
                HostEvent::Connected { spectators } => self.spectators = spectators,
                HostEvent::Spectators(n) => self.spectators = n,
                HostEvent::Refused { reason, message } => {
                    crate::errors::log("spectate host refused", &format_args!("{reason:?}: {message}"));
                }
            }
        }
    }

    fn flush_sink(&mut self, force: bool) {
        let Some(state) = &self.active_sink else { return };
        let Ok(mut s) = state.lock() else { return };
        if let Some(header) = s.header.take() {
            self.host.start_set(self.set_id, &header);
        }
        while let Some((count, bytes)) = s.completed.pop_front() {
            let from = self.sent_ticks;
            self.sent_ticks += count;
            self.last_flush = Instant::now();
            self.host.send_ticks(self.set_id, from, count, &bytes);
            while self.pending_checkpoints.front().is_some_and(|cp| cp.from_tick <= self.sent_ticks) {
                let cp = self.pending_checkpoints.pop_front().expect("checked front");
                self.host.send_checkpoint(self.set_id, cp);
            }
        }
        if s.pending_ticks >= BATCH_TICKS || (force && s.pending_ticks > 0) {
            let count = s.pending_ticks;
            let bytes = std::mem::take(&mut s.pending);
            s.pending_ticks = 0;
            let from = self.sent_ticks;
            self.sent_ticks += count;
            self.last_flush = Instant::now();
            self.host.send_ticks(self.set_id, from, count, &bytes);
        }
        while self.pending_checkpoints.front().is_some_and(|cp| cp.from_tick <= self.sent_ticks) {
            let cp = self.pending_checkpoints.pop_front().expect("checked front");
            self.host.send_checkpoint(self.set_id, cp);
        }
    }

    pub fn close(&mut self) {
        self.end_set("host-left");
        self.host.close();
    }
}

// ---- Watcher session -----------------------------------------------------------

/// High-level update returned by [`WatchSession::poll`].
pub enum WatchUpdate {
    /// Still watching normally.
    Active,
    /// Room closed cleanly (host left or disabled spectating).
    Closed(String),
    /// Connection refused or timed out.
    Failed(saito_signaling::Error),
}

/// Active spectator session watching a room across lobby and multiple sets.
pub struct WatchSession {
    watcher: SpectateWatcher,
    code: String,
    room: Option<RoomSummary>,
    active_set_id: Option<u32>,
    decoder: TickDecoder,
    viewer: Option<Viewer>,
    pending_header: Option<(u32, Header, Option<RoundStart>)>,
    pending_ticks: Vec<Tick>,
    set_ended_remote: bool,
    set_end_since: Option<Instant>,
    surface: Surface,
    samples: Vec<[f32; 2]>,
    view: u8,
    both: bool,
    catching_up: bool,
    last_frame: Option<Instant>,
    problem: Option<String>,
    game: Option<String>,
    lang: String,
    lang_loaded: bool,
    enhancements: Enhancements,
}

impl WatchSession {
    pub fn connect(server: &str, code: &str, peer: &str) -> Result<WatchSession, saito_signaling::Error> {
        let watcher = SpectateWatcher::connect(server, code, peer, Duration::from_secs(20))?;
        let normalized = saito_signaling::normalize_code(code).unwrap_or_else(|| code.to_uppercase());
        Ok(WatchSession {
            watcher,
            code: normalized,
            room: None,
            active_set_id: None,
            decoder: TickDecoder::new(),
            viewer: None,
            pending_header: None,
            pending_ticks: Vec::new(),
            set_ended_remote: false,
            set_end_since: None,
            surface: Surface::new(),
            samples: Vec::new(),
            view: 0,
            both: false,
            catching_up: false,
            last_frame: None,
            problem: None,
            game: None,
            lang: String::new(),
            lang_loaded: false,
            enhancements: Enhancements::NONE,
        })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn room(&self) -> Option<&RoomSummary> {
        self.room.as_ref()
    }

    pub fn in_battle(&self) -> bool {
        self.viewer.is_some()
    }

    pub fn catching_up(&self) -> bool {
        self.catching_up
    }

    pub fn view(&self) -> u8 {
        self.view
    }

    pub fn both(&self) -> bool {
        self.both
    }

    pub fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }

    pub fn set_view(&mut self, side: u8) {
        self.view = side & 1;
        self.both = false;
        if let Some(v) = &mut self.viewer {
            v.set_both(false);
            v.set_view(self.view);
            self.surface.stale = true;
        }
    }

    pub fn set_both(&mut self, both: bool) {
        self.both = both;
        if let Some(v) = &mut self.viewer {
            v.set_both(both);
            self.surface.stale = true;
        }
    }

    pub fn mark_stale(&mut self) {
        self.surface.stale = true;
    }

    pub fn close(&mut self) {
        self.watcher.close();
        self.viewer = None;
        self.game = None;
        self.pending_header = None;
        self.pending_ticks.clear();
    }

    pub fn poll(&mut self, shell: &mut Shell, now: Instant) -> WatchUpdate {
        loop {
            match self.watcher.poll(now) {
                Ok(None) => break,
                Ok(Some(WatcherEvent::Snapshot { room, set })) => {
                    if self.viewer.is_none() && !room.meta.game.is_empty() {
                        self.ensure_game(shell, &room.meta.game);
                    }
                    self.room = Some(room);
                    if let Some(snap) = set {
                        self.begin_set_snapshot(shell, snap);
                    }
                }
                Ok(Some(WatcherEvent::RoomUpdate(room))) => {
                    if self.viewer.is_none() && !room.meta.game.is_empty() {
                        self.ensure_game(shell, &room.meta.game);
                    }
                    self.room = Some(room);
                }
                Ok(Some(WatcherEvent::Spectators(count))) => {
                    if let Some(room) = &mut self.room {
                        room.spectators = count;
                    }
                }
                Ok(Some(WatcherEvent::SetStart { set_id, header_b64 })) => {
                    self.begin_set(shell, set_id, &header_b64, None, &[]);
                }
                Ok(Some(WatcherEvent::Ticks { set_id, data_b64, .. })) => {
                    if self.active_set_id == Some(set_id) {
                        self.ingest_chunk(&data_b64);
                    }
                }
                Ok(Some(WatcherEvent::SetEnd { set_id, .. })) => {
                    if self.active_set_id == Some(set_id) {
                        self.set_ended_remote = true;
                        if self.set_end_since.is_none() {
                            self.set_end_since = Some(now);
                        }
                    }
                }
                Ok(Some(WatcherEvent::Closed(reason))) => {
                    self.close();
                    return WatchUpdate::Closed(reason);
                }
                Err(err) => {
                    self.close();
                    return WatchUpdate::Failed(err);
                }
            }
        }

        // If waiting for the match's game pack to finish loading, try opening the viewer now.
        if self.viewer.is_none()
            && let Some((set_id, header, start_round)) = self.pending_header.take()
        {
            if !self.try_open_viewer(shell, set_id, header, start_round) {
                // Still loading or failed (pending_header restored if still loading).
            }
        }

        // Advance and present the active viewer.
        self.sync_presentation(shell);
        if let Some(viewer) = &mut self.viewer {
            let (music, effects) = shell.sound.battle_levels();
            viewer.set_sound_levels(music, effects);

            let mut played = 0;
            if self.catching_up {
                let still_behind = viewer.catch_up_step(CATCHUP_TICKS_PER_FRAME, CATCHUP_MARGIN);
                if !still_behind || viewer.lag_ticks() <= CATCHUP_END_LAG {
                    self.catching_up = false;
                    self.last_frame = Some(now);
                }
                self.surface.stale = true;
            } else {
                let dt = self.last_frame.map_or(Duration::ZERO, |last| now.saturating_duration_since(last));
                self.last_frame = Some(now);
                let ran = viewer.advance_live(dt);
                if ran > 0 {
                    viewer.take_samples(&mut self.samples);
                    played = ran;
                    self.surface.stale = true;
                }
                if viewer.lag_ticks() > CATCHUP_START_LAG {
                    self.catching_up = true;
                }
            }
            if played == 0 {
                self.samples.clear();
            }
            // Always submit a frame's tick count (including 0 while catching up or on >60Hz frames
            // with no new GBA tick) so `App::run` keeps title music suppressed.
            shell.play_samples(&mut self.samples, played);

            shell.show_surface(&mut self.surface, true, |size, buffer, width, height| {
                viewer.renderer().size = size;
                viewer.present(buffer, width, height);
            });

            // After remote set end and buffer drained (+ brief pause), return to spectator lobby.
            if self.set_ended_remote && viewer.lag_ticks() == 0 {
                let since = *self.set_end_since.get_or_insert(now);
                if now.saturating_duration_since(since) >= Duration::from_secs(2) {
                    self.viewer = None;
                    self.game = None;
                    self.active_set_id = None;
                    self.set_ended_remote = false;
                    self.set_end_since = None;
                    self.catching_up = false;
                    if let Some(room) = &mut self.room {
                        room.meta.stage = RoomStage::Lobby;
                    }
                }
            }
        }

        WatchUpdate::Active
    }

    fn sync_presentation(&mut self, shell: &Shell) {
        let (Some(viewer), Some(game)) = (&mut self.viewer, self.game.as_deref()) else { return };
        let Some(ready) = shell.games.ready(game) else { return };
        let lang = shell.i18n.code;
        let lang_loaded = ready.has_language(lang);
        let enhancements = shell.settings.enhancements();
        if self.lang != lang || self.lang_loaded != lang_loaded || self.enhancements != enhancements {
            viewer.set_language(&ready.graphics(lang));
            viewer.set_enhancements(enhancements, ready.loaded.font_in(lang));
            self.lang = lang.to_string();
            self.lang_loaded = lang_loaded;
            self.enhancements = enhancements;
            self.surface.stale = true;
        }
    }

    fn begin_set_snapshot(&mut self, shell: &mut Shell, snap: SetSnapshot) {
        let chunks: Vec<&str> = snap.chunks.iter().map(|c| c.data_b64.as_str()).collect();
        self.begin_set(shell, snap.set_id, &snap.header_b64, snap.checkpoint, &chunks);
    }

    fn begin_set(&mut self, shell: &mut Shell, set_id: u32, header_b64: &str, checkpoint: Option<RoundCheckpoint>, chunks: &[&str]) {
        self.viewer = None;
        self.game = None;
        self.pending_header = None;
        self.pending_ticks.clear();
        let start_round = checkpoint.map(|cp| {
            self.decoder = TickDecoder::with_state(cp.prev_inputs, cp.prev_delays, cp.from_tick as usize);
            RoundStart {
                settings: BattleSettings {
                    stage: StageHandle(cp.stage as u16),
                    background: BackgroundId(cp.background),
                    effects: cp.effects,
                },
                score: SetScore {
                    wins: cp.wins,
                    losses: cp.losses,
                    round: cp.round,
                    max_combo: cp.max_combo,
                },
                console_rng: cp.console_rng,
                console_frames: cp.console_frames,
            }
        });
        if start_round.is_none() {
            self.decoder = TickDecoder::new();
        }
        self.active_set_id = Some(set_id);
        self.set_ended_remote = false;
        self.set_end_since = None;
        self.catching_up = false;
        self.last_frame = None;
        self.problem = None;

        let raw = match base64_decode(header_b64) {
            Ok(b) => b,
            Err(why) => {
                self.problem = Some(format!("invalid stream header: {why}"));
                return;
            }
        };
        let header = match Header::read(&raw) {
            Ok(h) => h,
            Err(why) => {
                self.problem = Some(format!("incompatible stream header: {why}"));
                return;
            }
        };
        for chunk in chunks {
            self.ingest_chunk(chunk);
        }
        self.try_open_viewer(shell, set_id, header, start_round);
    }

    fn ingest_chunk(&mut self, data_b64: &str) {
        let Ok(raw) = base64_decode(data_b64) else { return };
        let Ok(ticks) = self.decoder.decode_chunk(&raw) else { return };
        if ticks.is_empty() {
            return;
        }
        if let Some(viewer) = &mut self.viewer {
            viewer.push_ticks(&ticks);
        } else {
            self.pending_ticks.extend(ticks);
        }
    }

    fn ensure_game(&mut self, shell: &mut Shell, game: &str) -> bool {
        if shell.games.ready(game).is_some() {
            return true;
        }
        if shell.games.playable().iter().any(|g| g == game) {
            shell.select_game(game);
        } else {
            self.problem = Some(format!("game '{game}' is not installed"));
        }
        false
    }

    fn try_open_viewer(&mut self, shell: &mut Shell, set_id: u32, header: Header, start_round: Option<RoundStart>) -> bool {
        let game = header.head.game.clone();
        if !self.ensure_game(shell, &game) {
            if self.problem.is_none() {
                self.pending_header = Some((set_id, header, start_round));
            }
            return false;
        }
        let Some(ready) = shell.games.ready(&game) else { return false };
        let lang = shell.i18n.code;
        let lang_loaded = ready.has_language(lang);
        if !lang_loaded {
            shell.games.load_language(lang, |id, lang, graphics| {
                crate::with_app(move |app| {
                    app.shell.games.language(&id, &lang, graphics);
                    if app.shell.i18n.code == lang {
                        app.shell.show_language();
                    }
                })
            });
        }
        let enhancements = shell.settings.enhancements();
        let localized = Loaded {
            game: ready.loaded.game.clone(),
            graphics: ready.graphics(lang),
            enhancements,
            font: ready.loaded.font_in(lang).filter(|_| enhancements.wants_font()),
            sound: ready.loaded.sound.as_ref().map(|s| Sound {
                bank: s.bank.clone(),
                songs: s.songs.clone(),
                report: Default::default(),
            }),
        };
        let music = crate::music::round_music(&shell.games, &ready, &crate::music::Music::Background);
        match Viewer::open_live_at(&localized, header, Some(self.view), music, start_round) {
            Ok(mut viewer) => {
                viewer.set_both(self.both);
                if !self.pending_ticks.is_empty() {
                    let ticks = std::mem::take(&mut self.pending_ticks);
                    viewer.push_ticks(&ticks);
                }
                self.catching_up = viewer.lag_ticks() > CATCHUP_START_LAG;
                self.surface = Surface::new();
                self.surface.stale = true;
                self.viewer = Some(viewer);
                self.game = Some(game);
                self.lang = lang.to_string();
                self.lang_loaded = lang_loaded;
                self.enhancements = enhancements;
                self.problem = None;
                true
            }
            Err(refused) => {
                self.problem = Some(refused.to_string());
                false
            }
        }
    }
}

// ---- Public directory client ---------------------------------------------------

/// Live directory of public spectatable rooms.
#[derive(Default)]
pub struct DirectoryClient {
    dir: Option<SpectateDirectory>,
    rooms: Vec<RoomSummary>,
    loading: bool,
}

impl DirectoryClient {
    pub fn open(&mut self, server: &str) {
        self.dir = Some(SpectateDirectory::connect(server));
        self.loading = true;
    }

    pub fn close(&mut self) {
        if let Some(mut dir) = self.dir.take() {
            dir.close();
        }
        self.loading = false;
    }

    pub fn is_open(&self) -> bool {
        self.dir.is_some()
    }

    pub fn loading(&self) -> bool {
        self.loading
    }

    pub fn rooms(&self) -> &[RoomSummary] {
        &self.rooms
    }

    pub fn refresh(&mut self) {
        if let Some(dir) = &mut self.dir {
            self.loading = true;
            dir.refresh();
        }
    }

    /// Returns `true` when the room list or loading state changed.
    pub fn poll(&mut self, now: Instant) -> bool {
        let Some(dir) = &mut self.dir else { return false };
        let was_loading = self.loading;
        let mut changed = false;
        loop {
            match dir.poll(now) {
                Ok(Some(rooms)) => {
                    self.loading = false;
                    if was_loading || self.rooms != rooms {
                        self.rooms = rooms;
                        changed = true;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    crate::errors::log("spectate directory", &err);
                    self.loading = false;
                    if was_loading {
                        changed = true;
                    }
                    break;
                }
            }
        }
        changed
    }
}
