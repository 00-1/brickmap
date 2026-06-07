//! cpal audio output (E16): drives the [`crate::audio::Drone`] through a cpal output stream,
//! synthesising in real time in the audio callback. **Desktop + Android** (cpal's AAudio backend
//! — Android gets its JavaVM/context from the android-native-activity via `ndk-context`); web uses
//! Web Audio (see `controls`).
//!
//! Params (volume / heaviness / murk / enabled) live in shared atomics so the UI thread can
//! nudge them lock-free while the audio thread reads them each block. Starting is fallible
//! and non-fatal: no output device → we just stay silent (the rest of the engine runs).

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Lock-free shared audio params (f32s stored as bits).
struct Shared {
    enabled: AtomicBool,
    volume: AtomicU32,
    drive: AtomicU32,
    tone: AtomicU32,
    intensity: AtomicU32,
}

impl Shared {
    fn store(a: &AtomicU32, v: f32) {
        a.store(v.to_bits(), Ordering::Relaxed);
    }
    fn load(a: &AtomicU32) -> f32 {
        f32::from_bits(a.load(Ordering::Relaxed))
    }
}

/// Owns the live cpal stream (kept alive by holding it) + the shared params.
pub struct AudioEngine {
    _stream: cpal::Stream,
    shared: Arc<Shared>,
}

impl AudioEngine {
    /// Start the drone for `seed`. Returns `None` (silently) if no output device/stream is
    /// available, so a machine without audio still runs everything else.
    pub fn start(seed: u32) -> Option<AudioEngine> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let sample_rate = supported.sample_rate();
        let channels = supported.channels() as usize;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let shared = Arc::new(Shared {
            enabled: AtomicBool::new(true),
            // Defaults match the web house mix: loud-ish, heavy distortion, fairly open.
            volume: AtomicU32::new(0.85f32.to_bits()),
            drive: AtomicU32::new(1.9f32.to_bits()),
            tone: AtomicU32::new(0.7f32.to_bits()),
            intensity: AtomicU32::new(0.0f32.to_bits()),
        });

        let mut drone = crate::audio::Drone::new(seed, sample_rate);
        let err = |e| log::error!("audio stream error: {e}");

        // Per-block writer: pull params, then fill `channels`-interleaved frames — L/R to the
        // first two channels, a mono fold to any extras.
        let s2 = shared.clone();
        let write_f32 = move |data: &mut [f32]| {
            drone.set_volume(Shared::load(&s2.volume));
            drone.set_drive(Shared::load(&s2.drive));
            drone.set_tone(Shared::load(&s2.tone));
            drone.set_intensity(Shared::load(&s2.intensity));
            let on = s2.enabled.load(Ordering::Relaxed);
            for frame in data.chunks_mut(channels) {
                let [l, r] = if on { drone.next_frame() } else { [0.0, 0.0] };
                for (i, out) in frame.iter_mut().enumerate() {
                    *out = match i {
                        0 => l,
                        1 => r,
                        _ => 0.5 * (l + r),
                    };
                }
            }
        };

        let stream = match format {
            cpal::SampleFormat::F32 => {
                let mut w = write_f32;
                device
                    .build_output_stream(&config, move |d: &mut [f32], _| w(d), err, None)
                    .ok()?
            }
            cpal::SampleFormat::I16 => {
                let mut w = write_f32;
                let mut buf: Vec<f32> = Vec::new();
                device
                    .build_output_stream(
                        &config,
                        move |d: &mut [i16], _| {
                            buf.resize(d.len(), 0.0);
                            w(&mut buf);
                            for (o, &v) in d.iter_mut().zip(buf.iter()) {
                                *o = (v.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            }
                        },
                        err,
                        None,
                    )
                    .ok()?
            }
            cpal::SampleFormat::U16 => {
                let mut w = write_f32;
                let mut buf: Vec<f32> = Vec::new();
                device
                    .build_output_stream(
                        &config,
                        move |d: &mut [u16], _| {
                            buf.resize(d.len(), 0.0);
                            w(&mut buf);
                            for (o, &v) in d.iter_mut().zip(buf.iter()) {
                                let n = (v.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32;
                                *o = n as u16;
                            }
                        },
                        err,
                        None,
                    )
                    .ok()?
            }
            _ => return None,
        };
        stream.play().ok()?;
        log::info!("audio: drone playing ({sample_rate} Hz, {channels} ch)");
        Some(AudioEngine {
            _stream: stream,
            shared,
        })
    }

    pub fn set_volume(&self, v: f32) {
        Shared::store(&self.shared.volume, v.clamp(0.0, 1.0));
    }
    pub fn set_drive(&self, m: f32) {
        Shared::store(&self.shared.drive, m);
    }
    pub fn set_tone(&self, t: f32) {
        Shared::store(&self.shared.tone, t);
    }
    /// Reactive intensity `0..1` (E16): the camera's flight state, read each audio block.
    pub fn set_intensity(&self, x: f32) {
        Shared::store(&self.shared.intensity, x);
    }
    /// Flip mute on/off; returns the new "enabled" state.
    pub fn toggle(&self) -> bool {
        let now = !self.shared.enabled.load(Ordering::Relaxed);
        self.shared.enabled.store(now, Ordering::Relaxed);
        now
    }
}
