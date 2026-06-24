//! GG1 music **renderer** (phase 4 audio) — turn a [`crate::synth`] score into an audible mono
//! buffer using GG1's **actual instrument patches** (re-authored from `synth.js`'s `PATCHES` /
//! `renderVoice`): unison/FM/sub/mono engines, an ADSR amp, a state-variable filter with its own
//! cutoff envelope (and the `wub` LFO), per the scene's `patches.{pad,bass,lead}`. The note schedule
//! is the vector-proven [`crate::synth`] score; this is the **perceptual** half, so the tests gate
//! the mechanics (determinism, no clipping, mute → silence, scenes distinct) and the by-ear
//! A/B-vs-web is banked for the owner (the `music_proto` WAVs).
//!
//! Faithful-by-construction (the patch params are documented), but not byte-identical to Web Audio's
//! exact biquad/oscillator DSP — the SVF approximates Web Audio's resonant biquad. Same sample-gen
//! style as `Drone`.

use crate::synth::{self, Role};

const MASTER: f32 = 0.5;

fn hz(midi: i32) -> f32 {
    440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0)
}

/// Deterministic xorshift32 noise for the drums.
struct Noise {
    s: u32,
}
impl Noise {
    fn new(seed: u32) -> Noise {
        Noise { s: seed | 1 }
    }
    fn next(&mut self) -> f32 {
        self.s ^= self.s << 13;
        self.s ^= self.s >> 17;
        self.s ^= self.s << 5;
        (self.s as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[derive(Clone, Copy)]
enum Engine {
    Unison { voices: u32, detune: f32 },
    Fm { ratio: f32, index: f32 },
    Sub,
    Mono,
}
#[derive(Clone, Copy, PartialEq)]
enum Wave {
    Saw,
    Square,
    Triangle,
    Sine,
}
#[derive(Clone, Copy)]
enum FilterKind {
    Lowpass,
    Bandpass,
}
#[derive(Clone, Copy)]
struct Filter {
    kind: FilterKind,
    cut: f32,
    env: f32,
    q: f32,
}
#[derive(Clone, Copy)]
struct Patch {
    engine: Engine,
    wave: Wave,
    gain: f32,
    amp: (f32, f32, f32, f32), // a, d, s, r
    filter: Option<Filter>,
    lfo: Option<(f32, f32)>, // (rate, depth) — the wub
}

fn lp(cut: f32, env: f32, q: f32) -> Option<Filter> {
    Some(Filter {
        kind: FilterKind::Lowpass,
        cut,
        env,
        q,
    })
}

/// GG1's patch definitions (`synth.js` `PATCHES`, the ones the 12 scenes use).
fn patch(name: &str) -> Patch {
    use Engine::*;
    use Wave::*;
    let p = |engine, wave, gain, amp, filter, lfo| Patch {
        engine,
        wave,
        gain,
        amp,
        filter,
        lfo,
    };
    match name {
        "pad" => p(
            Unison {
                voices: 3,
                detune: 12.0,
            },
            Saw,
            0.20,
            (0.6, 0.4, 0.8, 1.2),
            lp(1100.0, 1.4, 2.0),
            None,
        ),
        "padglass" => p(
            Unison {
                voices: 3,
                detune: 7.0,
            },
            Triangle,
            0.20,
            (0.9, 0.6, 0.85, 1.8),
            lp(2000.0, 0.5, 1.0),
            None,
        ),
        "padep" => p(
            Fm {
                ratio: 1.0,
                index: 110.0,
            },
            Sine,
            0.20,
            (0.05, 0.5, 0.6, 0.5),
            lp(1500.0, 0.8, 1.0),
            None,
        ),
        "padpwm" => p(
            Unison {
                voices: 3,
                detune: 9.0,
            },
            Square,
            0.16,
            (0.15, 0.3, 0.7, 0.5),
            lp(2800.0, 1.0, 2.0),
            None,
        ),
        "padorgan" => p(
            Unison {
                voices: 2,
                detune: 5.0,
            },
            Square,
            0.18,
            (0.01, 0.1, 0.8, 0.25),
            Some(Filter {
                kind: FilterKind::Bandpass,
                cut: 760.0,
                env: 0.15,
                q: 7.0,
            }),
            None,
        ),
        "croon" => p(
            Mono,
            Triangle,
            0.22,
            (0.045, 0.22, 0.4, 0.28),
            lp(1400.0, 0.6, 1.0),
            None,
        ),
        "bass" => p(
            Mono,
            Saw,
            0.38,
            (0.004, 0.2, 0.7, 0.1),
            lp(520.0, 1.0, 1.0),
            None,
        ),
        "bell" => p(
            Fm {
                ratio: 2.0,
                index: 220.0,
            },
            Sine,
            0.26,
            (0.002, 0.5, 0.0, 0.3),
            None,
            None,
        ),
        "lead" => p(
            Mono,
            Square,
            0.30,
            (0.01, 0.1, 0.6, 0.12),
            lp(2200.0, 1.5, 3.0),
            None,
        ),
        "wub" => p(
            Sub,
            Saw,
            0.40,
            (0.005, 0.05, 0.85, 0.1),
            Some(Filter {
                kind: FilterKind::Lowpass,
                cut: 600.0,
                env: 0.0,
                q: 12.0,
            }),
            Some((7.0, 700.0)),
        ),
        "chip" => p(
            Mono,
            Square,
            0.24,
            (0.001, 0.06, 0.0, 0.02),
            lp(4200.0, 2.0, 2.0),
            None,
        ),
        _ => p(
            Mono,
            Square,
            0.3,
            (0.01, 0.1, 0.6, 0.12),
            lp(2000.0, 1.0, 1.0),
            None,
        ),
    }
}

/// Each scene's `(pad, bass, lead)` patch names (`CONTEXTS[*].patches`).
fn patches_of(scene: &str) -> (&'static str, &'static str, &'static str) {
    match scene {
        "menu" => ("padglass", "bass", "bell"),
        "arena" => ("pad", "wub", "lead"),
        "lofi" => ("padep", "bass", "croon"),
        "ambient" => ("padglass", "bass", "bell"),
        "chiptune" => ("padpwm", "bass", "chip"),
        "synthwave" => ("pad", "bass", "lead"),
        "dubstep" => ("padorgan", "wub", "lead"),
        "dnb" => ("padep", "wub", "lead"),
        "bigroom" => ("pad", "bass", "lead"),
        "boss8bit" => ("padpwm", "bass", "chip"),
        "tropical" => ("padglass", "bass", "bell"),
        "techno" => ("padorgan", "wub", "lead"),
        _ => ("pad", "bass", "lead"),
    }
}

fn wave_sample(w: Wave, ph: f32) -> f32 {
    let frac = ph - ph.floor();
    match w {
        Wave::Sine => (std::f32::consts::TAU * ph).sin(),
        Wave::Saw => 2.0 * frac - 1.0,
        Wave::Square => {
            if frac < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Triangle => 1.0 - 4.0 * (frac - 0.5).abs(),
    }
}

/// ADSR amp envelope (linear segments), peak 1.0; releases after `hold`.
fn amp_env(t: f32, hold: f32, a: f32, d: f32, s: f32, r: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < a {
        t / a.max(1e-4)
    } else if t < a + d {
        1.0 - (1.0 - s) * ((t - a) / d.max(1e-4))
    } else if t < hold {
        s
    } else {
        (s * (1.0 - (t - hold) / r.max(1e-4))).max(0.0)
    }
}

/// Chamberlin state-variable filter step → (lowpass, bandpass). `f = 2 sin(π fc/sr)`, `q1 = 1/Q`.
struct Svf {
    low: f32,
    band: f32,
}
impl Svf {
    fn new() -> Svf {
        Svf {
            low: 0.0,
            band: 0.0,
        }
    }
    fn step(&mut self, input: f32, cutoff: f32, sr: f32, q: f32) -> (f32, f32) {
        // Cap the cutoff to the Chamberlin SVF's stable band (f < 1 ⇒ fc < ~sr/6) so a bright
        // patch's filter envelope can't push it into divergence.
        let fc = cutoff.clamp(20.0, sr * 0.15);
        let f = 2.0 * (std::f32::consts::PI * fc / sr).sin();
        let q1 = (1.0 / q.max(0.5)).clamp(0.1, 2.0);
        let high = input - self.low - q1 * self.band;
        self.band += f * high;
        self.low += f * self.band;
        (self.low, self.band)
    }
}

/// Render one patch note into `buf` from `start`, held for `hold` seconds.
fn add_patch_note(buf: &mut [f32], sr: f32, start: usize, freq: f32, hold: f32, p: &Patch) {
    let (a, d, s, r) = p.amp;
    let total = hold + r + 0.05;
    let n = (total * sr) as usize;
    // Oscillator phase accumulators (unison up to its voice count; FM carrier + modulator).
    let mut phases = [0.0f32; 3];
    let mut fm_mod_ph = 0.0f32;
    let detunes: Vec<f32> = match p.engine {
        Engine::Unison { voices, detune } => (0..voices)
            .map(|i| {
                let cents = (i as f32 - (voices as f32 - 1.0) / 2.0) * detune;
                freq * 2f32.powf(cents / 1200.0)
            })
            .collect(),
        _ => vec![freq],
    };
    let mut svf = Svf::new();
    let dt = 1.0 / sr;
    for i in 0..n {
        let idx = start + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        // Oscillator(s).
        let mut osc = match p.engine {
            Engine::Unison { .. } => {
                let mut sum = 0.0;
                for (k, &df) in detunes.iter().enumerate() {
                    phases[k] += df * dt;
                    sum += wave_sample(p.wave, phases[k]);
                }
                sum / detunes.len() as f32
            }
            Engine::Fm { ratio, index } => {
                fm_mod_ph += freq * ratio * dt;
                let m = (std::f32::consts::TAU * fm_mod_ph).sin() * index;
                // Linear FM: modulate the carrier's instantaneous frequency.
                phases[0] += (freq + m) * dt;
                wave_sample(p.wave, phases[0])
            }
            _ => {
                phases[0] += freq * dt;
                wave_sample(p.wave, phases[0])
            }
        };
        // Filter (cutoff envelope + the wub LFO), via the SVF.
        if let Some(f) = p.filter {
            let mut cut = f.cut;
            if f.env > 0.0 {
                let peak = f.cut * (1.0 + f.env);
                if t < a {
                    cut = f.cut + (peak - f.cut) * (t / a.max(1e-4));
                } else if t < a + d {
                    cut = peak + (f.cut - peak) * ((t - a) / d.max(1e-4));
                }
            }
            if let Some((rate, depth)) = p.lfo {
                cut += depth * (std::f32::consts::TAU * rate * t).sin();
            }
            let (low, band) = svf.step(osc, cut, sr, f.q);
            osc = match f.kind {
                FilterKind::Lowpass => low,
                FilterKind::Bandpass => band,
            };
        }
        buf[idx] += osc * amp_env(t, hold, a, d, s, r) * p.gain;
    }
}

fn add_drum(buf: &mut [f32], sr: f32, start: usize, piece: &str, nz: &mut Noise) {
    let (dur, gain) = match piece {
        "kick" => (0.18, 0.9),
        "snare" => (0.16, 0.5),
        _ => (0.05, 0.3),
    };
    let n = (dur * sr) as usize;
    for i in 0..n {
        let idx = start + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let decay = (-t / (dur * 0.4)).exp();
        let s = match piece {
            "kick" => {
                let f = 120.0 * (-t / 0.03).exp() + 45.0;
                (std::f32::consts::TAU * f * t).sin()
            }
            "snare" => 0.7 * nz.next() + 0.3 * (std::f32::consts::TAU * 180.0 * t).sin(),
            _ => nz.next(),
        };
        buf[idx] += s * decay * gain;
    }
}

/// Render `seconds` of a scene's music to a mono `f32` buffer at `sample_rate`, scaled by `volume`
/// (0 ⇒ silence). Empty buffer for an unknown scene.
pub fn render(name: &str, sample_rate: u32, seconds: f32, volume: f32) -> Vec<f32> {
    let sr = sample_rate as f32;
    let len = (seconds * sr) as usize;
    let mut buf = vec![0.0f32; len];
    let Some(tempo) = synth::tempo_of(name) else {
        return buf;
    };
    if volume <= 0.0 {
        return buf;
    }
    let (pad_n, bass_n, lead_n) = patches_of(name);
    let (pad, bass, lead) = (patch(pad_n), patch(bass_n), patch(lead_n));
    let step_secs = (60.0 / tempo as f32) / 4.0;
    let bar_secs = step_secs * 16.0;
    let steps = (seconds / step_secs).ceil() as usize + 1;
    let Some(score) = synth::voiced_score(name, steps) else {
        return buf;
    };
    let mut nz = Noise::new(0x1234_5678);
    for (step, evs) in score.iter().enumerate() {
        let start = (step as f32 * step_secs * sr) as usize;
        if start >= len {
            break;
        }
        for v in evs {
            match v.role {
                Role::Pad => add_patch_note(&mut buf, sr, start, hz(v.midi), bar_secs, &pad),
                Role::Bass => add_patch_note(&mut buf, sr, start, hz(v.midi), 0.9, &bass),
                Role::Lead => add_patch_note(&mut buf, sr, start, hz(v.midi), 0.16, &lead),
                Role::Drum => add_drum(&mut buf, sr, start, v.piece, &mut nz),
            }
        }
    }
    for x in &mut buf {
        let v = *x * MASTER * volume;
        // Defensive: a filter that rang shouldn't ever leak a non-finite sample.
        *x = if v.is_finite() {
            v.clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::STYLE_IDS;

    const SR: u32 = 22_050;

    fn peak(b: &[f32]) -> f32 {
        b.iter().fold(0.0f32, |m, &s| m.max(s.abs()))
    }
    fn fp(b: &[f32]) -> i64 {
        b.iter()
            .enumerate()
            .map(|(i, &s)| (s * 4096.0) as i64 * (i as i64 % 101 + 1))
            .sum()
    }

    #[test]
    fn every_scene_renders_deterministic_bounded_audio() {
        for name in STYLE_IDS {
            let a = render(name, SR, 2.0, 1.0);
            let b = render(name, SR, 2.0, 1.0);
            assert_eq!(a, b, "{name} render not deterministic");
            assert_eq!(a.len(), (2.0 * SR as f32) as usize);
            assert!(peak(&a) > 0.0, "{name} is silent");
            assert!(peak(&a) <= 1.0, "{name} clips (peak {})", peak(&a));
        }
    }

    #[test]
    fn mute_is_silence_and_unknown_scene_is_empty() {
        assert!(render("lofi", SR, 1.0, 0.0).iter().all(|&s| s == 0.0));
        assert!(render("nope", SR, 1.0, 1.0).iter().all(|&s| s == 0.0));
    }

    #[test]
    fn scenes_render_distinct_music() {
        let prints: Vec<i64> = STYLE_IDS
            .iter()
            .map(|n| fp(&render(n, SR, 2.0, 1.0)))
            .collect();
        for i in 0..prints.len() {
            for j in (i + 1)..prints.len() {
                assert_ne!(
                    prints[i], prints[j],
                    "{} and {} render identical audio",
                    STYLE_IDS[i], STYLE_IDS[j]
                );
            }
        }
    }
}
