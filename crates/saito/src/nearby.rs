//! The Legacy Collection from a computer nearby (saito-nearby; docs/app.md §17, "From a computer nearby"). A desktop
//! whose games' ROMs are found offers them on the local network, and when a device asks, its player is asked in a
//! dialog (Send, Not now); Send sends the ROMs as copies, as a phone keeps them ([`RomSource::copies`]), the dialog
//! staying up with a bar till they are sent (Cancel stops the send), or saying it failed. A device whose
//! ROMs aren't found (a phone, or a desktop without the collection) looks for one, asks the first it finds, and keeps
//! the copies it is sent ([`saito_extract::source::check_copy`]: each checked by its header), from which the games
//! are played at once and at each start. The loading screen's note says how that goes ([`saito_ui::NearbyState`]).
//!
//! Neither happens in the screenshot pass, which leaves the network alone and shows the note's states and the dialog
//! itself (`tour.rs`, the part `nearby`).

use crate::shell::{Dialog, Shell, button};
use saito_extract::RomSource;
use saito_nearby::{Event, File, Peer, Request, Seeker};
use saito_ui::{ButtonKind, NearbyState, NearbyWords};
use slint::ComponentHandle;
use std::collections::{HashSet, VecDeque};
use std::net::SocketAddr;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use web_time::{Duration, Instant};

/// The dialog's tag: the shell's own, answered by [`answered`].
pub const DIALOG: &str = "nearby";

/// How long the note says it is looking before it says what to check.
const LOOKING: Duration = Duration::from_secs(8);
/// How long before a computer that couldn't be reached, or was sending to another device, is asked again.
const ASK_AGAIN: Duration = Duration::from_secs(5);
/// How often looking is started again where the system stopped it (iOS: the local network not allowed yet).
const SEEK_AGAIN: Duration = Duration::from_secs(10);

pub struct Nearby {
    link: saito_nearby::Nearby,
    offering: bool,
    seeking: Option<Seeking>,
    /// Seeking devices found while offering (used for outbound reverse connections when firewalls drop inbound TCP).
    seekers: Vec<(Seeker, String)>,
    /// Seeking devices already attempted in this discovery cycle (cleared if lost).
    passed_seekers: HashSet<Seeker>,
    /// Manual `IP:port` target initiated from Settings, so a connection failure can notify the player.
    manual_reach: Option<String>,
    /// The asks heard, waiting for the player to be asked: one at a time, when no dialog nor battle is up.
    asks: VecDeque<(Request, String)>,
    /// The ask put to the player, from its dialog's opening till the send's end.
    asked: Option<Asked>,
}

/// An ask put to the player: the device's, by its name, and where it is at.
struct Asked {
    request: Request,
    name: String,
    step: Step,
}

enum Step {
    /// The dialog asks: Send, Not now.
    Asking,
    /// Send was chosen: the copies are made on a thread (`made`, till they come), then sent, the dialog's bar showing
    /// how far, with Cancel.
    Sending { made: Option<Receiver<Option<Vec<File>>>> },
    /// The send failed: the dialog says so, with OK.
    Failed,
}

/// Looking for a computer to send the collection, and what has come of it.
struct Seeking {
    since: Instant,
    /// When looking was last started (again).
    started: Instant,
    /// The computers found, in the order they were.
    found: Vec<(Peer, String)>,
    /// Those not asked again till found again: they said no, have another version, or sent what can't be played.
    passed: HashSet<Peer>,
    /// The one asked, while its answer or its files come, or while what it sent is checked.
    asking: Option<Peer>,
    /// True while receiving or awaiting an answer over a reverse connection from an offerer.
    connected: bool,
    /// When the next ask may be made.
    wait: Option<Instant>,
    /// What the note says: the state, the computer's name, our local seek address, the transfer's progress.
    state: NearbyState,
    name: String,
    address: String,
    progress: f32,
    /// The copies received being checked and kept, on a thread: how many were.
    keeping: Option<Receiver<Result<usize, String>>>,
}

impl Default for Nearby {
    fn default() -> Nearby {
        Nearby {
            link: saito_nearby::Nearby::new(),
            offering: false,
            seeking: None,
            seekers: Vec::new(),
            passed_seekers: HashSet::new(),
            manual_reach: None,
            asks: VecDeque::new(),
            asked: None,
        }
    }
}

/// Manually connect outbound to `text` (`IP:port`) when offering, as fallback when multicast discovery is blocked.
pub fn send_to(shell: &mut Shell, text: &str) -> bool {
    let Ok(addr) = text.trim().parse::<SocketAddr>() else { return false };
    if !shell.nearby.offering {
        return false;
    }
    let target = addr.to_string();
    shell.nearby.manual_reach = Some(target.clone());
    shell.nearby.link.reach_at(target, vec![addr]);
    true
}

/// A frame of the app's: offer while the games' ROMs are found, look while none are, hear what happened, and say it.
/// (Not in the screenshot pass, which shows the note's states itself: `tour.rs`.)
pub fn frame(shell: &mut Shell, now: Instant) {
    if shell.still {
        return;
    }
    let offer = saito_nearby::can_offer() && shell.games.rom_source().is_some();
    let seek = saito_nearby::can_seek() && shell.games.without_roms();
    let nearby = &mut shell.nearby;
    if offer != nearby.offering {
        nearby.offering = offer;
        nearby.link.offer(offer);
        if offer {
            crate::errors::log("the Legacy Collection nearby", &format_args!("offered as {}", saito_nearby::device_name()));
        } else {
            nearby.seekers.clear();
            nearby.passed_seekers.clear();
            nearby.manual_reach = None;
        }
    }
    match (&mut nearby.seeking, seek) {
        (None, true) => {
            crate::errors::log("the Legacy Collection nearby", &"looking for a computer that has it");
            nearby.link.seek(true);
            let address = nearby.link.seek_address().map(|a| a.to_string()).unwrap_or_default();
            nearby.seeking = Some(Seeking::new(now, address));
        }
        (Some(_), false) => {
            nearby.link.seek(false);
            nearby.seeking = None;
        }
        (Some(seeking), true) if now.saturating_duration_since(seeking.started) >= SEEK_AGAIN => {
            // (A no-op while it looks; else it starts again.)
            nearby.link.seek(true);
            seeking.started = now;
            seeking.address = nearby.link.seek_address().map(|a| a.to_string()).unwrap_or_default();
        }
        _ => {}
    }
    for event in nearby.link.poll() {
        hear(shell, event, now);
    }
    keep_received(shell);
    send_made(shell);
    let_go(shell);
    if let Some(seeking) = &mut shell.nearby.seeking {
        seeking.ask_next(&mut shell.nearby.link, now);
    }
    reach_seeker(shell);
    ask_player(shell);
    show(shell);
}

fn reach_seeker(shell: &mut Shell) {
    let busy_ui = shell.nearby.asked.is_some() || !shell.nearby.asks.is_empty() || shell.dialog.is_some() || shell.battle.is_some() || shell.loading_battle();
    if !shell.nearby.offering || busy_ui {
        return;
    }
    let Some(seeker) = shell.nearby.seekers.iter().find_map(|(s, _)| (!shell.nearby.passed_seekers.contains(s)).then_some(*s)) else {
        return;
    };
    shell.nearby.passed_seekers.insert(seeker);
    shell.nearby.link.reach(seeker);
}

/// What happened, for the side it is of.
fn hear(shell: &mut Shell, event: Event, now: Instant) {
    let nearby = &mut shell.nearby;
    match event {
        Event::SeekerFound { seeker, name } => {
            if !nearby.seekers.iter().any(|(s, _)| *s == seeker) {
                nearby.seekers.push((seeker, name));
            }
        }
        Event::SeekerLost { seeker } => {
            nearby.seekers.retain(|(s, _)| *s != seeker);
            nearby.passed_seekers.remove(&seeker);
        }
        Event::Asked { request, name } => {
            nearby.manual_reach = None;
            nearby.asks.push_back((request, name));
        }
        Event::Sending { request, done, total } => {
            if nearby.asked.as_ref().is_some_and(|a| a.request == request) {
                shell.dialog_progress(DIALOG, Some(done as f32 / total.max(1) as f32));
            }
        }
        Event::Sent { request, name } => {
            crate::errors::log("sending the Legacy Collection", &format_args!("sent to {name}"));
            if nearby.asked.as_ref().is_some_and(|a| a.request == request) {
                nearby.asked = None;
                if shell.dialog_is(DIALOG) {
                    shell.close_dialog();
                }
            }
        }
        Event::NotSent { request, name, why } => {
            crate::errors::log("sending the Legacy Collection", &format_args!("{name}: {why}"));
            if let Some(asked) = nearby.asked.as_mut().filter(|a| a.request == request) {
                asked.step = Step::Failed;
                let name = asked.name.clone();
                show_failed(shell, &name);
            }
        }
        Event::Failed { name, why } if nearby.seeking.as_ref().is_none_or(|s| !s.is_asking()) || name.is_empty() => {
            crate::errors::log("the Legacy Collection nearby", &format_args!("{name}: {why}"));
            let manual_failed = nearby.manual_reach.as_deref() == Some(name.as_str());
            if manual_failed {
                nearby.manual_reach = None;
            }
            if manual_failed {
                open_failed(shell, &name);
            }
        }
        Event::OtherVersion { name, version } if nearby.seeking.is_none() => {
            crate::errors::log("sending the Legacy Collection", &format_args!("{name} asked with version {version} of the transfer's"));
        }
        event => {
            if let Some(seeking) = &mut nearby.seeking {
                seeking.hear(event, &shell.games, now);
            }
        }
    }
}

impl Seeking {
    fn new(now: Instant, address: String) -> Seeking {
        Seeking {
            since: now,
            started: now,
            found: Vec::new(),
            passed: HashSet::new(),
            asking: None,
            connected: false,
            wait: None,
            state: NearbyState::Looking,
            name: String::new(),
            address,
            progress: 0.0,
            keeping: None,
        }
    }

    fn is_asking(&self) -> bool {
        self.asking.is_some() || self.connected
    }

    fn say(&mut self, state: NearbyState, name: String) {
        self.state = state;
        self.name = name;
    }

    /// What happened, while looking.
    fn hear(&mut self, event: Event, games: &crate::games::Games, now: Instant) {
        match event {
            Event::Found { peer, name } => self.found.push((peer, name)),
            Event::Lost { peer } => {
                self.found.retain(|(p, _)| *p != peer);
                self.passed.remove(&peer);
            }
            Event::Connected { name } => {
                self.asking = None;
                self.connected = true;
                self.wait = None;
                self.progress = 0.0;
                self.say(NearbyState::Asking, name);
            }
            Event::Receiving { done, total } if self.is_asking() => {
                self.state = NearbyState::Receiving;
                self.progress = done as f32 / total.max(1) as f32;
            }
            Event::Received { name, files } => {
                crate::errors::log("receiving the Legacy Collection", &format_args!("{} files from {name}; checking them", files.len()));
                self.say(NearbyState::Receiving, name);
                self.progress = 1.0;
                self.keeping = Some(keep(files, games.rom_codes()));
            }
            Event::Declined { name } => {
                self.passed.extend(self.asking.take());
                self.connected = false;
                self.say(NearbyState::Declined, name);
            }
            Event::Busy { name } => {
                self.asking = None;
                self.connected = false;
                self.wait = Some(now + ASK_AGAIN);
                self.say(NearbyState::Busy, name);
            }
            Event::OtherVersion { name, version } => {
                crate::errors::log("receiving the Legacy Collection", &format_args!("{name} has version {version} of the transfer's"));
                self.passed.extend(self.asking.take());
                self.connected = false;
                self.say(NearbyState::OtherVersion, name);
            }
            Event::Failed { name, why } => {
                crate::errors::log("receiving the Legacy Collection", &format_args!("{name}: {why}"));
                // (Asked again after the others found, which are asked at once: one long gone may be found till the
                // system's cache lets it go.)
                let failed_peer = self.asking.take();
                self.connected = false;
                if let Some(peer) = failed_peer
                    && let Some(at) = self.found.iter().position(|(p, _)| *p == peer)
                {
                    let failed = self.found.remove(at);
                    self.found.push(failed);
                }
                let others = self.found.iter().any(|(p, _)| Some(*p) != failed_peer && !self.passed.contains(p));
                self.wait = (!others).then(|| now + ASK_AGAIN);
                self.say(NearbyState::Unreachable, name);
            }
            Event::NotAllowed => {
                crate::errors::log("looking for a computer nearby", &"the local network isn't allowed");
                self.say(NearbyState::NotAllowed, String::new());
            }
            Event::SeekerFound { .. } | Event::SeekerLost { .. } | Event::Asked { .. } | Event::Receiving { .. } | Event::Sending { .. } | Event::Sent { .. } | Event::NotSent { .. } => {}
        }
    }

    /// Ask the first computer found that isn't passed over, unless one is being asked or it is too soon; with none to
    /// ask, the note says it is looking, or what to check, unless it says why the last one asked didn't send.
    fn ask_next(&mut self, link: &mut saito_nearby::Nearby, now: Instant) {
        if self.is_asking() || self.keeping.is_some() || self.wait.is_some_and(|w| now < w) {
            return;
        }
        match self.found.iter().find(|(p, _)| !self.passed.contains(p)).cloned() {
            Some((peer, name)) => {
                link.ask(peer);
                self.asking = Some(peer);
                self.progress = 0.0;
                self.say(NearbyState::Asking, name);
            }
            None if matches!(self.state, NearbyState::Declined | NearbyState::OtherVersion | NearbyState::Unusable | NearbyState::NotAllowed) => {}
            None => {
                let state = if now.saturating_duration_since(self.since) < LOOKING { NearbyState::Looking } else { NearbyState::NotFound };
                self.say(state, String::new());
            }
        }
    }
}

/// Check and keep the copies received, on a thread: how many were kept.
fn keep(files: Vec<File>, codes: Vec<String>) -> Receiver<Result<usize, String>> {
    let (tell, told) = mpsc::channel();
    // (A thread that can't start drops its sender: heard as the check's failure.)
    let _ = saito_threads::spawn("keeping the Legacy Collection's ROMs", move || {
        let dir = crate::games::copies();
        let mut kept = 0;
        for file in files {
            let checked = saito_extract::source::check_copy(&file.name, &file.bytes, &codes).map_err(|e| e.to_string());
            match checked.and_then(|_| saito_files::stow(&dir.join(&file.name), file.bytes).map_err(|e| e.to_string())) {
                Ok(()) => kept += 1,
                Err(why) => crate::errors::log("keeping a ROM received", &format_args!("{}: {why}", file.name)),
            }
        }
        let _ = tell.send(if kept > 0 { Ok(kept) } else { Err("none of the files received is a ROM a game wants".into()) });
    });
    told
}

/// The copies received, once checked and kept: the games are played from them, where they make a game whole.
fn keep_received(shell: &mut Shell) {
    let Some(seeking) = &mut shell.nearby.seeking else { return };
    let kept = match seeking.keeping.as_ref().map(Receiver::try_recv) {
        Some(Ok(kept)) => kept,
        Some(Err(TryRecvError::Disconnected)) => Err("the thread checking them didn't start".to_string()),
        _ => return,
    };
    seeking.keeping = None;
    let peer = seeking.asking.take();
    seeking.connected = false;
    let playable = match kept {
        Ok(kept) => {
            crate::errors::log("receiving the Legacy Collection", &format_args!("{kept} ROMs kept from {}", seeking.name));
            shell.games.found(RomSource::Copies(crate::games::copies()))
        }
        Err(why) => {
            crate::errors::log("receiving the Legacy Collection", &why);
            0
        }
    };
    if playable > 0 {
        // (Done: no other is asked, this frame or the next.)
        shell.nearby.link.seek(false);
        shell.nearby.seeking = None;
        return;
    }
    let seeking = shell.nearby.seeking.as_mut().expect("seeking");
    seeking.passed.extend(peer);
    seeking.state = NearbyState::Unusable;
}

/// The copies made for the ask the player said Send to: sent; none (they couldn't be made): declined, and the dialog
/// says it failed.
fn send_made(shell: &mut Shell) {
    let Some(asked) = &mut shell.nearby.asked else { return };
    let Step::Sending { made: Some(made) } = &asked.step else { return };
    let files = match made.try_recv() {
        Ok(files) => files,
        Err(TryRecvError::Disconnected) => None,
        Err(TryRecvError::Empty) => return,
    };
    let failed = files.is_none();
    shell.nearby.link.answer(asked.request, files);
    asked.step = if failed { Step::Failed } else { Step::Sending { made: None } };
    if failed {
        let name = asked.name.clone();
        show_failed(shell, &name);
    }
}

/// The ask's dialog closed by something else (another dialog over it): not asked, it is declined; being sent, the
/// send goes on, unseen.
fn let_go(shell: &mut Shell) {
    let Some(asked) = &shell.nearby.asked else { return };
    if shell.dialog_is(DIALOG) {
        return;
    }
    match asked.step {
        Step::Asking => {
            shell.nearby.link.answer(asked.request, None);
            shell.nearby.asked = None;
        }
        Step::Sending { .. } => {}
        Step::Failed => shell.nearby.asked = None,
    }
}

/// The next ask, put to the player in the shell's dialog when nothing else is up.
fn ask_player(shell: &mut Shell) {
    if shell.nearby.asked.is_some() || shell.dialog.is_some() || shell.battle.is_some() || shell.loading_battle() {
        return;
    }
    let Some((request, name)) = shell.nearby.asks.pop_front() else { return };
    open_ask(shell, &name);
    shell.nearby.asked = Some(Asked { request, name, step: Step::Asking });
}

/// The dialog that asks the player whether to send the collection to the device `name` (Send, Not now). (The
/// screenshot pass's opens it with no ask behind it: its answer goes nowhere.)
pub(crate) fn open_ask(shell: &mut Shell, name: &str) {
    let ui = shell.ui();
    let words = ui.global::<NearbyWords>();
    let (title, message) = (words.invoke_ask_title(name.into()).to_string(), words.invoke_ask_message(name.into()).to_string());
    let buttons = vec![button(&words.get_send(), ButtonKind::Main), button(&words.get_not_now(), ButtonKind::Normal)];
    shell.open_dialog(Dialog { tag: DIALOG, title, message, buttons, focus: 0, cancel: 1 }, None, false);
}

/// The dialog, once Send is chosen: sending to the device `name`, its bar at `progress`, with Cancel.
pub(crate) fn show_sending(shell: &mut Shell, name: &str, progress: f32) {
    let ui = shell.ui();
    let words = ui.global::<NearbyWords>();
    let (title, message) = (words.invoke_sending_title(name.into()).to_string(), words.get_sending_message().to_string());
    let buttons = vec![button(&words.get_cancel(), ButtonKind::Normal)];
    shell.change_dialog(Dialog { tag: DIALOG, title, message, buttons, focus: 0, cancel: 0 });
    shell.dialog_progress(DIALOG, Some(progress));
}

/// The dialog, once the send to the device `name` failed, with OK (if it is up).
fn show_failed(shell: &mut Shell, name: &str) {
    if !shell.dialog_is(DIALOG) {
        return;
    }
    let ui = shell.ui();
    let words = ui.global::<NearbyWords>();
    let (title, message) = (words.invoke_failed_title(name.into()).to_string(), words.get_failed_message().to_string());
    let ok = ui.global::<saito_ui::Errors>().invoke_ok();
    shell.change_dialog(Dialog { tag: DIALOG, title, message, buttons: vec![button(&ok, ButtonKind::Main)], focus: 0, cancel: 0 });
    shell.dialog_progress(DIALOG, None);
    shell.sound.play(crate::sound::Cue::Refused);
}

/// Open the failure dialog when a manual `IP:port` reach fails before opening a request dialog.
fn open_failed(shell: &mut Shell, name: &str) {
    if shell.dialog.is_some() || shell.battle.is_some() || shell.loading_battle() {
        return;
    }
    let ui = shell.ui();
    let words = ui.global::<NearbyWords>();
    let (title, message) = (words.invoke_failed_title(name.into()).to_string(), words.get_failed_message().to_string());
    let ok = ui.global::<saito_ui::Errors>().invoke_ok();
    shell.open_dialog(Dialog { tag: DIALOG, title, message, buttons: vec![button(&ok, ButtonKind::Main)], focus: 0, cancel: 0 }, None, false);
    shell.sound.play(crate::sound::Cue::Refused);
}

/// A button of the dialog: Send (it stays up, the copies made on a thread, then sent) or Not now; Cancel, which stops
/// the send; OK, once it failed. All but Send close it.
pub fn pressed(shell: &mut Shell, choice: usize) {
    let Some(asked) = shell.nearby.asked.take() else {
        shell.close_dialog();
        return;
    };
    match asked.step {
        Step::Asking if choice == 0 => {
            if let Some(source) = shell.games.rom_source().cloned() {
                let made = make_copies(source, shell.games.rom_codes(), asked.name.clone());
                show_sending(shell, &asked.name, 0.0);
                shell.nearby.asked = Some(Asked { step: Step::Sending { made: Some(made) }, ..asked });
                return;
            }
            // (The ROMs are gone since: declined.)
            shell.nearby.link.answer(asked.request, None);
        }
        Step::Asking => shell.nearby.link.answer(asked.request, None),
        Step::Sending { .. } => {
            crate::errors::log("sending the Legacy Collection", &format_args!("canceled for {}", asked.name));
            shell.nearby.link.stop(asked.request);
        }
        Step::Failed => {}
    }
    shell.close_dialog();
}

/// The copies of the ROMs `codes` from `source`, made on a thread for the device `name`: none where they couldn't be.
fn make_copies(source: RomSource, codes: Vec<String>, name: String) -> Receiver<Option<Vec<File>>> {
    let (tell, told) = mpsc::channel();
    // (A thread that can't start drops its sender: heard as no copies.)
    let _ = saito_threads::spawn("copying the Legacy Collection's ROMs", move || {
        let files = match source.copies(&codes) {
            Ok(copies) => Some(copies.into_iter().map(|(name, bytes)| File { name, bytes }).collect()),
            Err(why) => {
                crate::errors::log(&format!("copying the Legacy Collection's ROMs for {name}"), &why);
                None
            }
        };
        let _ = tell.send(files);
    });
    told
}

/// The loading screen's note, as it is now.
fn show(shell: &Shell) {
    let ui = shell.ui();
    let global = ui.global::<saito_ui::Shell>();
    let (state, name, address, progress) = match &shell.nearby.seeking {
        Some(s) => (s.state, s.name.as_str(), s.address.as_str(), s.progress),
        None => (NearbyState::None, "", "", 0.0),
    };
    if global.get_nearby() != state {
        global.set_nearby(state);
    }
    if global.get_nearby_name() != name {
        global.set_nearby_name(name.into());
    }
    if global.get_nearby_address() != address {
        global.set_nearby_address(address.into());
    }
    global.set_nearby_progress(progress);
}
