//! One-shot and multi-part `SSAR` voice clip playback for Nintendo DS `SDAT`
//! sound archives (e.g. EXE5DS battle voices for Lan, MegaMan, ProtoMan, and
//! Colonel).

use crate::bank::Wave;
use crate::OUT_RATE;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Number of concurrent voice channels (slot 0 = operator/Lan, slot 1 = Navi).
pub const VOICE_CHANNELS: usize = 2;

/// Output gain calibration matching GBA M4A SFX and NDS SDAT BGM headroom.
const VOICE_GAIN: f32 = 0.72;

/// One waveform step inside a [`VoiceClip`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoiceStep {
    /// Index into [`VoiceBank::waves`].
    pub wave: u16,
    /// Offset in output samples (`OUT_RATE`) from the start of the clip when
    /// this step begins playing.
    pub delay_samples: u32,
}

/// One voice entry extracted from an SDAT `SSAR` archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceClip {
    /// Master volume from the SSAR entry (`0..=127`).
    pub volume: u8,
    /// One or more sequential sample steps (single cry or concatenated callout
    /// such as "Soul Unison!" -> "ProtoSoul!").
    pub steps: Vec<VoiceStep>,
}

/// Decoded collection of `SSAR` voice clips and their deduplicated PCM16 waves.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VoiceBank {
    pub waves: Vec<Wave>,
    pub clips: BTreeMap<u16, VoiceClip>,
}

impl VoiceBank {
    pub fn clip(&self, id: u16) -> Option<&VoiceClip> {
        self.clips.get(&id)
    }

    pub fn wave(&self, idx: u16) -> Option<&Wave> {
        self.waves.get(idx as usize)
    }

    pub fn len(&self) -> usize {
        self.clips.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clips.is_empty()
    }
}

#[derive(Clone, Debug)]
struct PlayingStep {
    wave: u16,
    /// Fractional sample position inside `wave.samples` (`16.16` fixed-point).
    pos_fp: u64,
    /// Step per output sample in `16.16` fixed-point.
    step_fp: u64,
}

#[derive(Clone, Debug)]
struct ActiveVoice {
    clip_id: u16,
    volume: f32,
    /// Output samples elapsed since this clip started.
    elapsed: u32,
    /// Index of the next [`VoiceStep`] not yet launched.
    next_step: usize,
    /// Currently playing step (if any).
    current: Option<PlayingStep>,
}

/// Deterministic, `Clone`-able voice clip mixer rendering at [`OUT_RATE`].
#[derive(Clone, Debug)]
pub struct VoicePlayer {
    bank: Arc<VoiceBank>,
    slots: [Option<ActiveVoice>; VOICE_CHANNELS],
    /// Host effects level (`0..=256`, 256 = unity).
    level: u16,
}

impl VoicePlayer {
    pub fn new(bank: Arc<VoiceBank>) -> Self {
        Self {
            bank,
            slots: [None, None],
            level: 256,
        }
    }

    pub fn bank(&self) -> &Arc<VoiceBank> {
        &self.bank
    }

    /// Set the mixer level (`0..=256`, 256 = 1.0).
    pub fn set_level(&mut self, level: u16) {
        self.level = level.min(256);
    }

    /// Start playing `clip_id` on `channel` (`0..VOICE_CHANNELS`), preempting
    /// any previous clip on that channel. No-op if `clip_id` is absent.
    pub fn play(&mut self, clip_id: u16, channel: usize) {
        let Some(clip) = self.bank.clip(clip_id) else {
            return;
        };
        if clip.steps.is_empty() {
            return;
        }
        let slot = channel % VOICE_CHANNELS;
        let volume = (clip.volume as f32 / 127.0) * VOICE_GAIN;
        self.slots[slot] = Some(ActiveVoice {
            clip_id,
            volume,
            elapsed: 0,
            next_step: 0,
            current: None,
        });
    }

    /// Stop all playing voice channels.
    pub fn stop_all(&mut self) {
        for slot in &mut self.slots {
            *slot = None;
        }
    }

    /// Whether any voice channel is currently active.
    pub fn is_playing(&self) -> bool {
        self.slots.iter().any(Option::is_some)
    }

    /// Mix `out.len()` stereo samples into `out` in place, clamping to
    /// `[-1.0, 1.0]`.
    pub fn mix_into(&mut self, out: &mut [[f32; 2]]) {
        if out.is_empty() || !self.is_playing() {
            return;
        }
        let level_gain = self.level as f32 / 256.0;
        let bank = &self.bank;

        for slot in &mut self.slots {
            let Some(active) = slot.as_mut() else {
                continue;
            };
            let Some(clip) = bank.clip(active.clip_id) else {
                *slot = None;
                continue;
            };
            let gain = active.volume * level_gain;
            let mut finished = false;

            for sample in out.iter_mut() {
                // Launch any steps whose start time has arrived.
                while active.next_step < clip.steps.len() && active.elapsed >= clip.steps[active.next_step].delay_samples {
                    let st = clip.steps[active.next_step];
                    active.next_step += 1;
                    if let Some(wave) = bank.wave(st.wave)
                        && !wave.samples.is_empty()
                        && wave.sample_rate > 0
                    {
                        let step_fp = ((wave.sample_rate as u64) << 16) / (OUT_RATE as u64);
                        active.current = Some(PlayingStep {
                            wave: st.wave,
                            pos_fp: 0,
                            step_fp: step_fp.max(1),
                        });
                    }
                }

                if let Some(cur) = active.current.as_mut() {
                    if let Some(wave) = bank.wave(cur.wave) {
                        let idx = (cur.pos_fp >> 16) as usize;
                        if idx >= wave.samples.len() {
                            active.current = None;
                        } else {
                            let s0 = wave.samples[idx] as f32;
                            let s1 = wave.samples.get(idx + 1).copied().unwrap_or(wave.samples[idx]) as f32;
                            let frac = (cur.pos_fp & 0xFFFF) as f32 * (1.0 / 65536.0);
                            let mono = ((s0 + (s1 - s0) * frac) * (1.0 / 32768.0)) * gain;
                            sample[0] = (sample[0] + mono).clamp(-1.0, 1.0);
                            sample[1] = (sample[1] + mono).clamp(-1.0, 1.0);
                            cur.pos_fp = cur.pos_fp.saturating_add(cur.step_fp);
                        }
                    } else {
                        active.current = None;
                    }
                }

                active.elapsed = active.elapsed.saturating_add(1);

                if active.current.is_none() && active.next_step >= clip.steps.len() {
                    finished = true;
                    break;
                }
            }

            if finished {
                *slot = None;
            }
        }
    }
}
