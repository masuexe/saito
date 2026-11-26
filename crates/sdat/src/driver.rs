//! Real-time Nintendo DS NitroComposer (`SSEQ`) sequencer and synthesizer.
//!
//! Renders stereo `f32` audio at [`crate::OUT_RATE`] (`32768 Hz`) while stepping
//! the NitroComposer driver at its native ~192 Hz timer (`33513982 / 64 / 2728`).

use crate::bank::{NoteDef, NoteKind, SongId, SoundBank};
use crate::tables::{DECIBEL_TABLE, ENV_SILENCE, attack_coeff, db_to_gain, decay_rate, lfo_sin};
use crate::OUT_RATE;
use std::sync::Arc;

/// Exact NDS NitroComposer hardware timer tick frequency (Hz):
/// `ARM7_CLOCK / (64 * 2728) = 33_513_982 / 174_592 ≈ 191.959326 Hz`.
const DRIVER_TICK_RATE: f64 = 33_513_982.0 / (64.0 * 2728.0);
/// Driver ticks per output sample at 32768 Hz.
const DRIVER_TICKS_PER_SAMPLE: f64 = DRIVER_TICK_RATE / OUT_RATE as f64;
/// GBA frame cycles (matching `m4a::apu::FRAME_CYCLES` = 280896; 512 cycles/sample -> 548.625 samples/frame).
const FRAME_CYCLES: u32 = 280_896;
const CYCLES_PER_SAMPLE: u32 = 512;

/// Maximum tracks per SSEQ sequence (hardware NitroComposer limit).
const MAX_TRACKS: usize = 16;
/// Virtual hardware channels (32 avoids voice-stealing dropouts on dense 10-11 track BGM).
const MAX_CHANNELS: usize = 32;
/// Call / loop nesting depth per track.
const STACK_DEPTH: usize = 3;

/// Master output calibration scalar so NDS SSEQ loudness sits evenly alongside
/// GBA `m4a` BGM and SFX (`~0.12..0.18` RMS, peaks safely under `0.90`).
const MASTER_GAIN: f32 = 0.65;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnvPhase {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

#[derive(Clone, Debug)]
struct Channel {
    active: bool,
    track: u8,
    priority: u8,
    note_key: u8,
    velocity: u8,
    duration: i32,
    auto_release: bool,
    bank_id: u16,
    note: NoteDef,
    /// Current sample position (floating point for linear interpolation).
    pos: f64,
    /// Hardware LFSR state for noise instruments.
    lfsr: u16,
    env_phase: EnvPhase,
    /// Envelope attenuation in `1/128` of `0.1 dB` (`0` = max, `ENV_SILENCE` = off).
    env_level: i32,
    /// Modulation (LFO) state per channel.
    lfo_delay: u16,
    lfo_phase: u16,
    /// Pitch sweep interpolation state.
    sweep_counter: i32,
    sweep_len: i32,
    sweep_pitch: i32,
    /// Monotonic birth counter for oldest-voice stealing.
    age: u64,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            active: false,
            track: 0,
            priority: 64,
            note_key: 60,
            velocity: 127,
            duration: 0,
            auto_release: true,
            bank_id: 0,
            note: NoteDef::default(),
            pos: 0.0,
            lfsr: 0x7FFF,
            env_phase: EnvPhase::Off,
            env_level: ENV_SILENCE,
            lfo_delay: 0,
            lfo_phase: 0,
            sweep_counter: 0,
            sweep_len: 0,
            sweep_pitch: 0,
            age: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct Track {
    active: bool,
    pc: usize,
    wait: i32,
    program: u16,
    volume: u8,
    expression: u8,
    pan: u8,
    pitch_bend: i8,
    pitch_bend_range: u8,
    transpose: i8,
    priority: u8,
    note_wait: bool,
    tie: bool,
    mod_depth: u8,
    mod_speed: u8,
    mod_type: u8,
    mod_range: u8,
    mod_delay: u16,
    porta_on: bool,
    porta_key: u8,
    porta_time: u8,
    sweep_pitch: i16,
    /// Optional per-track ADSR overrides (`None` = use instrument's value).
    attack_override: Option<u8>,
    decay_override: Option<u8>,
    sustain_override: Option<u8>,
    release_override: Option<u8>,
    call_stack: [usize; STACK_DEPTH],
    call_sp: usize,
    loop_stack: [(usize, u8); STACK_DEPTH],
    loop_sp: usize,
    cmp_flag: bool,
    /// Last channel allocated by this track (for `tie` / note-wait tracking).
    last_channel: Option<usize>,
}

impl Default for Track {
    fn default() -> Self {
        Self {
            active: false,
            pc: 0,
            wait: 0,
            program: 0,
            volume: 127,
            expression: 127,
            pan: 64,
            pitch_bend: 0,
            pitch_bend_range: 2,
            transpose: 0,
            priority: 64,
            note_wait: true,
            tie: false,
            mod_depth: 0,
            mod_speed: 16,
            mod_type: 0,
            mod_range: 1,
            mod_delay: 0,
            porta_on: false,
            porta_key: 60,
            porta_time: 0,
            sweep_pitch: 0,
            attack_override: None,
            decay_override: None,
            sustain_override: None,
            release_override: None,
            call_stack: [0; STACK_DEPTH],
            call_sp: 0,
            loop_stack: [(0, 0); STACK_DEPTH],
            loop_sp: 0,
            cmp_flag: true,
            last_channel: None,
        }
    }
}

/// Pure-Rust NitroComposer player driving one `SDAT` [`SoundBank`].
#[derive(Clone)]
pub struct Driver {
    bank: Arc<SoundBank>,
    song: Option<SongId>,
    playing: bool,
    /// Sequence tempo in BPM (default 120).
    tempo: u16,
    /// Tempo accumulator (steps sequence each time `>= 240`).
    tempo_acc: u32,
    /// Sequence master volume (`0xC2`, default 127).
    master_volume: u8,
    /// Song record volume from SDAT `INFO` (`0..=127`).
    song_volume: u8,
    /// Instrument bank ID for the current song.
    bank_id: u16,
    /// Shared sequence variables (16 local + 16 global).
    vars: [i16; 32],
    /// External controls (matching `m4a` cue controls):
    /// `tempo_ratio` (`0x100` = 1.0x), `pitch_shift` (1/256 semitone),
    /// `cue_volume` (`0x100` = full), `host_level` (`0x100` = full).
    tempo_ratio: u16,
    pitch_shift: i16,
    cue_volume: u16,
    host_level: u16,
    /// Fractional NitroComposer 192 Hz driver-tick accumulator.
    tick_acc: f64,
    /// Fractional GBA-frame sample-count accumulator (for standalone `step_frame`).
    cycle_acc: u32,
    /// Deterministic PRNG for `0xA0` random prefix.
    rng: u32,
    age_counter: u64,
    tracks: [Track; MAX_TRACKS],
    channels: [Channel; MAX_CHANNELS],
    output: Vec<[f32; 2]>,
}

impl Driver {
    pub fn new(bank: Arc<SoundBank>) -> Self {
        Self {
            bank,
            song: None,
            playing: false,
            tempo: 120,
            tempo_acc: 0,
            master_volume: 127,
            song_volume: 127,
            bank_id: 0,
            vars: [-1; 32],
            tempo_ratio: 0x100,
            pitch_shift: 0,
            cue_volume: 0x100,
            host_level: 0x100,
            tick_acc: 0.0,
            cycle_acc: 0,
            rng: 0x1234_5678,
            age_counter: 0,
            tracks: std::array::from_fn(|_| Track::default()),
            channels: std::array::from_fn(|_| Channel::default()),
            output: Vec::new(),
        }
    }

    pub fn bank(&self) -> &Arc<SoundBank> {
        &self.bank
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn song(&self) -> Option<SongId> {
        self.playing.then_some(self.song).flatten()
    }

    /// Start playing `id` from the beginning (resetting tempo/pitch/cue_volume
    /// to normal, matching `m4aSongNumStart`).
    pub fn start(&mut self, id: SongId) {
        self.stop_all();
        let Some(song) = self.bank.song(id) else {
            return;
        };
        self.song = Some(id);
        self.playing = true;
        self.tempo = 120;
        self.tempo_acc = 240; // step tick 0 immediately on first driver tick
        self.master_volume = 127;
        self.song_volume = song.volume;
        self.bank_id = song.bank;
        self.vars = [-1; 32];
        self.tempo_ratio = 0x100;
        self.pitch_shift = 0;
        self.cue_volume = 0x100;
        self.tick_acc = 0.0;
        for t in &mut self.tracks {
            *t = Track::default();
        }
        self.tracks[0].active = true;
        self.tracks[0].priority = song.channel_priority;
    }

    /// Stop `id` if it is the song currently playing.
    pub fn stop_song(&mut self, id: SongId) {
        if self.playing && self.song == Some(id) {
            self.stop_all();
        }
    }

    /// Stop playback immediately and silence all channels.
    pub fn stop_all(&mut self) {
        self.playing = false;
        self.song = None;
        for t in &mut self.tracks {
            t.active = false;
        }
        for ch in &mut self.channels {
            ch.active = false;
            ch.env_phase = EnvPhase::Off;
        }
    }

    /// External tempo multiplier (`0x100` = 1.0x, `0x11A` = pinch).
    pub fn set_tempo(&mut self, tempo: u16) {
        self.tempo_ratio = tempo;
    }

    /// External pitch offset in `1/256` semitone (`0x100` = +1 semitone for pinch).
    pub fn set_pitch(&mut self, pitch: i16) {
        self.pitch_shift = pitch;
    }

    /// External cue volume (`0x100` = full, e.g. custom screen volume ducking).
    pub fn set_volume(&mut self, volume: u16) {
        self.cue_volume = volume;
    }

    /// Host mixer level (`0..=0x100`, user music volume slider).
    pub fn set_level(&mut self, level: u16) {
        self.host_level = level.min(0x100);
    }

    /// Advance by one GBA battle frame (`~548.625` samples at 32768 Hz), buffering
    /// samples for [`Driver::take_output`].
    pub fn step_frame(&mut self) {
        self.cycle_acc += FRAME_CYCLES;
        let samples = (self.cycle_acc / CYCLES_PER_SAMPLE) as usize;
        self.cycle_acc %= CYCLES_PER_SAMPLE;
        self.output.clear();
        let mut buf = std::mem::take(&mut self.output);
        self.render_samples(samples, &mut buf);
        self.output = buf;
    }

    /// Append buffered samples from [`Driver::step_frame`] into `out`.
    pub fn take_output(&mut self, out: &mut Vec<[f32; 2]>) {
        out.extend_from_slice(&self.output);
        self.output.clear();
    }

    /// Render exactly `count` stereo samples at 32768 Hz, appending them to `out`.
    pub fn render_samples(&mut self, count: usize, out: &mut Vec<[f32; 2]>) {
        out.reserve(count);
        if !self.playing && !self.channels.iter().any(|c| c.active) {
            out.resize(out.len() + count, [0.0, 0.0]);
            return;
        }
        let ext_gain = MASTER_GAIN
            * (self.cue_volume.min(0x200) as f32 / 256.0)
            * (self.host_level as f32 / 256.0);

        for _ in 0..count {
            self.tick_acc += DRIVER_TICKS_PER_SAMPLE;
            while self.tick_acc >= 1.0 {
                self.tick_acc -= 1.0;
                self.step_driver_tick();
            }
            out.push(self.mix_sample(ext_gain));
        }
    }

    fn step_driver_tick(&mut self) {
        // 1. Advance channel envelopes, LFOs, and pitch sweeps (192 Hz)
        for ch in &mut self.channels {
            if !ch.active {
                continue;
            }
            // Envelope update (NitroSDK 192 Hz): store attenuation magnitude so `>> 8` truncates toward 0.
            match ch.env_phase {
                EnvPhase::Attack => {
                    let coeff = attack_coeff(ch.note.attack);
                    let atten = (coeff * (-ch.env_level)) >> 8;
                    ch.env_level = -atten;
                    if atten == 0 {
                        ch.env_phase = EnvPhase::Decay;
                    }
                }
                EnvPhase::Decay => {
                    let sus_level = (DECIBEL_TABLE[ch.note.sustain.min(127) as usize] as i32) << 7;
                    ch.env_level -= decay_rate(ch.note.decay);
                    if ch.env_level <= sus_level {
                        ch.env_level = sus_level;
                        ch.env_phase = EnvPhase::Sustain;
                    }
                }
                EnvPhase::Sustain => {}
                EnvPhase::Release => {
                    ch.env_level -= decay_rate(ch.note.release);
                    if ch.env_level <= ENV_SILENCE {
                        ch.env_level = ENV_SILENCE;
                        ch.env_phase = EnvPhase::Off;
                        ch.active = false;
                        continue;
                    }
                }
                EnvPhase::Off => {
                    ch.active = false;
                    continue;
                }
            }
            // LFO update
            let tr = &self.tracks[ch.track as usize];
            if tr.mod_depth != 0 {
                if ch.lfo_delay < tr.mod_delay {
                    ch.lfo_delay += 1;
                } else {
                    // NitroSDK advances phase by (mod_speed << 6) on a 15-bit phase (>> 8 -> 0..127)
                    ch.lfo_phase = ch.lfo_phase.wrapping_add((tr.mod_speed as u16) << 6) & 0x7FFF;
                }
            }
            // Sweep update
            if ch.sweep_counter < ch.sweep_len {
                ch.sweep_counter += 1;
            }
        }

        // 2. Advance sequencer ticks according to BPM * tempo_ratio
        if !self.playing {
            return;
        }
        let eff_tempo = ((self.tempo as u32) * (self.tempo_ratio as u32)) >> 8;
        self.tempo_acc += eff_tempo.max(1);
        while self.tempo_acc >= 240 {
            self.tempo_acc -= 240;
            self.step_sequence_tick();
            if !self.playing {
                break;
            }
        }
    }

    fn step_sequence_tick(&mut self) {
        // Advance note gate durations on sequence ticks (matches NitroSDK so tempo/Pinch scales gates live).
        for ch in &mut self.channels {
            if ch.active && ch.auto_release && ch.env_phase != EnvPhase::Release && ch.duration > 0 {
                ch.duration -= 1;
                if ch.duration == 0 {
                    ch.env_phase = EnvPhase::Release;
                }
            }
        }

        let Some(song_id) = self.song else {
            self.playing = false;
            return;
        };
        let Some(song) = self.bank.song(song_id) else {
            self.playing = false;
            return;
        };
        let data_len = song.data.len();
        let mut any_active = false;

        for ti in 0..MAX_TRACKS {
            if !self.tracks[ti].active {
                continue;
            }
            any_active = true;
            if self.tracks[ti].wait > 0 {
                self.tracks[ti].wait -= 1;
            }
            // Guard against degenerate zero-wait infinite loops inside malformed data.
            let mut ops_this_tick = 0u32;
            while self.tracks[ti].active && self.tracks[ti].wait <= 0 {
                ops_this_tick += 1;
                if ops_this_tick > 512 {
                    break;
                }
                let pc = self.tracks[ti].pc;
                if pc >= data_len {
                    self.tracks[ti].active = false;
                    break;
                }
                self.exec_track_op(ti, song_id);
            }
        }

        if !any_active && !self.channels.iter().any(|c| c.active) {
            self.playing = false;
        }
    }

    fn next_rng(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(1664525).wrapping_add(1013904223);
        self.rng
    }

    fn exec_track_op(&mut self, ti: usize, song_id: SongId) {
        let bank = self.bank.clone();
        let Some(song) = bank.song(song_id) else {
            self.tracks[ti].active = false;
            return;
        };
        let data = &song.data;
        let mut pc = self.tracks[ti].pc;
        let read_u8 = |p: &mut usize| -> Option<u8> {
            let b = *data.get(*p)?;
            *p += 1;
            Some(b)
        };
        let read_u16 = |p: &mut usize| -> Option<u16> {
            let lo = read_u8(p)? as u16;
            let hi = read_u8(p)? as u16;
            Some(lo | (hi << 8))
        };
        let read_u24 = |p: &mut usize| -> Option<usize> {
            let b0 = read_u8(p)? as usize;
            let b1 = read_u8(p)? as usize;
            let b2 = read_u8(p)? as usize;
            Some(b0 | (b1 << 8) | (b2 << 16))
        };
        let read_vl = |p: &mut usize| -> Option<u32> {
            let mut val = 0u32;
            for _ in 0..4 {
                let b = read_u8(p)?;
                val = (val << 7) | (b & 0x7F) as u32;
                if (b & 0x80) == 0 {
                    return Some(val);
                }
            }
            Some(val)
        };

        let Some(mut op) = read_u8(&mut pc) else {
            self.tracks[ti].active = false;
            return;
        };

        // Optional conditional `if` prefix (`0xA2`)
        let mut execute = true;
        if op == 0xA2 {
            execute = self.tracks[ti].cmp_flag;
            let Some(next) = read_u8(&mut pc) else {
                self.tracks[ti].active = false;
                return;
            };
            op = next;
        }

        // Optional `random` (`0xA0`) or `from_var` (`0xA1`) arg override prefix
        let mut arg_prefix = 0u8;
        if op == 0xA0 || op == 0xA1 {
            arg_prefix = op;
            let Some(next) = read_u8(&mut pc) else {
                self.tracks[ti].active = false;
                return;
            };
            op = next;
        }

        let finish_prefix = |driver: &mut Driver, p: &mut usize| -> Option<i32> {
            match arg_prefix {
                0xA0 => {
                    let lo = read_u16(p)? as i16 as i32;
                    let hi = read_u16(p)? as i16 as i32;
                    let (a, b) = if lo <= hi { (lo, hi) } else { (hi, lo) };
                    let span = (b - a + 1).max(1) as u32;
                    let r = (driver.next_rng() >> 16) % span;
                    Some(a + r as i32)
                }
                0xA1 => {
                    let vid = read_u8(p)? as usize;
                    Some(driver.vars.get(vid).copied().unwrap_or(0) as i32)
                }
                _ => None,
            }
        };

        if op < 0x80 {
            // Note command: key = op, vel = u8, dur = vl (or prefix override)
            let Some(vel) = read_u8(&mut pc) else {
                self.tracks[ti].active = false;
                return;
            };
            let dur = if arg_prefix != 0 {
                finish_prefix(self, &mut pc).unwrap_or(0).max(0)
            } else {
                read_vl(&mut pc).unwrap_or(0) as i32
            };
            self.tracks[ti].pc = pc;
            if execute {
                self.play_note(ti, op, vel, dur);
                if self.tracks[ti].note_wait {
                    self.tracks[ti].wait += dur;
                }
            }
            return;
        }

        let read_arg_u8 = |driver: &mut Driver, p: &mut usize| -> Option<u8> {
            if arg_prefix != 0 {
                Some(finish_prefix(driver, p)? as u8)
            } else {
                read_u8(p)
            }
        };
        let read_arg_u16 = |driver: &mut Driver, p: &mut usize| -> Option<u16> {
            if arg_prefix != 0 {
                Some(finish_prefix(driver, p)? as u16)
            } else {
                read_u16(p)
            }
        };
        let read_arg_vl = |driver: &mut Driver, p: &mut usize| -> Option<u32> {
            if arg_prefix != 0 {
                Some(finish_prefix(driver, p)?.max(0) as u32)
            } else {
                read_vl(p)
            }
        };

        match op {
            0x80 => {
                // Wait
                let Some(w) = read_arg_vl(self, &mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                self.tracks[ti].pc = pc;
                if execute {
                    self.tracks[ti].wait += w as i32;
                }
            }
            0x81 => {
                // Program change
                let Some(prg) = read_arg_vl(self, &mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                self.tracks[ti].pc = pc;
                if execute {
                    self.tracks[ti].program = (prg & 0x7FFF) as u16;
                }
            }
            0x93 => {
                // Open track
                let Some(tr_num) = read_u8(&mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                let Some(off) = read_u24(&mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                self.tracks[ti].pc = pc;
                if execute && (tr_num as usize) < MAX_TRACKS {
                    let prio = self.tracks[ti].priority;
                    let t = &mut self.tracks[tr_num as usize];
                    *t = Track::default();
                    t.active = true;
                    t.pc = off;
                    t.priority = prio;
                }
            }
            0x94 => {
                // Jump
                let Some(off) = read_u24(&mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                self.tracks[ti].pc = if execute { off } else { pc };
            }
            0x95 => {
                // Call
                let Some(off) = read_u24(&mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                if execute {
                    let sp = self.tracks[ti].call_sp;
                    if sp < STACK_DEPTH {
                        self.tracks[ti].call_stack[sp] = pc;
                        self.tracks[ti].call_sp = sp + 1;
                        self.tracks[ti].pc = off;
                    } else {
                        self.tracks[ti].pc = pc;
                    }
                } else {
                    self.tracks[ti].pc = pc;
                }
            }
            0xB0..=0xBD => {
                // Variable arithmetic / comparison
                let Some(vid) = read_u8(&mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                let Some(raw) = read_arg_u16(self, &mut pc) else {
                    self.tracks[ti].active = false;
                    return;
                };
                let val = raw as i16;
                self.tracks[ti].pc = pc;
                if execute && (vid as usize) < self.vars.len() {
                    let cur = self.vars[vid as usize];
                    match op {
                        0xB0 => self.vars[vid as usize] = val,
                        0xB1 => self.vars[vid as usize] = cur.wrapping_add(val),
                        0xB2 => self.vars[vid as usize] = cur.wrapping_sub(val),
                        0xB3 => self.vars[vid as usize] = cur.wrapping_mul(val),
                        0xB4 => {
                            if val != 0 {
                                self.vars[vid as usize] = cur.wrapping_div(val);
                            }
                        }
                        0xB5 => {
                            self.vars[vid as usize] = if val >= 0 {
                                cur.wrapping_shl(val as u32)
                            } else {
                                cur.wrapping_shr((-val) as u32)
                            };
                        }
                        0xB6 => {
                            let span = (val.unsigned_abs() as u32).max(1);
                            let r = ((self.next_rng() >> 16) % span) as i16;
                            self.vars[vid as usize] = if val < 0 { -r } else { r };
                        }
                        0xB8 => self.tracks[ti].cmp_flag = cur == val,
                        0xB9 => self.tracks[ti].cmp_flag = cur >= val,
                        0xBA => self.tracks[ti].cmp_flag = cur > val,
                        0xBB => self.tracks[ti].cmp_flag = cur <= val,
                        0xBC => self.tracks[ti].cmp_flag = cur < val,
                        0xBD => self.tracks[ti].cmp_flag = cur != val,
                        _ => {}
                    }
                }
            }
            0xC0 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].pan = v & 0x7F; }
            }
            0xC1 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].volume = v & 0x7F; }
            }
            0xC2 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.master_volume = v & 0x7F; }
            }
            0xC3 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].transpose = v as i8; }
            }
            0xC4 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].pitch_bend = v as i8; }
            }
            0xC5 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].pitch_bend_range = v; }
            }
            0xC6 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].priority = v; }
            }
            0xC7 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].note_wait = v != 0; }
            }
            0xC8 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute {
                    self.tracks[ti].tie = v != 0;
                    self.release_track_channels(ti);
                }
            }
            0xC9 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute {
                    self.tracks[ti].porta_key = v;
                    self.tracks[ti].porta_on = true;
                }
            }
            0xCA => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].mod_depth = v; }
            }
            0xCB => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].mod_speed = v; }
            }
            0xCC => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].mod_type = v; }
            }
            0xCD => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].mod_range = v; }
            }
            0xCE => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].porta_on = v != 0; }
            }
            0xCF => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].porta_time = v; }
            }
            0xD0 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].attack_override = (v < 0x80).then_some(v); }
            }
            0xD1 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].decay_override = (v < 0x80).then_some(v); }
            }
            0xD2 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].sustain_override = (v < 0x80).then_some(v); }
            }
            0xD3 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].release_override = (v < 0x80).then_some(v); }
            }
            0xD4 => {
                // Loop start (`count == 0` = infinite loop)
                let Some(count) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute {
                    let sp = self.tracks[ti].loop_sp;
                    if sp < STACK_DEPTH {
                        self.tracks[ti].loop_stack[sp] = (pc, count);
                        self.tracks[ti].loop_sp = sp + 1;
                    }
                }
            }
            0xD5 => {
                let Some(v) = read_arg_u8(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].expression = v & 0x7F; }
            }
            0xD6 => {
                let _ = read_u8(&mut pc);
                self.tracks[ti].pc = pc;
            }
            0xE0 => {
                let Some(v) = read_arg_u16(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].mod_delay = v; }
            }
            0xE1 => {
                let Some(v) = read_arg_u16(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tempo = v.clamp(1, 240); }
            }
            0xE3 => {
                let Some(v) = read_arg_u16(self, &mut pc) else { self.tracks[ti].active = false; return; };
                self.tracks[ti].pc = pc;
                if execute { self.tracks[ti].sweep_pitch = v as i16; }
            }
            0xFC => {
                // Loop end
                self.tracks[ti].pc = pc;
                if execute && self.tracks[ti].loop_sp > 0 {
                    let top = self.tracks[ti].loop_sp - 1;
                    let (target, rem) = self.tracks[ti].loop_stack[top];
                    if rem == 0 {
                        self.tracks[ti].pc = target;
                    } else if rem > 1 {
                        self.tracks[ti].loop_stack[top].1 = rem - 1;
                        self.tracks[ti].pc = target;
                    } else {
                        self.tracks[ti].loop_sp = top;
                    }
                }
            }
            0xFD => {
                // Return from call
                if execute && self.tracks[ti].call_sp > 0 {
                    let top = self.tracks[ti].call_sp - 1;
                    self.tracks[ti].call_sp = top;
                    self.tracks[ti].pc = self.tracks[ti].call_stack[top];
                } else {
                    self.tracks[ti].pc = pc;
                }
            }
            0xFE => {
                // Allocate tracks bitmask (no-op; tracks are pre-allocated)
                let _ = read_u16(&mut pc);
                self.tracks[ti].pc = pc;
            }
            0xFF => {
                // End of track
                self.tracks[ti].pc = pc;
                if execute {
                    self.tracks[ti].active = false;
                }
            }
            _ => {
                self.tracks[ti].active = false;
            }
        }
    }

    fn release_track_channels(&mut self, ti: usize) {
        for ch in &mut self.channels {
            if ch.active && ch.track as usize == ti && ch.env_phase != EnvPhase::Release {
                ch.env_phase = EnvPhase::Release;
            }
        }
    }

    fn play_note(&mut self, ti: usize, key: u8, vel: u8, dur_seq_ticks: i32) {
        let tr = &self.tracks[ti];
        let effective_key = (key as i32 + tr.transpose as i32).clamp(0, 127) as u8;

        // Handle tie mode: retarget active channel on this track
        if tr.tie {
            if let Some(ci) = tr.last_channel {
                let ch = &mut self.channels[ci];
                if ch.active && ch.track as usize == ti && ch.env_phase != EnvPhase::Release {
                    ch.note_key = effective_key;
                    ch.velocity = vel & 0x7F;
                    return;
                }
            }
        }

        let Some(inst_bank) = self.bank.bank(self.bank_id) else {
            return;
        };
        let Some(inst) = inst_bank.instruments.get(tr.program as usize) else {
            return;
        };
        let Some(mut note_def) = inst.resolve(effective_key) else {
            return;
        };
        if let Some(a) = tr.attack_override {
            note_def.attack = a;
        }
        if let Some(d) = tr.decay_override {
            note_def.decay = d;
        }
        if let Some(s) = tr.sustain_override {
            note_def.sustain = s;
        }
        if let Some(r) = tr.release_override {
            note_def.release = r;
        }

        let eff_tempo = (((self.tempo as u32) * (self.tempo_ratio as u32)) >> 8).max(1);
        let dur_driver_ticks = if dur_seq_ticks > 0 {
            ((dur_seq_ticks as u64 * 240) / eff_tempo as u64).max(1) as i32
        } else {
            1
        };

        let slot = self.alloc_channel(tr.priority);
        self.age_counter += 1;
        let age = self.age_counter;

        let mut sweep_pitch = tr.sweep_pitch as i32;
        let mut sweep_len = dur_driver_ticks;
        if tr.porta_on {
            sweep_pitch += (tr.porta_key as i32 - effective_key as i32) * 64;
            if tr.porta_time != 0 {
                sweep_len = ((tr.porta_time as i32 * tr.porta_time as i32) * sweep_pitch.abs()) >> 11;
            }
        }

        let init_level = if note_def.attack >= 127 { 0 } else { ENV_SILENCE };
        let init_phase = if note_def.attack >= 127 { EnvPhase::Decay } else { EnvPhase::Attack };

        let ch = &mut self.channels[slot];
        *ch = Channel {
            active: true,
            track: ti as u8,
            priority: tr.priority,
            note_key: effective_key,
            velocity: vel & 0x7F,
            duration: dur_seq_ticks.max(1),
            auto_release: !tr.tie,
            bank_id: self.bank_id,
            note: note_def,
            pos: 0.0,
            lfsr: 0x7FFF,
            env_phase: init_phase,
            env_level: init_level,
            lfo_delay: 0,
            lfo_phase: 0,
            sweep_counter: 0,
            sweep_len: sweep_len.max(1),
            sweep_pitch,
            age,
        };
        self.tracks[ti].porta_key = effective_key;
        self.tracks[ti].last_channel = Some(slot);
    }

    fn alloc_channel(&self, priority: u8) -> usize {
        if let Some(i) = self.channels.iter().position(|c| !c.active) {
            return i;
        }
        // Prefer stealing a releasing channel, then lowest priority / oldest.
        let mut best = 0;
        let mut best_key = (false, u8::MAX, u64::MAX);
        for (i, ch) in self.channels.iter().enumerate() {
            let releasing = ch.env_phase == EnvPhase::Release;
            let k = (!releasing, ch.priority, ch.age);
            if k < best_key {
                best_key = k;
                best = i;
            }
        }
        let _ = priority;
        best
    }

    fn mix_sample(&mut self, ext_gain: f32) -> [f32; 2] {
        let mut left = 0.0_f32;
        let mut right = 0.0_f32;
        let master_db = DECIBEL_TABLE[self.master_volume.min(127) as usize] as i32
            + DECIBEL_TABLE[self.song_volume.min(127) as usize] as i32;
        let pitch_ext_semitones = self.pitch_shift as f64 / 256.0;

        for ch in &mut self.channels {
            if !ch.active {
                continue;
            }
            let tr = &self.tracks[ch.track as usize];

            // Compute LFO modulation value
            let mut lfo_pitch = 0.0_f64;
            let mut lfo_vol_db = 0_i32;
            let mut lfo_pan = 0_i32;
            if tr.mod_depth != 0 && ch.lfo_delay >= tr.mod_delay {
                let s = lfo_sin(((ch.lfo_phase >> 8) & 0x7F) as u8);
                let mod_val = (s * tr.mod_depth as i32 * tr.mod_range as i32) >> 7;
                match tr.mod_type {
                    0 => lfo_pitch = mod_val as f64 / 64.0, // 1/64 semitone units
                    1 => lfo_vol_db = (mod_val * 60) >> 7,
                    2 => lfo_pan = mod_val >> 1,
                    _ => {}
                }
            }

            // Compute pitch in semitones relative to base_note
            let sweep_semi = if ch.sweep_pitch != 0 && ch.sweep_counter < ch.sweep_len {
                let rem = (ch.sweep_len - ch.sweep_counter) as f64 / ch.sweep_len as f64;
                (ch.sweep_pitch as f64 / 64.0) * rem
            } else {
                0.0
            };
            let bend_semi = (tr.pitch_bend as f64 / 128.0) * tr.pitch_bend_range as f64;
            let semitones = (ch.note_key as f64 - ch.note.base_note as f64)
                + bend_semi
                + pitch_ext_semitones
                + lfo_pitch
                + sweep_semi;

            // Sample generation
            let sample = match ch.note.kind {
                NoteKind::Pcm { swav_id, swar_slot } => {
                    let Some(inst_bank) = self.bank.bank(ch.bank_id) else {
                        ch.active = false;
                        continue;
                    };
                    let Some(wave) = self.bank.wave(inst_bank, swar_slot, swav_id) else {
                        ch.active = false;
                        continue;
                    };
                    if wave.samples.is_empty() {
                        ch.active = false;
                        continue;
                    }
                    let idx0 = ch.pos as usize;
                    if idx0 >= wave.samples.len() {
                        ch.active = false;
                        continue;
                    }
                    let frac = (ch.pos - idx0 as f64) as f32;
                    let s0 = wave.samples[idx0] as f32;
                    let idx1 = if idx0 + 1 < wave.loop_end {
                        idx0 + 1
                    } else if wave.looped {
                        wave.loop_start
                    } else {
                        idx0
                    };
                    let s1 = wave.samples[idx1] as f32;
                    let out_s = (s0 + (s1 - s0) * frac) / 32768.0;

                    let freq = wave.sample_rate as f64 * (2.0_f64).powf(semitones / 12.0);
                    ch.pos += freq / OUT_RATE as f64;
                    if ch.pos >= wave.loop_end as f64 {
                        if wave.looped && wave.loop_end > wave.loop_start {
                            let loop_len = (wave.loop_end - wave.loop_start) as f64;
                            ch.pos = wave.loop_start as f64 + (ch.pos - wave.loop_start as f64) % loop_len;
                        } else {
                            ch.active = false;
                        }
                    }
                    out_s
                }
                NoteKind::Psg { duty } => {
                    let midi = ch.note_key as f64 + bend_semi + pitch_ext_semitones + lfo_pitch + sweep_semi;
                    let freq = 440.0 * (2.0_f64).powf((midi - 69.0) / 12.0);
                    let threshold = ((duty as f64) + 1.0) / 8.0;
                    let out_s = if ch.pos < threshold { 0.5 } else { -0.5 };
                    ch.pos = (ch.pos + freq / OUT_RATE as f64) % 1.0;
                    out_s
                }
                NoteKind::Noise => {
                    let midi = ch.note_key as f64 + bend_semi + pitch_ext_semitones + lfo_pitch + sweep_semi;
                    let freq = (440.0 * (2.0_f64).powf((midi - 69.0) / 12.0) * 8.0).min(OUT_RATE as f64 * 4.0);
                    ch.pos += freq / OUT_RATE as f64;
                    while ch.pos >= 1.0 {
                        ch.pos -= 1.0;
                        let bit = (ch.lfsr & 1) != 0;
                        ch.lfsr >>= 1;
                        if bit {
                            ch.lfsr ^= 0x6000;
                        }
                    }
                    if (ch.lfsr & 1) != 0 { 0.5 } else { -0.5 }
                }
                NoteKind::Empty => {
                    ch.active = false;
                    continue;
                }
            };

            // Combine decibel gains
            let db = master_db
                + DECIBEL_TABLE[tr.volume.min(127) as usize] as i32
                + DECIBEL_TABLE[tr.expression.min(127) as usize] as i32
                + DECIBEL_TABLE[ch.velocity.min(127) as usize] as i32
                + (ch.env_level >> 7)
                + lfo_vol_db;
            let gain = db_to_gain(db) * ext_gain;
            if gain <= 0.0 {
                continue;
            }

            let pan = (64 + (tr.pan as i32 - 64) + (ch.note.pan as i32 - 64) + lfo_pan).clamp(0, 127) as f32;
            let gain_r = pan / 127.0;
            let gain_l = 1.0 - gain_r;
            let s = sample * gain;
            left += s * gain_l;
            right += s * gain_r;
        }

        [left.clamp(-1.0, 1.0), right.clamp(-1.0, 1.0)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::{Instrument, InstrumentBank, SoundBank, Wave, WaveArchive};

    #[test]
    fn renders_simple_sequence_with_pinch_and_volume() {
        // Minimal 1-wave sine + 1-track looping SSEQ
        let samples: Vec<i16> = (0..64)
            .map(|i| (((i as f64 / 64.0) * std::f64::consts::TAU).sin() * 20000.0) as i16)
            .collect();
        let wave = Wave { samples, sample_rate: 16384, looped: true, loop_start: 0, loop_end: 64 };
        let inst = Instrument::Single(NoteDef {
            kind: NoteKind::Pcm { swav_id: 0, swar_slot: 0 },
            base_note: 60,
            attack: 127,
            decay: 127,
            sustain: 127,
            release: 80,
            pan: 64,
        });
        // SSEQ: tempo=144, prg=0, loop_start 0, note 60 vel 100 dur 24, loop_end
        let sseq = vec![0xE1, 144, 0, 0x81, 0, 0xD4, 0, 60, 100, 24, 0xFC];
        let bank = Arc::new(SoundBank {
            songs: vec![Some(crate::bank::Song {
                data: sseq,
                bank: 0,
                volume: 100,
                channel_priority: 64,
                player_priority: 64,
                player: 0,
            })],
            banks: vec![Some(InstrumentBank {
                wave_arcs: [Some(0), None, None, None],
                instruments: vec![inst],
            })],
            wave_arcs: vec![Some(WaveArchive { waves: vec![wave] })],
        });

        let mut driver = Driver::new(bank);
        driver.start(SongId(0));
        assert!(driver.is_playing());
        let mut out = Vec::new();
        for _ in 0..30 {
            driver.step_frame();
            driver.take_output(&mut out);
        }
        assert!(out.len() > 16_000);
        let rms = (out.iter().map(|s| s[0] * s[0] + s[1] * s[1]).sum::<f32>() / (2.0 * out.len() as f32)).sqrt();
        assert!(rms > 0.01, "expected audible output, got rms={rms}");

        // Snapshot clone determinism (required for rollback & replay seeking)
        let mut a = driver.clone();
        let mut b = driver.clone();
        a.set_pitch(0x100);
        a.set_tempo(0x11A);
        b.set_pitch(0x100);
        b.set_tempo(0x11A);
        let (mut oa, mut ob) = (Vec::new(), Vec::new());
        for _ in 0..10 {
            a.step_frame();
            a.take_output(&mut oa);
            b.step_frame();
            b.take_output(&mut ob);
        }
        assert_eq!(oa, ob, "cloned drivers must render bit-identical audio");

        driver.stop_song(SongId(0));
        assert!(!driver.is_playing());
    }
}
