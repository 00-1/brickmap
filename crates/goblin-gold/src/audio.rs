//! Phase-4 audio **playback wiring** — makes the re-authored SFX ([`crate::sfx`]) and generative
//! music ([`crate::music`]) actually *sound* in the live app, through a cpal output stream
//! (desktop + Android's AAudio backend — the same path `scraped-again`'s drone uses).
//!
//! Two layers, split so the testable half is testable:
//! - [`Mixer`] — the **pure** mixing core: one looping music bed + a handful of one-shot SFX
//!   voices summed to mono and fanned across the output channels. No platform I/O, so its
//!   wrap/voice-retire/clamp logic is unit-tested without a sound card.
//! - [`Player`] — the cpal wiring around it (open the default device, run the [`Mixer`] in the
//!   audio callback, accept commands from the UI thread over a tiny mutex-guarded queue). This is
//!   the **device-only** half: it builds + can't crash a machine with no audio (returns silently),
//!   but whether it *sounds right* is the owner's by-ear check (`OWNER-EYEBALL.md`), exactly like
//!   the immersive-fullscreen JNI — built blind, confirmed on device.
//!
//! The note schedules + timbres are already proven/banked (`synth.rs` byte-exact, `music.rs`/`sfx.rs`
//! perceptual WAV protos); this module is purely the connective tissue that plays them.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::sfx::Sfx;

/// A loud-enough but headroom-leaving master gain on the summed mix (SFX + music), so several
/// concurrent voices over the bed don't slam the clamp.
const MASTER: f32 = 0.8;
/// The music bed sits under the SFX (they're the foreground feedback).
const MUSIC_GAIN: f32 = 0.5;
/// Cap simultaneous one-shot voices — drop the oldest past this (bounds per-sample cost; a math
/// drill never needs more than a few blips at once).
const MAX_VOICES: usize = 8;
/// The looping music bed's target length before bar-rounding (seconds). Long enough not to feel
/// like a tight loop, short enough to render on a screen transition without a noticeable hitch.
const BED_SECS: f32 = 8.0;

/// One playing mono buffer + its read cursor.
struct Voice {
    buf: Arc<Vec<f32>>,
    pos: usize,
}

/// The pure mixing core: a looping music bed plus transient one-shot voices, summed to mono.
/// Drives one block at a time via [`Mixer::fill`]. No platform dependency — fully unit-testable.
#[derive(Default)]
pub struct Mixer {
    /// The looping bed (re-loops at its end); `None` = no music.
    music: Option<Voice>,
    /// Active one-shot SFX, retired as they finish.
    voices: Vec<Voice>,
}

impl Mixer {
    /// Start (or replace) the looping music bed. `None` stops the music.
    fn set_music(&mut self, buf: Option<Arc<Vec<f32>>>) {
        self.music = buf
            .filter(|b| !b.is_empty())
            .map(|buf| Voice { buf, pos: 0 });
    }

    /// Trigger a one-shot SFX buffer. Past [`MAX_VOICES`] the oldest voice is dropped so cost stays
    /// bounded (and a stuck stream can't grow without limit).
    fn trigger(&mut self, buf: Arc<Vec<f32>>) {
        if buf.is_empty() {
            return;
        }
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        self.voices.push(Voice { buf, pos: 0 });
    }

    /// One mono sample: sum the live one-shots (retiring any that just ended) + the looping bed.
    fn next_sample(&mut self) -> f32 {
        let mut s = 0.0f32;
        // One-shots — advance each, keep only those still playing.
        self.voices.retain_mut(|v| {
            if let Some(&x) = v.buf.get(v.pos) {
                s += x;
                v.pos += 1;
                v.pos < v.buf.len()
            } else {
                false
            }
        });
        // Looping bed.
        if let Some(m) = self.music.as_mut() {
            // `set_music` guarantees a non-empty buffer, so the modulo is safe.
            if m.pos >= m.buf.len() {
                m.pos = 0;
            }
            s += m.buf[m.pos] * MUSIC_GAIN;
            m.pos += 1;
        }
        (s * MASTER).clamp(-1.0, 1.0)
    }

    /// Fill an interleaved output block of `channels`-wide frames (mono fanned to every channel).
    pub fn fill(&mut self, out: &mut [f32], channels: usize) {
        let channels = channels.max(1);
        for frame in out.chunks_mut(channels) {
            let s = self.next_sample();
            for o in frame.iter_mut() {
                *o = s;
            }
        }
    }
}

/// A command from the UI thread to the audio callback (drained at the top of each block).
enum Cmd {
    Sfx(Arc<Vec<f32>>),
    Music(Option<Arc<Vec<f32>>>),
}

/// Render a seamless-ish looping music bed for `name` at `sample_rate` — rounded to a whole number
/// of bars so the loop point lands on a downbeat (the pad re-triggers there). Empty for an unknown
/// scene. Pure (no I/O), so it can be cached + rendered off the audio thread.
fn render_bed(name: &str, sample_rate: u32) -> Vec<f32> {
    // Unknown scene → no bed (rather than a buffer of silence).
    let Some(tempo) = crate::synth::tempo_of(name) else {
        return Vec::new();
    };
    // Round the target length up to a whole bar so the loop seam sits on a downbeat.
    let bar = (60.0 / tempo as f32) / 4.0 * 16.0;
    let seconds = (BED_SECS / bar).ceil().max(1.0) * bar;
    crate::music::render(name, sample_rate, seconds, 1.0)
}

/// The live audio engine: owns the cpal stream + the UI→audio command channel. Native-only; on a
/// machine with no output device it simply isn't created (the game runs silent).
#[cfg(not(target_arch = "wasm32"))]
pub struct Player {
    _stream: cpal::Stream,
    cmd: Arc<Mutex<Vec<Cmd>>>,
    enabled: Arc<AtomicBool>,
    sample_rate: u32,
    /// Cache of rendered music beds by scene name, so returning to a screen doesn't re-synthesise.
    beds: Mutex<std::collections::HashMap<String, Arc<Vec<f32>>>>,
    /// The scene currently set as the bed, to skip redundant re-sends.
    current_scene: Mutex<Option<String>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Player {
    /// Open the default output device and start the mixer. Returns `None` (silently) if there's no
    /// device / the stream won't build, so the game still runs everywhere.
    pub fn start() -> Option<Player> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let sample_rate = supported.sample_rate();
        let channels = supported.channels() as usize;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();

        let cmd: Arc<Mutex<Vec<Cmd>>> = Arc::new(Mutex::new(Vec::new()));
        let enabled = Arc::new(AtomicBool::new(true));
        let err = |e| log::error!("gg audio stream error: {e}");

        // The mixer lives on the audio thread (moved into the callback). Each block: drain pending
        // commands, then fill — or write silence while muted.
        let mut mixer = Mixer::default();
        let cmd_cb = cmd.clone();
        let en_cb = enabled.clone();
        let mut fill = move |data: &mut [f32]| {
            {
                // Tiny critical section: swap the queue out and apply it.
                let mut q = cmd_cb.lock().unwrap_or_else(|e| e.into_inner());
                if !q.is_empty() {
                    for c in q.drain(..) {
                        match c {
                            Cmd::Sfx(b) => mixer.trigger(b),
                            Cmd::Music(b) => mixer.set_music(b),
                        }
                    }
                }
            }
            if en_cb.load(Ordering::Relaxed) {
                mixer.fill(data, channels);
            } else {
                data.fill(0.0);
            }
        };

        let stream = match format {
            cpal::SampleFormat::F32 => device
                .build_output_stream(&config, move |d: &mut [f32], _| fill(d), err, None)
                .ok()?,
            cpal::SampleFormat::I16 => {
                let mut buf: Vec<f32> = Vec::new();
                device
                    .build_output_stream(
                        &config,
                        move |d: &mut [i16], _| {
                            buf.resize(d.len(), 0.0);
                            fill(&mut buf);
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
                let mut buf: Vec<f32> = Vec::new();
                device
                    .build_output_stream(
                        &config,
                        move |d: &mut [u16], _| {
                            buf.resize(d.len(), 0.0);
                            fill(&mut buf);
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
        log::info!("gg audio: mixer playing ({sample_rate} Hz, {channels} ch)");
        Some(Player {
            _stream: stream,
            cmd,
            enabled,
            sample_rate,
            beds: Mutex::new(std::collections::HashMap::new()),
            current_scene: Mutex::new(None),
        })
    }

    /// Queue a command for the audio thread.
    fn send(&self, c: Cmd) {
        if let Ok(mut q) = self.cmd.lock() {
            q.push(c);
        }
    }

    /// Fire a one-shot SFX (rendered at the stream's sample rate, off the audio thread).
    pub fn play(&self, sfx: Sfx) {
        let buf = crate::sfx::render(&sfx, self.sample_rate, 1.0);
        self.send(Cmd::Sfx(Arc::new(buf)));
    }

    /// Set the looping music bed to `scene` (a [`crate::synth::STYLE_IDS`] name), or `None` to stop.
    /// Cached + a no-op when already on that scene, so per-frame calls are cheap.
    pub fn set_scene(&self, scene: Option<&str>) {
        {
            let cur = self.current_scene.lock().unwrap_or_else(|e| e.into_inner());
            if cur.as_deref() == scene {
                return;
            }
        }
        let bed = scene.map(|name| {
            let mut beds = self.beds.lock().unwrap_or_else(|e| e.into_inner());
            beds.entry(name.to_string())
                .or_insert_with(|| Arc::new(render_bed(name, self.sample_rate)))
                .clone()
        });
        self.send(Cmd::Music(bed));
        *self.current_scene.lock().unwrap_or_else(|e| e.into_inner()) = scene.map(str::to_string);
    }

    /// Flip mute; returns the new enabled state.
    pub fn toggle(&self) -> bool {
        let now = !self.enabled.load(Ordering::Relaxed);
        self.enabled.store(now, Ordering::Relaxed);
        now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(vals: &[f32]) -> Arc<Vec<f32>> {
        Arc::new(vals.to_vec())
    }

    #[test]
    fn empty_mixer_is_silent() {
        let mut m = Mixer::default();
        let mut out = [1.0f32; 16];
        m.fill(&mut out, 2);
        assert!(out.iter().all(|&s| s == 0.0), "idle mixer should be silent");
    }

    #[test]
    fn one_shot_plays_then_retires() {
        let mut m = Mixer::default();
        m.trigger(buf(&[0.5, 0.5, 0.5]));
        assert_eq!(m.voices.len(), 1);
        let mut out = [0.0f32; 3]; // mono: 3 frames
        m.fill(&mut out, 1);
        assert!(out.iter().all(|&s| (s - 0.5 * MASTER).abs() < 1e-6));
        // After draining its 3 samples the voice is gone, and further fills are silent.
        assert_eq!(m.voices.len(), 0);
        let mut more = [9.0f32; 4];
        m.fill(&mut more, 1);
        assert!(more.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn music_bed_loops() {
        let mut m = Mixer::default();
        m.set_music(Some(buf(&[1.0, -1.0])));
        let mut out = [0.0f32; 5];
        m.fill(&mut out, 1);
        // The 2-sample bed repeats: 1,-1,1,-1,1 (×MUSIC_GAIN×MASTER, clamped).
        let g = MUSIC_GAIN * MASTER;
        let want = [g, -g, g, -g, g];
        for (o, w) in out.iter().zip(want) {
            assert!((o - w).abs() < 1e-6, "loop mismatch: {o} vs {w}");
        }
    }

    #[test]
    fn empty_buffers_are_ignored() {
        let mut m = Mixer::default();
        m.trigger(buf(&[])); // no voice
        m.set_music(Some(buf(&[]))); // no bed (avoids a modulo-by-zero)
        assert_eq!(m.voices.len(), 0);
        let mut out = [3.0f32; 4];
        m.fill(&mut out, 2);
        assert!(out.iter().all(|&s| s == 0.0));
    }

    #[test]
    fn voices_are_capped() {
        let mut m = Mixer::default();
        for _ in 0..(MAX_VOICES + 5) {
            m.trigger(buf(&[0.1, 0.1, 0.1, 0.1]));
        }
        assert_eq!(m.voices.len(), MAX_VOICES, "voice count must stay bounded");
    }

    #[test]
    fn output_is_clamped() {
        let mut m = Mixer::default();
        for _ in 0..MAX_VOICES {
            m.trigger(buf(&[1.0; 4]));
        }
        m.set_music(Some(buf(&[1.0; 4])));
        let mut out = [0.0f32; 4];
        m.fill(&mut out, 1);
        assert!(
            out.iter().all(|&s| (-1.0..=1.0).contains(&s)),
            "mix must never exceed full scale"
        );
    }

    #[test]
    fn bed_renders_bar_aligned_audio() {
        // The looping bed is non-empty for a real scene and silent-length-0 only for an unknown one.
        let bed = render_bed("menu", 22_050);
        assert!(!bed.is_empty(), "a real scene should render a bed");
        assert!(
            render_bed("nope", 22_050).is_empty(),
            "unknown scene → empty"
        );
    }
}
