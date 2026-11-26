//! Pure-Rust Nintendo DS NitroComposer (`SDAT` / `SSEQ` / `SBNK` / `SWAR`)
//! sound bank parser and real-time synthesizer driver.
//!
//! Designed as the NDS counterpart to Saito's GBA `m4a` crate: zero `unsafe`
//! code, deterministic and `Clone`-able driver state (so rollback audio and
//! replay seeking snapshot cleanly), rendering stereo `f32` samples at
//! [`OUT_RATE`] (`32768 Hz`, matching `m4a::OUT_RATE`).

#![forbid(unsafe_code)]

pub mod bank;
pub mod driver;
pub mod parse;
pub mod tables;
pub mod voice;

pub use bank::{Instrument, InstrumentBank, NoteDef, NoteKind, Song, SongId, SoundBank, Wave, WaveArchive};
pub use driver::Driver;
pub use parse::Error;
pub use voice::{VOICE_CHANNELS, VoiceBank, VoiceClip, VoicePlayer, VoiceStep};

/// Output sample rate in Hz (matches `m4a::OUT_RATE`).
pub const OUT_RATE: u32 = 32_768;
/// Battle frame rate in Hz (matches `m4a::FPS`).
pub const FPS: f64 = 59.7275;
