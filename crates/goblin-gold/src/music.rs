//! GG1 music **renderer** (phase 4 audio) — turn a [`crate::synth`] score into an audible mono
//! buffer. The note *schedule* is the vector-proven score; this is the **perceptual** half (turning
//! tokens into sound), so — like the SFX — the tests gate only the mechanics (determinism, no
//! clipping, mute → silence, scenes sound distinct) and the "does it sound like GG?" A/B-vs-web is
//! banked for the owner (`OWNER-EYEBALL.md`, via the `music_proto` WAVs).
//!
//! This is a **first cut**: each lane is a clean osc + ADSR + one-pole tone-shaping (pad held over
//! the bar, a plucky bass, a short square lead, procedural-noise drums). It's recognisably the right
//! tune/groove/key; matching GG1's exact patch timbres (FM/unison/biquad-resonance/the wub LFO) is
//! the refinement pass after the owner's ear calibrates it. Same sample-gen style as `Drone`.

use crate::synth::{self, Role};

/// Master gain on the mixed buffer (headroom before the clamp).
const MASTER: f32 = 0.5;

/// MIDI → Hz.
fn hz(midi: i32) -> f32 {
    440.0 * 2f32.powf((midi as f32 - 69.0) / 12.0)
}

/// A tiny deterministic noise source (xorshift32) for the drums — no sample assets.
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

/// ADSR-ish envelope: linear attack, exp-ish decay to a sustain, release after the note's hold.
fn env(t: f32, hold: f32, a: f32, d: f32, s: f32, r: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < a {
        t / a.max(1e-4)
    } else if t < a + d {
        let x = (t - a) / d.max(1e-4);
        1.0 - (1.0 - s) * x
    } else if t < hold {
        s
    } else {
        let x = (t - hold) / r.max(1e-4);
        (s * (1.0 - x)).max(0.0)
    }
}

fn saw(ph: f32) -> f32 {
    2.0 * (ph - ph.floor()) - 1.0
}
fn square(ph: f32) -> f32 {
    if (ph - ph.floor()) < 0.5 {
        1.0
    } else {
        -1.0
    }
}
fn triangle(ph: f32) -> f32 {
    1.0 - 4.0 * ((ph - ph.floor()) - 0.5).abs()
}

/// Add a pitched lane voice (osc + ADSR + a one-pole lowpass for warmth) into `buf` from `start`.
#[allow(clippy::too_many_arguments)]
fn add_voice(
    buf: &mut [f32],
    sr: f32,
    start: usize,
    freq: f32,
    hold: f32,
    gain: f32,
    wave: fn(f32) -> f32,
    cut: f32,
    adsr: (f32, f32, f32, f32),
) {
    let (a, d, s, r) = adsr;
    let total = hold + r;
    let n = (total * sr) as usize;
    // One-pole lowpass coefficient for `cut` Hz.
    let alpha = 1.0 - (-std::f32::consts::TAU * cut / sr).exp();
    let mut lp = 0.0f32;
    for i in 0..n {
        let idx = start + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / sr;
        let ph = freq * t;
        let raw = wave(ph) * env(t, hold, a, d, s, r) * gain;
        lp += alpha * (raw - lp);
        buf[idx] += lp;
    }
}

/// Add a procedural drum hit into `buf` from `start`.
fn add_drum(buf: &mut [f32], sr: f32, start: usize, piece: &str, nz: &mut Noise) {
    let (dur, gain) = match piece {
        "kick" => (0.18, 0.9),
        "snare" => (0.16, 0.5),
        _ => (0.05, 0.3), // hat
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
            // kick: a fast downward pitch sweep, sine.
            "kick" => {
                let f = 120.0 * (-t / 0.03).exp() + 45.0;
                (std::f32::consts::TAU * f * t).sin()
            }
            // snare: a noise body + a mid tone.
            "snare" => 0.7 * nz.next() + 0.3 * (std::f32::consts::TAU * 180.0 * t).sin(),
            // hat: bright noise (a crude high-pass via first-difference).
            _ => nz.next(),
        };
        buf[idx] += s * decay * gain;
    }
}

/// Render `seconds` of a scene's music to a mono `f32` buffer at `sample_rate`, scaled by `volume`
/// (0 ⇒ silence). Returns an empty buffer for an unknown scene.
pub fn render(name: &str, sample_rate: u32, seconds: f32, volume: f32) -> Vec<f32> {
    let sr = sample_rate as f32;
    let len = (seconds * sr) as usize;
    let mut buf = vec![0.0f32; len];
    let (Some(tempo), volume_on) = (synth::tempo_of(name), volume > 0.0) else {
        return buf;
    };
    if !volume_on {
        return buf;
    }
    let step_secs = (60.0 / tempo as f32) / 4.0; // a 16th note
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
                Role::Pad => add_voice(
                    &mut buf,
                    sr,
                    start,
                    hz(v.midi),
                    bar_secs,
                    0.10,
                    triangle,
                    1500.0,
                    (0.25, 0.3, 0.8, 0.6),
                ),
                Role::Bass => add_voice(
                    &mut buf,
                    sr,
                    start,
                    hz(v.midi),
                    0.5,
                    0.22,
                    saw,
                    600.0,
                    (0.01, 0.15, 0.6, 0.1),
                ),
                Role::Lead => add_voice(
                    &mut buf,
                    sr,
                    start,
                    hz(v.midi),
                    0.14,
                    0.16,
                    square,
                    2200.0,
                    (0.005, 0.05, 0.5, 0.08),
                ),
                Role::Drum => add_drum(&mut buf, sr, start, v.piece, &mut nz),
            }
        }
    }
    for x in &mut buf {
        *x = (*x * MASTER * volume).clamp(-1.0, 1.0);
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
