//! GG1 **sound effects** (full-port phase 4 — audio), re-authored in Rust from `sound.js`. Each SFX
//! is a set of short oscillator voices `{freq, start, dur, waveform, gain}` with an 8 ms exponential
//! attack then an exponential decay to silence — mixed to a mono sample buffer. This is the same
//! sample-generation style as `scraped-again`'s `Drone` (pure DSP; playback is a separate
//! native/web hook), so it stays deterministic and unit-testable.
//!
//! Audio parity is **perceptual, not vector-provable** — there's no web-rendered reference to diff
//! against — so the tests gate what *is* mechanical (determinism, no clipping, the right length,
//! mute → silence, distinct events sound distinct, combo raises the pitch). The actual "does it
//! sound like GG?" A/B-vs-web check is banked for the owner (`OWNER-EYEBALL.md`); the `sfx_proto`
//! bin renders each effect to a WAV for that.

/// Oscillator waveforms used by the SFX (matching the Web Audio `OscillatorType`s in `sound.js`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wave {
    Square,
    Saw,
    Triangle,
    Sine,
}

/// One voice in an SFX: frequency (Hz), start offset (s), duration (s), waveform, peak gain.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    pub f: f32,
    pub t: f32,
    pub d: f32,
    pub wave: Wave,
    pub g: f32,
}

/// Collectible rarity — scales the `item` jingle (rarer → higher + more notes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// A sound effect to synthesize (the `sound.js` events).
#[derive(Clone, Copy, Debug)]
pub enum Sfx {
    /// A correct answer — pitch rises with the combo streak (capped at +1 octave).
    Correct {
        combo: u32,
    },
    /// A skip — a soft descending buzz.
    Skip,
    /// An item collected — a jingle scaled by rarity.
    Item {
        rarity: Rarity,
    },
    /// Gold earned (`big` = a fuller chime for large amounts).
    Gold {
        big: bool,
    },
    TopicUnlock,
    Mastery,
    Topic100,
    RoundStart,
    RoundComplete,
}

/// The SFX bus gain (`sound.js` default — blips sit a little above the music).
pub const SFX_VOL: f32 = 0.16;

const FLOOR: f32 = 0.0001; // the Web Audio exponential-ramp floor
const ATTACK: f32 = 0.008; // 8 ms attack

/// MIDI note → frequency (A4 = 69 = 440 Hz, equal temperament).
fn hz(midi: f32) -> f32 {
    440.0 * 2f32.powf((midi - 69.0) / 12.0)
}

fn note(midi: f32, t: f32, d: f32, wave: Wave, g: f32) -> Note {
    Note {
        f: hz(midi),
        t,
        d,
        wave,
        g,
    }
}

/// An arpeggio: one note per root, stepped `step` seconds apart from `t0`.
fn arp(roots: &[f32], t0: f32, step: f32, d: f32, wave: Wave, g: f32) -> Vec<Note> {
    roots
        .iter()
        .enumerate()
        .map(|(i, &m)| note(m, t0 + i as f32 * step, d, wave, g))
        .collect()
}

/// The voices for an SFX — re-authored verbatim from `sound.js`'s `sfxSpec`.
pub fn spec(e: &Sfx) -> Vec<Note> {
    use Wave::*;
    match *e {
        Sfx::Correct { combo } => {
            let base = 72.0 + combo.min(12) as f32; // rises with the streak, capped at +1 octave
            vec![
                note(base, 0.0, 0.05, Square, 0.16),
                note(base + 7.0, 0.045, 0.07, Square, 0.13),
            ]
        }
        Sfx::Skip => vec![
            note(57.0, 0.0, 0.07, Saw, 0.12),
            note(52.0, 0.06, 0.12, Saw, 0.10),
        ],
        Sfx::Item { rarity } => {
            let cnt = match rarity {
                Rarity::Common => 3,
                Rarity::Uncommon => 4,
                Rarity::Rare => 5,
                Rarity::Epic => 6,
                Rarity::Legendary => 7,
            };
            let root = 76.0 + (cnt as f32 - 3.0) * 2.0;
            let scale = [0.0, 4.0, 7.0, 12.0, 16.0, 19.0, 24.0];
            scale[..cnt]
                .iter()
                .enumerate()
                .map(|(i, &s)| note(root + s, i as f32 * 0.05, 0.09, Square, 0.14))
                .collect()
        }
        Sfx::Gold { big } => {
            let mut v = vec![
                note(84.0, 0.0, 0.05, Square, 0.14),
                note(91.0, 0.05, 0.08, Square, 0.13),
            ];
            if big {
                v.push(note(96.0, 0.12, 0.12, Square, 0.13));
            }
            v
        }
        Sfx::TopicUnlock => {
            let mut v = arp(&[72.0, 76.0, 79.0], 0.0, 0.08, 0.09, Square, 0.15);
            v.push(note(84.0, 0.24, 0.18, Square, 0.15));
            v
        }
        Sfx::Mastery => {
            let mut v = arp(
                &[72.0, 76.0, 79.0, 84.0, 88.0],
                0.0,
                0.07,
                0.10,
                Square,
                0.15,
            );
            v.push(note(91.0, 0.36, 0.16, Square, 0.13));
            v
        }
        Sfx::Topic100 => {
            let mut v = arp(
                &[72.0, 79.0, 76.0, 83.0, 88.0],
                0.0,
                0.06,
                0.10,
                Square,
                0.15,
            );
            v.push(note(84.0, 0.32, 0.22, Square, 0.14));
            v.push(note(91.0, 0.32, 0.22, Triangle, 0.10));
            v
        }
        Sfx::RoundStart => vec![
            note(67.0, 0.0, 0.05, Square, 0.12),
            note(72.0, 0.05, 0.08, Square, 0.13),
        ],
        Sfx::RoundComplete => vec![
            note(72.0, 0.0, 0.06, Square, 0.13),
            note(76.0, 0.06, 0.10, Square, 0.13),
        ],
    }
}

/// The exponential gain envelope at `tau` seconds into a note of duration `d` peaking at `g`: an 8 ms
/// exp attack from the floor to `g`, then an exp decay back to the floor by `d`.
fn envelope(tau: f32, d: f32, g: f32) -> f32 {
    if tau < 0.0 || tau > d {
        0.0
    } else if tau <= ATTACK {
        FLOOR * (g / FLOOR).powf(tau / ATTACK)
    } else {
        // Guard a degenerate (d <= ATTACK) note, though all SFX durations exceed 8 ms.
        let span = (d - ATTACK).max(1e-6);
        g * (FLOOR / g).powf((tau - ATTACK) / span)
    }
}

/// One oscillator sample at phase `f·tau`.
fn osc(wave: Wave, f: f32, tau: f32) -> f32 {
    let frac = (f * tau).fract();
    let frac = if frac < 0.0 { frac + 1.0 } else { frac };
    match wave {
        Wave::Sine => (std::f32::consts::TAU * f * tau).sin(),
        Wave::Square => {
            if frac < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Wave::Saw => 2.0 * frac - 1.0,
        Wave::Triangle => 1.0 - 4.0 * (frac - 0.5).abs(),
    }
}

/// Render an SFX to a mono `f32` buffer at `sample_rate`, scaled by `volume` (0..1, the player's
/// sound level — `0` ⇒ silence, the mute path). Voices are summed and the bus gain applied, then
/// brick-wall clamped to ±1 (the `sound.js` limiter analog).
pub fn render(e: &Sfx, sample_rate: u32, volume: f32) -> Vec<f32> {
    let sr = sample_rate as f32;
    let notes = spec(e);
    let total = notes.iter().map(|n| n.t + n.d).fold(0.0f32, f32::max) + 0.03; // a short tail (the oscillator stop happens at t1 + 0.03)
    let len = (total * sr).ceil() as usize;
    let mut buf = vec![0.0f32; len];
    if volume <= 0.0 {
        return buf; // muted → silence
    }
    for n in &notes {
        let start = (n.t * sr) as usize;
        let dur = (n.d * sr).ceil() as usize;
        for i in 0..dur {
            let idx = start + i;
            if idx >= len {
                break;
            }
            let tau = i as f32 / sr;
            buf[idx] += osc(n.wave, n.f, tau) * envelope(tau, n.d, n.g);
        }
    }
    let gain = SFX_VOL * volume;
    for s in &mut buf {
        *s = (*s * gain).clamp(-1.0, 1.0);
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 44_100;

    fn peak(b: &[f32]) -> f32 {
        b.iter().fold(0.0f32, |m, &s| m.max(s.abs()))
    }
    // A deterministic fingerprint, to catch silent drift in the synthesis.
    fn fingerprint(b: &[f32]) -> i64 {
        b.iter()
            .enumerate()
            .map(|(i, &s)| (s * 32767.0) as i64 * (i as i64 % 97 + 1))
            .sum()
    }

    const EVENTS: &[Sfx] = &[
        Sfx::Correct { combo: 0 },
        Sfx::Skip,
        Sfx::Item {
            rarity: Rarity::Legendary,
        },
        Sfx::Gold { big: true },
        Sfx::TopicUnlock,
        Sfx::Mastery,
        Sfx::Topic100,
        Sfx::RoundStart,
        Sfx::RoundComplete,
    ];

    #[test]
    fn every_sfx_is_deterministic_bounded_and_audible() {
        for e in EVENTS {
            let a = render(e, SR, 1.0);
            let b = render(e, SR, 1.0);
            assert_eq!(a, b, "render must be deterministic for {e:?}");
            assert!(!a.is_empty(), "{e:?} produced no samples");
            assert!(peak(&a) > 0.0, "{e:?} is silent");
            assert!(peak(&a) <= 1.0, "{e:?} clips (peak {})", peak(&a));
        }
    }

    #[test]
    fn mute_is_silence() {
        for e in EVENTS {
            let b = render(e, SR, 0.0);
            assert!(b.iter().all(|&s| s == 0.0), "{e:?} not silent when muted");
            // Same length as the audible render (just zeroed), so timing is unaffected.
            assert_eq!(b.len(), render(e, SR, 1.0).len());
        }
    }

    #[test]
    fn distinct_events_sound_distinct() {
        // Different events must not collapse to the same buffer (a guard against a broken `spec`).
        let prints: Vec<i64> = EVENTS
            .iter()
            .map(|e| fingerprint(&render(e, SR, 1.0)))
            .collect();
        for (i, a) in prints.iter().enumerate() {
            for (j, b) in prints.iter().enumerate() {
                if i != j {
                    assert_ne!(
                        a, b,
                        "{:?} and {:?} render identically",
                        EVENTS[i], EVENTS[j]
                    );
                }
            }
        }
    }

    #[test]
    fn combo_raises_the_correct_pitch() {
        // A higher combo → a higher base note → a different (higher-frequency) buffer.
        let lo = spec(&Sfx::Correct { combo: 0 })[0].f;
        let hi = spec(&Sfx::Correct { combo: 5 })[0].f;
        let cap = spec(&Sfx::Correct { combo: 99 })[0].f;
        assert!(hi > lo, "combo should raise the pitch");
        assert!(
            (cap - spec(&Sfx::Correct { combo: 12 })[0].f).abs() < 1e-3,
            "pitch is capped at +12 semitones"
        );
    }

    #[test]
    fn item_jingle_scales_with_rarity() {
        let common = spec(&Sfx::Item {
            rarity: Rarity::Common,
        });
        let legendary = spec(&Sfx::Item {
            rarity: Rarity::Legendary,
        });
        assert_eq!(common.len(), 3, "common = 3 notes");
        assert_eq!(legendary.len(), 7, "legendary = 7 notes");
        assert!(legendary[0].f > common[0].f, "rarer items start higher");
    }
}
