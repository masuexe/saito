//! The Legacy Collection's ROMs from one device to another on the local network, with no server and nothing typed. A
//! device that has the collection offers it ([`Nearby::offer`]): it listens for a TCP connection and advertises itself
//! by DNS-SD (Bonjour: multicast DNS, the service [`SERVICE`]). One that hasn't seeks ([`Nearby::seek`]): it browses for
//! those, asks one ([`Nearby::ask`]), whose player answers ([`Nearby::answer`]: the files, or none; and may stop the
//! send, [`Nearby::stop`]), and the files arrive. Polled: [`Nearby::poll`] says what happened since, on the app's thread; the network's work is on threads of
//! this crate's.
//!
//! Discovery is the system's, a module each (`src/platform.rs`): `mdns-sd` on the desktop, which offers and seeks;
//! Apple's DNS-SD on iOS and NsdManager on Android, which seek; nothing in a browser ([`can_offer`], [`can_seek`]).
//!
//! The transfer ([`protocol`]): the asker connects to an address the offer resolved to (IPv4: the offerer listens on
//! IPv4 alone) and says its name; the offerer's app asks its player, and sends the files, or says no. One transfer at
//! a time on each side.
//!
//! Unsafe code is the systems' discovery alone: their C APIs and JNI (`src/platform/`).
#![deny(unsafe_code)]

mod platform;
pub mod protocol;
#[cfg(test)]
mod tests;
mod transfer;

use platform::system;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// The DNS-SD service saito advertises and browses for when offering the Legacy Collection.
pub const SERVICE: &str = "_saito._tcp";
/// The DNS-SD service a seeking device advertises so offerers can connect outbound to it.
pub const ASK_SERVICE: &str = "_saito-ask._tcp";

/// A device found offering ([`Event::Found`]), to ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Peer(u32);

/// A seeking device found nearby ([`Event::SeekerFound`]), to reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Seeker(u32);

/// A device's ask ([`Event::Asked`]), to answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Request(u32);

/// A file that goes: its name (no folder) and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// What happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A device offering was found (seeking), by its name.
    Found { peer: Peer, name: String },
    /// One found is gone.
    Lost { peer: Peer },
    /// A seeking device was found (offering), by its name.
    SeekerFound { seeker: Seeker, name: String },
    /// A discovered seeking device is gone.
    SeekerLost { seeker: Seeker },
    /// An offerer connected to our seeker listener directly or via reverse discovery.
    Connected { name: String },
    /// A device asks for what is offered (offering): [`Nearby::answer`] it.
    Asked { request: Request, name: String },
    /// The files' bytes arrived so far, of their total (asking).
    Receiving { done: u64, total: u64 },
    /// What was asked for arrived whole, from the device named.
    Received { name: String, files: Vec<File> },
    /// The files' bytes sent so far for `request`, of their total (offering).
    Sending { request: Request, done: u64, total: u64 },
    /// What was asked for was sent whole for `request`, to the device named.
    Sent { request: Request, name: String },
    /// Sending for `request` to the device named ended before the files did: stopped ([`Nearby::stop`]), or it
    /// failed: why, for the log.
    NotSent { request: Request, name: String, why: String },
    /// The device asked said no.
    Declined { name: String },
    /// The device asked is sending already, or asking its player of another.
    Busy { name: String },
    /// The device named runs another version of this (and of saito): its version.
    OtherVersion { name: String, version: u16 },
    /// A transfer with the device named failed, or discovery did (no name): why, for the log.
    Failed { name: String, why: String },
    /// Seeking stopped: the system doesn't allow the app to look on the local network (iOS: the player said no to
    /// its question, and can say yes in Settings > Privacy & Security > Local Network).
    NotAllowed,
}

/// Whether this system offers: the desktop's.
pub const fn can_offer() -> bool {
    system::OFFERS
}

/// Whether this system seeks: all but a browser.
pub const fn can_seek() -> bool {
    system::SEEKS
}

/// This device's name as others are told it: the computer's host name (without `.local`); on iOS UIDevice's (the
/// model's, as iOS tells apps: "iPhone"); on Android the name its player set, else its model's.
pub fn device_name() -> String {
    system::device_name()
}

/// Best-effort local LAN IPv4 address for manual fallback display.
pub fn local_address() -> Option<std::net::Ipv4Addr> {
    transfer::local_ipv4()
}

/// What discovery hears, by the system's key for a service (its instance's full name).
#[derive(Clone, Debug)]
#[allow(dead_code)] // (Each system's discovery says some of these.)
pub(crate) enum Seen {
    Found { key: String, name: String, addrs: Vec<SocketAddr> },
    Lost { key: String },
    /// Browsing stopped: why.
    Failed { why: String },
    /// Browsing stopped: the system doesn't allow the app the local network (iOS, where the player said no).
    NotAllowed,
}

/// How long an offerer waits for its player's answer, and an asker for it.
pub(crate) const ANSWER_WAIT: Duration = Duration::from_secs(10 * 60);

/// The answers awaited, by request: what the offerer's connection waits on.
pub(crate) type Answers = Arc<Mutex<HashMap<u32, Sender<Option<Vec<File>>>>>>;

/// The offerer's connections, by request, from the ask till the send's end: what [`Nearby::stop`] closes.
pub(crate) type Connections = Arc<Mutex<HashMap<u32, TcpStream>>>;

/// Offering, seeking and the transfers: one an app.
pub struct Nearby {
    name: String,
    /// Whether offers/seeks are advertised via DNS-SD (the tests disable this and use ports directly).
    advertise: bool,
    tell: Sender<Event>,
    heard: Receiver<Event>,
    seen_tell: Sender<Seen>,
    seen: Receiver<Seen>,
    seeker_seen_tell: Sender<Seen>,
    seeker_seen: Receiver<Seen>,
    offering: Option<Offering>,
    seeking: Option<Seeking>,
    /// Offering devices found: by peer, the service's key, its name and addresses; by key, the peer.
    peers: HashMap<u32, (String, String, Vec<SocketAddr>)>,
    by_key: HashMap<String, u32>,
    next: u32,
    /// Seeking devices found while offering: by seeker, the service's key, its name and addresses; by key, the seeker.
    seekers: HashMap<u32, (String, String, Vec<SocketAddr>)>,
    seeker_by_key: HashMap<String, u32>,
    next_seeker: u32,
    /// IP -> name lookup for inbound reverse connections from offering computers.
    known_peers: Arc<Mutex<HashMap<std::net::IpAddr, String>>>,
    answers: Answers,
    connections: Connections,
    next_request: Arc<AtomicU32>,
    /// An outbound `ask_at` dial/transfer is in flight.
    asking_out: Arc<AtomicBool>,
    /// An outbound `reach_at` dial is in flight.
    reaching: Arc<AtomicBool>,
    /// A transfer session is under way (offerer serve, outbound ask, or reverse seeker accept).
    transferring: Arc<AtomicBool>,
}

/// The listener and its advertisement, while offering.
struct Offering {
    stop: Arc<AtomicBool>,
    _advertised: Option<system::Advertised>,
    _browsing_seekers: Option<system::Browsing>,
    port: u16,
}

impl Drop for Offering {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// The reverse listener, advertisement, and offer-browser while seeking.
struct Seeking {
    stop: Arc<AtomicBool>,
    _advertised: Option<system::Advertised>,
    _browsing: system::Browsing,
    port: u16,
}

impl Drop for Seeking {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Default for Nearby {
    fn default() -> Nearby {
        Nearby::new()
    }
}

impl Nearby {
    pub fn new() -> Nearby {
        Nearby::named(device_name(), true)
    }

    /// One named `name`, whose offers are advertised or not.
    pub(crate) fn named(name: String, advertise: bool) -> Nearby {
        let (tell, heard) = mpsc::channel();
        let (seen_tell, seen) = mpsc::channel();
        let (seeker_seen_tell, seeker_seen) = mpsc::channel();
        Nearby {
            name,
            advertise,
            tell,
            heard,
            seen_tell,
            seen,
            seeker_seen_tell,
            seeker_seen,
            offering: None,
            seeking: None,
            peers: HashMap::new(),
            by_key: HashMap::new(),
            next: 0,
            seekers: HashMap::new(),
            seeker_by_key: HashMap::new(),
            next_seeker: 0,
            known_peers: Arc::default(),
            answers: Arc::default(),
            connections: Arc::default(),
            next_request: Arc::default(),
            asking_out: Arc::default(),
            reaching: Arc::default(),
            transferring: Arc::default(),
        }
    }

    /// Offer, or stop: advertise and listen while on (where the system offers: [`can_offer`]; elsewhere nothing).
    /// What fails is said as [`Event::Failed`].
    pub fn offer(&mut self, on: bool) {
        if !on {
            self.offering = None;
            self.seekers.clear();
            self.seeker_by_key.clear();
            return;
        }
        if self.offering.is_some() || self.advertise && !can_offer() {
            return;
        }
        match self.listen() {
            Ok(offering) => self.offering = Some(offering),
            Err(why) => self.say_failed(String::new(), format!("offering: {why}")),
        }
    }

    /// The listener, its thread, and the advertisement of its port.
    fn listen(&self) -> Result<Offering, String> {
        let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|e| format!("listening: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let advertised = if self.advertise { Some(system::advertise(SERVICE, &self.name, port)?) } else { None };
        let browsing_seekers = if self.advertise {
            system::browse(ASK_SERVICE, self.seeker_seen_tell.clone())
                .map_err(|why| log::warn!("nearby: browsing seekers: {why}"))
                .ok()
        } else {
            None
        };
        let stop = Arc::new(AtomicBool::new(false));
        let offerer = transfer::Offerer {
            tell: self.tell.clone(),
            answers: self.answers.clone(),
            connections: self.connections.clone(),
            next: self.next_request.clone(),
            busy: self.transferring.clone(),
        };
        let stopped = stop.clone();
        std::thread::Builder::new().name("saito-nearby-offer".into()).spawn(move || offerer.accept(listener, &stopped)).map_err(|e| format!("no thread: {e}"))?;
        Ok(Offering { stop, _advertised: advertised, _browsing_seekers: browsing_seekers, port })
    }

    /// The port offered on (the tests' way to ask without discovery).
    #[cfg(test)]
    pub(crate) fn port(&self) -> Option<u16> {
        self.offering.as_ref().map(|o| o.port)
    }

    /// Local `IP:port` this device is offering on, if any.
    pub fn offer_address(&self) -> Option<SocketAddr> {
        let port = self.offering.as_ref()?.port;
        local_address().map(|ip| SocketAddr::from((ip, port)))
    }

    /// Local `IP:port` this device is listening on while seeking (manual fallback address).
    pub fn seek_address(&self) -> Option<SocketAddr> {
        let port = self.seeking.as_ref()?.port;
        local_address().map(|ip| SocketAddr::from((ip, port)))
    }

    /// The port sought on (the tests' way to reach a seeker without discovery).
    #[cfg(test)]
    pub(crate) fn seek_port(&self) -> Option<u16> {
        self.seeking.as_ref().map(|s| s.port)
    }

    /// Seek, or stop: listen for reverse connections, advertise `_saito-ask._tcp` best-effort, and browse while on.
    pub fn seek(&mut self, on: bool) {
        if !on {
            self.seeking = None;
            return;
        }
        if self.seeking.is_none() && can_seek() {
            match self.start_seek() {
                Ok(seeking) => self.seeking = Some(seeking),
                Err(why) => self.say_failed(String::new(), why),
            }
        }
    }

    fn start_seek(&self) -> Result<Seeking, String> {
        let listener = TcpListener::bind(("0.0.0.0", 0)).map_err(|e| format!("listening: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let advertised = if self.advertise {
            system::advertise(ASK_SERVICE, &self.name, port)
                .map_err(|why| log::warn!("nearby: advertising seeker: {why}"))
                .ok()
        } else {
            None
        };
        let browsing = system::browse(SERVICE, self.seen_tell.clone()).map_err(|why| format!("browsing: {why}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let seeker = transfer::Seeker {
            me: self.name.clone(),
            tell: self.tell.clone(),
            transferring: self.transferring.clone(),
            known_peers: self.known_peers.clone(),
        };
        let stopped = stop.clone();
        std::thread::Builder::new().name("saito-nearby-seek".into()).spawn(move || seeker.accept(listener, &stopped)).map_err(|e| format!("no thread: {e}"))?;
        Ok(Seeking { stop, _advertised: advertised, _browsing: browsing, port })
    }

    /// Ask `peer` for what it offers: its answer and the files come as events.
    pub fn ask(&mut self, peer: Peer) {
        let Some((_, name, addrs)) = self.peers.get(&peer.0).cloned() else { return };
        self.ask_at(name, addrs);
    }

    /// Ask the device at `addrs`, named `name`.
    pub(crate) fn ask_at(&mut self, name: String, addrs: Vec<SocketAddr>) {
        if self.transferring.load(Ordering::Relaxed) || self.asking_out.swap(true, Ordering::Relaxed) {
            self.say_failed(name, "a transfer is under way already".into());
            return;
        }
        let (me, tell, asking_out, transferring) = (self.name.clone(), self.tell.clone(), self.asking_out.clone(), self.transferring.clone());
        let spawned = std::thread::Builder::new().name("saito-nearby-ask".into()).spawn(move || {
            let stream = transfer::connect_any(&addrs);
            match stream {
                Some(stream) => {
                    if transferring.swap(true, Ordering::Relaxed) {
                        // A reverse connection from the offerer started while dialing.
                        asking_out.store(false, Ordering::Relaxed);
                        return;
                    }
                    let event = transfer::receive_stream(&me, stream, &name, &tell).unwrap_or_else(|why| Event::Failed { name: name.clone(), why });
                    transferring.store(false, Ordering::Relaxed);
                    asking_out.store(false, Ordering::Relaxed);
                    let _ = tell.send(event);
                }
                None => {
                    let active = transferring.load(Ordering::Relaxed);
                    asking_out.store(false, Ordering::Relaxed);
                    if !active {
                        let _ = tell.send(Event::Failed { name, why: format!("connecting to {addrs:?} failed") });
                    }
                }
            }
        });
        if let Err(e) = spawned {
            self.asking_out.store(false, Ordering::Relaxed);
            self.say_failed(String::new(), format!("no thread: {e}"));
        }
    }

    /// Connect outbound from an offering device to a discovered `seeker` (starts `Event::Asked` on the offerer).
    pub fn reach(&mut self, seeker: Seeker) {
        let Some((_, name, addrs)) = self.seekers.get(&seeker.0).cloned() else { return };
        self.reach_at(name, addrs);
    }

    /// Connect outbound from an offering device to explicit address(es), e.g. manual `IP:port` fallback.
    pub fn reach_at(&mut self, name: String, addrs: Vec<SocketAddr>) {
        if self.offering.is_none() {
            self.say_failed(name, "not offering".into());
            return;
        }
        if self.transferring.load(Ordering::Relaxed) || self.reaching.swap(true, Ordering::Relaxed) {
            self.say_failed(name, "a transfer is under way already".into());
            return;
        }
        let offerer = transfer::Offerer {
            tell: self.tell.clone(),
            answers: self.answers.clone(),
            connections: self.connections.clone(),
            next: self.next_request.clone(),
            busy: self.transferring.clone(),
        };
        let reaching = self.reaching.clone();
        let spawned = std::thread::Builder::new().name("saito-nearby-reach".into()).spawn(move || {
            let stream = transfer::connect_any(&addrs);
            reaching.store(false, Ordering::Relaxed);
            match stream {
                Some(stream) => offerer.serve(stream),
                None => {
                    let _ = offerer.tell.send(Event::Failed { name, why: format!("connecting to {addrs:?} failed") });
                }
            }
        });
        if let Err(e) = spawned {
            self.reaching.store(false, Ordering::Relaxed);
            self.say_failed(String::new(), format!("no thread: {e}"));
        }
    }

    /// Answer `request`: the files to send, or none (declined).
    pub fn answer(&mut self, request: Request, files: Option<Vec<File>>) {
        if let Some(waiting) = self.answers.lock().unwrap_or_else(|e| e.into_inner()).remove(&request.0) {
            let _ = waiting.send(files);
        }
    }

    /// Stop `request`: one not answered yet is declined; one being sent ends where it is, said as [`Event::NotSent`].
    pub fn stop(&mut self, request: Request) {
        let waiting = self.answers.lock().unwrap_or_else(|e| e.into_inner()).remove(&request.0);
        match waiting {
            Some(waiting) => {
                let _ = waiting.send(None);
            }
            None => {
                if let Some(connection) = self.connections.lock().unwrap_or_else(|e| e.into_inner()).get(&request.0) {
                    // (The send's next write fails.)
                    let _ = connection.shutdown(std::net::Shutdown::Both);
                }
            }
        }
    }

    /// What happened since the last poll, in order.
    pub fn poll(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let mut peers_changed = false;
        while let Ok(seen) = self.seen.try_recv() {
            match seen {
                Seen::Found { key, name, addrs } => {
                    let addrs: Vec<SocketAddr> = addrs.into_iter().filter(SocketAddr::is_ipv4).collect();
                    if addrs.is_empty() {
                        continue;
                    }
                    if let Some(&id) = self.by_key.get(&key) {
                        if let Some(entry) = self.peers.get_mut(&id) {
                            entry.1 = name;
                            for addr in addrs {
                                if !entry.2.contains(&addr) {
                                    entry.2.push(addr);
                                }
                            }
                            peers_changed = true;
                        }
                    } else {
                        self.next += 1;
                        let id = self.next;
                        self.by_key.insert(key.clone(), id);
                        self.peers.insert(id, (key, name.clone(), addrs));
                        peers_changed = true;
                        events.push(Event::Found { peer: Peer(id), name });
                    }
                }
                Seen::Lost { key } => {
                    if let Some(id) = self.by_key.remove(&key) {
                        self.peers.remove(&id);
                        peers_changed = true;
                        events.push(Event::Lost { peer: Peer(id) });
                    }
                }
                Seen::Failed { why } => {
                    self.seeking = None;
                    events.push(Event::Failed { name: String::new(), why: format!("browsing: {why}") });
                }
                Seen::NotAllowed => {
                    self.seeking = None;
                    events.push(Event::NotAllowed);
                }
            }
        }
        if peers_changed {
            let mut known = self.known_peers.lock().unwrap_or_else(|e| e.into_inner());
            known.clear();
            for (_, name, addrs) in self.peers.values() {
                for addr in addrs {
                    known.entry(addr.ip()).or_insert_with(|| name.clone());
                }
            }
        }
        while let Ok(seen) = self.seeker_seen.try_recv() {
            match seen {
                Seen::Found { key, name, addrs } => {
                    let addrs: Vec<SocketAddr> = addrs.into_iter().filter(SocketAddr::is_ipv4).collect();
                    if addrs.is_empty() {
                        continue;
                    }
                    if let Some(&id) = self.seeker_by_key.get(&key) {
                        if let Some(entry) = self.seekers.get_mut(&id) {
                            entry.1 = name;
                            for addr in addrs {
                                if !entry.2.contains(&addr) {
                                    entry.2.push(addr);
                                }
                            }
                        }
                    } else {
                        self.next_seeker += 1;
                        let id = self.next_seeker;
                        self.seeker_by_key.insert(key.clone(), id);
                        self.seekers.insert(id, (key, name.clone(), addrs));
                        events.push(Event::SeekerFound { seeker: Seeker(id), name });
                    }
                }
                Seen::Lost { key } => {
                    if let Some(id) = self.seeker_by_key.remove(&key) {
                        self.seekers.remove(&id);
                        events.push(Event::SeekerLost { seeker: Seeker(id) });
                    }
                }
                Seen::Failed { why } => {
                    log::warn!("nearby: browsing seekers failed: {why}");
                }
                Seen::NotAllowed => {}
            }
        }
        events.extend(self.heard.try_iter());
        events
    }

    fn say_failed(&self, name: String, why: String) {
        let _ = self.tell.send(Event::Failed { name, why });
    }
}
