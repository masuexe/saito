#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;
use std::time::Instant;

use saito_frontend::driver::{LivePlayer, short_set};
use saito_frontend::replay::{Info, MatchKind, Recorder, Replay, play_out};
use saito_frontend::session::Session;
use saito_frontend::testing::Shared;
use saito_sim_host::WasmRuntime;

fn guest_wasm_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("SAITO_TEST_KERNEL") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for rel in [
        "target/wasm32-wasip1/release/saito_sim_guest.wasm",
        "target/wasm32-wasip1/debug/saito_sim_guest.wasm",
    ] {
        let p = root.join(rel);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn record_bytes(content: &std::sync::Arc<saito_battle::Content>, game: &str, seed: u32) -> (Vec<u8>, Replay) {
    let m = short_set::of(content, game, seed);
    let sink = Shared::default();
    let mut session = Session::new(Box::new(LivePlayer::new(saito_match::Set::of(content, &m, seed))));
    let info = Info { when: 1_790_000_000, names: ["Lan".into(), "Chaud".into()], ..Info::of(MatchKind::Offline, 0) };
    session.record(Recorder::new(Box::new(sink.clone()), content, &m, &info).unwrap()).unwrap();
    loop {
        let tick = session.ticks as u32;
        let input = short_set::shooter(&session.battle, 0, tick);
        if !session.step(input) {
            break;
        }
    }
    let bytes = sink.bytes();
    let replay = Replay::read(&bytes).unwrap();
    (bytes, replay)
}

#[test]
fn wasm_kernel_play_out_matches_native_on_cranelift_and_pulley() {
    let Some(kernel) = guest_wasm_path() else {
        eprintln!("skipping e2e wasm test: build saito-sim-guest for wasm32-wasip1 first");
        return;
    };
    let content_dir = saito_content::index::content();

    // 1. Cranelift JIT on EXE6 + EXE5
    let cranelift = WasmRuntime::with_pulley(false).expect("init cranelift runtime");
    assert!(!cranelift.is_pulley());

    for (game, content) in [
        ("exe6", saito_match::testing::exe6_content()),
        ("exe5", saito_match::testing::exe5_content()),
    ] {
        let (bytes, replay) = record_bytes(&content, game, 7);
        let expected = play_out(&content, &replay).expect("native play_out");
        assert!(expected.diverged.is_none(), "{game} native diverged");
        assert!(expected.stopped.is_none(), "{game} native stopped");

        let t0 = Instant::now();
        let got = cranelift
            .play_out(&kernel, game, &content_dir, &[], &bytes)
            .unwrap_or_else(|e| panic!("cranelift wasm play_out failed for {game}: {e}"));
        let elapsed = t0.elapsed();
        eprintln!(
            "[cranelift] {game}: {} ticks in {:.2?} ({:.0} ticks/s)",
            got.ticks,
            elapsed,
            got.ticks as f64 / elapsed.as_secs_f64().max(1e-6)
        );
        assert_eq!(got, expected, "{game} cranelift wasm outcome differs from native");
    }

    // 2. Pulley64 zero-JIT interpreter (iOS execution mode) on EXE6
    let pulley = WasmRuntime::with_pulley(true).expect("init pulley64 runtime");
    assert!(pulley.is_pulley());
    let exe6 = saito_match::testing::exe6_content();
    let (bytes, replay) = record_bytes(&exe6, "exe6", 7);
    let expected = play_out(&exe6, &replay).expect("native play_out");
    let t0 = Instant::now();
    let got = pulley
        .play_out(&kernel, "exe6", &content_dir, &[], &bytes)
        .expect("pulley64 wasm play_out failed");
    let elapsed = t0.elapsed();
    eprintln!(
        "[pulley64]  exe6: {} ticks in {:.2?} ({:.0} ticks/s)",
        got.ticks,
        elapsed,
        got.ticks as f64 / elapsed.as_secs_f64().max(1e-6)
    );
    assert_eq!(got, expected, "pulley64 wasm outcome differs from native");
}
