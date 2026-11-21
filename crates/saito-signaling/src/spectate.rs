//! Spectator relay protocol and clients over the signaling server's WebSocket.
//!
//! A room's host publishes confirmed (settled) replay headers and tick batches
//! to `/spectate/host/<CODE>?peer=<ID>&v=1`. Watchers connect to
//! `/spectate/watch/<CODE>?peer=<ID>&v=1` to receive the room's live state,
//! mid-match catch-up chunks, and subsequent ticks (held on the server for the
//! room's anti-ghosting delay). Anyone browsing live matches connects to
//! `/spectate/list?v=1` to receive the public room directory.
//!
//! All existing `/rooms` and `/reconnect` endpoints remain untouched.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use web_time::{Duration, Instant};

use crate::platform::system::{Socket, WsEvent};
use crate::protocol::{self as signal, PING, PONG, Refusal, VERSION};
use crate::room::{Error, ErrorKind};

/// Keepalive interval for spectator sockets.
const KEEP_ALIVE: Duration = Duration::from_secs(30);
/// Maximum reconnect backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// Who may spectate a room.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Visibility {
    /// Spectators are not allowed.
    Off,
    /// Spectators with the room code may join; hidden from the public list.
    #[default]
    CodeOnly,
    /// Spectators may join by code or from the public live matches list.
    Public,
}

/// High-level phase of a room, shown to spectators and in the public list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoomStage {
    /// Waiting in the lobby (waiting for opponent or between sets).
    #[default]
    Lobby,
    /// Both players agreed on a match and are loading it.
    Loading,
    /// A set is actively being played.
    Playing,
}

/// Host-published metadata describing a spectatable room.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomMeta {
    #[serde(default)]
    pub visibility: Visibility,
    /// Server-enforced anti-ghosting delay in seconds (e.g. 0, 5, 10, 15).
    #[serde(default)]
    pub delay_secs: u16,
    /// Game pack id (e.g. `"exe6"`).
    #[serde(default)]
    pub game: String,
    /// Engine version (e.g. `"0.1.2"`).
    #[serde(default)]
    pub engine: String,
    /// Content hash encoded as 16-char lowercase hex (avoids JS number precision loss).
    #[serde(default)]
    pub content_hash: String,
    /// Ruleset label for display.
    #[serde(default)]
    pub rules: String,
    /// Player names by side `[P1, P2]`.
    #[serde(default)]
    pub names: [String; 2],
    /// Current phase of the room.
    #[serde(default)]
    pub stage: RoomStage,
    /// Current set/round score `[P1, P2]`.
    #[serde(default)]
    pub score: [u8; 2],
}

/// Public or watcher-facing summary of a spectatable room.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomSummary {
    pub code: String,
    #[serde(flatten)]
    pub meta: RoomMeta,
    #[serde(default)]
    pub spectators: u32,
}

/// One batch of `saito-replay` tick records encoded as base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickChunk {
    pub from_tick: u32,
    pub count: u32,
    pub data_b64: String,
}

/// Active set state delivered to a watcher joining mid-set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSnapshot {
    pub set_id: u32,
    /// Base64-encoded `saito-replay` header (`STRP`..`STCK`).
    pub header_b64: String,
    #[serde(default)]
    pub chunks: Vec<TickChunk>,
}

/// Messages sent from the room host to the server on `/spectate/host/<CODE>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostToServer {
    Config {
        meta: RoomMeta,
    },
    SetStart {
        set_id: u32,
        header_b64: String,
    },
    Ticks {
        set_id: u32,
        from_tick: u32,
        count: u32,
        data_b64: String,
    },
    SetEnd {
        set_id: u32,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        reason: String,
    },
    Close,
}

/// Messages sent from the server to the room host on `/spectate/host/<CODE>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToHost {
    Welcome {
        code: String,
        #[serde(default)]
        spectators: u32,
    },
    Spectators {
        count: u32,
    },
    Error {
        #[serde(default)]
        reason: Refusal,
        message: String,
    },
}

/// Messages sent from the server to a watcher on `/spectate/watch/<CODE>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToWatcher {
    Snapshot {
        room: RoomSummary,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        set: Option<SetSnapshot>,
    },
    RoomUpdate {
        room: RoomSummary,
    },
    Spectators {
        count: u32,
    },
    SetStart {
        set_id: u32,
        header_b64: String,
    },
    Ticks {
        set_id: u32,
        from_tick: u32,
        count: u32,
        data_b64: String,
    },
    SetEnd {
        set_id: u32,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        reason: String,
    },
    Closed {
        #[serde(default)]
        reason: String,
    },
    Error {
        #[serde(default)]
        reason: Refusal,
        message: String,
    },
}

/// Messages sent from the server to a directory subscriber on `/spectate/list`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToList {
    Rooms {
        #[serde(default)]
        rooms: Vec<RoomSummary>,
    },
    Error {
        #[serde(default)]
        reason: Refusal,
        message: String,
    },
}

/// WebSocket URL for the room host's spectator publish channel.
pub fn host_url(server: &str, code: &str, peer: &str) -> String {
    format!("{}/spectate/host/{code}?peer={peer}&v={VERSION}", signal::websocket(server))
}

/// WebSocket URL for a spectator watching `code`.
pub fn watch_url(server: &str, code: &str, peer: &str) -> String {
    format!("{}/spectate/watch/{code}?peer={peer}&v={VERSION}", signal::websocket(server))
}

/// WebSocket URL for subscribing to the public spectatable room list.
pub fn list_url(server: &str) -> String {
    format!("{}/spectate/list?v={VERSION}", signal::websocket(server))
}

// ---- Host client ---------------------------------------------------------------

/// Events produced by [`SpectateHost::poll`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostEvent {
    /// Connected (or reconnected) to the server with current spectator count.
    Connected { spectators: u32 },
    /// Spectator count changed.
    Spectators(u32),
    /// Server refused this host channel (permanent; host stops retrying).
    Refused { reason: Refusal, message: String },
}

/// Host-side spectator publisher. Reconnects automatically on transient drops
/// and re-publishes its latest `RoomMeta` when back online.
pub struct SpectateHost {
    url: String,
    socket: Option<Socket>,
    open: bool,
    stopped: bool,
    spectators: u32,
    last_meta: Option<RoomMeta>,
    outbox: VecDeque<String>,
    retry: Option<Instant>,
    backoff: Duration,
    pinged: Instant,
}

impl SpectateHost {
    pub fn connect(server: &str, code: &str, peer: &str) -> SpectateHost {
        let now = Instant::now();
        let url = host_url(server, code, peer);
        let socket = Some(Socket::connect(&url));
        SpectateHost {
            url,
            socket,
            open: false,
            stopped: false,
            spectators: 0,
            last_meta: None,
            outbox: VecDeque::new(),
            retry: None,
            backoff: Duration::from_millis(500),
            pinged: now,
        }
    }

    pub fn spectators(&self) -> u32 {
        self.spectators
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn update_meta(&mut self, meta: RoomMeta) {
        self.last_meta = Some(meta.clone());
        self.send_msg(&HostToServer::Config { meta });
    }

    pub fn start_set(&mut self, set_id: u32, header_bytes: &[u8]) {
        self.send_msg(&HostToServer::SetStart { set_id, header_b64: base64_encode(header_bytes) });
    }

    pub fn send_ticks(&mut self, set_id: u32, from_tick: u32, count: u32, tick_bytes: &[u8]) {
        self.send_msg(&HostToServer::Ticks { set_id, from_tick, count, data_b64: base64_encode(tick_bytes) });
    }

    pub fn end_set(&mut self, set_id: u32, reason: impl Into<String>) {
        self.send_msg(&HostToServer::SetEnd { set_id, reason: reason.into() });
    }

    /// Gracefully close the spectate room (notifies all watchers).
    pub fn close(&mut self) {
        if self.stopped {
            return;
        }
        self.stopped = true;
        if let Some(s) = &mut self.socket {
            s.send(serde_json::to_string(&HostToServer::Close).expect("JSON"));
            s.close();
        }
        self.socket = None;
        self.open = false;
        self.retry = None;
        self.outbox.clear();
    }

    fn send_msg(&mut self, msg: &HostToServer) {
        if self.stopped {
            return;
        }
        let text = serde_json::to_string(msg).expect("JSON");
        match &mut self.socket {
            Some(s) if self.open => s.send(text),
            _ => {
                // Cap queued messages while disconnected so memory stays bounded.
                if self.outbox.len() >= 256 {
                    self.outbox.pop_front();
                }
                self.outbox.push_back(text);
            }
        }
    }

    pub fn poll(&mut self, now: Instant) -> Option<HostEvent> {
        if self.stopped {
            return None;
        }
        if self.socket.is_none() {
            match self.retry {
                Some(at) if now >= at => {
                    self.socket = Some(Socket::connect(&self.url));
                    self.retry = None;
                }
                _ => return None,
            }
        }
        let socket = self.socket.as_mut().expect("socket");
        if self.open && now.duration_since(self.pinged) >= KEEP_ALIVE {
            socket.send(PING.into());
            self.pinged = now;
        }
        loop {
            let event = socket.poll()?;
            match event {
                WsEvent::Open => {
                    self.open = true;
                    self.backoff = Duration::from_millis(500);
                    self.pinged = now;
                    if let Some(meta) = &self.last_meta {
                        socket.send(serde_json::to_string(&HostToServer::Config { meta: meta.clone() }).expect("JSON"));
                    }
                    for text in self.outbox.drain(..) {
                        socket.send(text);
                    }
                }
                WsEvent::Text(text) if text == PONG => {}
                WsEvent::Text(text) => match serde_json::from_str::<ServerToHost>(&text) {
                    Ok(ServerToHost::Welcome { spectators, .. }) => {
                        self.spectators = spectators;
                        return Some(HostEvent::Connected { spectators });
                    }
                    Ok(ServerToHost::Spectators { count }) => {
                        self.spectators = count;
                        return Some(HostEvent::Spectators(count));
                    }
                    Ok(ServerToHost::Error { reason, message }) => {
                        self.stopped = true;
                        self.open = false;
                        self.socket = None;
                        return Some(HostEvent::Refused { reason, message });
                    }
                    Err(_) => {}
                },
                WsEvent::Closed(_) => {
                    self.open = false;
                    self.socket = None;
                    self.retry = Some(now + self.backoff);
                    self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
                    return None;
                }
            }
        }
    }
}

impl Drop for SpectateHost {
    fn drop(&mut self) {
        self.close();
    }
}

// ---- Watcher client ------------------------------------------------------------

/// Events produced by [`SpectateWatcher::poll`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatcherEvent {
    Snapshot {
        room: RoomSummary,
        set: Option<SetSnapshot>,
    },
    RoomUpdate(RoomSummary),
    Spectators(u32),
    SetStart {
        set_id: u32,
        header_b64: String,
    },
    Ticks {
        set_id: u32,
        from_tick: u32,
        count: u32,
        data_b64: String,
    },
    SetEnd {
        set_id: u32,
        reason: String,
    },
    /// Room ended cleanly (host left or disabled spectating).
    Closed(String),
}

/// Spectator-side client watching a single room.
pub struct SpectateWatcher {
    url: String,
    socket: Option<Socket>,
    open: bool,
    welcomed: bool,
    began: Instant,
    server_timeout: Duration,
    retry: Option<Instant>,
    backoff: Duration,
    pinged: Instant,
    last_error: Option<String>,
}

impl SpectateWatcher {
    pub fn connect(server: &str, code: &str, peer: &str, server_timeout: Duration) -> Result<SpectateWatcher, Error> {
        let code = signal::normalize_code(code).ok_or_else(|| Error {
            kind: ErrorKind::Refused(Refusal::NotFound),
            detail: format!("{code:?} isn't a room's code: {} of {}", signal::CODE_LENGTH, signal::ALPHABET),
        })?;
        let now = Instant::now();
        let url = watch_url(server, &code, peer);
        let socket = Some(Socket::connect(&url));
        Ok(SpectateWatcher {
            url,
            socket,
            open: false,
            welcomed: false,
            began: now,
            server_timeout,
            retry: None,
            backoff: Duration::from_millis(500),
            pinged: now,
            last_error: None,
        })
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        if let Some(s) = &mut self.socket {
            s.close();
        }
        self.socket = None;
        self.open = false;
        self.retry = None;
    }

    pub fn poll(&mut self, now: Instant) -> Result<Option<WatcherEvent>, Error> {
        if !self.welcomed && now.duration_since(self.began) >= self.server_timeout {
            self.close();
            let mut why = format!("no answer from the signaling server in {} seconds", self.server_timeout.as_secs());
            if let Some(last) = &self.last_error {
                why = format!("{why} (the last try: {last})");
            }
            return Err(Error { kind: ErrorKind::Unreachable, detail: why });
        }
        if self.socket.is_none() {
            match self.retry {
                Some(at) if now >= at => {
                    self.socket = Some(Socket::connect(&self.url));
                    self.retry = None;
                }
                _ => return Ok(None),
            }
        }
        let socket = self.socket.as_mut().expect("socket");
        if self.open && now.duration_since(self.pinged) >= KEEP_ALIVE {
            socket.send(PING.into());
            self.pinged = now;
        }
        loop {
            let Some(event) = socket.poll() else { return Ok(None) };
            match event {
                WsEvent::Open => {
                    self.open = true;
                    self.backoff = Duration::from_millis(500);
                    self.pinged = now;
                }
                WsEvent::Text(text) if text == PONG => {}
                WsEvent::Text(text) => match serde_json::from_str::<ServerToWatcher>(&text) {
                    Ok(ServerToWatcher::Snapshot { room, set }) => {
                        self.welcomed = true;
                        return Ok(Some(WatcherEvent::Snapshot { room, set }));
                    }
                    Ok(ServerToWatcher::RoomUpdate { room }) => {
                        return Ok(Some(WatcherEvent::RoomUpdate(room)));
                    }
                    Ok(ServerToWatcher::Spectators { count }) => {
                        return Ok(Some(WatcherEvent::Spectators(count)));
                    }
                    Ok(ServerToWatcher::SetStart { set_id, header_b64 }) => {
                        return Ok(Some(WatcherEvent::SetStart { set_id, header_b64 }));
                    }
                    Ok(ServerToWatcher::Ticks { set_id, from_tick, count, data_b64 }) => {
                        return Ok(Some(WatcherEvent::Ticks { set_id, from_tick, count, data_b64 }));
                    }
                    Ok(ServerToWatcher::SetEnd { set_id, reason }) => {
                        return Ok(Some(WatcherEvent::SetEnd { set_id, reason }));
                    }
                    Ok(ServerToWatcher::Closed { reason }) => {
                        self.close();
                        return Ok(Some(WatcherEvent::Closed(reason)));
                    }
                    Ok(ServerToWatcher::Error { reason, message }) => {
                        self.close();
                        return Err(Error { kind: ErrorKind::Refused(reason), detail: format!("the signaling server refused: {message}") });
                    }
                    Err(_) => {}
                },
                WsEvent::Closed(why) => {
                    self.open = false;
                    self.socket = None;
                    self.last_error = Some(why);
                    self.retry = Some(now + self.backoff);
                    self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
                    return Ok(None);
                }
            }
        }
    }
}

impl Drop for SpectateWatcher {
    fn drop(&mut self) {
        self.close();
    }
}

// ---- Public directory client ---------------------------------------------------

/// Subscribes to `/spectate/list` to receive the list of public spectatable rooms.
pub struct SpectateDirectory {
    url: String,
    socket: Option<Socket>,
    open: bool,
    retry: Option<Instant>,
    backoff: Duration,
    pinged: Instant,
    last_error: Option<String>,
}

impl SpectateDirectory {
    pub fn connect(server: &str) -> SpectateDirectory {
        let now = Instant::now();
        let url = list_url(server);
        let socket = Some(Socket::connect(&url));
        SpectateDirectory {
            url,
            socket,
            open: false,
            retry: None,
            backoff: Duration::from_millis(500),
            pinged: now,
            last_error: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    /// Ask the server to resend the current public room list.
    pub fn refresh(&mut self) {
        if let Some(s) = &mut self.socket {
            if self.open {
                s.send("refresh".into());
            }
        } else {
            self.retry = Some(Instant::now());
        }
    }

    pub fn close(&mut self) {
        if let Some(s) = &mut self.socket {
            s.close();
        }
        self.socket = None;
        self.open = false;
        self.retry = None;
    }

    pub fn poll(&mut self, now: Instant) -> Result<Option<Vec<RoomSummary>>, Error> {
        if self.socket.is_none() {
            match self.retry {
                Some(at) if now >= at => {
                    self.socket = Some(Socket::connect(&self.url));
                    self.retry = None;
                }
                _ => return Ok(None),
            }
        }
        let socket = self.socket.as_mut().expect("socket");
        if self.open && now.duration_since(self.pinged) >= KEEP_ALIVE {
            socket.send(PING.into());
            self.pinged = now;
        }
        loop {
            let Some(event) = socket.poll() else { return Ok(None) };
            match event {
                WsEvent::Open => {
                    self.open = true;
                    self.backoff = Duration::from_millis(500);
                    self.pinged = now;
                    self.last_error = None;
                }
                WsEvent::Text(text) if text == PONG => {}
                WsEvent::Text(text) => match serde_json::from_str::<ServerToList>(&text) {
                    Ok(ServerToList::Rooms { rooms }) => return Ok(Some(rooms)),
                    Ok(ServerToList::Error { reason, message }) => {
                        self.close();
                        return Err(Error { kind: ErrorKind::Refused(reason), detail: format!("the signaling server refused: {message}") });
                    }
                    Err(_) => {}
                },
                WsEvent::Closed(why) => {
                    self.open = false;
                    self.socket = None;
                    self.last_error = Some(why);
                    self.retry = Some(now + self.backoff);
                    self.backoff = (self.backoff * 2).min(MAX_BACKOFF);
                    return Ok(None);
                }
            }
        }
    }
}

impl Drop for SpectateDirectory {
    fn drop(&mut self) {
        self.close();
    }
}

// ---- Zero-dependency standard base64 -------------------------------------------

const B64_ENC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard RFC 4648 base64 encoding.
pub fn base64_encode(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    let mut chunks = input.chunks_exact(3);
    for chunk in &mut chunks {
        let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
        out.push(B64_ENC[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_ENC[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64_ENC[((n >> 6) & 0x3F) as usize] as char);
        out.push(B64_ENC[(n & 0x3F) as usize] as char);
    }
    let rem = chunks.remainder();
    if rem.len() == 1 {
        let n = (rem[0] as u32) << 16;
        out.push(B64_ENC[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_ENC[((n >> 12) & 0x3F) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem.len() == 2 {
        let n = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
        out.push(B64_ENC[((n >> 18) & 0x3F) as usize] as char);
        out.push(B64_ENC[((n >> 12) & 0x3F) as usize] as char);
        out.push(B64_ENC[((n >> 6) & 0x3F) as usize] as char);
        out.push('=');
    }
    out
}

/// Standard RFC 4648 base64 decoding.
pub fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if bytes.len() % 4 != 0 {
        return Err("invalid base64 length".into());
    }
    let val = |b: u8| -> Option<u32> {
        match b {
            b'A'..=b'Z' => Some((b - b'A') as u32),
            b'a'..=b'z' => Some((b - b'a' + 26) as u32),
            b'0'..=b'9' => Some((b - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    };
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    for (idx, chunk) in bytes.chunks_exact(4).enumerate() {
        let last = idx + 1 == bytes.len() / 4;
        let a = val(chunk[0]).ok_or("invalid base64 character")?;
        let b = val(chunk[1]).ok_or("invalid base64 character")?;
        if chunk[2] == b'=' {
            if !last || chunk[3] != b'=' {
                return Err("invalid base64 padding".into());
            }
            let n = (a << 18) | (b << 12);
            out.push((n >> 16) as u8);
        } else if chunk[3] == b'=' {
            if !last {
                return Err("invalid base64 padding".into());
            }
            let c = val(chunk[2]).ok_or("invalid base64 character")?;
            let n = (a << 18) | (b << 12) | (c << 6);
            out.push((n >> 16) as u8);
            out.push((n >> 8) as u8);
        } else {
            let c = val(chunk[2]).ok_or("invalid base64 character")?;
            let d = val(chunk[3]).ok_or("invalid base64 character")?;
            let n = (a << 18) | (b << 12) | (c << 6) | d;
            out.push((n >> 16) as u8);
            out.push((n >> 8) as u8);
            out.push(n as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for sample in [b"".as_slice(), b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar", &[0, 1, 2, 254, 255]] {
            let enc = base64_encode(sample);
            assert_eq!(base64_decode(&enc).unwrap(), sample);
        }
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert!(base64_decode("abc").is_err());
        assert!(base64_decode("a=aa").is_err());
    }

    #[test]
    fn spectate_urls_and_json() {
        assert_eq!(host_url("https://signal.example/", "ABC234", "p1"), "wss://signal.example/spectate/host/ABC234?peer=p1&v=1");
        assert_eq!(watch_url("http://127.0.0.1:8787", "ABC234", "w1"), "ws://127.0.0.1:8787/spectate/watch/ABC234?peer=w1&v=1");
        assert_eq!(list_url("signal.example"), "wss://signal.example/spectate/list?v=1");

        let meta = RoomMeta {
            visibility: Visibility::Public,
            delay_secs: 10,
            game: "exe6".into(),
            engine: "0.1.2".into(),
            content_hash: "0123456789abcdef".into(),
            rules: "masters".into(),
            names: ["Lan".into(), "Chaud".into()],
            stage: RoomStage::Playing,
            score: [1, 0],
        };
        let msg = HostToServer::Config { meta: meta.clone() };
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(serde_json::from_str::<HostToServer>(&json).unwrap(), msg);
    }

    #[cfg(feature = "testing")]
    mod live {
        use super::*;
        use crate::testing::Server;

        fn wait_for<T>(mut poll: impl FnMut() -> Option<T>) -> T {
            let until = Instant::now() + Duration::from_secs(3);
            while Instant::now() < until {
                if let Some(v) = poll() {
                    return v;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            panic!("timed out waiting for event");
        }

        #[test]
        fn public_list_mid_match_catchup_and_multi_set() {
            let server = Server::start();
            let mut dir = SpectateDirectory::connect(&server.url());
            let initial = wait_for(|| dir.poll(Instant::now()).unwrap());
            assert!(initial.is_empty());

            let mut host = SpectateHost::connect(&server.url(), "ABC234", "host1");
            assert_eq!(wait_for(|| host.poll(Instant::now())), HostEvent::Connected { spectators: 0 });

            // CodeOnly stays hidden from public list.
            let mut meta = RoomMeta {
                visibility: Visibility::CodeOnly,
                delay_secs: 0,
                game: "exe6".into(),
                engine: "0.1.2".into(),
                content_hash: "0000000000000001".into(),
                rules: "masters".into(),
                names: ["Lan".into(), "Chaud".into()],
                stage: RoomStage::Lobby,
                score: [0, 0],
            };
            host.update_meta(meta.clone());

            // Switching to Public publishes to directory subscribers.
            meta.visibility = Visibility::Public;
            meta.stage = RoomStage::Playing;
            host.update_meta(meta.clone());
            let listed = wait_for(|| {
                let rooms = dir.poll(Instant::now()).unwrap()?;
                (!rooms.is_empty()).then_some(rooms)
            });
            assert_eq!(listed.len(), 1);
            assert_eq!(listed[0].code, "ABC234");
            assert_eq!(listed[0].meta.names, ["Lan", "Chaud"]);

            // Host starts set 1 and sends ticks before watcher joins.
            host.start_set(1, b"HEADER1");
            host.send_ticks(1, 0, 10, b"TICKS_A");
            std::thread::sleep(Duration::from_millis(40));

            // Watcher joins mid-match and gets full catch-up snapshot.
            let mut watcher = SpectateWatcher::connect(&server.url(), "abc-234", "watch1", Duration::from_secs(3)).unwrap();
            let snap = wait_for(|| watcher.poll(Instant::now()).unwrap());
            match snap {
                WatcherEvent::Snapshot { room, set } => {
                    assert_eq!(room.code, "ABC234");
                    let set = set.expect("active set snapshot");
                    assert_eq!(set.set_id, 1);
                    assert_eq!(base64_decode(&set.header_b64).unwrap(), b"HEADER1");
                    assert_eq!(set.chunks.len(), 1);
                    assert_eq!(base64_decode(&set.chunks[0].data_b64).unwrap(), b"TICKS_A");
                }
                other => panic!("expected Snapshot, got {other:?}"),
            }
            assert_eq!(wait_for(|| host.poll(Instant::now())), HostEvent::Spectators(1));

            // Live ticks and set end, followed by set 2 in same room.
            host.send_ticks(1, 10, 5, b"TICKS_B");
            let live_ticks = wait_for(|| match watcher.poll(Instant::now()).unwrap() {
                Some(e @ WatcherEvent::Ticks { .. }) => Some(e),
                _ => None,
            });
            assert_eq!(live_ticks, WatcherEvent::Ticks { set_id: 1, from_tick: 10, count: 5, data_b64: base64_encode(b"TICKS_B") });

            host.end_set(1, "finished");
            let ended = wait_for(|| match watcher.poll(Instant::now()).unwrap() {
                Some(e @ WatcherEvent::SetEnd { .. }) => Some(e),
                _ => None,
            });
            assert_eq!(ended, WatcherEvent::SetEnd { set_id: 1, reason: "finished".into() });

            host.start_set(2, b"HEADER2");
            let set2 = wait_for(|| match watcher.poll(Instant::now()).unwrap() {
                Some(e @ WatcherEvent::SetStart { .. }) => Some(e),
                _ => None,
            });
            assert_eq!(set2, WatcherEvent::SetStart { set_id: 2, header_b64: base64_encode(b"HEADER2") });

            // Host closes room cleanly -> watcher notified.
            host.close();
            let closed = wait_for(|| match watcher.poll(Instant::now()).unwrap() {
                Some(e @ WatcherEvent::Closed(_)) => Some(e),
                _ => None,
            });
            assert_eq!(closed, WatcherEvent::Closed("host-left".into()));
        }

        #[test]
        fn server_enforces_delay_visibility_off_and_limit() {
            let server = Server::start();
            server.set_max_spectators(1);

            let mut host = SpectateHost::connect(&server.url(), "DEF567", "host1");
            wait_for(|| host.poll(Instant::now()));
            host.update_meta(RoomMeta { visibility: Visibility::CodeOnly, delay_secs: 1, game: "exe6".into(), ..RoomMeta::default() });
            std::thread::sleep(Duration::from_millis(30));

            let mut w1 = SpectateWatcher::connect(&server.url(), "DEF567", "w1", Duration::from_secs(3)).unwrap();
            let snap = wait_for(|| w1.poll(Instant::now()).unwrap());
            assert!(matches!(snap, WatcherEvent::Snapshot { set: None, .. }));

            // Second watcher hits limit.
            let mut w2 = SpectateWatcher::connect(&server.url(), "DEF567", "w2", Duration::from_secs(3)).unwrap();
            let err = wait_for(|| w2.poll(Instant::now()).err());
            assert_eq!(err.kind, ErrorKind::Refused(Refusal::Full));

            // Start set with 1s anti-ghosting delay: must NOT arrive within 300ms.
            let sent_at = Instant::now();
            host.start_set(1, b"DELAYED_HDR");
            host.send_ticks(1, 0, 5, b"DELAYED_TICKS");
            std::thread::sleep(Duration::from_millis(300));
            while let Some(ev) = w1.poll(Instant::now()).unwrap() {
                assert!(!matches!(ev, WatcherEvent::SetStart { .. } | WatcherEvent::Ticks { .. }), "released too early: {ev:?}");
            }

            // Arrives after ~1s.
            let got = wait_for(|| match w1.poll(Instant::now()).unwrap() {
                Some(e @ WatcherEvent::SetStart { .. }) => Some(e),
                _ => None,
            });
            assert!(sent_at.elapsed() >= Duration::from_millis(900));
            assert_eq!(got, WatcherEvent::SetStart { set_id: 1, header_b64: base64_encode(b"DELAYED_HDR") });

            // Turning visibility Off kicks active watcher and refuses new ones with Disabled.
            host.update_meta(RoomMeta { visibility: Visibility::Off, ..RoomMeta::default() });
            let closed = wait_for(|| match w1.poll(Instant::now()).unwrap() {
                Some(e @ WatcherEvent::Closed(_)) => Some(e),
                _ => None,
            });
            assert_eq!(closed, WatcherEvent::Closed("disabled".into()));

            let mut w3 = SpectateWatcher::connect(&server.url(), "DEF567", "w3", Duration::from_secs(3)).unwrap();
            let err = wait_for(|| w3.poll(Instant::now()).err());
            assert_eq!(err.kind, ErrorKind::Refused(Refusal::Disabled));
        }
    }
}
