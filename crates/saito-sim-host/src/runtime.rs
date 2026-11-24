//! Wasmtime/Pulley host runtime for executing `saito-sim-<line>.wasm` modules in a strict read-only sandbox.

use std::path::{Path, PathBuf};
use std::time::Duration;

use saito_battle::BattleResult;
use saito_extract::RomSource;
use saito_frontend::game::Enhancements;
use saito_frontend::player::time_of;
use saito_frontend::replay::{End, Info, MatchKind, Outcome};
use saito_frontend::viewer::Speed;
use saito_sim_abi::{
    ABI_VERSION, EnhancementsConfig, PartySnapshot, PlayOutRequest, PlayOutResponse, RomMount, StillRequest,
    VFS_CONTENT, VFS_POOL_PREFIX, VFS_ROMS, ViewerOpenRequest, ViewerOpenResponse, ViewerState,
};

fn enhancements_config(e: Enhancements) -> EnhancementsConfig {
    EnhancementsConfig {
        text: e.text,
        hud: e.hud,
        motion: e.motion,
        fill: e.fill,
        reduce_flashes: e.reduce_flashes,
        hide_aura_number: e.hide_aura_number,
        hd_chip_art: e.hd_chip_art,
    }
}

fn end_of(code: u8) -> End {
    match code {
        0 => End::Set,
        2 => End::Cut,
        _ => End::Open,
    }
}

fn speed_of(code: u8) -> Speed {
    match code {
        0 => Speed::Half,
        1 => Speed::Normal,
        2 => Speed::Double,
        _ => Speed::Quadruple,
    }
}

fn speed_code(speed: Speed) -> u32 {
    match speed {
        Speed::Half => 0,
        Speed::Normal => 1,
        Speed::Double => 2,
        Speed::Quadruple => 3,
    }
}

fn result_of(code: u8) -> Option<BattleResult> {
    [
        BattleResult::Won,
        BattleResult::Lost,
        BattleResult::Drawn,
        BattleResult::Escaped,
        BattleResult::CommError,
        BattleResult::Terminated,
        BattleResult::TerminatedA,
    ]
    .into_iter()
    .find(|r| *r as u8 == code)
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use wasmtime::{Config, Engine, Instance, Linker, Memory, Module, Store, TypedFunc};
    use wasmtime_wasi::p1::WasiP1Ctx;
    use wasmtime_wasi::{FsPerms, WasiCtxBuilder};

    /// Shared Wasmtime engine + compiled module cache (safe to share across threads).
    pub struct WasmRuntime {
        engine: Engine,
        modules: Mutex<HashMap<PathBuf, Arc<Module>>>,
        /// Cached headless guest instances for `play_out`, keyed by `(kernel_path, content_dir, pool)`,
        /// so listing many legacy replays reuses the guest's loaded `Content` and Luau VM.
        play_out_pool: Mutex<HashMap<(PathBuf, PathBuf, Vec<(String, PathBuf)>), GuestInstance>>,
        pulley: bool,
    }

    impl WasmRuntime {
        /// Create a runtime configured for this platform (Cranelift JIT on desktop/Android,
        /// Pulley64 interpreter on iOS or when `SAITO_WASM_PULLEY=1`).
        pub fn new() -> Result<WasmRuntime, String> {
            let pulley = cfg!(target_os = "ios")
                || std::env::var_os("SAITO_WASM_PULLEY").is_some_and(|v| !v.is_empty() && v != "0");
            WasmRuntime::with_pulley(pulley)
        }

        /// Create a runtime explicitly selecting Cranelift JIT (`pulley = false`) or
        /// the portable Pulley64 interpreter (`pulley = true`).
        pub fn with_pulley(pulley: bool) -> Result<WasmRuntime, String> {
            let mut config = Config::new();
            config.wasm_exceptions(true);
            if pulley {
                config.target("pulley64").map_err(|e| format!("configuring pulley64 target: {e}"))?;
            }
            let engine = Engine::new(&config).map_err(|e| format!("initializing wasmtime engine: {e}"))?;
            Ok(WasmRuntime {
                engine,
                modules: Mutex::new(HashMap::new()),
                play_out_pool: Mutex::new(HashMap::new()),
                pulley,
            })
        }

        /// Whether this runtime executes modules using the Pulley64 bytecode interpreter.
        pub fn is_pulley(&self) -> bool {
            self.pulley
        }

        /// Compile (or return cached) `Module` from `.wasm` file `path`.
        fn module(&self, path: &Path) -> Result<Arc<Module>, String> {
            if let Some(m) = self.modules.lock().unwrap_or_else(|e| e.into_inner()).get(path).cloned() {
                return Ok(m);
            }
            let bytes = std::fs::read(path).map_err(|e| format!("reading {}: {e}", path.display()))?;
            let module = Arc::new(
                Module::new(&self.engine, &bytes).map_err(|e| format!("compiling {}: {e}", path.display()))?,
            );
            self.modules.lock().unwrap_or_else(|e| e.into_inner()).insert(path.to_path_buf(), module.clone());
            Ok(module)
        }

        /// Execute headless `play_out` of a replay inside kernel `kernel_path`.
        pub fn play_out(
            &self,
            kernel_path: &Path,
            game: &str,
            content_dir: &Path,
            pool: &[(String, PathBuf)],
            replay_bytes: &[u8],
        ) -> Result<Outcome, String> {
            let module = self.module(kernel_path)?;
            let (mounts, vfs_pool, _) = build_mounts(content_dir, pool, None)?;
            let cache_key = (kernel_path.to_path_buf(), content_dir.to_path_buf(), pool.to_vec());
            let mut guest = match self.play_out_pool.lock().unwrap_or_else(|e| e.into_inner()).remove(&cache_key) {
                Some(g) => g,
                None => GuestInstance::new(&self.engine, &module, &mounts)?,
            };
            let req = PlayOutRequest {
                game: game.to_string(),
                content_dir: VFS_CONTENT.to_string(),
                pool: vfs_pool,
            };
            let req_bytes = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
            let out = guest.call_with_req_and_replay("saito_sim_play_out", &req_bytes, replay_bytes);
            let out = match out {
                Ok(bytes) => {
                    // Return healthy guest instance to pool so subsequent replays skip content boot.
                    let mut guard = self.play_out_pool.lock().unwrap_or_else(|e| e.into_inner());
                    if guard.len() >= 4 {
                        guard.clear();
                    }
                    guard.insert(cache_key, guest);
                    bytes
                }
                Err(e) => return Err(e),
            };
            let resp: PlayOutResponse = serde_json::from_slice(&out).map_err(|e| format!("decode PlayOutResponse: {e}"))?;
            Ok(Outcome {
                ticks: resp.ticks,
                rounds: resp.rounds,
                result: resp.result.and_then(result_of),
                end: end_of(resp.end),
                diverged: resp.diverged,
                stopped: resp.stopped,
            })
        }

        /// Render a 240×160 still (`0x00RRGGBB` pixels) inside kernel `kernel_path`.
        #[allow(clippy::too_many_arguments)]
        pub fn still(
            &self,
            kernel_path: &Path,
            game: &str,
            content_dir: &Path,
            pool: &[(String, PathBuf)],
            roms: &RomSource,
            side: u8,
            lang: &str,
            enhancements: Enhancements,
            replay_bytes: &[u8],
        ) -> Result<Vec<u32>, String> {
            let module = self.module(kernel_path)?;
            let (mounts, vfs_pool, rom_mount) = build_mounts(content_dir, pool, Some(roms))?;
            let mut guest = GuestInstance::new(&self.engine, &module, &mounts)?;
            let req = StillRequest {
                game: game.to_string(),
                content_dir: VFS_CONTENT.to_string(),
                pool: vfs_pool,
                roms: rom_mount.expect("roms mounted"),
                side,
                lang: lang.to_string(),
                enhancements: EnhancementsConfig { motion: false, ..enhancements_config(enhancements) },
            };
            let req_bytes = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
            let out = guest.call_with_req_and_replay("saito_sim_still", &req_bytes, replay_bytes)?;
            if out.len() != 240 * 160 * 4 {
                return Err(format!("unexpected still byte length: {}", out.len()));
            }
            let pixels = out.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
            Ok(pixels)
        }

        /// Open an interactive [`WasmViewer`] for a replay inside kernel `kernel_path`.
        #[allow(clippy::too_many_arguments)]
        pub fn open_viewer(
            &self,
            kernel_path: &Path,
            game: &str,
            content_dir: &Path,
            pool: &[(String, PathBuf)],
            roms: &RomSource,
            view: Option<u8>,
            lang: &str,
            enhancements: Enhancements,
            want_sound: bool,
            replay_bytes: &[u8],
        ) -> Result<WasmViewer, String> {
            let module = self.module(kernel_path)?;
            let (mounts, vfs_pool, rom_mount) = build_mounts(content_dir, pool, Some(roms))?;
            let mut guest = GuestInstance::new(&self.engine, &module, &mounts)?;
            let req = ViewerOpenRequest {
                game: game.to_string(),
                content_dir: VFS_CONTENT.to_string(),
                pool: vfs_pool,
                roms: rom_mount.expect("roms mounted"),
                view,
                lang: lang.to_string(),
                enhancements: enhancements_config(enhancements),
                want_sound,
            };
            let req_bytes = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
            let out = guest.call_with_req_and_replay("saito_sim_viewer_open", &req_bytes, replay_bytes)?;
            let meta: ViewerOpenResponse =
                serde_json::from_slice(&out).map_err(|e| format!("decode ViewerOpenResponse: {e}"))?;
            let info = Info {
                when: meta.when,
                side: meta.side,
                kind: if meta.kind == 1 { MatchKind::Online } else { MatchKind::Offline },
                open_setups: meta.open_setups,
                names: meta.names,
                presentation: String::new(),
                ruleset: String::new(),
            };
            let mut viewer = WasmViewer {
                guest,
                handle: meta.handle,
                length: meta.length,
                rounds: meta.rounds,
                end: end_of(meta.end),
                engine: meta.engine,
                info,
                has_sound: meta.has_sound,
                state: ViewerState::default(),
                out_scratch: Vec::new(),
            };
            viewer.refresh_state();
            Ok(viewer)
        }
    }

    type Mount = (PathBuf, String);

    fn build_mounts(
        content_dir: &Path,
        pool: &[(String, PathBuf)],
        roms: Option<&RomSource>,
    ) -> Result<(Vec<Mount>, Vec<(String, String)>, Option<RomMount>), String> {
        let mut mounts: Vec<Mount> = vec![(content_dir.to_path_buf(), VFS_CONTENT.to_string())];
        let mut vfs_pool = Vec::with_capacity(pool.len());
        for (game, dir) in pool {
            let guest_dir = format!("{VFS_POOL_PREFIX}/{game}");
            mounts.push((dir.clone(), guest_dir.clone()));
            vfs_pool.push((game.clone(), guest_dir));
        }
        let rom_mount = match roms {
            None => None,
            Some(RomSource::Collection(install)) => {
                if let Some(vol2) = install.vol2_dat()
                    && let Some(vol2_dir) = vol2.parent()
                {
                    mounts.push((install.data.clone(), format!("{VFS_ROMS}/exe/data")));
                    mounts.push((vol2_dir.to_path_buf(), format!("{VFS_ROMS}/launcher/data")));
                } else {
                    mounts.push((install.data.clone(), VFS_ROMS.to_string()));
                }
                Some(RomMount::Collection { dir: VFS_ROMS.to_string() })
            }
            Some(RomSource::Copies(dir)) => {
                mounts.push((dir.clone(), VFS_ROMS.to_string()));
                Some(RomMount::Copies { dir: VFS_ROMS.to_string() })
            }
            Some(RomSource::Files(files)) => {
                let mut parent_map: HashMap<PathBuf, String> = HashMap::new();
                let mut guest_files = Vec::with_capacity(files.len());
                for f in files {
                    let parent = f.parent().ok_or_else(|| format!("ROM path {} has no parent", f.display()))?;
                    let name = f.file_name().and_then(|n| n.to_str()).ok_or_else(|| format!("invalid ROM file name {}", f.display()))?;
                    let guest_parent = if let Some(existing) = parent_map.get(parent) {
                        existing.clone()
                    } else {
                        let gp = format!("{VFS_ROMS}/{}", parent_map.len());
                        mounts.push((parent.to_path_buf(), gp.clone()));
                        parent_map.insert(parent.to_path_buf(), gp.clone());
                        gp
                    };
                    guest_files.push(format!("{guest_parent}/{name}"));
                }
                Some(RomMount::Files { paths: guest_files })
            }
        };
        Ok((mounts, vfs_pool, rom_mount))
    }

    struct GuestInstance {
        store: Store<WasiP1Ctx>,
        instance: Instance,
        memory: Memory,
        alloc: TypedFunc<u32, u32>,
        dealloc: TypedFunc<(u32, u32), ()>,
        last_error: TypedFunc<(u32, u32), i32>,
    }

    impl GuestInstance {
        fn new(engine: &Engine, module: &Module, mounts: &[Mount]) -> Result<GuestInstance, String> {
            let mut wasi_builder = WasiCtxBuilder::new();
            for (host_dir, guest_dir) in mounts {
                wasi_builder
                    .preopened_dir(host_dir, guest_dir, FsPerms::ReadOnly)
                    .map_err(|e| format!("preopening {} as {guest_dir}: {e}", host_dir.display()))?;
            }
            let wasi = wasi_builder.build_p1();
            let mut store = Store::new(engine, wasi);
            let mut linker: Linker<WasiP1Ctx> = Linker::new(engine);
            wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |ctx| ctx)
                .map_err(|e| format!("linking wasi p1: {e}"))?;
            let instance = linker
                .instantiate(&mut store, module)
                .map_err(|e| format!("instantiating kernel: {e}"))?;
            if let Ok(init) = instance.get_typed_func::<(), ()>(&mut store, "_initialize") {
                init.call(&mut store, ()).map_err(|e| format!("running _initialize: {e}"))?;
            }
            let abi: TypedFunc<(), u32> = instance
                .get_typed_func(&mut store, "saito_sim_abi_version")
                .map_err(|e| format!("missing saito_sim_abi_version: {e}"))?;
            let got_abi = abi.call(&mut store, ()).map_err(|e| format!("calling saito_sim_abi_version: {e}"))?;
            if got_abi != ABI_VERSION {
                return Err(format!("unsupported kernel ABI version {got_abi} (expected {ABI_VERSION})"));
            }
            let memory = instance
                .get_memory(&mut store, "memory")
                .ok_or_else(|| "kernel missing exported linear memory".to_string())?;
            let alloc = instance.get_typed_func(&mut store, "saito_sim_alloc").map_err(|e| e.to_string())?;
            let dealloc = instance.get_typed_func(&mut store, "saito_sim_dealloc").map_err(|e| e.to_string())?;
            let last_error = instance.get_typed_func(&mut store, "saito_sim_last_error").map_err(|e| e.to_string())?;
            Ok(GuestInstance { store, instance, memory, alloc, dealloc, last_error })
        }

        fn write_bytes(&mut self, data: &[u8]) -> Result<(u32, u32), String> {
            let len = data.len() as u32;
            let ptr = self.alloc.call(&mut self.store, len).map_err(|e| e.to_string())?;
            if !data.is_empty() {
                self.memory.write(&mut self.store, ptr as usize, data).map_err(|e| e.to_string())?;
            }
            Ok((ptr, len))
        }

        fn free_bytes(&mut self, ptr: u32, len: u32) {
            let _ = self.dealloc.call(&mut self.store, (ptr, len));
        }

        fn read_last_error(&mut self) -> String {
            let mut cap = 1024u32;
            for _ in 0..2 {
                let Ok(ptr) = self.alloc.call(&mut self.store, cap) else { return "guest error".into() };
                let n = self.last_error.call(&mut self.store, (ptr, cap)).unwrap_or(0);
                if n >= 0 {
                    let mut buf = vec![0u8; n as usize];
                    let _ = self.memory.read(&self.store, ptr as usize, &mut buf);
                    self.free_bytes(ptr, cap);
                    return String::from_utf8_lossy(&buf).into_owned();
                }
                self.free_bytes(ptr, cap);
                if n < -1 {
                    cap = (-n - 2) as u32;
                } else {
                    break;
                }
            }
            "guest error".into()
        }

        fn call_output(&mut self, initial_cap: u32, mut f: impl FnMut(&mut Self, u32, u32) -> Result<i32, String>) -> Result<Vec<u8>, String> {
            let mut cap = initial_cap.max(64);
            for _ in 0..3 {
                let out_ptr = self.alloc.call(&mut self.store, cap).map_err(|e| e.to_string())?;
                let res = f(self, out_ptr, cap);
                match res {
                    Ok(n) if n >= 0 => {
                        let mut buf = vec![0u8; n as usize];
                        if n > 0 {
                            self.memory.read(&self.store, out_ptr as usize, &mut buf).map_err(|e| e.to_string())?;
                        }
                        self.free_bytes(out_ptr, cap);
                        return Ok(buf);
                    }
                    Ok(-1) => {
                        self.free_bytes(out_ptr, cap);
                        return Err(self.read_last_error());
                    }
                    Ok(neg) => {
                        self.free_bytes(out_ptr, cap);
                        cap = (-neg - 2) as u32;
                    }
                    Err(e) => {
                        self.free_bytes(out_ptr, cap);
                        return Err(e);
                    }
                }
            }
            Err("guest output buffer retry limit exceeded".into())
        }

        fn call_with_req_and_replay(&mut self, func_name: &str, req: &[u8], replay: &[u8]) -> Result<Vec<u8>, String> {
            let func: TypedFunc<(u32, u32, u32, u32, u32, u32), i32> =
                self.instance.get_typed_func(&mut self.store, func_name).map_err(|e| e.to_string())?;
            let (req_ptr, req_len) = self.write_bytes(req)?;
            let (rep_ptr, rep_len) = self.write_bytes(replay)?;
            let result = self.call_output(4096, |this, out_ptr, out_cap| {
                func.call(&mut this.store, (req_ptr, req_len, rep_ptr, rep_len, out_ptr, out_cap))
                    .map_err(|e| format!("{func_name} trapped: {e}"))
            });
            self.free_bytes(rep_ptr, rep_len);
            self.free_bytes(req_ptr, req_len);
            result
        }
    }

    /// An open replay viewer running inside a sandboxed historical WASM simulation kernel.
    pub struct WasmViewer {
        guest: GuestInstance,
        handle: u32,
        length: u32,
        rounds: Vec<u32>,
        end: End,
        engine: String,
        info: Info,
        has_sound: bool,
        state: ViewerState,
        out_scratch: Vec<u8>,
    }

    impl Drop for WasmViewer {
        fn drop(&mut self) {
            if let Ok(close) = self.guest.instance.get_typed_func::<u32, ()>(&mut self.guest.store, "saito_sim_viewer_close") {
                let _ = close.call(&mut self.guest.store, self.handle);
            }
        }
    }

    impl WasmViewer {
        fn refresh_state(&mut self) {
            let handle = self.handle;
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32, u32), i32>(&mut self.guest.store, "saito_sim_viewer_state")
            else {
                return;
            };
            if let Ok(bytes) = self.guest.call_output(1024, |g, out_ptr, out_cap| {
                func.call(&mut g.store, (handle, out_ptr, out_cap)).map_err(|e| e.to_string())
            }) && let Ok(state) = serde_json::from_slice::<ViewerState>(&bytes)
            {
                self.state = state;
            }
        }

        pub fn length(&self) -> u32 {
            self.length
        }

        pub fn duration(&self) -> Duration {
            time_of(self.length as u64)
        }

        pub fn rounds(&self) -> &[u32] {
            &self.rounds
        }

        pub fn round_at(&self, tick: u32) -> u32 {
            self.rounds.partition_point(|&start| start <= tick).max(1) as u32
        }

        pub fn end(&self) -> End {
            self.end
        }

        pub fn info(&self) -> &Info {
            &self.info
        }

        pub fn engine(&self) -> &str {
            &self.engine
        }

        pub fn has_sound(&self) -> bool {
            self.has_sound
        }

        pub fn position(&self) -> u32 {
            self.state.position
        }

        pub fn indexed(&self) -> u32 {
            self.state.indexed
        }

        pub fn at_end(&self) -> bool {
            self.state.at_end
        }

        pub fn paused(&self) -> bool {
            self.state.paused
        }

        pub fn speed(&self) -> Speed {
            speed_of(self.state.speed)
        }

        pub fn view(&self) -> u8 {
            self.state.view
        }

        pub fn both(&self) -> bool {
            self.state.both
        }

        pub fn stopped(&self) -> Option<&str> {
            self.state.stopped.as_deref()
        }

        pub fn diverged(&self) -> Option<&str> {
            self.state.diverged.as_deref()
        }

        pub fn held_inputs(&self) -> [u16; 2] {
            self.state.held_inputs
        }

        pub fn parties(&self) -> &[Option<PartySnapshot>; 2] {
            &self.state.parties
        }

        pub fn advance(&mut self, elapsed: Duration) -> u32 {
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u64), u32>(&mut self.guest.store, "saito_sim_viewer_advance")
            else {
                return 0;
            };
            let ran = func.call(&mut self.guest.store, (self.handle, elapsed.as_nanos() as u64)).unwrap_or(0);
            if ran > 0 {
                self.refresh_state();
            }
            ran
        }

        pub fn step_forward(&mut self) -> bool {
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<u32, i32>(&mut self.guest.store, "saito_sim_viewer_step_forward")
            else {
                return false;
            };
            let stepped = func.call(&mut self.guest.store, self.handle).unwrap_or(0) != 0;
            self.refresh_state();
            stepped
        }

        pub fn step_back(&mut self) {
            if let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<u32, ()>(&mut self.guest.store, "saito_sim_viewer_step_back")
            {
                let _ = func.call(&mut self.guest.store, self.handle);
                self.refresh_state();
            }
        }

        pub fn seek(&mut self, tick: u32) {
            if let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32), ()>(&mut self.guest.store, "saito_sim_viewer_seek")
            {
                let _ = func.call(&mut self.guest.store, (self.handle, tick));
                self.refresh_state();
            }
        }

        pub fn index(&mut self, ticks: u32) -> bool {
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32), i32>(&mut self.guest.store, "saito_sim_viewer_index")
            else {
                return true;
            };
            let whole = func.call(&mut self.guest.store, (self.handle, ticks)).unwrap_or(1) != 0;
            self.refresh_state();
            whole
        }

        pub fn set_paused(&mut self, paused: bool) {
            if let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32), ()>(&mut self.guest.store, "saito_sim_viewer_set_paused")
            {
                let _ = func.call(&mut self.guest.store, (self.handle, paused as u32));
                self.state.paused = paused;
            }
        }

        pub fn set_speed(&mut self, speed: Speed) {
            if let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32), ()>(&mut self.guest.store, "saito_sim_viewer_set_speed")
            {
                let code = speed_code(speed);
                let _ = func.call(&mut self.guest.store, (self.handle, code));
                self.state.speed = code as u8;
            }
        }

        pub fn set_view(&mut self, side: u8) -> bool {
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32), i32>(&mut self.guest.store, "saito_sim_viewer_set_view")
            else {
                return false;
            };
            let ok = func.call(&mut self.guest.store, (self.handle, side as u32)).unwrap_or(0) != 0;
            if ok {
                self.refresh_state();
            }
            ok
        }

        pub fn set_both(&mut self, both: bool) {
            if let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32), ()>(&mut self.guest.store, "saito_sim_viewer_set_both")
            {
                let _ = func.call(&mut self.guest.store, (self.handle, both as u32));
                self.state.both = both;
            }
        }

        pub fn set_sound_levels(&mut self, music: f32, effects: f32) {
            if let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, f32, f32), ()>(&mut self.guest.store, "saito_sim_viewer_set_sound_levels")
            {
                let _ = func.call(&mut self.guest.store, (self.handle, music, effects));
            }
        }

        pub fn set_language(&mut self, lang: &str) {
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, u32, u32), i32>(&mut self.guest.store, "saito_sim_viewer_set_language")
            else {
                return;
            };
            if let Ok((ptr, len)) = self.guest.write_bytes(lang.as_bytes()) {
                let _ = func.call(&mut self.guest.store, (self.handle, ptr, len));
                self.guest.free_bytes(ptr, len);
            }
        }

        fn present_inner(&mut self, side_or_current: i32, buffer: &mut [u32], width: usize, height: usize) {
            let needed = width.saturating_mul(height).saturating_mul(4);
            if needed == 0 || buffer.len() < width * height {
                return;
            }
            let handle = self.handle;
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, i32, u32, u32, u32, u32), i32>(&mut self.guest.store, "saito_sim_viewer_present")
            else {
                return;
            };
            let Ok(out_ptr) = self.guest.alloc.call(&mut self.guest.store, needed as u32) else { return };
            let n = func
                .call(&mut self.guest.store, (handle, side_or_current, width as u32, height as u32, out_ptr, needed as u32))
                .unwrap_or(-1);
            if n == needed as i32 {
                self.out_scratch.resize(needed, 0);
                if self.guest.memory.read(&self.guest.store, out_ptr as usize, &mut self.out_scratch).is_ok() {
                    for (dst, src) in buffer.iter_mut().zip(self.out_scratch.chunks_exact(4)) {
                        *dst = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
                    }
                }
            }
            self.guest.free_bytes(out_ptr, needed as u32);
        }

        pub fn present(&mut self, buffer: &mut [u32], width: usize, height: usize) {
            self.present_inner(-1, buffer, width, height);
        }

        pub fn present_view(&mut self, side: u8, buffer: &mut [u32], width: usize, height: usize) {
            self.present_inner(side as i32, buffer, width, height);
        }

        fn take_samples_inner(&mut self, side_or_current: i32, out: &mut Vec<[f32; 2]>) {
            let Ok(func) = self
                .guest
                .instance
                .get_typed_func::<(u32, i32, u32, u32), i32>(&mut self.guest.store, "saito_sim_viewer_take_samples")
            else {
                return;
            };
            const CHUNK_BYTES: u32 = 16_384 * 8;
            let Ok(out_ptr) = self.guest.alloc.call(&mut self.guest.store, CHUNK_BYTES) else { return };
            self.out_scratch.resize(CHUNK_BYTES as usize, 0);
            loop {
                let n = func
                    .call(&mut self.guest.store, (self.handle, side_or_current, out_ptr, CHUNK_BYTES))
                    .unwrap_or(0);
                if n <= 0 {
                    break;
                }
                let bytes = n as usize;
                if self.guest.memory.read(&self.guest.store, out_ptr as usize, &mut self.out_scratch[..bytes]).is_err() {
                    break;
                }
                for frame in self.out_scratch[..bytes].chunks_exact(8) {
                    let l = f32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]);
                    let r = f32::from_le_bytes([frame[4], frame[5], frame[6], frame[7]]);
                    out.push([l, r]);
                }
                if (n as u32) < CHUNK_BYTES {
                    break;
                }
            }
            self.guest.free_bytes(out_ptr, CHUNK_BYTES);
        }

        pub fn take_samples(&mut self, out: &mut Vec<[f32; 2]>) {
            self.take_samples_inner(-1, out);
        }

        pub fn take_samples_of(&mut self, side: u8, out: &mut Vec<[f32; 2]>) {
            self.take_samples_inner(side as i32, out);
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use super::*;

    /// Browser runtime placeholder: on `wasm32`, replays fall back to lenient mode when a native
    /// kernel is not instantiated via JS WebAssembly.
    pub struct WasmRuntime;

    impl WasmRuntime {
        pub fn new() -> Result<WasmRuntime, String> {
            Ok(WasmRuntime)
        }

        pub fn with_pulley(_pulley: bool) -> Result<WasmRuntime, String> {
            Ok(WasmRuntime)
        }

        pub fn is_pulley(&self) -> bool {
            false
        }

        pub fn play_out(
            &self,
            _kernel_path: &Path,
            _game: &str,
            _content_dir: &Path,
            _pool: &[(String, PathBuf)],
            _replay_bytes: &[u8],
        ) -> Result<Outcome, String> {
            Err("nested WASM sim kernels are not yet wired in the browser build".into())
        }

        #[allow(clippy::too_many_arguments)]
        pub fn still(
            &self,
            _kernel_path: &Path,
            _game: &str,
            _content_dir: &Path,
            _pool: &[(String, PathBuf)],
            _roms: &RomSource,
            _side: u8,
            _lang: &str,
            _enhancements: Enhancements,
            _replay_bytes: &[u8],
        ) -> Result<Vec<u32>, String> {
            Err("nested WASM sim kernels are not yet wired in the browser build".into())
        }

        #[allow(clippy::too_many_arguments)]
        pub fn open_viewer(
            &self,
            _kernel_path: &Path,
            _game: &str,
            _content_dir: &Path,
            _pool: &[(String, PathBuf)],
            _roms: &RomSource,
            _view: Option<u8>,
            _lang: &str,
            _enhancements: Enhancements,
            _want_sound: bool,
            _replay_bytes: &[u8],
        ) -> Result<WasmViewer, String> {
            Err("nested WASM sim kernels are not yet wired in the browser build".into())
        }
    }

    pub struct WasmViewer {
        info: Info,
        parties: [Option<PartySnapshot>; 2],
    }

    impl WasmViewer {
        pub fn length(&self) -> u32 { 0 }
        pub fn duration(&self) -> Duration { Duration::ZERO }
        pub fn rounds(&self) -> &[u32] { &[] }
        pub fn round_at(&self, _tick: u32) -> u32 { 1 }
        pub fn end(&self) -> End { End::Open }
        pub fn info(&self) -> &Info { &self.info }
        pub fn engine(&self) -> &str { "" }
        pub fn has_sound(&self) -> bool { false }
        pub fn position(&self) -> u32 { 0 }
        pub fn indexed(&self) -> u32 { 0 }
        pub fn at_end(&self) -> bool { true }
        pub fn paused(&self) -> bool { true }
        pub fn speed(&self) -> Speed { Speed::Normal }
        pub fn view(&self) -> u8 { 0 }
        pub fn both(&self) -> bool { false }
        pub fn stopped(&self) -> Option<&str> { None }
        pub fn diverged(&self) -> Option<&str> { None }
        pub fn held_inputs(&self) -> [u16; 2] { [0, 0] }
        pub fn parties(&self) -> &[Option<PartySnapshot>; 2] { &self.parties }
        pub fn advance(&mut self, _elapsed: Duration) -> u32 { 0 }
        pub fn step_forward(&mut self) -> bool { false }
        pub fn step_back(&mut self) {}
        pub fn seek(&mut self, _tick: u32) {}
        pub fn index(&mut self, _ticks: u32) -> bool { true }
        pub fn set_paused(&mut self, _paused: bool) {}
        pub fn set_speed(&mut self, _speed: Speed) {}
        pub fn set_view(&mut self, _side: u8) -> bool { false }
        pub fn set_both(&mut self, _both: bool) {}
        pub fn set_sound_levels(&mut self, _music: f32, _effects: f32) {}
        pub fn set_language(&mut self, _lang: &str) {}
        pub fn present(&mut self, _buffer: &mut [u32], _width: usize, _height: usize) {}
        pub fn present_view(&mut self, _side: u8, _buffer: &mut [u32], _width: usize, _height: usize) {}
        pub fn take_samples(&mut self, _out: &mut Vec<[f32; 2]>) {}
        pub fn take_samples_of(&mut self, _side: u8, _out: &mut Vec<[f32; 2]>) {}
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{WasmRuntime, WasmViewer};
#[cfg(target_arch = "wasm32")]
pub use web::{WasmRuntime, WasmViewer};
