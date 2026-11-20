//! The two sides of a transfer, each on a thread of its own: the asker's ([`receive_stream`], plus
//! [`Seeker`] accepting reverse connections from an offerer) and the offerer's ([`Offerer::accept`], [`Offerer::serve`]).

use crate::{ANSWER_WAIT, Answers, Connections, Event, Request, protocol};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a connection may go silent mid-transfer.
const SILENCE: Duration = Duration::from_secs(30);
/// How long connecting to one of a peer's addresses may take.
const CONNECTING: Duration = Duration::from_secs(4);
/// Stagger between launching parallel connection attempts across multiple resolved addresses.
const STAGGER: Duration = Duration::from_millis(120);

/// Best-effort local RFC1918 LAN IPv4 address (probing private ranges first to skip proxy/VPN TUN adapters).
pub(crate) fn local_ipv4() -> Option<Ipv4Addr> {
    for target in ["192.168.255.254:80", "10.255.255.254:80", "172.31.255.254:80"] {
        if let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0))
            && socket.connect(target).is_ok()
            && let Ok(SocketAddr::V4(local)) = socket.local_addr()
        {
            let ip = *local.ip();
            if !ip.is_loopback() && !ip.is_unspecified() && ip.is_private() {
                return Some(ip);
            }
        }
    }
    if let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0))
        && socket.connect("1.1.1.1:80").is_ok()
        && let Ok(SocketAddr::V4(local)) = socket.local_addr()
    {
        let ip = *local.ip();
        if !ip.is_loopback() && !ip.is_unspecified() {
            return Some(ip);
        }
    }
    None
}

fn addr_rank(addr: SocketAddr, local: Option<Ipv4Addr>) -> u8 {
    let IpAddr::V4(ip) = addr.ip() else {
        return 4;
    };
    if let Some(l) = local {
        let lo = l.octets();
        let io = ip.octets();
        if lo[..3] == io[..3] {
            return 0;
        }
        if lo[..2] == io[..2] {
            return 1;
        }
    }
    if ip.is_private() || ip.is_loopback() {
        2
    } else {
        3
    }
}

/// Order resolved addresses so same-subnet LAN IPv4 addresses are attempted first, removing duplicates.
pub(crate) fn sort_addrs(addrs: &[SocketAddr], local: Option<Ipv4Addr>) -> Vec<SocketAddr> {
    let mut ranked: Vec<(u8, usize, SocketAddr)> = Vec::new();
    for (idx, &addr) in addrs.iter().enumerate() {
        if !ranked.iter().any(|(_, _, existing)| *existing == addr) {
            ranked.push((addr_rank(addr, local), idx, addr));
        }
    }
    ranked.sort_by_key(|(rank, idx, _)| (*rank, *idx));
    ranked.into_iter().map(|(_, _, addr)| addr).collect()
}

/// Dial `addrs` with LAN-first ordering and staggered parallel attempts so blackholed virtual NICs don't stall LAN IPs.
pub(crate) fn connect_any(addrs: &[SocketAddr]) -> Option<TcpStream> {
    let mut ordered = sort_addrs(addrs, local_ipv4());
    ordered.truncate(12);
    if ordered.is_empty() {
        return None;
    }
    if ordered.len() == 1 {
        return TcpStream::connect_timeout(&ordered[0], CONNECTING).ok();
    }
    let (tx, rx) = mpsc::channel::<Result<TcpStream, ()>>();
    let mut active = 0usize;
    for (i, addr) in ordered.into_iter().enumerate() {
        if i > 0 {
            match rx.recv_timeout(STAGGER) {
                Ok(Ok(stream)) => return Some(stream),
                Ok(Err(())) => active = active.saturating_sub(1),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        active += 1;
        let tx = tx.clone();
        if std::thread::Builder::new()
            .name("saito-nearby-dial".into())
            .spawn(move || {
                let res = TcpStream::connect_timeout(&addr, CONNECTING).map_err(|_| ());
                let _ = tx.send(res);
            })
            .is_err()
        {
            active -= 1;
        }
    }
    drop(tx);
    let deadline = Instant::now() + CONNECTING;
    while active > 0 {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else { break };
        match rx.recv_timeout(left) {
            Ok(Ok(stream)) => return Some(stream),
            Ok(Err(())) => active -= 1,
            Err(_) => break,
        }
    }
    None
}

/// Run the asker protocol over an already-connected TCP stream.
pub fn receive_stream(me: &str, mut stream: TcpStream, name: &str, tell: &Sender<Event>) -> Result<Event, String> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_nodelay(true);
    stream.set_write_timeout(Some(SILENCE)).map_err(|e| e.to_string())?;
    protocol::write_hello(&mut stream, me).map_err(|e| format!("asking: {e}"))?;
    stream.set_read_timeout(Some(ANSWER_WAIT)).map_err(|e| e.to_string())?;
    let reply = protocol::read_reply(&mut stream).map_err(|e| format!("the answer: {e}"))?;
    let name = name.to_string();
    Ok(match reply {
        protocol::Reply::Declined => Event::Declined { name },
        protocol::Reply::Busy => Event::Busy { name },
        protocol::Reply::OtherVersion(version) => Event::OtherVersion { name, version },
        protocol::Reply::Accepted { count, total } => {
            stream.set_read_timeout(Some(SILENCE)).map_err(|e| e.to_string())?;
            let mut said = 0;
            let mut input = std::io::BufReader::with_capacity(256 << 10, &mut stream);
            let files = protocol::read_files(&mut input, count, total, &mut |done, total| {
                if passes_percent(&mut said, done, total) {
                    let _ = tell.send(Event::Receiving { done, total });
                }
            })
            .map_err(|e| format!("receiving: {e}"))?;
            Event::Received { name, files }
        }
    })
}

/// The seeker's listener side: accepts reverse connections from offerers that dial us outbound.
pub struct Seeker {
    pub me: String,
    pub tell: Sender<Event>,
    pub transferring: Arc<AtomicBool>,
    pub known_peers: Arc<Mutex<HashMap<IpAddr, String>>>,
}

impl Seeker {
    pub fn accept(self, listener: TcpListener, stop: &AtomicBool) {
        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    let seeker = Seeker {
                        me: self.me.clone(),
                        tell: self.tell.clone(),
                        transferring: self.transferring.clone(),
                        known_peers: self.known_peers.clone(),
                    };
                    let _ = std::thread::Builder::new().name("saito-nearby-recv".into()).spawn(move || seeker.serve(stream, peer_addr));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => {
                    log::warn!("nearby: accepting reverse connection: {e}");
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }

    fn serve(self, stream: TcpStream, peer_addr: SocketAddr) {
        if self.transferring.swap(true, Ordering::Relaxed) {
            return;
        }
        let name = self
            .known_peers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&peer_addr.ip())
            .cloned()
            .unwrap_or_else(|| peer_addr.ip().to_string());
        let _ = self.tell.send(Event::Connected { name: name.clone() });
        let event = receive_stream(&self.me, stream, &name, &self.tell).unwrap_or_else(|why| Event::Failed { name, why });
        self.transferring.store(false, Ordering::Relaxed);
        let _ = self.tell.send(event);
    }
}

/// The offerer's side: what each connection's thread shares.
pub struct Offerer {
    pub tell: Sender<Event>,
    pub answers: Answers,
    pub connections: Connections,
    pub next: Arc<AtomicU32>,
    pub busy: Arc<AtomicBool>,
}

impl Offerer {
    /// Accept connections till `stop`, each served on a thread of its own.
    pub fn accept(self, listener: TcpListener, stop: &AtomicBool) {
        while !stop.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let offerer = Offerer {
                        tell: self.tell.clone(),
                        answers: self.answers.clone(),
                        connections: self.connections.clone(),
                        next: self.next.clone(),
                        busy: self.busy.clone(),
                    };
                    let _ = std::thread::Builder::new().name("saito-nearby-send".into()).spawn(move || offerer.serve(stream));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => {
                    let _ = self.tell.send(Event::Failed { name: String::new(), why: format!("accepting: {e}") });
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
    }

    /// One connection: hear who asks, ask the player, and send or say no.
    pub(crate) fn serve(self, mut stream: TcpStream) {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(SILENCE));
        let _ = stream.set_write_timeout(Some(SILENCE));
        let hello = match protocol::read_hello(&mut stream) {
            Ok(hello) => hello,
            Err(e) => {
                log::warn!("nearby: a connection that isn't saito's asking: {e}");
                return;
            }
        };
        let name = hello.name;
        if hello.version != protocol::VERSION {
            let _ = protocol::write_refusal(&mut stream, protocol::Reply::OtherVersion(protocol::VERSION));
            let _ = self.tell.send(Event::OtherVersion { name, version: hello.version });
            return;
        }
        if self.busy.swap(true, Ordering::Relaxed) {
            let _ = protocol::write_refusal(&mut stream, protocol::Reply::Busy);
            return;
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = mpsc::channel();
        self.answers.lock().unwrap_or_else(|e| e.into_inner()).insert(id, answer);
        if let Ok(connection) = stream.try_clone() {
            self.connections.lock().unwrap_or_else(|e| e.into_inner()).insert(id, connection);
        }
        let _ = self.tell.send(Event::Asked { request: Request(id), name: name.clone() });
        let files = answered.recv_timeout(ANSWER_WAIT).ok().flatten();
        self.answers.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
        let event = match files {
            None => {
                let _ = protocol::write_refusal(&mut stream, protocol::Reply::Declined);
                None
            }
            Some(files) => {
                let mut said = 0;
                let mut out = std::io::BufWriter::with_capacity(256 << 10, &mut stream);
                let request = Request(id);
                let sent = protocol::write_files(&mut out, &files, &mut |done, total| {
                    if passes_percent(&mut said, done, total) {
                        let _ = self.tell.send(Event::Sending { request, done, total });
                    }
                })
                .and_then(|()| {
                    use std::io::Write;
                    out.flush()
                });
                Some(match sent {
                    Ok(()) => Event::Sent { request, name },
                    Err(e) => Event::NotSent { request, name, why: format!("sending: {e}") },
                })
            }
        };
        self.connections.lock().unwrap_or_else(|e| e.into_inner()).remove(&id);
        self.busy.store(false, Ordering::Relaxed);
        if let Some(event) = event {
            let _ = self.tell.send(event);
        }
    }
}

/// Whether `done` of `total` passes a whole percent since the one `said` (or is the end): progress is said as each is.
fn passes_percent(said: &mut u64, done: u64, total: u64) -> bool {
    let percent = done * 100 / total.max(1);
    let passes = percent != *said || done == total;
    *said = percent;
    passes
}
