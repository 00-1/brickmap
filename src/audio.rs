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
    base_drive: f32,
    res: f32,
    master: f32,
    // --- Live, settable params (the in-game audio sliders drive these) ---
    /// Master volume `0..1`.
    vol: f32,
    /// Distortion/heaviness multiplier on the per-seed base drive (`~0.3..2.5`).
    drive_mul: f32,
    /// Murk ↔ openness `0..1`: scales the filter cutoff sweep (low = darker/muffled).
    tone: f32,
    /// Reactive intensity `0..1` (E16): driven by the camera's flight (speed + altitude). Opens
    /// the cutoff and lifts the swell a touch, so the dirge breathes with the world rather than
    /// sitting static. `_s` is the per-sample-smoothed value (no zipper noise on param steps).
    intensity: f32,
    intensity_s: f32,
    /// Warp `0..1` (E18×E16): set high near a max-wobble ("warping") colossus. Drives a new
    /// aggressive modulation — heavier drive + a throbbing tremolo — so those zones sound as
    /// intense/unstable as they look. `_s` smoothed; `trem_phase` is the tremolo LFO.
    warp: f32,
    warp_s: f32,
    trem_phase: f32,
    /// Ethereal `0..1` (E10): set high in the rare pristine pockets. A distinct, *unmistakable*
    /// shift — guts the distortion, throws the filter wide open, lightens the sub + dread voices,
    /// and adds a glassy shimmer — so a pristine pocket sounds airy/holy, not like the doom drone.
    ethereal: f32,
    ethereal_s: f32,
    shimmer_phase: f32,
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
            base_drive: rng.range(2.6, 4.2),
            res: rng.range(0.35, 0.6),
            // Headroom so the resonant filter's overshoot doesn't slam the hard clamp
            // (that clips harshly — unlike the musical tanh saturation upstream).
            master: 0.68,
            vol: 0.9,
            drive_mul: 1.0,
            tone: 0.4,
            intensity: 0.0,
            intensity_s: 0.0,
            warp: 0.0,
            warp_s: 0.0,
            trem_phase: 0.0,
            ethereal: 0.0,
            ethereal_s: 0.0,
            shimmer_phase: 0.0,
        }
    }

    /// Master volume, `0..1`.
    pub fn set_volume(&mut self, v: f32) {
        self.vol = v.clamp(0.0, 1.0);
    }
    /// Heaviness: multiplier on the per-seed distortion drive (clamped `0.3..2.5`).
    pub fn set_drive(&mut self, m: f32) {
        self.drive_mul = m.clamp(0.3, 2.5);
    }
    /// Murk ↔ openness, `0..1` (low = darker/muffled, high = brighter).
    pub fn set_tone(&mut self, t: f32) {
        self.tone = t.clamp(0.0, 1.0);
    }
    /// Reactive intensity, `0..1` (E16): the world's flight state (speed + altitude). Smoothed
    /// internally, so callers can set it per-frame without zipper noise.
    pub fn set_intensity(&mut self, x: f32) {
        self.intensity = x.clamp(0.0, 1.0);
    }
    /// Warp amount, `0..1` (E18): proximity to a max-wobble "warping" colossus. Adds heavier
    /// drive + a throbbing tremolo. Smoothed internally.
    pub fn set_warp(&mut self, x: f32) {
        self.warp = x.clamp(0.0, 1.0);
    }
    /// Ethereal amount, `0..1` (E10): the rare pristine pockets. Transforms the drone to an airy,
    /// clean, shimmering voice. Smoothed internally.
    pub fn set_ethereal(&mut self, x: f32) {
        self.ethereal = x.clamp(0.0, 1.0);
    }

    /// Render the next stereo frame (`[left, right]`), each in roughly `[-1, 1]`.
    pub fn next_frame(&mut self) -> [f32; 2] {
        // Ethereal (E10): smooth toward target (~0.2 s). In a pristine pocket it lightens the deep
        // voices (sub = voice 0, the dread ♭2 = voice 7) so the drone loses its weight.
        self.ethereal_s += (self.ethereal - self.ethereal_s) * 0.00012;
        let eth = self.ethereal_s;
        let mut l = 0.0f32;
        let mut r = 0.0f32;
        for (i, v) in self.voices.iter_mut().enumerate() {
            let s = v.sample(self.sr)
                * if i == 0 || i == 7 {
                    1.0 - eth * 0.85
                } else {
                    1.0
                };
            // Equal-power pan.
            let p = (v.pan * 0.5 + 0.5) * (std::f32::consts::PI * 0.5);
            l += s * p.cos();
            r += s * p.sin();
        }
        // Warp (E18): smooth toward target (~0.1 s — proximity-driven, wants to respond) and run
        // the tremolo LFO (~6 Hz). High warp = a max-wobble colossus nearby.
        self.warp_s += (self.warp - self.warp_s) * 0.0003;
        self.trem_phase += 6.0 / self.sr;
        if self.trem_phase >= 1.0 {
            self.trem_phase -= 1.0;
        }
        // Tame the summed level before distortion, then waveshape hard for the doom grit.
        // Heaviness scales the drive; warp pushes it heavier still (more grit in warp zones);
        // ethereal guts it (a pristine pocket is almost clean — no doom grind).
        let pre = 0.42;
        let drive =
            self.base_drive * self.drive_mul * (1.0 + self.warp_s * 0.9) * (1.0 - eth * 0.8);
        let dl = (drive * l * pre).tanh();
        let dr = (drive * r * pre).tanh();

        // Reactive intensity (E16): smooth toward the target (~0.4 s) so flight changes glide in.
        self.intensity_s += (self.intensity - self.intensity_s) * 0.00006;
        // Murk: scale the cutoff sweep (tone 0 → ×0.5 darker, 1 → ×2.5 brighter). Flight
        // intensity nudges the effective openness up, so faster/higher flight brightens the drone;
        // ethereal throws it wide open (airy/bright, the murk lifts entirely).
        let tone_eff = (self.tone + self.intensity_s * 0.35 + eth * 1.2).min(2.0);
        let tone_scale = 0.5 + tone_eff * 2.0;
        let cutoff = (self.cutoff_lfo.step(self.sr) * tone_scale).clamp(60.0, self.sr * 0.45);
        let amp = self.amp_lfo.step(self.sr);
        // Step the second LFO read for R off the same value (mono modulation, stereo audio).
        let mut fl = self.filt_l.lowpass(dl, cutoff, self.res, self.sr);
        let mut fr = self.filt_r.lowpass(dr, cutoff, self.res, self.sr);

        // Ethereal shimmer: a glassy high chord (added *after* the filter so it stays bright),
        // panned gently, only present in pristine pockets — the "holy" sparkle over the cleaned drone.
        if eth > 0.001 {
            self.shimmer_phase += 1.0 / self.sr;
            if self.shimmer_phase >= 1.0 {
                self.shimmer_phase -= 1.0;
            }
            let t = self.shimmer_phase * TAU;
            let sh = ((t * 330.0).sin() + 0.6 * (t * 495.0).sin() + 0.4 * (t * 660.0).sin())
                * eth
                * 0.12;
            fl += sh;
            fr += sh * 0.85;
        }

        // A subtle swell with intensity so motion feels like it pushes the dirge; warp adds a
        // throbbing tremolo so warp zones pulse, unstable.
        let trem = 1.0 - self.warp_s * 0.5 * (0.5 + 0.5 * (self.trem_phase * TAU).sin());
        let g = self.master * amp * self.vol * (1.0 + 0.10 * self.intensity_s) * trem;
        [(fl * g).clamp(-1.0, 1.0), (fr * g).clamp(-1.0, 1.0)]
    }

    /// Fill an interleaved stereo buffer (`L, R, L, R, …`) — for live audio callbacks.
    pub fn render_block(&mut self, out: &mut [f32]) {
        for frame in out.chunks_exact_mut(2) {
            let [l, r] = self.next_frame();
            frame[0] = l;
            frame[1] = r;
        }
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

    #[test]
    fn intensity_changes_output_without_clipping() {
        // Same seed, one driven at full reactive intensity: it should audibly differ yet stay
        // within full scale (the reactive modulation must never blow the output up).
        let mut a = Drone::new(5, 44_100);
        let mut b = Drone::new(5, 44_100);
        b.set_intensity(1.0);
        let (mut peak, mut diff) = (0.0f32, 0.0f32);
        for _ in 0..44_100 {
            let [la, _] = a.next_frame();
            let [lb, rb] = b.next_frame();
            assert!(lb.is_finite() && rb.is_finite());
            peak = peak.max(lb.abs()).max(rb.abs());
            diff += (la - lb).abs();
        }
        assert!(peak <= 1.0, "intensity pushed past full scale: {peak}");
        assert!(diff > 0.0, "intensity had no audible effect");
    }

    #[test]
    fn warp_modulates_without_clipping() {
        let mut a = Drone::new(9, 44_100);
        let mut b = Drone::new(9, 44_100);
        b.set_warp(1.0);
        let (mut peak, mut diff) = (0.0f32, 0.0f32);
        for _ in 0..44_100 {
            let [la, _] = a.next_frame();
            let [lb, rb] = b.next_frame();
            assert!(lb.is_finite() && rb.is_finite());
            peak = peak.max(lb.abs()).max(rb.abs());
            diff += (la - lb).abs();
        }
        assert!(peak <= 1.0, "warp pushed past full scale: {peak}");
        assert!(diff > 0.0, "warp had no audible effect");
    }

    #[test]
    fn ethereal_transforms_strongly_without_clipping() {
        let mut a = Drone::new(11, 44_100);
        let mut b = Drone::new(11, 44_100);
        b.set_ethereal(1.0);
        let (mut peak, mut diff, mut e_a, mut e_b) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for _ in 0..44_100 {
            let [la, _] = a.next_frame();
            let [lb, rb] = b.next_frame();
            assert!(lb.is_finite() && rb.is_finite());
            peak = peak.max(lb.abs()).max(rb.abs());
            diff += (la - lb).abs();
            e_a += la * la;
            e_b += lb * lb;
        }
        assert!(peak <= 1.0, "ethereal pushed past full scale: {peak}");
        // It should be a *strong* change, not a subtle nudge.
        assert!(diff > 1000.0, "ethereal barely changed the drone: {diff}");
        assert!(e_a > 0.0 && e_b > 0.0);
    }
}
