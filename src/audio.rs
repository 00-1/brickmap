//! Procedural doom-drone synthesizer (E16, synth stage). Dependency-free DSP that renders
//! a dark, downtuned, hypnotic drone in the spirit of Sleep's *Dopesmoker* — a per-seed
//! "dirge" to sit under the grimy, point-lit world.
//!
//! This module only *generates samples* (`Drone::next_frame` → stereo `[f32; 2]`); it has
//! no platform audio I/O, so it builds and is testable everywhere. Live playback (cpal on
//! native, Web Audio on web) hooks into the same `Drone` next — the hard, taste-driven part
//! is the sound, which the `drone` dev bin renders to a WAV so we can listen + iterate.
//!
//! Signal chain (per channel): stacked detuned oscillators (sub sine + body + power-chord
//! fifth + a faint Phrygian ♭2 for unease) → hard waveshaping distortion (the doom grit) →
//! a slowly-swept resonant low-pass (murk that breathes) → slow amplitude swell. Everything
//! moves on slow LFOs so the drone evolves without ever resolving.

use std::f32::consts::TAU;

const SUB_GAIN: f32 = 0.55;
const BODY_GAIN: f32 = 0.32;
const FIFTH_GAIN: f32 = 0.42;
const OCT_GAIN: f32 = 0.16;
const FLAT2_GAIN: f32 = 0.09; // faint minor-2nd dissonance (Phrygian darkness)

/// A small seeded PRNG (SplitMix64) so each world's dirge is deterministic + its own.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        let u = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        lo + u * (hi - lo)
    }
}

/// One detuned oscillator voice.
#[derive(Clone, Copy)]
struct Voice {
    phase: f32,
    freq: f32,
    /// 0 = sine (sub/body), 1 = sawtooth (everything with grit).
    saw: bool,
    gain: f32,
    /// Equal-power pan position in `[-1, 1]`.
    pan: f32,
}

impl Voice {
    fn sample(&mut self, sr: f32) -> f32 {
        let dt = self.freq / sr;
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let v = if self.saw {
            // Band-limited sawtooth (PolyBLEP) so the heavy distortion downstream adds
            // musical harmonics rather than aliasing fizz.
            2.0 * self.phase - 1.0 - poly_blep(self.phase, dt)
        } else {
            (self.phase * TAU).sin()
        };
        v * self.gain
    }
}

/// PolyBLEP discontinuity correction for a naive ramp at `t` with step `dt`.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// A slow sine LFO in `[lo, hi]`, used to make cutoff/amplitude breathe over many seconds.
#[derive(Clone, Copy)]
struct Lfo {
    phase: f32,
    rate: f32, // Hz (fractions of one — periods of tens of seconds)
    lo: f32,
    hi: f32,
}
impl Lfo {
    fn step(&mut self, sr: f32) -> f32 {
        self.phase += self.rate / sr;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let s = 0.5 - 0.5 * (self.phase * TAU).cos(); // 0..1, smooth
        self.lo + s * (self.hi - self.lo)
    }
}

/// A state-variable low-pass filter (one per channel), for a resonant, sweepable murk.
#[derive(Clone, Copy, Default)]
struct Svf {
    low: f32,
    band: f32,
}
impl Svf {
    /// Process one sample. `cutoff` in Hz, `res` in `[0, 1]` (higher = more resonance).
    fn lowpass(&mut self, x: f32, cutoff: f32, res: f32, sr: f32) -> f32 {
        let f = 2.0 * (std::f32::consts::PI * (cutoff.min(sr * 0.45) / sr)).sin();
        let q = 1.0 - res * 0.97;
        self.low += f * self.band;
        let high = x - self.low - q * self.band;
        self.band += f * high;
        self.low
    }
}

/// The streaming doom-drone voice. Build with [`Drone::new`], pull stereo frames with
/// [`Drone::next_frame`]. Cheap enough to run per audio-callback on weak hardware.
pub struct Drone {
    sr: f32,
    voices: Vec<Voice>,
    /// Filter cutoff sweep + amplitude swell, both very slow.
    cutoff_lfo: Lfo,
    amp_lfo: Lfo,
    filt_l: Svf,
    filt_r: Svf,
    drive: f32,
    res: f32,
    master: f32,
}

impl Drone {
    /// The lowest, heaviest roots we pick from (Hz) — sub-bass territory so it crushes.
    /// Roughly E1, F1, F♯1, G1, A1; the seed leans the choice low.
    const ROOTS: [f32; 5] = [41.20, 43.65, 46.25, 49.00, 55.00];

    pub fn new(seed: u32, sample_rate: u32) -> Drone {
        let sr = sample_rate as f32;
        let mut rng = Rng::new(seed as u64);

        // Lean low: pick a root, biased toward the bottom of the set.
        let pick = (rng.range(0.0, 1.0).powf(1.6) * Drone::ROOTS.len() as f32) as usize;
        let root = Drone::ROOTS[pick.min(Drone::ROOTS.len() - 1)];

        // Per-seed detune spread + stereo width, so each dirge beats differently.
        let det = rng.range(0.0015, 0.006); // fractional detune of the saw pairs
        let flat2 = root * 2.0f32.powf(1.0 / 12.0); // minor 2nd above root (Phrygian unease)

        let mk = |freq: f32, saw: bool, gain: f32, pan: f32, phase: f32| Voice {
            phase,
            freq,
            saw,
            gain,
            pan,
        };
        let voices = vec![
            // Weight: sub-octave + fundamental sines (centred).
            mk(root * 0.5, false, SUB_GAIN, 0.0, rng.range(0.0, 1.0)),
            mk(root, false, BODY_GAIN, 0.0, rng.range(0.0, 1.0)),
            // Detuned saw pair at the root, panned apart for width.
            mk(root * (1.0 - det), true, 0.5, -0.5, rng.range(0.0, 1.0)),
            mk(
                root * (1.0 + det * 1.3),
                true,
                0.5,
                0.5,
                rng.range(0.0, 1.0),
            ),
            // Power-chord fifth, also a detuned pair.
            mk(
                root * 1.5 * (1.0 - det),
                true,
                FIFTH_GAIN,
                -0.3,
                rng.range(0.0, 1.0),
            ),
            mk(
                root * 1.5 * (1.0 + det),
                true,
                FIFTH_GAIN,
                0.3,
                rng.range(0.0, 1.0),
            ),
            // A little octave-up presence so the distortion has something to bite.
            mk(
                root * 2.0 * (1.0 + det * 0.5),
                true,
                OCT_GAIN,
                0.15,
                rng.range(0.0, 1.0),
            ),
            // Faint, dissonant ♭2 — the dread note. Centred and quiet.
            mk(flat2, true, FLAT2_GAIN, 0.0, rng.range(0.0, 1.0)),
        ];

        Drone {
            sr,
            voices,
            // Cutoff breathes between murky and merely-dark over ~22 s; amp swells over ~14 s.
            cutoff_lfo: Lfo {
                phase: rng.range(0.0, 1.0),
                rate: 1.0 / rng.range(18.0, 26.0),
                lo: 150.0,
                hi: 1200.0,
            },
            amp_lfo: Lfo {
                phase: rng.range(0.0, 1.0),
                rate: 1.0 / rng.range(11.0, 17.0),
                lo: 0.62,
                hi: 1.0,
            },
            filt_l: Svf::default(),
            filt_r: Svf::default(),
            drive: rng.range(2.6, 4.2),
            res: rng.range(0.35, 0.6),
            // Headroom so the resonant filter's overshoot doesn't slam the hard clamp
            // (that clips harshly — unlike the musical tanh saturation upstream).
            master: 0.68,
        }
    }

    /// Render the next stereo frame (`[left, right]`), each in roughly `[-1, 1]`.
    pub fn next_frame(&mut self) -> [f32; 2] {
        let mut l = 0.0f32;
        let mut r = 0.0f32;
        for v in &mut self.voices {
            let s = v.sample(self.sr);
            // Equal-power pan.
            let p = (v.pan * 0.5 + 0.5) * (std::f32::consts::PI * 0.5);
            l += s * p.cos();
            r += s * p.sin();
        }
        // Tame the summed level before distortion, then waveshape hard for the doom grit.
        let pre = 0.42;
        let dl = (self.drive * l * pre).tanh();
        let dr = (self.drive * r * pre).tanh();

        let cutoff = self.cutoff_lfo.step(self.sr);
        let amp = self.amp_lfo.step(self.sr);
        // Step the second LFO read for R off the same value (mono modulation, stereo audio).
        let fl = self.filt_l.lowpass(dl, cutoff, self.res, self.sr);
        let fr = self.filt_r.lowpass(dr, cutoff, self.res, self.sr);

        let g = self.master * amp;
        [(fl * g).clamp(-1.0, 1.0), (fr * g).clamp(-1.0, 1.0)]
    }

    /// Render `seconds` of audio into an interleaved stereo `Vec<f32>` (L, R, L, R, …).
    /// Includes short fade-in/out so an offline render starts/ends without a click.
    pub fn render(seed: u32, sample_rate: u32, seconds: f32) -> Vec<f32> {
        let mut d = Drone::new(seed, sample_rate);
        let n = (seconds * sample_rate as f32) as usize;
        let fade = (sample_rate as usize / 2).min(n / 2).max(1); // ~0.5 s
        let mut out = Vec::with_capacity(n * 2);
        for i in 0..n {
            let [mut l, mut r] = d.next_frame();
            let env =
                (i.min(fade) as f32 / fade as f32).min((n - i).min(fade) as f32 / fade as f32);
            l *= env;
            r *= env;
            out.push(l);
            out.push(r);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drone_is_finite_and_bounded() {
        let mut d = Drone::new(1337, 44_100);
        let mut peak = 0.0f32;
        for _ in 0..44_100 {
            let [l, r] = d.next_frame();
            assert!(l.is_finite() && r.is_finite());
            peak = peak.max(l.abs()).max(r.abs());
        }
        // It should make real sound (not silence) but never exceed full scale.
        assert!(peak > 0.1, "drone is suspiciously quiet: {peak}");
        assert!(peak <= 1.0, "drone clipped past full scale: {peak}");
    }

    #[test]
    fn different_seeds_give_different_dirges() {
        let a = Drone::render(1, 8_000, 0.25);
        let b = Drone::render(2, 8_000, 0.25);
        assert_ne!(a, b, "two seeds produced the identical drone");
    }
}
