//! Sandboxed WASM guest simulation kernel (`saito-sim-<line>.wasm`).
//!
//! Wraps `saito-frontend`'s exact engine, content loader, renderer, and audio mixer behind the
//! version-stable C ABI defined in `saito-sim-abi`. Compiles to single-threaded `wasm32-wasip1`
//! (read-only preopened directories) with zero UI, networking, or OS write capabilities.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use saito_battle::battle::mode;
use saito_battle::content::Content;
use saito_extract::RomSource;
use saito_frontend::game::{Enhancements, Loaded, Options};
use saito_frontend::party::{self, Panel};
use saito_frontend::replay::{ENGINE, End, MatchKind, Replay, play_out_compat};
use saito_frontend::viewer::{Speed, Viewer};
use saito_render::compose::Size;
use saito_sim_abi::{
    ABI_VERSION, EnhancementsConfig, PartyMemberSnapshot, PartySnapshot, PlayOutRequest, PlayOutResponse, RomMount,
    StillRequest, ViewerOpenRequest, ViewerOpenResponse, ViewerState,
};

const STILL_AFTER: u32 = 150;

struct OpenViewer {
    viewer: Viewer,
    game: String,
    loaded: Arc<Loaded>,
    panels: [Panel; 2],
    limits: [Option<Option<u32>>; 2],
    samples_buf: Vec<[f32; 2]>,
    pixels_buf: Vec<u32>,
}

#[derive(Default)]
struct GuestState {
    last_error: String,
    /// Cached battle content for fast headless `play_out` across multiple replays.
    content_cache: HashMap<(String, String, Vec<(String, String)>), Arc<Content>>,
    /// Cached loaded games (content + extracted ROM graphics/sound) for fast `still` and `viewer_open`.
    game_cache: HashMap<(String, String, String), Arc<Loaded>>,
    next_handle: u32,
    viewers: HashMap<u32, OpenViewer>,
}

thread_local! {
    static STATE: RefCell<GuestState> = RefCell::new(GuestState::default());
}

fn set_error(msg: impl Into<String>) -> i32 {
    let msg = msg.into();
    STATE.with(|s| s.borrow_mut().last_error = msg);
    -1
}

fn write_out(data: &[u8], out_ptr: *mut u8, out_cap: u32) -> i32 {
    if data.len() > out_cap as usize {
        // Negative value past -1 encodes required capacity: `-(required + 2)`.
        return -((data.len() as i32).saturating_add(2));
    }
    if !data.is_empty() && !out_ptr.is_null() {
        // Safety: caller (the Wasm host) allocated `out_ptr..out_ptr+out_cap` via `saito_sim_alloc`.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), out_ptr, data.len());
        }
    }
    data.len() as i32
}

fn read_slice<'a>(ptr: *const u8, len: u32) -> &'a [u8] {
    if len == 0 || ptr.is_null() {
        return &[];
    }
    // Safety: caller passed a valid guest linear-memory slice.
    unsafe { std::slice::from_raw_parts(ptr, len as usize) }
}

fn end_code(end: End) -> u8 {
    match end {
        End::Set => 0,
        End::Open => 1,
        End::Cut => 2,
    }
}

fn speed_from_code(code: u32) -> Speed {
    match code {
        0 => Speed::Half,
        1 => Speed::Normal,
        2 => Speed::Double,
        _ => Speed::Quadruple,
    }
}

fn speed_to_code(speed: Speed) -> u8 {
    match speed {
        Speed::Half => 0,
        Speed::Normal => 1,
        Speed::Double => 2,
        Speed::Quadruple => 3,
    }
}

fn enhancements_of(cfg: EnhancementsConfig) -> Enhancements {
    Enhancements {
        text: cfg.text,
        hud: cfg.hud,
        motion: cfg.motion,
        fill: cfg.fill,
        reduce_flashes: cfg.reduce_flashes,
        hide_aura_number: cfg.hide_aura_number,
        hd_chip_art: cfg.hd_chip_art,
    }
}

fn rom_source_of(mount: &RomMount) -> Result<RomSource, String> {
    match mount {
        RomMount::Collection { dir } => RomSource::at(Path::new(dir)).map_err(|e| e.to_string()),
        RomMount::Copies { dir } => Ok(RomSource::Copies(PathBuf::from(dir))),
        RomMount::Files { paths } => Ok(RomSource::Files(paths.iter().map(PathBuf::from).collect())),
    }
}

fn pool_paths(pool: &[(String, String)]) -> Vec<(String, PathBuf)> {
    pool.iter().map(|(g, d)| (g.clone(), PathBuf::from(d))).collect()
}

fn load_content(game: &str, content_dir: &str, pool: &[(String, String)]) -> Result<Arc<Content>, String> {
    let key = (game.to_string(), content_dir.to_string(), pool.to_vec());
    if let Some(cached) = STATE.with(|s| s.borrow().content_cache.get(&key).cloned()) {
        return Ok(cached);
    }
    let dir = PathBuf::from(content_dir);
    let (mut content, report) = saito_content::load::battle_content(&dir, game).map_err(|r| r.to_string())?;
    let pool_dirs = pool_paths(pool);
    if !pool_dirs.is_empty() {
        content.background_pool = saito_content::locale::background_pool_of(&pool_dirs);
    }
    content.define().map_err(|e| format!("{}: {report}", e.message))?;
    let content = Arc::new(content);
    STATE.with(|s| s.borrow_mut().content_cache.insert(key, content.clone()));
    Ok(content)
}

fn load_game(
    game: &str,
    content_dir: &str,
    pool: &[(String, String)],
    roms: &RomMount,
    lang: &str,
    enhancements: Enhancements,
    want_sound: bool,
) -> Result<Arc<Loaded>, String> {
    let roms_key = serde_json::to_string(roms).unwrap_or_default();
    let cache_key = (
        game.to_string(),
        content_dir.to_string(),
        format!(
            "{roms_key}|{lang}|{}|{}|{}|{}|{}|{}|{want_sound}",
            enhancements.text, enhancements.hud, enhancements.motion, enhancements.fill, enhancements.reduce_flashes, enhancements.hide_aura_number
        ),
    );
    if let Some(cached) = STATE.with(|s| s.borrow().game_cache.get(&cache_key).cloned()) {
        return Ok(cached);
    }
    let source = rom_source_of(roms)?;
    let options = Options {
        roms: Some(source),
        content: Some(PathBuf::from(content_dir)),
        pool: pool_paths(pool),
        lang: lang.to_string(),
        enhancements,
        sound: want_sound,
        ..Options::default()
    };
    let loaded = saito_frontend::game::load(game, &options).map_err(|e| format!("{e}: {}", e.report))?;
    let loaded = Arc::new(loaded);
    STATE.with(|s| s.borrow_mut().game_cache.insert(cache_key, loaded.clone()));
    Ok(loaded)
}

fn limit_of(b: &saito_battle::Battle, game: &str, side: u8) -> Option<u32> {
    let block = b.setup.players[side as usize & 1].rules.clone()?;
    let s = saito_match::Side { facts: saito_match::facts::Facts::of_block(block) };
    saito_match::build::Checked::of(&b.content, game, &s).overview(&b.content, &s)?.switches
}

fn party_snapshot(open: &mut OpenViewer, side: u8) -> Option<PartySnapshot> {
    let b = open.viewer.battle();
    let view = party::view(b, side);
    let idx = side as usize & 1;
    open.panels[idx].follow(view.as_ref());
    let v = view?;
    let limit = *open.limits[idx].get_or_insert_with(|| limit_of(b, &open.game, side));
    let members = v
        .members
        .iter()
        .map(|m| PartyMemberSnapshot {
            navi_key: b.content.defs.navi(m.navi).key.clone(),
            hp: m.hp,
            max_hp: m.max_hp,
        })
        .collect();
    Some(PartySnapshot {
        members,
        active: open.panels[idx].active(),
        left: open.panels[idx].member(0),
        right: open.panels[idx].member(1),
        switches: v.switches,
        limit,
    })
}

/// Core implementation of headless play-out (callable directly from Rust unit tests too).
pub fn exec_play_out(req: &PlayOutRequest, replay_bytes: &[u8]) -> Result<PlayOutResponse, String> {
    let replay = Replay::read(replay_bytes).map_err(|e| e.to_string())?;
    let content = load_content(&req.game, &req.content_dir, &req.pool)?;
    let compat = replay.head.compat(saito_match::capabilities::SIM_EPOCHS, ENGINE, content.game(), content.hash().0).map_err(|e| e.to_string())?;
    let outcome = play_out_compat(&content, &replay, compat).map_err(|e| e.to_string())?;
    Ok(PlayOutResponse {
        ticks: outcome.ticks,
        rounds: outcome.rounds,
        result: outcome.result.map(|r| r as u8),
        end: end_code(outcome.end),
        diverged: outcome.diverged,
        stopped: outcome.stopped,
    })
}

/// Core implementation of 240×160 still rendering (returns `240 * 160` `0x00RRGGBB` pixels as little-endian bytes).
pub fn exec_still(req: &StillRequest, replay_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let replay = Replay::read(replay_bytes).map_err(|e| e.to_string())?;
    let enhancements = Enhancements { motion: false, ..enhancements_of(req.enhancements) };
    let loaded = load_game(&req.game, &req.content_dir, &req.pool, &req.roms, &req.lang, enhancements, false)?;
    let compat = replay.head.compat(saito_match::capabilities::SIM_EPOCHS, ENGINE, loaded.game.content.game(), loaded.game.content.hash().0).map_err(|e| e.to_string())?;
    let mut viewer = Viewer::open_compat(&loaded, &replay, Some(req.side), compat).map_err(|e| e.to_string())?;
    let mut fought = 0;
    while fought < STILL_AFTER && viewer.step_forward() {
        fought += (viewer.battle().round.mode == mode::FIGHTING) as u32;
    }
    let mut pixels = vec![0u32; 240 * 160];
    viewer.present(&mut pixels, 240, 160);
    let mut bytes = Vec::with_capacity(pixels.len() * 4);
    for px in pixels {
        bytes.extend_from_slice(&px.to_le_bytes());
    }
    Ok(bytes)
}

/// Core implementation of opening an interactive viewer.
pub fn exec_viewer_open(req: &ViewerOpenRequest, replay_bytes: &[u8]) -> Result<ViewerOpenResponse, String> {
    let replay = Replay::read(replay_bytes).map_err(|e| e.to_string())?;
    let enhancements = enhancements_of(req.enhancements);
    let loaded = load_game(&req.game, &req.content_dir, &req.pool, &req.roms, &req.lang, enhancements, req.want_sound)?;
    let compat = replay.head.compat(saito_match::capabilities::SIM_EPOCHS, ENGINE, loaded.game.content.game(), loaded.game.content.hash().0).map_err(|e| e.to_string())?;
    let viewer = Viewer::open_compat(&loaded, &replay, req.view, compat).map_err(|e| e.to_string())?;
    let resp = ViewerOpenResponse {
        handle: 0,
        length: viewer.length(),
        rounds: viewer.rounds().to_vec(),
        end: end_code(viewer.end()),
        engine: viewer.engine().to_string(),
        when: viewer.info().when,
        side: viewer.info().side,
        kind: match viewer.info().kind {
            MatchKind::Offline => 0,
            MatchKind::Online => 1,
        },
        open_setups: viewer.info().open_setups,
        names: viewer.info().names.clone(),
        has_sound: viewer.has_sound(),
    };
    let handle = STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.next_handle = st.next_handle.wrapping_add(1).max(1);
        let h = st.next_handle;
        st.viewers.insert(
            h,
            OpenViewer {
                viewer,
                game: req.game.clone(),
                loaded,
                panels: Default::default(),
                limits: [None, None],
                samples_buf: Vec::new(),
                pixels_buf: Vec::new(),
            },
        );
        h
    });
    Ok(ViewerOpenResponse { handle, ..resp })
}

// ---- C ABI exports -------------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_abi_version() -> u32 {
    ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_alloc(len: u32) -> *mut u8 {
    let len = len.max(1) as usize;
    let mut buf = Vec::<u8>::with_capacity(len);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_dealloc(ptr: *mut u8, len: u32) {
    if ptr.is_null() {
        return;
    }
    let len = len.max(1) as usize;
    // Safety: pointer/capacity originally allocated by `saito_sim_alloc`.
    unsafe {
        drop(Vec::from_raw_parts(ptr, 0, len));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_engine_version(out_ptr: *mut u8, out_cap: u32) -> i32 {
    write_out(ENGINE.as_bytes(), out_ptr, out_cap)
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_last_error(out_ptr: *mut u8, out_cap: u32) -> i32 {
    STATE.with(|s| write_out(s.borrow().last_error.as_bytes(), out_ptr, out_cap))
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_play_out(
    req_ptr: *const u8,
    req_len: u32,
    replay_ptr: *const u8,
    replay_len: u32,
    out_ptr: *mut u8,
    out_cap: u32,
) -> i32 {
    let req: PlayOutRequest = match serde_json::from_slice(read_slice(req_ptr, req_len)) {
        Ok(r) => r,
        Err(e) => return set_error(format!("bad PlayOutRequest: {e}")),
    };
    let replay = read_slice(replay_ptr, replay_len);
    match exec_play_out(&req, replay) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(bytes) => write_out(&bytes, out_ptr, out_cap),
            Err(e) => set_error(format!("encode PlayOutResponse: {e}")),
        },
        Err(e) => set_error(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_still(
    req_ptr: *const u8,
    req_len: u32,
    replay_ptr: *const u8,
    replay_len: u32,
    out_ptr: *mut u8,
    out_cap: u32,
) -> i32 {
    let req: StillRequest = match serde_json::from_slice(read_slice(req_ptr, req_len)) {
        Ok(r) => r,
        Err(e) => return set_error(format!("bad StillRequest: {e}")),
    };
    let replay = read_slice(replay_ptr, replay_len);
    match exec_still(&req, replay) {
        Ok(bytes) => write_out(&bytes, out_ptr, out_cap),
        Err(e) => set_error(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_open(
    req_ptr: *const u8,
    req_len: u32,
    replay_ptr: *const u8,
    replay_len: u32,
    out_ptr: *mut u8,
    out_cap: u32,
) -> i32 {
    let req: ViewerOpenRequest = match serde_json::from_slice(read_slice(req_ptr, req_len)) {
        Ok(r) => r,
        Err(e) => return set_error(format!("bad ViewerOpenRequest: {e}")),
    };
    let replay = read_slice(replay_ptr, replay_len);
    match exec_viewer_open(&req, replay) {
        Ok(resp) => match serde_json::to_vec(&resp) {
            Ok(bytes) => write_out(&bytes, out_ptr, out_cap),
            Err(e) => set_error(format!("encode ViewerOpenResponse: {e}")),
        },
        Err(e) => set_error(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_close(handle: u32) {
    STATE.with(|s| {
        s.borrow_mut().viewers.remove(&handle);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_advance(handle: u32, elapsed_ns: u64) -> u32 {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.viewers.get_mut(&handle).map_or(0, |v| v.viewer.advance(Duration::from_nanos(elapsed_ns)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_step_forward(handle: u32) -> i32 {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.viewers.get_mut(&handle).map_or(0, |v| v.viewer.step_forward() as i32)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_step_back(handle: u32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().viewers.get_mut(&handle) {
            v.viewer.step_back();
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_seek(handle: u32, tick: u32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().viewers.get_mut(&handle) {
            v.viewer.seek(tick);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_index(handle: u32, ticks: u32) -> i32 {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.viewers.get_mut(&handle).map_or(1, |v| v.viewer.index(ticks) as i32)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_set_paused(handle: u32, paused: u32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().viewers.get_mut(&handle) {
            v.viewer.set_paused(paused != 0);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_set_speed(handle: u32, speed: u32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().viewers.get_mut(&handle) {
            v.viewer.set_speed(speed_from_code(speed));
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_set_view(handle: u32, side: u32) -> i32 {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.viewers.get_mut(&handle).map_or(0, |v| v.viewer.set_view(side as u8) as i32)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_set_both(handle: u32, both: u32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().viewers.get_mut(&handle) {
            v.viewer.set_both(both != 0);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_set_sound_levels(handle: u32, music: f32, effects: f32) {
    STATE.with(|s| {
        if let Some(v) = s.borrow_mut().viewers.get_mut(&handle) {
            v.viewer.set_sound_levels(music, effects);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_set_language(handle: u32, lang_ptr: *const u8, lang_len: u32) -> i32 {
    let lang = match std::str::from_utf8(read_slice(lang_ptr, lang_len)) {
        Ok(l) => l.to_string(),
        Err(e) => return set_error(format!("invalid UTF-8 lang: {e}")),
    };
    let result: Result<(), String> = STATE.with(|s| {
        let mut st = s.borrow_mut();
        let Some(open) = st.viewers.get_mut(&handle) else { return Err("invalid viewer handle".to_string()) };
        let graphics = open.loaded.game.graphics(&lang).map_err(|e| format!("{e}: {}", e.report))?;
        open.viewer.set_language(&graphics);
        Ok(())
    });
    match result {
        Ok(()) => 0,
        Err(e) => set_error(e),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_state(handle: u32, out_ptr: *mut u8, out_cap: u32) -> i32 {
    let state = STATE.with(|s| {
        let mut st = s.borrow_mut();
        let open = st.viewers.get_mut(&handle)?;
        let inputs = open.viewer.battle().inputs;
        let held_inputs = [inputs[0].held & 0x3FF, inputs[1].held & 0x3FF];
        let p0 = party_snapshot(open, 0);
        let p1 = party_snapshot(open, 1);
        Some(ViewerState {
            position: open.viewer.position(),
            indexed: open.viewer.indexed(),
            whole: open.viewer.indexed() >= open.viewer.length() || open.viewer.stopped().is_some(),
            paused: open.viewer.paused(),
            speed: speed_to_code(open.viewer.speed()),
            view: open.viewer.view(),
            both: open.viewer.both(),
            at_end: open.viewer.at_end(),
            stopped: open.viewer.stopped().map(str::to_string),
            diverged: open.viewer.diverged().map(str::to_string),
            held_inputs,
            parties: [p0, p1],
        })
    });
    let Some(state) = state else {
        return set_error("invalid viewer handle");
    };
    match serde_json::to_vec(&state) {
        Ok(bytes) => write_out(&bytes, out_ptr, out_cap),
        Err(e) => set_error(format!("encode ViewerState: {e}")),
    }
}

/// Render the current frame into `out_ptr` as `width * height` little-endian `u32` pixels (`0x00RRGGBB`).
/// `side_or_current`: `-1` = current view (respecting `both`), `0` or `1` = `present_view(side)`.
#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_present(
    handle: u32,
    side_or_current: i32,
    width: u32,
    height: u32,
    out_ptr: *mut u8,
    out_cap: u32,
) -> i32 {
    let (w, h) = (width as usize, height as usize);
    let Some(needed_pixels) = w.checked_mul(h) else {
        return set_error("frame dimensions overflow");
    };
    let Some(needed_bytes) = needed_pixels.checked_mul(4) else {
        return set_error("frame byte size overflow");
    };
    if needed_bytes > out_cap as usize {
        return -((needed_bytes as i32).saturating_add(2));
    }
    let ok = STATE.with(|s| {
        let mut st = s.borrow_mut();
        let Some(open) = st.viewers.get_mut(&handle) else { return false };
        open.pixels_buf.resize(needed_pixels, 0);
        open.viewer.renderer().size = Size { width: w, height: h };
        if side_or_current < 0 {
            open.viewer.present(&mut open.pixels_buf, w, h);
        } else {
            open.viewer.present_view(side_or_current as u8, &mut open.pixels_buf, w, h);
        }
        if needed_bytes > 0 && !out_ptr.is_null() {
            // Safety: `out_ptr` has capacity >= `needed_bytes`, and `u32` on wasm32 is little-endian.
            unsafe {
                std::ptr::copy_nonoverlapping(open.pixels_buf.as_ptr() as *const u8, out_ptr, needed_bytes);
            }
        }
        true
    });
    if ok { needed_bytes as i32 } else { set_error("invalid viewer handle") }
}

/// Drain pending stereo `f32` PCM samples (`[L, R]` pairs, 8 bytes/frame) into `out_ptr`.
/// `side_or_current`: `-1` = `take_samples`, `0` or `1` = `take_samples_of(side)`.
#[unsafe(no_mangle)]
pub extern "C" fn saito_sim_viewer_take_samples(handle: u32, side_or_current: i32, out_ptr: *mut u8, out_cap: u32) -> i32 {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let Some(open) = st.viewers.get_mut(&handle) else {
            return set_error("invalid viewer handle");
        };
        if open.samples_buf.is_empty() {
            if side_or_current < 0 {
                open.viewer.take_samples(&mut open.samples_buf);
            } else {
                open.viewer.take_samples_of(side_or_current as u8, &mut open.samples_buf);
            }
        }
        let max_frames = (out_cap as usize) / 8;
        let take = open.samples_buf.len().min(max_frames);
        let bytes = take * 8;
        if bytes > 0 && !out_ptr.is_null() {
            // Safety: `[f32; 2]` has size 8 and no padding; little-endian on wasm32.
            unsafe {
                std::ptr::copy_nonoverlapping(open.samples_buf.as_ptr() as *const u8, out_ptr, bytes);
            }
            open.samples_buf.drain(..take);
        }
        bytes as i32
    })
}
