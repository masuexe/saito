//! Danger-gated microphone input monitor for EXE5DS Crisis Shout.
//!
//! Privacy-first design:
//! - [`MicMonitor::new`] does **not** open any audio input stream.
//! - Calling [`MicMonitor::set_armed`]`(true)` opens the default system input device
//!   (on desktop targets with the `playback` feature) and computes a lightweight rolling
//!   amplitude level (`0.0..=1.0`). If no input device exists or OS permission is denied,
//!   it records the failure and silently degrades without panicking or blocking.
//! - Calling [`MicMonitor::set_armed`]`(false)` or dropping [`MicMonitor`] immediately
//!   drops the underlying `cpal::Stream`, turning off the OS microphone indicator at once.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Normalized scaled amplitude threshold for Tier 1 (Low shout).
pub const LOW_THRESHOLD: f32 = 0.12;
/// Normalized scaled amplitude threshold for Tier 2 (Medium shout).
pub const MID_THRESHOLD: f32 = 0.28;
/// Normalized scaled amplitude threshold for Tier 3 (High / Max shout).
pub const HIGH_THRESHOLD: f32 = 0.52;

/// Minimum allowed user sensitivity multiplier.
pub const SENSITIVITY_MIN: f32 = 0.25;
/// Maximum allowed user sensitivity multiplier.
pub const SENSITIVITY_MAX: f32 = 4.0;

/// How long [`PeakDetector`] waits after crossing [`LOW_THRESHOLD`] to capture the
/// rising edge's peak amplitude before emitting Tier 1 or Tier 2.
/// Reaching Tier 3 emits immediately because no higher tier exists.
pub const CAPTURE_WINDOW: Duration = Duration::from_millis(120);

/// Cooldown after emitting a shout tier before another shout can be emitted while armed.
pub const COOLDOWN_WINDOW: Duration = Duration::from_millis(350);

/// Classify a normalized microphone amplitude (`0.0..=1.0`) scaled by `sensitivity`
/// into an EXE5DS shout tier: `0` (silent), `1` (Low), `2` (Medium), or `3` (High).
pub fn classify_level(level: f32, sensitivity: f32) -> u8 {
    if !level.is_finite() || level <= 0.0 {
        return 0;
    }
    let gain = if sensitivity.is_finite() { sensitivity.clamp(SENSITIVITY_MIN, SENSITIVITY_MAX) } else { 1.0 };
    let scaled = level.clamp(0.0, 1.0) * gain;
    if scaled >= HIGH_THRESHOLD {
        3
    } else if scaled >= MID_THRESHOLD {
        2
    } else if scaled >= LOW_THRESHOLD {
        1
    } else {
        0
    }
}

/// Compute a normalized amplitude (`0.0..=1.0`) from a slice of floating-point samples in `-1.0..=1.0`.
/// Blends RMS and peak so short impulsive shouts and sustained voices both register cleanly.
#[cfg(any(
    test,
    all(feature = "playback", not(target_arch = "wasm32"), not(target_os = "android"), not(target_os = "ios"))
))]
fn buffer_amplitude(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut sum_sq = 0.0f64;
    let mut peak = 0.0f32;
    for &s in samples {
        let a = if s.is_finite() { s.abs().min(1.0) } else { 0.0 };
        sum_sq += (a as f64) * (a as f64);
        if a > peak {
            peak = a;
        }
    }
    let rms = (sum_sq / samples.len() as f64).sqrt() as f32;
    (rms * 0.65 + peak * 0.35).clamp(0.0, 1.0)
}

#[cfg(any(
    test,
    all(feature = "playback", not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))
))]
fn update_envelope(prev: f32, measured: f32) -> f32 {
    // Fast attack so sudden shouts register on the first buffer, moderate release for smooth metering.
    let alpha = if measured >= prev { 0.75 } else { 0.20 };
    (prev + (measured - prev) * alpha).clamp(0.0, 1.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CaptureState {
    Idle,
    Capturing { started: Instant, peak_tier: u8 },
    Cooldown { until: Instant },
}

/// Pure peak-capture state machine that waits [`CAPTURE_WINDOW`] after crossing
/// [`LOW_THRESHOLD`] so a loud shout isn't prematurely emitted at Tier 1 on its rising edge.
#[derive(Clone, Debug)]
pub struct PeakDetector {
    state: CaptureState,
}

impl Default for PeakDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl PeakDetector {
    pub fn new() -> Self {
        Self { state: CaptureState::Idle }
    }

    pub fn reset(&mut self) {
        self.state = CaptureState::Idle;
    }

    /// Feed a sample level at timestamp `now` and return the emitted tier (`0` if none yet, `1..=3` when ready).
    pub fn poll_at(&mut self, level: f32, sensitivity: f32, now: Instant) -> u8 {
        let tier = classify_level(level, sensitivity);
        match self.state {
            CaptureState::Idle => {
                if tier >= 3 {
                    self.state = CaptureState::Cooldown { until: now + COOLDOWN_WINDOW };
                    3
                } else if tier >= 1 {
                    self.state = CaptureState::Capturing { started: now, peak_tier: tier };
                    0
                } else {
                    0
                }
            }
            CaptureState::Capturing { started, peak_tier } => {
                let best = peak_tier.max(tier);
                if best >= 3 {
                    self.state = CaptureState::Cooldown { until: now + COOLDOWN_WINDOW };
                    3
                } else if now.saturating_duration_since(started) >= CAPTURE_WINDOW {
                    self.state = CaptureState::Cooldown { until: now + COOLDOWN_WINDOW };
                    best
                } else {
                    self.state = CaptureState::Capturing { started, peak_tier: best };
                    0
                }
            }
            CaptureState::Cooldown { until } => {
                if now >= until {
                    self.state = CaptureState::Idle;
                }
                0
            }
        }
    }
}

#[cfg(all(feature = "playback", not(any(target_arch = "wasm32", target_os = "android", target_os = "ios"))))]
mod backend {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    pub const SUPPORTED: bool = true;

    pub struct StreamHandle {
        _stream: cpal::Stream,
    }

    pub fn open(level: Arc<AtomicU32>, broken: Arc<AtomicBool>) -> Result<StreamHandle, String> {
        let device = cpal::default_host()
            .default_input_device()
            .ok_or_else(|| "no microphone input device found".to_string())?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("failed to query microphone config: {e}"))?;

        let err_broken = broken.clone();
        let err_fn = move |e| {
            err_broken.store(true, Ordering::Relaxed);
            eprintln!("mic input stream error: {e}");
        };

        let lvl = level.clone();
        let mut envelope = 0.0f32;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                config.clone().into(),
                move |data: &[f32], _| {
                    envelope = update_envelope(envelope, buffer_amplitude(data));
                    lvl.store(envelope.to_bits(), Ordering::Relaxed);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let mut scratch = Vec::new();
                device.build_input_stream(
                    config.clone().into(),
                    move |data: &[i16], _| {
                        scratch.clear();
                        scratch.extend(data.iter().map(|&s| s as f32 / 32768.0));
                        envelope = update_envelope(envelope, buffer_amplitude(&scratch));
                        lvl.store(envelope.to_bits(), Ordering::Relaxed);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let mut scratch = Vec::new();
                device.build_input_stream(
                    config.clone().into(),
                    move |data: &[u16], _| {
                        scratch.clear();
                        scratch.extend(data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0));
                        envelope = update_envelope(envelope, buffer_amplitude(&scratch));
                        lvl.store(envelope.to_bits(), Ordering::Relaxed);
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I32 => {
                let mut scratch = Vec::new();
                device.build_input_stream(
                    config.clone().into(),
                    move |data: &[i32], _| {
                        scratch.clear();
                        scratch.extend(data.iter().map(|&s| (s as f64 / 2147483648.0) as f32));
                        envelope = update_envelope(envelope, buffer_amplitude(&scratch));
                        lvl.store(envelope.to_bits(), Ordering::Relaxed);
                    },
                    err_fn,
                    None,
                )
            }
            other => return Err(format!("unsupported microphone sample format: {other:?}")),
        }
        .map_err(|e| format!("failed to open microphone stream: {e}"))?;

        stream.play().map_err(|e| format!("failed to start microphone stream: {e}"))?;
        Ok(StreamHandle { _stream: stream })
    }
}

#[cfg(not(all(feature = "playback", not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))))]
mod backend {
    use super::*;

    pub const SUPPORTED: bool = false;

    pub struct StreamHandle;

    pub fn open(_level: Arc<AtomicU32>, _broken: Arc<AtomicBool>) -> Result<StreamHandle, String> {
        Err("microphone input is not supported on this platform".to_string())
    }
}

/// Danger-gated microphone input monitor.
///
/// The microphone stream is **never** opened until [`MicMonitor::set_armed`]`(true)` is called,
/// and is dropped immediately when [`MicMonitor::set_armed`]`(false)` is called or the monitor
/// is dropped.
pub struct MicMonitor {
    armed: bool,
    level: Arc<AtomicU32>,
    broken: Arc<AtomicBool>,
    stream: Option<backend::StreamHandle>,
    error: Option<String>,
    detector: Mutex<PeakDetector>,
}

impl Default for MicMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl MicMonitor {
    /// Create a new monitor. Does **not** open any microphone stream.
    pub fn new() -> Self {
        Self {
            armed: false,
            level: Arc::new(AtomicU32::new(0.0f32.to_bits())),
            broken: Arc::new(AtomicBool::new(false)),
            stream: None,
            error: None,
            detector: Mutex::new(PeakDetector::new()),
        }
    }

    /// Whether live microphone capture is supported on this build/target.
    pub const fn is_supported() -> bool {
        backend::SUPPORTED
    }

    /// Arm (`true`) or disarm (`false`) microphone capture.
    ///
    /// - `false -> true`: opens the default input device and begins level monitoring.
    ///   Failure (no mic / permission denied) is captured in [`Self::error`] and degrades silently.
    /// - `true -> false`: immediately drops the `cpal::Stream`, turning off the OS mic indicator.
    pub fn set_armed(&mut self, armed: bool) {
        if armed == self.armed {
            return;
        }
        self.armed = armed;
        self.level.store(0.0f32.to_bits(), Ordering::Relaxed);
        self.broken.store(false, Ordering::Relaxed);
        if let Ok(mut det) = self.detector.lock() {
            det.reset();
        }
        if !armed {
            self.stream = None;
            self.error = None;
            return;
        }
        match backend::open(self.level.clone(), self.broken.clone()) {
            Ok(handle) => {
                self.stream = Some(handle);
                self.error = None;
            }
            Err(why) => {
                self.stream = None;
                self.error = Some(why);
            }
        }
    }

    /// Whether the caller has currently armed this monitor (even if no hardware device was available).
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// True only while an audio input stream is open and actively running.
    pub fn is_active(&self) -> bool {
        self.armed && self.stream.is_some() && !self.broken.load(Ordering::Relaxed)
    }

    /// Most recent open/availability error, if arming failed.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Current normalized microphone amplitude (`0.0..=1.0`), or `0.0` when disarmed/inactive.
    pub fn level(&self) -> f32 {
        if !self.is_active() {
            return 0.0;
        }
        let v = f32::from_bits(self.level.load(Ordering::Relaxed));
        if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }
    }

    /// Instantaneous volume tier (`0..=3`) for live metering UI.
    pub fn current_tier(&self, sensitivity: f32) -> u8 {
        classify_level(self.level(), sensitivity)
    }

    /// Poll the peak-capture detector with the current microphone level.
    ///
    /// Returns `0` while silent or capturing the rising edge, or `1..=3` once a shout's
    /// peak volume tier is captured. Always returns `0` when inactive.
    pub fn poll_tier(&self, sensitivity: f32) -> u8 {
        if !self.is_active() {
            return 0;
        }
        let lvl = self.level();
        let Ok(mut det) = self.detector.lock() else {
            return 0;
        };
        det.poll_at(lvl, sensitivity, Instant::now())
    }
}

impl Drop for MicMonitor {
    fn drop(&mut self) {
        self.set_armed(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_level_respects_thresholds_and_sensitivity() {
        assert_eq!(classify_level(0.0, 1.0), 0);
        assert_eq!(classify_level(-0.5, 1.0), 0);
        assert_eq!(classify_level(f32::NAN, 1.0), 0);
        assert_eq!(classify_level(LOW_THRESHOLD - 0.001, 1.0), 0);
        assert_eq!(classify_level(LOW_THRESHOLD, 1.0), 1);
        assert_eq!(classify_level(MID_THRESHOLD, 1.0), 2);
        assert_eq!(classify_level(HIGH_THRESHOLD, 1.0), 3);

        // 2x sensitivity halves the required raw amplitude.
        assert_eq!(classify_level(HIGH_THRESHOLD * 0.5, 2.0), 3);
        // 0.5x sensitivity doubles the required raw amplitude.
        assert_eq!(classify_level(MID_THRESHOLD, 0.5), 1);
        assert_eq!(classify_level(LOW_THRESHOLD, 0.5), 0);
        // Non-finite sensitivity falls back to 1.0.
        assert_eq!(classify_level(HIGH_THRESHOLD, f32::NAN), 3);
    }

    #[test]
    fn peak_capture_waits_for_rising_edge_and_emits_peak_tier() {
        let mut det = PeakDetector::new();
        let t0 = Instant::now();

        // Silence stays idle.
        assert_eq!(det.poll_at(0.02, 1.0, t0), 0);

        // Voice starts at Tier 1: enters capture window, does NOT emit Tier 1 prematurely.
        assert_eq!(det.poll_at(0.15, 1.0, t0 + Duration::from_millis(10)), 0);

        // 50ms later, voice rises into Tier 2: still capturing in case it hits Tier 3.
        assert_eq!(det.poll_at(0.35, 1.0, t0 + Duration::from_millis(60)), 0);

        // 90ms after start, voice drops back down; peak remains Tier 2.
        assert_eq!(det.poll_at(0.05, 1.0, t0 + Duration::from_millis(100)), 0);

        // Once the 120ms capture window elapses, emits the captured peak (Tier 2).
        assert_eq!(det.poll_at(0.05, 1.0, t0 + Duration::from_millis(130)), 2);

        // Cooldown suppresses immediate re-trigger.
        assert_eq!(det.poll_at(0.60, 1.0, t0 + Duration::from_millis(200)), 0);
    }

    #[test]
    fn peak_capture_emits_tier_1_when_voice_stays_soft() {
        let mut det = PeakDetector::new();
        let t0 = Instant::now();
        assert_eq!(det.poll_at(0.15, 1.0, t0), 0);
        assert_eq!(det.poll_at(0.18, 1.0, t0 + Duration::from_millis(80)), 0);
        assert_eq!(det.poll_at(0.16, 1.0, t0 + CAPTURE_WINDOW), 1);
    }

    #[test]
    fn peak_capture_emits_tier_3_immediately_on_loud_shout() {
        let mut det = PeakDetector::new();
        let t0 = Instant::now();
        // Starts at Tier 1 -> hits Tier 3 30ms later -> emits Tier 3 right away!
        assert_eq!(det.poll_at(0.15, 1.0, t0), 0);
        assert_eq!(det.poll_at(0.60, 1.0, t0 + Duration::from_millis(30)), 3);

        // Reset clears cooldown immediately (as disarm/rearm does).
        det.reset();
        assert_eq!(det.poll_at(0.70, 1.0, t0 + Duration::from_millis(40)), 3);
    }

    #[test]
    fn peak_capture_recovers_after_cooldown() {
        let mut det = PeakDetector::new();
        let t0 = Instant::now();
        assert_eq!(det.poll_at(0.75, 1.0, t0), 3);
        assert_eq!(det.poll_at(0.75, 1.0, t0 + Duration::from_millis(100)), 0);
        // First poll at/after cooldown ends returns to Idle; next poll can trigger again.
        assert_eq!(det.poll_at(0.0, 1.0, t0 + COOLDOWN_WINDOW), 0);
        assert_eq!(det.poll_at(0.75, 1.0, t0 + COOLDOWN_WINDOW + Duration::from_millis(16)), 3);
    }

    #[test]
    fn monitor_starts_disarmed_and_idle() {
        let mut mon = MicMonitor::new();
        assert!(!mon.is_armed());
        assert!(!mon.is_active());
        assert_eq!(mon.level(), 0.0);
        assert_eq!(mon.current_tier(1.0), 0);
        assert_eq!(mon.poll_tier(1.0), 0);
        assert!(mon.error().is_none());

        // Disarming when already disarmed is a clean no-op.
        mon.set_armed(false);
        assert!(!mon.is_armed());
        assert!(!mon.is_active());
        assert_eq!(mon.level(), 0.0);
    }

    #[test]
    fn buffer_amplitude_and_envelope_behave_smoothly() {
        assert_eq!(buffer_amplitude(&[]), 0.0);
        assert_eq!(buffer_amplitude(&[0.0; 64]), 0.0);
        assert!((buffer_amplitude(&[0.5; 64]) - 0.5).abs() < 1e-5);
        assert!((buffer_amplitude(&[-1.0; 16]) - 1.0).abs() < 1e-5);
        // Fast attack vs slower release.
        let attacked = update_envelope(0.0, 0.8);
        assert!(attacked >= 0.55, "fast attack: {attacked}");
        let released = update_envelope(0.8, 0.0);
        assert!(released > 0.5 && released < 0.8, "smooth release: {released}");
    }
}
