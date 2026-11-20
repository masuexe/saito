//! Transfers over the loopback, asked by port (no discovery: the offers aren't advertised).

use crate::{Event, File, Nearby, protocol};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

fn wait_for(nearby: &mut Nearby, what: impl Fn(&Event) -> bool) -> Event {
    let until = Instant::now() + Duration::from_secs(10);
    let mut seen = Vec::new();
    while Instant::now() < until {
        let batch = nearby.poll();
        for event in &batch {
            if what(event) {
                return event.clone();
            }
        }
        seen.extend(batch);
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no such event in time; seen: {seen:?}");
}

fn files() -> Vec<File> {
    vec![File { name: "BR6E.zip".into(), bytes: vec![7; 300_000] }, File { name: "BR5J.zip".into(), bytes: vec![9; 5] }]
}

/// An offer, unadvertised, and where to ask it.
fn offerer() -> (Nearby, Vec<SocketAddr>) {
    let mut offerer = Nearby::named("computer".into(), false);
    offerer.offer(true);
    let port = offerer.port().expect("offering");
    (offerer, vec![SocketAddr::from(([127, 0, 0, 1], port))])
}

/// Asked over the loopback: the offerer's player is asked, sends, and the asker has the files whole; then a second
/// ask is declined.
#[test]
fn an_ask_is_answered_with_the_files() {
    let (mut offerer, at) = offerer();
    let mut asker = Nearby::named("phone".into(), false);

    asker.ask_at("computer".into(), at.clone());
    let Event::Asked { request, name } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    assert_eq!(name, "phone");
    offerer.answer(request, Some(files()));
    let Event::Received { name, files: got } = wait_for(&mut asker, |e| matches!(e, Event::Received { .. })) else { unreachable!() };
    assert_eq!((name.as_str(), got), ("computer", files()));
    assert!(matches!(wait_for(&mut offerer, |e| matches!(e, Event::Sent { .. })), Event::Sent { request: r, name } if r == request && name == "phone"));

    asker.ask_at("computer".into(), at);
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    offerer.answer(request, None);
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Declined { .. })), Event::Declined { name } if name == "computer"));
}

/// While one asker waits for the offerer's player, another is told it is busy.
#[test]
fn a_second_asker_is_told_busy() {
    let (mut offerer, at) = offerer();
    let mut first = Nearby::named("phone".into(), false);
    first.ask_at("computer".into(), at.clone());
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    let mut second = Nearby::named("tablet".into(), false);
    second.ask_at("computer".into(), at);
    assert!(matches!(wait_for(&mut second, |e| matches!(e, Event::Busy { .. })), Event::Busy { name } if name == "computer"));
    offerer.answer(request, None);
    assert!(matches!(wait_for(&mut first, |e| matches!(e, Event::Declined { .. })), Event::Declined { .. }));
}

/// A send stopped by the offerer's player ends: said as not sent, and the asker's transfer fails. (The asker here
/// reads nothing, so the send can't be done before it is stopped.)
#[test]
fn a_send_stopped_is_not_sent() {
    let (mut offerer, at) = offerer();
    let mut stream = TcpStream::connect(at[0]).unwrap();
    protocol::write_hello(&mut stream, "phone").unwrap();
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    offerer.answer(request, Some(vec![File { name: "BR6E.zip".into(), bytes: vec![7; 64 << 20] }]));
    offerer.stop(request);
    let said = wait_for(&mut offerer, |e| matches!(e, Event::NotSent { .. } | Event::Sent { .. }));
    assert!(matches!(said, Event::NotSent { request: r, ref name, .. } if r == request && name == "phone"), "{said:?}");
}

/// Stopped before its player answered, an ask is declined.
#[test]
fn an_ask_stopped_is_declined() {
    let (mut offerer, at) = offerer();
    let mut asker = Nearby::named("phone".into(), false);
    asker.ask_at("computer".into(), at);
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    offerer.stop(request);
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Declined { .. })), Event::Declined { .. }));
}

/// An asker of another version is told the offerer's, and the offerer's player isn't asked.
#[test]
fn another_version_is_refused() {
    let (mut offerer, at) = offerer();
    let mut stream = TcpStream::connect(at[0]).unwrap();
    use std::io::Write;
    stream.write_all(&protocol::MAGIC).unwrap();
    stream.write_all(&(protocol::VERSION + 1).to_be_bytes()).unwrap();
    stream.write_all(&[0, 1, b'x']).unwrap();
    assert_eq!(protocol::read_reply(&mut stream).unwrap(), protocol::Reply::OtherVersion(protocol::VERSION));
    let said = wait_for(&mut offerer, |e| matches!(e, Event::OtherVersion { .. }));
    assert!(matches!(said, Event::OtherVersion { version, .. } if version == protocol::VERSION + 1));
}

/// Nothing answers at the address: the ask fails, and a next ask can be made.
#[test]
fn an_ask_of_nothing_fails() {
    let (offerer, at) = offerer();
    drop(offerer);
    // (The listener's thread lets the port go within its next turn.)
    std::thread::sleep(Duration::from_millis(200));
    let mut asker = Nearby::named("phone".into(), false);
    asker.ask_at("computer".into(), at.clone());
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Failed { .. })), Event::Failed { name, .. } if name == "computer"));
    asker.ask_at("computer".into(), at);
    assert!(matches!(wait_for(&mut asker, |e| matches!(e, Event::Failed { .. })), Event::Failed { why, .. } if why.contains("connecting")));
}

/// An offerer reaches a seeker over loopback (reverse TCP): the seeker emits Connected, the offerer prompts
/// the player and sends, and the seeker receives the files whole.
#[test]
fn reverse_connect_sends_files() {
    let (mut offerer, _) = offerer();
    let mut seeker = Nearby::named("phone".into(), false);
    seeker.seek(true);
    let seek_port = seeker.seek_port().expect("seeking listener");
    let seek_at = vec![SocketAddr::from(([127, 0, 0, 1], seek_port))];

    offerer.reach_at("phone".into(), seek_at);
    assert!(matches!(wait_for(&mut seeker, |e| matches!(e, Event::Connected { .. })), Event::Connected { .. }));
    let Event::Asked { request, name } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    assert_eq!(name, "phone");
    offerer.answer(request, Some(files()));
    let Event::Received { files: got, .. } = wait_for(&mut seeker, |e| matches!(e, Event::Received { .. })) else { unreachable!() };
    assert_eq!(got, files());
    assert!(matches!(wait_for(&mut offerer, |e| matches!(e, Event::Sent { .. })), Event::Sent { request: r, name } if r == request && name == "phone"));
}

/// Reverse connection succeeds immediately even while an outbound `ask_at` is in flight (`asking_out` held).
#[test]
fn reverse_connect_succeeds_while_ask_at_stalls() {
    let (mut offerer, _) = offerer();
    let mut seeker = Nearby::named("phone".into(), false);
    seeker.seek(true);
    let seek_port = seeker.seek_port().expect("seeking listener");

    // Simulate an in-flight outbound dial (`ask_at` blocked on firewall timeout) without holding `transferring`.
    seeker.asking_out.store(true, std::sync::atomic::Ordering::Relaxed);
    offerer.reach_at("phone".into(), vec![SocketAddr::from(([127, 0, 0, 1], seek_port))]);
    assert!(matches!(wait_for(&mut seeker, |e| matches!(e, Event::Connected { .. })), Event::Connected { .. }));
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    offerer.answer(request, Some(files()));
    let Event::Received { files: got, .. } = wait_for(&mut seeker, |e| matches!(e, Event::Received { .. })) else { unreachable!() };
    assert_eq!(got, files());
}

/// When an address list contains unreachable endpoints before a live offerer endpoint, `connect_any` still reaches the live offerer quickly.
#[test]
fn connect_any_skips_unreachable_addresses() {
    let (mut offerer, mut at) = offerer();
    let closed_port_a = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let closed_port_b = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let live = at.pop().unwrap();
    let addrs = vec![
        SocketAddr::from(([127, 0, 0, 1], closed_port_a)),
        SocketAddr::from(([127, 0, 0, 1], closed_port_b)),
        live,
    ];
    let mut asker = Nearby::named("phone".into(), false);
    asker.ask_at("computer".into(), addrs);
    let Event::Asked { request, .. } = wait_for(&mut offerer, |e| matches!(e, Event::Asked { .. })) else { unreachable!() };
    offerer.answer(request, Some(files()));
    let Event::Received { files: got, .. } = wait_for(&mut asker, |e| matches!(e, Event::Received { .. })) else { unreachable!() };
    assert_eq!(got, files());
}
